import copy
import hashlib
import json
import math
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator


MIN_PROBABILITY = 0.5
MAX_RESIDUAL_MS = 180
MIN_REGION_ANCHORS = 5
MIN_INLIER_RATIO = 0.6
MAX_REGION_SPREAD_MS = 180


def normalize(text):
    return "".join(character for character in text if character.isalnum()).lower()

ANCHOR_SCHEMA = {
    "type": "object", "additionalProperties": False,
    "required": ["region", "line_id", "word_index", "text", "qrc_ms", "audio_ms", "probability"],
    "properties": {
        "region": {"type": "integer", "minimum": 0, "maximum": 2},
        "line_id": {"type": "integer", "minimum": 0},
        "word_index": {"type": "integer", "minimum": 0},
        "text": {"type": "string", "minLength": 1},
        "qrc_ms": {"type": "number"}, "audio_ms": {"type": "number", "minimum": 0},
        "probability": {"type": "number", "minimum": 0, "maximum": 1},
    },
}
CALIBRATION_SCHEMA = {
    "type": "object", "additionalProperties": False,
    "required": ["version", "status", "method", "model", "model_sha256", "audio_sha256",
                 "timing_sha256", "offset_ms", "anchors", "analysis", "windows"],
    "properties": {
        "version": {"const": 1}, "status": {"const": "passed"},
        "method": {"const": "whisper-known-text"}, "model": {"type": "string", "minLength": 1},
        **{key: {"type": "string", "pattern": "^[0-9a-f]{64}$"}
           for key in ["model_sha256", "audio_sha256", "timing_sha256"]},
        "offset_ms": {"type": "number"},
        "anchors": {"type": "array", "minItems": 15, "items": ANCHOR_SCHEMA},
        "analysis": {"type": "object"}, "windows": {"type": "array", "minItems": 3, "maxItems": 3,
            "items": {"type": "object", "additionalProperties": False,
                "required": ["region", "start_ms", "end_ms", "line_ids"], "properties": {
                    "region": {"type": "integer", "minimum": 0, "maximum": 2},
                    "start_ms": {"type": "number", "minimum": 0}, "end_ms": {"type": "number", "exclusiveMinimum": 0},
                    "line_ids": {"type": "array", "minItems": 1, "uniqueItems": True,
                                 "items": {"type": "integer", "minimum": 0}}}}},
    },
}


def file_hash(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def timing_hash(project):
    data = json.dumps(project["lines"], ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False)
    return hashlib.sha256(data.encode("utf-8")).hexdigest()


def validate_offset_range(project, offset):
    if not math.isfinite(offset):
        raise ValueError("歌词整体偏移必须为有限数值")
    for line in project["lines"]:
        for word in line["words"] + line["romaji_words"]:
            if word["text"].strip() and (word["start_ms"] + offset < 0
                                          or word["end_ms"] + offset > project["duration_ms"]):
                raise ValueError(f"校准后的演唱片段超出完整音频范围：{word['text']}")


def analyze_anchors(anchors):
    for anchor in anchors:
        Draft202012Validator(ANCHOR_SCHEMA).validate(anchor)
        if not all(math.isfinite(anchor[field]) for field in ["qrc_ms", "audio_ms", "probability"]):
            raise ValueError("校准测量值必须为有限数值")
    reliable = [anchor for anchor in anchors if anchor["probability"] >= MIN_PROBABILITY]
    if not reliable:
        raise ValueError("音频没有可靠的歌词测量位置")
    offsets = np.array([anchor["audio_ms"] - anchor["qrc_ms"] for anchor in reliable])
    # 在有限误差内寻找共同偏移，避免少量错误读音影响整首歌曲。
    ordered = np.sort(offsets)
    candidates = [(left, int(np.searchsorted(ordered, value + 2 * MAX_RESIDUAL_MS, side="right")))
                  for left, value in enumerate(ordered)]
    left, right = max(candidates, key=lambda pair: pair[1] - pair[0])
    offset = float(np.median(ordered[left:right]))
    inliers = [anchor for anchor in reliable
               if abs(anchor["audio_ms"] - anchor["qrc_ms"] - offset) <= MAX_RESIDUAL_MS]
    if len(inliers) / len(reliable) < MIN_INLIER_RATIO:
        raise ValueError("歌词测量位置的偏移不一致，无法确认整体偏移")
    offset = round(float(np.median([anchor["audio_ms"] - anchor["qrc_ms"] for anchor in inliers])), 3)
    inliers = [anchor for anchor in reliable
               if abs(anchor["audio_ms"] - anchor["qrc_ms"] - offset) <= MAX_RESIDUAL_MS]
    regions = []
    for region in range(3):
        sampled = [anchor for anchor in reliable if anchor["region"] == region]
        accepted = [anchor for anchor in inliers if anchor["region"] == region]
        if (len(accepted) < MIN_REGION_ANCHORS or not sampled
                or len(accepted) / len(sampled) < MIN_INLIER_RATIO):
            raise ValueError(f"歌曲第 {region + 1} 个区段缺少一致的可靠校准位置")
        median = float(np.median([anchor["audio_ms"] - anchor["qrc_ms"] for anchor in accepted]))
        regions.append({"region": region, "reliable_anchors": len(sampled), "accepted_anchors": len(accepted),
                        "offset_ms": round(median, 3),
                        "max_residual_ms": round(max(abs(a["audio_ms"] - a["qrc_ms"] - offset) for a in accepted), 3)})
    if max(region["offset_ms"] for region in regions) - min(region["offset_ms"] for region in regions) > MAX_REGION_SPREAD_MS:
        raise ValueError("歌曲前段、中段、后段的偏移存在漂移")
    return {"offset_ms": offset, "reliable_anchors": len(reliable), "accepted_anchors": len(inliers),
            "regions": regions, "max_residual_ms": round(max(abs(a["audio_ms"] - a["qrc_ms"] - offset) for a in inliers), 3),
            "criteria": {"min_probability": MIN_PROBABILITY, "max_residual_ms": MAX_RESIDUAL_MS,
                         "min_region_anchors": MIN_REGION_ANCHORS, "min_inlier_ratio": MIN_INLIER_RATIO,
                         "max_region_spread_ms": MAX_REGION_SPREAD_MS}}


def validate_calibration(project):
    path = Path(project["calibration_path"])
    if not path.is_absolute():
        raise ValueError("校准报告路径必须为绝对路径")
    report = json.loads(path.read_text(encoding="utf-8"))
    Draft202012Validator(CALIBRATION_SCHEMA).validate(report)
    if file_hash(project["audio_path"]) != report["audio_sha256"] or timing_hash(project) != report["timing_sha256"]:
        raise ValueError("校准报告与当前音频或原始歌词时间不一致")
    if not math.isfinite(project["lyric_offset_ms"]) or report["offset_ms"] != project["lyric_offset_ms"]:
        raise ValueError("项目歌词偏移与校准报告不同")
    known = {(line["id"], index): word for line in project["lines"] for index, word in enumerate(line["words"])}
    lines = {line["id"]: line for line in project["lines"]}
    windows = {window["region"]: window for window in report["windows"]}
    if set(windows) != {0, 1, 2}:
        raise ValueError("校准报告必须覆盖三个独立区段")
    for region, indices in enumerate(np.array_split(np.arange(len(project["lines"])), 3)):
        window = windows[region]
        ids = {project["lines"][int(index)]["id"] for index in indices}
        if (not set(window["line_ids"]) <= ids or window["start_ms"] >= window["end_ms"]
                or window["end_ms"] > project["duration_ms"] or window["end_ms"] - window["start_ms"] > 30000):
            raise ValueError("校准窗口与完整歌曲的区段或时间范围不同")
    seen = set()
    for anchor in report["anchors"]:
        pair = anchor["line_id"], anchor["word_index"]
        word = known.get(pair)
        if pair in seen or word is None or word["start_ms"] != anchor["qrc_ms"] or word["end_ms"] <= word["start_ms"]:
            raise ValueError("校准测量位置与原始歌词片段不同或重复")
        window = windows[anchor["region"]]
        if anchor["line_id"] not in window["line_ids"] or not window["start_ms"] <= anchor["audio_ms"] < window["end_ms"]:
            raise ValueError("校准测量位置超出所属歌词区段或音频窗口")
        line_index = window["line_ids"].index(anchor["line_id"])
        tail = "".join(item["text"] for item in lines[anchor["line_id"]]["words"][anchor["word_index"]:])
        tail += "".join(lines[line_id]["text"] for line_id in window["line_ids"][line_index + 1:])
        if not normalize(anchor["text"]) or not normalize(tail).startswith(normalize(anchor["text"])):
            raise ValueError("校准测量文字与对应的原始歌词不同")
        seen.add(pair)
    analysis = analyze_anchors(report["anchors"])
    if analysis != report["analysis"] or analysis["offset_ms"] != report["offset_ms"]:
        raise ValueError("校准报告的测量结果无法重复计算")
    validate_offset_range(project, project["lyric_offset_ms"])
    return report


def aligned_project(project):
    validate_calibration(project)
    result = copy.deepcopy(project)
    offset = project["lyric_offset_ms"]
    for line in result["lines"]:
        for item in [line, *line["words"], *line["romaji_words"]]:
            item["start_ms"] = round(item["start_ms"] + offset, 3)
            item["end_ms"] = round(item["end_ms"] + offset, 3)
    return result
