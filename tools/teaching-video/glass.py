import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageOps


def draw_liquid_glass(image, backdrop, bounds):
    left, top, right, bottom = bounds
    width, height = right - left, bottom - top
    radius = min(28, width // 2, height // 2)
    scale = 3
    shape = Image.new("L", (width * scale, height * scale))
    ImageDraw.Draw(shape).rounded_rectangle((0, 0, width * scale - 1, height * scale - 1),
                                           radius=radius * scale, fill=255)
    mask = shape.resize((width, height), Image.Resampling.LANCZOS)

    padding = 24
    shadow = Image.new("L", (width + padding * 2, height + padding * 2))
    shadow.paste(mask.point(lambda value: round(value * 0.32)), (padding, padding + 5))
    shadow = shadow.filter(ImageFilter.GaussianBlur(9))
    image.paste((0, 0, 0), (left - padding, top - padding), shadow)

    # 使用组件实际位置下的封面画面，局部放大表现玻璃的折射。
    sample = backdrop.crop(bounds).filter(ImageFilter.GaussianBlur(8))
    expanded_width, expanded_height = round(width * 1.045), round(height * 1.045)
    sample = sample.resize((expanded_width, expanded_height), Image.Resampling.BICUBIC)
    offset_x, offset_y = (expanded_width - width) // 2, (expanded_height - height) // 2
    sample = sample.crop((offset_x, offset_y, offset_x + width, offset_y + height))
    pixels = np.asarray(sample, dtype=np.float32)
    y, x = np.ogrid[:height, :width]
    horizontal, vertical = x / max(1, width - 1), y / max(1, height - 1)
    reflection = np.exp(-((horizontal - 0.18) / 0.7) ** 2 - ((vertical + 0.08) / 0.65) ** 2)
    surface = pixels * 0.7 + np.array([15, 24, 36], dtype=np.float32) * 0.3
    surface += reflection[..., None] * np.array([10, 12, 15], dtype=np.float32)
    # 文字区域保持自动取色使用的亮度上限。
    surface = np.minimum(surface, [61, 66, 74])

    inner = Image.new("L", mask.size)
    ImageDraw.Draw(inner).rounded_rectangle((2, 2, width - 3, height - 3), radius=max(0, radius - 2), fill=255)
    edge = np.asarray(ImageChops.subtract(mask, inner), dtype=np.float32) / 255
    softened = ImageOps.expand(mask, border=15).filter(ImageFilter.GaussianBlur(5))
    softened = softened.crop((15, 15, width + 15, height + 15))
    rim = np.asarray(ImageChops.subtract(mask, softened), dtype=np.float32) / 255
    lighting = 0.28 + 0.72 * (1 - vertical) * (0.85 + 0.15 * (1 - horizontal))
    highlight = np.clip(edge * lighting * 0.26 + rim * lighting * 0.13, 0, 1)
    surface = surface * (1 - highlight[..., None]) + np.array([213, 230, 249]) * highlight[..., None]
    material = Image.fromarray(np.rint(surface).astype(np.uint8))
    image.paste(material, (left, top), mask)
    return {"material": "liquid-glass", "radius": radius}
