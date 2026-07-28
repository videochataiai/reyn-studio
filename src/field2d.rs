//! N3 — the 2D pressure-recovery view. Colormaps a 2D field (velocity magnitude,
//! vorticity, or recovered pressure) from the engine's `[3,N,N]` (u,v,p) planes
//! into an egui image. Self-consistency and solver-reference comparison values
//! come straight from `engine::Field2D`; this module just turns fields into pixels.
use crate::engine::Field2D;
use egui::{Color32, ColorImage};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

/// Interactive-view colormap preference (Settings › Appearance). Deterministic
/// evidence exports do NOT read this — they call the `*_in(FieldColormap::Ember, …)`
/// variants directly so archived pixels never depend on a UI preference.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldColormap {
    /// The calibrated instrument vocabulary: ember heat + blue/gold diverging.
    #[default]
    Ember,
    /// Perceptually uniform viridis; signed fields use Moreland cool–warm.
    Viridis,
    /// Perceptually uniform magma; signed fields use Moreland cool–warm.
    Magma,
}

impl FieldColormap {
    pub const ALL: [Self; 3] = [Self::Ember, Self::Viridis, Self::Magma];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ember => "Ember (instrument default)",
            Self::Viridis => "Viridis (perceptually uniform)",
            Self::Magma => "Magma (perceptually uniform)",
        }
    }

    fn id(self) -> u8 {
        match self {
            Self::Ember => 0,
            Self::Viridis => 1,
            Self::Magma => 2,
        }
    }

    fn from_id(id: u8) -> Self {
        match id {
            1 => Self::Viridis,
            2 => Self::Magma,
            _ => Self::Ember,
        }
    }
}

/// Colormap used by interactive views. An atomic keeps the many render call
/// sites free of settings plumbing; it is written once per settings save.
static VIEW_COLORMAP: AtomicU8 = AtomicU8::new(0);

pub fn set_view_colormap(map: FieldColormap) {
    VIEW_COLORMAP.store(map.id(), Ordering::Relaxed);
}

pub fn view_colormap() -> FieldColormap {
    FieldColormap::from_id(VIEW_COLORMAP.load(Ordering::Relaxed))
}

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
            FieldVar::Pressure => "Recovered pressure",
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
                    w[i * n + j] =
                        0.5 * ((at(1, ip, j) - at(1, im, j)) - (at(0, i, jp) - at(0, i, jm)));
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

/// Sample a piecewise-linear LUT of evenly spaced RGB stops at `t` in [0,1].
fn sample_stops(stops: &[[f32; 3]], t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let index = (t.floor() as usize).min(stops.len() - 2);
    lerp(stops[index], stops[index + 1], t - index as f32)
}

/// Viridis (matplotlib), 7-stop approximation.
const VIRIDIS: [[f32; 3]; 7] = [
    [68.0, 1.0, 84.0],
    [70.0, 50.0, 127.0],
    [54.0, 92.0, 141.0],
    [39.0, 127.0, 142.0],
    [31.0, 161.0, 135.0],
    [74.0, 194.0, 109.0],
    [253.0, 231.0, 37.0],
];

/// Magma (matplotlib), 7-stop approximation.
const MAGMA: [[f32; 3]; 7] = [
    [0.0, 0.0, 4.0],
    [40.0, 11.0, 84.0],
    [101.0, 21.0, 110.0],
    [159.0, 42.0, 99.0],
    [212.0, 72.0, 66.0],
    [245.0, 125.0, 21.0],
    [252.0, 253.0, 191.0],
];

/// Blue (−) → dark → ember/gold (+), for signed vorticity & pressure —
/// evaluated in an explicit colormap so evidence exports can pin Ember.
pub fn diverging_in(map: FieldColormap, t: f32) -> Color32 {
    match map {
        FieldColormap::Ember => {
            const DARK: [f32; 3] = [16.0, 12.0, 10.0];
            const BLUE: [f32; 3] = [70.0, 140.0, 210.0];
            const GOLD: [f32; 3] = [240.0, 180.0, 70.0];
            if t < 0.0 {
                lerp(DARK, BLUE, (-t).clamp(0.0, 1.0))
            } else {
                lerp(DARK, GOLD, t.clamp(0.0, 1.0))
            }
        }
        // Moreland cool–warm diverging pairs both uniform sequential maps.
        FieldColormap::Viridis | FieldColormap::Magma => {
            const COOL: [f32; 3] = [59.0, 76.0, 192.0];
            const NEUTRAL: [f32; 3] = [221.0, 221.0, 221.0];
            const WARM: [f32; 3] = [180.0, 4.0, 38.0];
            if t < 0.0 {
                lerp(NEUTRAL, COOL, (-t).clamp(0.0, 1.0))
            } else {
                lerp(NEUTRAL, WARM, t.clamp(0.0, 1.0))
            }
        }
    }
}

/// Sequential ramp for magnitudes (0..1) in an explicit colormap.
pub fn sequential_in(map: FieldColormap, t: f32) -> Color32 {
    match map {
        FieldColormap::Ember => {
            // Black-body ember heat ramp.
            let r = (255.0 * (t * 1.7).clamp(0.0, 1.0)) as u8;
            let g = (255.0 * ((t - 0.22) * 1.6).clamp(0.0, 1.0)) as u8;
            let b = (255.0 * ((t - 0.62) * 2.2).clamp(0.0, 1.0)) as u8;
            Color32::from_rgb(r, g, b)
        }
        FieldColormap::Viridis => sample_stops(&VIRIDIS, t),
        FieldColormap::Magma => sample_stops(&MAGMA, t),
    }
}

/// Signed colormap in the active interactive-view preference.
fn diverging(t: f32) -> Color32 {
    diverging_in(view_colormap(), t)
}

/// Magnitude colormap in the active interactive-view preference.
fn sequential(t: f32) -> Color32 {
    sequential_in(view_colormap(), t)
}

/// Colormap sample for legends and markers: `t` in [0,1] (sequential) or
/// [-1,1] (signed/diverging).
pub fn colormap_color(t: f32, signed: bool) -> Color32 {
    if signed {
        diverging(t.clamp(-1.0, 1.0))
    } else {
        sequential(t.clamp(0.0, 1.0))
    }
}

/// Colormap an already-derived nonnegative scalar map with an explicit scale.
/// Benchmark model/reference speed maps share a scale; their error map uses its own.
pub fn magnitude_image(values: &[f32], n: usize, scale: f32) -> ColorImage {
    let pixels = values
        .iter()
        .map(|&value| sequential((value / scale.max(1e-12)).clamp(0.0, 1.0)))
        .collect();
    ColorImage {
        size: [n, n],
        pixels,
        source_size: egui::Vec2::new(n as f32, n as f32),
    }
}

/// Normalization scale of a source for the chosen variable (abs-max for signed
/// fields, max for magnitudes) plus whether the variable is signed. Comparison
/// model/reference panels must share max(scale) so their colors are comparable.
pub fn scalar_scale(src: &[f32], n: usize, var: FieldVar) -> (f32, bool) {
    let (vals, signed) = scalar(src, n, var);
    let scale = vals.iter().fold(
        1e-6f32,
        |m, &x| if signed { m.max(x.abs()) } else { m.max(x) },
    );
    (scale, signed)
}

/// Colormap one `[3,N,N]` source with an explicit scale (use `scalar_scale`,
/// shared across panels being compared).
pub fn image_scaled(f: &Field2D, src: &[f32], var: FieldVar, scale: f32) -> ColorImage {
    let n = f.n;
    let (vals, signed) = scalar(src, n, var);
    let pixels: Vec<Color32> = vals
        .iter()
        .map(|&val| {
            if signed {
                diverging((val / scale).clamp(-1.0, 1.0))
            } else {
                sequential((val / scale).clamp(0.0, 1.0))
            }
        })
        .collect();
    ColorImage {
        size: [n, n],
        pixels,
        source_size: egui::Vec2::new(n as f32, n as f32),
    }
}

// -- field insights (auto-surfaced critical points) ---------------------------
// The pattern every serious post-processor converges on (Ansys CFD-Post min/max
// flags, COMSOL annotations): the tool FINDS the engineering-critical points and
// pins them, instead of making the engineer hunt through a contour plot.

#[derive(Clone, Copy, PartialEq)]
pub enum InsightKind {
    PeakPressure, // max recovered p/ρ — stagnation/load point
    SuctionPeak,  // min recovered p/ρ — the low-pressure core
    MaxVorticity, // max |ω| — strongest shear/rotation
    MaxSpeed,     // max |v|
    MaxError,     // max |model − solver reference| of the displayed variable
}

impl InsightKind {
    pub fn glyph(self) -> &'static str {
        match self {
            InsightKind::PeakPressure => "P▲",
            InsightKind::SuctionPeak => "P▼",
            InsightKind::MaxVorticity => "ω",
            InsightKind::MaxSpeed => "v",
            InsightKind::MaxError => "ε",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            InsightKind::PeakPressure => "Recovered-pressure maximum",
            InsightKind::SuctionPeak => "Recovered-pressure minimum",
            InsightKind::MaxVorticity => "Max vorticity",
            InsightKind::MaxSpeed => "Max speed",
            InsightKind::MaxError => "Max error",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Insight {
    pub kind: InsightKind,
    pub i: usize, // row (y)
    pub j: usize, // col (x)
    pub value: f32,
}

fn argext(vals: &[f32], n: usize, max: bool) -> (usize, usize, f32) {
    let (mut bi, mut bv) = (0usize, vals[0]);
    for (idx, &v) in vals.iter().enumerate() {
        if (max && v > bv) || (!max && v < bv) {
            bi = idx;
            bv = v;
        }
    }
    (bi / n, bi % n, bv)
}

/// First maximum absolute value, matching `argext(abs(values), max=true)`
/// without allocating the temporary absolute-value map.
fn argmax_abs(vals: &[f32], n: usize) -> (usize, usize, f32) {
    let (mut best_index, mut best_abs) = (0usize, vals[0].abs());
    for (index, value) in vals.iter().enumerate() {
        let absolute = value.abs();
        if absolute > best_abs {
            best_index = index;
            best_abs = absolute;
        }
    }
    (best_index / n, best_index % n, vals[best_index])
}

/// First maximum absolute pointwise difference, matching materialization of an
/// error map followed by `argext`, but retaining only the winning scalar.
fn argmax_abs_diff(left: &[f32], right: &[f32], n: usize) -> (usize, usize, f32) {
    let (mut best_index, mut best_error) = (0usize, (left[0] - right[0]).abs());
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        let error = (left - right).abs();
        if error > best_error {
            best_index = index;
            best_error = error;
        }
    }
    (best_index / n, best_index % n, best_error)
}

/// Auto-detected critical points of a `[3,N,N]` source (+ the error hotspot vs
/// `truth` when present, measured on the displayed variable).
pub fn insights(f: &Field2D, src: &[f32], truth: Option<&[f32]>, var: FieldVar) -> Vec<Insight> {
    let n = f.n;
    let mut out = Vec::with_capacity(5);
    let p = &src[2 * n * n..3 * n * n];
    let (i, j, v) = argext(p, n, true);
    out.push(Insight {
        kind: InsightKind::PeakPressure,
        i,
        j,
        value: v,
    });
    let (i, j, v) = argext(p, n, false);
    out.push(Insight {
        kind: InsightKind::SuctionPeak,
        i,
        j,
        value: v,
    });
    let (w, _) = scalar(src, n, FieldVar::Vorticity);
    let (i, j, value) = argmax_abs(&w, n);
    out.push(Insight {
        kind: InsightKind::MaxVorticity,
        i,
        j,
        value,
    });
    let (s, _) = scalar(src, n, FieldVar::Velocity);
    let (i, j, v) = argext(&s, n, true);
    out.push(Insight {
        kind: InsightKind::MaxSpeed,
        i,
        j,
        value: v,
    });
    if let Some(t) = truth {
        let (a, _) = scalar(src, n, var);
        let (b, _) = scalar(t, n, var);
        let (i, j, v) = argmax_abs_diff(&a, &b, n);
        out.push(Insight {
            kind: InsightKind::MaxError,
            i,
            j,
            value: v,
        });
    }
    out
}

/// Everything at one cell — the live hover probe readout.
pub struct ProbeSample {
    pub u: f32,
    pub v: f32,
    pub speed: f32,
    pub omega: f32,
    pub p: f32,
}

pub fn probe(src: &[f32], n: usize, i: usize, j: usize) -> ProbeSample {
    let plane = n * n;
    let at = |c: usize, i: usize, j: usize| src[c * plane + i * n + j];
    let (ip, im) = ((i + 1) % n, (i + n - 1) % n);
    let (jp, jm) = ((j + 1) % n, (j + n - 1) % n);
    let (u, v) = (at(0, i, j), at(1, i, j));
    ProbeSample {
        u,
        v,
        speed: (u * u + v * v).sqrt(),
        omega: 0.5 * ((at(1, ip, j) - at(1, im, j)) - (at(0, i, jp) - at(0, i, jm))),
        p: at(2, i, j),
    }
}

/// Per-pixel absolute error between model prediction and solver reference for
/// the chosen variable, rendered on a hot map.
pub fn error_image(f: &Field2D, ai: &[f32], truth: &[f32], var: FieldVar) -> ColorImage {
    let n = f.n;
    let (a, _) = scalar(ai, n, var);
    let (t, _) = scalar(truth, n, var);
    let err: Vec<f32> = a.iter().zip(&t).map(|(x, y)| (x - y).abs()).collect();
    let scale = err.iter().cloned().fold(1e-6f32, f32::max);
    let pixels: Vec<Color32> = err
        .iter()
        .map(|&e| sequential((e / scale).clamp(0.0, 1.0)))
        .collect();
    ColorImage {
        size: [n, n],
        pixels,
        source_size: egui::Vec2::new(n as f32, n as f32),
    }
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
        Field2D {
            n,
            ai,
            truth: None,
            horizon: 8,
            dt_frame: 0.04,
            peak_p: 1.0,
            low_p: -1.0,
            semigroup: Some(0.01),
            rel_l2: None,
            persist: None,
            p_residual: 3e-5,
            p_iters: 0,
            p_method: "spectral".into(),
            scenario: "test".into(),
        }
    }

    #[test]
    fn colormap_produces_varied_image() {
        let n = 16;
        let f = synthetic(n);
        assert_eq!(FieldVar::Pressure.label(), "Recovered pressure");
        for var in [FieldVar::Velocity, FieldVar::Vorticity, FieldVar::Pressure] {
            let (scale, _) = scalar_scale(&f.ai, n, var);
            let img = image_scaled(&f, &f.ai, var, scale);
            assert_eq!(img.size, [n, n]);
            assert_eq!(img.pixels.len(), n * n);
            let distinct = img.pixels.iter().collect::<HashSet<_>>().len();
            assert!(
                distinct > 4,
                "{} produced a near-flat image ({distinct} colors)",
                var.label()
            );
        }
    }

    #[test]
    fn insights_find_known_extrema() {
        let n = 16;
        let mut f = synthetic(n);
        // plant unambiguous extrema at known cells
        let plane = n * n;
        f.ai[2 * plane + 3 * n + 4] = 9.0; // pressure max at (3,4)
        f.ai[2 * plane + 12 * n + 7] = -9.0; // pressure min at (12,7)
        f.ai[5 * n + 9] = 30.0; // huge u at (5,9) → max speed there
        let found = insights(&f, &f.ai, None, FieldVar::Pressure);
        assert_eq!(found.len(), 4, "no error insight without truth");
        let get = |k: InsightKind| found.iter().find(|x| x.kind == k).unwrap();
        let pk = get(InsightKind::PeakPressure);
        assert_eq!((pk.i, pk.j, pk.value), (3, 4, 9.0));
        let sc = get(InsightKind::SuctionPeak);
        assert_eq!((sc.i, sc.j, sc.value), (12, 7, -9.0));
        let sp = get(InsightKind::MaxSpeed);
        assert_eq!((sp.i, sp.j), (5, 9));

        // with truth: the error hotspot lands on the planted discrepancy
        let mut truth = f.ai.clone();
        truth[2 * plane + 3 * n + 4] = 0.0; // biggest |Δp| at (3,4)
        let found = insights(&f, &f.ai, Some(&truth), FieldVar::Pressure);
        let err = found
            .iter()
            .find(|x| x.kind == InsightKind::MaxError)
            .unwrap();
        assert_eq!((err.i, err.j), (3, 4));
    }

    #[test]
    fn allocation_free_extrema_keep_first_tie_semantics() {
        let values = [1.0, -4.0, 4.0, 2.0];
        assert_eq!(argmax_abs(&values, 2), (0, 1, -4.0));
        let reference = [1.0, 0.0, 8.0, 2.0];
        assert_eq!(argmax_abs_diff(&values, &reference, 2), (0, 1, 4.0));
    }

    #[test]
    fn probe_reads_the_cell() {
        let n = 16;
        let f = synthetic(n);
        let s = probe(&f.ai, n, 2, 5);
        let plane = n * n;
        assert_eq!(s.u, f.ai[2 * n + 5]);
        assert_eq!(s.v, f.ai[plane + 2 * n + 5]);
        assert_eq!(s.p, f.ai[2 * plane + 2 * n + 5]);
        assert!((s.speed - (s.u * s.u + s.v * s.v).sqrt()).abs() < 1e-6);
    }

    #[test]
    fn error_of_identical_fields_is_uniform() {
        let f = synthetic(12);
        let img = error_image(&f, &f.ai, &f.ai, FieldVar::Vorticity);
        let c0 = img.pixels[0];
        assert!(
            img.pixels.iter().all(|&c| c == c0),
            "error of a field with itself should be flat"
        );
    }

    #[test]
    fn magnitude_image_uses_calibrated_scale() {
        let img = magnitude_image(&[0.0, 0.5, 1.0, 2.0], 2, 2.0);
        assert_eq!(img.size, [2, 2]);
        assert_ne!(img.pixels[0], img.pixels[1]);
        assert_ne!(img.pixels[2], img.pixels[3]);
        assert_eq!(img.pixels[3], colormap_color(1.0, false));
    }

    /// The pure per-colormap samplers hit their published endpoints — tested
    /// directly so the global view preference is never mutated inside tests.
    #[test]
    fn colormap_variants_hit_their_endpoints() {
        assert_eq!(
            sequential_in(FieldColormap::Viridis, 0.0),
            Color32::from_rgb(68, 1, 84)
        );
        assert_eq!(
            sequential_in(FieldColormap::Viridis, 1.0),
            Color32::from_rgb(253, 231, 37)
        );
        assert_eq!(
            sequential_in(FieldColormap::Magma, 1.0),
            Color32::from_rgb(252, 253, 191)
        );
        // Ember stays the calibrated ramp: full heat saturates to near-white.
        assert_eq!(
            sequential_in(FieldColormap::Ember, 1.0),
            Color32::from_rgb(255, 255, 213)
        );
        // Diverging endpoints: ember keeps blue/gold; uniform maps use cool–warm.
        assert_eq!(
            diverging_in(FieldColormap::Ember, 1.0),
            Color32::from_rgb(240, 180, 70)
        );
        assert_eq!(
            diverging_in(FieldColormap::Viridis, -1.0),
            Color32::from_rgb(59, 76, 192)
        );
        assert_eq!(
            diverging_in(FieldColormap::Magma, 1.0),
            Color32::from_rgb(180, 4, 38)
        );
    }
}
