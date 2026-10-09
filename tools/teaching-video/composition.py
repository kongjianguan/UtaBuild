import copy
import colorsys

from jsonschema import Draft202012Validator
from PIL import Image

from glass import draw_liquid_glass


EDIT_SCHEMA = {
    "type": "object", "additionalProperties": False,
    "required": ["version", "text", "deleted", "hidden", "positions", "automatic", "arrangements"],
    "properties": {
        "version": {"const": 2},
        "text": {"type": "object", "additionalProperties": {"type": "string", "minLength": 1}},
        "deleted": {"type": "array", "uniqueItems": True, "items": {"type": "string"}},
        "hidden": {"type": "array", "uniqueItems": True, "items": {"type": "string"}},
        "automatic": {"type": "object", "additionalProperties": False,
                      "required": ["landscape", "portrait"],
                      "properties": {layout: {"type": "boolean"} for layout in ["landscape", "portrait"]}},
        "arrangements": {"type": "object", "additionalProperties": False,
                         "required": ["landscape", "portrait"],
                         "properties": {layout: {"type": "object", "additionalProperties": {
                             "type": "object", "additionalProperties": False, "required": ["pages", "nodes"],
                             "properties": {"pages": {"type": "integer", "minimum": 1}, "nodes": {
                                 "type": "array", "items": {"type": "object", "additionalProperties": False,
                                     "required": ["id", "bounds", "page", "baseline"], "properties": {
                                         "id": {"type": "string"},
                                         "bounds": {"type": "array", "minItems": 4, "maxItems": 4,
                                                    "items": {"type": "number"}},
                                         "page": {"type": ["integer", "null"], "minimum": 0},
                                         "baseline": {"type": ["number", "null"]}}}}}}}
                                         for layout in ["landscape", "portrait"]}},
        "positions": {"type": "object", "additionalProperties": False,
                      "required": ["landscape", "portrait"],
                      "properties": {layout: {"type": "object", "additionalProperties": {
                          "type": "array", "minItems": 2, "maxItems": 2,
                          "items": {"type": "number", "minimum": -1920, "maximum": 1920}}}
                                     for layout in ["landscape", "portrait"]}},
    },
}


def empty_edits():
    return {"version": 2, "text": {}, "deleted": [], "hidden": [],
            "positions": {"landscape": {}, "portrait": {}},
            "automatic": {"landscape": False, "portrait": False},
            "arrangements": {"landscape": {}, "portrait": {}}}


def validate_edits(edits):
    Draft202012Validator(EDIT_SCHEMA).validate(edits)
    if any(not value.strip() for value in edits["text"].values()):
        raise ValueError("文本不能为空白；请使用删除组件操作")
    if any(("/lyrics/" in key or "/roman/" in key) and any(character in value for character in "\r\n")
           for key, value in edits["text"].items()):
        raise ValueError("歌词片段、Ruby 与罗马音使用自动换行，请保持片段文字为单行")
    return edits


def cover_accent(path):
    with Image.open(path) as source:
        thumbnail = source.convert("RGB")
        thumbnail.thumbnail((192, 192))
        palette = thumbnail.quantize(colors=12, method=Image.Quantize.MEDIANCUT)
    colors = palette.getpalette()
    candidates = []
    for count, index in palette.getcolors():
        rgb = colors[index * 3:index * 3 + 3]
        hue, lightness, saturation = colorsys.rgb_to_hls(*(value / 255 for value in rgb))
        candidates.append((count * (0.15 + saturation) * (0.2 + lightness), hue, lightness, saturation))
    _, hue, lightness, saturation = max(candidates)
    # 保留封面的色相，提高深色画面上的辨识度。
    for value in range(round(max(0.68, lightness) * 100), 101):
        rgb = tuple(round(channel * 255) for channel in colorsys.hls_to_rgb(hue, value / 100, min(0.78, saturation)))
        if (luminance(rgb) + 0.05) / (luminance((61, 66, 74)) + 0.05) >= 4.5:
            return "#" + "".join(f"{channel:02x}" for channel in rgb)
    raise ValueError("封面颜色无法满足文字对比度要求")


def luminance(rgb):
    linear = [value / 255 / 12.92 if value / 255 <= 0.04045 else ((value / 255 + 0.055) / 1.055) ** 2.4
              for value in rgb]
    return sum(value * weight for value, weight in zip(linear, [0.2126, 0.7152, 0.0722]))


def union(bounds):
    return [min(b[0] for b in bounds), min(b[1] for b in bounds),
            max(b[2] for b in bounds), max(b[3] for b in bounds)]


def validate_component_ids(edits, project, entries, canonical):
    components = {"background", "title", "artist", "clock", "progress"}
    texts = {"title", "artist", "clock"}
    for index, line in enumerate(project["lines"]):
        prefix = f"line:{line['id']}"
        components.update(f"{prefix}/{kind}" for kind in ["lyrics", "roman", "vocabulary", "expressions"])
        fields = ["translation", "page"]
        if index > 0:
            fields.append("previous")
        if index + 1 < len(project["lines"]):
            fields.append("next")
        texts.update(f"{prefix}/{field}" for field in fields)
        for kind, source, item_kind in [("lyrics", "words", "word"), ("lyrics", "ruby", "ruby"),
                                        ("roman", "romaji_words", "word")]:
            texts.update(f"{prefix}/{kind}/{item_kind}:{number}" for number in range(len(line[source])))
        entry = entries[canonical(line["text"])]
        for kind in ["vocabulary", "expressions"]:
            texts.add(f"{prefix}/{kind}/heading")
            for number in range(len(entry[kind])):
                key = f"{prefix}/{kind}/card:{number}"
                components.add(key)
                names = ["surface", "reading", "detail", "conjugation", "meaning"] if kind == "vocabulary" else ["surface", "reading", "explanation"]
                texts.update(f"{key}/{field}" for field in names)
    components.update(texts)
    unknown = (set(edits["text"]) - texts) | ((set(edits["deleted"]) | set(edits["hidden"])) - components)
    for positions in edits["positions"].values():
        unknown.update(set(positions) - components)
    line_ids = {str(line["id"]) for line in project["lines"]}
    for arrangements in edits["arrangements"].values():
        if set(arrangements) - line_ids:
            raise ValueError("排版记录包含无效歌词场景")
        for line_id, arrangement in arrangements.items():
            pairs = set()
            for node in arrangement["nodes"]:
                unknown.update({node["id"]} - components)
                if node["id"].startswith("line:") and not node["id"].startswith(f"line:{line_id}/"):
                    raise ValueError("排版记录的组件不属于指定歌词场景")
                pair = (node["id"], node["page"])
                if pair in pairs or (node["page"] is not None and node["page"] >= arrangement["pages"]):
                    raise ValueError("排版记录包含重复组件或无效分页")
                pairs.add(pair)
                x, y, right, bottom = node["bounds"]
                if not 0 <= x <= right <= 1920 or not 0 <= y <= bottom <= 1920:
                    raise ValueError("排版记录的组件边界无效")
    if unknown:
        raise ValueError(f"编辑文件包含无效组件标识：{', '.join(sorted(unknown))}")


class Composition:
    def __init__(self, renderer, prefix, arrangement=None, omit_hidden=False):
        self.r = renderer
        self.prefix = prefix
        self.nodes = []
        self.paints = []
        self.arrangement = arrangement
        self.omit_hidden = omit_hidden
        self.layout_offsets = {}

    def slot(self, key, page=None):
        if self.arrangement:
            return next((node for node in self.arrangement["nodes"]
                         if node["id"] == key and node["page"] == page), None)
        return None

    def exists(self, key):
        removed = self.r.edits["deleted"] + (self.r.edits["hidden"] if self.omit_hidden else [])
        return not any(key == item or key.startswith(item + "/") for item in removed)

    def visible(self, key):
        return self.exists(key) and not any(key == item or key.startswith(item + "/") for item in self.r.edits["hidden"])

    def text(self, key, value):
        return self.r.edits["text"].get(key, value)

    def offset(self, key):
        offsets = self.r.edits["positions"][self.r.layout]
        parts = key.split("/")
        values = [offsets.get("/".join(parts[:n]), [0, 0]) for n in range(1, len(parts) + 1)]
        values += [self.layout_offsets.get("/".join(parts[:n]), [0, 0]) for n in range(1, len(parts) + 1)]
        return [round(sum(v[i] for v in values)) for i in [0, 1]]

    def add(self, key, label, bounds, text=None, parent=None, page=None, baseline=None):
        dx, dy = self.offset(key)
        bounds = [round(bounds[0] + dx), round(bounds[1] + dy), round(bounds[2] + dx), round(bounds[3] + dy)]
        if key != "background" and (bounds[0] < 0 or bounds[1] < 0 or bounds[2] > self.r.width or bounds[3] > self.r.height):
            raise ValueError(f"组件超出画面：{label}，请调整位置或减少文字长度")
        node = {"id": key, "label": label, "bounds": bounds, "text": text, "parent": parent, "page": page,
                "hidden": not self.visible(key), "baseline": baseline}
        self.nodes.append(node)
        return node

    def rows(self, key, label, value, size, color, width):
        if not self.exists(key):
            return None
        text = self.text(key, value)
        rows = self.r.text_rows(text, size, color, width)
        return {"id": key, "label": label, "text": text, "rows": rows,
                "height": sum(row.height for row in rows)}

    def draw_rows(self, image, item, x, y, parent=None, center=False, page=None):
        key = item["id"]
        dx, dy = self.offset(key)
        bounds = []
        top = y
        slot = self.slot(key, page)
        if slot:
            top = slot["bounds"][1]
        for row in item["rows"]:
            left = (self.r.width - row.font.getlength(row.text)) / 2 if center else x
            if slot:
                left = ((slot["bounds"][0] + slot["bounds"][2] - row.font.getlength(row.text)) / 2
                        if center else slot["bounds"][0])
            if self.visible(key):
                self.r._paint(image, row.text, left + dx, top + dy, row.font, row.color)
            bounds.append([left, top, left + row.font.getlength(row.text), top + row.height])
            top += row.height
        self.add(key, item["label"], union(bounds), item["text"], parent, page)
        return top

    def timed(self, image, paints, values, parent):
        by_key = {}
        for key in {paint.key for paint in paints}:
            slot = self.slot(key)
            if slot:
                actual = union([[p.x, p.y, p.x + p.mask.width, p.y + p.mask.height] for p in paints if p.key == key])
                top = (slot["baseline"] - min(p.baseline for p in paints if p.key == key)
                       if slot["baseline"] is not None else slot["bounds"][1] - actual[1])
                self.layout_offsets[key] = [slot["bounds"][0] - actual[0], top]
        for paint in paints:
            key = paint.key
            if not self.exists(key):
                continue
            dx, dy = self.offset(key)
            paint.x += dx
            paint.y += dy
            paint.baseline += dy
            if self.visible(key):
                image.paste(self.r.white, (paint.x, paint.y), paint.mask)
                self.paints.append(paint)
            by_key.setdefault(key, []).append([paint.x - dx, paint.y - dy,
                                               paint.x - dx + paint.mask.width, paint.y - dy + paint.mask.height])
        for key, bounds in by_key.items():
            self.add(key, values[key][0], union(bounds), values[key][1], parent,
                     baseline=min(p.baseline for p in paints if p.key == key))
        return by_key

    def group(self, key, label, children, page=None):
        if children:
            # 子组件已经包含父级位移，容器使用相同的实际边界。
            self.nodes.append({"id": key, "label": label, "bounds": union([n["bounds"] for n in children]),
                               "text": None, "parent": None, "page": page, "hidden": not self.visible(key), "baseline": None})


def edited_line(c, source):
    line = copy.deepcopy(source)
    prefix = c.prefix
    words, boundaries = [], {0: 0}
    old, new = 0, 0
    values = {}
    for index, word in enumerate(source["words"]):
        key = f"{prefix}/lyrics/word:{index}"
        text = c.text(key, word["text"]) if c.exists(key) else ""
        if text.strip() and word["end_ms"] <= word["start_ms"]:
            raise ValueError("没有演唱时长的空白片段不能修改为歌词文字")
        for position in range(len(word["text"]) + 1):
            boundaries[old + position] = new + round(position * len(text) / len(word["text"]))
        if text:
            words.append({**word, "text": text, "_key": key})
            values[key] = ("歌词文字", text)
        old += len(word["text"])
        new += len(text)
    line["words"] = words
    line["text"] = "".join(word["text"] for word in words)
    line["ruby"] = []
    for index, ruby in enumerate(source["ruby"]):
        key = f"{prefix}/lyrics/ruby:{index}"
        start, end = boundaries[ruby["start"]], boundaries[ruby["end"]]
        if c.exists(key) and end > start:
            reading = c.text(key, ruby["reading"])
            line["ruby"].append({**ruby, "start": start, "end": end, "base": line["text"][start:end],
                                 "reading": reading, "_key": key})
            values[key] = ("Ruby 读音", reading)
    line["romaji_words"] = []
    for index, word in enumerate(source["romaji_words"]):
        key = f"{prefix}/roman/word:{index}"
        if c.exists(key):
            text = c.text(key, word["text"])
            line["romaji_words"].append({**word, "text": text, "_key": key})
            values[key] = ("罗马音文字", text)
    return line, values


def compose(r, index, arrangement=None, omit_hidden=False):
    source = r.project["lines"][index]
    entry = r.entries[r.canonical(source["text"])]
    prefix = f"line:{source['id']}"
    c = Composition(r, prefix, arrangement, omit_hidden)
    line, values = edited_line(c, source)
    base = Image.new("RGB", (r.width, r.height), (5, 13, 23))
    if c.visible("background"):
        base.paste(r.background, tuple(c.offset("background")))
    background = base.copy()
    c.add("background", "封面背景", [0, 0, r.width, r.height]) if c.exists("background") else None
    top = 42 if r.layout == "landscape" else 55
    for key, label, value, size, color in [
        ("title", "歌曲名称", r.project["title"], 42 if r.layout == "landscape" else 46, r.white),
        ("artist", "歌手名称", r.project["artist"], 25 if r.layout == "landscape" else 29, r.muted)]:
        item = c.rows(key, label, value, size, color, r.content_width)
        if item:
            top = c.draw_rows(base, item, r.margin, top) + 10
    top += 12 if r.layout == "landscape" else 25
    original_rows, original_paints, roman_paints = [], [], []
    for kind, label in [("lyrics", "歌词与 Ruby"), ("roman", "罗马音")]:
        key = f"{prefix}/{kind}"
        if not c.exists(key) or not (line["text"].strip() if kind == "lyrics" else line["romaji_words"]):
            continue
        start = len(c.nodes)
        scratch = Image.new("RGB", base.size)
        if kind == "lyrics":
            top, original_paints, original_rows = r._original(scratch, line, top)
            paints = original_paints
        else:
            top, roman_paints = r._roman(scratch, line, top)
            paints = roman_paints
        c.timed(base, paints, values, key)
        c.group(key, label, c.nodes[start:])
        top += 12
    # 独立移动歌词文字时仍保留每个字的真实字体基线。
    if original_rows:
        adjusted = []
        cursor_words = []
        for word in line["words"]:
            cursor_words.extend([word["_key"]] * len(word["text"]))
        for row in original_rows:
            grouped = {}
            for glyph in row["glyphs"]:
                if not c.visible(cursor_words[glyph["index"]]):
                    continue
                dx, dy = c.offset(cursor_words[glyph["index"]])
                target = grouped.setdefault(dy, {"baseline": row["baseline"] + dy, "font_size": row["font_size"], "glyphs": []})
                target["glyphs"].append({**glyph, "origin_x": glyph["origin_x"] + dx,
                                          "bounds": [glyph["bounds"][i] + (dx if i % 2 == 0 else dy) for i in range(4)]})
            adjusted.extend(grouped.values())
        original_rows = adjusted
    for kind, label, value, size, color in [
        ("translation", "中文翻译", entry["translation"], 32 if r.layout == "landscape" else 40, r.white),
        ("previous", "上一句翻译", "", 20 if r.layout == "landscape" else 26, r.muted),
        ("next", "下一句翻译", "", 20 if r.layout == "landscape" else 26, r.muted)]:
        if kind != "translation":
            nearby = index + (-1 if kind == "previous" else 1)
            if not 0 <= nearby < len(r.project["lines"]):
                continue
            context = r.entries[r.canonical(r.project["lines"][nearby]["text"])]["translation"]
            value = ("上一句" if kind == "previous" else "下一句") + "  " + context
        item = c.rows(f"{prefix}/{kind}", label, value, size, color, r.content_width)
        if item:
            top = c.draw_rows(base, item, 0, top, center=True) + 8
    top += 20 if r.layout == "landscape" else 44
    gap = 20
    card_width = (r.content_width - gap * (r.columns - 1)) // r.columns

    def cards(kind, width):
        result = []
        if not c.exists(f"{prefix}/{kind}"):
            return result
        for number, item in enumerate(entry[kind]):
            key = f"{prefix}/{kind}/card:{number}"
            if not c.exists(key):
                continue
            if kind == "vocabulary":
                size = 24 if r.layout == "landscape" else 28
                detail = f"{item['pos']}  ·  原形 {item['lemma']}"
                if item["level"]:
                    detail = f"{item['level']}  ·  {detail}"
                fields = [("surface", "单词", item["surface"], size + 12, r.white),
                          ("reading", "单词读音", item["reading"], size, r.accent),
                          ("detail", "词性与原形", detail, size, r.muted),
                          ("conjugation", "活用说明", f"活用 {item['conjugation']}" if item["conjugation"] else "", size, r.muted),
                          ("meaning", "单词释义", item["meaning"], size + 2, r.white)]
            else:
                size = 27 if r.layout == "landscape" else 32
                fields = [("surface", "表达", item["surface"], size + 7, r.accent),
                          ("reading", "表达读音", item["reading"], size - 3, r.muted),
                          ("explanation", "表达讲解", item["explanation"], size, r.white)]
            rows = [c.rows(f"{key}/{field}", label, text, size, color, width - 44)
                    for field, label, text, size, color in fields if text or f"{key}/{field}" in r.edits["text"]]
            rows = [row for row in rows if row]
            if rows:
                result.append({"id": key, "item": item, "fields": rows,
                               "height": sum(row["height"] for row in rows) + 44})
        return result

    vocabulary = cards("vocabulary", card_width)
    expressions = cards("expressions", r.content_width)
    heading_size = 25 if r.layout == "landscape" else 31
    headings = {kind: c.rows(f"{prefix}/{kind}/heading", label, label, heading_size, r.accent, r.content_width)
                if content else None for kind, label, content in
                [("vocabulary", "单词释义", vocabulary), ("expressions", "表达讲解", expressions)]}
    footer = r.height - (85 if r.layout == "landscape" else 115)
    expression_height = max((card["height"] for card in expressions), default=0)
    expression_heading = headings["expressions"]["height"] + 12 if headings["expressions"] else 0
    vocabulary_heading = headings["vocabulary"]["height"] + 12 if headings["vocabulary"] else 0
    available = footer - top - vocabulary_heading - expression_height - expression_heading - (20 if expressions else 0)
    vocabulary_pages = r._pack_cards(vocabulary, available)
    vocabulary_height = max((sum(height for _, height in page) + max(0, len(page) - 1) * 18
                             for page in vocabulary_pages), default=0)
    expression_top = top + vocabulary_heading + vocabulary_height + (20 if vocabulary else 0)
    count = arrangement["pages"] if arrangement else max(len(vocabulary_pages), len(expressions), 1)
    start_ms = 0 if index == 0 else source["start_ms"]
    end_ms = r.project["lines"][index + 1]["start_ms"] if index + 1 < len(r.project["lines"]) else r.project["duration_ms"]
    if count > 1 and (end_ms - max(start_ms, source["start_ms"])) / count < 1000:
        raise ValueError(f"教学分页需要至少每页 1000 ms：{source['text']} / {count} 页")
    pages, inventory = [], []

    def draw_card(image, card, x, y, width, height, kind, page):
        key = card["id"]
        dx, dy = c.offset(key)
        node = c.add(key, "单词卡片" if kind == "vocabulary" else "表达卡片", [x, y, x + width, y + height],
                     parent=f"{prefix}/{kind}", page=page)
        material = draw_liquid_glass(image, background, node["bounds"]) if c.visible(key) else {}
        field_top = y + 22
        for field in card["fields"]:
            field_top = c.draw_rows(image, field, x + 22, field_top, key, page=page)
        if c.visible(key):
            inventory.append({"kind": "vocabulary" if kind == "vocabulary" else "expression", "page": page,
                              "surface": c.text(f"{key}/surface", card["item"]["surface"]),
                              "bounds": [x + dx, y + dy, x + width + dx, y + height + dy], **material})

    for page_index in range(count):
        page = base.copy()
        for kind, label, content in [("vocabulary", "单词区域", vocabulary), ("expressions", "表达区域", expressions)]:
            begin = len(c.nodes)
            if not content:
                continue
            y = top if kind == "vocabulary" else expression_top
            if headings[kind]:
                y = c.draw_rows(page, headings[kind], r.margin, y, f"{prefix}/{kind}", page=page_index) + 12
            if arrangement:
                for card in content:
                    slot = c.slot(card["id"], page_index)
                    if slot:
                        x, y, right, bottom = slot["bounds"]
                        draw_card(page, card, x, y, right - x, bottom - y, kind, page_index)
            elif kind == "vocabulary":
                for row, height in vocabulary_pages[page_index % len(vocabulary_pages)]:
                    for column, card in enumerate(row):
                        draw_card(page, card, r.margin + column * (card_width + gap), y, card_width, height, kind, page_index)
                    y += height + 18
            else:
                card = expressions[page_index % len(expressions)]
                draw_card(page, card, r.margin, y, r.content_width, card["height"], kind, page_index)
            c.group(f"{prefix}/{kind}", label, c.nodes[begin:], page_index)
        if count > 1:
            item = c.rows(f"{prefix}/page", "分页文字", f"第 {page_index + 1} / {count} 页", 21 if r.layout == "landscape" else 26, r.muted, 300)
            if item:
                c.draw_rows(page, item, r.width - r.margin - max(row.font.getlength(row.text) for row in item["rows"]), top, page=page_index)
        pages.append(page)
    return {"pages": pages, "background": background, "paints": c.paints, "original_paints": original_paints, "roman_paints": roman_paints,
            "original_rows": original_rows, "nodes": c.nodes, "composition": c,
            "start_ms": start_ms, "end_ms": end_ms, "page_start_ms": max(start_ms, source["start_ms"]),
            "inventory": inventory, "line_id": source["id"], "text": source["text"]}
