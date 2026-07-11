//! Reyn Studio shell — matches the 3D Volumetric Analysis mockup. (egui 0.35 API.)
use crate::icons::{self, Icon};
use crate::theme::*;
use crate::{engine, flow, viewport};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Rect, RichText,
    Sense, Stroke, Vec2,
};

#[derive(PartialEq, Clone, Copy)]
enum Nav { Models, FlowPainter, Metrics, Settings }

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
}

impl Default for ReynApp {
    fn default() -> Self {
        let engine = engine::EngineHandle::spawn();
        let current_model = "flow3d_obs_v1.pth".to_string();
        let _ = engine.tx.send(engine::Cmd::ListModels);
        let _ = engine.tx.send(engine::Cmd::Predict { model: current_model.clone(), seed: 1 });
        Self {
            nav: Nav::Metrics, volumetric: true,
            slice: [true, false, false], slice_pos: [0.50, 0.0, 0.0],
            density_lo: 0.85, density_hi: 1.0, opacity: 0.75,
            shadows: true, streamlines: false,
            cam: viewport::Camera::default(),
            particles: flow::generate(6000, 1), // procedural until the model field arrives
            seed: 1,
            engine, engine_status: "starting engine…".into(), engine_ok: false, current_model,
            models: Vec::new(),
        }
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
                    let n = f.shape.get(1).copied().unwrap_or(0);
                    self.engine_status = format!("● model field {n}³ · {}", f.scenario);
                    self.engine_ok = true;
                }
                engine::Msg::Error(e) => { self.engine_status = format!("○ {e}"); self.engine_ok = false; }
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::G)) {
            self.seed = self.seed.wrapping_add(1);
            if self.engine_ok {
                let _ = self.engine.tx.send(engine::Cmd::Predict { model: self.current_model.clone(), seed: self.seed });
                self.engine_status = "● predicting…".into();
            } else {
                self.particles = flow::generate(6000, self.seed);
            }
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
                    ui.add_space(30.0);
                    for m in ["File", "Edit", "View", "Simulation", "Window"] {
                        let active = m == "View";
                        ui.label(RichText::new(m).size(13.5)
                            .color(if active { TEXT } else { TEXT_DIM }));
                        ui.add_space(18.0);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if action_button(ui, Some(Icon::Play), "Live Session", BRAND, ON_EMBER, None, 34.0, 132.0) {}
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
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("checkpoint", &["pth"])
                        .set_directory(engine::research_dir())
                        .pick_file()
                    {
                        self.load_model(path.to_string_lossy().into_owned());
                    }
                }
                ui.add_space(18.0);

                if nav_row(ui, Icon::Orbit, "Models", self.nav == Nav::Models) { self.nav = Nav::Models; }
                if nav_row(ui, Icon::Brush, "Flow Painter", self.nav == Nav::FlowPainter) { self.nav = Nav::FlowPainter; }
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
                    foot_link(ui, Icon::Heart, "Support");
                    foot_link(ui, Icon::Book, "Docs");
                });
            });
    }

    fn right_controls(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("controls").exact_size(330.0).resizable(false)
            .frame(Frame::NONE.fill(BG).inner_margin(Margin::same(24))
                .stroke(Stroke::new(1.0, OUTLINE_VARIANT)))
            .show(ui, |ui| {
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
                });

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    action_button(ui, Some(Icon::Download), "EXPORT CALCULATIONS", GOLD, ON_EMBER, None, 44.0, ui.available_width());
                });
            });
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
                        slice_x: if self.slice[0] { Some(self.slice_pos[0]) } else { None },
                        streamlines: self.streamlines,
                    };
                    viewport::show(ui, rect, &mut self.cam, &opts, &self.particles);
                }

                let p = ui.painter_at(rect);
                // camera chip — live azimuth / elevation / zoom
                let cam_text = format!("Perspective  ·  az {:>3.0}°  el {:>3.0}°  ·  zoom {:.2}×",
                    self.cam.yaw.to_degrees().rem_euclid(360.0),
                    self.cam.pitch.to_degrees(),
                    viewport::Camera::default().dist / self.cam.dist);
                let cg = p.layout_no_wrap(cam_text, FontId::monospace(12.0), TEXT_DIM);
                let chip = Rect::from_min_size(rect.min + Vec2::new(16.0, 16.0), Vec2::new(cg.size().x + 24.0, 30.0));
                p.rect_filled(chip, CornerRadius::same(3), SURFACE);
                p.rect_stroke(chip, CornerRadius::same(3), Stroke::new(1.0, OUTLINE_VARIANT), egui::StrokeKind::Inside);
                p.galley(egui::pos2(chip.min.x + 12.0, chip.center().y - cg.size().y / 2.0), cg, TEXT_DIM);

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
                } else {
                    let name = match self.nav {
                        Nav::Models => "Model Library", Nav::FlowPainter => "Flow Painter",
                        Nav::Settings => "Settings", Nav::Metrics => "",
                    };
                    p.text(rect.center(), Align2::CENTER_CENTER, name, FontId::proportional(22.0), TEXT_DIM);
                    p.text(rect.center() + Vec2::new(0.0, 30.0), Align2::CENTER_CENTER,
                        "wired next — the 3D viewport is live under Metrics (3D)", FontId::proportional(13.0), TEXT_MUTE);
                }
            });
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

fn foot_link(ui: &mut egui::Ui, icon: Icon, label: &str) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    let fg = if resp.hovered() { TEXT } else { TEXT_DIM };
    let p = ui.painter();
    let ir = Rect::from_min_size(rect.min + Vec2::new(2.0, 7.0), Vec2::splat(16.0));
    icons::draw(p, ir, icon, fg);
    p.text(rect.min + Vec2::new(28.0, 15.0), Align2::LEFT_CENTER, label, FontId::proportional(13.5), fg);
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
