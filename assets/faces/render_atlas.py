#!/usr/bin/env python3
"""Original 8x8 geometric face atlas. Not derived from another product."""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path

CELL = 128
GRID = 8
SIZE = CELL * GRID
HUES = [200, 220, 260, 280, 320, 20, 40, 160, 180, 140, 100, 80]


def hsl_to_rgb(h: float, s: float, l: float) -> tuple[int, int, int]:
    c = (1 - abs(2 * l - 1)) * s
    x = c * (1 - abs((h / 60) % 2 - 1))
    m = l - c / 2
    if h < 60:
        r, g, b = c, x, 0.0
    elif h < 120:
        r, g, b = x, c, 0.0
    elif h < 180:
        r, g, b = 0.0, c, x
    elif h < 240:
        r, g, b = 0.0, x, c
    elif h < 300:
        r, g, b = x, 0.0, c
    else:
        r, g, b = c, 0.0, x
    return int((r + m) * 255), int((g + m) * 255), int((b + m) * 255)


def write_png(path: Path, width: int, height: int, rgba: bytes) -> None:
    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + rgba[y * width * 4 : (y + 1) * width * 4] for y in range(height))
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def rasterize() -> bytes:
    out = bytearray(SIZE * SIZE * 4)
    for slot in range(GRID * GRID):
        row, col = divmod(slot, GRID)
        hue = HUES[slot % len(HUES)]
        fill = hsl_to_rgb(hue, 0.35, 0.28)
        cx = col * CELL + CELL / 2
        cy = row * CELL + CELL / 2
        r = CELL * 0.36
        for y in range(row * CELL, (row + 1) * CELL):
            for x in range(col * CELL, (col + 1) * CELL):
                dx, dy = x - cx, y - cy
                i = (y * SIZE + x) * 4
                if math.hypot(dx, dy) <= r:
                    out[i : i + 4] = bytes((*fill, 255))
                else:
                    out[i : i + 4] = b"\x0b\x0b\x0c\xff"
                if math.hypot(dx + 18, dy + 8) < 7 or math.hypot(dx - 18, dy + 8) < 7:
                    out[i : i + 4] = b"\xf5\xf5\xf5\xff"
    return bytes(out)


def main() -> None:
    dest = Path(__file__).with_name("atlas.png")
    write_png(dest, SIZE, SIZE, rasterize())
    webp = Path(__file__).with_name("atlas.webp")
    try:
        from PIL import Image

        Image.open(dest).save(webp, "WEBP", quality=90)
    except Exception:
        webp.write_bytes(dest.read_bytes())


if __name__ == "__main__":
    main()
