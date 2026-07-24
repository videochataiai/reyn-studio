//! Interactive 3D flow viewport. Mouse-drag orbits, scroll zooms, and every 3D
//! control (opacity, density, slice, streamlines) drives the render. The
//! particles are projected here on the CPU, then handed to the native wgpu
//! bloom renderer (`gpu.rs`, N2) which lights the vortex cores with a real HDR
//! + bloom pass. A CPU halo+core fallback keeps the viewport working if wgpu is
//! ever unavailable.
use crate::flow::Particle;
use crate::gpu::{self, GpuInstance, SegInstance};
use crate::theme::*;
use egui::{Color32, Pos2, Rect, Sense, Stroke};

pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.8,
            pitch: 0.32,
            dist: 4.4,
        }
    }
}

pub struct ViewOpts {
    pub opacity: f32,
    pub density_lo: f32,
    pub density_hi: f32,
    pub slice: [Option<f32>; 3], // clip plane per axis when enabled
    pub streamlines: bool,
    pub shadows: bool,
    pub mode2d: bool,
    pub gpu: bool,         // route particles through the native wgpu bloom pass
    pub volume_mode: bool, // GPU volume raymarch instead of point sprites
    pub volume: Option<gpu::VolumeData>, // the |ω| scalar field for the raymarch
    pub surface: Option<gpu::SurfaceData>, // CAD body + surface-load layer
    pub markers: Vec<Marker3D>, // billboarded critical-point annotations
    /// Settings › Viewport: multiplier on the calibrated orbit drag rate.
    pub orbit_sensitivity: f32,
    /// Settings › Viewport: scroll up moves the camera away instead of closer.
    pub invert_scroll_zoom: bool,
    /// Settings › Viewport: draw the [-1, 1]³ solver-domain wireframe.
    pub show_domain_bounds: bool,
}

/// A billboarded critical-point annotation: fixed screen-size ring + value chip
/// at a 3D domain position (the 3D counterpart of the 2D Field Insights).
#[derive(Clone)]
pub struct Marker3D {
    pub pos: [f32; 3], // [-1,1]³ domain coords
    pub color: Color32,
    pub text: String, // "glyph value", mono
}

/// Must match `fs_volume`'s `tanH` (gpu.rs) so CPU-projected annotations land on
/// the raymarched image.
pub const TAN_HALF_FOV: f32 = 0.55;

/// Project a domain point with the raymarch camera (orbit eye looking at the
/// origin). Returns None behind the near plane.
fn project_volume(cam: &Camera, rect: Rect, p: [f32; 3]) -> Option<Pos2> {
    let eye = [
        cam.dist * cam.pitch.cos() * cam.yaw.sin(),
        cam.dist * cam.pitch.sin(),
        cam.dist * cam.pitch.cos() * cam.yaw.cos(),
    ];
    let norm = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let fwd = norm([-eye[0], -eye[1], -eye[2]]);
    let right = norm(cross(fwd, [0.0, 1.0, 0.0]));
    let up = cross(right, fwd);
    let v = [p[0] - eye[0], p[1] - eye[1], p[2] - eye[2]];
    let z = dot(v, fwd);
    if z < 0.15 {
        return None;
    }
    let aspect = rect.width() / rect.height().max(1.0);
    let ndx = dot(v, right) / (z * TAN_HALF_FOV * aspect);
    let ndy = dot(v, up) / (z * TAN_HALF_FOV);
    Some(Pos2::new(
        rect.min.x + (ndx + 1.0) * 0.5 * rect.width(),
        rect.min.y + (1.0 - ndy) * 0.5 * rect.height(),
    ))
}

/// Screen-space instrument marker: ring + dot at `center`, 1px leader to a
/// collision-avoiding mono value chip clamped inside `bounds`. Shared by the 2D
/// Field Insights and the 3D billboards — one visual vocabulary.
pub fn insight_marker(
    p: &egui::Painter,
    bounds: Rect,
    center: Pos2,
    col: Color32,
    text: &str,
    chips: &mut Vec<Rect>,
) {
    p.circle_stroke(center, 6.0, Stroke::new(1.5, col));
    p.circle_filled(center, 1.6, col);

    let galley = p.layout_no_wrap(text.to_owned(), egui::FontId::monospace(10.5), TEXT);
    let size = galley.size() + egui::Vec2::new(14.0, 9.0);
    let mut anchor = Pos2::new(center.x + 12.0, center.y - 22.0);
    if anchor.x + size.x > bounds.max.x - 2.0 {
        anchor.x = center.x - 12.0 - size.x;
    }
    if anchor.y < bounds.min.y + 2.0 {
        anchor.y = center.y + 12.0;
    }
    let mut chip = Rect::from_min_size(anchor, size);
    for _ in 0..8 {
        if !chips.iter().any(|c| c.intersects(chip.expand(2.0))) {
            break;
        }
        chip = chip.translate(egui::Vec2::new(0.0, size.y + 3.0));
    }
    chip = Rect::from_min_size(
        Pos2::new(
            chip.min.x.clamp(bounds.min.x, bounds.max.x - size.x),
            chip.min.y.clamp(bounds.min.y, bounds.max.y - size.y),
        ),
        size,
    );

    let to = if chip.center().x >= center.x {
        Pos2::new(chip.min.x, chip.center().y)
    } else {
        Pos2::new(chip.max.x, chip.center().y)
    };
    let dir = (to - center).normalized();
    p.line_segment(
        [center + dir * 6.0, to],
        Stroke::new(1.0, col.gamma_multiply(0.7)),
    );

    p.rect_filled(chip, egui::CornerRadius::same(2), SURFACE);
    p.rect_stroke(
        chip,
        egui::CornerRadius::same(2),
        Stroke::new(1.0, col),
        egui::StrokeKind::Inside,
    );
    p.galley(chip.min + egui::Vec2::new(7.0, 4.5), galley, TEXT);
    chips.push(chip);
}

/// Draw the billboarded critical-point markers with a mode-appropriate
/// projection; respects the same slice-plane clipping as the renderers.
fn draw_markers(
    ui: &egui::Ui,
    rect: Rect,
    cam: &Camera,
    opts: &ViewOpts,
    project_pt: Option<&dyn Fn([f32; 3]) -> (Pos2, f32)>,
) {
    if opts.markers.is_empty() {
        return;
    }
    let p = ui.painter_at(rect);
    let mut chips: Vec<Rect> = Vec::new();
    'markers: for m in &opts.markers {
        for a in 0..3 {
            if let Some(pos) = opts.slice[a] {
                if m.pos[a] < pos * 2.0 - 1.0 {
                    continue 'markers;
                }
            }
        }
        let screen = match project_pt {
            Some(f) => {
                let (s, depth) = f(m.pos);
                if !opts.mode2d && depth < 0.2 {
                    continue;
                }
                s
            }
            None => match project_volume(cam, rect, m.pos) {
                Some(s) => s,
                None => continue,
            },
        };
        if !rect.contains(screen) {
            continue;
        }
        insight_marker(&p, rect, screen, m.color, &m.text, &mut chips);
    }
}

/// blue (neg) -> dark -> ember -> gold (pos), returned as linear-ish rgb 0..1.
fn colormap_rgb(vort: f32) -> [f32; 3] {
    let t = vort.clamp(-1.0, 1.0);
    if t < 0.0 {
        let k = -t;
        [
            0.10 + 0.44 * (1.0 - k),
            0.40 + 0.4 * (1.0 - k),
            0.55 + 0.45 * k,
        ]
    } else {
        [0.55 + 0.42 * t, 0.28 + 0.18 * t, 0.05 + 0.02 * t]
    }
}

/// A projected particle carrying everything both the GPU and CPU paths need.
struct Proj {
    screen: Pos2,
    ndc: [f32; 2],
    depth: f32,
    r_pts: f32,
    base: [f32; 3],
    weight: f32,
    gain: f32,
}

pub fn show(
    ui: &mut egui::Ui,
    rect: Rect,
    cam: &mut Camera,
    opts: &ViewOpts,
    particles: &[Particle],
) {
    let resp = ui.interact(rect, ui.id().with("viewport3d"), Sense::drag());
    let orbit_rate = 0.008 * opts.orbit_sensitivity.clamp(0.4, 2.0);
    if resp.dragged() && !opts.mode2d {
        let d = resp.drag_delta();
        cam.yaw += d.x * orbit_rate;
        cam.pitch = (cam.pitch + d.y * orbit_rate).clamp(-1.45, 1.45);
    }
    if resp.hovered() {
        let mut sc = ui.input(|i| i.smooth_scroll_delta.y);
        if opts.invert_scroll_zoom {
            sc = -sc;
        }
        if sc != 0.0 {
            cam.dist = (cam.dist * (1.0 - sc * 0.0018)).clamp(2.4, 9.5);
        }
    }

    // Volume raymarch mode: hand the whole 3D field to the GPU and return. The
    // orbit camera becomes a ray origin looking at the domain origin.
    if opts.gpu && opts.volume_mode && !opts.mode2d {
        let eye = [
            cam.dist * cam.pitch.cos() * cam.yaw.sin(),
            cam.dist * cam.pitch.sin(),
            cam.dist * cam.pitch.cos() * cam.yaw.cos(),
        ];
        let slice_c = [
            opts.slice[0].map(|p| p * 2.0 - 1.0).unwrap_or(-2.0),
            opts.slice[1].map(|p| p * 2.0 - 1.0).unwrap_or(-2.0),
            opts.slice[2].map(|p| p * 2.0 - 1.0).unwrap_or(-2.0),
        ];
        gpu::add_volume(
            ui,
            rect,
            eye,
            TAN_HALF_FOV,
            opts.density_lo,
            opts.density_hi,
            slice_c,
            opts.shadows,
            opts.volume.clone(),
            opts.surface.clone(),
        );
        draw_markers(ui, rect, cam, opts, None); // billboards over the raymarch
        return;
    }

    let p = ui.painter_at(rect);
    let center = rect.center();
    let scale = rect.height().min(rect.width()) * 0.44;
    let focal = 2.7_f32;
    let zoom = Camera::default().dist / cam.dist;
    let (cy, sy) = (cam.yaw.cos(), cam.yaw.sin());
    let (cp, sp) = (cam.pitch.cos(), cam.pitch.sin());
    let mode2d = opts.mode2d;
    let project = |v: [f32; 3]| -> (Pos2, f32) {
        if mode2d {
            (
                Pos2::new(
                    center.x + v[0] * scale * zoom,
                    center.y - v[1] * scale * zoom,
                ),
                1.0 - v[2],
            )
        } else {
            let x = v[0] * cy - v[2] * sy;
            let z = v[0] * sy + v[2] * cy;
            let y = v[1] * cp - z * sp;
            let zc = v[1] * sp + z * cp + cam.dist;
            let f = focal / zc.max(0.1);
            (
                Pos2::new(center.x + x * f * scale, center.y - y * f * scale),
                zc,
            )
        }
    };

    // domain bounding box (3D only; Settings › Viewport can hide it)
    if !mode2d && opts.show_domain_bounds {
        let cs = [
            [-1., -1., -1.],
            [1., -1., -1.],
            [1., 1., -1.],
            [-1., 1., -1.],
            [-1., -1., 1.],
            [1., -1., 1.],
            [1., 1., 1.],
            [-1., 1., 1.],
        ];
        let edges = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        let box_stroke = Stroke::new(1.0, OUTLINE_VARIANT.gamma_multiply(0.9));
        for (a, b) in edges {
            p.line_segment([project(cs[a]).0, project(cs[b]).0], box_stroke);
        }
    }

    // project + cull + slice-clip every particle once
    let thr = (opts.density_lo - 0.5).max(0.0); // density -> |vort| threshold
    let mut proj: Vec<Proj> = Vec::with_capacity(particles.len());
    for pt in particles {
        if pt.vort.abs() < thr {
            continue;
        }
        let mut clipped = false;
        for a in 0..3 {
            if let Some(pos) = opts.slice[a] {
                if pt.pos[a] < pos * 2.0 - 1.0 {
                    clipped = true;
                    break;
                }
            }
        }
        if clipped {
            continue;
        }
        let (s, depth) = project(pt.pos);
        if !rect.expand(60.0).contains(s) {
            continue;
        }
        let fade = if mode2d {
            1.0
        } else {
            (2.2 / depth).clamp(0.15, 1.0)
        };
        let shadow = if opts.shadows && !mode2d {
            0.45 + 0.55 * fade
        } else {
            1.0
        };
        let weight = opts.opacity * fade * (0.35 + 0.65 * pt.speed) * shadow;
        let r_pts = if mode2d {
            (3.0 * zoom).clamp(1.5, 4.0)
        } else {
            (fade * 3.0).clamp(1.2, 3.4)
        };
        // HDR gain: push the fast, high-vorticity cores above 1.0 so they bloom
        let hot = (pt.speed * (0.45 + 0.55 * pt.vort.abs())).clamp(0.0, 1.4);
        let gain = 0.75 + 2.4 * hot;
        let ndc = [
            (s.x - rect.min.x) / rect.width() * 2.0 - 1.0,
            1.0 - (s.y - rect.min.y) / rect.height() * 2.0,
        ];
        proj.push(Proj {
            screen: s,
            ndc,
            depth,
            r_pts,
            base: colormap_rgb(pt.vort),
            weight,
            gain,
        });
    }

    if opts.gpu {
        let ppp = ui.ctx().pixels_per_point();
        let instances: Vec<GpuInstance> = proj
            .iter()
            .map(|q| GpuInstance {
                pos: q.ndc,
                radius_px: q.r_pts * ppp,
                weight: q.weight,
                color: [
                    q.base[0] * q.gain,
                    q.base[1] * q.gain,
                    q.base[2] * q.gain,
                    1.0,
                ],
            })
            .collect();
        let segments = if opts.streamlines {
            streamline_segments(&project, particles, rect, ppp)
        } else {
            Vec::new()
        };
        gpu::add_flow(ui, rect, instances, segments);
    } else {
        // CPU fallback: depth-sorted faint halo + bright core (soft glow, no GPU)
        proj.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for q in &proj {
            let col = Color32::from_rgba_unmultiplied(
                (q.base[0] * 255.0) as u8,
                (q.base[1] * 255.0) as u8,
                (q.base[2] * 255.0) as u8,
                (q.weight.clamp(0.0, 1.0) * 255.0) as u8,
            );
            let halo = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), col.a() / 3);
            p.circle_filled(q.screen, q.r_pts * 2.1, halo);
            p.circle_filled(q.screen, q.r_pts, col);
        }
        if opts.streamlines {
            for poly in streamline_polys(&project, particles) {
                p.line(poly, Stroke::new(1.0, GOLD.gamma_multiply(0.5)));
            }
        }
    }

    // billboarded critical points, projected with this mode's own camera
    draw_markers(ui, rect, cam, opts, Some(&project));
}

/// Project a few streamlines (seeded from strong-vorticity particles, advected
/// through the ABC field) to screen polylines. Shared by the CPU (egui lines)
/// and GPU (glowing ribbons) paths.
fn streamline_polys(
    project: &impl Fn([f32; 3]) -> (Pos2, f32),
    particles: &[Particle],
) -> Vec<Vec<Pos2>> {
    let seeds: Vec<[f32; 3]> = particles
        .iter()
        .filter(|q| q.vort.abs() > 0.6)
        .take(40)
        .map(|q| q.pos)
        .collect();
    let mut polys = Vec::with_capacity(seeds.len());
    for s in seeds {
        let mut pos = s;
        let mut poly = Vec::with_capacity(24);
        for _ in 0..24 {
            poly.push(project(pos).0);
            let (x, y, z) = (
                pos[0] * std::f32::consts::PI,
                pos[1] * std::f32::consts::PI,
                pos[2] * std::f32::consts::PI,
            );
            let v = [
                (z.sin() + y.cos()),
                (x.sin() + z.cos()),
                (y.sin() + x.cos()),
            ];
            for k in 0..3 {
                pos[k] = (pos[k] + v[k] * 0.02).clamp(-1.0, 1.0);
            }
        }
        polys.push(poly);
    }
    polys
}

/// The same streamlines as GPU ribbon segments (NDC endpoints, HDR gold so they
/// bloom into glowing tubes).
fn streamline_segments(
    project: &impl Fn([f32; 3]) -> (Pos2, f32),
    particles: &[Particle],
    rect: Rect,
    ppp: f32,
) -> Vec<SegInstance> {
    let to_ndc = |s: Pos2| {
        [
            (s.x - rect.min.x) / rect.width() * 2.0 - 1.0,
            1.0 - (s.y - rect.min.y) / rect.height() * 2.0,
        ]
    };
    let mut segs = Vec::new();
    for poly in streamline_polys(project, particles) {
        for w in poly.windows(2) {
            segs.push(SegInstance {
                p0: to_ndc(w[0]),
                p1: to_ndc(w[1]),
                width_px: 1.6 * ppp,
                _pad: 0.0,
                color: [1.5, 1.0, 0.35, 1.0], // gold, HDR
            });
        }
    }
    segs
}
