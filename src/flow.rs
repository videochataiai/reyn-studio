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
