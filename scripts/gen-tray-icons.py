#!/usr/bin/env python3
"""Generate the slipstream-tray status icons (committed, like the other branding assets).

Renders the brand mark — the blue 3-swoosh S from assets/slipstream-mark.png (SS.png-derived)
— with a status dot in the lower-right corner:

  running    colored mark + green dot
  stopped    grayscale mark + gray dot
  error      colored mark + red dot
  degraded   colored mark + amber dot        (starting / running-but-status-unreachable)
  streaming  colored mark + bright-violet dot

Outputs (all checked in; re-run only when the brand or the palette changes):
  packaging/windows/branding/slipstream-tray-<state>.ico   16/20/24/32/48 px PNG-entry icos
                                                          (Vista+ format, same as slipstream.ico)
  packaging/linux/icons/hicolor/{22x22,48x48}/apps/slipstream-tray[-<state>].png
                                                          (running is the unsuffixed base name)

Pure stdlib (zlib PNG writer + mark sampling from the committed mark PNG) so it runs on any
dev box — no PIL/ImageMagick/librsvg needed. The mark PNG is read once and supersampled with
the same analytic 4x rasterizer the circle mark used.
"""

import struct
import zlib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
MARK = REPO / "assets/slipstream-mark.png"

STATES = {
    "running": {"dot": (0x2E, 0xCC, 0x71), "gray": False},
    "stopped": {"dot": (0x8A, 0x8A, 0x8A), "gray": True},
    "error": {"dot": (0xE7, 0x4C, 0x3C), "gray": False},
    "degraded": {"dot": (0xF0, 0xA0, 0x30), "gray": False},
    "streaming": {"dot": (0xB4, 0x4C, 0xF0), "gray": False},
}


def luma(c):
    y = round(0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2])
    return (y, y, y)


def load_mark_rgba():
    """Parse the committed mark PNG into a (w, h, [r,g,b,a ...]) tuple (stdlib zlib)."""
    data = MARK.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, w, h, idat = 8, 0, 0, b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        tag = data[pos + 4 : pos + 8]
        payload = data[pos + 8 : pos + 8 + length]
        if tag == b"IHDR":
            w, h, depth, color = struct.unpack(">IIBB", payload[:10])
            assert depth == 8 and color in (6, 2), f"unsupported PNG ({depth}bit/{color})"
        elif tag == b"IDAT":
            idat += payload
        elif tag == b"IEND":
            break
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride = w * 4
    # RGBA (color 6) or RGB (color 2) → flat RGBA list.
    px = []
    for y in range(h):
        row = raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)]
        for x in range(w):
            i = x * 4
            if color == 6:
                px.extend(row[i : i + 4])
            else:
                px.extend((row[i], row[i + 1], row[i + 2], 255))
    return w, h, px


def sample_mark(mark, mx, my):
    """Nearest-sample the mark RGBA at normalized (0..1) coords; None when transparent."""
    w, h, px = mark
    xi = min(int(mx * w), w - 1)
    yi = min(int(my * h), h - 1)
    i = (yi * w + xi) * 4
    a = px[i + 3]
    if a < 40:
        return None
    return (px[i], px[i + 1], px[i + 2])


def render(size, dot_rgb, gray, ss=2):
    """RGBA rows: the S-swoosh mark as a crisp solid glyph + a plain status dot.

    A 2x supersample with a hard alpha threshold keeps the shape clean at 22/48 px (the
    4x + soft edges of the mark PNG read as noise at tray size). The dot is a plain circle
    with no dark ring — the ring only muddied it at small sizes.
    """
    n = size * ss
    mark = load_mark_rgba()
    w, h, px = mark
    def alpha_at(x, y):
        return px[(y * w + x) * 4 + 3]
    minx = next((x for x in range(w) if any(alpha_at(x, y) > 60 for y in range(h))), 0)
    maxx = next((x for x in range(w - 1, -1, -1) if any(alpha_at(x, y) > 60 for y in range(h))), w - 1)
    miny = next((y for y in range(h) if any(alpha_at(x, y) > 60 for x in range(w))), 0)
    maxy = next((y for y in range(h - 1, -1, -1) if any(alpha_at(x, y) > 60 for x in range(w))), h - 1)
    span = max(maxx - minx, maxy - miny)
    cx = (minx + maxx) / 2 / w
    cy = (miny + maxy) / 2 / h
    cw = span / w
    ch = span / h
    mark_c = (0.44 * n, 0.44 * n)
    dot_c = (0.76 * n, 0.76 * n)
    dot_r = 0.20 * n
    fill = luma((0x09, 0xA0, 0xE8)) if gray else (0x09, 0xA0, 0xE8)

    rows = []
    for y in range(size):
        row = bytearray()
        for x in range(size):
            hit = False
            for sy in range(ss):
                for sx in range(ss):
                    px_ = x * ss + sx + 0.5
                    py_ = y * ss + sy + 0.5
                    dd = ((px_ - dot_c[0]) ** 2 + (py_ - dot_c[1]) ** 2) ** 0.5
                    if dd < dot_r:
                        hit = True
                        col = dot_rgb
                        break
                    mx = (px_ - mark_c[0]) / (0.80 * n) * cw + cx
                    my = (py_ - mark_c[1]) / (0.80 * n) * ch + cy
                    if 0.0 <= mx <= 1.0 and 0.0 <= my <= 1.0 and sample_mark(mark, mx, my) is not None:
                        hit = True
                        col = fill
                        break
                if hit:
                    break
            if not hit:
                row += b"\x00\x00\x00\x00"
            else:
                row += bytes((*col, 255))
        rows.append(bytes(row))
    return rows


def png_bytes(size, rows):
    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    idat = zlib.compress(b"".join(b"\x00" + r for r in rows), 9)
    return (
        b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b"")
    )


def ico_bytes(pngs):
    """PNG-entry .ico (Vista+; the format slipstream.ico already uses)."""
    header = struct.pack("<HHH", 0, 1, len(pngs))
    entries = b""
    blobs = b""
    offset = len(header) + 16 * len(pngs)
    for size, png in pngs:
        entries += struct.pack(
            "<BBBBHHII", size if size < 256 else 0, size if size < 256 else 0, 0, 0, 1, 32, len(png), offset
        )
        blobs += png
        offset += len(png)
    return header + entries + blobs


def main():
    ico_dir = REPO / "packaging/windows/branding"
    for state, spec in STATES.items():
        pngs = [
            (s, png_bytes(s, render(s, spec["dot"], spec["gray"])))
            for s in (16, 20, 24, 32, 48)
        ]
        out = ico_dir / f"slipstream-tray-{state}.ico"
        out.write_bytes(ico_bytes(pngs))
        print(f"wrote {out.relative_to(REPO)}")

        for s in (22, 48):
            name = "slipstream-tray" if state == "running" else f"slipstream-tray-{state}"
            png_dir = REPO / f"packaging/linux/icons/hicolor/{s}x{s}/apps"
            png_dir.mkdir(parents=True, exist_ok=True)
            out = png_dir / f"{name}.png"
            out.write_bytes(png_bytes(s, render(s, spec["dot"], spec["gray"])))
            print(f"wrote {out.relative_to(REPO)}")


if __name__ == "__main__":
    main()
