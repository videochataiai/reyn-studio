//! Reyn Studio shell — matches the 3D Volumetric Analysis mockup. (egui 0.35 API.)
use crate::theme::*;
use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Vec2};

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
}

impl Default for ReynApp {
    fn default() -> Self {
        Self {
            nav: Nav::Metrics,
            volumetric: true,
            slice: [true, false, false],
            slice_pos: [0.50, 0.0, 0.0],
            density_lo: 0.85,
            density_hi: 1.0,
            opacity: 0.75,
            shadows: true,
            streamlines: false,
        }
    }
}

impl eframe::App for ReynApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.top_bar(ui);
        self.left_sidebar(ui);
        self.right_controls(ui);
        self.viewport(ui);
        ui.ctx().request_repaint(); // keep the (future) 3D animating
    }
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
                    ui.label(RichText::new("Reyn Studio").size(19.0).strong().color(BRAND));
                    ui.add_space(28.0);
                    for m in ["File", "Edit", "View", "Simulation", "Window"] {
                        let active = m == "View";
                        ui.label(RichText::new(m).size(14.0)
                            .color(if active { TEXT } else { TEXT_DIM }));
                        ui.add_space(14.0);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let live = egui::Button::new(RichText::new("▶  Live Session").size(13.0).color(ON_EMBER))
                            .fill(BRAND).corner_radius(CornerRadius::same(3));
                        ui.add(live);
                        ui.add_space(14.0);
                        Frame::NONE.fill(SURFACE_HIGH).corner_radius(CornerRadius::same(3))
                            .stroke(Stroke::new(1.0, OUTLINE_VARIANT)).inner_margin(3)
                            .show(ui, |ui| {
                                if seg(ui, "2D", !self.volumetric) { self.volumetric = false; }
                                if seg(ui, "3D VOLUMETRIC", self.volumetric) { self.volumetric = true; }
                            });
                    });
                });
            });
    }

    fn left_sidebar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("sidebar").exact_size(280.0).resizable(false)
            .frame(Frame::NONE.fill(SURFACE_LOWEST).inner_margin(Margin::same(18))
                .stroke(Stroke::new(1.0, OUTLINE_VARIANT)))
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Project Alpha").size(22.0).strong().color(TEXT));
                ui.label(mono("Neural CFD v2.4", TEXT_MUTE).size(12.0));
                ui.add_space(18.0);

                let import = egui::Button::new(RichText::new("⭱  Import Model").color(TEXT))
                    .fill(SURFACE_HIGH).stroke(Stroke::new(1.0, OUTLINE))
                    .min_size(Vec2::new(ui.available_width(), 40.0));
                ui.add(import);
                ui.add_space(20.0);

                self.nav_item(ui, "◍  Models", Nav::Models);
                self.nav_item(ui, "✎  Flow Painter", Nav::FlowPainter);
                self.nav_item(ui, "▦  Metrics (3D)", Nav::Metrics);
                self.nav_item(ui, "⚙  Settings", Nav::Settings);

                ui.add_space(24.0);
                Frame::NONE.fill(SURFACE).stroke(Stroke::new(1.0, OUTLINE_VARIANT))
                    .corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(caps("Voxel Diagnostics"));
                        ui.add_space(10.0);
                        diag(ui, "Helicity", "4.2e-3", BRAND);
                        diag(ui, "Enstrophy Vol.", "1.8e-2", BRAND);
                        diag(ui, "Q-Criterion", "0.85", GOLD);
                        diag(ui, "Voxel Count", "16.8M", TEXT);
                    });

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("♡  Support").color(TEXT_DIM));
                    ui.add_space(6.0);
                    ui.label(RichText::new("▤  Docs").color(TEXT_DIM));
                });
            });
    }

    fn right_controls(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("controls").exact_size(330.0).resizable(false)
            .frame(Frame::NONE.fill(BG).inner_margin(Margin::same(24))
                .stroke(Stroke::new(1.0, OUTLINE_VARIANT)))
            .show(ui, |ui| {
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
                                ui.add(egui::Slider::new(&mut self.slice_pos[i], 0.0..=1.0).show_value(false));
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
                Frame::NONE.fill(SURFACE).stroke(Stroke::new(1.0, OUTLINE_VARIANT))
                    .corner_radius(CornerRadius::same(3)).inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Density").color(TEXT_DIM));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(mono(&format!("{:.2} – {:.1}", self.density_lo, self.density_hi), EMBER).size(12.0));
                            });
                        });
                        ui.add_space(6.0);
                        ui.add(egui::Slider::new(&mut self.density_lo, 0.0..=self.density_hi).show_value(false));
                    });

                ui.add_space(18.0);
                ui.label(caps("Rendering Options"));
                ui.add_space(8.0);
                Frame::NONE.fill(SURFACE).stroke(Stroke::new(1.0, OUTLINE_VARIANT))
                    .corner_radius(CornerRadius::same(3)).inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Global Opacity").color(TEXT_DIM));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(mono(&format!("{:.0}%", self.opacity * 100.0), TEXT_DIM).size(12.0));
                            });
                        });
                        ui.add(egui::Slider::new(&mut self.opacity, 0.0..=1.0).show_value(false));
                        ui.add_space(8.0);
                        ui.checkbox(&mut self.shadows, RichText::new("Volumetric Shadows").color(TEXT_DIM));
                        ui.checkbox(&mut self.streamlines, RichText::new("Show Streamlines").color(TEXT_DIM));
                    });

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    let export = egui::Button::new(RichText::new("⭳  EXPORT CALCULATIONS").size(12.0).strong().color(ON_EMBER))
                        .fill(GOLD).corner_radius(CornerRadius::same(3))
                        .min_size(Vec2::new(ui.available_width(), 44.0));
                    ui.add(export);
                });
            });
    }

    fn viewport(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::from_rgb(0x0e, 0x0a, 0x07)))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let overlay = egui::Rect::from_min_size(rect.min + Vec2::new(16.0, 16.0), Vec2::new(240.0, 30.0));
                ui.painter().rect_filled(overlay, 3.0, SURFACE);
                ui.painter().text(overlay.left_center() + Vec2::new(12.0, 0.0),
                    egui::Align2::LEFT_CENTER, "Camera: Perspective | FOV: 45°",
                    egui::FontId::monospace(12.0), TEXT_DIM);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
                    "wgpu 3D viewport — next step", egui::FontId::proportional(15.0), TEXT_MUTE);
            });
    }

    fn nav_item(&mut self, ui: &mut egui::Ui, label: &str, nav: Nav) {
        let active = self.nav == nav;
        let (fill, text) = if active { (EMBER, ON_EMBER) } else { (Color32::TRANSPARENT, TEXT_DIM) };
        let resp = ui.add(egui::Button::new(RichText::new(label).size(14.0).color(text).strong())
            .fill(fill).stroke(Stroke::NONE)
            .min_size(Vec2::new(ui.available_width(), 38.0)));
        if resp.clicked() { self.nav = nav; }
    }
}

/// Returns true when clicked.
fn seg(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let (fill, color) = if active { (EMBER, ON_EMBER) } else { (Color32::TRANSPARENT, TEXT_DIM) };
    let b = egui::Button::new(RichText::new(label).size(11.0).strong().color(color))
        .fill(fill).corner_radius(CornerRadius::same(2)).stroke(Stroke::NONE);
    ui.add(b).clicked()
}

fn diag(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(TEXT_DIM));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(mono(value, color).size(14.0));
        });
    });
    ui.add_space(6.0);
}
