#!/usr/bin/env python3
"""Generate the three deterministic YC demo STL fixtures."""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path

from fixture_tools import sha256_file, topology

ROOT = Path(__file__).resolve().parents[3]
GENERATOR = ROOT / "test-geometry" / "make_test_stls.py"


def load_generator():
    spec = importlib.util.spec_from_file_location("reyn_test_geometry", GENERATOR)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {GENERATOR}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def missing_cap(triangles, radius: float):
    return [
        triangle
        for triangle in triangles
        if sum(point[2] for point in triangle) / 3.0 < 0.72 * radius
    ]


def generate(output_dir: Path) -> Path:
    base = load_generator()
    output_dir.mkdir(parents=True, exist_ok=True)
    radius = 50.0
    definitions = [
        {
            "file": "primary_capsule_d80_l260mm.stl",
            "role": "primary",
            "title": "Watertight road-vehicle-like capsule",
            "source": (
                "Deterministically generated from test-geometry/make_test_stls.py "
                "using capsule_body(radius=40, length=260, n=40, rings=12)."
            ),
            "units": "millimeter",
            "expected_diagnostics": [],
            "triangles_data": base.capsule_body(40.0, 260.0, n=40, rings=12),
        },
        {
            "file": "fallback_cube_100mm.stl",
            "role": "fallback",
            "title": "Watertight 100 mm cube",
            "source": (
                "Deterministically generated from test-geometry/make_test_stls.py "
                "using box(0, 0, 0, 100, 100, 100)."
            ),
            "units": "millimeter",
            "expected_diagnostics": [],
            "triangles_data": base.box(0, 0, 0, 100, 100, 100),
        },
        {
            "file": "defective_sphere_missing_cap_r50mm.stl",
            "role": "intentional_rejection",
            "title": "Intentionally open sphere with missing cap",
            "source": (
                "Deterministically generated from test-geometry/make_test_stls.py "
                "using uv_sphere(r=50, nu=48, nv=24), then removing triangles "
                "whose centroid has z >= 36 mm; mirrors the existing large-leak fixture."
            ),
            "units": "millimeter",
            "expected_diagnostics": [
                {
                    "code": "mesh.open_boundary",
                    "waivable": False,
                    "expected": "nonzero open boundary edges; execution blocked",
                }
            ],
            "triangles_data": missing_cap(base.uv_sphere(radius), radius),
        },
    ]

    fixtures = []
    for definition in definitions:
        triangles = definition.pop("triangles_data")
        path = output_dir / definition["file"]
        base.write_stl(path, triangles)
        fixtures.append(
            {
                **definition,
                **topology(path),
                "bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )

    manifest = {
        "schema": "com.reyn.studio.yc-demo-fixtures/1",
        "generator": "demo/yc/scripts/build_fixtures.py",
        "fixture_policy": (
            "Pipeline fixtures only. They demonstrate import and preflight behavior; "
            "they are not model-accuracy evidence."
        ),
        "fixtures": fixtures,
    }
    manifest_path = output_dir / "fixture-manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return manifest_path


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "demo" / "yc" / "assets",
    )
    args = parser.parse_args()
    print(generate(args.output_dir.resolve()))
