//! Reyn Studio shell — matches the 3D Volumetric Analysis mockup. (egui 0.35 API.)
use crate::field2d::{self, FieldVar};
use crate::icons::{self, Icon};
use crate::theme::*;
use crate::{engine, flow, gpu, viewport};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Rect, RichText,
    Sense, Stroke, Vec2,
};

#[derive(PartialEq, Clone, Copy)]
enum Nav { Models, FlowPainter, Fields2D, Metrics, Settings }

#[derive(PartialEq, Clone, Copy)]
enum PMethod { Spectral, Fd }
#[derive(PartialEq, Clone, Copy)]
enum PBoundary { Periodic, Dirichlet }

pub struct ReynApp {
    nav: Nav,
    volumetric: bool,
    slice: [bool; 3],
    slice_pos: [f32; 3],
    density_lo: f32,
    density_hi: f32,
    opacity: f32,
    shadows: bool,
    streamlines: bool,
    cam: viewport::Camera,
    particles: Vec<flow::Particle>,
    seed: u64,
    engine: engine::EngineHandle,
    engine_status: String,
    engine_ok: bool,
    current_model: String,
    models: Vec<String>,
    live: bool,
    live_timer: f32,
    gpu_ready: bool,
    render_volume: bool,
    volume_data: std::sync::Arc<Vec<u8>>,
    volume_dims: [u32; 3],
    volume_version: u64,
    // N3 — 2D pressure-recovery view
    f2d: Option<engine::Field2D>,
    f2d_var: FieldVar,
    f2d_horizon: u32,
    f2d_truth: bool,
    f2d_model: String,
    f2d_pending: bool,
    f2d_dirty: bool,
    f2d_req_at: Option<std::time::Instant>,
    f2d_latency_ms: u32,
    f2d_gen: u64,
    f2d_tex: Vec<egui::TextureHandle>,
    f2d_sig: u64,
    f2d_method: PMethod,
    f2d_tol_exp: i32, // FD tolerance = 10^-exp
    f2d_boundary: PBoundary,
}

impl Default for ReynApp {
    fn default() -> Self {
        let engine = engine::EngineHandle::spawn();
        let current_model = "flow3d_obs_v1.pth".to_string();
        let _ = engine.tx.send(engine::Cmd::ListModels);
        let _ = engine.tx.send(engine::Cmd::Predict { model: current_model.clone(), seed: 1 });
        let (vol, vdims) = flow::procedural_volume(48, 1); // placeholder until a field arrives
        Self {
            nav: Nav::Metrics, volumetric: true,
            slice: [true, false, false], slice_pos: [0.50, 0.0, 0.0],
            density_lo: 0.85, density_hi: 1.0, opacity: 0.75,
            shadows: true, streamlines: false,
            cam: viewport::Camera::default(),
            particles: flow::generate(6000, 1), // procedural until the model field arrives
            seed: 1,
            engine, engine_status: "starting engine…".into(), engine_ok: false, current_model,
            models: Vec::new(), live: false, live_timer: 0.0, gpu_ready: false,
            render_volume: false,
            volume_data: std::sync::Arc::new(vol), volume_dims: vdims, volume_version: 1,
            f2d: None, f2d_var: FieldVar::Vorticity, f2d_horizon: 8, f2d_truth: false,
            f2d_model: "obstacle_v2_shapes.pth".into(),
            f2d_pending: false, f2d_dirty: false, f2d_req_at: None, f2d_latency_ms: 0,
            f2d_gen: 0, f2d_tex: Vec::new(), f2d_sig: u64::MAX,
            f2d_method: PMethod::Spectral, f2d_tol_exp: 5, f2d_boundary: PBoundary::Periodic,
        }
    }
}

impl ReynApp {
    /// Build the app and register the native wgpu bloom renderer (N2). Falls
    /// back to the CPU glow if the wgpu backend is somehow unavailable.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        if let Some(rs) = cc.wgpu_render_state.as_ref() {
            gpu::install(rs);
            app.gpu_ready = true;
        }
        app
    }
}

impl eframe::App for ReynApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // drain engine messages (non-blocking)
        while let Ok(msg) = self.engine.rx.try_recv() {
            match msg {
                engine::Msg::Status(s) => { self.engine_status = s; self.engine_ok = true; }
                engine::Msg::Models(m) => { self.models = m; }
                engine::Msg::Field(f) => {
                    let ps = flow::from_field(&f.shape, &f.data);
                    if !ps.is_empty() { self.particles = ps; }
                    if let Some((vol, dims)) = flow::vorticity_volume(&f.shape, &f.data) {
                        self.volume_data = std::sync::Arc::new(vol);
                        self.volume_dims = dims;
                        self.volume_version = self.volume_version.wrapping_add(1);
                    }
                    let n = f.shape.get(1).copied().unwrap_or(0);
                    self.engine_status = format!("● model field {n}³ · {}", f.scenario);
                    self.engine_ok = true;
                }
                engine::Msg::Field2D(f) => {
                    if let Some(t) = self.f2d_req_at.take() {
                        self.f2d_latency_ms = t.elapsed().as_millis() as u32;
                    }
                    self.f2d = Some(f);
                    self.f2d_gen = self.f2d_gen.wrapping_add(1);
                    self.f2d_pending = false;
                    self.engine_ok = true;
                    if self.f2d_dirty { self.f2d_dirty = false; self.request_2d(); }
                }
                engine::Msg::Error(e) => { self.engine_status = format!("○ {e}"); self.engine_ok = false; self.f2d_pending = false; }
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::G)) { self.regenerate(); }
        if self.live {
            self.live_timer += ui.input(|i| i.stable_dt);
            if self.live_timer > 2.5 { self.live_timer = 0.0; self.regenerate(); }
        }
        self.top_bar(ui);
        self.left_sidebar(ui);
        self.right_controls(ui);
        self.viewport(ui);
        ui.ctx().request_repaint();
    }
}

/// Live diagnostics derived from the current field: (helicity, enstrophy, q, count).
fn diagnostics(ps: &[flow::Particle]) -> (f32, f32, f32, usize) {
    if ps.is_empty() { return (0.0, 0.0, 0.0, 0); }
    let n = ps.len() as f32;
    let mut hel = 0.0; let mut ens = 0.0; let mut q = 0.0;
    for p in ps {
        hel += p.vort * p.speed;
        ens += p.vort * p.vort;
        q += 0.5 * (p.speed * p.speed - p.vort * p.vort);
    }
    (hel / n * 0.1, ens / n, q / n, ps.len())
}

fn caps(text: &str) -> RichText {
    RichText::new(text.to_uppercase()).size(10.0).color(TEXT_MUTE).strong()
}
fn mono(text: &str, color: Color32) -> RichText {
    RichText::new(text).monospace().color(color)
}

impl ReynApp {
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("top").exact_size(52.0).resizable(false)
            .frame(Frame::NONE.fill(SURFACE_LOWEST).inner_margin(Margin::symmetric(20, 0))
                .stroke(Stroke::new(1.0, OUTLINE_VARIANT)))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("Reyn Studio").size(18.0).strong().color(BRAND));
                    ui.add_space(24.0);
                    ui.menu_button(RichText::new("File").size(13.5).color(TEXT_DIM), |ui| {
                        if ui.button("Import Model…").clicked() { self.import_model(); }
                        if ui.button("Export Calculations…").clicked() { self.export(); }
                        ui.separator();
                        if ui.button("Quit").clicked() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close); }
                    });
                    ui.menu_button(RichText::new("Edit").size(13.5).color(TEXT_DIM), |ui| {
                        if ui.button("Reset Controls").clicked() { self.reset_controls(); }
                    });
                    ui.menu_button(RichText::new("View").size(13.5).color(TEXT_DIM), |ui| {
                        if ui.button("Reset Camera").clicked() { self.cam = viewport::Camera::default(); }
                        if ui.button(if self.volumetric { "Switch to 2D" } else { "Switch to 3D" }).clicked() {
                            self.volumetric = !self.volumetric;
                        }
                    });
                    ui.menu_button(RichText::new("Simulation").size(13.5).color(TEXT_DIM), |ui| {
                        if ui.button("Regenerate Field").clicked() { self.regenerate(); }
                        if ui.button(if self.live { "Stop Live Session" } else { "Start Live Session" }).clicked() {
                            self.live = !self.live;
                        }
                    });
                    ui.menu_button(RichText::new("Window").size(13.5).color(TEXT_DIM), |ui| {
                        if ui.button("Minimize").clicked() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true)); }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let live_icon = if self.live { None } else { Some(Icon::Play) };
                        let live_label = if self.live { "◉  LIVE" } else { "Live Session" };
                        let live_fill = if self.live { EMBER } else { BRAND };
                        if action_button(ui, live_icon, live_label, live_fill, ON_EMBER, None, 34.0, 132.0) {
                            self.live = !self.live;
                        }
                        ui.add_space(14.0);
                        // 2D | 3D VOLUMETRIC segmented toggle (left-to-right order)
                        Frame::NONE.fill(SURFACE_HIGH).corner_radius(CornerRadius::same(3))
                            .stroke(Stroke::new(1.0, OUTLINE_VARIANT)).inner_margin(3)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    if seg(ui, "2D", !self.volumetric) { self.volumetric = false; }
                                    if seg(ui, "3D VOLUMETRIC", self.volumetric) { self.volumetric = true; }
                                });
                            });
                    });
                });
            });
    }

    fn left_sidebar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("sidebar").exact_size(276.0).resizable(false)
            .frame(Frame::NONE.fill(SURFACE_LOWEST).inner_margin(Margin::same(18))
                .stroke(Stroke::new(1.0, OUTLINE_VARIANT)))
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Project Alpha").size(21.0).strong().color(TEXT));
                ui.label(mono("Neural CFD v2.4", TEXT_MUTE).size(12.0));
                let stem = std::path::Path::new(&self.current_model).file_stem()
                    .and_then(|s| s.to_str()).unwrap_or(&self.current_model);
                ui.label(mono(&format!("{} · {} models", stem, self.models.len()), BRAND).size(11.0));
                ui.add_space(18.0);

                if action_button(ui, Some(Icon::Upload), "Import Model", SURFACE_HIGH, TEXT, Some(OUTLINE), 40.0, ui.available_width()) {
                    self.import_model();
                }
                ui.add_space(18.0);

                if nav_row(ui, Icon::Orbit, "Models", self.nav == Nav::Models) { self.nav = Nav::Models; }
                if nav_row(ui, Icon::Brush, "Flow Painter", self.nav == Nav::FlowPainter) { self.nav = Nav::FlowPainter; }
                if nav_row(ui, Icon::Layers, "Fields (2D)", self.nav == Nav::Fields2D) {
                    self.nav = Nav::Fields2D;
                    if self.f2d.is_none() && !self.f2d_pending { self.request_2d(); }
                }
                if nav_row(ui, Icon::Chart, "Metrics (3D)", self.nav == Nav::Metrics) { self.nav = Nav::Metrics; }
                if nav_row(ui, Icon::Gear, "Settings", self.nav == Nav::Settings) { self.nav = Nav::Settings; }

                ui.add_space(22.0);
                Frame::NONE.fill(SURFACE).stroke(Stroke::new(1.0, OUTLINE_VARIANT))
                    .corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let (hel, ens, q, count) = diagnostics(&self.particles);
                        ui.label(caps("Voxel Diagnostics"));
                        ui.add_space(12.0);
                        diag(ui, "Helicity", &format!("{:.1e}", hel), BRAND);
                        diag(ui, "Enstrophy Vol.", &format!("{:.2e}", ens), BRAND);
                        diag(ui, "Q-Criterion", &format!("{:.2}", q), GOLD);
                        diag(ui, "Voxel Count", &format!("{:.1}K", count as f32 / 1000.0), TEXT);
                    });

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.add_space(2.0);
                    if foot_link(ui, Icon::Heart, "Support") { open_url("mailto:support@reyn.studio"); }
                    if foot_link(ui, Icon::Book, "Docs") {
                        open_url(concat!("file://", env!("CARGO_MANIFEST_DIR"), "/PRD.md"));
                    }
                });
            });
    }

    fn right_controls(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("controls").exact_size(330.0).resizable(false)
            .frame(Frame::NONE.fill(BG).inner_margin(Margin::same(24))
                .stroke(Stroke::new(1.0, OUTLINE_VARIANT)))
            .show(ui, |ui| {
                if self.nav == Nav::Fields2D { self.controls_2d(ui); return; }
                ui.spacing_mut().slider_width = 120.0;
                ui.label(RichText::new("3D Controls").size(20.0).strong().color(TEXT));
                ui.add_space(20.0);

                ui.label(caps("Slicing Planes"));
                ui.add_space(8.0);
                for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
                    Frame::NONE.fill(SURFACE).stroke(Stroke::new(1.0, OUTLINE_VARIANT))
                        .corner_radius(CornerRadius::same(3)).inner_margin(Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.slice[i], "");
                                ui.label(RichText::new(*axis).color(TEXT).strong());
                                ui.add(egui::Slider::new(&mut self.slice_pos[i], 0.0..=1.0)
                                    .show_value(false).trailing_fill(true));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(mono(&format!("{:.2}", self.slice_pos[i]), TEXT_DIM).size(12.0));
                                });
                            });
                        });
                    ui.add_space(6.0);
                }

                ui.add_space(14.0);
                ui.label(caps("Isosurface Threshold"));
                ui.add_space(8.0);
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Density").color(TEXT_DIM));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(mono(&format!("{:.2} – {:.1}", self.density_lo, self.density_hi), EMBER).size(12.0));
                        });
                    });
                    ui.add_space(6.0);
                    ui.spacing_mut().slider_width = ui.available_width() - 8.0;
                    ui.add(egui::Slider::new(&mut self.density_lo, 0.0..=self.density_hi)
                        .show_value(false).trailing_fill(true));
                });

                ui.add_space(18.0);
                ui.label(caps("Rendering Options"));
                ui.add_space(8.0);
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Global Opacity").color(TEXT_DIM));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(mono(&format!("{:.0}%", self.opacity * 100.0), TEXT_DIM).size(12.0));
                        });
                    });
                    ui.spacing_mut().slider_width = ui.available_width() - 8.0;
                    ui.add(egui::Slider::new(&mut self.opacity, 0.0..=1.0).show_value(false).trailing_fill(true));
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.shadows, RichText::new("Volumetric Shadows").color(TEXT_DIM));
                    ui.checkbox(&mut self.streamlines, RichText::new("Show Streamlines").color(TEXT_DIM));
                    ui.add_enabled_ui(self.volumetric, |ui| {
                        ui.checkbox(&mut self.render_volume, RichText::new("Volume Raymarch").color(TEXT_DIM));
                    });
                });

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    if action_button(ui, Some(Icon::Download), "EXPORT CALCULATIONS", GOLD, ON_EMBER, None, 44.0, ui.available_width()) {
                        self.export();
                    }
                });
            });
    }

    fn regenerate(&mut self) {
        self.seed = self.seed.wrapping_add(1);
        if self.engine_ok {
            let _ = self.engine.tx.send(engine::Cmd::Predict { model: self.current_model.clone(), seed: self.seed });
            self.engine_status = "● predicting…".into();
        } else {
            self.particles = flow::generate(6000, self.seed);
            let (vol, dims) = flow::procedural_volume(48, self.seed);
            self.volume_data = std::sync::Arc::new(vol);
            self.volume_dims = dims;
            self.volume_version = self.volume_version.wrapping_add(1);
        }
    }

    /// Request a 2D prediction, coalesced to one in-flight request (TimeJump can
    /// fire many per second while dragging; stale ones are dropped, the newest
    /// re-fires when the current result lands).
    fn request_2d(&mut self) {
        if !self.engine_ok { return; }
        if self.f2d_pending { self.f2d_dirty = true; return; }
        let _ = self.engine.tx.send(engine::Cmd::Predict2D {
            model: self.f2d_model.clone(),
            steps: self.f2d_horizon,
            seed: 1,
            want_truth: self.f2d_truth,
            method: match self.f2d_method { PMethod::Spectral => "spectral", PMethod::Fd => "fd" }.into(),
            tolerance: 10f32.powi(-self.f2d_tol_exp),
            boundary: match self.f2d_boundary { PBoundary::Periodic => "periodic", PBoundary::Dirichlet => "dirichlet" }.into(),
        });
        self.f2d_pending = true;
        self.f2d_req_at = Some(std::time::Instant::now());
    }

    /// Rebuild the colormapped textures only when the field, variable, or overlay
    /// changed (not every frame).
    fn ensure_f2d_textures(&mut self, ctx: &egui::Context) {
        let Some(f) = &self.f2d else { return };
        let var_id = match self.f2d_var { FieldVar::Velocity => 0, FieldVar::Vorticity => 1, FieldVar::Pressure => 2 };
        let sig = self.f2d_gen.wrapping_mul(131) ^ (var_id << 1) ^ ((self.f2d_truth as u64) << 4);
        if sig == self.f2d_sig && !self.f2d_tex.is_empty() { return; }
        let opts = egui::TextureOptions::NEAREST;
        let mut tex = vec![ctx.load_texture("f2d.ai", field2d::image(f, &f.ai, self.f2d_var), opts)];
        if self.f2d_truth {
            if let Some(truth) = &f.truth {
                tex.push(ctx.load_texture("f2d.truth", field2d::image(f, truth, self.f2d_var), opts));
                tex.push(ctx.load_texture("f2d.err", field2d::error_image(f, &f.ai, truth, self.f2d_var), opts));
            }
        }
        self.f2d_tex = tex;
        self.f2d_sig = sig;
    }

    fn import_model(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("checkpoint", &["pth"])
            .set_directory(engine::research_dir())
            .pick_file()
        {
            self.load_model(path.to_string_lossy().into_owned());
        }
    }

    fn export(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("reyn_diagnostics.csv")
            .save_file()
        {
            let (hel, ens, q, count) = diagnostics(&self.particles);
            let csv = format!(
                "metric,value\nmodel,{}\nsamples,{}\nhelicity,{:.6e}\nenstrophy,{:.6e}\n\
                 q_criterion,{:.6}\ndensity_lo,{:.3}\nopacity,{:.3}\n",
                self.current_model, count, hel, ens, q, self.density_lo, self.opacity);
            let _ = std::fs::write(&path, csv);
            self.engine_status = format!("● exported {}",
                path.file_name().and_then(|s| s.to_str()).unwrap_or("file"));
        }
    }

    fn reset_controls(&mut self) {
        self.slice = [true, false, false];
        self.slice_pos = [0.5, 0.0, 0.0];
        self.density_lo = 0.85;
        self.opacity = 0.75;
        self.shadows = true;
        self.streamlines = false;
    }

    fn load_model(&mut self, path: String) {
        self.current_model = path;
        self.seed = self.seed.wrapping_add(1);
        if self.engine_ok {
            let _ = self.engine.tx.send(engine::Cmd::Predict { model: self.current_model.clone(), seed: self.seed });
            self.engine_status = "● predicting…".into();
        }
        self.nav = Nav::Metrics;
    }

    fn viewport(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::from_rgb(0x0e, 0x0a, 0x07)))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                {
                    let p = ui.painter_at(rect);
                    let grid = Stroke::new(1.0, OUTLINE_VARIANT.gamma_multiply(0.3));
                    let step = 40.0;
                    let mut x = rect.min.x;
                    while x < rect.max.x { p.line_segment([egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)], grid); x += step; }
                    let mut y = rect.min.y;
                    while y < rect.max.y { p.line_segment([egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)], grid); y += step; }
                }

                if self.nav == Nav::Metrics {
                    let opts = viewport::ViewOpts {
                        opacity: self.opacity,
                        density_lo: self.density_lo,
                        density_hi: self.density_hi,
                        slice: [
                            if self.slice[0] { Some(self.slice_pos[0]) } else { None },
                            if self.slice[1] { Some(self.slice_pos[1]) } else { None },
                            if self.slice[2] { Some(self.slice_pos[2]) } else { None },
                        ],
                        streamlines: self.streamlines,
                        shadows: self.shadows,
                        mode2d: !self.volumetric,
                        gpu: self.gpu_ready,
                        volume_mode: self.render_volume && self.volumetric,
                        volume: Some(gpu::VolumeData {
                            data: self.volume_data.clone(),
                            dims: self.volume_dims,
                            version: self.volume_version,
                        }),
                    };
                    viewport::show(ui, rect, &mut self.cam, &opts, &self.particles);
                }
                if self.nav == Nav::Fields2D {
                    self.field2d_view(ui, rect);
                }

                let p = ui.painter_at(rect);
                // camera chip — live azimuth / elevation / zoom (3D only)
                if self.nav == Nav::Metrics {
                    let cam_text = format!("Perspective  ·  az {:>3.0}°  el {:>3.0}°  ·  zoom {:.2}×",
                        self.cam.yaw.to_degrees().rem_euclid(360.0),
                        self.cam.pitch.to_degrees(),
                        viewport::Camera::default().dist / self.cam.dist);
                    let cg = p.layout_no_wrap(cam_text, FontId::monospace(12.0), TEXT_DIM);
                    let chip = Rect::from_min_size(rect.min + Vec2::new(16.0, 16.0), Vec2::new(cg.size().x + 24.0, 30.0));
                    p.rect_filled(chip, CornerRadius::same(3), SURFACE);
                    p.rect_stroke(chip, CornerRadius::same(3), Stroke::new(1.0, OUTLINE_VARIANT), egui::StrokeKind::Inside);
                    p.galley(egui::pos2(chip.min.x + 12.0, chip.center().y - cg.size().y / 2.0), cg, TEXT_DIM);
                }

                // engine status pill (top-right)
                let scol = if self.engine_ok { SUCCESS } else { EMBER };
                let galley = p.layout_no_wrap(self.engine_status.clone(), FontId::monospace(11.5), scol);
                let pw = galley.size().x + 24.0;
                let pill = Rect::from_min_size(egui::pos2(rect.max.x - pw - 16.0, rect.min.y + 16.0), Vec2::new(pw, 28.0));
                p.rect_filled(pill, CornerRadius::same(3), SURFACE);
                p.rect_stroke(pill, CornerRadius::same(3), Stroke::new(1.0, OUTLINE_VARIANT), egui::StrokeKind::Inside);
                p.galley(egui::pos2(pill.min.x + 12.0, pill.center().y - galley.size().y / 2.0), galley, scol);

                if self.nav == Nav::Metrics {
                    p.text(rect.center_bottom() - Vec2::new(0.0, 22.0), Align2::CENTER_CENTER,
                        "drag to orbit  ·  scroll to zoom  ·  G to regenerate", FontId::proportional(12.5), TEXT_MUTE);
                } else if self.nav != Nav::Fields2D {
                    let name = match self.nav {
                        Nav::Models => "Model Library", Nav::FlowPainter => "Flow Painter",
                        Nav::Settings => "Settings", _ => "",
                    };
                    p.text(rect.center(), Align2::CENTER_CENTER, name, FontId::proportional(22.0), TEXT_DIM);
                    p.text(rect.center() + Vec2::new(0.0, 30.0), Align2::CENTER_CENTER,
                        "wired next — the 3D viewport is live under Metrics (3D)", FontId::proportional(13.0), TEXT_MUTE);
                }
            });
    }

    /// Central 2D field render: the AI field (and, under Truth Overlay, the
    /// solver truth + error) as colormapped images.
    fn field2d_view(&mut self, ui: &mut egui::Ui, rect: Rect) {
        self.ensure_f2d_textures(ui.ctx());
        let p = ui.painter_at(rect);
        if self.f2d_tex.is_empty() {
            let t = if self.f2d_pending { "predicting…" } else { "no field" };
            p.text(rect.center(), Align2::CENTER_CENTER, t, FontId::proportional(15.0), TEXT_MUTE);
            return;
        }
        let pad = 30.0;
        let avail = Rect::from_min_max(rect.min + Vec2::splat(pad), rect.max - Vec2::new(pad, pad + 22.0));
        let n = self.f2d_tex.len();
        let gap = 16.0;
        let cell_w = (avail.width() - gap * (n as f32 - 1.0)) / n as f32;
        let side = cell_w.min(avail.height());
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let titles = ["AI prediction", "Solver truth", "|error|"];
        for (k, tex) in self.f2d_tex.iter().enumerate() {
            let x0 = avail.min.x + k as f32 * (cell_w + gap) + (cell_w - side) / 2.0;
            let y0 = avail.min.y + (avail.height() - side) / 2.0;
            let r = Rect::from_min_size(egui::pos2(x0, y0), Vec2::splat(side));
            p.image(tex.id(), r, uv, Color32::WHITE);
            p.rect_stroke(r, CornerRadius::same(3), Stroke::new(1.0, OUTLINE_VARIANT), egui::StrokeKind::Outside);
            if n > 1 {
                p.text(egui::pos2(r.center().x, r.max.y + 12.0), Align2::CENTER_CENTER,
                    titles[k.min(2)], FontId::proportional(12.0), TEXT_DIM);
            }
        }
        if let Some(f) = self.f2d.as_ref() {
            let cap = format!("{}  ·  {}  ·  t = {:.2}s ({} steps)",
                f.scenario, self.f2d_var.label(), f.horizon as f32 * f.dt_frame, f.horizon);
            p.text(rect.center_bottom() - Vec2::new(0.0, 14.0), Align2::CENTER_CENTER,
                cap, FontId::proportional(12.5), TEXT_MUTE);
        }
    }

    fn controls_2d(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Pressure Recovery (2D)").size(20.0).strong().color(TEXT));
        ui.add_space(8.0);
        // model selector — the obstacle-family 2D checkpoints (all work with predict2d)
        let stem = |m: &str| m.trim_end_matches(".pth").to_string();
        let models: Vec<String> = self.models.iter().filter(|m| m.starts_with("obstacle")).cloned().collect();
        let mut pick = self.f2d_model.clone();
        egui::ComboBox::from_id_salt("f2d.model")
            .selected_text(RichText::new(stem(&pick)).color(BRAND).size(12.5))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for m in &models {
                    ui.selectable_value(&mut pick, m.clone(), stem(m));
                }
            });
        if pick != self.f2d_model {
            self.f2d_model = pick;
            self.f2d = None;
            self.f2d_tex.clear();
            self.f2d_sig = u64::MAX;
            self.request_2d();
        }
        ui.add_space(18.0);

        ui.label(caps("Field Variable"));
        ui.add_space(8.0);
        Frame::NONE.fill(SURFACE_HIGH).corner_radius(CornerRadius::same(3))
            .stroke(Stroke::new(1.0, OUTLINE_VARIANT)).inner_margin(3)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for v in [FieldVar::Velocity, FieldVar::Vorticity, FieldVar::Pressure] {
                        if seg(ui, v.label(), self.f2d_var == v) { self.f2d_var = v; }
                    }
                });
            });

        ui.add_space(16.0);
        ui.label(caps("TimeJump"));
        ui.add_space(8.0);
        let dt = self.f2d.as_ref().map(|f| f.dt_frame).unwrap_or(0.04);
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Horizon").color(TEXT_DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(mono(&format!("t = {:.2}s · {} steps", self.f2d_horizon as f32 * dt, self.f2d_horizon), EMBER).size(12.0));
                });
            });
            ui.add_space(6.0);
            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
            let resp = ui.add(egui::Slider::new(&mut self.f2d_horizon, 1..=32).show_value(false).trailing_fill(true));
            if resp.changed() { self.request_2d(); }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let (hud, col) = if self.f2d_pending {
                    ("● predicting…".to_string(), EMBER)
                } else {
                    (format!("◌ {} ms", self.f2d_latency_ms), TEXT_MUTE)
                };
                ui.label(mono(&hud, col).size(11.0));
                if self.f2d_horizon > 16 {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(mono("beyond trained horizon", GOLD).size(11.0));
                    });
                }
            });
        });

        ui.add_space(16.0);
        ui.label(caps("Trust Meter"));
        ui.add_space(8.0);
        card(ui, |ui| {
            match self.f2d.as_ref().and_then(|f| f.semigroup) {
                Some(s) => {
                    let pct = (1.0 - s).clamp(0.0, 1.0) * 100.0;
                    let col = if pct >= 98.0 { SUCCESS } else if pct >= 90.0 { GOLD } else { EMBER };
                    diag(ui, "Self-consistency", &format!("{:.1}%", pct), col);
                    ui.label(RichText::new("semigroup: predict h  vs  h/2 then h/2").size(10.5).color(TEXT_MUTE));
                }
                None => { diag(ui, "Self-consistency", "— (odd h)", TEXT_MUTE); }
            }
        });

        ui.add_space(16.0);
        ui.label(caps("Truth Overlay"));
        ui.add_space(8.0);
        card(ui, |ui| {
            if ui.checkbox(&mut self.f2d_truth, RichText::new("Compare to solver truth").color(TEXT_DIM)).changed() {
                self.request_2d();
            }
            if self.f2d_truth {
                if let Some((rel, per)) = self.f2d.as_ref().and_then(|f| f.rel_l2.zip(f.persist)) {
                    ui.add_space(6.0);
                    diag(ui, "RelL2 vs truth", &format!("{:.4}", rel), if rel < per { SUCCESS } else { EMBER });
                    diag(ui, "Persistence floor", &format!("{:.4}", per), TEXT_DIM);
                    let x = per / rel.max(1e-6);
                    diag(ui, "Beats persistence", &format!("{:.1}×", x), if x > 1.0 { SUCCESS } else { EMBER });
                }
            }
        });

        ui.add_space(16.0);
        ui.label(caps("Pressure Recovery"));
        ui.add_space(8.0);
        let mut recompute = false;
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if seg(ui, "Spectral", self.f2d_method == PMethod::Spectral) && self.f2d_method != PMethod::Spectral {
                    self.f2d_method = PMethod::Spectral; recompute = true;
                }
                if seg(ui, "FD (iterative)", self.f2d_method == PMethod::Fd) && self.f2d_method != PMethod::Fd {
                    self.f2d_method = PMethod::Fd; recompute = true;
                }
            });
            if self.f2d_method == PMethod::Fd {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Tolerance").color(TEXT_DIM).size(12.5));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(mono(&format!("1e-{}", self.f2d_tol_exp), EMBER).size(12.0));
                    });
                });
                ui.spacing_mut().slider_width = ui.available_width() - 8.0;
                if ui.add(egui::Slider::new(&mut self.f2d_tol_exp, 2..=8).show_value(false).trailing_fill(true)).changed() {
                    recompute = true;
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if seg(ui, "Periodic", self.f2d_boundary == PBoundary::Periodic) && self.f2d_boundary != PBoundary::Periodic {
                        self.f2d_boundary = PBoundary::Periodic; recompute = true;
                    }
                    if seg(ui, "Dirichlet", self.f2d_boundary == PBoundary::Dirichlet) && self.f2d_boundary != PBoundary::Dirichlet {
                        self.f2d_boundary = PBoundary::Dirichlet; recompute = true;
                    }
                });
            }
            ui.add_space(10.0);
            if action_button(ui, None, "RECOMPUTE PRESSURE", SURFACE_HIGH, TEXT, Some(OUTLINE), 32.0, ui.available_width()) {
                recompute = true;
            }
            if let Some(f) = self.f2d.as_ref() {
                ui.add_space(10.0);
                let good = f.p_residual < 1e-3;
                diag(ui, &format!("Recovery error · {}", f.p_method), &format!("{:.1e}", f.p_residual),
                    if good { SUCCESS } else { GOLD });
                if f.p_iters > 0 { diag(ui, "CG iterations", &format!("{}", f.p_iters), TEXT_DIM); }
                diag(ui, "Peak / Low", &format!("{:.2} / {:.2}", f.peak_p, f.low_p), BRAND);
            }
        });
        if recompute { self.request_2d(); }
    }
}

// -- reusable widgets --------------------------------------------------------
fn seg(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let (fill, color) = if active { (EMBER, ON_EMBER) } else { (Color32::TRANSPARENT, TEXT_DIM) };
    ui.add(egui::Button::new(RichText::new(label).size(11.0).strong().color(color))
        .fill(fill).corner_radius(CornerRadius::same(2)).stroke(Stroke::NONE)).clicked()
}

fn nav_row(ui: &mut egui::Ui, icon: Icon, label: &str, active: bool) -> bool {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 40.0), Sense::click());
    let (bg, fg) = if active { (EMBER, ON_EMBER) }
        else if resp.hovered() { (SURFACE_HIGH, TEXT) }
        else { (Color32::TRANSPARENT, TEXT_DIM) };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(4), bg);
    let ir = Rect::from_min_size(rect.min + Vec2::new(12.0, 11.0), Vec2::splat(18.0));
    icons::draw(p, ir, icon, fg);
    p.text(rect.min + Vec2::new(42.0, 20.0), Align2::LEFT_CENTER, label, FontId::proportional(14.5), fg);
    resp.clicked()
}

fn foot_link(ui: &mut egui::Ui, icon: Icon, label: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    let fg = if resp.hovered() { TEXT } else { TEXT_DIM };
    let p = ui.painter();
    let ir = Rect::from_min_size(rect.min + Vec2::new(2.0, 7.0), Vec2::splat(16.0));
    icons::draw(p, ir, icon, fg);
    p.text(rect.min + Vec2::new(28.0, 15.0), Align2::LEFT_CENTER, label, FontId::proportional(13.5), fg);
    resp.clicked()
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/C", "start", url]).spawn();
}

/// Centered icon+label button. `border` gives a ghost style.
fn action_button(ui: &mut egui::Ui, icon: Option<Icon>, label: &str, fill: Color32,
    fg: Color32, border: Option<Color32>, height: f32, width: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let bg = if resp.hovered() { fill.gamma_multiply(1.12) } else { fill };
    let font = FontId::proportional(13.5);
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(3), bg);
    if let Some(b) = border {
        p.rect_stroke(rect, CornerRadius::same(3), Stroke::new(1.0, b), egui::StrokeKind::Inside);
    }
    let galley = p.layout_no_wrap(label.to_owned(), font, fg);
    let icon_w = if icon.is_some() { 24.0 } else { 0.0 };
    let start = rect.center().x - (icon_w + galley.size().x) / 2.0;
    if let Some(ic) = icon {
        let ir = Rect::from_min_size(egui::pos2(start, rect.center().y - 8.0), Vec2::splat(16.0));
        icons::draw(p, ir, ic, fg);
    }
    let gpos = egui::pos2(start + icon_w, rect.center().y - galley.size().y / 2.0);
    p.galley(gpos, galley, fg);
    resp.clicked()
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) {
    Frame::NONE.fill(SURFACE).stroke(Stroke::new(1.0, OUTLINE_VARIANT))
        .corner_radius(CornerRadius::same(3)).inner_margin(Margin::same(14))
        .show(ui, |ui| { ui.set_width(ui.available_width()); add(ui); });
}

fn diag(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(TEXT_DIM));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(mono(value, color).size(14.0));
        });
    });
    ui.add_space(7.0);
}
