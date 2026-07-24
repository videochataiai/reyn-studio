//! Reyn Studio — fully-native neural-CFD workbench (egui + wgpu), linked to a
//! Python inference engine. Entry point: sets up the window and theme.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod benchmark_evidence;
mod benchmark_export;
mod cad;
mod engine;
mod engineering;
mod engineering_section;
mod field2d;
mod flow;
mod fonts;
mod gpu;
mod library;
#[cfg(target_os = "macos")]
mod menubar;
mod painter;
pub mod project;
mod project_lifecycle;
mod report;
mod settings;
mod signing;
mod theme;
mod units;
mod viewport;

fn main() -> eframe::Result<()> {
    if let Some(request) = signing::parse_verify_cli(std::env::args()) {
        let exit_code = match request.and_then(run_signature_verification) {
            Ok(outcome) => {
                println!(
                    "{:?} · {} · key {} · fingerprint {}",
                    outcome.status,
                    outcome.detail,
                    outcome.key_id.as_deref().unwrap_or("UNKNOWN"),
                    outcome
                        .key_fingerprint_sha256
                        .as_deref()
                        .unwrap_or("UNKNOWN")
                );
                if outcome.status.is_cryptographically_valid() {
                    0
                } else {
                    2
                }
            }
            Err(error) => {
                eprintln!("signature verification failed: {error}");
                2
            }
        };
        std::process::exit(exit_code);
    }

    // Dev/QA affordance (matches REYN_STUDIO_START_NAV): launch at an exact
    // window size, e.g. REYN_STUDIO_WINDOW=1100x700 for min-window audits.
    let inner_size = std::env::var("REYN_STUDIO_WINDOW")
        .ok()
        .and_then(|spec| {
            let (w, h) = spec.split_once('x')?;
            Some([w.parse().ok()?, h.parse().ok()?])
        })
        .unwrap_or([1440.0, 900.0]);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(inner_size)
            .with_min_inner_size([1100.0, 700.0])
            // Title text stays for Mission Control / the Dock; the native
            // titlebar keeps only its traffic lights (single chrome, §4.1) —
            // the in-app 44px top bar owns window identity.
            //
            // egui-winit maps `with_titlebar_shown(false)` to winit's
            // `with_titlebar_transparent(true)`: the titlebar (and its traffic
            // lights) stays, but macOS stops painting its default material as
            // an opaque strip *over* the fullsize content view.
            .with_title("Reyn Studio")
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
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
            Ok(Box::new(app::ReynApp::new(cc)))
        }),
    )
}

fn run_signature_verification(
    request: signing::VerifyCliRequest,
) -> Result<signing::VerificationOutcome, String> {
    let report = std::fs::read_to_string(&request.report)
        .map_err(|error| format!("could not read {}: {error}", request.report.display()))?;
    let signature = std::fs::read_to_string(&request.signature)
        .map_err(|error| format!("could not read {}: {error}", request.signature.display()))?;
    let artifact = signing::SignedEvidenceArtifact::from_json(&signature)
        .map_err(|error| error.to_string())?;
    let policy = signing::VerificationPolicy::new(
        request.trusted_fingerprints,
        request.revoked_fingerprints,
    );
    Ok(benchmark_export::verify_report_signature(
        &report, &artifact, &policy,
    ))
}
