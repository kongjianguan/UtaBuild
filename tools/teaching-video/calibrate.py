import argparse
import copy
import json
from pathlib import Path

import numpy as np
from faster_whisper import WhisperModel
from faster_whisper.audio import decode_audio, pad_or_trim
from faster_whisper.tokenizer import Tokenizer
from faster_whisper.utils import download_model

from render import PROJECT_SCHEMA, read_json
from timing import aligned_project, analyze_anchors, file_hash, normalize, timing_hash, validate_offset_range


SOURCE_PROJECT_SCHEMA = copy.deepcopy(PROJECT_SCHEMA)
for key in ["lyric_offset_ms", "calibration_path"]:
    SOURCE_PROJECT_SCHEMA["required"].remove(key)
    del SOURCE_PROJECT_SCHEMA["properties"][key]


def sample_windows(project):
    lines = project["lines"]
    if len(lines) < 9:
        raise ValueError("自动校准需要分布于歌曲前段、中段、后段的完整歌词")
    windows = []
    for region, indices in enumerate(np.array_split(np.arange(len(lines)), 3)):
        candidates = []
        for index in indices:
            selected = []
            for line in lines[int(index):int(indices[-1]) + 1]:
                if line["end_ms"] - lines[int(index)]["start_ms"] > 22000:
                    break
                selected.append(line)
            if selected:
                candidates.append(selected)
        if not candidates:
            raise ValueError("校准歌词片段超出模型的音频窗口")
        selected = max(candidates, key=lambda group: sum(len(normalize(line["text"])) for line in group))
        start = max(0, selected[0]["start_ms"] - 4000)
        end = min(project["duration_ms"], selected[-1]["end_ms"] + 4000)
        windows.append({"region": region, "start_ms": start, "end_ms": end,
                        "line_ids": [line["id"] for line in selected], "lines": selected})
    return windows


def measure_window(model, tokenizer, audio, window):
    clip = audio[round(window["start_ms"] * 16):round(window["end_ms"] * 16)]
    if not len(clip) or len(clip) > 30 * 16000:
        raise ValueError("校准音频窗口必须位于 30 秒以内")
    text = "\n".join(line["text"] for line in window["lines"])
    features = model.feature_extractor(clip)
    num_frames = features.shape[-1] - 1
    encoded = model.encode(pad_or_trim(features))
    aligned = model.find_alignment(tokenizer, [tokenizer.encode(text)], encoded,
                                   [num_frames])[0]
    reference = {}
    cursor = 0
    for line in window["lines"]:
        for index, word in enumerate(line["words"]):
            content = normalize(word["text"])
            if content and word["end_ms"] > word["start_ms"]:
                reference[cursor] = {"line_id": line["id"], "word_index": index, **word}
            cursor += len(content)
    if normalize("".join(word["word"] for word in aligned)) != normalize(text):
        raise ValueError("声学匹配输出没有覆盖给定的完整歌词")
    cursor = 0
    anchors = []
    for word in aligned:
        content = normalize(word["word"])
        if content and cursor in reference and word["end"] > word["start"]:
            original = reference[cursor]
            anchors.append({"region": window["region"], "line_id": original["line_id"],
                            "word_index": original["word_index"], "text": word["word"],
                            "qrc_ms": original["start_ms"],
                            "audio_ms": round(float(word["start"] * 1000 + window["start_ms"]), 3),
                            "probability": float(word["probability"])})
        cursor += len(content)
    return anchors


def write_json(path, document):
    path.write_text(json.dumps(document, ensure_ascii=False, indent=2, allow_nan=False) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description="根据实际音频自动测量 QRC 歌词的整体时间偏移")
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--model", default="small")
    args = parser.parse_args()
    source = args.project.resolve(strict=True)
    output = args.output.resolve()
    if output == source or output.exists():
        raise FileExistsError("校准输出必须使用独立且未使用的 project 路径")
    output.parent.mkdir(parents=True, exist_ok=True)
    report_path = output.with_suffix(".calibration.json")
    measurements_path = output.with_suffix(".measurements.json")
    if report_path.exists() or measurements_path.exists():
        raise FileExistsError("校准证据文件已经存在")
    project = read_json(source, SOURCE_PROJECT_SCHEMA)
    source_sha256, audio_sha256 = file_hash(source), file_hash(project["audio_path"])
    windows = sample_windows(project)
    cache = Path(__file__).resolve().parents[2] / "work" / "whisper-models"
    model_path = download_model(args.model, cache_dir=str(cache))
    model = WhisperModel(model_path, device="cpu", compute_type="int8")
    tokenizer = Tokenizer(model.hf_tokenizer, model.model.is_multilingual, task="transcribe", language="ja")
    audio = decode_audio(project["audio_path"], sampling_rate=16000)
    if abs(len(audio) / 16 - project["duration_ms"]) > 100:
        raise ValueError("校准输入的音频时长与项目不同")
    anchors = []
    for window in windows:
        measured = measure_window(model, tokenizer, audio, window)
        anchors.extend(measured)
        print(json.dumps({"measured_region": window["region"], "anchors": len(measured)}, ensure_ascii=False), flush=True)
    evidence = {"version": 1, "method": "whisper-known-text", "model": args.model,
                "model_sha256": file_hash(Path(model_path) / "model.bin"),
                "audio_sha256": audio_sha256, "timing_sha256": timing_hash(project),
                "anchors": anchors, "windows": [{key: value for key, value in window.items() if key != "lines"} for window in windows]}
    write_json(measurements_path, evidence)
    analysis = analyze_anchors(anchors)
    validate_offset_range(project, analysis["offset_ms"])
    if file_hash(source) != source_sha256 or file_hash(project["audio_path"]) != audio_sha256:
        raise ValueError("校准期间源项目或音频发生变化")
    report = {**evidence, "status": "passed", "offset_ms": analysis["offset_ms"], "analysis": analysis}
    write_json(report_path, report)
    project["lyric_offset_ms"] = report["offset_ms"]
    project["calibration_path"] = str(report_path)
    aligned_project(project)
    write_json(output, project)
    print(json.dumps({"status": "passed", "project": str(output), "calibration": str(report_path),
                      "lyric_offset_ms": report["offset_ms"], "analysis": analysis}, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
