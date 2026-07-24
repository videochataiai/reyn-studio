"""Generate watertight binary STL test geometries for Reyn Studio.

Every mesh is closed (each edge shared by exactly two triangles) so it passes
the source-aware preflight. Dimensions are in millimetres, the most common STL
export convention, so unit handling in preflight gets exercised too.
"""

import math
import struct
from pathlib import Path

OUT = Path(__file__).parent


def write_stl(path, tris):
    with open(path, "wb") as f:
        f.write(b"Reyn Studio test geometry".ljust(80, b"\0"))
        f.write(struct.pack("<I", len(tris)))
        for a, b, c in tris:
            u = [b[i] - a[i] for i in range(3)]
            v = [c[i] - a[i] for i in range(3)]
            n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ]
            mag = math.sqrt(sum(x * x for x in n)) or 1.0
            n = [x / mag for x in n]
            f.write(struct.pack("<3f", *n))
            for p in (a, b, c):
                f.write(struct.pack("<3f", *p))
            f.write(struct.pack("<H", 0))


def check_watertight(tris):
    """Each undirected edge must appear exactly twice."""
    edges = {}
    for tri in tris:
        pts = [tuple(round(x, 6) for x in p) for p in tri]
        for i in range(3):
            e = frozenset((pts[i], pts[(i + 1) % 3]))
            edges[e] = edges.get(e, 0) + 1
    bad = [e for e, n in edges.items() if n != 2]
    return len(bad) == 0, len(bad)


def quad(a, b, c, d):
    """Two triangles for quad a-b-c-d (counter-clockwise seen from outside)."""
    return [(a, b, c), (a, c, d)]


def box(cx, cy, cz, sx, sy, sz):
    x0, x1 = cx - sx / 2, cx + sx / 2
    y0, y1 = cy - sy / 2, cy + sy / 2
    z0, z1 = cz - sz / 2, cz + sz / 2
    v = {
        (i, j, k): ((x1 if i else x0), (y1 if j else y0), (z1 if k else z0))
        for i in (0, 1)
        for j in (0, 1)
        for k in (0, 1)
    }
    t = []
    t += quad(v[0, 0, 0], v[0, 1, 0], v[1, 1, 0], v[1, 0, 0])  # z0, normal -z
    t += quad(v[0, 0, 1], v[1, 0, 1], v[1, 1, 1], v[0, 1, 1])  # z1, normal +z
    t += quad(v[0, 0, 0], v[1, 0, 0], v[1, 0, 1], v[0, 0, 1])  # y0, normal -y
    t += quad(v[0, 1, 0], v[0, 1, 1], v[1, 1, 1], v[1, 1, 0])  # y1, normal +y
    t += quad(v[0, 0, 0], v[0, 0, 1], v[0, 1, 1], v[0, 1, 0])  # x0, normal -x
    t += quad(v[1, 0, 0], v[1, 1, 0], v[1, 1, 1], v[1, 0, 1])  # x1, normal +x
    return t


def uv_sphere(r, nu=48, nv=24, cx=0.0, cy=0.0, cz=0.0):
    def pt(iu, iv):
        theta = math.pi * iv / nv
        phi = 2 * math.pi * iu / nu
        return (
            cx + r * math.sin(theta) * math.cos(phi),
            cy + r * math.sin(theta) * math.sin(phi),
            cz + r * math.cos(theta),
        )

    t = []
    for iv in range(nv):
        for iu in range(nu):
            p00, p01 = pt(iu, iv), pt(iu, iv + 1)
            p10, p11 = pt(iu + 1, iv), pt(iu + 1, iv + 1)
            # Skip the degenerate half of the quad at each pole.
            if iv < nv - 1:
                t.append((p00, p01, p11))
            if iv > 0:
                t.append((p00, p11, p10))
    return t


def cylinder(r, h, n=64, axis="x"):
    """Closed cylinder centered at origin, axis along `axis`."""

    def swizzle(a, b, c):
        if axis == "x":
            return (c, a, b)
        if axis == "y":
            return (a, c, b)
        return (a, b, c)

    ring0 = [
        swizzle(r * math.cos(2 * math.pi * i / n), r * math.sin(2 * math.pi * i / n), -h / 2)
        for i in range(n)
    ]
    ring1 = [
        swizzle(r * math.cos(2 * math.pi * i / n), r * math.sin(2 * math.pi * i / n), h / 2)
        for i in range(n)
    ]
    c0, c1 = swizzle(0, 0, -h / 2), swizzle(0, 0, h / 2)
    t = []
    for i in range(n):
        j = (i + 1) % n
        t += quad(ring0[i], ring0[j], ring1[j], ring1[i])
        t.append((c0, ring0[j], ring0[i]))
        t.append((c1, ring1[i], ring1[j]))
    return t


def naca0012_wing(chord, span, n=80):
    """Straight NACA 0012 wing: extruded airfoil with flat capped tips."""

    def half_thickness(xc):
        return (
            0.12
            / 0.2
            * chord
            * (
                0.2969 * math.sqrt(xc)
                - 0.1260 * xc
                - 0.3516 * xc**2
                + 0.2843 * xc**3
                - 0.1036 * xc**4  # closed trailing edge variant
            )
        )

    # Cosine-spaced closed loop: TE -> upper -> LE -> lower -> TE
    loop = []
    for i in range(n):
        xc = 0.5 * (1 + math.cos(math.pi * i / (n - 1)))  # 1 -> 0
        loop.append((xc * chord, half_thickness(xc)))
    for i in range(1, n - 1):
        xc = 0.5 * (1 - math.cos(math.pi * i / (n - 1)))  # 0 -> 1
        loop.append((xc * chord, -half_thickness(xc)))

    m = len(loop)
    y0, y1 = -span / 2, span / 2
    t = []
    for i in range(m):
        j = (i + 1) % m
        a = (loop[i][0], y0, loop[i][1])
        b = (loop[j][0], y0, loop[j][1])
        c = (loop[j][0], y1, loop[j][1])
        d = (loop[i][0], y1, loop[i][1])
        t += quad(a, b, c, d)
    # Cap tips with a triangle fan around the section centroid.
    cx = sum(p[0] for p in loop) / m
    cz = sum(p[1] for p in loop) / m
    for i in range(m):
        j = (i + 1) % m
        t.append(((cx, y0, cz), (loop[j][0], y0, loop[j][1]), (loop[i][0], y0, loop[i][1])))
        t.append(((cx, y1, cz), (loop[i][0], y1, loop[i][1]), (loop[j][0], y1, loop[j][1])))
    return t


def capsule_body(r, length, n=40, rings=12):
    """Blunt bluff body: cylinder along x with hemispherical nose and tail."""
    t = []
    half = length / 2 - r
    ring_prev = None

    def ring_at(x, rad):
        return [
            (x, rad * math.cos(2 * math.pi * i / n), rad * math.sin(2 * math.pi * i / n))
            for i in range(n)
        ]

    profile = []
    for k in range(rings + 1):  # nose hemisphere
        a = math.pi / 2 * k / rings
        profile.append((-half - r * math.cos(a), r * math.sin(a)))
    profile.append((half, r))  # straight section
    for k in range(1, rings + 1):  # tail hemisphere
        a = math.pi / 2 * k / rings
        profile.append((half + r * math.sin(a), r * math.cos(a)))

    tip0 = (profile[0][0], 0.0, 0.0)
    tip1 = (profile[-1][0], 0.0, 0.0)
    for x, rad in profile[1:-1]:
        ring = ring_at(x, rad)
        if ring_prev is None:
            for i in range(n):
                j = (i + 1) % n
                t.append((tip0, ring[i], ring[j]))
        else:
            for i in range(n):
                j = (i + 1) % n
                t += quad(ring_prev[i], ring[i], ring[j], ring_prev[j])
        ring_prev = ring
    for i in range(n):
        j = (i + 1) % n
        t.append((tip1, ring_prev[j], ring_prev[i]))
    return t


MODELS = {
    # name: (triangles, note)
    "cube_100mm.stl": (box(0, 0, 0, 100, 100, 100), "sharp-edged bluff body, trivial sanity check"),
    "sphere_d100mm.stl": (uv_sphere(50.0), "canonical smooth bluff body"),
    "cylinder_d60_l200mm.stl": (
        cylinder(30.0, 200.0, axis="y"),
        "classic cross-flow cylinder (axis spanwise)",
    ),
    "naca0012_wing_c120_s300mm.stl": (
        naca0012_wing(120.0, 300.0),
        "streamlined lifting body, thin trailing edge",
    ),
    "capsule_d80_l260mm.stl": (capsule_body(40.0, 260.0), "blunt nose/tail road-vehicle-like body"),
}

if __name__ == "__main__":
    for name, (tris, note) in MODELS.items():
        ok, bad = check_watertight(tris)
        write_stl(OUT / name, tris)
        size_kb = (OUT / name).stat().st_size / 1024
        status = "watertight" if ok else f"NOT WATERTIGHT ({bad} bad edges)"
        print(f"{name:38s} {len(tris):6d} tris  {size_kb:7.1f} KB  {status}  — {note}")
