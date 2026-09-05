#!/usr/bin/env python3
"""Rasterize the original Harbor window mark (open ring + beacon) to icon.png."""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path

SIZE = 512
SUPERSAMPLE = 2
BG = (0x0B, 0x0B, 0x0C, 255)
RING = (0xF5, 0xF5, 0xF5, 255)
BEACON = (0x22, 0xD3, 0xEE, 255)


def mix(a: tuple[int, int, int, int], b: tuple[int, int, int, int], t: float) -> tuple[int, int, int, int]:
    t = 0.0 if t < 0.0 else 1.0 if t > 1.0 else t
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(4))  # type: ignore[return-value]


def coverage(edge: float) -> float:
    return 0.0 if edge >= 1.0 else 1.0 if edge <= -1.0 else 0.5 - edge * 0.5


def write_png(path: Path, width: int, height: int, rgba: bytes) -> None:
    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + rgba[y * width * 4 : (y + 1) * width * 4] for y in range(height))
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    )


def rasterize() -> bytes:
    n = SIZE * SUPERSAMPLE
    cx = cy = (n - 1) / 2.0
    scale = n / 2.0
    ring_r = 0.62 * scale
    ring_w = 0.18 * scale
    gap = math.radians(58)
    beacon_r = 0.11 * scale
    glow_r = 0.28 * scale
    pixels = bytearray(n * n * 4)
    for y in range(n):
        for x in range(n):
            dx = x - cx
            dy = y - cy
            dist = math.hypot(dx, dy)
            ang = math.atan2(dy, dx)
            pixel = BG
            glow = coverage((dist - glow_r) / SUPERSAMPLE) * (1.0 - dist / glow_r) * 0.35
            if glow > 0:
                pixel = mix(pixel, BEACON, glow)
            beacon = coverage((dist - beacon_r) / SUPERSAMPLE)
            if beacon > 0:
                pixel = mix(pixel, BEACON, beacon)
            ring = coverage((abs(dist - ring_r) - ring_w / 2.0) / SUPERSAMPLE)
            delta = abs((ang - math.pi / 2 + math.pi) % (2 * math.pi) - math.pi)
            ang_aa = SUPERSAMPLE / max(ring_r, 1.0)
            gap_t = coverage((delta - gap / 2.0) / ang_aa)
            ring *= 1.0 - gap_t
            if ring > 0:
                pixel = mix(pixel, RING, ring)
            i = (y * n + x) * 4
            pixels[i : i + 4] = bytes(pixel)
    out = bytearray(SIZE * SIZE * 4)
    factor = SUPERSAMPLE * SUPERSAMPLE
    for y in range(SIZE):
        for x in range(SIZE):
            acc = [0, 0, 0, 0]
            for oy in range(SUPERSAMPLE):
                for ox in range(SUPERSAMPLE):
                    i = ((y * SUPERSAMPLE + oy) * n + (x * SUPERSAMPLE + ox)) * 4
                    for c in range(4):
                        acc[c] += pixels[i + c]
            j = (y * SIZE + x) * 4
            out[j : j + 4] = bytes(v // factor for v in acc)
    return bytes(out)


def main() -> None:
    dest = Path(__file__).with_name("icon.png")
    write_png(dest, SIZE, SIZE, rasterize())


if __name__ == "__main__":
    main()
