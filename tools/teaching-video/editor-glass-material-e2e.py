import argparse
import base64
import io
import json
from pathlib import Path
from urllib.request import Request, urlopen

import numpy as np
from PIL import Image

from composition import empty_edits


def request(url, route, body=None, token=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["X-Editor-Token"] = token
    payload = json.dumps(body).encode() if body is not None else None
    with urlopen(Request(url + route, data=payload, headers=headers), timeout=180) as response:
        if response.status != 200:
            raise ValueError(f"实际编辑服务请求失败：{response.status}")
        return json.load(response)


def picture(preview):
    with Image.open(io.BytesIO(base64.b64decode(preview["image"].split(",", 1)[1]))) as source:
        return source.convert("RGB")


def luminance(rgb):
    channels = np.asarray(rgb, dtype=np.float64) / 255
    return np.where(channels <= 0.04045, channels / 12.92, ((channels + 0.055) / 1.055) ** 2.4) @ [0.2126, 0.7152, 0.0722]


def inspect_material(image, nodes, card, accent):
    left, top, right, bottom = card["bounds"]
    pixels = np.asarray(image)[top:bottom, left:right]
    blank = np.zeros(pixels.shape[:2], dtype=bool)
    blank[18:-18, 18:-18] = True
    for child in nodes:
        if child["parent"] != card["id"]:
            continue
        x1, y1, x2, y2 = child["bounds"]
        blank[max(0, y1 - top):min(bottom - top, y2 - top), max(0, x1 - left):min(right - left, x2 - left)] = False
    background = pixels[blank]
    if len(background) < 100:
        raise ValueError(f"卡片没有足够的背景采样区域：{card['id']}")
    contrasts = {name: float(((luminance(color) + .05) / (luminance(background) + .05)).min())
                 for name, color in {"white": [241, 243, 246], "muted": [180, 190, 203], "accent": accent}.items()}
    return {"component": card["id"], "samples": len(background),
            "channel_stddev": background.std(axis=0).tolist(), "mean_rgb": background.mean(axis=0).tolist(),
            "minimum_text_contrast": contrasts,
            "top_edge_luminance": float(luminance(pixels[2, (right - left) // 2])),
            "interior_luminance": float(luminance(pixels[16, (right - left) // 2]))}


def main():
    parser = argparse.ArgumentParser(description="通过实际编辑服务检查视频卡片的玻璃材质和文字对比度")
    parser.add_argument("--url", default="http://127.0.0.1:49609")
    parser.add_argument("--evidence-dir", type=Path, default=Path("work/video-glass-e2e/material-evidence"))
    args = parser.parse_args()
    args.evidence_dir.mkdir(parents=True, exist_ok=True)
    session = request(args.url, "/api/session")
    checks = []
    for layout in ["landscape", "portrait"]:
        body = {"edits": empty_edits(), "layout": layout, "time_ms": session["lines"][0]["start_ms"]}
        preview = request(args.url, "/api/preview", body, session["token"])
        image = picture(preview)
        image.save(args.evidence_dir / f"glass-{layout}.png")
        accent = [int(preview["accent"][index:index + 2], 16) for index in [1, 3, 5]]
        cards = [node for node in preview["nodes"] if "/card:" in node["id"] and node["text"] is None]
        if not cards:
            raise ValueError("实际歌曲场景缺少教学卡片")
        for card in cards:
            new = inspect_material(image, preview["nodes"], card, accent)
            if max(new["channel_stddev"]) < .5:
                raise ValueError(f"玻璃卡片没有呈现背景与反光变化：{card['id']}")
            if min(new["minimum_text_contrast"].values()) < 4.5:
                raise ValueError(f"玻璃卡片文字对比度不足：{new}")
            if new["top_edge_luminance"] <= new["interior_luminance"]:
                raise ValueError(f"玻璃卡片缺少边缘高光：{card['id']}")
            checks.append({"name": layout + "-" + card["id"], "glass": new})

        removed = empty_edits()
        removed["automatic"][layout] = True
        removed["deleted"] = [cards[0]["id"]]
        deleted = request(args.url, "/api/preview", {**body, "edits": removed}, session["token"])
        next_card = next(node for node in deleted["nodes"] if node["id"] == cards[1]["id"])
        if next_card["bounds"][:2] != cards[0]["bounds"][:2]:
            raise ValueError("删除玻璃卡片之后，其他卡片没有重新排版")
        picture(deleted).save(args.evidence_dir / f"reflow-{layout}.png")
        checks.append({"name": layout + "-delete-and-reflow", "component": next_card})

        moved = empty_edits()
        moved["positions"][layout][cards[0]["id"]] = [12, 0]
        shifted = request(args.url, "/api/preview", {**body, "edits": moved}, session["token"])
        shifted_card = next(node for node in shifted["nodes"] if node["id"] == cards[0]["id"])
        if shifted_card["bounds"][0] != cards[0]["bounds"][0] + 12:
            raise ValueError("玻璃卡片没有随组件移动")
        details = inspect_material(picture(shifted), shifted["nodes"], shifted_card, accent)
        if min(details["minimum_text_contrast"].values()) < 4.5:
            raise ValueError("移动后的玻璃卡片文字对比度不足")
        checks.append({"name": layout + "-move", "glass": details})

        removed["deleted"] = [node["id"] for node in preview["nodes"] if node["parent"] is None]
        blank = request(args.url, "/api/preview", {**body, "edits": removed}, session["token"])
        if blank["nodes"] or not np.all(np.asarray(picture(blank)) == [5, 13, 23]):
            raise ValueError("删除全部组件后，画面仍然包含玻璃背景或阴影")
        checks.append({"name": layout + "-delete-all-components", "nodes": blank["nodes"]})

    saved = request(args.url, "/api/session")
    if saved["edits"] != session["edits"]:
        raise ValueError("材质验证改变了保存的编辑内容")
    path = args.evidence_dir / "glass-material-e2e.json"
    path.write_text(json.dumps({"status": "passed", "url": args.url, "checks": checks}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": "passed", "checks": len(checks), "report": str(path.resolve())}, ensure_ascii=False))


if __name__ == "__main__":
    main()
