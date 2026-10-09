import argparse
import json
import subprocess
import sys
from pathlib import Path

from render import PROJECT_SCHEMA, TEACHING_SCHEMA, Renderer, file_hash, probe, read_json


def run(command, log):
    with log.open("w", encoding="utf-8") as stream:
        subprocess.run(command, stdout=stream, stderr=stream, check=True)


def main():
    parser = argparse.ArgumentParser(description="通过真实歌曲与已知音频延迟验证自动歌词校准和视频导出")
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--teaching", type=Path, required=True)
    parser.add_argument("--font", type=Path, required=True)
    parser.add_argument("--evidence-dir", type=Path, required=True)
    args = parser.parse_args()
    folder = args.evidence_dir.resolve()
    folder.mkdir(parents=True, exist_ok=False)
    original = json.loads(args.project.read_text(encoding="utf-8"))
    paths = [args.project, args.teaching, Path(original["audio_path"]), Path(original["background_path"])]
    before = {str(path.resolve()): file_hash(path) for path in paths}
    script = Path(__file__).parent
    calibrated_path = folder / "project.json"
    run([sys.executable, str(script / "calibrate.py"), "--project", str(args.project), "--output", str(calibrated_path)],
        folder / "calibrate.log")
    calibrated = read_json(calibrated_path, PROJECT_SCHEMA)
    assert calibrated["lines"] == original["lines"]
    teaching = read_json(args.teaching, TEACHING_SCHEMA)
    for layout in ["landscape", "portrait"]:
        renderer = Renderer(calibrated, teaching, layout, args.font)
        for source_line, line in zip(original["lines"], renderer.project["lines"]):
            for key in ["words", "romaji_words"]:
                for source_word, word in zip(source_line[key], line[key]):
                    assert abs(word["start_ms"] - source_word["start_ms"] - calibrated["lyric_offset_ms"]) < 0.001
                    assert abs((word["end_ms"] - word["start_ms"]) - (source_word["end_ms"] - source_word["start_ms"])) < 0.001
    # 使用真实录音增加明确的前导静音，检验自动测量是否恢复同样的时间变化。
    delay_ms = 1250
    delayed_audio = folder / "delayed.m4a"
    run(["ffmpeg", "-v", "error", "-nostdin", "-i", original["audio_path"], "-vn",
         "-af", f"adelay={delay_ms}:all=1", "-c:a", "aac", "-b:a", "192k", "-n", str(delayed_audio)], folder / "delay.log")
    delayed_source = {**original, "audio_path": str(delayed_audio),
                      "duration_ms": float(probe(delayed_audio)["format"]["duration"]) * 1000}
    delayed_source_path = folder / "delayed-source.json"
    delayed_source_path.write_text(json.dumps(delayed_source, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    delayed_path = folder / "delayed-project.json"
    run([sys.executable, str(script / "calibrate.py"), "--project", str(delayed_source_path), "--output", str(delayed_path)],
        folder / "delayed-calibrate.log")
    delayed = read_json(delayed_path, PROJECT_SCHEMA)
    measured_delay = delayed["lyric_offset_ms"] - calibrated["lyric_offset_ms"]
    assert abs(measured_delay - delay_ms) <= 180, measured_delay
    changed_input = {**calibrated, "audio_path": str(delayed_audio), "duration_ms": delayed["duration_ms"]}
    changed_path = folder / "changed-input.json"
    changed_path.write_text(json.dumps(changed_input, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    rejected_video = folder / "rejected.mp4"
    with (folder / "changed-input.log").open("w", encoding="utf-8") as log:
        rejected = subprocess.run([sys.executable, str(script / "render.py"), "--project", str(changed_path),
                                   "--teaching", str(args.teaching), "--font", str(args.font), "--layout", "landscape",
                                   "--output", str(rejected_video)], stdout=log, stderr=log)
    assert rejected.returncode != 0 and not rejected_video.exists()
    assert "校准报告与当前音频或原始歌词时间不一致" in (folder / "changed-input.log").read_text(encoding="utf-8")
    video = folder / "complete.mp4"
    run([sys.executable, str(script / "render.py"), "--project", str(calibrated_path), "--teaching", str(args.teaching),
         "--font", str(args.font), "--layout", "landscape", "--output", str(video)], folder / "render.log")
    run([sys.executable, str(script / "verify.py"), "--manifest", str(video.with_suffix(".render.json")),
         "--require-full-song", "--evidence-dir", str(folder / "video-evidence")], folder / "verify.log")
    verification = json.loads((folder / "video-evidence" / "verification.json").read_text(encoding="utf-8"))
    assert verification["status"] == "passed" and verification["song_scope"] == "full"
    after = {str(path.resolve()): file_hash(path) for path in paths}
    assert before == after
    result = {"status": "passed", "source_times_preserved": True, "source_sha256": after,
              "project": str(calibrated_path), "offset_ms": calibrated["lyric_offset_ms"],
              "known_audio_delay_ms": delay_ms, "measured_delay_ms": measured_delay,
              "changed_audio_rejected": True, "orientations": ["landscape", "portrait"],
              "video": str(video), "verification": str(folder / "video-evidence" / "verification.json")}
    report = folder / "verification.json"
    report.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": "passed", "report": str(report)}, ensure_ascii=False))


if __name__ == "__main__":
    main()
