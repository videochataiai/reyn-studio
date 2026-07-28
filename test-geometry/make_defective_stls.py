"""Generate DEFECTIVE STL fixtures and quantify the ray-parity failure they cause.

`src/cad.rs` fills the occupancy mask by casting +x rays, sorting the crossings,
and pairing them two at a time (`hits[m], hits[m+1]`, `m += 2`), skipping any row
with fewer than two hits. That is only correct for a closed surface. A hole makes
the crossing count odd along the rays that pass through it, so spans are paired
against the wrong partner or dropped entirely — and nothing in the pipeline
reports it. Meanwhile `mesh.open_boundary` is a waivable preflight issue and
`record_waiver` accepts any rationale of eight characters or more, so a case can
be approved on a mask that is silently wrong.

These fixtures exist so that failure is reproducible and measurable. For the
sphere cases the true interior is known analytically, so the corruption is
reported as an exact cell count rather than an impression.

Run: python3 make_defective_stls.py
"""

import math
from pathlib import Path

import make_test_stls as base

OUT = Path(__file__).parent / "defective"
GRID = 64


def drop_patch(tris, keep_pred):
    """Remove triangles whose centroid fails keep_pred, opening a hole."""
    kept = []
    for tri in tris:
        cx = sum(p[0] for p in tri) / 3.0
        cy = sum(p[1] for p in tri) / 3.0
        cz = sum(p[2] for p in tri) / 3.0
        if keep_pred(cx, cy, cz):
            kept.append(tri)
    return kept


def reverse_winding(tris):
    return [(a, c, b) for a, b, c in tris]


def translate(tris, dx, dy, dz):
    return [
        tuple((p[0] + dx, p[1] + dy, p[2] + dz) for p in tri) for tri in tris
    ]


def ray_x_hit(tri, y, z):
    """Intersection x of the +x line at (y, z) with tri, mirroring cad.rs."""
    (x0, y0, z0), (x1, y1, z1), (x2, y2, z2) = tri
    d = (y1 - y0) * (z2 - z0) - (y2 - y0) * (z1 - z0)
    if abs(d) < 1e-20:
        return None
    a = ((y - y0) * (z2 - z0) - (y2 - y0) * (z - z0)) / d
    b = ((y1 - y0) * (z - z0) - (y - y0) * (z1 - z0)) / d
    if a < 0.0 or b < 0.0 or a + b > 1.0:
        return None
    return x0 + a * (x1 - x0) + b * (x2 - x0)


def voxelize_parity(tris, origin, extent, n=GRID):
    """Reproduce the cad.rs parity fill, including its skip/pair behaviour.

    Returns (mask, odd_rows) where odd_rows counts scanlines whose crossing
    count was odd — i.e. rows where a closed surface is impossible and the
    pairing below is provably meaningless.
    """
    dx = extent / n
    boxes = []
    for tri in tris:
        ys = [p[1] for p in tri]
        zs = [p[2] for p in tri]
        boxes.append((min(ys), max(ys), min(zs), max(zs)))
    mask = bytearray(n * n * n)
    odd_rows = 0
    for j in range(n):
        y = origin[1] + (j + 0.5) * dx + 1.3e-4
        for k in range(n):
            z = origin[2] + (k + 0.5) * dx + 2.7e-4
            hits = []
            for tri, (ymin, ymax, zmin, zmax) in zip(tris, boxes):
                if y < ymin or y > ymax or z < zmin or z > zmax:
                    continue
                x = ray_x_hit(tri, y, z)
                if x is not None:
                    hits.append(x)
            if len(hits) % 2 == 1:
                odd_rows += 1
            if len(hits) < 2:
                continue
            hits.sort()
            m = 0
            while m + 1 < len(hits):
                xa, xb = hits[m], hits[m + 1]
                for i in range(n):
                    c = origin[0] + (i + 0.5) * dx
                    if xa <= c <= xb:
                        mask[i * n * n + j * n + k] = 1
                m += 2
    return mask, odd_rows


def analytic_sphere_mask(radius, center, origin, extent, n=GRID):
    dx = extent / n
    mask = bytearray(n * n * n)
    for i in range(n):
        cx = origin[0] + (i + 0.5) * dx - center[0]
        for j in range(n):
            cy = origin[1] + (j + 0.5) * dx - center[1]
            for k in range(n):
                cz = origin[2] + (k + 0.5) * dx - center[2]
                if cx * cx + cy * cy + cz * cz <= radius * radius:
                    mask[i * n * n + j * n + k] = 1
    return mask


def analytic_box_union_mask(boxes, origin, extent, n=GRID):
    """Truth mask for a union of axis-aligned boxes given as (center, size)."""
    dx = extent / n
    mask = bytearray(n * n * n)
    for i in range(n):
        cx = origin[0] + (i + 0.5) * dx
        for j in range(n):
            cy = origin[1] + (j + 0.5) * dx
            for k in range(n):
                cz = origin[2] + (k + 0.5) * dx
                for (bx, by, bz), (sx, sy, sz) in boxes:
                    if (
                        abs(cx - bx) <= sx / 2
                        and abs(cy - by) <= sy / 2
                        and abs(cz - bz) <= sz / 2
                    ):
                        mask[i * n * n + j * n + k] = 1
                        break
    return mask


def boundary_edge_count(tris):
    edges = {}
    for tri in tris:
        pts = [tuple(round(v, 6) for v in p) for p in tri]
        for i in range(3):
            e = frozenset((pts[i], pts[(i + 1) % 3]))
            edges[e] = edges.get(e, 0) + 1
    return sum(1 for n in edges.values() if n == 1)


def compare(name, mask, truth):
    wrong = sum(1 for a, b in zip(mask, truth) if a != b)
    missing = sum(1 for a, b in zip(mask, truth) if b and not a)
    spurious = sum(1 for a, b in zip(mask, truth) if a and not b)
    true_solid = sum(truth)
    pct = 100.0 * wrong / true_solid if true_solid else float("nan")
    print(
        f"    vs analytic truth: {wrong} cells wrong ({pct:.1f}% of the "
        f"{true_solid}-cell true interior) — {missing} missing, {spurious} spurious"
    )
    return wrong


def main():
    OUT.mkdir(exist_ok=True)
    radius = 50.0
    intact = base.uv_sphere(radius)

    # One shared domain so every sphere variant lands on identical cell centers.
    extent = 4.0 * radius
    origin = (-extent / 2.0, -extent / 2.0, -extent / 2.0)
    truth = analytic_sphere_mask(radius, (0.0, 0.0, 0.0), origin, extent)

    # A hole near the +z pole: small (a few triangles) and large (a cap).
    small_leak = drop_patch(intact, lambda x, y, z: not (z > 0.93 * radius and x > 0))
    large_leak = drop_patch(intact, lambda x, y, z: z < 0.72 * radius)

    box = base.box(0, 0, 0, 100, 100, 100)
    solid_box_truth = [((0.0, 0.0, 0.0), (100.0, 100.0, 100.0))]
    cases = [
        (
            "sphere_leak_small.stl",
            small_leak,
            "sphere with a few triangles removed near the +z pole",
            truth,
        ),
        (
            "sphere_leak_large.stl",
            large_leak,
            "sphere with the entire +z cap removed",
            truth,
        ),
        (
            "box_inverted_normals.stl",
            reverse_winding(box),
            "closed box, all winding reversed (inside-out normals)",
            analytic_box_union_mask(solid_box_truth, origin, extent),
        ),
        (
            "box_double_shell.stl",
            box + base.box(0, 0, 0, 50, 50, 50),
            "solid box with a second closed surface inside it",
            analytic_box_union_mask(solid_box_truth, origin, extent),
        ),
        (
            "boxes_self_intersecting.stl",
            box + translate(base.box(0, 0, 0, 100, 100, 100), 60, 20, 20),
            "two interpenetrating boxes (self-intersection, every edge still paired)",
            analytic_box_union_mask(
                solid_box_truth + [((60.0, 20.0, 20.0), (100.0, 100.0, 100.0))],
                origin,
                extent,
            ),
        ),
    ]

    print(f"Reference: intact sphere, r={radius:.0f}mm, {GRID}^3 grid")
    mask, odd = voxelize_parity(intact, origin, extent)
    print(f"  intact sphere: {sum(mask)} solid cells, {odd} odd-crossing rows")
    compare("intact", mask, truth)
    print()

    for name, tris, note, case_truth in cases:
        path = OUT / name
        base.write_stl(path, tris)
        open_edges = boundary_edge_count(tris)
        print(f"{name}")
        print(f"    {note}")
        print(f"    {len(tris)} triangles, {open_edges} open boundary edges")
        mask, odd = voxelize_parity(tris, origin, extent)
        print(f"    parity fill: {sum(mask)} solid cells, {odd} odd-crossing rows")
        compare(name, mask, case_truth)
        print()

    print(
        "Every fixture above is accepted by the current preflight with an\n"
        "eight-character waiver, and none of the corruption is reported."
    )


if __name__ == "__main__":
    main()
