//! 3D flow field for the viewport. Particles advected through an ABC/Beltrami
//! field (an exact 3D vortex structure), coloured by signed vorticity. This is a
//! real, interactive visualization now; the seam `regenerate()` is where the
//! Python engine's real predicted field will plug in later.

#[derive(Clone, Copy)]
pub struct Particle {
    pub pos: [f32; 3], // in [-1, 1]^3
    pub vort: f32,     // signed, normalized ~[-1, 1] (blue < 0 < ember)
    pub speed: f32,    // [0, 1]
}

/// ABC (Arnold–Beltrami–Childress) velocity — swirling Beltrami vortex lines.
fn abc(p: [f32; 3]) -> [f32; 3] {
    let (x, y, z) = (p[0], p[1], p[2]);
    [z.sin() + y.cos(), x.sin() + z.cos(), y.sin() + x.cos()]
}

fn curl_x(p: [f32; 3]) -> f32 {
    // ω_x = ∂w/∂y - ∂v/∂z ; for ABC this equals u_x (Beltrami), gives a smooth signed field
    let h = 0.01;
    let dwdy = (abc([p[0], p[1] + h, p[2]])[2] - abc([p[0], p[1] - h, p[2]])[2]) / (2.0 * h);
    let dvdz = (abc([p[0], p[1], p[2] + h])[1] - abc([p[0], p[1], p[2] - h])[1]) / (2.0 * h);
    dwdy - dvdz
}

pub fn generate(n: usize, seed: u64) -> Vec<Particle> {
    let mut rng = seed.wrapping_mul(0x9E3779B97F4A7C15).max(1);
    let mut rnd = || {
        rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        (rng >> 11) as f32 / (1u64 << 53) as f32
    };
    let scale = std::f32::consts::PI; // domain [-π, π] for ABC, mapped to [-1,1] at the end
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let mut p = [
            (rnd() * 2.0 - 1.0) * scale,
            (rnd() * 2.0 - 1.0) * scale,
            (rnd() * 2.0 - 1.0) * scale,
        ];
        // advect along the flow so particles collect onto vortex tubes
        let dt = 0.06;
        for _ in 0..36 {
            let v = abc(p);
            for k in 0..3 { p[k] += v[k] * dt; }
        }
        let v = abc(p);
        let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() / 3.0_f32.sqrt();
        let vort = (curl_x(p) / 2.0).clamp(-1.0, 1.0);
        // wrap into [-π,π] then normalize to [-1,1]
        let wrap = |a: f32| {
            let m = (a + scale).rem_euclid(2.0 * scale) - scale;
            m / scale
        };
        out.push(Particle {
            pos: [wrap(p[0]), wrap(p[1]), wrap(p[2])],
            vort,
            speed: speed.clamp(0.0, 1.0),
        });
    }
    out
}

/// Convert a real engine velocity field `[3, N, N, N]` (row-major, C first) into
/// render particles: sample voxels, take the vorticity (curl) for signed colour,
/// speed for brightness, position normalized to [-1, 1]. This is the real model
/// output driving the viewport.
pub fn from_field(shape: &[usize], data: &[f32]) -> Vec<Particle> {
    if shape.len() != 4 || shape[0] != 3 { return Vec::new(); }
    let (nx, ny, nz) = (shape[1], shape[2], shape[3]);
    if data.len() < 3 * nx * ny * nz { return Vec::new(); }
    let at = |c: usize, i: usize, j: usize, k: usize| data[((c * nx + i) * ny + j) * nz + k];

    let target = 8000usize;
    let stride = (((nx * ny * nz) as f32 / target as f32).cbrt().floor() as usize).max(1);
    // per-sample jitter breaks the visible lattice so it reads as a field
    let cell = 2.0 / (nx.max(2) - 1) as f32 * stride as f32;
    let hash = |a: usize, b: usize, c: usize, salt: u32| -> f32 {
        let mut h = (a as u32).wrapping_mul(73856093) ^ (b as u32).wrapping_mul(19349663)
            ^ (c as u32).wrapping_mul(83492791) ^ salt.wrapping_mul(2654435761);
        h ^= h >> 13; h = h.wrapping_mul(0x5bd1e995); h ^= h >> 15;
        (h as f32 / u32::MAX as f32) - 0.5
    };
    let mut raw: Vec<([f32; 3], f32, f32)> = Vec::new();
    let (mut maxv, mut maxs) = (1e-6f32, 1e-6f32);
    let mut i = 1;
    while i < nx - 1 {
        let mut j = 1;
        while j < ny - 1 {
            let mut k = 1;
            while k < nz - 1 {
                let wx = (at(2, i, j + 1, k) - at(2, i, j - 1, k)) - (at(1, i, j, k + 1) - at(1, i, j, k - 1));
                let wy = (at(0, i, j, k + 1) - at(0, i, j, k - 1)) - (at(2, i + 1, j, k) - at(2, i - 1, j, k));
                let wz = (at(1, i + 1, j, k) - at(1, i - 1, j, k)) - (at(0, i, j + 1, k) - at(0, i, j - 1, k));
                let mag = (wx * wx + wy * wy + wz * wz).sqrt();
                let (u, v, w) = (at(0, i, j, k), at(1, i, j, k), at(2, i, j, k));
                let sp = (u * u + v * v + w * w).sqrt();
                maxv = maxv.max(mag);
                maxs = maxs.max(sp);
                let pos = [
                    (i as f32 / (nx - 1) as f32 * 2.0 - 1.0) + hash(i, j, k, 1) * 0.9 * cell,
                    (j as f32 / (ny - 1) as f32 * 2.0 - 1.0) + hash(i, j, k, 2) * 0.9 * cell,
                    (k as f32 / (nz - 1) as f32 * 2.0 - 1.0) + hash(i, j, k, 3) * 0.9 * cell,
                ];
                raw.push((pos, wx.signum() * mag, sp));
                k += stride;
            }
            j += stride;
        }
        i += stride;
    }
    raw.into_iter().map(|(pos, vort, sp)| Particle {
        pos, vort: (vort / maxv).clamp(-1.0, 1.0), speed: (sp / maxs).clamp(0.0, 1.0),
    }).collect()
}

/// Vorticity-magnitude scalar volume from an engine velocity field `[3,N,N,N]`,
/// normalized to `[0,1]` and laid out for a wgpu 3D texture (x = i fastest,
/// then j, then k). Feeds the volume raymarch. Returns `(bytes, [nx,ny,nz])`.
pub fn vorticity_volume(shape: &[usize], data: &[f32]) -> Option<(Vec<u8>, [u32; 3])> {
    if shape.len() != 4 || shape[0] != 3 { return None; }
    let (nx, ny, nz) = (shape[1], shape[2], shape[3]);
    if nx < 2 || ny < 2 || nz < 2 || data.len() < 3 * nx * ny * nz { return None; }
    let at = |c: usize, i: usize, j: usize, k: usize| data[((c * nx + i) * ny + j) * nz + k];
    let cl = |v: i64, n: usize| v.clamp(0, n as i64 - 1) as usize;
    let mut mag = vec![0f32; nx * ny * nz];
    let mut maxv = 1e-6f32;
    for i in 0..nx {
        let (ip, im) = (cl(i as i64 + 1, nx), cl(i as i64 - 1, nx));
        for j in 0..ny {
            let (jp, jm) = (cl(j as i64 + 1, ny), cl(j as i64 - 1, ny));
            for k in 0..nz {
                let (kp, km) = (cl(k as i64 + 1, nz), cl(k as i64 - 1, nz));
                let wx = (at(2, i, jp, k) - at(2, i, jm, k)) - (at(1, i, j, kp) - at(1, i, j, km));
                let wy = (at(0, i, j, kp) - at(0, i, j, km)) - (at(2, ip, j, k) - at(2, im, j, k));
                let wz = (at(1, ip, j, k) - at(1, im, j, k)) - (at(0, i, jp, k) - at(0, i, jm, k));
                let m = (wx * wx + wy * wy + wz * wz).sqrt();
                mag[(k * ny + j) * nx + i] = m;
                if m > maxv { maxv = m; }
            }
        }
    }
    let bytes = mag.iter().map(|m| ((m / maxv).clamp(0.0, 1.0) * 255.0) as u8).collect();
    Some((bytes, [nx as u32, ny as u32, nz as u32]))
}

/// Procedural |ω| volume (ABC/Beltrami, where curl u = u so |ω| = |u|) for the
/// placeholder before a model field arrives. `seed` phase-shifts it so
/// regenerate visibly changes the view.
pub fn procedural_volume(n: usize, seed: u64) -> (Vec<u8>, [u32; 3]) {
    let sc = std::f32::consts::PI;
    let ph = (seed as f32) * 0.7;
    let mut vals = vec![0f32; n * n * n];
    let mut maxv = 1e-6f32;
    let coord = |a: usize| (a as f32 / (n - 1) as f32 * 2.0 - 1.0) * sc;
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                let v = abc([coord(i) + ph, coord(j) + ph, coord(k) + ph]);
                let m = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                vals[(k * n + j) * n + i] = m;
                if m > maxv { maxv = m; }
            }
        }
    }
    let bytes = vals.iter().map(|m| ((m / maxv).clamp(0.0, 1.0) * 255.0) as u8).collect();
    (bytes, [n as u32, n as u32, n as u32])
}
