#!/usr/bin/env python3
"""Generate the Windows ICO from Reyn's existing square-and-flow mark."""

from __future__ import annotations

import struct
import zlib
from pathlib import Path


SIZE = 64
BG = (14, 12, 10, 255)
FRAME = (90, 73, 59, 255)
EMBER = (255, 132, 47, 255)
FLOW = (223, 100, 29, 255)
LIGHT = (255, 210, 178, 255)


def pixel(x: int, y: int) -> tuple[int, int, int, int]:
    if not (2 <= x < SIZE - 2 and 2 <= y < SIZE - 2):
        return (0, 0, 0, 0)
    color = BG
    if (17 <= x <= 47 and y in range(17, 20)) or (
        17 <= x <= 47 and y in range(45, 48)
    ):
        color = FRAME
    if (17 <= y <= 47 and x in range(17, 20)) or (
        17 <= y <= 47 and x in range(45, 48)
    ):
        color = FRAME
    if 22 <= x <= 43 and 31 <= y <= 34:
        color = FLOW
    if (x - 18) ** 2 + (y - 18) ** 2 <= 9:
        color = EMBER
    if (x - 46) ** 2 + (y - 46) ** 2 <= 9:
        color = EMBER
    if (x - 43) ** 2 + (y - 32) ** 2 <= 4:
        color = LIGHT
    return color


def png_bytes() -> bytes:
    raw = bytearray()
    for y in range(SIZE):
        raw.append(0)
        for x in range(SIZE):
            raw.extend(pixel(x, y))

    def chunk(kind: bytes, payload: bytes) -> bytes:
        return (
            struct.pack(">I", len(payload))
            + kind
            + payload
            + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), level=9))
        + chunk(b"IEND", b"")
    )


def write_icon(destination: Path) -> None:
    image = png_bytes()
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", SIZE, SIZE, 0, 0, 1, 32, len(image), 22)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(header + entry + image)


if __name__ == "__main__":
    write_icon(
        Path(__file__).resolve().parents[1]
        / "packaging"
        / "windows"
        / "ReynStudio.ico"
    )
