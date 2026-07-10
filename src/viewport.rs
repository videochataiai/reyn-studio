//! Interactive 3D flow viewport (egui painter projection). Mouse-drag orbits,
//! scroll zooms, and every 3D control (opacity, density, slice, streamlines)
//! drives the render. GPU-bloom upgrade (Bevy/wgpu) is a later swap; this works now.
use crate::flow::Particle;
use crate::theme::*;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

pub struct Camera { pub yaw: f32, pub pitch: f32, pub dist: f32 }
impl Default for Camera {
    fn default() -> Self { Self { yaw: 0.8, pitch: 0.32, dist: 4.4 } }
}

pub struct ViewOpts {
    pub opacity: f32,
    pub density_lo: f32,
    pub slice_x: Option<f32>,
    pub streamlines: bool,
}

fn colormap(vort: f32, alpha: f32) -> Color32 {
    // blue (neg) -> dark -> ember -> gold (pos)
    let t = vort.clamp(-1.0, 1.0);
    let (r, g, b) = if t < 0.0 {
        let k = -t; // 0..1
        (0.10 + 0.44 * (1.0 - k), 0.40 + 0.4 * (1.0 - k), 0.55 + 0.45 * k)
    } else {
        // dark ember -> ember -> gold
        (0.55 + 0.42 * t, 0.28 + 0.18 * t, 0.05 + 0.02 * t)
    };
    Color32::from_rgba_unmultiplied((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8,
        (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

pub fn show(ui: &mut egui::Ui, rect: Rect, cam: &mut Camera, opts: &ViewOpts, particles: &[Particle]) {
    let resp = ui.interact(rect, ui.id().with("viewport3d"), Sense::drag());
    if resp.dragged() {
        let d = resp.drag_delta();
        cam.yaw += d.x * 0.008;
        cam.pitch = (cam.pitch + d.y * 0.008).clamp(-1.45, 1.45);
    }
    if resp.hovered() {
        let sc = ui.input(|i| i.smooth_scroll_delta.y);
        if sc != 0.0 { cam.dist = (cam.dist * (1.0 - sc * 0.0018)).clamp(2.4, 9.5); }
    }

    let p = ui.painter_at(rect);
    let center = rect.center();
    let scale = rect.height().min(rect.width()) * 0.44;
    let focal = 2.7_f32;
    let (cy, sy) = (cam.yaw.cos(), cam.yaw.sin());
    let (cp, sp) = (cam.pitch.cos(), cam.pitch.sin());
    let project = |v: [f32; 3]| -> (Pos2, f32) {
        let x = v[0] * cy - v[2] * sy;
        let z = v[0] * sy + v[2] * cy;
        let y = v[1] * cp - z * sp;
        let zc = v[1] * sp + z * cp + cam.dist;
        let f = focal / zc.max(0.1);
        (Pos2::new(center.x + x * f * scale, center.y - y * f * scale), zc)
    };

    // domain bounding box
    let cs = [
        [-1., -1., -1.], [1., -1., -1.], [1., 1., -1.], [-1., 1., -1.],
        [-1., -1., 1.], [1., -1., 1.], [1., 1., 1.], [-1., 1., 1.],
    ];
    let edges = [(0,1),(1,2),(2,3),(3,0),(4,5),(5,6),(6,7),(7,4),(0,4),(1,5),(2,6),(3,7)];
    let box_stroke = Stroke::new(1.0, OUTLINE_VARIANT.gamma_multiply(0.9));
    for (a, b) in edges {
        p.line_segment([project(cs[a]).0, project(cs[b]).0], box_stroke);
    }

    let thr = (opts.density_lo - 0.5).max(0.0); // density -> |vort| threshold
    let mut drawn: Vec<(f32, Pos2, Color32, f32)> = Vec::with_capacity(particles.len());
    for pt in particles {
        if pt.vort.abs() < thr { continue; }
        if let Some(sx) = opts.slice_x {
            if pt.pos[0] < sx * 2.0 - 1.0 { continue; }
        }
        let (s, depth) = project(pt.pos);
        if !rect.expand(40.0).contains(s) { continue; }
        let fade = (2.2 / depth).clamp(0.15, 1.0);
        let a = opts.opacity * fade * (0.35 + 0.65 * pt.speed);
        let col = colormap(pt.vort, a);
        let r = (fade * 3.0).clamp(1.2, 3.4);
        drawn.push((depth, s, col, r));
    }
    drawn.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, pos, col, r) in &drawn {
        // faint halo + bright core = soft glow without GPU blending
        let halo = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), col.a() / 3);
        p.circle_filled(*pos, r * 2.1, halo);
        p.circle_filled(*pos, *r, *col);
    }

    if opts.streamlines {
        draw_streamlines(&p, &project, particles, cam.dist);
    }
}

fn draw_streamlines(
    p: &egui::Painter,
    project: &impl Fn([f32; 3]) -> (Pos2, f32),
    particles: &[Particle],
    _dist: f32,
) {
    // a few streamlines seeded from strong-vorticity particles
    let seeds: Vec<[f32; 3]> = particles.iter()
        .filter(|q| q.vort.abs() > 0.6)
        .take(40)
        .map(|q| q.pos)
        .collect();
    for s in seeds {
        let mut pos = s;
        let mut poly = Vec::with_capacity(24);
        for _ in 0..24 {
            poly.push(project(pos).0);
            // ABC advection in normalized space
            let (x, y, z) = (pos[0] * std::f32::consts::PI, pos[1] * std::f32::consts::PI, pos[2] * std::f32::consts::PI);
            let v = [(z.sin() + y.cos()), (x.sin() + z.cos()), (y.sin() + x.cos())];
            for k in 0..3 { pos[k] = (pos[k] + v[k] * 0.02).clamp(-1.0, 1.0); }
        }
        p.line(poly, Stroke::new(1.0, GOLD.gamma_multiply(0.5)));
    }
    let _ = Vec2::ZERO;
}
