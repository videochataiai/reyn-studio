//! CAD import: STL (binary + ASCII) → voxel obstacle mask for the immersed-mask
//! pipeline, plus recovered-pressure surface analysis. Maxima identify
//! stagnation/load points; minima identify suction/low-pressure points.
//!
//! Voxelization is the standard ray-parity test (cast an axis ray per grid row,
//! count triangle crossings, odd = inside) — the same approach GPU CFD codes use
//! for STL→lattice classification. The mesh is auto-fit to the solver's
//! training envelope: centred in the tunnel with a cross-stream size inside the
//! band the 3D obstacle models were trained on.

use crate::flow::{Insight3D, Insight3DKind};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub struct Mesh {
    pub tris: Vec<[[f32; 3]; 3]>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshDiagnostics {
    pub triangles: usize,
    pub components: usize,
    pub degenerate_triangles: usize,
    pub boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub inconsistent_winding_edges: usize,
    pub extents: [f32; 3],
}

/// Topology and scale checks performed before solver preprocessing. STL has no
/// unit metadata, so extents are intentionally reported in source units.
pub fn diagnose_mesh(mesh: &Mesh) -> MeshDiagnostics {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for triangle in &mesh.tris {
        for vertex in triangle {
            for axis in 0..3 {
                lo[axis] = lo[axis].min(vertex[axis]);
                hi[axis] = hi[axis].max(vertex[axis]);
            }
        }
    }
    let extents = if mesh.tris.is_empty() {
        [0.0; 3]
    } else {
        [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]
    };
    let diagonal = (extents[0] * extents[0] + extents[1] * extents[1] + extents[2] * extents[2])
        .sqrt()
        .max(1.0);
    let tolerance = diagonal * 1e-6;
    let key = |vertex: [f32; 3]| -> [i64; 3] {
        [
            (vertex[0] / tolerance).round() as i64,
            (vertex[1] / tolerance).round() as i64,
            (vertex[2] / tolerance).round() as i64,
        ]
    };
    let mut degenerate_triangles = 0usize;
    let mut edge_uses: HashMap<([i64; 3], [i64; 3]), Vec<(usize, bool)>> = HashMap::new();
    for (triangle_index, triangle) in mesh.tris.iter().enumerate() {
        let ab = [
            triangle[1][0] - triangle[0][0],
            triangle[1][1] - triangle[0][1],
            triangle[1][2] - triangle[0][2],
        ];
        let ac = [
            triangle[2][0] - triangle[0][0],
            triangle[2][1] - triangle[0][1],
            triangle[2][2] - triangle[0][2],
        ];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        if cross.iter().map(|value| value * value).sum::<f32>().sqrt() <= tolerance * tolerance {
            degenerate_triangles += 1;
        }
        let vertices = [key(triangle[0]), key(triangle[1]), key(triangle[2])];
        for (start, end) in [(0, 1), (1, 2), (2, 0)] {
            let a = vertices[start];
            let b = vertices[end];
            let (canonical, forward) = if a <= b {
                ((a, b), true)
            } else {
                ((b, a), false)
            };
            edge_uses
                .entry(canonical)
                .or_default()
                .push((triangle_index, forward));
        }
    }
    let boundary_edges = edge_uses.values().filter(|uses| uses.len() == 1).count();
    let non_manifold_edges = edge_uses.values().filter(|uses| uses.len() > 2).count();
    let inconsistent_winding_edges = edge_uses
        .values()
        .filter(|uses| uses.len() == 2 && uses[0].1 == uses[1].1)
        .count();

    let mut parent: Vec<usize> = (0..mesh.tris.len()).collect();
    fn root(parent: &mut [usize], mut index: usize) -> usize {
        while parent[index] != index {
            parent[index] = parent[parent[index]];
            index = parent[index];
        }
        index
    }
    for uses in edge_uses.values() {
        if let Some((first, _)) = uses.first() {
            for (other, _) in uses.iter().skip(1) {
                let first_root = root(&mut parent, *first);
                let other_root = root(&mut parent, *other);
                parent[other_root] = first_root;
            }
        }
    }
    let mut component_roots = std::collections::HashSet::new();
    for triangle_index in 0..mesh.tris.len() {
        component_roots.insert(root(&mut parent, triangle_index));
    }
    MeshDiagnostics {
        triangles: mesh.tris.len(),
        components: component_roots.len(),
        degenerate_triangles,
        boundary_edges,
        non_manifold_edges,
        inconsistent_winding_edges,
        extents,
    }
}

/// Parse an STL file (auto-detects binary vs ASCII).
pub fn parse_stl(bytes: &[u8]) -> Result<Mesh, String> {
    if bytes.len() < 15 {
        return Err("file too small to be an STL".into());
    }
    // binary layout: 80-byte header | u32 count | count × 50 bytes
    if bytes.len() >= 84 {
        let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
        if bytes.len() == 84 + count * 50 {
            let mut tris = Vec::with_capacity(count);
            for t in 0..count {
                let base = 84 + t * 50 + 12; // skip the normal
                let f = |o: usize| {
                    f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]])
                };
                let mut tri = [[0f32; 3]; 3];
                for (v, vert) in tri.iter_mut().enumerate() {
                    for (c, x) in vert.iter_mut().enumerate() {
                        *x = f(base + v * 12 + c * 4);
                    }
                }
                tris.push(tri);
            }
            return Ok(Mesh { tris });
        }
    }
    // ASCII: "solid ... facet normal ... vertex x y z"
    let text = std::str::from_utf8(bytes).map_err(|_| "neither binary nor ASCII STL")?;
    let mut tris = Vec::new();
    let mut verts: Vec<[f32; 3]> = Vec::with_capacity(3);
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("vertex") {
            let mut it = rest
                .split_whitespace()
                .filter_map(|s| s.parse::<f32>().ok());
            let v = [it.next(), it.next(), it.next()];
            if let (Some(x), Some(y), Some(z)) = (v[0], v[1], v[2]) {
                verts.push([x, y, z]);
                if verts.len() == 3 {
                    tris.push([verts[0], verts[1], verts[2]]);
                    verts.clear();
                }
            }
        }
    }
    if tris.is_empty() {
        return Err("no triangles found in STL".into());
    }
    Ok(Mesh { tris })
}

pub struct VoxelMask {
    pub n: usize,
    pub mask: Vec<f32>, // [n³], layout [i*n*n + j*n + k] matching engine fields
    pub solid_voxels: usize,
    pub char_len: f32, // cross-stream extent in solver units (Re scaling)
    pub components: usize,
    pub minimum_cells_across: usize,
    pub boundary_clearance_cells: usize,
    pub transform_4x4: [f64; 16],
}

/// Voxelize a mesh onto the solver grid `[0,2π]³` at resolution `n`: the mesh is
/// scaled so its largest cross-stream (y/z) extent is `0.6` solver units (inside
/// the trained π/7..π/4 band) and centred at the training obstacle station
/// (x=0.65π, y=z=π). Ray-parity along +x per (y,z) row, with a tiny jitter so
/// rays never thread triangle edges exactly.
pub fn voxelize(mesh: &Mesh, n: usize) -> Result<VoxelMask, String> {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for t in &mesh.tris {
        for v in t {
            for c in 0..3 {
                lo[c] = lo[c].min(v[c]);
                hi[c] = hi[c].max(v[c]);
            }
        }
    }
    let ext = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let cross = ext[1].max(ext[2]).max(1e-9);
    let target_char = 0.6f32; // solver units, mid training band
    let scale = target_char / cross;
    let tau = std::f32::consts::TAU;
    let centre = [
        0.65 * std::f32::consts::PI,
        std::f32::consts::PI,
        std::f32::consts::PI,
    ];
    let mid = [
        (lo[0] + hi[0]) * 0.5,
        (lo[1] + hi[1]) * 0.5,
        (lo[2] + hi[2]) * 0.5,
    ];
    let map = |v: [f32; 3]| {
        [
            centre[0] + (v[0] - mid[0]) * scale,
            centre[1] + (v[1] - mid[1]) * scale,
            centre[2] + (v[2] - mid[2]) * scale,
        ]
    };
    let tris: Vec<[[f32; 3]; 3]> = mesh
        .tris
        .iter()
        .map(|t| [map(t[0]), map(t[1]), map(t[2])])
        .collect();

    // bin triangles by their (y,z) bounding boxes so each row only tests a few
    let dx = tau / n as f32;
    let cell_of = |w: f32| ((w / dx) as i64).clamp(0, n as i64 - 1) as usize;
    let mut bins: Vec<Vec<u32>> = vec![Vec::new(); n * n];
    for (ti, t) in tris.iter().enumerate() {
        let (mut ylo, mut yhi, mut zlo, mut zhi) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for v in t {
            ylo = ylo.min(v[1]);
            yhi = yhi.max(v[1]);
            zlo = zlo.min(v[2]);
            zhi = zhi.max(v[2]);
        }
        for j in cell_of(ylo)..=cell_of(yhi) {
            for k in cell_of(zlo)..=cell_of(zhi) {
                bins[j * n + k].push(ti as u32);
            }
        }
    }

    let mut mask = vec![0f32; n * n * n];
    let mut solid = 0usize;
    for j in 0..n {
        let y = (j as f32 + 0.5) * dx + 1.3e-4; // jitter off exact edges
        for k in 0..n {
            let z = (k as f32 + 0.5) * dx + 2.7e-4;
            let bin = &bins[j * n + k];
            if bin.is_empty() {
                continue;
            }
            let mut hits: Vec<f32> = Vec::new();
            for &ti in bin {
                if let Some(x) = ray_x_hit(&tris[ti as usize], y, z) {
                    hits.push(x);
                }
            }
            if hits.len() < 2 {
                continue;
            }
            hits.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut m = 0;
            while m + 1 < hits.len() {
                let (x0, x1) = (hits[m], hits[m + 1]);
                let (i0, i1) = (
                    ((x0 / dx) - 0.5).ceil().max(0.0) as usize,
                    (((x1 / dx) - 0.5).floor() as i64).min(n as i64 - 1),
                );
                for i in i0..=(i1.max(0) as usize) {
                    let c = (i as f32 + 0.5) * dx;
                    if c >= x0 && c <= x1 && i < n {
                        let idx = i * n * n + j * n + k;
                        if mask[idx] == 0.0 {
                            solid += 1;
                        }
                        mask[idx] = 1.0;
                    }
                }
                m += 2;
            }
        }
    }
    if solid == 0 {
        return Err("voxelization produced an empty solid (is the mesh watertight?)".into());
    }
    let (components, minimum_cells_across, boundary_clearance_cells) = voxel_diagnostics(&mask, n);
    let transform_4x4 = [
        scale as f64,
        0.0,
        0.0,
        0.0,
        0.0,
        scale as f64,
        0.0,
        0.0,
        0.0,
        0.0,
        scale as f64,
        0.0,
        (centre[0] - mid[0] * scale) as f64,
        (centre[1] - mid[1] * scale) as f64,
        (centre[2] - mid[2] * scale) as f64,
        1.0,
    ];
    Ok(VoxelMask {
        n,
        mask,
        solid_voxels: solid,
        char_len: target_char,
        components,
        minimum_cells_across,
        boundary_clearance_cells,
        transform_4x4,
    })
}

fn voxel_diagnostics(mask: &[f32], n: usize) -> (usize, usize, usize) {
    let index = |i: usize, j: usize, k: usize| i * n * n + j * n + k;
    let mut bounds_lo = [n; 3];
    let mut bounds_hi = [0usize; 3];
    let mut occupied = false;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                if mask[index(i, j, k)] > 0.5 {
                    occupied = true;
                    bounds_lo[0] = bounds_lo[0].min(i);
                    bounds_lo[1] = bounds_lo[1].min(j);
                    bounds_lo[2] = bounds_lo[2].min(k);
                    bounds_hi[0] = bounds_hi[0].max(i);
                    bounds_hi[1] = bounds_hi[1].max(j);
                    bounds_hi[2] = bounds_hi[2].max(k);
                }
            }
        }
    }
    if !occupied {
        return (0, 0, 0);
    }
    let minimum_cells_across = (0..3)
        .map(|axis| bounds_hi[axis] - bounds_lo[axis] + 1)
        .min()
        .unwrap_or(0);
    let boundary_clearance_cells = (0..3)
        .flat_map(|axis| [bounds_lo[axis], n - 1 - bounds_hi[axis]])
        .min()
        .unwrap_or(0);

    let mut visited = vec![false; mask.len()];
    let mut components = 0usize;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let start = index(i, j, k);
                if mask[start] <= 0.5 || visited[start] {
                    continue;
                }
                components += 1;
                visited[start] = true;
                let mut queue = VecDeque::from([(i, j, k)]);
                while let Some((ci, cj, ck)) = queue.pop_front() {
                    for (di, dj, dk) in [
                        (-1isize, 0isize, 0isize),
                        (1, 0, 0),
                        (0, -1, 0),
                        (0, 1, 0),
                        (0, 0, -1),
                        (0, 0, 1),
                    ] {
                        let ni = ci as isize + di;
                        let nj = cj as isize + dj;
                        let nk = ck as isize + dk;
                        if ni < 0
                            || nj < 0
                            || nk < 0
                            || ni >= n as isize
                            || nj >= n as isize
                            || nk >= n as isize
                        {
                            continue;
                        }
                        let neighbor = index(ni as usize, nj as usize, nk as usize);
                        if mask[neighbor] > 0.5 && !visited[neighbor] {
                            visited[neighbor] = true;
                            queue.push_back((ni as usize, nj as usize, nk as usize));
                        }
                    }
                }
            }
        }
    }
    (components, minimum_cells_across, boundary_clearance_cells)
}

/// x-coordinate where the +x ray through (y, z) crosses the triangle, if it does
/// (2D point-in-triangle in the yz-plane, then interpolate x).
fn ray_x_hit(t: &[[f32; 3]; 3], y: f32, z: f32) -> Option<f32> {
    let (a, b, c) = (t[0], t[1], t[2]);
    let d = (b[1] - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (b[2] - a[2]);
    if d.abs() < 1e-12 {
        return None;
    } // degenerate in yz (edge-on to the ray)
    let w1 = ((y - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (z - a[2])) / d;
    let w2 = ((b[1] - a[1]) * (z - a[2]) - (y - a[1]) * (b[2] - a[2])) / d;
    if w1 < 0.0 || w2 < 0.0 || w1 + w2 > 1.0 {
        return None;
    }
    Some(a[0] + w1 * (b[0] - a[0]) + w2 * (c[0] - a[0]))
}

/// Recovered-pressure surface insights: scan fluid voxels adjacent to the solid
/// and pin the maximum (stagnation/load point) and minimum (suction/low-pressure
/// point). These density-normalized recovered values are not a physical pressure
/// coefficient.
pub fn surface_insights(mask: &[f32], p: &[f32], n: usize) -> Vec<Insight3D> {
    let at = |v: &[f32], i: usize, j: usize, k: usize| v[i * n * n + j * n + k];
    let mut hi = (f32::MIN, [0usize; 3]);
    let mut lo = (f32::MAX, [0usize; 3]);
    let mut found = false;
    for i in 1..n - 1 {
        for j in 1..n - 1 {
            for k in 1..n - 1 {
                if at(mask, i, j, k) > 0.5 {
                    continue;
                } // want FLUID cells
                let near_solid = at(mask, i + 1, j, k) > 0.5
                    || at(mask, i - 1, j, k) > 0.5
                    || at(mask, i, j + 1, k) > 0.5
                    || at(mask, i, j - 1, k) > 0.5
                    || at(mask, i, j, k + 1) > 0.5
                    || at(mask, i, j, k - 1) > 0.5;
                if !near_solid {
                    continue;
                }
                found = true;
                let pv = at(p, i, j, k);
                if pv > hi.0 {
                    hi = (pv, [i, j, k]);
                }
                if pv < lo.0 {
                    lo = (pv, [i, j, k]);
                }
            }
        }
    }
    if !found {
        return Vec::new();
    }
    let to_pos = |c: [usize; 3]| {
        [
            c[0] as f32 / (n - 1) as f32 * 2.0 - 1.0,
            c[1] as f32 / (n - 1) as f32 * 2.0 - 1.0,
            c[2] as f32 / (n - 1) as f32 * 2.0 - 1.0,
        ]
    };
    vec![
        Insight3D {
            kind: Insight3DKind::SurfLoad,
            pos: to_pos(hi.1),
            value: hi.0,
        },
        Insight3D {
            kind: Insight3DKind::SurfSuction,
            pos: to_pos(lo.1),
            value: lo.0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A binary STL of an axis-aligned cuboid (12 triangles).
    fn cube_stl(lo: [f32; 3], hi: [f32; 3]) -> Vec<u8> {
        let v = |x: usize, y: usize, z: usize| {
            [
                if x == 0 { lo[0] } else { hi[0] },
                if y == 0 { lo[1] } else { hi[1] },
                if z == 0 { lo[2] } else { hi[2] },
            ]
        };
        // 12 triangles, two per face
        let quads = [
            [v(0, 0, 0), v(0, 1, 0), v(0, 1, 1), v(0, 0, 1)], // x-
            [v(1, 0, 0), v(1, 0, 1), v(1, 1, 1), v(1, 1, 0)], // x+
            [v(0, 0, 0), v(0, 0, 1), v(1, 0, 1), v(1, 0, 0)], // y-
            [v(0, 1, 0), v(1, 1, 0), v(1, 1, 1), v(0, 1, 1)], // y+
            [v(0, 0, 0), v(1, 0, 0), v(1, 1, 0), v(0, 1, 0)], // z-
            [v(0, 0, 1), v(0, 1, 1), v(1, 1, 1), v(1, 0, 1)], // z+
        ];
        let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
        for q in quads {
            tris.push([q[0], q[1], q[2]]);
            tris.push([q[0], q[2], q[3]]);
        }
        let mut out = vec![0u8; 80];
        out.extend_from_slice(&(tris.len() as u32).to_le_bytes());
        for t in tris {
            out.extend_from_slice(&[0u8; 12]); // normal (unused)
            for vert in t {
                for c in vert {
                    out.extend_from_slice(&c.to_le_bytes());
                }
            }
            out.extend_from_slice(&[0u8; 2]); // attribute
        }
        out
    }

    #[test]
    fn parses_binary_and_ascii() {
        let bin = cube_stl([0.0; 3], [1.0; 3]);
        let m = parse_stl(&bin).expect("binary parse");
        assert_eq!(m.tris.len(), 12);

        let ascii = "solid cube\n facet normal 0 0 0\n  outer loop\n   vertex 0 0 0\n   vertex 1 0 0\n   vertex 0 1 0\n  endloop\n endfacet\nendsolid";
        let m = parse_stl(ascii.as_bytes()).expect("ascii parse");
        assert_eq!(m.tris.len(), 1);
    }

    #[test]
    fn topology_preflight_distinguishes_closed_and_open_sources() {
        let cube = parse_stl(&cube_stl([0.0; 3], [1.0; 3])).unwrap();
        let closed = diagnose_mesh(&cube);
        assert_eq!(closed.triangles, 12);
        assert_eq!(closed.components, 1);
        assert_eq!(closed.boundary_edges, 0);
        assert_eq!(closed.non_manifold_edges, 0);
        assert_eq!(closed.degenerate_triangles, 0);

        let open = Mesh {
            tris: vec![[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]],
        };
        let open = diagnose_mesh(&open);
        assert_eq!(open.boundary_edges, 3);
        assert_eq!(open.components, 1);
    }

    #[test]
    fn voxelized_cube_fills_the_right_box() {
        // a tall cuboid: y/z cross-section 1×1 (the char dimension), x length 2
        let mesh = parse_stl(&cube_stl([0.0, 0.0, 0.0], [2.0, 1.0, 1.0])).unwrap();
        let n = 32;
        let vm = voxelize(&mesh, n).expect("voxelize");
        assert_eq!(vm.n, n);
        // char target 0.6 solver units at dx = 2π/32 ≈ 0.196 → ~3 cells across,
        // x is 2× the cross-section → ~6 cells; expect roughly 3*3*6 ≈ 54 solid
        assert!(
            vm.solid_voxels >= 18 && vm.solid_voxels <= 200,
            "unexpected solid count {}",
            vm.solid_voxels
        );
        // the solid must be near the training obstacle station (x≈0.65π→i≈10, y=z≈π→16)
        let mut ci = 0f32;
        let mut cj = 0f32;
        let mut ck = 0f32;
        let mut m = 0f32;
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let w = vm.mask[i * n * n + j * n + k];
                    ci += w * i as f32;
                    cj += w * j as f32;
                    ck += w * k as f32;
                    m += w;
                }
            }
        }
        let (ci, cj, ck) = (ci / m, cj / m, ck / m);
        assert!((ci - 0.325 * n as f32).abs() < 2.0, "centroid x off: {ci}");
        assert!((cj - 0.5 * n as f32).abs() < 2.0, "centroid y off: {cj}");
        assert!((ck - 0.5 * n as f32).abs() < 2.0, "centroid z off: {ck}");
    }

    #[test]
    fn surface_insights_find_load_and_suction() {
        let n = 12;
        let mut mask = vec![0f32; n * n * n];
        let mut p = vec![0f32; n * n * n];
        // a solid block in the middle
        for i in 4..8 {
            for j in 4..8 {
                for k in 4..8 {
                    mask[i * n * n + j * n + k] = 1.0;
                }
            }
        }
        // pressure: high in front of the block (i=3 face), low behind (i=8)
        p[3 * n * n + 5 * n + 5] = 2.0;
        p[8 * n * n + 6 * n + 6] = -1.5;
        let out = surface_insights(&mask, &p, n);
        assert_eq!(out.len(), 2);
        let load = out
            .iter()
            .find(|x| x.kind == Insight3DKind::SurfLoad)
            .unwrap();
        let suck = out
            .iter()
            .find(|x| x.kind == Insight3DKind::SurfSuction)
            .unwrap();
        assert!((load.value - 2.0).abs() < 1e-6);
        assert!((suck.value + 1.5).abs() < 1e-6);
    }
}
