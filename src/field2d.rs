//! N3 — the 2D pressure-recovery view. Colormaps a 2D field (velocity magnitude,
//! vorticity, or recovered pressure) from the engine's `[3,N,N]` (u,v,p) planes
//! into an egui image. The verification numbers (Trust Meter, Truth Overlay) come
//! straight off `engine::Field2D`; this module just turns fields into pixels.
use crate::engine::Field2D;
use egui::{Color32, ColorImage};

#[derive(PartialEq, Clone, Copy)]
pub enum FieldVar {
    Velocity,
    Vorticity,
    Pressure,
}

impl FieldVar {
    pub fn label(self) -> &'static str {
        match self {
            FieldVar::Velocity => "Velocity",
            FieldVar::Vorticity => "Vorticity",
            FieldVar::Pressure => "Pressure",
        }
    }
}

/// Derive the chosen scalar from a `[3,N,N]` (u,v,p) plane set. Returns
/// `(values, signed)`; signed fields use a diverging map, magnitudes a heat map.
fn scalar(src: &[f32], n: usize, var: FieldVar) -> (Vec<f32>, bool) {
    let plane = n * n;
    let at = |c: usize, i: usize, j: usize| src[c * plane + i * n + j];
    match var {
        FieldVar::Velocity => {
            let mut v = vec![0f32; plane];
            for idx in 0..plane {
                let (u, w) = (src[idx], src[plane + idx]);
                v[idx] = (u * u + w * w).sqrt();
            }
            (v, false)
        }
        FieldVar::Pressure => (src[2 * plane..3 * plane].to_vec(), true),
        FieldVar::Vorticity => {
            // ω = ∂v/∂x − ∂u/∂y (central difference, periodic)
            let mut w = vec![0f32; plane];
            for i in 0..n {
                let (ip, im) = ((i + 1) % n, (i + n - 1) % n);
                for j in 0..n {
                    let (jp, jm) = ((j + 1) % n, (j + n - 1) % n);
                    w[i * n + j] = 0.5 * ((at(1, ip, j) - at(1, im, j)) - (at(0, i, jp) - at(0, i, jm)));
                }
            }
            (w, true)
        }
    }
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> Color32 {
    let c = |x: f32, y: f32| (x + (y - x) * t) as u8;
    Color32::from_rgb(c(a[0], b[0]), c(a[1], b[1]), c(a[2], b[2]))
}

/// Blue (−) → dark → ember/gold (+), for signed vorticity & pressure.
fn diverging(t: f32) -> Color32 {
    const DARK: [f32; 3] = [16.0, 12.0, 10.0];
    const BLUE: [f32; 3] = [70.0, 140.0, 210.0];
    const GOLD: [f32; 3] = [240.0, 180.0, 70.0];
    if t < 0.0 { lerp(DARK, BLUE, (-t).clamp(0.0, 1.0)) } else { lerp(DARK, GOLD, t.clamp(0.0, 1.0)) }
}

/// Black-body ember heat ramp for magnitudes (0..1).
fn sequential(t: f32) -> Color32 {
    let r = (255.0 * (t * 1.7).clamp(0.0, 1.0)) as u8;
    let g = (255.0 * ((t - 0.22) * 1.6).clamp(0.0, 1.0)) as u8;
    let b = (255.0 * ((t - 0.62) * 2.2).clamp(0.0, 1.0)) as u8;
    Color32::from_rgb(r, g, b)
}

/// Colormap one `[3,N,N]` source (AI or truth) for the chosen variable.
pub fn image(f: &Field2D, src: &[f32], var: FieldVar) -> ColorImage {
    let n = f.n;
    let (vals, signed) = scalar(src, n, var);
    let scale = vals.iter().fold(1e-6f32, |m, &x| if signed { m.max(x.abs()) } else { m.max(x) });
    let pixels: Vec<Color32> = vals.iter().map(|&val| {
        if signed { diverging((val / scale).clamp(-1.0, 1.0)) }
        else { sequential((val / scale).clamp(0.0, 1.0)) }
    }).collect();
    ColorImage { size: [n, n], pixels, source_size: egui::Vec2::new(n as f32, n as f32) }
}

/// Per-pixel absolute error between AI and truth for the chosen variable, on a
/// hot map — the Truth Overlay's error panel.
pub fn error_image(f: &Field2D, ai: &[f32], truth: &[f32], var: FieldVar) -> ColorImage {
    let n = f.n;
    let (a, _) = scalar(ai, n, var);
    let (t, _) = scalar(truth, n, var);
    let err: Vec<f32> = a.iter().zip(&t).map(|(x, y)| (x - y).abs()).collect();
    let scale = err.iter().cloned().fold(1e-6f32, f32::max);
    let pixels: Vec<Color32> = err.iter().map(|&e| sequential((e / scale).clamp(0.0, 1.0))).collect();
    ColorImage { size: [n, n], pixels, source_size: egui::Vec2::new(n as f32, n as f32) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Field2D;
    use std::collections::HashSet;
    use std::f32::consts::PI;

    fn synthetic(n: usize) -> Field2D {
        // u=sin(2πj), v=sin(2πi) → velocity, vorticity all vary; p = radial bump
        let mut ai = vec![0f32; 3 * n * n];
        for i in 0..n {
            for j in 0..n {
                let (fi, fj) = (i as f32 / n as f32, j as f32 / n as f32);
                ai[i * n + j] = (2.0 * PI * fj).sin();
                ai[n * n + i * n + j] = (2.0 * PI * fi).sin();
                ai[2 * n * n + i * n + j] = ((fi - 0.5).powi(2) + (fj - 0.5).powi(2)).sqrt();
            }
        }
        Field2D { n, ai, truth: None, horizon: 8, dt_frame: 0.04, peak_p: 1.0, low_p: -1.0,
                  semigroup: Some(0.01), rel_l2: None, persist: None,
                  p_residual: 3e-5, p_iters: 0, p_method: "spectral".into(), scenario: "test".into() }
    }

    #[test]
    fn colormap_produces_varied_image() {
        let n = 16;
        let f = synthetic(n);
        for var in [FieldVar::Velocity, FieldVar::Vorticity, FieldVar::Pressure] {
            let img = image(&f, &f.ai, var);
            assert_eq!(img.size, [n, n]);
            assert_eq!(img.pixels.len(), n * n);
            let distinct = img.pixels.iter().collect::<HashSet<_>>().len();
            assert!(distinct > 4, "{} produced a near-flat image ({distinct} colors)", var.label());
        }
    }

    #[test]
    fn error_of_identical_fields_is_uniform() {
        let f = synthetic(12);
        let img = error_image(&f, &f.ai, &f.ai, FieldVar::Vorticity);
        let c0 = img.pixels[0];
        assert!(img.pixels.iter().all(|&c| c == c0), "error of a field with itself should be flat");
    }
}
