#!/usr/bin/env python3
"""Validate the YC demo's duration and hard upload-size limits."""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

MIN_DURATION_SECONDS = 150.0
MAX_DURATION_SECONDS = 170.0
MAX_BYTES = 100_000_000


def validate_metrics(duration_seconds: float, size_bytes: int) -> list[str]:
    errors = []
    if not MIN_DURATION_SECONDS <= duration_seconds <= MAX_DURATION_SECONDS:
        errors.append(
            f"duration {duration_seconds:.3f}s is outside "
            f"{MIN_DURATION_SECONDS:.0f}–{MAX_DURATION_SECONDS:.0f}s"
        )
    if size_bytes > MAX_BYTES:
        errors.append(f"size {size_bytes} bytes exceeds {MAX_BYTES} bytes")
    if size_bytes <= 0:
        errors.append("output file is empty")
    return errors


def probe(path: Path) -> tuple[float, int]:
    ffprobe = shutil.which("ffprobe")
    if ffprobe is None:
        raise RuntimeError("ffprobe is required (install with: brew install ffmpeg)")
    result = subprocess.run(
        [
            ffprobe,
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            str(path),
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    return float(result.stdout.strip()), path.stat().st_size


def main(path: Path) -> int:
    duration, size = probe(path)
    errors = validate_metrics(duration, size)
    print(f"duration={duration:.3f}s size={size} bytes ({size / 1_000_000:.2f} MB)")
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {sys.argv[0]} VIDEO.mp4")
    raise SystemExit(main(Path(sys.argv[1]).expanduser().resolve()))
