import argparse
import copy
import bisect
import hashlib
import json
import math
import re
import subprocess
import unicodedata
from dataclasses import dataclass
from pathlib import Path

from fontTools.ttLib import TTFont
from jsonschema import Draft202012Validator
from PIL import Image, ImageDraw, ImageFilter, ImageFont, ImageOps

from composition import EDIT_SCHEMA, Composition, compose, cover_accent, empty_edits, validate_component_ids, validate_edits
from timing import aligned_project, validate_calibration


TIMED_WORD = {
    "type": "object", "additionalProperties": False,
    "required": ["text", "start_ms", "end_ms"],
    "properties": {"text": {"type": "string", "minLength": 1},
                   "start_ms": {"type": "number", "minimum": 0},
                   "end_ms": {"type": "number", "minimum": 0}},
}
PROJECT_SCHEMA = {
    "type": "object", "additionalProperties": False,
    "required": ["version", "title", "artist", "duration_ms", "audio_path", "background_path",
                 "lyrics_url", "ruby_url", "lines", "lyric_offset_ms", "calibration_path"],
    "properties": {
        "version": {"const": 1}, "title": {"type": "string", "minLength": 1},
        "artist": {"type": "string", "minLength": 1}, "duration_ms": {"type": "number", "exclusiveMinimum": 0},
        "audio_path": {"type": "string", "minLength": 1}, "background_path": {"type": "string", "minLength": 1},
        "audio_url": {"type": "string", "minLength": 1},
        "lyric_offset_ms": {"type": "number"}, "calibration_path": {"type": "string", "minLength": 1},
        "lyrics_url": {"type": "string", "minLength": 1}, "ruby_url": {"type": "string", "minLength": 1},
        "lines": {"type": "array", "minItems": 1, "items": {
            "type": "object", "additionalProperties": False,
            "required": ["id", "text", "start_ms", "end_ms", "words", "romaji_words", "ruby"],
            "properties": {
                "id": {"type": "integer", "minimum": 0}, "text": {"type": "string", "minLength": 1},
                "start_ms": {"type": "number", "minimum": 0}, "end_ms": {"type": "number", "minimum": 0},
                "words": {"type": "array", "minItems": 1, "items": TIMED_WORD},
                "romaji_words": {"type": "array", "minItems": 1, "items": TIMED_WORD},
                "ruby": {"type": "array", "items": {
                    "type": "object", "additionalProperties": False,
                    "required": ["start", "end", "base", "reading"],
                    "properties": {"start": {"type": "integer", "minimum": 0},
                                   "end": {"type": "integer", "minimum": 1},
                                   "base": {"type": "string", "minLength": 1},
                                   "reading": {"type": "string", "minLength": 1}},
                }},
            },
        }},
    },
}
TEACHING_SCHEMA = {
    "type": "object", "additionalProperties": False,
    "required": ["version", "entries"],
    "properties": {"version": {"const": 1}, "entries": {"type": "array", "minItems": 1, "items": {
        "type": "object", "additionalProperties": False,
        "required": ["text", "translation", "vocabulary", "expressions"],
        "properties": {
            "text": {"type": "string", "minLength": 1}, "translation": {"type": "string", "minLength": 1},
            "vocabulary": {"type": "array", "minItems": 1, "items": {
                "type": "object", "additionalProperties": False,
                "required": ["surface", "reading", "lemma", "pos", "conjugation", "meaning", "level"],
                "properties": {key: {"type": "string", "minLength": 1} for key in
                               ["surface", "reading", "lemma", "pos", "meaning"]}
                | {"conjugation": {"type": "string"}, "level": {"type": ["string", "null"]}},
            }},
            "expressions": {"type": "array", "minItems": 1, "items": {
                "type": "object", "additionalProperties": False,
                "required": ["surface", "reading", "explanation"],
                "properties": {key: {"type": "string", "minLength": 1} for key in
                               ["surface", "reading", "explanation"]},
            }},
        },
    }}},
}


def canonical(text):
    return "".join(character for character in unicodedata.normalize("NFC", text) if not character.isspace())


def file_hash(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_json(path, schema):
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    errors = list(Draft202012Validator(schema).iter_errors(data))
    if errors:
        error = errors[0]
        location = "/".join(str(part) for part in error.path) or "根对象"
        raise ValueError(f"JSON 字段无效：{path} / {location} / {error.message}")
    return data


def probe(path):
    result = subprocess.run(["ffprobe", "-v", "error", "-show_format", "-show_streams", "-of", "json", str(path)],
                            check=True, capture_output=True, text=True)
    return json.loads(result.stdout)


def validate_inputs(project, teaching):
    line_keys = {canonical(line["text"]) for line in project["lines"]}
    entries = {}
    for entry in teaching["entries"]:
        key = canonical(entry["text"])
        if key in entries:
            raise ValueError(f"教学内容重复：{entry['text']}")
        if not entry["translation"].strip():
            raise ValueError(f"缺少句子翻译：{entry['text']}")
        for item in entry["vocabulary"] + entry["expressions"]:
            if canonical(item["surface"]) not in key:
                raise ValueError(f"教学表达未出现在歌词中：{item['surface']} / {entry['text']}")
            if any(isinstance(value, str) and not value.strip() for field, value in item.items()
                   if field != "conjugation"):
                raise ValueError(f"教学内容包含空白字段：{item['surface']}")
            if item.get("conjugation") and not item["conjugation"].strip():
                raise ValueError(f"活用说明包含空白字符：{item['surface']}")
        entries[key] = entry
    if set(entries) != line_keys:
        raise ValueError(f"教学内容与完整演唱歌词不匹配：缺少 {line_keys - set(entries)}，多出 {set(entries) - line_keys}")
    previous_start = -math.inf
    ids = set()
    for line in project["lines"]:
        text = line["text"]
        if not text.strip() or line["start_ms"] <= previous_start or line["end_ms"] <= line["start_ms"]:
            raise ValueError(f"歌词时间或内容无效：{line['text']}")
        if line["id"] in ids:
            raise ValueError(f"歌词 id 重复：{line['id']}")
        ids.add(line["id"])
        previous_start = line["start_ms"]
        if "".join(word["text"] for word in line["words"]) != text:
            raise ValueError(f"逐字歌词内容与原文不一致：{line['text']}")
        for key in ["words", "romaji_words"]:
            previous_word_start = -math.inf
            for word in line[key]:
                if (word["end_ms"] < word["start_ms"]
                        or (word["end_ms"] == word["start_ms"] and word["text"].strip())
                        or word["start_ms"] < previous_word_start):
                    raise ValueError(f"逐字时间无效：{key} / {word}")
                if word["start_ms"] < line["start_ms"] - 500 or word["end_ms"] > project["duration_ms"] + 1000:
                    raise ValueError(f"逐字时间超出歌曲范围：{word}")
                previous_word_start = word["start_ms"]
        previous_ruby_end = 0
        for ruby in line["ruby"]:
            start, end = ruby["start"], ruby["end"]
            if start < previous_ruby_end or end <= start or end > len(text) or text[start:end] != ruby["base"]:
                raise ValueError(f"Ruby 范围与歌词不一致：{ruby} / {line['text']}")
            previous_ruby_end = end
    audio_path = Path(project["audio_path"])
    background_path = Path(project["background_path"])
    if not audio_path.is_absolute() or not background_path.is_absolute():
        raise ValueError("音频和背景路径必须使用绝对路径")
    audio_probe = probe(audio_path)
    if not any(stream["codec_type"] == "audio" for stream in audio_probe["streams"]):
        raise ValueError("源文件缺少音频")
    if abs(float(audio_probe["format"]["duration"]) * 1000 - project["duration_ms"]) > 100:
        raise ValueError("project.duration_ms 与音频实际时长不一致")
    with Image.open(background_path) as background:
        background.verify()
    return entries


@dataclass
class TextRow:
    text: str
    font: object
    color: tuple
    height: int


@dataclass
class TimedPaint:
    x: int
    y: int
    mask: object
    start_ms: float
    end_ms: float
    progress_start: float = 0
    progress_end: float = 1
    key: str = ""
    baseline: float = 0


@dataclass
class Glyph:
    x: int
    y: int
    text: str
    mask: object
    baseline: int


class Renderer:
    def __init__(self, project, teaching, layout, font_path, font_index=0, accent=None, edits=None, natural=False, base=None):
        project = aligned_project(project)
        self.project = project
        self.entries = validate_inputs(project, teaching)
        self.layout = layout
        self.width, self.height = (1920, 1080) if layout == "landscape" else (1080, 1920)
        self.margin = 72 if layout == "landscape" else 64
        self.content_width = self.width - self.margin * 2
        self.columns = 3 if layout == "landscape" else 2
        self.font_path = str(Path(font_path).resolve(strict=True))
        self.font_index = font_index
        self.edits = validate_edits(edits if edits is not None else empty_edits())
        validate_component_ids(self.edits, project, self.entries, canonical)
        self.canonical = canonical
        accent = cover_accent(project["background_path"]) if accent is None else accent
        if not re.fullmatch(r"#[0-9a-fA-F]{6}", accent):
            raise ValueError("accent 必须使用 #RRGGBB 格式")
        self.accent = tuple(int(accent[index:index + 2], 16) for index in [1, 3, 5])
        self.accent_hex = accent
        self.white, self.muted = (241, 243, 246), (180, 190, 203)
        self.fonts = base.fonts if base is not None else {}
        self.starts = [line["start_ms"] for line in project["lines"]]
        if base is None:
            with Image.open(project["background_path"]) as source:
                background = ImageOps.fit(source.convert("RGB"), (self.width, self.height), method=Image.Resampling.LANCZOS)
            background = background.filter(ImageFilter.GaussianBlur(36))
            self.background = Image.blend(background, Image.new("RGB", background.size, (5, 13, 23)), 0.78)
        else:
            self.background = base.background
        self._verify_glyphs(teaching)
        baseline = copy.copy(self)
        baseline.edits = empty_edits()
        self.default_arrangements = {}
        self.default_nodes = []
        if base is None:
            for index, line in enumerate(project["lines"]):
                scene = compose(baseline, index)
                self.default_arrangements[str(line["id"])] = self.arrangement(scene)
                self.default_nodes.extend(scene["nodes"])
        else:
            self.default_arrangements = base.default_arrangements
            self.default_nodes = base.default_nodes
        self.scenes = []
        for index, line in enumerate(project["lines"]):
            fixed = None
            if not natural and not self.edits["automatic"][layout]:
                fixed = self.edits["arrangements"][layout].get(str(line["id"]), self.default_arrangements[str(line["id"])])
                fixed = copy.deepcopy(fixed)
                known = {node["id"] for node in fixed["nodes"]}
                visibility = Composition(self, f"line:{line['id']}")
                additions = [node for node in self.default_arrangements[str(line["id"])]["nodes"]
                             if node["id"] not in known and visibility.visible(node["id"])]
                fixed["nodes"].extend(additions)
                fixed["pages"] = max([fixed["pages"]] + [node["page"] + 1 for node in additions if node["page"] is not None])
            self.scenes.append(compose(self, index, fixed, omit_hidden=natural or self.edits["automatic"][layout]))
        self.preview_nodes(0)

    @staticmethod
    def arrangement(scene):
        return {"pages": len(scene["pages"]), "nodes": [
            {"id": node["id"], "bounds": node["bounds"], "page": node["page"], "baseline": node["baseline"]} for node in scene["nodes"]]}

    def catalog(self):
        unique = {}
        for scene in self.scenes:
            for node in scene["nodes"]:
                unique.setdefault(node["id"], {**node, "line_id": scene["line_id"] if node["id"].startswith("line:") else None})
            c = scene["composition"]
            for node in self.default_nodes:
                if node["id"].startswith(f"line:{scene['line_id']}/") and c.exists(node["id"]) and not c.visible(node["id"]) and node["id"] not in unique:
                    dx, dy = c.offset(node["id"])
                    unique[node["id"]] = {**node, "text": c.text(node["id"], node["text"]) if node["text"] is not None else None,
                                          "bounds": [value + (dx if i % 2 == 0 else dy) for i, value in enumerate(node["bounds"])],
                                          "hidden": True, "line_id": scene["line_id"]}
        for node in self.preview_nodes(0)["nodes"]:
            if not node["id"].startswith("line:"):
                unique.setdefault(node["id"], {**node, "line_id": None})
        for line in self.project["lines"]:
            start = 0
            for number, word in enumerate(line["words"]):
                key = f"line:{line['id']}/lyrics/word:{number}"
                if key in unique:
                    unique[key]["linked"] = [f"line:{line['id']}/lyrics/ruby:{index}" for index, ruby in enumerate(line["ruby"])
                                               if start < ruby["end"] and start + len(word["text"]) > ruby["start"]
                                               and f"line:{line['id']}/lyrics/ruby:{index}" in unique]
                start += len(word["text"])
        return list(unique.values())

    def font(self, size):
        if size not in self.fonts:
            self.fonts[size] = ImageFont.truetype(self.font_path, size=size, index=self.font_index)
        return self.fonts[size]

    def _verify_glyphs(self, teaching):
        font = TTFont(self.font_path, fontNumber=self.font_index, lazy=True)
        cmap = font.getBestCmap()
        content = json.dumps(teaching, ensure_ascii=False) + self.project["title"] + self.project["artist"]
        content += "".join(line["text"] + "".join(word["text"] for word in line["romaji_words"])
                           + "".join(ruby["reading"] for ruby in line["ruby"]) for line in self.project["lines"])
        content += "单词释义表达讲解原形活用第页歌曲学习当前时间总时长上一句下一句"
        content += "".join(self.edits["text"].values())
        missing = sorted({character for character in content if not character.isspace() and ord(character) not in cmap})
        font.close()
        if missing:
            raise ValueError(f"指定字体缺少字符：{''.join(missing)}")

    def wrap(self, text, font, width):
        if not text.strip():
            return []
        lines = []
        for paragraph in text.splitlines():
            line = ""
            for character in paragraph:
                if font.getlength(character) > width:
                    raise ValueError(f"单个字符超出排版宽度：{character}")
                if line and font.getlength(line + character) > width:
                    lines.append(line.rstrip())
                    line = character.lstrip()
                else:
                    line += character
            if line.strip():
                lines.append(line.rstrip())
        return lines

    def text_rows(self, text, size, color, width, spacing=8):
        font = self.font(size)
        return [TextRow(line, font, color, size + spacing) for line in self.wrap(text, font, width)]

    def _paint(self, target, text, x, y, font, color):
        ImageDraw.Draw(target).text((round(x), round(y)), text, font=font, fill=color, anchor="lt")

    def _raster(self, text, font):
        core, offset = font.getmask2(text, mode="L", anchor="ls")
        mask = Image.frombytes("L", core.size, bytes(core))
        bounds = mask.getbbox()
        if bounds is None:
            raise ValueError(f"文字字形为空：{text}")
        return mask.crop(bounds), offset[0] + bounds[0], offset[1] + bounds[1]

    def _timed_paint(self, target, text, x, y, font, start_ms, end_ms, key):
        if not text.strip():
            return None
        mask, left, top = self._raster(text, font)
        target.paste(self.white, (round(x) + left, round(y) + top), mask)
        return TimedPaint(round(x) + left, round(y) + top, mask, start_ms, end_ms, key=key, baseline=y)

    def _original(self, image, line, top):
        text = line["text"]
        ruby_by_start = {ruby["start"]: ruby for ruby in line["ruby"]}
        atoms = []
        position = 0
        while position < len(text):
            ruby = ruby_by_start.get(position)
            end = ruby["end"] if ruby else position + 1
            atoms.append((position, end, text[position:end], ruby["reading"] if ruby else ""))
            position = end
        preferred, minimum, max_rows = (60, 48, 2) if self.layout == "landscape" else (68, 48, 3)
        chosen = None
        for size in range(preferred, minimum - 1, -2):
            base_font, ruby_font = self.font(size), self.font(round(size * 0.43))
            rows = [[]]
            row_width = 0
            for atom in atoms:
                atom_width = max(base_font.getlength(atom[2]), ruby_font.getlength(atom[3])) + (6 if atom[3] else 0)
                if atom_width > self.content_width:
                    raise ValueError(f"Ruby 字段超出画面宽度：{atom[2]} / {atom[3]}")
                if rows[-1] and row_width + atom_width > self.content_width:
                    rows.append([])
                    row_width = 0
                rows[-1].append((*atom, atom_width))
                row_width += atom_width
            if len(rows) <= max_rows and (chosen is None or len(rows) < len(chosen[0])):
                chosen = rows, base_font, ruby_font, size
                if len(rows) == 1:
                    break
        if chosen is None:
            raise ValueError(f"歌词无法在规定区域完整显示：{line['text']}")
        rows, base_font, ruby_font, size = chosen
        character_words = []
        for word in line["words"]:
            character_words.extend([word] * len(word["text"]))
        paints = []
        placements = {}
        original_rows = []
        row_height = size + ruby_font.size + 20
        for row_number, row in enumerate(rows):
            x = (self.width - sum(atom[4] for atom in row)) / 2
            y = top + row_number * row_height
            row_text = "".join(atom[2] for atom in row)
            baseline = round(y + ruby_font.size + 10 - base_font.getbbox(row_text, anchor="ls")[1])
            reading_text = "".join(atom[3] for atom in row)
            ruby_baseline = y - ruby_font.getbbox(reading_text, anchor="ls")[1]
            row_geometry = {"baseline": baseline, "font_size": size, "glyphs": []}
            for start, end, base, reading, atom_width in row:
                base_x = x + (atom_width - base_font.getlength(base)) / 2
                offset = base_x
                for index, character in enumerate(base, start):
                    origin_x = round(offset)
                    if character.isspace():
                        placements[index] = Glyph(origin_x, baseline, character, None, baseline)
                    else:
                        mask, left, glyph_top = self._raster(character, base_font)
                        glyph = Glyph(origin_x + left, baseline + glyph_top, character, mask, baseline)
                        image.paste(self.white, (glyph.x, glyph.y), mask)
                        placements[index] = glyph
                        row_geometry["glyphs"].append({"index": index, "text": character, "origin_x": origin_x,
                                                       "bounds": [glyph.x, glyph.y, glyph.x + mask.width,
                                                                  glyph.y + mask.height]})
                    offset += base_font.getlength(character)
                if reading:
                    paint = self._timed_paint(image, reading, x + (atom_width - ruby_font.getlength(reading)) / 2,
                                              ruby_baseline, ruby_font, character_words[start]["start_ms"], character_words[end - 1]["end_ms"],
                                              ruby_by_start[start]["_key"])
                    if paint:
                        paints.append(paint)
                x += atom_width
            original_rows.append(row_geometry)
        cursor = 0
        for word in line["words"]:
            fragments = []
            for position in range(cursor, cursor + len(word["text"])):
                placement = placements[position]
                if not fragments or fragments[-1][0].baseline != placement.baseline:
                    fragments.append([])
                fragments[-1].append(placement)
            total_advance = sum(base_font.getlength(glyph.text) for fragment in fragments for glyph in fragment)
            advance = 0
            for fragment in fragments:
                fragment_advance = sum(base_font.getlength(glyph.text) for glyph in fragment)
                visible = [glyph for glyph in fragment if glyph.mask is not None]
                if visible:
                    left, glyph_top = min(glyph.x for glyph in visible), min(glyph.y for glyph in visible)
                    right = max(glyph.x + glyph.mask.width for glyph in visible)
                    glyph_bottom = max(glyph.y + glyph.mask.height for glyph in visible)
                    mask = Image.new("L", (right - left, glyph_bottom - glyph_top))
                    for glyph in visible:
                        mask.paste(255, (glyph.x - left, glyph.y - glyph_top), glyph.mask)
                    paints.append(TimedPaint(left, glyph_top, mask, word["start_ms"], word["end_ms"],
                                             advance / total_advance, (advance + fragment_advance) / total_advance,
                                             word["_key"], baseline=fragment[0].baseline))
                advance += fragment_advance
            cursor += len(word["text"])
        return top + len(rows) * row_height, paints, original_rows

    def _roman(self, image, line, top):
        preferred, minimum, max_rows = (28, 23, 2) if self.layout == "landscape" else (32, 25, 4)
        chosen = None
        for size in range(preferred, minimum - 1, -2):
            font = self.font(size)
            rows = [[]]
            width = 0
            for word in line["romaji_words"]:
                word_width = font.getlength(word["text"])
                if word_width > self.content_width:
                    raise ValueError(f"罗马音字段超出画面宽度：{word['text']}")
                if rows[-1] and width + word_width > self.content_width:
                    rows.append([])
                    width = 0
                rows[-1].append((word, word_width))
                width += word_width
            if len(rows) <= max_rows:
                chosen = rows, font, size
                break
        if chosen is None:
            raise ValueError(f"罗马音无法在规定区域完整显示：{line['text']}")
        rows, font, size = chosen
        paints = []
        for row_number, row in enumerate(rows):
            x = (self.width - sum(width for _, width in row)) / 2
            y = top + row_number * (size + 12)
            baseline = y - font.getbbox("".join(word["text"] for word, _ in row), anchor="ls")[1]
            for word, width in row:
                paint = self._timed_paint(image, word["text"], x, baseline, font, word["start_ms"], word["end_ms"], word["_key"])
                if paint:
                    paints.append(paint)
                x += width
        return top + len(rows) * (size + 12), paints

    def _pack_cards(self, cards, available):
        pages = []
        page = []
        current_height = 0
        for index in range(0, len(cards), self.columns):
            row = cards[index:index + self.columns]
            row_height = max(card["height"] for card in row)
            if row_height > available:
                raise ValueError(f"单词卡片内容超出画面高度：{row[0]['item']['surface']}")
            if page and current_height + 18 + row_height > available:
                pages.append(page)
                page, current_height = [], 0
            if page:
                current_height += 18
            page.append((row, row_height))
            current_height += row_height
        if page:
            pages.append(page)
        return pages or [[]]

    def page_index(self, scene, time_ms):
        if time_ms < scene["page_start_ms"]:
            return 0
        duration = scene["end_ms"] - scene["page_start_ms"]
        return min(len(scene["pages"]) - 1, int((time_ms - scene["page_start_ms"]) * len(scene["pages"]) / duration))

    def frame(self, time_ms):
        index = max(0, min(len(self.scenes) - 1, bisect.bisect_right(self.starts, time_ms) - 1))
        scene = self.scenes[index]
        frame = scene["pages"][self.page_index(scene, time_ms)].copy()
        for paint in scene["paints"]:
            if time_ms <= paint.start_ms:
                continue
            progress = (time_ms - paint.start_ms) / (paint.end_ms - paint.start_ms)
            fraction = max(0, min(1, (progress - paint.progress_start) / (paint.progress_end - paint.progress_start)))
            width = round(paint.mask.width * fraction)
            if width:
                frame.paste(self.accent, (paint.x, paint.y), paint.mask.crop((0, 0, width, paint.mask.height)))
        draw = ImageDraw.Draw(frame)
        c = scene["composition"]
        y = self.height - (37 if self.layout == "landscape" else 50)
        dx, dy = c.offset("progress")
        if c.visible("progress"):
            draw.rounded_rectangle((self.margin + dx, y + dy, self.width - self.margin + dx, y + 5 + dy), radius=2, fill=(75, 83, 93))
        progress = max(0, min(1, time_ms / self.project["duration_ms"]))
        if progress and c.visible("progress"):
            draw.rounded_rectangle((self.margin + dx, y + dy, self.margin + self.content_width * progress + dx, y + 5 + dy), radius=2, fill=self.accent)
        clock = f"{format_time(time_ms)}  /  {format_time(self.project['duration_ms'])}"
        clock_font = self.font(23 if self.layout == "landscape" else 28)
        if c.visible("clock"):
            dx, dy = c.offset("clock")
            self._paint(frame, c.text("clock", clock), self.margin + dx, y - clock_font.size - 16 + dy, clock_font, self.muted)
        return frame

    def preview_nodes(self, time_ms):
        index = max(0, min(len(self.scenes) - 1, bisect.bisect_right(self.starts, time_ms) - 1))
        scene = self.scenes[index]
        page = self.page_index(scene, time_ms)
        nodes = [node for node in scene["nodes"] if node["page"] is None or node["page"] == page]
        c = scene["composition"]
        y = self.height - (37 if self.layout == "landscape" else 50)
        before = len(c.nodes)
        if c.exists("progress"):
            c.add("progress", "播放进度条", [self.margin, y, self.width - self.margin, y + 5])
        if c.exists("clock"):
            clock = c.text("clock", f"{format_time(time_ms)}  /  {format_time(self.project['duration_ms'])}")
            font = self.font(23 if self.layout == "landscape" else 28)
            c.add("clock", "播放时间", [self.margin, y - font.size - 16, self.margin + font.getlength(clock), y - 16], clock)
        nodes += c.nodes[before:]
        del c.nodes[before:]
        return {"line_id": scene["line_id"], "page": page, "page_count": len(scene["pages"]), "nodes": nodes}

    def evidence(self):
        result = []
        for scene in self.scenes:
            pages = len(scene["pages"])
            duration = scene["end_ms"] - scene["page_start_ms"]
            result.append({"line_id": scene["line_id"], "text": scene["text"], "page_count": pages,
                           "original_rows": scene["original_rows"],
                           "start_ms": scene["start_ms"], "end_ms": scene["end_ms"],
                           "pages": [{"page": index, "start_ms": scene["page_start_ms"] + duration * index / pages,
                                      "end_ms": scene["page_start_ms"] + duration * (index + 1) / pages}
                                     for index in range(pages)], "content": scene["inventory"]})
        return result


def format_time(time_ms):
    seconds = max(0, math.floor(time_ms / 1000))
    return f"{seconds // 60:02d}:{seconds % 60:02d}"


def audio_filter(start_ms, duration_ms, offset_ms):
    audio_start = max(0, start_ms - offset_ms) / 1000
    audio_end = max(0, start_ms + duration_ms - offset_ms) / 1000
    delay = max(0, offset_ms - start_ms)
    if audio_end <= audio_start:
        raise ValueError("所选区间没有源音频")
    return (f"atrim=start={audio_start:.9f}:end={audio_end:.9f},asetpts=PTS-STARTPTS,"
            f"adelay={delay:.6f}:all=1,apad,atrim=duration={duration_ms / 1000:.9f}")


def make_parser():
    parser = argparse.ArgumentParser(description="将完整歌词和教学内容渲染为逐字同步教学视频")
    parser.add_argument("--project", required=True, type=Path)
    parser.add_argument("--teaching", required=True, type=Path)
    parser.add_argument("--layout", choices=["landscape", "portrait"], required=True)
    parser.add_argument("--font", required=True, type=Path)
    parser.add_argument("--font-index", type=int, default=0)
    parser.add_argument("--accent", help="省略时从歌曲封面自动提取")
    parser.add_argument("--edits", type=Path)
    parser.add_argument("--audio-offset-ms", type=float, default=0,
                        help="正数表示音频晚于歌词时间播放，默认保持源音频时间")
    parser.add_argument("--start-ms", type=float, default=0)
    parser.add_argument("--end-ms", type=float)
    parser.add_argument("--output", required=True, type=Path)
    return parser


def main():
    args = make_parser().parse_args()
    project_path, teaching_path = args.project.resolve(strict=True), args.teaching.resolve(strict=True)
    input_hashes = {"project": file_hash(project_path), "teaching": file_hash(teaching_path)}
    project, teaching = read_json(project_path, PROJECT_SCHEMA), read_json(teaching_path, TEACHING_SCHEMA)
    validate_calibration(project)
    input_paths = {"project": project_path, "teaching": teaching_path,
                   "audio": Path(project["audio_path"]), "background": Path(project["background_path"]),
                   "font": args.font.resolve(strict=True), "calibration": Path(project["calibration_path"]).resolve(strict=True)}
    for name in ["audio", "background", "font", "calibration"]:
        input_hashes[name] = file_hash(input_paths[name])
    edits = read_json(args.edits, EDIT_SCHEMA) if args.edits else empty_edits()
    if args.edits:
        input_paths["edits"] = args.edits.resolve(strict=True)
        input_hashes["edits"] = file_hash(input_paths["edits"])
    renderer = Renderer(project, teaching, args.layout, args.font, args.font_index, args.accent, edits)
    end_ms = project["duration_ms"] if args.end_ms is None else args.end_ms
    if not 0 <= args.start_ms < end_ms <= project["duration_ms"]:
        raise ValueError("视频区间必须位于歌曲时长范围内")
    if args.start_ms == 0 and end_ms == project["duration_ms"] and args.audio_offset_ms != 0:
        raise ValueError("完整歌曲导出必须保持源音频时间，audio-offset-ms 必须为零")
    frames = math.ceil((end_ms - args.start_ms) * 30 / 1000)
    duration_ms = frames * 1000 / 30
    output = args.output.resolve()
    if output.suffix.lower() != ".mp4":
        raise ValueError("输出文件必须使用 .mp4 扩展名")
    if output.exists():
        raise FileExistsError(f"输出文件已经存在：{output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    log_path = output.with_suffix(".encode.log")
    filter_body = audio_filter(args.start_ms, duration_ms, args.audio_offset_ms)
    command = ["ffmpeg", "-hide_banner", "-loglevel", "error", "-nostdin", "-f", "rawvideo", "-pixel_format", "rgb24",
               "-video_size", f"{renderer.width}x{renderer.height}", "-framerate", "30", "-i", "pipe:0",
               "-i", str(project["audio_path"]), "-filter_complex", f"[1:a:0]{filter_body}[audio]",
               "-map", "0:v:0", "-map", "[audio]", "-c:v", "libx264", "-preset", "veryfast", "-crf", "18",
               "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-ac", "2",
               "-frames:v", str(frames), "-t", f"{duration_ms / 1000:.9f}", "-movflags", "+faststart",
               "-metadata", f"title={project['title']} — {project['artist']}", "-n", str(output)]
    with log_path.open("wb") as log:
        process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=log)
        for number in range(frames):
            if process.poll() is not None:
                raise RuntimeError(f"FFmpeg 提前结束，请检查 {log_path}")
            process.stdin.write(renderer.frame(args.start_ms + number * 1000 / 30).tobytes())
            if number % 300 == 0:
                print(json.dumps({"rendered_frames": number, "total_frames": frames, "output": str(output)}), flush=True)
        process.stdin.close()
        if process.wait() != 0:
            raise RuntimeError(f"FFmpeg 编码失败，请检查 {log_path}")
    if log_path.stat().st_size:
        raise RuntimeError(f"FFmpeg 返回错误，请检查 {log_path}")
    for name, path in input_paths.items():
        if file_hash(path) != input_hashes[name]:
            raise ValueError(f"编码期间渲染输入发生变化：{path}")
    manifest = {
        "version": 1, "output": str(output), "output_sha256": file_hash(output),
        "project": str(project_path), "project_sha256": input_hashes["project"],
        "teaching": str(teaching_path), "teaching_sha256": input_hashes["teaching"],
        "audio_sha256": input_hashes["audio"], "background_sha256": input_hashes["background"],
        "calibration": str(input_paths["calibration"]), "calibration_sha256": input_hashes["calibration"],
        "lyric_offset_ms": project["lyric_offset_ms"],
        "font": renderer.font_path, "font_sha256": input_hashes["font"], "font_index": args.font_index,
        "layout": args.layout, "accent": renderer.accent_hex, "width": renderer.width, "height": renderer.height,
        "edits": str(input_paths["edits"]) if args.edits else None,
        "edits_sha256": input_hashes.get("edits"),
        "fps": 30, "frames": frames, "start_ms": args.start_ms, "duration_ms": duration_ms,
        "audio_offset_ms": args.audio_offset_ms, "scenes": renderer.evidence(), "ffmpeg_command": command,
    }
    manifest_path = output.with_suffix(".render.json")
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(output), "manifest": str(manifest_path), "frames": frames,
                      "duration_ms": duration_ms}, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
