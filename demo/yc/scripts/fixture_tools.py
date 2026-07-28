#!/usr/bin/env python3
"""Small, dependency-free binary-STL checks for the YC demo fixtures."""

from __future__ import annotations

import hashlib
import json
import struct
from collections import Counter
from pathlib import Path


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_binary_stl(path: Path) -> list[tuple[tuple[float, float, float], ...]]:
    data = path.read_bytes()
    if len(data) < 84:
        raise ValueError(f"{path} is shorter than a binary STL header")
    triangle_count = struct.unpack_from("<I", data, 80)[0]
    expected_size = 84 + 50 * triangle_count
    if len(data) != expected_size:
        raise ValueError(
            f"{path} declares {triangle_count} triangles but has {len(data)} bytes; "
            f"expected {expected_size}"
        )
    triangles = []
    for index in range(triangle_count):
        offset = 84 + index * 50 + 12
        values = struct.unpack_from("<9f", data, offset)
        triangles.append(
            tuple(tuple(values[axis : axis + 3]) for axis in range(0, 9, 3))
        )
    return triangles


def topology(path: Path) -> dict[str, int | bool]:
    triangles = read_binary_stl(path)
    edges: Counter[tuple[tuple[float, ...], tuple[float, ...]]] = Counter()
    for triangle in triangles:
        points = [tuple(round(value, 6) for value in point) for point in triangle]
        for index in range(3):
            edge = tuple(sorted((points[index], points[(index + 1) % 3])))
            edges[edge] += 1
    boundary_edges = sum(count == 1 for count in edges.values())
    non_manifold_edges = sum(count > 2 for count in edges.values())
    return {
        "triangles": len(triangles),
        "boundary_edges": boundary_edges,
        "non_manifold_edges": non_manifold_edges,
        "watertight": boundary_edges == 0 and non_manifold_edges == 0,
    }


def validate_manifest(manifest_path: Path) -> list[dict[str, object]]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    asset_dir = manifest_path.parent
    results = []
    for fixture in manifest["fixtures"]:
        path = asset_dir / fixture["file"]
        actual = {
            **topology(path),
            "sha256": sha256_file(path),
            "bytes": path.stat().st_size,
        }
        for key in (
            "triangles",
            "boundary_edges",
            "non_manifold_edges",
            "watertight",
            "sha256",
            "bytes",
        ):
            if actual[key] != fixture[key]:
                raise ValueError(
                    f"{fixture['file']}: {key} is {actual[key]!r}, "
                    f"manifest says {fixture[key]!r}"
                )
        results.append({"file": fixture["file"], **actual})
    return results
