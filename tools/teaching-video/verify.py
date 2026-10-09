import argparse
import bisect
import json
import math
import subprocess
from pathlib import Path

import numpy as np
from PIL import Image

from render import PROJECT_SCHEMA, TEACHING_SCHEMA, Renderer, audio_filter, file_hash, probe, read_json
from composition import EDIT_SCHEMA, empty_edits
from timing import validate_calibration


def run(command):
    result = subprocess.run(command, capture_output=True)
    if result.stderr:
        raise RuntimeError(result.stderr.decode("utf-8"))
    result.check_returncode()
    return result.stdout


def sample_indices(manifest, project):
    fps, count = manifest["fps"], manifest["frames"]
    start_ms = manifest["start_ms"]
    end_ms = start_ms + manifest["duration_ms"]
    indices = {0, count - 1}
    for scene in manifest["scenes"]:
        for page in scene["pages"]:
            left, right = max(start_ms, page["start_ms"]), min(end_ms, page["end_ms"])
            if right > left:
                indices.add(min(count - 1, math.floor(((left + right) / 2 - start_ms) * fps / 1000)))
        line = next(line for line in project["lines"] if line["id"] == scene["line_id"])
        finish = math.ceil((max(word["end_ms"] for word in line["words"]) - start_ms) * fps / 1000)
        if 0 <= finish < count and start_ms + finish * 1000 / fps < scene["end_ms"]:
            indices.add(finish)
    visible_lines = [line for line in project["lines"] if line["end_ms"] > start_ms and line["start_ms"] < end_ms]
    for index in sorted({0, len(visible_lines) // 2, len(visible_lines) - 1}):
        if index < 0 or not visible_lines:
            continue
        line = visible_lines[index]
        for key in ["words", "romaji_words"]:
            word = line[key][len(line[key]) // 2]
            for time_ms in [word["start_ms"], (word["start_ms"] + word["end_ms"]) / 2, word["end_ms"]]:
                if start_ms <= time_ms < end_ms:
                    indices.add(min(count - 1, math.floor((time_ms - start_ms) * fps / 1000)))
    return sorted(indices)


def check_baselines(scene, observed, expected, background, renderer):
    results = []
    for row in scene["original_rows"]:
        font = renderer.font(row["font_size"])
        for glyph in row["glyphs"]:
            core, offset = font.getmask2(glyph["text"], mode="L", anchor="ls")
            mask = Image.frombytes("L", core.size, bytes(core))
            ink = mask.getbbox()
            left, top = glyph["origin_x"] + offset[0] + ink[0], row["baseline"] + offset[1] + ink[1]
            right, bottom = left + ink[2] - ink[0], top + ink[3] - ink[1]
            if glyph["bounds"] != [left, top, right, bottom]:
                raise ValueError(f"歌词字符没有使用整行字体基线：{glyph['text']} / {scene['text']}")
            reference = np.max(np.abs(expected[top:bottom, left:right] - background[top:bottom, left:right]), axis=2)
            threshold = max(8, float(reference.max()) * 0.3)
            reference = reference > threshold
            if not reference.any():
                raise ValueError(f"歌词字符没有可验证的画面对比度：{glyph['text']}")
            matches = []
            for delta in range(-4, 5):
                if top + delta < 0 or bottom + delta > renderer.height:
                    continue
                shifted = observed[top + delta:bottom + delta, left:right]
                base = background[top + delta:bottom + delta, left:right]
                foreground = np.max(np.abs(shifted - base), axis=2) > threshold
                intersection = np.count_nonzero(reference & foreground)
                union = np.count_nonzero(reference | foreground)
                matches.append((intersection / union, delta))
            score, delta = max(matches, key=lambda match: (match[0], -abs(match[1])))
            if abs(delta) > 2 or score < 0.6:
                raise ValueError(f"编码后的歌词字符偏离字体基线：{glyph['text']} / delta={delta} / score={score}")
            results.append({"index": glyph["index"], "text": glyph["text"], "baseline": row["baseline"],
                            "bounds": glyph["bounds"], "encoded_vertical_offset": delta, "match_score": score})
    return results


def check_hashes(manifest, project, output):
    if Path(manifest["calibration"]).resolve(strict=True) != Path(project["calibration_path"]).resolve(strict=True):
        raise ValueError("校准报告与导出记录不同")
    if manifest["lyric_offset_ms"] != project["lyric_offset_ms"]:
        raise ValueError("歌词偏移与导出记录不同")
    if manifest["edits"] and file_hash(manifest["edits"]) != manifest["edits_sha256"]:
        raise ValueError("后期编辑文件与导出记录不同")
    for path, expected in [(output, manifest["output_sha256"]),
                           (manifest["project"], manifest["project_sha256"]),
                           (manifest["teaching"], manifest["teaching_sha256"]),
                           (project["audio_path"], manifest["audio_sha256"]),
                           (project["background_path"], manifest["background_sha256"]),
                           (manifest["calibration"], manifest["calibration_sha256"]),
                           (manifest["font"], manifest["font_sha256"])]:
        if file_hash(path) != expected:
            raise ValueError(f"渲染输入或产物发生变化：{path}")


def check_streams(manifest, output):
    metadata = probe(output)
    video = [stream for stream in metadata["streams"] if stream["codec_type"] == "video"]
    audio = [stream for stream in metadata["streams"] if stream["codec_type"] == "audio"]
    if len(video) != 1 or len(audio) != 1:
        raise ValueError("成品必须恰好包含一个视频和一个音频")
    video, audio = video[0], audio[0]
    if video["codec_name"] != "h264" or audio["codec_name"] != "aac":
        raise ValueError("成品编码必须使用 H.264 和 AAC")
    if [video["width"], video["height"]] != [manifest["width"], manifest["height"]]:
        raise ValueError("成品分辨率与布局不一致")
    if video["avg_frame_rate"] != "30/1" or int(video["nb_frames"]) != manifest["frames"]:
        raise ValueError("成品帧率或完整帧数错误")
    if video["pix_fmt"] != "yuv420p" or audio["sample_rate"] != "48000" or audio["channels"] != 2:
        raise ValueError("成品像素格式或音频格式错误")
    if abs(float(video["duration"]) * 1000 - manifest["duration_ms"]) > 2:
        raise ValueError("视频时长与完整帧数不一致")
    if abs(float(audio["duration"]) * 1000 - manifest["duration_ms"]) > 50:
        raise ValueError("音频与视频时长不一致")
    run(["ffmpeg", "-v", "error", "-nostdin", "-i", str(output), "-map", "0:v:0", "-map", "0:a:0", "-f", "null", "-"])
    return metadata


def check_audio(manifest, project, output):
    body = audio_filter(manifest["start_ms"], manifest["duration_ms"], manifest["audio_offset_ms"])
    expected = np.frombuffer(run(["ffmpeg", "-v", "error", "-nostdin", "-i", project["audio_path"],
                                  "-vn", "-af", body, "-ar", "48000", "-ac", "1", "-f", "f32le", "-"]), dtype="<f4")
    observed = np.frombuffer(run(["ffmpeg", "-v", "error", "-nostdin", "-i", str(output),
                                  "-vn", "-ar", "48000", "-ac", "1", "-f", "f32le", "-"]), dtype="<f4")
    target_samples = round(manifest["duration_ms"] * 48)
    if min(len(expected), len(observed)) < target_samples - 2400:
        raise ValueError("成品音频缺少歌曲末尾采样")
    count = min(len(expected), len(observed), target_samples)
    expected, observed = expected[:count].astype(np.float64), observed[:count].astype(np.float64)
    energy = float(np.dot(expected, expected))
    observed_energy = float(np.dot(observed, observed))
    if energy / count < 0.000001 or observed_energy / count < 0.000001:
        raise ValueError("源音频或成品音频为空白")
    correlation = float(np.dot(expected, observed) / math.sqrt(energy * observed_energy))
    normalized_error = float(math.sqrt(np.dot(expected - observed, expected - observed) / energy))
    if correlation < 0.97 or normalized_error > 0.25:
        raise ValueError(f"成品音频与指定源时间不同步：correlation={correlation}，normalized_error={normalized_error}")
    return {"sample_rate": 48000, "compared_samples": count, "correlation": correlation,
            "normalized_error": normalized_error, "audio_offset_ms": manifest["audio_offset_ms"]}


def main():
    parser = argparse.ArgumentParser(description="验证真实教学视频的完整解码、全部教学分页、逐字画面和源音频同步")
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--evidence-dir", type=Path)
    parser.add_argument("--require-full-song", action="store_true")
    args = parser.parse_args()
    manifest_path = args.manifest.resolve(strict=True)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest["version"] != 1:
        raise ValueError("不支持此渲染记录版本")
    output = Path(manifest["output"]).resolve(strict=True)
    project = read_json(manifest["project"], PROJECT_SCHEMA)
    teaching = read_json(manifest["teaching"], TEACHING_SCHEMA)
    calibration = validate_calibration(project)
    full_song = (manifest["start_ms"] == 0
                 and manifest["frames"] == math.ceil(project["duration_ms"] * 30 / 1000)
                 and manifest["audio_offset_ms"] == 0)
    if args.require_full_song and not full_song:
        raise ValueError("成品没有覆盖从 00:00 到歌曲末尾的完整音频")
    check_hashes(manifest, project, output)
    metadata = check_streams(manifest, output)
    edits = read_json(manifest["edits"], EDIT_SCHEMA) if manifest["edits"] else empty_edits()
    renderer = Renderer(project, teaching, manifest["layout"], manifest["font"], manifest["font_index"], manifest["accent"], edits)
    if renderer.evidence() != manifest["scenes"]:
        raise ValueError("当前排版与成品渲染记录不一致")
    evidence_dir = (args.evidence_dir or output.parent / f"{output.stem}-evidence").resolve()
    evidence_dir.mkdir(parents=True, exist_ok=True)
    indices = sample_indices(manifest, renderer.project)
    expressions = [f"eq(n,{index})" for index in indices]
    while len(expressions) > 1:
        expressions = [f"({'+'.join(expressions[start:start + 2])})" for start in range(0, len(expressions), 2)]
    select = expressions[0]
    pattern = evidence_dir / "frame-%05d.png"
    run(["ffmpeg", "-v", "error", "-nostdin", "-i", str(output), "-an", "-vf", f"select='{select}'",
         "-fps_mode", "vfr", "-start_number", "0", "-y", str(pattern)])
    frame_results = []
    for ordinal, index in enumerate(indices):
        path = evidence_dir / f"frame-{ordinal:05d}.png"
        with Image.open(path) as image:
            observed = np.asarray(image.convert("RGB"), dtype=np.int16)
        time_ms = manifest["start_ms"] + index * 1000 / manifest["fps"]
        expected = np.asarray(renderer.frame(time_ms), dtype=np.int16)
        difference = np.abs(expected - observed)
        mean_error, percentile = float(difference.mean()), float(np.percentile(difference, 99))
        if mean_error > 7 or percentile > 50:
            raise ValueError(f"视频画面与歌词时间或教学分页不一致：frame={index}，mean={mean_error}，p99={percentile}")
        scene_index = max(0, min(len(renderer.scenes) - 1, bisect.bisect_right(renderer.starts, time_ms) - 1))
        background = np.asarray(renderer.scenes[scene_index]["background"], dtype=np.int16)
        baselines = check_baselines(renderer.scenes[scene_index], observed, expected, background, renderer)
        frame_results.append({"frame": index, "song_time_ms": time_ms, "file": str(path),
                              "mean_absolute_error": mean_error, "error_percentile_99": percentile,
                              "lyric_baselines": baselines})
    audio = check_audio(manifest, project, output)
    start_ms, end_ms = manifest["start_ms"], manifest["start_ms"] + manifest["duration_ms"]
    visible_scenes = [scene for scene in manifest["scenes"] if scene["end_ms"] > start_ms and scene["start_ms"] < end_ms]
    visible_pages = [page for scene in visible_scenes for page in scene["pages"]
                     if page["end_ms"] > start_ms and page["start_ms"] < end_ms]
    report = {
        "version": 1, "status": "passed", "video": str(output), "video_sha256": file_hash(output),
        "manifest": str(manifest_path), "full_decode": True, "metadata": metadata,
        "song_scope": "full" if full_song else "clip", "scene_count": len(visible_scenes), "page_count": len(visible_pages),
        "sampled_frames": frame_results, "audio": audio,
        "lyric_alignment": {"calibration": manifest["calibration"], "calibration_sha256": manifest["calibration_sha256"],
                            "lyric_offset_ms": manifest["lyric_offset_ms"], "report": calibration},
        "baseline_check": {"status": "passed", "glyph_samples": sum(len(frame["lyric_baselines"]) for frame in frame_results),
                           "max_encoded_vertical_offset": max((abs(glyph["encoded_vertical_offset"])
                                                               for frame in frame_results for glyph in frame["lyric_baselines"]), default=0)},
    }
    report_path = evidence_dir / "verification.json"
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": "passed", "video": str(output), "report": str(report_path),
                      "sampled_frames": len(frame_results), "audio_correlation": audio["correlation"]}, ensure_ascii=False))


if __name__ == "__main__":
    main()
