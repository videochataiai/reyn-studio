//! Reyn Studio — fully-native neural-CFD workbench (egui + wgpu), linked to a
//! Python inference engine. Entry point: sets up the window and theme.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod flow;
mod fonts;
mod icons;
mod theme;
mod viewport;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1100.0, 700.0])
            .with_title("Reyn Studio"),
        // wgpu backend = native Metal on macOS, Vulkan on Linux, DX12 on Windows.
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Reyn Studio",
        options,
        Box::new(|cc| {
            fonts::install(&cc.egui_ctx);
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(app::ReynApp::default()))
        }),
    )
}
