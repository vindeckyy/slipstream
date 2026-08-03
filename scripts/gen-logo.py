#!/usr/bin/env python3
"""Generate the clean Slipstream logo lockup (committed, like the other branding assets).

The source SS.png banner has a busy dotted background and a blurry wordmark, so the
README/docs logo is recomposed instead: the crisp full-res S-swoosh (extracted from
SS.png by colour classification) + "Slipstream" in Inter Bold Italic on a solid
near-black background. The result is sharp at every size the lockup is shown.

Output: assets/slipstream-logo.png (1800x420)

Pure stdlib + PIL (the repo's branding scripts already use PIL where available; the
tray generator stays stdlib-only because it must run anywhere).
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "assets/slipstream-logo.png"
OUT = REPO / "assets/slipstream-logo.png"
FONT = "/usr/share/fonts/opentype/inter/Inter-BoldItalic.otf"

SCALE = 3
W, H = 1800 * SCALE, 420 * SCALE
BG = (5, 8, 12, 255)          # solid near-black (no texture)
TEXT = (103, 203, 247, 255)   # the S's light cyan — cohesive with the mark


def extract_s(src: Image.Image) -> Image.Image:
    """Crop the S-swoosh out of the banner by its saturated-blue signature."""
    W, H = src.size
    px = src.load()

    def is_s(p):
        r, g, b, a = p
        return a > 40 and b > 100 and b > r * 1.6 and g > r * 0.9

    cols = [x for x in range(W) if any(is_s(px[x, y]) for y in range(0, H, 4))]
    rows = [y for y in range(H) if any(is_s(px[x, y]) for x in range(0, W, 4))]
    if not cols or not rows:
        raise SystemExit("could not locate the S in the source banner")
    bbox = (cols[0], rows[0], cols[-1] + 1, rows[-1] + 1)
    return src.crop(bbox)


def main():
    src = Image.open(SRC).convert("RGBA")
    s = extract_s(src)

    img = Image.new("RGBA", (W, H), BG)
    d = ImageDraw.Draw(img)

    # S at ~72% of the canvas height (it is short/wide).
    s_h = int(H * 0.72)
    s_w = int(s.width * s_h / s.height)
    sx = int(W * 0.05)
    sy = (H - s_h) // 2
    s2 = s.resize((s_w, s_h), Image.LANCZOS)
    img.paste(s2, (sx, sy), s2)

    # Wordmark at ~46% of the canvas height, optically centered.
    word = "Slipstream"
    font = ImageFont.truetype(FONT, 10)
    while d.textbbox((0, 0), word, font=font)[3] - d.textbbox((0, 0), word, font=font)[1] < H * 0.46:
        font = ImageFont.truetype(FONT, font.size + 1)
    tb = d.textbbox((0, 0), word, font=font)
    tx = sx + s_w + int(W * 0.055)
    ty = (H - (tb[3] - tb[1])) // 2 - tb[1]
    d.text((tx, ty), word, font=font, fill=TEXT)

    img = img.resize((W // SCALE, H // SCALE), Image.LANCZOS)
    img.save(OUT)
    print(f"wrote {OUT.relative_to(REPO)} ({img.size[0]}x{img.size[1]})")


if __name__ == "__main__":
    main()
