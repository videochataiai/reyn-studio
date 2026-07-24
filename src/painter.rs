//! N4 — Flow Painter: paint a 2D vorticity field, make it a legal incompressible
//! initial condition, hand it to the model.
//!
//! The projection is native and exact-by-construction: painted ω → streamfunction
//! ψ (∇²ψ = −ω, conjugate-gradient on the periodic 5-point Laplacian, f64) →
//! u = ∂ψ/∂y, v = −∂ψ/∂x with central differences. Discrete central differences
//! commute, so div u = ∂ₓ∂ᵧψ − ∂ᵧ∂ₓψ = 0 to machine epsilon regardless of how
//! tightly ψ converged — CG accuracy only affects how faithfully ω is reproduced,
//! never divergence-freeness. Everything runs client-side (a 128² CG solve is
//! milliseconds native); the engine is only involved when the flow is generated.

pub const N: usize = 128; // must match the 2D model grid

pub struct PaintField {
    pub omega: Vec<f32>, // [N*N], row-major (i = row/y, j = col/x)
    /// projected velocity (u, v) — valid until the next stroke edits ω
    pub velocity: Option<(Vec<f32>, Vec<f32>)>,
    pub div_max: f64, // max |∇·u| after projection
    pub energy: f64,  // ½ Σ (u²+v²) / N²
    pub cg_iters: usize,
}

impl Default for PaintField {
    fn default() -> Self {
        Self {
            omega: vec![0.0; N * N],
            velocity: None,
            div_max: 0.0,
            energy: 0.0,
            cg_iters: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Symmetry {
    pub mirror_h: bool, // mirror across the vertical axis (left↔right)
    pub mirror_v: bool, // mirror across the horizontal axis (top↔bottom)
    pub radial: u32,    // rotational copies about the centre (1 = off)
}

impl Default for Symmetry {
    fn default() -> Self {
        Self {
            mirror_h: false,
            mirror_v: false,
            radial: 1,
        }
    }
}

impl PaintField {
    pub fn clear(&mut self) {
        self.omega.iter_mut().for_each(|w| *w = 0.0);
        self.velocity = None;
    }

    pub fn mean_enstrophy(&self) -> f64 {
        self.omega
            .iter()
            .map(|&w| 0.5 * (w as f64) * (w as f64))
            .sum::<f64>()
            / (N * N) as f64
    }

    /// Gaussian ω splat at grid position (x, y) in cells; `sign` from the mouse
    /// button. Symmetries stamp mirrored/rotated copies — mirrors flip the sign
    /// (vorticity is a pseudoscalar: a reflected vortex counter-rotates), proper
    /// rotations keep it.
    pub fn stamp(&mut self, x: f32, y: f32, radius: f32, strength: f32, sign: f32, sym: Symmetry) {
        let c = (N as f32 - 1.0) * 0.5;
        let mut targets: Vec<(f32, f32, f32)> = vec![(x, y, sign)];
        if sym.mirror_h {
            for k in 0..targets.len() {
                let (tx, ty, ts) = targets[k];
                targets.push((2.0 * c - tx, ty, -ts));
            }
        }
        if sym.mirror_v {
            for k in 0..targets.len() {
                let (tx, ty, ts) = targets[k];
                targets.push((tx, 2.0 * c - ty, -ts));
            }
        }
        if sym.radial > 1 {
            let base = targets.clone();
            for r in 1..sym.radial {
                let a = r as f32 * std::f32::consts::TAU / sym.radial as f32;
                let (sa, ca) = a.sin_cos();
                for &(tx, ty, ts) in &base {
                    let (dx, dy) = (tx - c, ty - c);
                    targets.push((c + dx * ca - dy * sa, c + dx * sa + dy * ca, ts));
                }
            }
        }
        for (tx, ty, ts) in targets {
            self.splat(tx, ty, radius, strength * ts);
        }
        self.velocity = None; // projection is now stale
    }

    fn splat(&mut self, x: f32, y: f32, radius: f32, amp: f32) {
        let r = radius.max(1.0);
        let ext = (r * 3.0).ceil() as i64;
        let (xi, yi) = (x.round() as i64, y.round() as i64);
        for dj in -ext..=ext {
            for di in -ext..=ext {
                let (i, j) = (yi + di, xi + dj);
                if i < 0 || j < 0 || i >= N as i64 || j >= N as i64 {
                    continue;
                }
                let d2 = ((i as f32 - y).powi(2) + (j as f32 - x).powi(2)) / (r * r);
                if d2 > 9.0 {
                    continue;
                }
                self.omega[i as usize * N + j as usize] += amp * (-2.0 * d2).exp();
            }
        }
    }

    // -- presets (all zero-mean by construction: the torus requires ∮ω = 0) ----

    /// Counter-rotating vortex pair — the classic dipole that self-propels.
    pub fn preset_vortex_pair(&mut self) {
        self.clear();
        let c = N as f32 * 0.5;
        self.splat(c - N as f32 * 0.13, c, N as f32 * 0.07, 2.2);
        self.splat(c + N as f32 * 0.13, c, N as f32 * 0.07, -2.2);
    }

    /// Double shear layer — two opposite-sign vorticity strips (the standard
    /// periodic CFD test; rolls up into vortices).
    pub fn preset_shear_layer(&mut self) {
        self.clear();
        let (y1, y2) = (N as f32 * 0.3, N as f32 * 0.7);
        let th = N as f32 * 0.02;
        for i in 0..N {
            for j in 0..N {
                let y = i as f32;
                let s1 = ((y - y1) / th).powi(2);
                let s2 = ((y - y2) / th).powi(2);
                // small sinusoidal modulation seeds the roll-up
                let seed = 1.0 + 0.08 * (j as f32 / N as f32 * std::f32::consts::TAU * 2.0).sin();
                self.omega[i * N + j] += 1.8 * seed * ((-s1).exp() - (-s2).exp());
            }
        }
        self.velocity = None;
    }

    /// Kármán-street-like stamp: two staggered rows of alternating vortices.
    pub fn preset_karman_street(&mut self) {
        self.clear();
        let r = N as f32 * 0.035;
        let (y_top, y_bot) = (N as f32 * 0.42, N as f32 * 0.58);
        let k = 4;
        for m in 0..k {
            let x = N as f32 * (0.14 + 0.72 * m as f32 / (k - 1) as f32);
            let stagger = N as f32 * 0.36 / (k - 1) as f32;
            self.splat(x, y_top, r, 2.4);
            self.splat((x + stagger * 0.5).min(N as f32 - 2.0), y_bot, r, -2.4);
        }
        self.velocity = None;
    }

    // -- the projection ---------------------------------------------------------

    /// ω → ψ → (u, v): an exactly divergence-free velocity field. Returns after
    /// storing the velocity, the max |div| check, energy, and CG iterations.
    pub fn project(&mut self, tol: f64, max_iter: usize) {
        // zero-mean gauge (the periodic Poisson problem requires it)
        let mean = self.omega.iter().map(|&w| w as f64).sum::<f64>() / (N * N) as f64;
        let rhs: Vec<f64> = self.omega.iter().map(|&w| w as f64 - mean).collect();

        // CG on A ψ = rhs with A = −∇² (SPD on the zero-mean subspace)
        let lap = |p: &[f64], out: &mut [f64]| {
            for i in 0..N {
                let (ip, im) = ((i + 1) % N, (i + N - 1) % N);
                for j in 0..N {
                    let (jp, jm) = ((j + 1) % N, (j + N - 1) % N);
                    out[i * N + j] = p[ip * N + j] + p[im * N + j] + p[i * N + jp] + p[i * N + jm]
                        - 4.0 * p[i * N + j];
                }
            }
        };
        let mut psi = vec![0.0f64; N * N];
        let mut ap = vec![0.0f64; N * N];
        let mut r = rhs.clone(); // r = b − Aψ₀ = b
        let mut d = r.clone();
        let mut rs: f64 = r.iter().map(|x| x * x).sum();
        let bnorm = rs.sqrt().max(1e-300);
        let mut iters = 0;
        for it in 1..=max_iter {
            iters = it;
            lap(&d, &mut ap);
            ap.iter_mut().for_each(|x| *x = -*x); // A = −∇²
            let denom: f64 = d.iter().zip(&ap).map(|(a, b)| a * b).sum();
            if denom.abs() < 1e-300 {
                break;
            }
            let alpha = rs / denom;
            for k in 0..N * N {
                psi[k] += alpha * d[k];
                r[k] -= alpha * ap[k];
            }
            let rs_new: f64 = r.iter().map(|x| x * x).sum();
            if rs_new.sqrt() / bnorm < tol {
                break;
            }
            let beta = rs_new / rs;
            for k in 0..N * N {
                d[k] = r[k] + beta * d[k];
            }
            rs = rs_new;
        }

        // u = ∂ψ/∂y, v = −∂ψ/∂x (central, periodic) — exactly div-free
        let mut u = vec![0.0f32; N * N];
        let mut v = vec![0.0f32; N * N];
        for i in 0..N {
            let (ip, im) = ((i + 1) % N, (i + N - 1) % N);
            for j in 0..N {
                let (jp, jm) = ((j + 1) % N, (j + N - 1) % N);
                u[i * N + j] = (0.5 * (psi[ip * N + j] - psi[im * N + j])) as f32;
                v[i * N + j] = (-0.5 * (psi[i * N + jp] - psi[i * N + jm])) as f32;
            }
        }

        // verification: max |∂ₓu + ∂ᵧv| in f64
        let mut div_max = 0.0f64;
        let mut energy = 0.0f64;
        for i in 0..N {
            let (ip, im) = ((i + 1) % N, (i + N - 1) % N);
            for j in 0..N {
                let (jp, jm) = ((j + 1) % N, (j + N - 1) % N);
                let div = 0.5 * (u[i * N + jp] as f64 - u[i * N + jm] as f64)
                    + 0.5 * (v[ip * N + j] as f64 - v[im * N + j] as f64);
                div_max = div_max.max(div.abs());
                energy += 0.5 * ((u[i * N + j] as f64).powi(2) + (v[i * N + j] as f64).powi(2));
            }
        }
        self.velocity = Some((u, v));
        self.div_max = div_max;
        self.energy = energy / (N * N) as f64;
        self.cg_iters = iters;
    }

    /// The `[2,N,N]` f32 payload for the engine (`predict_ic`).
    pub fn ic_payload(&self) -> Option<Vec<f32>> {
        let (u, v) = self.velocity.as_ref()?;
        let mut out = Vec::with_capacity(2 * N * N);
        out.extend_from_slice(u);
        out.extend_from_slice(v);
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_is_divergence_free_and_faithful() {
        let mut f = PaintField::default();
        f.preset_vortex_pair();
        assert!(f.mean_enstrophy() > 0.0);
        f.project(1e-10, 4000);
        let (u, v) = f.velocity.as_ref().expect("velocity after project");
        assert_eq!(u.len(), N * N);
        // the AC: divergence at machine precision (f32 fields, f64 check)
        assert!(f.div_max < 1e-6, "div too high: {}", f.div_max);
        assert!(f.energy > 0.0);
        // the dipole's velocity between the two cores is the strongest jet:
        // speed at the centre must dwarf the domain-median speed
        let c = N / 2;
        let speed_c = (u[c * N + c].powi(2) + v[c * N + c].powi(2)).sqrt();
        let mut speeds: Vec<f32> = (0..N * N)
            .map(|k| (u[k].powi(2) + v[k].powi(2)).sqrt())
            .collect();
        speeds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(
            speed_c > 5.0 * speeds[N * N / 2],
            "dipole jet missing: centre {speed_c} vs median {}",
            speeds[N * N / 2]
        );
    }

    #[test]
    fn mirror_symmetry_counter_rotates() {
        let mut f = PaintField::default();
        let sym = Symmetry {
            mirror_h: true,
            mirror_v: false,
            radial: 1,
        };
        f.stamp(32.0, 64.0, 4.0, 1.5, 1.0, sym);
        let c = (N as f32 - 1.0) * 0.5;
        let mirrored_x = (2.0 * c - 32.0).round() as usize;
        let w_orig = f.omega[64 * N + 32];
        let w_mirr = f.omega[64 * N + mirrored_x];
        assert!(w_orig > 0.0);
        assert!(
            (w_orig + w_mirr).abs() < 1e-4,
            "mirror must counter-rotate: {w_orig} vs {w_mirr}"
        );
    }

    #[test]
    fn radial_symmetry_stamps_fold_copies() {
        let mut f = PaintField::default();
        let sym = Symmetry {
            mirror_h: false,
            mirror_v: false,
            radial: 4,
        };
        f.stamp(96.0, 64.0, 3.0, 1.0, 1.0, sym); // right of centre → 4-fold ring
        let c = 63.5f32;
        let d = 96.0 - c;
        // the 90°-rotated copy lands above/below the centre with the SAME sign
        let (rx, ry) = (c.round() as usize, (c + d).round() as usize);
        assert!(f.omega[ry * N + rx] > 0.05, "rotated copy missing");
        // presets/stamps stay in-domain
        assert!(f.omega.iter().all(|w| w.is_finite()));
    }

    #[test]
    fn presets_are_near_zero_mean() {
        for (name, apply) in [
            (
                "pair",
                PaintField::preset_vortex_pair as fn(&mut PaintField),
            ),
            ("shear", PaintField::preset_shear_layer),
            ("karman", PaintField::preset_karman_street),
        ] {
            let mut f = PaintField::default();
            apply(&mut f);
            let mean = f.omega.iter().map(|&w| w as f64).sum::<f64>() / (N * N) as f64;
            let rms = f.mean_enstrophy().sqrt();
            assert!(rms > 0.0, "{name}: empty preset");
            assert!(mean.abs() < 0.25 * rms, "{name}: mean {mean} vs rms {rms}");
        }
    }
}
