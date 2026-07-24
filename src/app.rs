//! Reyn Studio shell — matches the 3D Volumetric Analysis mockup. (egui 0.35 API.)
use crate::benchmark_evidence::{
    InspectorVariable, INSPECTOR_DERIVATIVE, INSPECTOR_DOMAIN, INSPECTOR_LAYOUT,
    INSPECTOR_PRESSURE, INSPECTOR_PROTOCOL_VERSION, INSPECTOR_SCHEMA,
};
use crate::field2d::{self, FieldVar};
#[cfg(target_os = "macos")]
use crate::menubar::{MenuBar, MenuCommand, MenuSignal, MenuSyncState};
use crate::signing::LocalSigningKeyStore;
use crate::theme::*;
use crate::{
    cad, engine, engineering, engineering_section, flow, gpu, library, painter, project,
    project_lifecycle, report, settings, signing, units, viewport,
};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Rect, RichText, Sense,
    Stroke, Vec2,
};
use egui_phosphor::regular as ph;
use sha2::{Digest, Sha256};

#[derive(PartialEq, Clone, Copy)]
enum Nav {
    Projects,
    Case,
    Results,
    Evidence,
    Models,
    Settings,
    // Permanently available only when Settings → Developer enables the sandbox.
    FlowPainter,
    Fields2D,
    Metrics,
    Benchmark,
}

/// The active external-flow case and its exact CAD/result data.
struct CadCase {
    mask: std::sync::Arc<Vec<f32>>,
    model: String,
    steps: u32,
    surf: Option<gpu::SurfaceData>,
    name: String,
    workflow: engineering::ExternalFlowCase,
    velocity: Vec<f32>,
    pressure: Vec<f32>,
    cp: Vec<f32>,
    traction: Vec<f32>,
    result_grid: usize,
    active_run_id: Option<String>,
    pending: bool,
    pending_request_id: Option<String>,
    pending_run: Option<PendingCadRun>,
}

#[derive(Clone)]
struct PendingCadRun {
    request_id: String,
    workflow: engineering::ExternalFlowCase,
    started_at: std::time::Instant,
}

#[derive(PartialEq, Clone, Copy)]
enum PMethod {
    Spectral,
    Fd,
}
#[derive(PartialEq, Clone, Copy)]
enum PBoundary {
    Periodic,
    Dirichlet,
}

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
    last_window_title: String,
    current_model: String,
    models: Vec<engine::ModelCard>,
    library: library::LibraryState,
    settings: settings::AppSettings,
    settings_draft: settings::AppSettings,
    settings_notice: Option<(String, bool)>,
    signing_notice: Option<(String, bool)>,
    settings_ui: settings::SettingsUiState,
    /// Session copy of the per-field Case Setup entry units (seeded from
    /// settings; switching a unit here never changes the stored SI value).
    input_units: units::InputUnitPrefs,
    preset_name_draft: String,
    preset_notice: Option<(String, bool)>,
    /// Last frame's render-viewport rect (points) for PNG capture cropping.
    last_render_rect: Option<Rect>,
    /// Destination for an in-flight composited-frame screenshot.
    pending_viewport_shot: Option<std::path::PathBuf>,
    project: project_lifecycle::ProjectLifecycle,
    project_name_draft: String,
    project_guard: project_lifecycle::UnsavedChangesGuard,
    project_notice: Option<(String, bool)>,
    allow_close: bool,
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
    insights_on: bool,
    insight_classes: [bool; 5], // P▲, P▼, ω, v, ε — mirrors InsightKind order
    insights3d: Vec<flow::Insight3D>, // critical points of the current 3D field
    f2d_scale: f32,             // shared model/reference colormap scale (for the legend)
    f2d_signed: bool,           // diverging vs sequential legend
    // N4 — Flow Painter
    paint: painter::PaintField,
    paint_sym: painter::Symmetry,
    brush_radius: f32,
    brush_strength: f32,
    paint_tex: Option<egui::TextureHandle>,
    paint_dirty: bool,
    paint_last: Option<(f32, f32)>, // previous stamp in grid coords (stroke interp)
    f2d_painted: Option<std::sync::Arc<Vec<f32>>>, // active painted IC in Fields (2D)
    // CAD flow analysis
    cad: Option<CadCase>,
    waiver_draft: String,
    waiver_code: Option<String>,
    surface_on: bool,
    cad_version: u64,
    section_axis: engineering_section::SectionAxis,
    section_quantity: engineering_section::SectionQuantity,
    section_tex: Option<egui::TextureHandle>,
    section_data: Option<engineering_section::SectionPlane>,
    section_sig: u64,
    section_error: Option<String>,
    // N5 — Benchmark Lab
    bench: Option<engine::BenchResult>,
    bench_running: bool,
    bench_seeds: u32,
    bench_seed_start: u32,
    bench_selected: Option<(usize, usize)>,
    bench_inspector: Option<engine::BenchInspector>,
    bench_inspector_pending: bool,
    bench_error: Option<String>,
    bench_inspector_error: Option<String>,
    bench_tex: Vec<egui::TextureHandle>,
    bench_var: InspectorVariable,
    active_benchmark_run_id: Option<String>,
    // Shell chrome + motion bookkeeping
    #[cfg(target_os = "macos")]
    menubar: Option<MenuBar>,
    last_nav: Nav,
    nav_changed_at: f64,
    // Command palette (⌘K, §6 Tier 3.5) — actions only, same gating as nav.
    palette_open: bool,
    palette_query: String,
    palette_selected: usize,
}

impl Default for ReynApp {
    fn default() -> Self {
        let (settings, settings_warning) = settings::AppSettings::load();
        let project_state_directory = settings::config_path()
            .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
            .unwrap_or_else(|| std::path::PathBuf::from(".reyn-studio"));
        let (project, project_warnings) =
            project_lifecycle::ProjectLifecycle::load(project_state_directory, now_utc_unix());
        let project_name_draft = project.display_name().to_owned();
        let engine = engine::EngineHandle::spawn_with_config(settings.engine_config());
        let current_model = "flow3d_obs_v1.pth".to_string();
        let _ = engine.tx.send(engine::Cmd::ListModels);
        let (vol, vdims) = flow::procedural_volume(48, 1); // placeholder until a field arrives
        Self {
            nav: Nav::Projects,
            volumetric: true,
            slice: [true, false, false],
            slice_pos: [0.50, 0.0, 0.0],
            density_lo: 0.85,
            density_hi: 1.0,
            opacity: 0.75,
            shadows: true,
            streamlines: false,
            cam: viewport::Camera::default(),
            particles: flow::generate(6000, 1), // procedural until the model field arrives
            seed: 1,
            engine,
            engine_status: "○ Starting engine…".into(),
            engine_ok: false,
            last_window_title: String::new(),
            current_model,
            models: Vec::new(),
            library: library::LibraryState::default(),
            settings_draft: settings.clone(),
            settings,
            settings_notice: settings_warning.map(|warning| (warning, true)),
            signing_notice: None,
            settings_ui: settings::SettingsUiState::default(),
            input_units: units::InputUnitPrefs::default(),
            preset_name_draft: String::new(),
            preset_notice: None,
            last_render_rect: None,
            pending_viewport_shot: None,
            project,
            project_name_draft,
            project_guard: project_lifecycle::UnsavedChangesGuard::default(),
            project_notice: (!project_warnings.is_empty())
                .then(|| (project_warnings.join(" "), true)),
            allow_close: false,
            live: false,
            live_timer: 0.0,
            gpu_ready: false,
            render_volume: false,
            volume_data: std::sync::Arc::new(vol),
            volume_dims: vdims,
            volume_version: 1,
            f2d: None,
            f2d_var: FieldVar::Vorticity,
            f2d_horizon: 8,
            f2d_truth: false,
            f2d_model: "obstacle_v2_shapes.pth".into(),
            f2d_pending: false,
            f2d_dirty: false,
            f2d_req_at: None,
            f2d_latency_ms: 0,
            f2d_gen: 0,
            f2d_tex: Vec::new(),
            f2d_sig: u64::MAX,
            f2d_method: PMethod::Spectral,
            f2d_tol_exp: 5,
            f2d_boundary: PBoundary::Periodic,
            insights_on: true,
            insight_classes: [true; 5],
            insights3d: Vec::new(),
            f2d_scale: 1.0,
            f2d_signed: true,
            paint: painter::PaintField::default(),
            paint_sym: painter::Symmetry::default(),
            brush_radius: 7.0,
            brush_strength: 1.2,
            paint_tex: None,
            paint_dirty: true,
            paint_last: None,
            f2d_painted: None,
            cad: None,
            waiver_draft: String::new(),
            waiver_code: None,
            surface_on: false,
            cad_version: 0,
            section_axis: engineering_section::SectionAxis::X,
            section_quantity: engineering_section::SectionQuantity::PhysicalCp,
            section_tex: None,
            section_data: None,
            section_sig: u64::MAX,
            section_error: None,
            bench: None,
            bench_running: false,
            bench_seeds: 3,
            bench_seed_start: 70000,
            bench_selected: None,
            bench_inspector: None,
            bench_inspector_pending: false,
            bench_error: None,
            bench_inspector_error: None,
            bench_tex: Vec::new(),
            bench_var: InspectorVariable::Velocity,
            active_benchmark_run_id: None,
            #[cfg(target_os = "macos")]
            menubar: None,
            last_nav: Nav::Projects,
            nav_changed_at: f64::NEG_INFINITY,
            palette_open: false,
            palette_query: String::new(),
            palette_selected: 0,
        }
    }
}

impl ReynApp {
    /// Build the app and register the native wgpu bloom renderer (N2). Falls
    /// back to the CPU glow if the wgpu backend is somehow unavailable.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        apply_with_contrast(
            &cc.egui_ctx,
            app.settings.theme == settings::ThemeMode::HighContrast,
        );
        set_reduced_motion(&cc.egui_ctx, app.settings.reduced_motion);
        // Persisted display preferences take effect from the first frame.
        cc.egui_ctx.set_zoom_factor(app.settings.ui_scale);
        field2d::set_view_colormap(app.settings.colormap);
        app.section_axis = app.settings.default_section_axis;
        app.section_quantity = app.settings.default_section_quantity;
        app.input_units = app.settings.input_units;
        if let Some(rs) = cc.wgpu_render_state.as_ref() {
            gpu::install(rs);
            app.gpu_ready = true;
        }
        // Native macOS menu bar replaces the deleted in-app menu row (§4.1).
        // If installation fails, keyboard shortcuts and in-screen actions
        // keep every command reachable.
        #[cfg(target_os = "macos")]
        {
            app.menubar = MenuBar::install();
        }
        // Dev/QA affordance: pick the startup screen without clicking through
        // the rail (e.g. REYN_STUDIO_START_NAV=settings). Screens stay gated
        // exactly as in the UI — no state is faked to reach them.
        if let Ok(start) = std::env::var("REYN_STUDIO_START_NAV") {
            app.nav = match start.as_str() {
                "case" => Nav::Case,
                "evidence" => Nav::Evidence,
                "models" => Nav::Models,
                "settings" => Nav::Settings,
                _ => Nav::Projects,
            };
        }
        app
    }
}

impl eframe::App for ReynApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Complete any in-flight viewport PNG capture first: the screenshot
        // event carries the previous composited frame.
        self.handle_screenshot_events(ui.ctx());
        // drain engine messages (non-blocking)
        while let Ok(msg) = self.engine.rx.try_recv() {
            match msg {
                engine::Msg::Status(s) => {
                    self.engine_status = s;
                    self.engine_ok = true;
                    if self.nav == Nav::Fields2D && self.f2d.is_none() && !self.f2d_pending {
                        self.request_2d();
                    }
                }
                engine::Msg::Models(m) => {
                    self.models = m;
                    self.library.busy = false;
                }
                engine::Msg::ModelImported { model, models } => {
                    self.library.busy = false;
                    self.library.validation = None;
                    self.library.notice = Some((
                        format!("Imported {}; checkpoint contract validated", model.name),
                        false,
                    ));
                    self.models = models;
                    self.activate_model(&model.id);
                }
                engine::Msg::ModelImportRejected(validation) => {
                    self.library.busy = false;
                    // Single owner: the structured-validation panel renders the
                    // rejection (summary + verbatim issue codes) once; a second
                    // notice with the same string would duplicate it (A3).
                    self.library.notice = None;
                    self.library.validation = Some(validation);
                }
                engine::Msg::ModelDeleted { model, models } => {
                    self.library.busy = false;
                    self.library.pending_delete = None;
                    self.library.notice = Some((format!("Deleted {model}"), false));
                    self.models = models;
                }
                engine::Msg::Field(f) => {
                    let ps = flow::from_field(&f.shape, &f.data);
                    if !ps.is_empty() {
                        self.particles = ps;
                    }
                    if let Some((vol, dims)) = flow::vorticity_volume(&f.shape, &f.data) {
                        self.volume_data = std::sync::Arc::new(vol);
                        self.volume_dims = dims;
                        self.volume_version = self.volume_version.wrapping_add(1);
                    }
                    self.insights3d = flow::insights3d(&f.shape, &f.data);
                    let n = f.shape.get(1).copied().unwrap_or(0);
                    self.engine_status = format!("● Model field {n}³ · {}", f.scenario);
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
                    if self.f2d_dirty {
                        self.f2d_dirty = false;
                        self.request_2d();
                    }
                }
                engine::Msg::CadField(f) => {
                    let request_matches = self.cad.as_ref().is_some_and(|case| {
                        case.pending_run
                            .as_ref()
                            .is_some_and(|pending| pending.request_id == f.request_id)
                    });
                    if !request_matches {
                        self.project_notice = Some((
                            format!(
                                "Discarded CAD result for stale request {}. The active case and immutable runs were not changed.",
                                short_id(&f.request_id)
                            ),
                            true,
                        ));
                        continue;
                    }
                    self.invalidate_cad_section();
                    let persisted_run = self.persist_external_flow_run(&f);
                    let shape = vec![3usize, f.n, f.n, f.n];
                    let ps = flow::from_field(&shape, &f.vel);
                    if !ps.is_empty() {
                        self.particles = ps;
                    }
                    if let Some((vol, dims)) = flow::vorticity_volume(&shape, &f.vel) {
                        self.volume_data = std::sync::Arc::new(vol);
                        self.volume_dims = dims;
                        self.volume_version = self.volume_version.wrapping_add(1);
                    }
                    let mut ins = flow::insights3d(&shape, &f.vel);
                    ins.extend(cad::surface_insights(&f.mask, &f.cp, f.n));
                    self.insights3d = ins;
                    // surface layer textures (transposed to x-fastest for wgpu)
                    let n = f.n;
                    let cp_scale =
                        f.cp.iter()
                            .fold(0.0f32, |scale, value| scale.max(value.abs()))
                            .max(1e-6);
                    let mut mask_u8 = vec![0u8; n * n * n];
                    let mut p_u8 = vec![128u8; n * n * n];
                    for i in 0..n {
                        for j in 0..n {
                            for k in 0..n {
                                let src = i * n * n + j * n + k;
                                let dst = (k * n + j) * n + i;
                                mask_u8[dst] = (f.mask[src].clamp(0.0, 1.0) * 255.0) as u8;
                                let t = (f.cp[src] / cp_scale) * 0.5 + 0.5;
                                p_u8[dst] = (t.clamp(0.0, 1.0) * 255.0) as u8;
                            }
                        }
                    }
                    self.cad_version = self.cad_version.wrapping_add(1);
                    let surf = gpu::SurfaceData {
                        mask: std::sync::Arc::new(mask_u8),
                        pressure: std::sync::Arc::new(p_u8),
                        dims: [n as u32; 3],
                        version: self.cad_version,
                    };
                    if let Some(c) = &mut self.cad {
                        c.mask = std::sync::Arc::new(f.mask);
                        c.surf = Some(surf);
                        c.steps = f.horizon;
                        c.velocity = f.vel;
                        c.pressure = f.pressure;
                        c.cp = f.cp;
                        c.traction = f.traction;
                        c.result_grid = f.n;
                        c.pending = false;
                        c.pending_request_id = None;
                        c.pending_run = None;
                        match persisted_run {
                            Ok(run_id) => c.active_run_id = Some(run_id),
                            Err(error) => {
                                c.active_run_id = None;
                                self.project_notice = Some((
                                    format!(
                                        "Prediction completed, but immutable run persistence failed: {error}"
                                    ),
                                    true,
                                ));
                            }
                        }
                        c.workflow.stage = engineering::CaseStage::Results;
                        c.workflow.parent_run_id = c.active_run_id.clone();
                        c.workflow.result = Some(engineering::EngineeringResult {
                            method: f.load_method,
                            cp_min: c.cp.iter().copied().fold(f32::INFINITY, f32::min) as f64,
                            cp_max: c.cp.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64,
                            force_coefficients: f.force_coefficients.map(f64::from),
                            moment_coefficients: f.moment_coefficients.map(f64::from),
                            force_newtons: f.force_newtons.map(f64::from),
                            moment_newton_meters: f.moment_newton_meters.map(f64::from),
                            surface_area_m2: f.surface_area_m2 as f64,
                            pressure_force_fraction: f.pressure_force_fraction as f64,
                            load_hotspot: f.load_hotspot.map(f64::from),
                            suction_hotspot: f.suction_hotspot.map(f64::from),
                            divergence_rms: f.divergence_rms as f64,
                            wake_deficit_peak: f.wake_deficit_peak as f64,
                            wake_deficit_mean: f.wake_deficit_mean as f64,
                            warnings: f.warnings,
                        });
                    }
                    self.surface_on = true;
                    self.volumetric = true;
                    self.render_volume = true;
                    self.nav = Nav::Results;
                    self.engine_status = format!(
                        "● Engineering result {}³ · Re {:.0} · t = {} steps",
                        n, f.reynolds, f.horizon
                    );
                    self.engine_ok = true;
                }
                engine::Msg::Benchmark(b) => {
                    self.bench_running = false;
                    let expected_seeds: Vec<u32> = (0..self.bench_seeds)
                        .map(|offset| self.bench_seed_start.saturating_add(offset))
                        .collect();
                    let current_request = b.model == self.f2d_model
                        && b.seeds == expected_seeds
                        && b.horizons == [1, 4, 8, 16];
                    if current_request {
                        match self.persist_benchmark_run(&b) {
                            Ok(run_id) => {
                                self.active_benchmark_run_id = Some(run_id);
                            }
                            Err(error) => {
                                self.project_notice = Some((
                                    format!(
                                        "Suite completed, but its immutable project run was not recorded: {error}"
                                    ),
                                    true,
                                ));
                            }
                        }
                        self.bench = Some(b);
                        self.bench_error = None;
                        self.engine_ok = true;
                        self.select_bench_cell(0, 0);
                    } else {
                        self.bench_error = Some(
                            "discarded a suite result after its model or seed controls changed"
                                .into(),
                        );
                    }
                }
                engine::Msg::BenchmarkInspector(cell) => {
                    let belongs_to_selection = self.bench.as_ref().is_some_and(|benchmark| {
                        self.bench_selected
                            .is_some_and(|(seed_index, horizon_index)| {
                                benchmark.seeds.get(seed_index) == Some(&cell.seed)
                                    && benchmark.horizons.get(horizon_index) == Some(&cell.horizon)
                            })
                    });
                    if belongs_to_selection {
                        self.bench_inspector_pending = false;
                        if let Err(error) = self.persist_benchmark_inspector(&cell) {
                            self.project_notice = Some((
                                format!(
                                    "Cell evidence remains visible, but its project link was not recorded: {error}"
                                ),
                                true,
                            ));
                        }
                        self.bench_inspector = Some(cell);
                        self.bench_inspector_error = None;
                        self.bench_tex.clear();
                    } else if self.bench_inspector_pending {
                        self.bench_inspector_pending = false;
                        self.bench_inspector_error =
                            Some("discarded stale cell evidence after selection changed".into());
                    }
                    self.engine_ok = true;
                }
                engine::Msg::Error(e) => {
                    let inspector_failed = self.bench_inspector_pending;
                    let suite_failed = self.bench_running;
                    let library_failed = self.library.busy;
                    self.engine_status = format!("○ {e}");
                    if e.starts_with("engine io:") || e.starts_with("engine unavailable:") {
                        self.engine_ok = false;
                    }
                    self.f2d_pending = false;
                    self.bench_running = false;
                    self.bench_inspector_pending = false;
                    self.library.busy = false;
                    if let Some(case) = &mut self.cad {
                        if case.pending {
                            case.pending = false;
                            case.workflow.stage = engineering::CaseStage::Ready;
                        }
                    }
                    if inspector_failed {
                        self.bench_inspector_error = Some(e);
                    } else if suite_failed {
                        self.bench_error = Some(e);
                    } else if library_failed {
                        self.library.notice = Some((e, true));
                    }
                }
            }
        }
        if self.is_research_sandbox() && ui.input(|i| i.key_pressed(egui::Key::G)) {
            self.regenerate();
        }
        if self.nav == Nav::Benchmark {
            self.bench_keyboard(ui);
        }
        self.handle_project_shortcuts(ui);
        let available_model_hashes: Vec<String> = self
            .models
            .iter()
            .filter(|model| model.status != "invalid")
            .map(|model| model.checkpoint_sha256.clone())
            .collect();
        self.project.reconcile_dependencies(
            self.engine_ok,
            available_model_hashes.iter().map(String::as_str),
        );
        let now = now_utc_unix();
        if let Err(error) = self
            .project
            .autosave_if_due(now, self.settings.autosave_interval_seconds as u64)
        {
            self.project_notice = Some((format!("Recovery snapshot failed: {error}"), true));
        }
        if ui.input(|input| input.viewport().close_requested()) && !self.allow_close {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.project_guard.pending().is_none() {
                self.request_project_action(
                    project_lifecycle::DeferredProjectAction::Quit,
                    ui.ctx(),
                );
            }
        }
        if self.live && self.is_research_sandbox() {
            self.live_timer += ui.input(|i| i.stable_dt);
            if self.live_timer > 2.5 {
                self.live_timer = 0.0;
                self.regenerate();
            }
        } else if !self.is_research_sandbox() {
            self.live = false;
            self.live_timer = 0.0;
        }
        #[cfg(target_os = "macos")]
        self.handle_menu_signals(ui.ctx());
        self.top_bar(ui);
        self.status_bar(ui);
        self.left_sidebar(ui);
        self.right_controls(ui);
        self.viewport(ui);
        self.command_palette(ui.ctx());
        self.show_unsaved_changes_dialog(ui.ctx());
        // Repaint discipline (§5.4): full-rate only while engine work or the
        // sandbox loop is in flight; otherwise poll the engine channel at a
        // low cadence. Widget animations schedule their own repaints.
        let engine_busy = !self.engine_ok
            || self.f2d_pending
            || self.bench_running
            || self.bench_inspector_pending
            || self.library.busy
            || self.live
            || self.cad.as_ref().is_some_and(|case| case.pending);
        if engine_busy {
            ui.ctx().request_repaint();
        } else {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(150));
        }
    }
}

/// Live diagnostics derived from the current field: (helicity, enstrophy, q, count).
fn diagnostics(ps: &[flow::Particle]) -> (f32, f32, f32, usize) {
    if ps.is_empty() {
        return (0.0, 0.0, 0.0, 0);
    }
    let n = ps.len() as f32;
    let mut hel = 0.0;
    let mut ens = 0.0;
    let mut q = 0.0;
    for p in ps {
        hel += p.vort * p.speed;
        ens += p.vort * p.vort;
        q += 0.5 * (p.speed * p.speed - p.vort * p.vort);
    }
    (hel / n * 0.1, ens / n, q / n, ps.len())
}

fn now_utc_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Section eyebrow — the single sanctioned caps style (`overline`, §3.1):
/// tracked, Medium weight, tertiary color. Scientific-state tokens use
/// `theme::chip_text`; everything else is sentence case.
fn caps(text: &str) -> RichText {
    overline_text(text)
}
fn mono(text: &str, color: Color32) -> RichText {
    RichText::new(text).monospace().color(color)
}

fn is_research_nav(nav: Nav) -> bool {
    matches!(
        nav,
        Nav::FlowPainter | Nav::Fields2D | Nav::Metrics | Nav::Benchmark
    )
}

fn nav_is_available(nav: Nav, developer_research_sandbox: bool) -> bool {
    !is_research_nav(nav) || developer_research_sandbox
}

impl ReynApp {
    fn is_research_sandbox(&self) -> bool {
        self.settings.developer_research_sandbox && is_research_nav(self.nav)
    }

    fn invalidate_cad_section(&mut self) {
        self.section_tex = None;
        self.section_data = None;
        self.section_sig = u64::MAX;
        self.section_error = None;
    }

    /// Drop only the colormapped texture caches (underlying data stays) so an
    /// appearance-preference change re-renders every field view immediately.
    fn invalidate_field_textures(&mut self) {
        self.invalidate_cad_section();
        self.f2d_tex.clear();
        self.f2d_sig = u64::MAX;
        self.bench_tex.clear();
        self.paint_tex = None;
    }

    fn invalidate_active_case_result(&mut self) {
        self.invalidate_cad_section();
        if let Some(case) = &mut self.cad {
            if case.workflow.result.is_some() {
                case.workflow.parent_run_id = case.active_run_id.clone();
            }
            case.workflow.result = None;
            case.workflow.stage = engineering::CaseStage::Setup;
            case.surf = None;
            case.velocity.clear();
            case.pressure.clear();
            case.cp.clear();
            case.traction.clear();
            case.result_grid = 0;
            case.pending = false;
            case.pending_request_id = None;
            case.pending_run = None;
        }
        self.surface_on = false;
    }

    fn commit_active_case_revision(&mut self) -> Result<(), String> {
        let case = self
            .cad
            .as_ref()
            .ok_or_else(|| "No external-flow case is active.".to_string())?;
        let mut workflow = case.workflow.clone();
        let parent_revision_id = workflow
            .case_revision_id
            .clone()
            .ok_or_else(|| "The active case has no persisted revision.".to_string())?;
        let source_revision_id = workflow
            .source_revision_id
            .clone()
            .ok_or_else(|| "The active case has no source revision.".to_string())?;
        let current_source = self
            .project
            .manifest()
            .source_revisions()
            .iter()
            .find(|source| source.source_revision_id == source_revision_id)
            .cloned()
            .ok_or_else(|| "The active geometry source revision is unavailable.".to_string())?;
        let approved_units = workflow.operating.length_unit.symbol().to_string();
        let approved_frame = "approved STL source frame to fixed-body solver frame".to_string();
        let source_changed = current_source.declared_units.as_deref()
            != Some(approved_units.as_str())
            || current_source.frame.as_deref() != Some(approved_frame.as_str())
            || current_source.transform_4x4 != workflow.preflight.transform_4x4;
        let approved_source = source_changed.then(|| {
            let approved_source_id = format!("source-{}", uuid::Uuid::new_v4());
            workflow.source_revision_id = Some(approved_source_id.clone());
            let mut warnings = current_source.warnings.clone();
            warnings.extend(workflow.preflight.warnings.clone());
            warnings.extend(
                workflow
                    .preflight
                    .waivers
                    .iter()
                    .map(|waiver| format!("APPROVED WAIVER · {waiver}")),
            );
            warnings.sort();
            warnings.dedup();
            project::SourceRevision {
                source_revision_id: approved_source_id,
                source_kind: project::SourceKind::Geometry,
                revision: current_source.revision.saturating_add(1),
                imported_utc_unix: now_utc_unix(),
                uri_hint: current_source.uri_hint.clone(),
                byte_size: current_source.byte_size,
                content_sha256: current_source.content_sha256.clone(),
                declared_units: Some(approved_units),
                frame: Some(approved_frame),
                transform_4x4: workflow.preflight.transform_4x4,
                parent_revision_id: Some(current_source.source_revision_id.clone()),
                warnings,
            }
        });
        let discretization = serde_json::json!({
                "grid": [
                    workflow.preflight.target_grid,
                    workflow.preflight.target_grid,
                    workflow.preflight.target_grid,
                ],
                "solid_voxels": workflow.preflight.solid_voxels,
                "voxel_components": workflow.preflight.voxel_components,
                "minimum_cells_across": workflow.preflight.minimum_cells_across,
                "boundary_clearance_cells": workflow.preflight.boundary_clearance_cells,
                "transform_4x4": workflow.preflight.transform_4x4,
        });
        let outputs = serde_json::json!({
                "velocity": {"source": "model_prediction", "units": "m/s"},
                "pressure": {"source": "recovered", "units": "Pa"},
                "cp": {"source": "derived_from_recovered_pressure", "units": "1"},
                "surface_loads": {
                    "source": "derived",
                    "method": engineering::SURFACE_LOAD_METHOD,
                    "downstream_use": "FEA mapping",
                },
                "field_artifact": {
                    "schema": engineering::ENGINEERING_FIELD_SCHEMA,
                    "layout": ["velocity_x", "velocity_y", "velocity_z", "pressure_pa", "mask", "cp", "traction_x_pa", "traction_y_pa", "traction_z_pa"],
                },
        });
        let active_case = self
            .project
            .manifest()
            .case(&workflow.case_id)
            .cloned()
            .ok_or_else(|| "The active external-flow case is unavailable.".to_string())?;
        workflow.case_revision_id = Some(parent_revision_id.clone());
        let active_contract = workflow.exact_contract();
        let unchanged = approved_source.is_none()
            && active_case.active_revision().contract == active_contract
            && active_case.active_revision().discretization == discretization
            && active_case.active_revision().outputs == outputs;
        if unchanged {
            return Ok(());
        }
        let revision_id = format!("case-revision-{}", uuid::Uuid::new_v4());
        workflow.case_revision_id = Some(revision_id.clone());
        let source_revision_id = workflow
            .source_revision_id
            .clone()
            .expect("approved workflow retains a source revision");
        let revision = project::CaseRevision {
            case_revision_id: revision_id.clone(),
            parent_revision_id: Some(parent_revision_id),
            created_utc_unix: now_utc_unix(),
            source_revision_ids: vec![source_revision_id],
            contract: workflow.exact_contract(),
            discretization,
            outputs,
        };
        let case_id = workflow.case_id.clone();
        self.project
            .transact(now_utc_unix(), move |manifest| {
                if let Some(source) = approved_source {
                    manifest.add_source_revision(source, now_utc_unix())?;
                }
                manifest.append_case_revision(&case_id, revision, now_utc_unix())?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        if let Some(case) = &mut self.cad {
            case.workflow.case_revision_id = Some(revision_id);
            case.workflow.source_revision_id = workflow.source_revision_id;
        }
        Ok(())
    }

    fn run_external_flow(&mut self) {
        if !self.engine_ok {
            self.project_notice = Some((
                "The local engine is unavailable. Stored evidence remains readable, but a new run cannot start."
                    .into(),
                true,
            ));
            return;
        }
        let issues = match self.cad.as_ref() {
            Some(case) => case.workflow.readiness_issues(),
            None => {
                self.import_cad();
                return;
            }
        };
        if !issues.is_empty() {
            self.project_notice = Some((format!("Run blocked: {}", issues.join(" ")), true));
            return;
        }
        if let Err(error) = self.commit_active_case_revision() {
            self.project_notice = Some((format!("Run revision was not persisted: {error}"), true));
            return;
        }
        let Some(case) = &mut self.cad else {
            return;
        };
        let scale = case
            .workflow
            .operating
            .length_unit
            .meters_per_unit()
            .unwrap_or(1.0);
        let reference_length_m = (case.workflow.operating.reference_length * scale) as f32;
        let reynolds = case.workflow.operating.reynolds().unwrap_or_default() as f32;
        case.workflow.stage = engineering::CaseStage::Running;
        case.pending = true;
        case.steps = case.workflow.operating.horizon_steps;
        let request_id = format!("cad-request-{}", uuid::Uuid::new_v4());
        case.pending_request_id = Some(request_id.clone());
        case.pending_run = Some(PendingCadRun {
            request_id: request_id.clone(),
            workflow: case.workflow.clone(),
            started_at: std::time::Instant::now(),
        });
        let request = engine::Cmd::CadPredict {
            request_id,
            model: case.model.clone(),
            steps: case.steps,
            mask: case.mask.clone(),
            reynolds,
            characteristic_length_solver: case.workflow.preflight.solver_characteristic_length
                as f32,
            reference_length_m,
            velocity_mps: case.workflow.operating.velocity as f32,
            density_kg_m3: case.workflow.operating.density as f32,
            reference_pressure_pa: case.workflow.operating.reference_pressure as f32,
        };
        if self.engine.tx.send(request).is_err() {
            case.pending = false;
            case.pending_request_id = None;
            case.pending_run = None;
            case.workflow.stage = engineering::CaseStage::Ready;
            self.project_notice = Some(("Engine request channel is unavailable.".into(), true));
            return;
        }
        self.nav = Nav::Case;
        self.engine_status = format!(
            "● Running {} · Re {:.0} · H{}",
            case.name, reynolds, case.steps
        );
        self.project_notice = Some((
            "Immutable run started from the approved source, transform, operating point, and model revision."
                .into(),
            false,
        ));
    }

    fn persist_external_flow_run(&mut self, field: &engine::CadField) -> Result<String, String> {
        let case = self
            .cad
            .as_ref()
            .ok_or_else(|| "CAD result arrived without an active case".to_string())?;
        let pending = case
            .pending_run
            .as_ref()
            .filter(|pending| pending.request_id == field.request_id)
            .ok_or_else(|| {
                format!(
                    "CAD result request {} does not match the submitted run contract",
                    field.request_id
                )
            })?;
        let workflow = pending.workflow.clone();
        let runtime_ms = pending.started_at.elapsed().as_millis() as u64;
        let case_revision_id = workflow
            .case_revision_id
            .clone()
            .ok_or_else(|| "active case revision missing".to_string())?;
        let run_id = format!("run-{}", uuid::Uuid::new_v4());
        let field_bytes =
            engineering::encode_engineering_field(&engineering::EngineeringFieldBlob {
                n: field.n,
                velocity: field.vel.clone(),
                pressure_pa: field.pressure.clone(),
                mask: field.mask.clone(),
                cp: field.cp.clone(),
                traction_pa: field.traction.clone(),
            })?;
        let field_sha256 = format!("{:x}", Sha256::digest(&field_bytes));
        self.project
            .add_content_with_digest(
                field_bytes,
                "application/vnd.reyn.engineering-field.f32le",
                &field_sha256,
            )
            .map_err(|error| error.to_string())?;
        let result_json = serde_json::json!({
            "schema": engineering::ENGINEERING_RESULT_SCHEMA,
            "field_schema": engineering::ENGINEERING_FIELD_SCHEMA,
            "field_sha256": field_sha256.clone(),
            "submitted_request_id": field.request_id,
            "run_id": run_id,
            "case_revision_id": case_revision_id,
            "method": field.load_method,
            "grid": field.n,
            "horizon": field.horizon,
            "reynolds": field.reynolds,
            "solver_characteristic_length": field.characteristic_length_solver,
            "solver_dt": field.solver_dt,
            "solver_stride": field.solver_stride,
            "warmup_steps": field.warmup_steps,
            "dt_frame": field.dt_frame,
            "cp": {
                "minimum": field.cp.iter().copied().fold(f32::INFINITY, f32::min),
                "maximum": field.cp.iter().copied().fold(f32::NEG_INFINITY, f32::max),
                "source": "derived_from_recovered_pressure",
            },
            "force_coefficients": field.force_coefficients,
            "moment_coefficients": field.moment_coefficients,
            "force_newtons": field.force_newtons,
            "moment_newton_meters": field.moment_newton_meters,
            "moment_reference": "diffuse_surface_area_centroid",
            "surface_area_m2": field.surface_area_m2,
            "pressure_force_fraction": field.pressure_force_fraction,
            "load_hotspot_m": field.load_hotspot,
            "suction_hotspot_m": field.suction_hotspot,
            "divergence_rms": field.divergence_rms,
            "wake_deficit_peak": field.wake_deficit_peak,
            "wake_deficit_mean": field.wake_deficit_mean,
            "warnings": field.warnings,
        });
        let result_bytes =
            serde_json::to_vec_pretty(&result_json).map_err(|error| error.to_string())?;
        let result_sha256 = format!("{:x}", Sha256::digest(&result_bytes));
        self.project
            .add_content_with_digest(
                result_bytes,
                "application/vnd.reyn.engineering-result+json",
                &result_sha256,
            )
            .map_err(|error| error.to_string())?;
        let scalar = |key: &str, value: f32, units: &str| project::ScalarOutput {
            key: key.into(),
            value: value as f64,
            units: units.into(),
            abs_tolerance: 1e-6,
        };
        let scalar_outputs = vec![
            scalar("force_coefficient_x", field.force_coefficients[0], "1"),
            scalar("force_coefficient_y", field.force_coefficients[1], "1"),
            scalar("force_coefficient_z", field.force_coefficients[2], "1"),
            scalar("moment_coefficient_x", field.moment_coefficients[0], "1"),
            scalar("moment_coefficient_y", field.moment_coefficients[1], "1"),
            scalar("moment_coefficient_z", field.moment_coefficients[2], "1"),
            scalar("divergence_rms", field.divergence_rms, "1/s"),
            scalar("wake_deficit_peak", field.wake_deficit_peak, "1"),
            scalar("wake_deficit_mean", field.wake_deficit_mean, "1"),
        ];
        let parent_run_id = workflow
            .parent_run_id
            .clone()
            .or_else(|| case.active_run_id.clone());
        let parent_run = parent_run_id.as_deref().and_then(|parent_id| {
            self.project
                .manifest()
                .cases()
                .iter()
                .find(|record| record.case_id() == workflow.case_id)
                .and_then(|record| {
                    record
                        .runs()
                        .iter()
                        .find(|run| run.run_id() == parent_id)
                        .cloned()
                })
        });
        let mut manifest = project::RunManifest {
            schema_version: project::PROJECT_SCHEMA_VERSION,
            app: project::VersionedComponent {
                name: "Reyn Studio".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                sha256: None,
            },
            engine: Some(project::VersionedComponent {
                name: "reyn_engine.py".into(),
                version: "engineering_result.v1".into(),
                sha256: None,
            }),
            model: Some(project::VersionedComponent {
                name: workflow.model_id.clone(),
                version: "checkpoint".into(),
                sha256: workflow
                    .model_sha256
                    .clone()
                    .filter(|digest| digest.len() == 64),
            }),
            solver: Some(project::VersionedComponent {
                name: "ObstacleFlowSolver3D warmup".into(),
                version: "fixed_body".into(),
                sha256: None,
            }),
            converter: Some(project::VersionedComponent {
                name: engineering::SURFACE_LOAD_METHOD.into(),
                version: "1".into(),
                sha256: None,
            }),
            exact_contract: workflow.exact_contract(),
            exact_settings: serde_json::json!({
                "submitted_request_id": field.request_id,
                "preprocessing_transform": workflow.preflight.transform_4x4,
                "grid": workflow.preflight.target_grid,
                "solver_characteristic_length": field.characteristic_length_solver,
                "solver_dt": field.solver_dt,
                "solver_stride": field.solver_stride,
                "warmup_steps": field.warmup_steps,
                "dt_frame": field.dt_frame,
                "flow_direction": workflow.operating.flow_direction,
                "approved_waivers": workflow.preflight.waivers,
            }),
            seeds: vec![7],
            device: self.settings.compute_device.engine_value().into(),
            runtime_ms,
            stop_reason: "completed".into(),
            warnings: field.warnings.clone(),
            waivers: workflow.preflight.waivers.clone(),
            missing_dependencies: Vec::new(),
            output_sha256: vec![result_sha256.clone(), field_sha256],
            scalar_outputs,
            determinism: None,
        };
        if let Some(parent) = &parent_run {
            manifest.compare_scalars_against(parent);
        }
        let cp_min = field.cp.iter().copied().fold(f32::INFINITY, f32::min);
        let cp_max = field.cp.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let calibrated_views = vec![
            project::CalibratedView {
                view_id: format!("view-{run_id}-cp"),
                quantity: "pressure coefficient Cp".into(),
                units: "1".into(),
                scale_min: cp_min.min(cp_max - 1e-6) as f64,
                scale_max: cp_max.max(cp_min + 1e-6) as f64,
                source_class: project::EvidenceSourceClass::Derived,
                method: "Cp=(p_recovered-p_inf)/q_inf".into(),
            },
            project::CalibratedView {
                view_id: format!("view-{run_id}-velocity"),
                quantity: "velocity".into(),
                units: "m/s".into(),
                scale_min: 0.0,
                scale_max: workflow.operating.velocity.max(1e-6),
                source_class: project::EvidenceSourceClass::ModelPrediction,
                method: "DirectFlowMap fixed-body prediction".into(),
            },
        ];
        let run = project::RunRecord::new(
            run_id.clone(),
            parent_run.as_ref().map(|parent| parent.run_id().to_owned()),
            case_revision_id,
            now_utc_unix(),
            now_utc_unix(),
            project::LifecycleState::Complete,
            manifest,
            calibrated_views.clone(),
        );
        let evidence = project::EvidenceArtifact {
            evidence_id: format!("evidence-{}", uuid::Uuid::new_v4()),
            run_ids: vec![run_id.clone()],
            created_utc_unix: now_utc_unix(),
            source_class: project::EvidenceSourceClass::Derived,
            media_type: "application/vnd.reyn.engineering-result+json".into(),
            byte_size: serde_json::to_vec(&result_json)
                .map_err(|error| error.to_string())?
                .len() as u64,
            content_sha256: result_sha256,
            derivation_method: Some(engineering::SURFACE_LOAD_METHOD.into()),
            derivation_version: Some("1".into()),
            warnings: field.warnings.clone(),
            metadata: result_json,
            calibrated_views,
        };
        let case_id = workflow.case_id;
        self.project
            .transact(now_utc_unix(), |project_manifest| {
                project_manifest.append_run(&case_id, run, now_utc_unix())?;
                project_manifest.append_evidence(evidence, now_utc_unix())?;
                project_manifest.set_selection(
                    project::ProjectSelection {
                        active_case_id: Some(case_id),
                        selected_run_id: Some(run_id.clone()),
                        selected_evidence_id: None,
                        selected_view_id: None,
                    },
                    now_utc_unix(),
                )?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        Ok(run_id)
    }

    fn controls_engineering_case(&mut self, ui: &mut egui::Ui) {
        let model_inventory = self.models.clone();
        let mut waiver_draft = std::mem::take(&mut self.waiver_draft);
        let mut waiver_code = self.waiver_code.take();
        ui.label(title_text("External-Flow Case"));
        ui.label(
            RichText::new("source → preflight → contract → run")
                .size(11.0)
                .color(TEXT_MUTE),
        );
        ui.add_space(16.0);
        let Some(case) = self.cad.as_mut() else {
            card(ui, |ui| {
                ui.label(RichText::new("No geometry imported").color(TEXT));
                ui.label(
                    RichText::new(
                        "Start with an STL. Reyn will preserve the source bytes, hash, transform, and every case revision.",
                    )
                    .text_style(caption())
                    .color(TEXT_MUTE),
                );
            });
            ui.add_space(10.0);
            if ui.button("Import Geometry…").clicked() {
                self.import_cad();
            }
            return;
        };
        if case.pending {
            ui.disable();
        }
        let mut changed = false;
        card(ui, |ui| {
            ui.label(caps("Source & transform"));
            diag(ui, "File", &case.workflow.source_name, TEXT);
            diag(
                ui,
                "SHA-256",
                &short_hash(&case.workflow.preflight.source_sha256),
                TEXT_MUTE,
            );
            diag(
                ui,
                "Triangles",
                &case.workflow.preflight.triangles.to_string(),
                TEXT_DIM,
            );
            // Bounding box in the declared units (SI echo when not meters);
            // before units are declared, it is honestly labeled "source units".
            let extents = case.workflow.preflight.source_extents;
            let unit = case.workflow.operating.length_unit;
            let extents_text = match unit.meters_per_unit() {
                Some(scale) => {
                    let si = if (scale - 1.0).abs() > 1e-12 {
                        format!(
                            "  ({:.4} × {:.4} × {:.4} m)",
                            extents[0] * scale,
                            extents[1] * scale,
                            extents[2] * scale
                        )
                    } else {
                        String::new()
                    };
                    format!(
                        "{:.4} × {:.4} × {:.4} {}{si}",
                        extents[0],
                        extents[1],
                        extents[2],
                        unit.symbol()
                    )
                }
                None => format!(
                    "{:.4} × {:.4} × {:.4} source units — declare units below",
                    extents[0], extents[1], extents[2]
                ),
            };
            diag(
                ui,
                "Bounding box",
                &extents_text,
                if unit.meters_per_unit().is_some() {
                    TEXT_DIM
                } else {
                    WARN
                },
            );
            diag(
                ui,
                "Surface components",
                &case.workflow.preflight.components.to_string(),
                TEXT_DIM,
            );
            // Watertightness verdict in words (never color-only).
            let watertight = case.workflow.preflight.boundary_edges == 0
                && case.workflow.preflight.non_manifold_edges == 0;
            diag(
                ui,
                "Watertight",
                if watertight {
                    "yes · closed manifold surface"
                } else {
                    "no · open or non-manifold edges"
                },
                if watertight { SUCCESS } else { WARN },
            );
            diag(
                ui,
                "Defects",
                &format!(
                    "{} degenerate · {} open · {} non-manifold",
                    case.workflow.preflight.degenerate_triangles,
                    case.workflow.preflight.boundary_edges,
                    case.workflow.preflight.non_manifold_edges
                ),
                if case.workflow.preflight.degenerate_triangles == 0 && watertight {
                    SUCCESS
                } else {
                    WARN
                },
            );
            diag(
                ui,
                "Grid",
                &format!("{}³", case.workflow.preflight.target_grid),
                BRAND,
            );
            diag(
                ui,
                "Proposed scale",
                &format!("{:.6}", case.workflow.preflight.proposed_scale),
                TEXT_DIM,
            );
            diag(
                ui,
                "Solver characteristic length",
                &format!(
                    "{:.6} solver units",
                    case.workflow.preflight.solver_characteristic_length
                ),
                TEXT_DIM,
            );
            diag(
                ui,
                "Voxel adequacy",
                &format!(
                    "{} solid · {} cells thick · {} cells clear",
                    case.workflow.preflight.solid_voxels,
                    case.workflow.preflight.minimum_cells_across,
                    case.workflow.preflight.boundary_clearance_cells
                ),
                TEXT_DIM,
            );
            ui.collapsing("Preprocessing transform 4×4", |ui| {
                for row in 0..4 {
                    ui.label(
                        RichText::new(format!(
                            "{:>10.5} {:>10.5} {:>10.5} {:>10.5}",
                            case.workflow.preflight.transform_4x4[row],
                            case.workflow.preflight.transform_4x4[4 + row],
                            case.workflow.preflight.transform_4x4[8 + row],
                            case.workflow.preflight.transform_4x4[12 + row],
                        ))
                        .text_style(mono_s())
                        .color(TEXT_MUTE),
                    );
                }
            });
            ui.add_space(8.0);
            egui::ComboBox::from_id_salt("engineering.length-unit")
                .selected_text(case.workflow.operating.length_unit.label())
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for unit in engineering::LengthUnit::ALL {
                        changed |= ui
                            .selectable_value(
                                &mut case.workflow.operating.length_unit,
                                unit,
                                unit.label(),
                            )
                            .changed();
                    }
                });
            changed |= ui
                .checkbox(
                    &mut case.workflow.preflight.transform_approved,
                    "Approve units, orientation, scale, and solver placement",
                )
                .changed();
            for warning in &case.workflow.preflight.warnings {
                ui.label(
                    RichText::new(format!("NOTICE · {warning}"))
                        .text_style(caption())
                        .color(WARN),
                );
            }
        });
        ui.add_space(10.0);
        card(ui, |ui| {
            ui.label(caps("Operating point"));
            ui.label(
                RichText::new("Qualified model")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
            egui::ComboBox::from_id_salt("engineering.model")
                .selected_text(&case.workflow.model_id)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for model in model_inventory.iter().filter(|model| {
                        model.dimension == 3
                            && model.grid as usize == case.workflow.preflight.target_grid
                            && model.in_channels > model.out_channels
                            && model.out_channels == 3
                            && model.scenario == "obstacle"
                    }) {
                        if ui
                            .selectable_value(
                                &mut case.workflow.model_id,
                                model.id.clone(),
                                &model.name,
                            )
                            .changed()
                        {
                            case.model = model.id.clone();
                            case.workflow.model_sha256 = Some(model.checkpoint_sha256.clone());
                            case.workflow.model_max_steps = model.max_steps;
                            case.workflow.model_support = engineering::ModelSupport {
                                status: model.status.clone(),
                                dimension: model.dimension,
                                grid: model.grid,
                                input_channels: model.in_channels,
                                output_channels: model.out_channels,
                                scenario: model.scenario.clone(),
                                physics_contract: model.physics_contract.clone(),
                            };
                            changed = true;
                        }
                    }
                });
            diag(
                ui,
                "Model grid",
                &format!("{}³", case.workflow.model_support.grid),
                TEXT_DIM,
            );
            diag(
                ui,
                "Channels",
                &format!(
                    "{} → {}",
                    case.workflow.model_support.input_channels,
                    case.workflow.model_support.output_channels
                ),
                TEXT_DIM,
            );
            diag(
                ui,
                "Scenario",
                &case.workflow.model_support.scenario,
                TEXT_DIM,
            );
            diag(
                ui,
                "Physics",
                &case.workflow.model_support.physics_contract,
                TEXT_DIM,
            );
            diag(
                ui,
                "Horizon support",
                &format!("1–{} steps", case.workflow.model_max_steps),
                TEXT_DIM,
            );
            ui.separator();
            diag(ui, "Flow direction", "+X · fixed-body contract", TEXT_DIM);
            ui.label(
                RichText::new("Reference length")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
            changed |= ui
                .add(
                    egui::DragValue::new(&mut case.workflow.operating.reference_length)
                        .speed(0.01)
                        .range(1e-9..=1e9)
                        .suffix(format!(" {}", case.workflow.operating.length_unit.symbol())),
                )
                .changed();
            // Preflight suggestion: the largest cross-flow extent (the frontal
            // dimensions for a +X free stream), stated with its rationale.
            let suggested_reference = case.workflow.preflight.source_extents[1]
                .max(case.workflow.preflight.source_extents[2]);
            if suggested_reference > 0.0
                && (case.workflow.operating.reference_length - suggested_reference).abs()
                    > 1e-12
            {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "suggested {suggested_reference:.4} {} · max cross-flow extent",
                            case.workflow.operating.length_unit.symbol()
                        ))
                        .text_style(caption())
                        .color(TEXT_MUTE),
                    );
                    if ui.small_button("Use").clicked() {
                        case.workflow.operating.reference_length = suggested_reference;
                        changed = true;
                    }
                });
            }
            ui.label(
                RichText::new("Operating preset")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
            egui::ComboBox::from_id_salt("engineering.preset")
                .selected_text("Apply preset…")
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    let saved_presets = self.settings.operating_presets.clone();
                    for preset in settings::built_in_presets()
                        .iter()
                        .chain(saved_presets.iter())
                    {
                        if ui.selectable_label(false, &preset.name).clicked() {
                            case.workflow.operating.velocity = preset.velocity_mps;
                            case.workflow.operating.density = preset.density_kg_m3;
                            case.workflow.operating.viscosity = preset.viscosity_pa_s;
                            case.workflow.operating.reference_pressure =
                                preset.reference_pressure_pa;
                            changed = true;
                        }
                    }
                });
            ui.label(
                RichText::new("Free-stream speed")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
            changed |= unit_value_input(
                ui,
                "engineering.unit.velocity",
                &mut case.workflow.operating.velocity,
                &mut self.input_units.velocity,
                0.1,
                1e-6..=1e5,
            );
            ui.label(
                RichText::new("Density")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
            changed |= unit_value_input(
                ui,
                "engineering.unit.density",
                &mut case.workflow.operating.density,
                &mut self.input_units.density,
                0.001,
                1e-9..=1e5,
            );
            ui.label(
                RichText::new("Dynamic viscosity")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
            changed |= unit_value_input(
                ui,
                "engineering.unit.viscosity",
                &mut case.workflow.operating.viscosity,
                &mut self.input_units.viscosity,
                1e-6,
                1e-12..=1e3,
            );
            ui.label(
                RichText::new("Reference pressure")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
            changed |= unit_value_input(
                ui,
                "engineering.unit.pressure",
                &mut case.workflow.operating.reference_pressure,
                &mut self.input_units.pressure,
                10.0,
                0.0..=1e9,
            );
            ui.label(
                RichText::new("Prediction horizon")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
            changed |= ui
                .add(
                    egui::Slider::new(
                        &mut case.workflow.operating.horizon_steps,
                        1..=case.workflow.model_max_steps.max(1),
                    )
                    .suffix(" steps"),
                )
                .changed();
            let reynolds = case.workflow.operating.reynolds();
            diag(
                ui,
                "Reynolds number",
                &reynolds
                    .map(|value| format!("{value:.1}"))
                    .unwrap_or_else(|| "incomplete".into()),
                if reynolds.is_some_and(|value| (60.0..=400.0).contains(&value)) {
                    SUCCESS
                } else {
                    WARN
                },
            );
            diag(
                ui,
                "Dynamic pressure",
                &case
                    .workflow
                    .operating
                    .dynamic_pressure()
                    .map(|value| {
                        units::format_quantity(
                            units::Quantity::Pressure,
                            value,
                            self.settings.unit_system,
                            self.settings.value_format(),
                        )
                    })
                    .unwrap_or_else(|| "incomplete".into()),
                TEXT_DIM,
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.preset_name_draft)
                        .hint_text("Preset name")
                        .desired_width((ui.available_width() - 100.0).max(80.0)),
                );
                let named = !self.preset_name_draft.trim().is_empty();
                if ui
                    .add_enabled(named, egui::Button::new("Save preset"))
                    .on_hover_text("Save this fluid state and speed as a named preset (Settings › Workflow)")
                    .clicked()
                {
                    let name = self.preset_name_draft.trim().to_string();
                    let preset = settings::OperatingPointPreset {
                        name: name.clone(),
                        velocity_mps: case.workflow.operating.velocity,
                        density_kg_m3: case.workflow.operating.density,
                        viscosity_pa_s: case.workflow.operating.viscosity,
                        reference_pressure_pa: case.workflow.operating.reference_pressure,
                    };
                    self.settings
                        .operating_presets
                        .retain(|existing| existing.name != name);
                    self.settings.operating_presets.push(preset);
                    self.settings_draft.operating_presets =
                        self.settings.operating_presets.clone();
                    self.preset_notice = Some(match self.settings.save() {
                        Ok(_) => (format!("Preset \u{201c}{name}\u{201d} saved."), false),
                        Err(error) => (format!("Preset was not saved: {error}"), true),
                    });
                    self.preset_name_draft.clear();
                }
            });
            if let Some((message, is_error)) = &self.preset_notice {
                ui.label(
                    RichText::new(message)
                        .text_style(caption())
                        .color(if *is_error { WARN } else { SUCCESS }),
                );
            }
        });
        ui.add_space(10.0);
        let issues = case.workflow.readiness_issues();
        if issues.is_empty() && case.workflow.model_support.status == "clean" {
            case.workflow.stage = engineering::CaseStage::Ready;
            ui.label(
                RichText::new("READY · CONTRACT WITHIN QUALIFIED ENVELOPE")
                    .strong()
                    .text_style(mono_s())
                    .color(SUCCESS),
            );
        } else if issues.is_empty() {
            case.workflow.stage = engineering::CaseStage::Ready;
            ui.label(
                RichText::new("READY WITH MODEL METADATA REVIEW")
                    .strong()
                    .text_style(mono_s())
                    .color(WARN),
            );
            ui.label(
                RichText::new(
                    "The executable channel/grid contract is compatible, but checkpoint provenance is not fully CLEAN.",
                )
                .text_style(caption())
                .color(TEXT_MUTE),
            );
        } else {
            ui.label(
                RichText::new(format!("{} BLOCKER(S)", issues.len()))
                    .strong()
                    .text_style(mono_s())
                    .color(WARN),
            );
            for issue in case
                .workflow
                .preflight
                .support_issues()
                .into_iter()
                .filter(|issue| issues.contains(&issue.message))
                .take(6)
            {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("• {} · {}", issue.code, issue.message))
                            .text_style(caption())
                            .color(TEXT_MUTE),
                    );
                    if issue.waivable && ui.small_button("Name waiver…").clicked() {
                        waiver_code = Some(issue.code.into());
                    }
                });
            }
            if waiver_code.is_some() || !waiver_draft.is_empty() {
                ui.add(
                    egui::TextEdit::singleline(&mut waiver_draft)
                        .hint_text("Specific engineering rationale (8+ characters)"),
                );
                if let Some(code) = waiver_code.as_deref() {
                    if ui.button(format!("Apply waiver for {code}")).clicked() {
                        match case.workflow.preflight.record_waiver(code, &waiver_draft) {
                            Ok(()) => {
                                waiver_draft.clear();
                                waiver_code = None;
                                changed = true;
                            }
                            Err(error) => {
                                self.project_notice = Some((error, true));
                            }
                        }
                    }
                }
            }
        }
        ui.add_space(10.0);
        let running = case.pending;
        let ready = case.workflow.ready() && !running;
        if ui
            .add_enabled(
                ready,
                egui::Button::new(
                    RichText::new(if running {
                        "Running…"
                    } else {
                        "Run qualified analysis"
                    })
                    .color(ON_EMBER),
                )
                .fill(EMBER),
            )
            .clicked()
        {
            self.run_external_flow();
        }
        if changed {
            self.invalidate_active_case_result();
            self.project_notice = Some((
                "Case contract changed. Completed runs remain immutable; the draft requires a new run."
                    .into(),
                false,
            ));
        }
        self.waiver_draft = waiver_draft;
        self.waiver_code = waiver_code;
    }

    fn controls_engineering_results(&mut self, ui: &mut egui::Ui) {
        let comparison = self.cad.as_ref().and_then(|active| {
            let case = self
                .project
                .manifest()
                .cases()
                .iter()
                .find(|case| case.case_id() == active.workflow.case_id)?;
            let current = active
                .active_run_id
                .as_deref()
                .and_then(|run_id| case.runs().iter().find(|run| run.run_id() == run_id))?;
            let parent_id = current.parent_run_id()?;
            let parent = case.runs().iter().find(|run| run.run_id() == parent_id)?;
            let parent_scalars: std::collections::BTreeMap<_, _> = parent
                .manifest()
                .scalar_outputs
                .iter()
                .map(|scalar| (scalar.key.clone(), scalar.clone()))
                .collect();
            let rows = current
                .manifest()
                .scalar_outputs
                .iter()
                .filter_map(|scalar| {
                    parent_scalars.get(&scalar.key).and_then(|prior| {
                        (prior.units == scalar.units).then(|| {
                            (
                                scalar.key.clone(),
                                prior.value,
                                scalar.value,
                                scalar.units.clone(),
                            )
                        })
                    })
                })
                .collect::<Vec<_>>();
            Some((
                parent.run_id().to_owned(),
                current.run_id().to_owned(),
                rows,
            ))
        });
        // R9: designed empty states — a level-1 card with one action,
        // matching the library's empty-state grammar.
        let mut go_to_case = false;
        if self.cad.is_none() {
            card(ui, |ui| {
                ui.label(title_text("No engineering result"));
                ui.label(
                    RichText::new("Import geometry and qualify a case to produce loads.")
                        .text_style(caption())
                        .color(TEXT_MUTE),
                );
                ui.add_space(8.0);
                go_to_case = ui.button("Go to Case Setup").clicked();
            });
            if go_to_case {
                self.nav = Nav::Case;
            }
            return;
        }
        let case = self.cad.as_ref().expect("checked above");
        if case.workflow.result.is_none() {
            card(ui, |ui| {
                ui.label(title_text("No current result"));
                ui.label(
                    RichText::new("Complete the approved case workflow first.")
                        .text_style(caption())
                        .color(TEXT_MUTE),
                );
                ui.add_space(8.0);
                go_to_case = ui.button("Go to Case Setup").clicked();
            });
            if go_to_case {
                self.nav = Nav::Case;
            }
            return;
        }
        let result = case.workflow.result.as_ref().expect("checked above");
        let can_export_fea = case.active_run_id.is_some();
        let result_grid = case.result_grid;
        ui.label(title_text("Engineering Results"));
        ui.label(
            RichText::new("applicability → loads → geometry-linked evidence")
                .text_style(caption())
                .color(TEXT_MUTE),
        );
        ui.add_space(14.0);
        // §4.4 decision order: applicability verdict first, then the numbers.
        let support_clean = case.workflow.model_support.status == "clean";
        card(ui, |ui| {
            alert_line(
                ui,
                if support_clean { OK } else { WARN },
                if support_clean { "✓" } else { "!" },
                if support_clean {
                    "Supported fixed-body contract · model-derived loads"
                } else {
                    "Supported contract · model metadata review — provenance incomplete, preserved in evidence"
                },
            );
        });
        ui.add_space(12.0);
        // Measurement table (§4.4): label body / value mono right-aligned /
        // shared unit column / source class chip on every row (N5X-EV-02).
        let unit_system = self.settings.unit_system;
        let value_format = self.settings.value_format();
        let mut copied_summary = false;
        card(ui, |ui| {
            ui.label(caps("Loads & derived quantities"));
            ui.add_space(8.0);
            let fmt = |value: f64| units::format_value(value, value_format);
            let vector = |si: [f64; 3], quantity: units::Quantity| -> (String, &'static str) {
                let symbol = units::display_value(quantity, 0.0, unit_system).1;
                let component =
                    |value: f64| fmt(units::display_value(quantity, value, unit_system).0);
                (
                    format!(
                        "[{}, {}, {}]",
                        component(si[0]),
                        component(si[1]),
                        component(si[2])
                    ),
                    symbol,
                )
            };
            // Named force coefficients (the fixed-body contract puts the free
            // stream on +X, so Cx is the drag coefficient).
            measure_row(
                ui,
                "Cd · drag (+X)",
                &fmt(result.force_coefficients[0]),
                "–",
                "MODEL",
                BRAND,
            )
            .on_hover_text("Streamwise force coefficient; +X is the free-stream direction.");
            measure_row(
                ui,
                "Cs · side (+Y)",
                &fmt(result.force_coefficients[1]),
                "–",
                "MODEL",
                BRAND,
            );
            measure_row(
                ui,
                "Cl · vertical (+Z)",
                &fmt(result.force_coefficients[2]),
                "–",
                "MODEL",
                BRAND,
            );
            let (force_text, force_unit) = vector(result.force_newtons, units::Quantity::Force);
            measure_row(ui, "Fluid force", &force_text, force_unit, "MODEL", BRAND);
            measure_row(
                ui,
                "Moment coefficients",
                &format!(
                    "[{}, {}, {}]",
                    fmt(result.moment_coefficients[0]),
                    fmt(result.moment_coefficients[1]),
                    fmt(result.moment_coefficients[2])
                ),
                "–",
                "MODEL",
                BRAND,
            );
            let (moment_text, moment_unit) =
                vector(result.moment_newton_meters, units::Quantity::Moment);
            measure_row(
                ui,
                "Fluid moment · surface centroid",
                &moment_text,
                moment_unit,
                "MODEL",
                BRAND,
            );
            // Cp keeps its recovered-pressure honesty (N5X-PHYS-01): the
            // nondimensionalization note sits on hover, one disclosure away.
            measure_row(
                ui,
                "Cp range",
                &format!("{} … {}", fmt(result.cp_min), fmt(result.cp_max)),
                "–",
                "RECOVERED",
                GOLD,
            )
            .on_hover_text(
                "Derived from recovered pressure, nondimensionalized by ½·ρ·U². \
                 Recovered fields are reconstructed, not directly solved.",
            );
            let (area_value, area_unit) =
                units::display_value(units::Quantity::Area, result.surface_area_m2, unit_system);
            measure_row(
                ui,
                "Diffuse surface area",
                &fmt(area_value),
                area_unit,
                "MODEL",
                TEXT_DIM,
            );
            measure_row(
                ui,
                "Pressure share · component norms",
                &format!("{:.1}", result.pressure_force_fraction * 100.0),
                "%",
                "MODEL",
                TEXT_DIM,
            );
            measure_row(
                ui,
                "Divergence RMS",
                &format!("{:.3e}", result.divergence_rms),
                "–",
                "MODEL",
                TEXT_DIM,
            );
            measure_row(
                ui,
                "Wake deficit · peak / mean",
                &format!(
                    "{} / {}",
                    fmt(result.wake_deficit_peak),
                    fmt(result.wake_deficit_mean)
                ),
                "–",
                "MODEL",
                TEXT_DIM,
            );
        });
        ui.add_space(8.0);
        // Reference values the coefficients were scaled with — visible next
        // to the numbers, in the display unit system.
        card(ui, |ui| {
            ui.label(caps("Reference values"));
            ui.add_space(6.0);
            let operating = &case.workflow.operating;
            let reference = |quantity: units::Quantity, si: f64| {
                units::format_quantity(quantity, si, unit_system, value_format)
            };
            diag(
                ui,
                "V∞",
                &reference(units::Quantity::Velocity, operating.velocity),
                TEXT_DIM,
            );
            diag(
                ui,
                "ρ∞",
                &reference(units::Quantity::Density, operating.density),
                TEXT_DIM,
            );
            diag(
                ui,
                "q∞",
                &operating
                    .dynamic_pressure()
                    .map(|value| reference(units::Quantity::Pressure, value))
                    .unwrap_or_else(|| "incomplete".into()),
                TEXT_DIM,
            );
            diag(
                ui,
                "L reference",
                &format!(
                    "{} {}",
                    units::format_value(operating.reference_length, value_format),
                    operating.length_unit.symbol()
                ),
                TEXT_DIM,
            );
            diag(
                ui,
                "Reynolds",
                &operating
                    .reynolds()
                    .map(|value| units::format_value(value, value_format))
                    .unwrap_or_else(|| "incomplete".into()),
                TEXT_DIM,
            );
            ui.add_space(6.0);
            if ui
                .button(format!("{} Copy summary", ph::COPY))
                .on_hover_text(
                    "Copies the full results table (coefficients, loads, reference values, \
                     provenance ids) as tab-separated text for spreadsheets and notes.",
                )
                .clicked()
            {
                let text = results_summary_tsv(
                    &case.workflow,
                    case.active_run_id.as_deref(),
                    unit_system,
                    value_format,
                );
                ui.ctx().copy_text(text);
                copied_summary = true;
            }
        });
        if copied_summary {
            self.project_notice =
                Some(("Results summary copied to the clipboard.".into(), false));
        }
        if let Some((parent_run_id, current_run_id, rows)) = comparison {
            ui.add_space(12.0);
            card(ui, |ui| {
                ui.label(caps("Variant comparison · shared units"));
                ui.label(
                    RichText::new(format!(
                        "{} → {}",
                        short_id(&parent_run_id),
                        short_id(&current_run_id)
                    ))
                    .text_style(mono_s())
                    .color(TEXT_MUTE),
                );
                for (key, parent, current, units) in rows.iter().take(7) {
                    diag(
                        ui,
                        key,
                        &format!(
                            "{parent:.5e} → {current:.5e}  Δ {:+.3e} {units}",
                            current - parent
                        ),
                        if (current - parent).abs() <= 1e-6 {
                            TEXT_DIM
                        } else {
                            GOLD
                        },
                    );
                }
                ui.horizontal(|ui| {
                    if ui.button("Inspect parent evidence").clicked() {
                        self.select_external_run(&parent_run_id);
                    }
                    if ui.button("Inspect current evidence").clicked() {
                        self.select_external_run(&current_run_id);
                    }
                });
            });
        }
        ui.add_space(12.0);
        if self.volumetric {
            // Viewport controls collapse into an inspector accordion (§4.4)
            // so numbers stay above pictures without scrolling.
            inspector_group(ui, "results-clip", "Geometry clipping planes", true, |ui| {
                for (axis, label) in ["X", "Y", "Z"].iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.slice[axis], *label);
                        ui.add(
                            egui::Slider::new(&mut self.slice_pos[axis], 0.0..=1.0)
                                .show_value(true)
                                .trailing_fill(true),
                        );
                    });
                }
            });
            inspector_group(ui, "results-layers", "Layers", true, |ui| {
                ui.checkbox(&mut self.surface_on, "Cp surface");
                ui.checkbox(&mut self.render_volume, "Velocity / vorticity volume");
                ui.checkbox(&mut self.insights_on, "Load and suction hotspots");
            });
        } else {
            ui.label(caps("2D section quantity"));
            egui::ComboBox::from_id_salt("engineering_section_quantity")
                .width(ui.available_width())
                .selected_text(self.section_quantity.label())
                .show_ui(ui, |ui| {
                    for quantity in engineering_section::SectionQuantity::ALL {
                        ui.selectable_value(&mut self.section_quantity, quantity, quantity.label());
                    }
                });
            ui.add_space(10.0);
            ui.label(caps("Section plane"));
            // Concentric radii: thumb r-1 = 4 inside container r-2 = 6 (§3.5).
            Frame::NONE
                .fill(SURFACE)
                .corner_radius(CornerRadius::same(R2))
                .stroke(Stroke::new(1.0, HAIRLINE))
                .inner_margin(2)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for axis in engineering_section::SectionAxis::ALL {
                            if seg(ui, axis.label(), self.section_axis == axis) {
                                self.section_axis = axis;
                            }
                        }
                    });
                });
            let axis_index = self.section_axis.id() as usize;
            ui.add(
                egui::Slider::new(&mut self.slice_pos[axis_index], 0.0..=1.0)
                    .show_value(false)
                    .trailing_fill(true),
            );
            let section_index =
                engineering_section::section_index(result_grid.max(3), self.slice_pos[axis_index])
                    .unwrap_or_default();
            ui.label(
                mono(
                    &format!(
                        "{} = {:.3} domain · cell {}/{}",
                        self.section_axis.label(),
                        self.slice_pos[axis_index],
                        section_index,
                        result_grid.saturating_sub(1)
                    ),
                    TEXT_DIM,
                )
                .text_style(caption()),
            );
            ui.add_space(10.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} · {}",
                        self.section_quantity.label(),
                        self.section_quantity.units()
                    ))
                    .strong()
                    .color(TEXT),
                );
                ui.label(
                    RichText::new(self.section_quantity.source())
                        .text_style(mono_s())
                        .color(GOLD),
                );
                ui.label(
                    RichText::new(self.section_quantity.method())
                        .text_style(caption())
                        .color(TEXT_DIM),
                );
                ui.label(
                    RichText::new(format!(
                        "VIEW +{} · +{} right · +{} up",
                        self.section_axis.label(),
                        self.section_axis.horizontal_axis(),
                        self.section_axis.vertical_axis()
                    ))
                    .text_style(mono_s())
                    .color(TEXT_MUTE),
                );
                ui.label(
                    RichText::new("GEOMETRY · stored diffuse CAD mask")
                        .text_style(mono_s())
                        .color(BRAND),
                );
            });
        }
        ui.add_space(8.0);
        // WARN is the status hue for missing capability; GOLD stays on data.
        alert_line(
            ui,
            WARN,
            "!",
            "Spatial error unavailable — attach an exact solver reference before displaying error.",
        );
        // Voxel diagnostics moved here from global nav (§4.1): they describe
        // the rendered model velocity field, so they live beside it, with an
        // explicit source class (SCI-AC-01).
        ui.add_space(8.0);
        inspector_group(ui, "results-voxel", "Voxel diagnostics", false, |ui| {
            ui.label(chip_text("MODEL · rendered velocity field").color(BRAND));
            ui.add_space(8.0);
            let (helicity, enstrophy, q_criterion, voxel_count) = diagnostics(&self.particles);
            diag(ui, "Helicity", &format!("{:.1e}", helicity), BRAND);
            diag(ui, "Enstrophy Vol.", &format!("{:.2e}", enstrophy), BRAND);
            diag(ui, "Q-Criterion", &format!("{:.2}", q_criterion), GOLD);
            diag(
                ui,
                "Voxel Count",
                &format!("{:.1}K", voxel_count as f32 / 1000.0),
                TEXT,
            );
        });
        ui.add_space(12.0);
        if ui.button("Create Operating-Point Variant").clicked() {
            self.invalidate_active_case_result();
            self.nav = Nav::Case;
        }
        // §4.4: one quiet export entry point; the disabled item explains
        // itself (UX-AC-01) instead of disappearing.
        let mut export_fea = false;
        let mut export_report = false;
        let mut export_section = false;
        let mut export_viewport = false;
        let has_section = !self.volumetric && self.section_data.is_some();
        ui.menu_button("Export…", |ui| {
            if ui
                .add_enabled(
                    can_export_fea,
                    egui::Button::new("Surface loads for FEA (CSV)…"),
                )
                .on_disabled_hover_text("A durable immutable run is required for provenance.")
                .clicked()
            {
                export_fea = true;
                ui.close();
            }
            if ui
                .add_enabled(
                    can_export_fea,
                    egui::Button::new("Engineering report (HTML)…"),
                )
                .on_disabled_hover_text("A durable immutable run is required for provenance.")
                .on_hover_text(
                    "Self-contained HTML: provenance chain, operating point, preflight, \
                     coefficients, section figure, and the limitations block.",
                )
                .clicked()
            {
                export_report = true;
                ui.close();
            }
            if ui
                .add_enabled(has_section, egui::Button::new("Section view (PNG)…"))
                .on_disabled_hover_text("Open a 2D section in Results first.")
                .clicked()
            {
                export_section = true;
                ui.close();
            }
            if ui
                .add_enabled(self.volumetric, egui::Button::new("3D viewport (PNG)…"))
                .on_disabled_hover_text("Switch to the 3D view to capture the viewport.")
                .clicked()
            {
                export_viewport = true;
                ui.close();
            }
        });
        if export_fea {
            self.export_fea_loads();
        }
        if export_report {
            self.export_engineering_report();
        }
        if export_section {
            self.export_section_png();
        }
        if export_viewport {
            self.request_viewport_png(ui.ctx());
        }
        if ui.button("Open Evidence & Provenance").clicked() {
            self.nav = Nav::Evidence;
        }
    }

    fn select_external_run(&mut self, run_id: &str) {
        let Some(case_id) = self
            .project
            .manifest()
            .cases()
            .iter()
            .find(|case| case.runs().iter().any(|run| run.run_id() == run_id))
            .map(|case| case.case_id().to_owned())
        else {
            self.project_notice = Some(("The selected run no longer exists.".into(), true));
            return;
        };
        let selected_evidence_id = self
            .project
            .manifest()
            .evidence_for_run(run_id)
            .into_iter()
            .rev()
            .find(|artifact| {
                artifact
                    .metadata
                    .get("schema")
                    .and_then(serde_json::Value::as_str)
                    == Some(engineering::ENGINEERING_RESULT_SCHEMA)
            })
            .map(|artifact| artifact.evidence_id.clone());
        let run_id_owned = run_id.to_owned();
        let selection = project::ProjectSelection {
            active_case_id: Some(case_id),
            selected_run_id: Some(run_id_owned),
            selected_evidence_id,
            selected_view_id: None,
        };
        match self.project.transact(now_utc_unix(), |manifest| {
            manifest.set_selection(selection, now_utc_unix())
        }) {
            Ok(()) => self.hydrate_project_runtime(),
            Err(error) => {
                self.project_notice = Some((
                    format!("Evidence selection was not persisted: {error}"),
                    true,
                ));
            }
        }
    }

    fn controls_engineering_evidence(&mut self, ui: &mut egui::Ui) {
        ui.label(title_text("Evidence"));
        ui.label(
            RichText::new("exact inputs, lineage, warnings, and exports")
                .text_style(caption())
                .color(TEXT_MUTE),
        );
        ui.add_space(14.0);
        if let Some(case) = &self.cad {
            card(ui, |ui| {
                diag(
                    ui,
                    "Source",
                    &short_hash(&case.workflow.preflight.source_sha256),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Case revision",
                    &case
                        .workflow
                        .case_revision_id
                        .as_deref()
                        .map(short_id)
                        .unwrap_or_else(|| "unavailable".into()),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Run",
                    &case
                        .active_run_id
                        .as_deref()
                        .map(short_id)
                        .unwrap_or_else(|| "not run".into()),
                    BRAND,
                );
                diag(ui, "Model", &case.workflow.model_id, TEXT);
                diag(
                    ui,
                    "Load method",
                    engineering::SURFACE_LOAD_METHOD,
                    TEXT_DIM,
                );
            });
            ui.add_space(10.0);
            if ui
                .add_enabled(
                    case.active_run_id.is_some(),
                    egui::Button::new("Export surface loads for FEA…"),
                )
                .on_disabled_hover_text("A durable immutable run is required for provenance.")
                .clicked()
            {
                self.export_fea_loads();
            }
            if ui.button("Save project").clicked() {
                self.save_project_dialog();
            }
        } else {
            ui.label(RichText::new("No case evidence is active.").color(TEXT_MUTE));
        }
    }

    fn engineering_case_view(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // One shared content column (§3.2, QA G3/G4): symmetric gutters
            // with the 34px minimum and the capped — never expanded — width.
            content_column(ui, CONTENT_MAX_WIDTH, |ui| {
                ui.add_space(28.0);
                    if self.cad.is_none() {
                        ui.add_space(30.0);
                        ui.label(
                            display_text("Start an engineering case"),
                        );
                        ui.label(
                            RichText::new(
                                "Import a fixed-body STL to create a source-aware, revisioned external-flow analysis.",
                            )
                            .size(13.0)
                            .color(TEXT_MUTE),
                        );
                        ui.add_space(20.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Import Geometry…").color(ON_EMBER),
                                )
                                .fill(EMBER)
                                .min_size(Vec2::new(190.0, 42.0)),
                            )
                            .clicked()
                        {
                            self.import_cad();
                        }
                        ui.add_space(24.0);
                        internal_flow_reference_card(ui);
                        return;
                    }
                    let case = self.cad.as_ref().expect("checked above");
                    ui.label(
                        display_text(&case.workflow.name),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "{} · source revision {} · case revision {}",
                            case.workflow.source_name,
                            case.workflow
                                .source_revision_id
                                .as_deref()
                                .map(short_id)
                                .unwrap_or_else(|| "unknown".into()),
                            case.workflow
                                .case_revision_id
                                .as_deref()
                                .map(short_id)
                                .unwrap_or_else(|| "unknown".into())
                        ))
                        .text_style(mono_s())
                        .color(TEXT_MUTE),
                    );
                    ui.add_space(24.0);
                    self.case_stage_spine(ui);
                    ui.add_space(28.0);
                });
        });
    }

    /// §4.3: the guided vertical stage spine — Source → Preflight → Contract
    /// → Operating point → Run. Each stage carries a verdict-first body
    /// (named gate result, then a facts table), the current stage opens by
    /// default, and nothing is hidden that a gate needs to explain.
    fn case_stage_spine(&self, ui: &mut egui::Ui) {
        let Some(case) = self.cad.as_ref() else {
            return;
        };
        let workflow = &case.workflow;
        let preflight = &workflow.preflight;
        let stage_index = workflow.stage.progress_index();
        let spine_units = self.settings.unit_system;
        let spine_format = self.settings.value_format();

        // -- Source ----------------------------------------------------------
        let source_summary = format!(
            "{} · {} triangles · {}",
            workflow.source_name,
            preflight.triangles,
            format_bytes(preflight.source_bytes),
        );
        spine_stage(
            ui,
            "source",
            "✓",
            OK,
            "Source",
            &source_summary,
            stage_index == 0,
            |ui| {
                diag(ui, "Source file", &workflow.source_name, TEXT_DIM);
                diag(
                    ui,
                    "Source SHA-256",
                    &short_hash(&preflight.source_sha256),
                    TEXT_DIM,
                );
                diag(ui, "Triangles", &preflight.triangles.to_string(), TEXT_DIM);
                diag(
                    ui,
                    "Components",
                    &preflight.components.to_string(),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Extents",
                    &format!(
                        "{:.3} × {:.3} × {:.3}",
                        preflight.source_extents[0],
                        preflight.source_extents[1],
                        preflight.source_extents[2]
                    ),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Transform",
                    if preflight.transform_approved {
                        "approved"
                    } else {
                        "awaiting approval"
                    },
                    if preflight.transform_approved {
                        TEXT_DIM
                    } else {
                        WARN
                    },
                );
            },
        );

        // -- Preflight -------------------------------------------------------
        let blocking = if preflight.ready() {
            Vec::new()
        } else {
            preflight.blocking_issues()
        };
        let watertight = preflight.boundary_edges == 0 && preflight.non_manifold_edges == 0;
        let (pf_glyph, pf_color, pf_summary) = if blocking.is_empty() {
            (
                "✓",
                OK,
                format!(
                    "Accepted · {} · {} component{}",
                    if watertight {
                        "watertight"
                    } else {
                        "open edges waived"
                    },
                    preflight.components,
                    if preflight.components == 1 { "" } else { "s" },
                ),
            )
        } else {
            (
                "!",
                DANGER,
                format!(
                    "Blocked · {} issue{}",
                    blocking.len(),
                    if blocking.len() == 1 { "" } else { "s" },
                ),
            )
        };
        spine_stage(
            ui,
            "preflight",
            pf_glyph,
            pf_color,
            "Preflight",
            &pf_summary,
            stage_index == 1 || !blocking.is_empty(),
            |ui| {
                if blocking.is_empty() {
                    alert_line(
                        ui,
                        OK,
                        "✓",
                        &format!(
                            "Geometry gate accepted · {} boundary edges · {} non-manifold edges",
                            preflight.boundary_edges, preflight.non_manifold_edges
                        ),
                    );
                } else {
                    for issue in &blocking {
                        alert_line(ui, DANGER, "×", issue);
                    }
                }
                for waiver in &preflight.waivers {
                    alert_line(ui, WARN, "!", &format!("Waived · {waiver}"));
                }
                ui.add_space(4.0);
                diag(
                    ui,
                    "Topology",
                    if watertight { "closed" } else { "open edges" },
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Boundary edges",
                    &preflight.boundary_edges.to_string(),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Non-manifold edges",
                    &preflight.non_manifold_edges.to_string(),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Minimum thickness",
                    &format!("{} cells", preflight.minimum_cells_across),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Boundary clearance",
                    &format!("{} cells", preflight.boundary_clearance_cells),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Solid voxels",
                    &preflight.solid_voxels.to_string(),
                    TEXT_DIM,
                );
            },
        );

        // -- Contract (model support) -----------------------------------------
        let support = &workflow.model_support;
        let contract_clean = support.status == "clean";
        let (ct_glyph, ct_color, ct_summary) = if contract_clean {
            (
                "✓",
                OK,
                format!(
                    "Compatible · {}³ grid · {} → {} channels",
                    support.grid, support.input_channels, support.output_channels
                ),
            )
        } else if support.status.is_empty() {
            ("○", TEXT_MUTE, "No model selected".to_owned())
        } else {
            (
                "!",
                WARN,
                format!("Metadata review · status {}", support.status),
            )
        };
        spine_stage(
            ui,
            "contract",
            ct_glyph,
            ct_color,
            "Contract",
            &ct_summary,
            stage_index == 2,
            |ui| {
                diag(ui, "Grid", &format!("{}³", support.grid), TEXT_DIM);
                diag(
                    ui,
                    "Channels",
                    &format!("{} → {}", support.input_channels, support.output_channels),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Horizon",
                    &format!("1–{}", workflow.model_max_steps),
                    TEXT_DIM,
                );
                diag(ui, "Physics contract", &support.physics_contract, TEXT_DIM);
                diag(ui, "Geometry regime", &support.scenario, TEXT_DIM);
            },
        );

        // -- Operating point ---------------------------------------------------
        let operating = &workflow.operating;
        let op_issues = operating.validation(workflow.model_max_steps);
        let op_summary = match operating.reynolds() {
            Some(reynolds) => format!(
                "Re {reynolds:.1} · {:.2} m/s · {} · horizon {}",
                operating.velocity,
                operating.length_unit.symbol(),
                operating.horizon_steps,
            ),
            None => "Units UNKNOWN — confirm geometry units".to_owned(),
        };
        let (op_glyph, op_color) = if op_issues.is_empty() {
            ("✓", OK)
        } else {
            ("!", WARN)
        };
        spine_stage(
            ui,
            "operating",
            op_glyph,
            op_color,
            "Operating point",
            &op_summary,
            stage_index == 2 || !op_issues.is_empty(),
            |ui| {
                for issue in &op_issues {
                    alert_line(ui, WARN, "!", issue);
                }
                if !op_issues.is_empty() {
                    ui.add_space(4.0);
                }
                diag(
                    ui,
                    "Geometry units",
                    operating.length_unit.symbol(),
                    if operating.length_unit == engineering::LengthUnit::Unknown {
                        WARN
                    } else {
                        TEXT_DIM
                    },
                );
                diag(
                    ui,
                    "Reference length",
                    &format!(
                        "{:.4} {}",
                        operating.reference_length,
                        operating.length_unit.symbol()
                    ),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Free-stream speed",
                    &units::format_quantity(
                        units::Quantity::Velocity,
                        operating.velocity,
                        spine_units,
                        spine_format,
                    ),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Density",
                    &units::format_quantity(
                        units::Quantity::Density,
                        operating.density,
                        spine_units,
                        spine_format,
                    ),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Dynamic viscosity",
                    &units::format_quantity(
                        units::Quantity::Viscosity,
                        operating.viscosity,
                        spine_units,
                        spine_format,
                    ),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Reynolds number",
                    &operating
                        .reynolds()
                        .map(|re| format!("{re:.1}"))
                        .unwrap_or_else(|| "UNKNOWN".into()),
                    TEXT_DIM,
                );
                diag(
                    ui,
                    "Horizon",
                    &format!("{} steps", operating.horizon_steps),
                    TEXT_DIM,
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Edit these values in the case controls on the right.")
                        .text_style(caption())
                        .color(TEXT_MUTE),
                );
            },
        );

        // -- Run (execution gate) ----------------------------------------------
        let issues = workflow.readiness_issues();
        let (run_glyph, run_color, run_summary) = if case.pending {
            ("●", EMBER, "Running · inputs locked".to_owned())
        } else if issues.is_empty() && contract_clean {
            ("✓", OK, "Ready".to_owned())
        } else if issues.is_empty() {
            ("!", WARN, "Ready · model metadata review".to_owned())
        } else {
            (
                "×",
                DANGER,
                format!(
                    "Blocked · {} issue{}",
                    issues.len(),
                    if issues.len() == 1 { "" } else { "s" },
                ),
            )
        };
        spine_stage(
            ui,
            "run",
            run_glyph,
            run_color,
            "Run",
            &run_summary,
            stage_index >= 3 || !issues.is_empty(),
            |ui| {
                if case.pending {
                    alert_line(
                        ui,
                        EMBER,
                        "●",
                        "Running — the result will be stored as a new immutable run with exact lineage.",
                    );
                } else if issues.is_empty() && contract_clean {
                    alert_line(
                        ui,
                        OK,
                        "✓",
                        "Geometry, transform, operating point, horizon, and model satisfy the current fixed-body support contract.",
                    );
                } else if issues.is_empty() {
                    alert_line(
                        ui,
                        WARN,
                        "!",
                        "The executable fixed-body contract is compatible. Checkpoint provenance or qualification metadata remains incomplete and is preserved in evidence.",
                    );
                } else {
                    for issue in &issues {
                        alert_line(ui, DANGER, "×", issue);
                    }
                }
            },
        );
    }

    fn engineering_evidence_view(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // One shared content column (§3.2, QA G3/G4).
            content_column(ui, CONTENT_MAX_WIDTH, |ui| {
                    ui.add_space(28.0);
                    ui.label(
                        display_text("Traceable engineering evidence"),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Every result is tied to exact source bytes, transform, operating point, model hash, run, and derivation method.",
                        )
                        .text_style(caption())
                        .color(TEXT_MUTE),
                    );
                    ui.add_space(20.0);
                    // §4.5: read-only review mode gets a designed banner
                    // (info recipe) instead of implied absence.
                    if !self.engine_ok {
                        card(ui, |ui| {
                            alert_line(
                                ui,
                                TEXT_DIM,
                                "○",
                                "Read-only review — engine unavailable. Stored fields and evidence remain inspectable.",
                            );
                        });
                        ui.add_space(14.0);
                    }
                    let Some(case) = &self.cad else {
                        ui.label(RichText::new("No active case.").color(TEXT_MUTE));
                        return;
                    };
                    // Lineage as a scannable ledger (§4.5): level-0 rows,
                    // hairline separated — source → case revision → run →
                    // model hash. Hash rows truncate to 12 chars with the
                    // full value on hover and a copy affordance.
                    ui.label(overline_text("Lineage"));
                    ui.add_space(8.0);
                    ledger_row(
                        ui,
                        "Geometry SHA-256",
                        Some(&case.workflow.preflight.source_sha256),
                        TEXT_DIM,
                        "unknown",
                    );
                    ledger_row(
                        ui,
                        "Source revision",
                        case.workflow.source_revision_id.as_deref(),
                        TEXT_DIM,
                        "unknown",
                    );
                    ledger_row(
                        ui,
                        "Case revision",
                        case.workflow.case_revision_id.as_deref(),
                        TEXT_DIM,
                        "unknown",
                    );
                    ledger_row(
                        ui,
                        "Immutable run",
                        case.active_run_id.as_deref(),
                        BRAND,
                        "not completed",
                    );
                    ledger_row(
                        ui,
                        "Model SHA-256",
                        case.workflow.model_sha256.as_deref(),
                        TEXT_DIM,
                        "unknown",
                    );
                    ledger_row(
                        ui,
                        "Load method",
                        Some(engineering::SURFACE_LOAD_METHOD),
                        TEXT_DIM,
                        "unknown",
                    );

                    // Run ledger: every stored run for this case, newest
                    // first; each row deep-links to its immutable run
                    // (N6-COMP-01 pattern).
                    let runs: Vec<(String, u64, project::LifecycleState, bool)> = self
                        .project
                        .manifest()
                        .cases()
                        .iter()
                        .find(|record| record.case_id() == case.workflow.case_id)
                        .map(|record| {
                            record
                                .runs()
                                .iter()
                                .map(|run| {
                                    (
                                        run.run_id().to_owned(),
                                        run.created_utc_unix(),
                                        run.state(),
                                        case.active_run_id.as_deref() == Some(run.run_id()),
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let mut inspect_run: Option<String> = None;
                    if !runs.is_empty() {
                        ui.add_space(20.0);
                        ui.label(overline_text("Run ledger"));
                        ui.add_space(8.0);
                        for (run_id, created, state, active) in runs.iter().rev() {
                            if run_ledger_row(ui, run_id, *created, *state, *active) {
                                inspect_run = Some(run_id.clone());
                            }
                        }
                    }
                    if let Some(result) = &case.workflow.result {
                        ui.add_space(20.0);
                        ui.label(overline_text("Scientific labels"));
                        ui.add_space(8.0);
                        for (quantity, label) in [
                            ("Velocity", "model prediction"),
                            ("Pressure", "recovered from predicted velocity"),
                            ("Cp", "derived using the complete reference state"),
                            ("Surface traction", "pressure + Newtonian viscous fluid load"),
                            ("Structural stress", "not computed"),
                        ] {
                            diag(ui, quantity, label, TEXT_DIM);
                        }
                        if !result.warnings.is_empty() {
                            ui.add_space(8.0);
                            for warning in &result.warnings {
                                alert_line(ui, WARN, "!", &format!("Notice · {warning}"));
                            }
                        }
                    }
                    if let Some(run_id) = inspect_run {
                        self.select_external_run(&run_id);
                    }
                    ui.add_space(28.0);
                });
        });
    }

    fn export_fea_loads(&mut self) {
        let Some(case) = self.cad.as_ref() else {
            self.project_notice = Some(("No case result is available to export.".into(), true));
            return;
        };
        let n = case.result_grid;
        let cube = n.saturating_mul(n).saturating_mul(n);
        if n < 3
            || case.mask.len() != cube
            || case.cp.len() != cube
            || case.traction.len() != 3 * cube
        {
            self.project_notice = Some((
                "The active result has no complete mapped surface-load field.".into(),
                true,
            ));
            return;
        }
        let scale = case
            .workflow
            .operating
            .length_unit
            .meters_per_unit()
            .unwrap_or(1.0);
        let dx_solver = std::f64::consts::TAU / n as f64;
        let index = |i: usize, j: usize, k: usize| i * n * n + j * n + k;
        let mut positions = Vec::new();
        let mut tractions = Vec::new();
        let mut coefficients = Vec::new();
        for i in 1..n - 1 {
            for j in 1..n - 1 {
                for k in 1..n - 1 {
                    let gradient = [
                        (case.mask[index(i + 1, j, k)] - case.mask[index(i - 1, j, k)]) as f64,
                        (case.mask[index(i, j + 1, k)] - case.mask[index(i, j - 1, k)]) as f64,
                        (case.mask[index(i, j, k + 1)] - case.mask[index(i, j, k - 1)]) as f64,
                    ];
                    let magnitude = gradient
                        .iter()
                        .map(|value| value * value)
                        .sum::<f64>()
                        .sqrt();
                    if magnitude <= 0.02 {
                        continue;
                    }
                    let cell = index(i, j, k);
                    let solver_point = [
                        (i as f64 + 0.5) * dx_solver,
                        (j as f64 + 0.5) * dx_solver,
                        (k as f64 + 0.5) * dx_solver,
                    ];
                    let position = match engineering::solver_point_to_source_m(
                        solver_point,
                        case.workflow.preflight.transform_4x4,
                        scale,
                    ) {
                        Ok(position) => position,
                        Err(error) => {
                            self.project_notice =
                                Some((format!("FEA coordinate mapping failed: {error}"), true));
                            return;
                        }
                    };
                    positions.push(position);
                    tractions.push([
                        case.traction[cell] as f64,
                        case.traction[cube + cell] as f64,
                        case.traction[2 * cube + cell] as f64,
                    ]);
                    coefficients.push(case.cp[cell] as f64);
                }
            }
        }
        let provenance = engineering::FeaLoadProvenance {
            source_revision_id: case.workflow.source_revision_id.clone().unwrap_or_default(),
            case_revision_id: case.workflow.case_revision_id.clone().unwrap_or_default(),
            run_id: case.active_run_id.clone().unwrap_or_default(),
            model_sha256: case.workflow.model_sha256.clone().unwrap_or_default(),
            contract_kind: engineering::EXTERNAL_FLOW_CONTRACT.into(),
            coordinate_frame: "approved_stl_source_frame_si_meters".into(),
        };
        let csv =
            match engineering::fea_load_csv(&positions, &tractions, &coefficients, &provenance) {
                Ok(csv) => csv,
                Err(error) => {
                    self.project_notice = Some((error, true));
                    return;
                }
            };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(format!(
                "{}_surface_loads.csv",
                case.workflow.name.replace(' ', "_")
            ))
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, csv) {
            Ok(()) => {
                self.project_notice = Some((
                    format!(
                        "Exported {} mapped fluid-load points for downstream FEA to {}.",
                        positions.len(),
                        path.display()
                    ),
                    false,
                ));
            }
            Err(error) => {
                self.project_notice = Some((format!("FEA load export failed: {error}"), true));
            }
        }
    }

    /// A save dialog seeded with the Settings › Workflow default export
    /// directory (when set and present on disk).
    fn export_dialog(&self, file_name: &str) -> rfd::FileDialog {
        let mut dialog = rfd::FileDialog::new().set_file_name(file_name);
        let directory = self.settings.default_export_directory.trim();
        if !directory.is_empty() && std::path::Path::new(directory).is_dir() {
            dialog = dialog.set_directory(directory);
        }
        dialog
    }

    /// Export the currently displayed engineering section (stored field data,
    /// same renderer as the on-screen view) as a PNG.
    fn export_section_png(&mut self) {
        let Some(section) = self.section_data.as_ref() else {
            self.project_notice = Some((
                "No section is currently rendered — open a 2D section in Results first.".into(),
                true,
            ));
            return;
        };
        let image = engineering_section_image(section);
        let bytes = match color_image_png_bytes(&image, 512) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.project_notice = Some((format!("Section PNG failed: {error}"), true));
                return;
            }
        };
        let case_name = self
            .cad
            .as_ref()
            .map(|case| case.workflow.name.replace(' ', "_"))
            .unwrap_or_else(|| "case".into());
        let file_name = format!(
            "{case_name}_{}section_{}.png",
            section.axis.label(),
            section.quantity.label().to_lowercase().replace(' ', "_")
        );
        let Some(path) = self
            .export_dialog(&file_name)
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        self.project_notice = Some(match std::fs::write(&path, bytes) {
            Ok(()) => (format!("Section exported to {}.", path.display()), false),
            Err(error) => (format!("Section PNG was not written: {error}"), true),
        });
    }

    /// Ask the windowing backend for a composited frame (includes the wgpu 3D
    /// pass) and save the current render-viewport region on arrival.
    fn request_viewport_png(&mut self, ctx: &egui::Context) {
        if self.last_render_rect.is_none() {
            self.project_notice = Some((
                "No render viewport is currently visible to capture.".into(),
                true,
            ));
            return;
        }
        let case_name = self
            .cad
            .as_ref()
            .map(|case| case.workflow.name.replace(' ', "_"))
            .unwrap_or_else(|| "viewport".into());
        let Some(path) = self
            .export_dialog(&format!("{case_name}_viewport.png"))
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        self.pending_viewport_shot = Some(path);
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
    }

    /// Consume a completed screenshot event: crop to the render viewport and
    /// write the pending PNG.
    fn handle_screenshot_events(&mut self, ctx: &egui::Context) {
        if self.pending_viewport_shot.is_none() {
            return;
        }
        let image = ctx.input(|input| {
            input.events.iter().rev().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(image) = image else { return };
        let Some(path) = self.pending_viewport_shot.take() else {
            return;
        };
        let pixels_per_point = ctx.pixels_per_point();
        let cropped = self
            .last_render_rect
            .map(|rect| image.region(&rect, Some(pixels_per_point)))
            .unwrap_or_else(|| (*image).clone());
        self.project_notice = Some(
            match color_image_png_bytes(&cropped, 0)
                .and_then(|bytes| std::fs::write(&path, bytes).map_err(|error| error.to_string()))
            {
                Ok(()) => (format!("Viewport exported to {}.", path.display()), false),
                Err(error) => (format!("Viewport PNG failed: {error}"), true),
            },
        );
    }

    /// Export the self-contained HTML engineering report for the active
    /// immutable run (provenance chain + limitations always included).
    fn export_engineering_report(&mut self) {
        let Some(case) = self.cad.as_ref() else {
            self.project_notice = Some(("No case is active.".into(), true));
            return;
        };
        let Some(run_id) = case.active_run_id.clone() else {
            self.project_notice = Some((
                "A durable immutable run is required before a report can be produced.".into(),
                true,
            ));
            return;
        };
        let run_created_utc_unix = self
            .project
            .manifest()
            .cases()
            .iter()
            .find(|record| record.case_id() == case.workflow.case_id)
            .and_then(|record| {
                record
                    .runs()
                    .iter()
                    .find(|run| run.run_id() == run_id)
                    .map(|run| run.created_utc_unix())
            })
            .unwrap_or_else(now_utc_unix);
        let section_figure = self.section_data.as_ref().and_then(|section| {
            let image = engineering_section_image(section);
            use base64::Engine as _;
            color_image_png_bytes(&image, 512)
                .ok()
                .map(|bytes| report::SectionFigure {
                    png_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
                    caption: format!(
                        "{} section at {:.3} (domain) · {} [{}] — stored run field, same renderer as the interactive view",
                        section.axis.label(),
                        section.location,
                        section.quantity.label(),
                        section.quantity.units()
                    ),
                })
        });
        let input = report::ReportInput {
            case: &case.workflow,
            run_id: &run_id,
            run_created_utc_unix,
            generated_utc_unix: now_utc_unix(),
            app_version: env!("CARGO_PKG_VERSION"),
            unit_system: self.settings.unit_system,
            format: self.settings.value_format(),
            section_figure,
        };
        let html = match report::engineering_report_html(&input) {
            Ok(html) => html,
            Err(error) => {
                self.project_notice = Some((format!("Report was not produced: {error}"), true));
                return;
            }
        };
        let file_name = format!(
            "{}_{}_report.html",
            case.workflow.name.replace(' ', "_"),
            short_id(&run_id)
        );
        let Some(path) = self
            .export_dialog(&file_name)
            .add_filter("HTML", &["html"])
            .save_file()
        else {
            return;
        };
        self.project_notice = Some(match std::fs::write(&path, html) {
            Ok(()) => (
                format!("Engineering report exported to {}.", path.display()),
                false,
            ),
            Err(error) => (format!("Report was not written: {error}"), true),
        });
    }

    /// Route native macOS menu clicks through the same handlers the deleted
    /// in-app menus used, then mirror app state back onto the menu items.
    #[cfg(target_os = "macos")]
    fn handle_menu_signals(&mut self, ctx: &egui::Context) {
        let signals = match self.menubar.as_ref() {
            Some(menubar) => menubar.poll(),
            None => return,
        };
        for signal in signals {
            match signal {
                MenuSignal::Command(command) => match command {
                    MenuCommand::NewProject => self
                        .request_project_action(project_lifecycle::DeferredProjectAction::New, ctx),
                    MenuCommand::OpenProject => self.open_project_dialog(ctx),
                    MenuCommand::SaveProject => {
                        self.save_project_dialog();
                    }
                    MenuCommand::SaveProjectAs => {
                        self.save_project_as_dialog();
                    }
                    MenuCommand::ImportModel => self.import_model(),
                    MenuCommand::ImportCad => self.import_cad(),
                    MenuCommand::ExportCalculations => self.export(),
                    MenuCommand::Quit => self.request_project_action(
                        project_lifecycle::DeferredProjectAction::Quit,
                        ctx,
                    ),
                    MenuCommand::ResetControls => self.reset_controls(),
                    MenuCommand::ResetCamera => self.cam = viewport::Camera::default(),
                    MenuCommand::ToggleDimension => self.volumetric = !self.volumetric,
                    MenuCommand::RunAnalysis => {
                        if matches!(self.nav, Nav::Case | Nav::Results) {
                            self.run_external_flow();
                        }
                    }
                    MenuCommand::ExportFea => {
                        if matches!(self.nav, Nav::Case | Nav::Results) {
                            self.export_fea_loads();
                        }
                    }
                    MenuCommand::RegenerateSandbox => {
                        if self.settings.developer_research_sandbox {
                            self.regenerate();
                        }
                    }
                    MenuCommand::ToggleSandboxLive => {
                        if self.settings.developer_research_sandbox {
                            self.live = !self.live;
                        }
                    }
                    MenuCommand::OpenDocs => {
                        open_url(concat!("file://", env!("CARGO_MANIFEST_DIR"), "/PRD.md"));
                    }
                },
                MenuSignal::OpenRecent(path) => self.request_project_action(
                    project_lifecycle::DeferredProjectAction::Open(path),
                    ctx,
                ),
            }
        }
        let sync = MenuSyncState {
            can_save: self.project.is_dirty() || self.project.path().is_none(),
            analysis_available: matches!(self.nav, Nav::Case | Nav::Results),
            sandbox_enabled: self.settings.developer_research_sandbox,
            sandbox_live: self.live,
            recents: self
                .project
                .recent_projects()
                .iter()
                .map(|recent| (recent.name.clone(), recent.path.clone()))
                .collect(),
        };
        if let Some(menubar) = self.menubar.as_mut() {
            menubar.sync(sync);
        }
    }

    /// Why a new run cannot start right now (`None` = it can). One source of
    /// truth for the top-bar Run button, the command palette, and anything
    /// else that gates on runnability (UX-AC-01: reasons, not dead controls).
    fn run_gate_reason(&self) -> Option<String> {
        if self.cad.as_ref().is_some_and(|case| case.pending) {
            return Some("An immutable run attempt is already in flight.".to_owned());
        }
        if !self.engine_ok {
            return Some(
                "Engine unavailable — stored evidence stays readable, but a new run cannot start."
                    .to_owned(),
            );
        }
        match self.cad.as_ref() {
            None => Some("No case yet — import geometry in Case Setup first.".to_owned()),
            Some(case) => {
                let issues = case.workflow.readiness_issues();
                if issues.is_empty() {
                    None
                } else {
                    Some(format!("Run blocked: {}", issues.join(" ")))
                }
            }
        }
    }

    /// ⌘K command palette (§6 Tier 3.5): navigation + actions with the same
    /// state-gating as the nav rail. A gated entry never executes — it shows
    /// its reason instead. Actions only; no fake content.
    fn command_palette(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            self.palette_open = !self.palette_open;
            self.palette_query.clear();
            self.palette_selected = 0;
        }
        if !self.palette_open {
            return;
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.palette_open = false;
            return;
        }

        // Build the gated entry list from live state.
        enum PaletteAction {
            Nav(Nav),
            Run,
            ImportGeometry,
            OpenProject,
            SaveProject,
            OpenRecent(std::path::PathBuf),
        }
        let has_result = self
            .cad
            .as_ref()
            .and_then(|case| case.workflow.result.as_ref())
            .is_some();
        let mut entries: Vec<(String, PaletteAction, Option<String>)> = vec![
            (
                "Go to Project".into(),
                PaletteAction::Nav(Nav::Projects),
                None,
            ),
            (
                "Go to Case Setup".into(),
                PaletteAction::Nav(Nav::Case),
                None,
            ),
            (
                "Go to Results".into(),
                PaletteAction::Nav(Nav::Results),
                (!has_result).then(|| "No completed run yet — run the case first.".to_owned()),
            ),
            (
                "Go to Evidence".into(),
                PaletteAction::Nav(Nav::Evidence),
                None,
            ),
            (
                "Go to Model Library".into(),
                PaletteAction::Nav(Nav::Models),
                None,
            ),
            (
                "Go to Settings".into(),
                PaletteAction::Nav(Nav::Settings),
                None,
            ),
            (
                "Run analysis".into(),
                PaletteAction::Run,
                self.run_gate_reason(),
            ),
            (
                "Import geometry (STL)…".into(),
                PaletteAction::ImportGeometry,
                None,
            ),
            ("Open project…".into(), PaletteAction::OpenProject, None),
            ("Save project".into(), PaletteAction::SaveProject, None),
        ];
        if self.settings.developer_research_sandbox {
            entries.push((
                "Go to Procedural 3D (sandbox)".into(),
                PaletteAction::Nav(Nav::Metrics),
                None,
            ));
            entries.push((
                "Go to Flow Painter (sandbox)".into(),
                PaletteAction::Nav(Nav::FlowPainter),
                None,
            ));
            entries.push((
                "Go to Fields 2D (sandbox)".into(),
                PaletteAction::Nav(Nav::Fields2D),
                None,
            ));
            entries.push((
                "Go to Benchmark Lab (sandbox)".into(),
                PaletteAction::Nav(Nav::Benchmark),
                None,
            ));
        }
        for recent in self.project.recent_projects() {
            entries.push((
                format!("Open recent: {}", recent.name),
                PaletteAction::OpenRecent(recent.path.clone()),
                (!recent.path.is_file())
                    .then(|| "The file no longer exists at this path.".to_owned()),
            ));
        }
        let query = self.palette_query.to_lowercase();
        let filtered: Vec<(String, PaletteAction, Option<String>)> = entries
            .into_iter()
            .filter(|(title, _, _)| query.is_empty() || title.to_lowercase().contains(&query))
            .collect();
        if self.palette_selected >= filtered.len() {
            self.palette_selected = filtered.len().saturating_sub(1);
        }
        // Keyboard: arrows move, Enter fires (only when the entry is free).
        if ctx.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
            self.palette_selected =
                (self.palette_selected + 1).min(filtered.len().saturating_sub(1));
        }
        if ctx.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
            self.palette_selected = self.palette_selected.saturating_sub(1);
        }
        let enter = ctx.input(|input| input.key_pressed(egui::Key::Enter));

        let mut chosen: Option<PaletteAction> = None;
        let modal = egui::Modal::new(egui::Id::new("command-palette"))
            .backdrop_color(Color32::from_black_alpha(90))
            .show(ctx, |ui| {
                ui.set_width(520.0);
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.palette_query)
                        .hint_text("Navigate or act… (↑↓ to select, ↵ to run)")
                        .desired_width(f32::INFINITY)
                        .frame(Frame::NONE),
                );
                edit.request_focus();
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                if filtered.is_empty() {
                    ui.label(
                        RichText::new("No matching command.")
                            .text_style(caption())
                            .color(TEXT_MUTE),
                    );
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (index, (title, action, gate)) in filtered.into_iter().enumerate() {
                            let selected = index == self.palette_selected;
                            let gated = gate.is_some();
                            let (rect, resp) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 32.0),
                                if gated {
                                    Sense::hover()
                                } else {
                                    Sense::click()
                                },
                            );
                            if resp.hovered() && !selected {
                                self.palette_selected = index;
                            }
                            let painter = ui.painter();
                            if selected {
                                painter.rect_filled(rect, CornerRadius::same(R1), SURFACE_HIGH);
                                if !gated {
                                    painter.rect_filled(
                                        Rect::from_min_size(
                                            rect.min + Vec2::new(0.0, 6.0),
                                            Vec2::new(2.0, rect.height() - 12.0),
                                        ),
                                        CornerRadius::same(1),
                                        EMBER,
                                    );
                                }
                            }
                            painter.text(
                                egui::pos2(rect.min.x + 12.0, rect.center().y),
                                Align2::LEFT_CENTER,
                                &title,
                                body_strong().resolve(ui.style()),
                                if gated {
                                    TEXT_MUTE
                                } else if selected {
                                    TEXT
                                } else {
                                    TEXT_DIM
                                },
                            );
                            if let Some(reason) = &gate {
                                painter.text(
                                    egui::pos2(rect.max.x - 12.0, rect.center().y),
                                    Align2::RIGHT_CENTER,
                                    format!("○ {reason}"),
                                    caption().resolve(ui.style()),
                                    TEXT_MUTE,
                                );
                            }
                            let fire = (!gated && resp.clicked()) || (!gated && selected && enter);
                            if fire {
                                chosen = Some(action);
                            }
                        }
                    });
            });
        if modal.should_close() {
            self.palette_open = false;
        }
        if let Some(action) = chosen {
            self.palette_open = false;
            match action {
                PaletteAction::Nav(nav) => {
                    self.nav = nav;
                    if nav == Nav::Fields2D && self.f2d.is_none() && !self.f2d_pending {
                        self.request_2d();
                    }
                }
                PaletteAction::Run => self.run_external_flow(),
                PaletteAction::ImportGeometry => self.import_cad(),
                PaletteAction::OpenProject => self.open_project_dialog(ctx),
                PaletteAction::SaveProject => {
                    self.save_project_dialog();
                }
                PaletteAction::OpenRecent(path) => self.request_project_action(
                    project_lifecycle::DeferredProjectAction::Open(path),
                    ctx,
                ),
            }
        }
    }

    /// 44px single-chrome top bar (§4.1): traffic-light inset, project
    /// identity + truthful dirty state, and the contextual run action. The
    /// bar itself is a window drag region.
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let dirty = self.project.is_dirty();
        let window_title = format!(
            "{}{} — Reyn Studio",
            self.project.display_name(),
            if dirty { " •" } else { "" }
        );
        // Send the title command only when it changes (A21).
        if window_title != self.last_window_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(window_title.clone()));
            self.last_window_title = window_title;
        }
        let top_response = egui::Panel::top("top")
            .exact_size(44.0)
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(SURFACE_LOWEST)
                    .inner_margin(Margin::symmetric(14, 0)),
            )
            .show(ui, |ui| {
                // Window drag region behind the bar's widgets; double-click
                // zooms, matching native titlebar behavior.
                let drag = ui.interact(
                    ui.max_rect(),
                    egui::Id::new("topbar.drag"),
                    Sense::click_and_drag(),
                );
                if drag.drag_started_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                ui.horizontal_centered(|ui| {
                    let fullscreen = ui.input(|input| input.viewport().fullscreen.unwrap_or(false));
                    // Native traffic lights sit inside the fullsize content
                    // view; inset our content past them when they're visible.
                    // 78px inset per §4.1 so the brand never crowds the
                    // traffic lights (QA C10).
                    ui.add_space(if fullscreen { 4.0 } else { 78.0 });
                    ui.label(
                        RichText::new("Reyn Studio")
                            .text_style(body_strong())
                            .color(BRAND),
                    );
                    ui.add_space(12.0);
                    // Project identity chip — the one place project name +
                    // dirty state appear in chrome (truthful, never implied).
                    let location = self
                        .project
                        .path()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "not saved to disk".into());
                    Frame::NONE
                        .fill(SURFACE)
                        .stroke(Stroke::new(1.0, HAIRLINE))
                        .corner_radius(CornerRadius::same(R1))
                        .inner_margin(Margin::symmetric(10, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                ui.label(
                                    RichText::new(self.project.display_name())
                                        .text_style(body_strong())
                                        .color(TEXT),
                                );
                                if dirty {
                                    ui.label(RichText::new("●").size(8.0).color(WARN))
                                        .on_hover_text("Unsaved changes");
                                }
                            });
                        })
                        .response
                        .on_hover_text(location);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if matches!(self.nav, Nav::Case | Nav::Results) {
                            // Ember only when the run can actually start;
                            // otherwise the button is visibly disabled and
                            // carries the blocking reason (UX-AC-01: no dead
                            // or lying controls, ember budget §3.3).
                            let pending = self.cad.as_ref().is_some_and(|case| case.pending);
                            let reason = self.run_gate_reason();
                            if action_button_gated(
                                ui,
                                Some(ph::PLAY),
                                if pending {
                                    "Running…"
                                } else {
                                    "Run analysis"
                                },
                                EMBER,
                                ON_EMBER,
                                None,
                                34.0,
                                132.0,
                                reason.as_deref(),
                            ) {
                                self.run_external_flow();
                            }
                        } else if self.is_research_sandbox() {
                            let live_icon = if self.live { None } else { Some(ph::PLAY) };
                            let live_label = if self.live {
                                "◉  Sandbox live"
                            } else {
                                "Sandbox live"
                            };
                            // Ember only while running (running-state indicator);
                            // the idle toggle is a quiet/tonal button.
                            let (live_fill, live_fg, live_border) = if self.live {
                                (EMBER, ON_EMBER, None)
                            } else {
                                (SURFACE_HIGH, TEXT, Some(HAIRLINE))
                            };
                            if action_button(
                                ui,
                                live_icon,
                                live_label,
                                live_fill,
                                live_fg,
                                live_border,
                                34.0,
                                132.0,
                            ) {
                                self.live = !self.live;
                            }
                        }
                        ui.add_space(14.0);
                        // 2D | 3D VOLUMETRIC segmented toggle (left-to-right order)
                        if matches!(self.nav, Nav::Results | Nav::Metrics) {
                            // Concentric radii: thumb r-1 = 4 inside container
                            // r-2 = 6 with a 2px inset (§3.5).
                            Frame::NONE
                                .fill(SURFACE)
                                .corner_radius(CornerRadius::same(R2))
                                .stroke(Stroke::new(1.0, HAIRLINE))
                                .inner_margin(2)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 2.0;
                                        if seg(ui, "2D section", !self.volumetric) {
                                            self.volumetric = false;
                                        }
                                        if seg(ui, "3D", self.volumetric) {
                                            self.volumetric = true;
                                        }
                                    });
                                });
                        }
                    });
                });
            });
        // Single meeting-edge hairline (QA C5): the bar owns only its bottom
        // border instead of stroking all four edges.
        let rect = top_response.response.rect;
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, OUTLINE_VARIANT),
        );
    }

    /// 24px status bar (§4.1): the single truthful home for engine state,
    /// long-operation progress, and the active model — replaces the floating
    /// engine pill that hovered over the viewport.
    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let status_response = egui::Panel::bottom("status")
            .exact_size(24.0)
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(SURFACE_LOWEST)
                    .inner_margin(Margin::symmetric(12, 0)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    // WARN is the status hue; GOLD stays reserved for data.
                    let engine_color = if self.engine_ok { SUCCESS } else { WARN };
                    // C6: long engine errors elide before the right-aligned
                    // run/model cluster; full text on hover.
                    ui.scope(|ui| {
                        ui.set_max_width((ui.available_width() - 330.0).max(140.0));
                        ui.add(
                            egui::Label::new(
                                RichText::new(&self.engine_status)
                                    .text_style(mono_s())
                                    .color(engine_color),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&self.engine_status);
                    });
                    let busy = if self.cad.as_ref().is_some_and(|case| case.pending) {
                        Some("◐ running immutable attempt…")
                    } else if self.bench_running {
                        Some("◐ benchmark suite running…")
                    } else if self.f2d_pending {
                        Some("◐ 2D prediction pending…")
                    } else if self.library.busy {
                        Some("◐ checkpoint operation in progress…")
                    } else {
                        None
                    };
                    if let Some(busy) = busy {
                        ui.add_space(16.0);
                        // Busy is passive status — WARN, never the ember
                        // action accent (QA C6).
                        ui.label(RichText::new(busy).text_style(mono_s()).color(WARN));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("model · {}", self.current_model))
                                .text_style(mono_s())
                                .color(TEXT_MUTE),
                        );
                        if let Some(run_id) = self
                            .cad
                            .as_ref()
                            .and_then(|case| case.active_run_id.as_deref())
                        {
                            ui.add_space(14.0);
                            ui.label(
                                RichText::new(format!("run · {}", short_id(run_id)))
                                    .text_style(mono_s())
                                    .color(TEXT_MUTE),
                            );
                        }
                    });
                });
            });
        // Single meeting-edge hairline (QA C5).
        let rect = status_response.response.rect;
        ui.painter().hline(
            rect.x_range(),
            rect.top() + 0.5,
            Stroke::new(1.0, OUTLINE_VARIANT),
        );
    }

    /// Left rail (§4.1): project identity, then two visibly distinct groups —
    /// the workflow lifecycle with per-stage state glyphs, and workbench
    /// destinations. Resizable; the voxel-diagnostics card moved to the
    /// Results right rail where its field actually lives (SCI-AC-01).
    fn left_sidebar(&mut self, ui: &mut egui::Ui) {
        let sidebar_response = egui::Panel::left("sidebar")
            .resizable(true)
            .default_size(248.0)
            .size_range(208.0..=320.0)
            .frame(
                Frame::NONE
                    .fill(SURFACE_LOWEST)
                    .inner_margin(Margin::same(16)),
            )
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.label(title_text(self.project.display_name()));
                let project_location = self
                    .project
                    .path()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("not saved");
                ui.label(
                    RichText::new(format!("{project_location} · schema v2"))
                        .text_style(mono_s())
                        .color(TEXT_MUTE),
                );
                let (project_state, project_state_color) = if self.project.is_recovered() {
                    ("RECOVERED · UNSAVED", WARN)
                } else if self.project.is_dirty() {
                    ("UNSAVED CHANGES", WARN)
                } else if self.project.path().is_some() {
                    ("SAVED LOCALLY", SUCCESS)
                } else {
                    ("LOCAL SESSION", TEXT_MUTE)
                };
                ui.label(chip_text(project_state).color(project_state_color));
                ui.add_space(24.0);

                ui.label(caps("Workflow"));
                ui.add_space(8.0);
                let summary = self.project.summary();
                let has_result = self
                    .cad
                    .as_ref()
                    .and_then(|case| case.workflow.result.as_ref())
                    .is_some();
                let stages = workflow_stage_states(
                    summary.cases > 0 || summary.runs > 0,
                    self.cad.is_some(),
                    has_result,
                    summary.runs > 0,
                );
                if stage_row(
                    ui,
                    ph::FOLDER,
                    "Project",
                    self.nav == Nav::Projects,
                    stages[0],
                ) {
                    self.nav = Nav::Projects;
                }
                if stage_row(ui, ph::WIND, "Case Setup", self.nav == Nav::Case, stages[1]) {
                    self.nav = Nav::Case;
                }
                // No silent redirect (A5): a blocked Results row explains
                // itself inline; stage_row never fires a click while blocked.
                if stage_row(
                    ui,
                    ph::CHART_BAR,
                    "Results",
                    self.nav == Nav::Results,
                    stages[2],
                ) {
                    self.nav = Nav::Results;
                }
                if stage_row(
                    ui,
                    ph::BOOK_OPEN,
                    "Evidence",
                    self.nav == Nav::Evidence,
                    stages[3],
                ) {
                    self.nav = Nav::Evidence;
                }

                ui.add_space(24.0);
                ui.label(caps("Workbench"));
                ui.add_space(8.0);
                if nav_row(ui, ph::CUBE, "Model Library", self.nav == Nav::Models) {
                    self.nav = Nav::Models;
                }
                if nav_row(ui, ph::GEAR, "Settings", self.nav == Nav::Settings) {
                    self.nav = Nav::Settings;
                }

                if self.settings.developer_research_sandbox {
                    ui.add_space(24.0);
                    ui.label(caps("Developer · Research Sandbox"));
                    ui.label(chip_text("NOT ENGINEERING EVIDENCE").color(WARN));
                    ui.add_space(8.0);
                    if nav_row(ui, ph::ATOM, "Procedural 3D", self.nav == Nav::Metrics) {
                        self.nav = Nav::Metrics;
                    }
                    if nav_row(
                        ui,
                        ph::PAINT_BRUSH,
                        "Flow Painter",
                        self.nav == Nav::FlowPainter,
                    ) {
                        self.nav = Nav::FlowPainter;
                    }
                    if nav_row(ui, ph::STACK, "Fields (2D)", self.nav == Nav::Fields2D) {
                        self.nav = Nav::Fields2D;
                        if self.f2d.is_none() && !self.f2d_pending {
                            self.request_2d();
                        }
                    }
                    if nav_row(ui, ph::FLASK, "Benchmark Lab", self.nav == Nav::Benchmark) {
                        self.nav = Nav::Benchmark;
                    }
                }
            });
        // Single meeting-edge hairline (QA C5).
        let rect = sidebar_response.response.rect;
        ui.painter().vline(
            rect.right() - 0.5,
            rect.y_range(),
            Stroke::new(1.0, OUTLINE_VARIANT),
        );
    }

    fn right_controls(&mut self, ui: &mut egui::Ui) {
        // Model Library owns its whole screen (§4.6) — the 330px rail is
        // dissolved into the screen's own toolbar and header.
        if self.nav == Nav::Models {
            return;
        }
        let rail_response = egui::Panel::right("controls")
            .resizable(true)
            .default_size(330.0)
            .size_range(280.0..=420.0)
            .frame(Frame::NONE.fill(BG).inner_margin(Margin::same(16)))
            .show(ui, |ui| {
                // Every rail scrolls (QA CS1/R1/P3/X1): one wrapper at the
                // dispatch point so short windows can always reach the
                // bottom-most controls on every screen.
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.right_controls_body(ui));
            });
        // Single meeting-edge hairline (QA C5).
        let rect = rail_response.response.rect;
        ui.painter().vline(
            rect.left() + 0.5,
            rect.y_range(),
            Stroke::new(1.0, OUTLINE_VARIANT),
        );
    }

    fn right_controls_body(&mut self, ui: &mut egui::Ui) {
        if self.nav == Nav::Projects {
            self.controls_project(ui);
            return;
        }
        if self.nav == Nav::Case {
            self.controls_engineering_case(ui);
            return;
        }
        if self.nav == Nav::Results {
            self.controls_engineering_results(ui);
            return;
        }
        if self.nav == Nav::Evidence {
            self.controls_engineering_evidence(ui);
            return;
        }
        if self.nav == Nav::Fields2D {
            self.controls_2d(ui);
            return;
        }
        if self.nav == Nav::FlowPainter {
            self.controls_painter(ui);
            return;
        }
        if self.nav == Nav::Benchmark {
            self.controls_bench(ui);
            return;
        }
        if self.nav == Nav::Settings {
            settings::show_controls(
                ui,
                &self.settings,
                &self.engine_status,
                self.engine_ok,
                self.settings_notice.as_ref(),
            );
            return;
        }
        ui.spacing_mut().slider_width = 120.0;
        ui.label(title_text("3D Controls"));
        ui.add_space(20.0);

        ui.label(caps("Slicing Planes"));
        ui.add_space(8.0);
        for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
            Frame::NONE
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, OUTLINE_VARIANT))
                .corner_radius(CornerRadius::same(3))
                .inner_margin(Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.slice[i], "");
                        ui.label(RichText::new(*axis).color(TEXT).strong());
                        ui.add(
                            egui::Slider::new(&mut self.slice_pos[i], 0.0..=1.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                mono(&format!("{:.2}", self.slice_pos[i]), TEXT_DIM).size(12.0),
                            );
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
                    ui.label(
                        mono(
                            &format!("{:.2} – {:.1}", self.density_lo, self.density_hi),
                            EMBER,
                        )
                        .size(12.0),
                    );
                });
            });
            ui.add_space(6.0);
            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
            ui.add(
                egui::Slider::new(&mut self.density_lo, 0.0..=self.density_hi)
                    .show_value(false)
                    .trailing_fill(true),
            );
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
            ui.add(
                egui::Slider::new(&mut self.opacity, 0.0..=1.0)
                    .show_value(false)
                    .trailing_fill(true),
            );
            ui.add_space(8.0);
            ui.checkbox(
                &mut self.shadows,
                RichText::new("Volumetric Shadows").color(TEXT_DIM),
            );
            ui.checkbox(
                &mut self.streamlines,
                RichText::new("Show Streamlines").color(TEXT_DIM),
            );
            ui.add_enabled_ui(self.volumetric, |ui| {
                ui.checkbox(
                    &mut self.render_volume,
                    RichText::new("Volume Raymarch").color(TEXT_DIM),
                );
            });
            ui.add_enabled_ui(!self.insights3d.is_empty(), |ui| {
                ui.checkbox(
                    &mut self.insights_on,
                    RichText::new("Pin Critical Points").color(TEXT_DIM),
                )
                .on_disabled_hover_text("needs a model field (engine prediction)");
            });
            ui.add_enabled_ui(self.cad.as_ref().is_some_and(|c| c.surf.is_some()), |ui| {
                        ui.checkbox(
                            &mut self.surface_on,
                            RichText::new("Normalized recovered pressure").color(TEXT_DIM),
                        )
                        .on_hover_text(
                            "Density-normalized recovered pressure, normalized again to the visible color range; a physical pressure coefficient is not computed.",
                        )
                        .on_disabled_hover_text("import a CAD geometry (File → Import CAD)");
                    });
        });

        ui.add_space(20.0);
        // Quiet export button — gold returns to data duty only. Flows
        // with the content (the rail scrolls) instead of bottom_up,
        // which would stretch inside the ScrollArea.
        if action_button(
            ui,
            Some(ph::DOWNLOAD_SIMPLE),
            "Export calculations…",
            SURFACE_HIGH,
            TEXT,
            Some(HAIRLINE),
            40.0,
            ui.available_width(),
        ) {
            self.export();
        }
    }

    fn controls_project(&mut self, ui: &mut egui::Ui) {
        let summary = self.project.summary();
        let availability = self.project.availability().clone();
        let mut relink_digest = None;
        ui.label(title_text("Project"));
        ui.label(
            RichText::new("local lifecycle and recovery")
                .size(11.5)
                .color(TEXT_MUTE),
        );
        ui.add_space(16.0);

        if let Some((message, is_error)) = &self.project_notice {
            // Calm alert recipe (§3.3): tint fill + same-hue hairline; full
            // color only on the leading glyph.
            let (fill, stroke) = if *is_error {
                (tint_fill(DANGER), tint_hairline(DANGER))
            } else {
                (SURFACE, HAIRLINE)
            };
            Frame::NONE
                .fill(fill)
                .stroke(Stroke::new(1.0, stroke))
                .corner_radius(CornerRadius::same(R1))
                .inner_margin(Margin::same(11))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if *is_error {
                            ui.label(
                                RichText::new("!")
                                    .text_style(caption())
                                    .strong()
                                    .color(DANGER),
                            );
                        }
                        ui.label(RichText::new(message).text_style(caption()).color(TEXT_DIM));
                    });
                });
            ui.add_space(12.0);
        }

        card(ui, |ui| {
            ui.label(caps("Active document"));
            ui.add_space(8.0);
            diag(ui, "Schema", "v2 · strict", TEXT);
            diag(ui, "Cases", &summary.cases.to_string(), TEXT_DIM);
            diag(ui, "Immutable runs", &summary.runs.to_string(), BRAND);
            diag(ui, "Evidence links", &summary.evidence.to_string(), GOLD);
            diag(
                ui,
                "Verified bundle objects",
                &format!(
                    "{} / {}",
                    availability.bundle.available_objects, availability.bundle.required_objects
                ),
                if availability.bundle.diagnostics == 0 {
                    SUCCESS
                } else {
                    WARN
                },
            );
            let state = if self.project.is_recovered() {
                "RECOVERED · SAVE REQUIRED"
            } else if self.project.is_dirty() {
                "UNSAVED CHANGES"
            } else if self.project.path().is_some() {
                "SAVED LOCALLY"
            } else {
                "NO UNSAVED CHANGES"
            };
            ui.label(chip_text(state).color(if self.project.is_dirty() {
                WARN
            } else {
                SUCCESS
            }));
        });

        ui.add_space(12.0);
        let project_id = self.project.manifest().project_id().to_owned();
        let mut name_changed = false;
        card(ui, |ui| {
            ui.label(caps("Project identity"));
            ui.add_space(7.0);
            name_changed = ui
                .add(
                    egui::TextEdit::singleline(&mut self.project_name_draft)
                        .char_limit(120)
                        .desired_width(ui.available_width()),
                )
                .on_hover_text("Stored project name; edits create unsaved project metadata.")
                .changed();
            ui.label(
                RichText::new(project_id)
                    .text_style(mono_s())
                    .color(TEXT_MUTE),
            );
        });
        if name_changed {
            self.project
                .rename_project(self.project_name_draft.clone(), now_utc_unix());
            self.project_notice = Some(("Project name changed; save is required.".into(), false));
        }

        ui.add_space(12.0);
        card(ui, |ui| {
            let (title, detail, color) = if availability.is_read_only_evidence() {
                (
                    "READ-ONLY EVIDENCE MODE",
                    "Stored results remain inspectable. Compute or content dependencies must reconcile before rerun.",
                    WARN,
                )
            } else if self.engine_ok {
                (
                    "DEPENDENCIES RECONCILED",
                    "Portable content is verified and the validated compute inventory satisfies this project.",
                    SUCCESS,
                )
            } else {
                (
                    "COMPUTE UNAVAILABLE · STORAGE READY",
                    "Project files and stored evidence remain available. New engine-backed runs are unavailable.",
                    WARN,
                )
            };
            ui.label(
                RichText::new(title)
                    .strong()
                    .text_style(mono_s())
                    .color(color),
            );
            ui.label(RichText::new(detail).text_style(caption()).color(TEXT_MUTE));
        });

        if !availability.issues.is_empty() {
            ui.add_space(12.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} PRECISE DIAGNOSTIC{}",
                        availability.issues.len(),
                        if availability.issues.len() == 1 {
                            ""
                        } else {
                            "S"
                        }
                    ))
                    .strong()
                    .text_style(mono_s())
                    .color(WARN),
                );
                ui.label(
                    RichText::new(
                        "Stored runs remain immutable and inspectable; affected reruns stay unavailable.",
                    )
                    .text_style(caption())
                    .color(TEXT_MUTE),
                );
                ui.collapsing("Dependency & integrity details", |ui| {
                    for issue in availability.issues.iter().take(8) {
                        ui.label(
                            RichText::new(format!(
                                "{} · {}",
                                dependency_kind_label(issue.kind),
                                issue.detail
                            ))
                            .text_style(caption())
                            .color(
                                if issue.kind == project_lifecycle::DependencyKind::Integrity {
                                    TEXT_DIM
                                } else {
                                    WARN
                                },
                            ),
                        );
                        if issue.relinkable {
                            if let Some(digest) = issue.content_sha256.as_deref() {
                                if ui
                                    .small_button(format!(
                                        "Relink {}…",
                                        digest.get(..12).unwrap_or(digest)
                                    ))
                                    .clicked()
                                {
                                    relink_digest = Some(digest.to_owned());
                                }
                            }
                        }
                    }
                });
            });
        }

        ui.add_space(14.0);
        // Quiet save — the Projects screen's single ember action is the
        // "Import Geometry…" primary in the content region.
        if action_button(
            ui,
            Some(ph::DOWNLOAD_SIMPLE),
            "Save project",
            SURFACE_HIGH,
            TEXT,
            Some(HAIRLINE),
            40.0,
            ui.available_width(),
        ) {
            self.save_project_dialog();
        }
        ui.add_space(6.0);
        if action_button(
            ui,
            None,
            "Save As…",
            SURFACE_HIGH,
            TEXT,
            Some(OUTLINE),
            34.0,
            ui.available_width(),
        ) {
            self.save_project_as_dialog();
        }
        ui.add_space(6.0);
        if action_button(
            ui,
            Some(ph::FOLDER_OPEN),
            "Open Project…",
            Color32::TRANSPARENT,
            TEXT_DIM,
            Some(OUTLINE_VARIANT),
            34.0,
            ui.available_width(),
        ) {
            self.open_project_dialog(ui.ctx());
        }

        ui.add_space(12.0);
        ui.label(
            RichText::new(format!(
                "autosave {} s · {} recovery snapshot{}",
                self.settings.autosave_interval_seconds,
                self.project.recovery_entries().len(),
                if self.project.recovery_entries().len() == 1 {
                    ""
                } else {
                    "s"
                }
            ))
            .text_style(mono_s())
            .color(TEXT_MUTE),
        );
        if let Some(digest) = relink_digest {
            self.relink_project_content_dialog(&digest);
        }
    }

    fn project_view(&mut self, ui: &mut egui::Ui) {
        let summary = self.project.summary();
        let recent_projects = self.project.recent_projects().to_vec();
        let recovery_entries = self.project.recovery_entries().to_vec();
        let notice = self.project_notice.clone();
        let availability = self.project.availability().clone();
        let location = self
            .project
            .path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Not saved yet".into());
        let mut requested_action = None;
        let mut discard_recovery = None;
        let mut remove_recent = None;
        let mut save_current = false;
        let mut save_as = false;
        let mut open_dialog = false;
        let mut inspect_run = None;
        let mut relink_digest = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // One shared content column (§3.2, QA G3/G4).
                content_column(ui, CONTENT_MAX_WIDTH, |ui| {
                        ui.add_space(24.0);
                        // §4.2: first-run gets a deliberate landing; a
                        // returning user gets their working set first.
                        let first_run = recent_projects.is_empty()
                            && recovery_entries.is_empty()
                            && self.project.manifest().cases().is_empty()
                            && self.project.path().is_none();
                        if first_run {
                            ui.add_space(24.0);
                            ui.label(display_text("Start an analysis"));
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(
                                    "Fixed-body external flow · STL import and preprocessing. Everything stays on this machine.",
                                )
                                .text_style(caption())
                                .color(TEXT_MUTE),
                            );
                            ui.add_space(24.0);
                            let zone_width = (ui.available_width() - 24.0) / 2.0;
                            ui.horizontal_top(|ui| {
                                // Left zone: the single ember action.
                                ui.allocate_ui(Vec2::new(zone_width, 160.0), |ui| {
                                    ui.vertical(|ui| {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("Import geometry (STL)…")
                                                        .color(ON_EMBER),
                                                )
                                                .fill(EMBER)
                                                .min_size(Vec2::new(190.0, 38.0)),
                                            )
                                            .clicked()
                                        {
                                            self.import_cad();
                                        }
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new(
                                                "Import geometry, qualify the setup, run the fixed-body model, and produce traceable fluid-load evidence.",
                                            )
                                            .text_style(caption())
                                            .color(TEXT_MUTE),
                                        );
                                    });
                                });
                                ui.add_space(24.0);
                                // Right zone: quiet open + drop target.
                                ui.vertical(|ui| {
                                    ui.set_width(zone_width);
                                    if ui
                                        .add(egui::Button::new("Open project…")
                                            .min_size(Vec2::new(140.0, 30.0)))
                                        .clicked()
                                    {
                                        open_dialog = true;
                                    }
                                    ui.add_space(8.0);
                                    drop_target(ui, zone_width);
                                });
                            });
                            ui.add_space(32.0);
                            ui.label(overline_text("How Reyn works"));
                            ui.add_space(8.0);
                            // Three *text* steps in one flat group — no
                            // icon-card row (banned three-equal-cards).
                            for (step, title, body) in [
                                (
                                    "1",
                                    "Source",
                                    "Exact geometry bytes are hashed and preflighted; nothing enters the case unverified.",
                                ),
                                (
                                    "2",
                                    "Run",
                                    "The qualified model executes an immutable attempt with the full operating point recorded.",
                                ),
                                (
                                    "3",
                                    "Evidence",
                                    "Every number stays tied to its source, case revision, run, and model hash.",
                                ),
                            ] {
                                ui.horizontal_top(|ui| {
                                    ui.label(
                                        RichText::new(step)
                                            .text_style(mono_s())
                                            .color(TEXT_MUTE),
                                    );
                                    ui.add_space(8.0);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.label(
                                            RichText::new(title)
                                                .text_style(body_strong())
                                                .color(TEXT),
                                        );
                                        ui.label(
                                            RichText::new(body)
                                                .text_style(caption())
                                                .color(TEXT_MUTE),
                                        );
                                    });
                                });
                                ui.add_space(12.0);
                            }
                        } else {
                            ui.label(
                                display_text("Projects"),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(
                                    "Local project documents, real recent paths, and recoverable unsaved work.",
                                )
                                .text_style(caption())
                                .color(TEXT_MUTE),
                            );
                            ui.add_space(20.0);

                            Frame::NONE
                                .fill(SURFACE)
                                .stroke(Stroke::new(1.0, HAIRLINE))
                                .corner_radius(CornerRadius::same(R2))
                                .inner_margin(Margin::same(20))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // P1: cap the text column so the long
                                        // description wraps instead of pushing
                                        // the ember CTA past the panel edge.
                                        let text_width = (ui.available_width() - 150.0 - 24.0)
                                            .max(180.0);
                                        ui.vertical(|ui| {
                                            ui.set_max_width(text_width);
                                            ui.label(
                                                title_text("New external-flow analysis"),
                                            );
                                            ui.label(
                                                RichText::new(
                                                    "Import geometry, qualify the setup, run the fixed-body model, and produce traceable fluid-load evidence.",
                                                )
                                                .text_style(caption())
                                                .color(TEXT_MUTE),
                                            );
                                        });
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                    RichText::new("Import geometry…")
                                                        .color(ON_EMBER),
                                                )
                                                .fill(EMBER)
                                                .min_size(Vec2::new(150.0, 38.0)),
                                                )
                                                .clicked()
                                            {
                                                self.import_cad();
                                            }
                                        });
                                    });
                                });
                        }
                        ui.add_space(16.0);

                        if let Some((message, is_error)) = &notice {
                            // Calm alert recipe (§3.3).
                            let (fill, stroke) = if *is_error {
                                (tint_fill(DANGER), tint_hairline(DANGER))
                            } else {
                                (SURFACE_LOW, HAIRLINE)
                            };
                            Frame::NONE
                                .fill(fill)
                                .stroke(Stroke::new(1.0, stroke))
                                .corner_radius(CornerRadius::same(R1))
                                .inner_margin(Margin::same(12))
                                .show(ui, |ui| {
                                    ui.horizontal_top(|ui| {
                                        if *is_error {
                                            ui.label(
                                                RichText::new("!")
                                                    .size(11.0)
                                                    .strong()
                                                    .color(DANGER),
                                            );
                                        }
                                        ui.label(
                                            RichText::new(message).size(11.0).color(TEXT_DIM),
                                        );
                                    });
                                });
                            ui.add_space(14.0);
                        }

                        if !first_run {
                        Frame::NONE
                            .fill(SURFACE)
                            .stroke(Stroke::new(1.0, OUTLINE_VARIANT))
                            .corner_radius(CornerRadius::same(R2))
                            .inner_margin(Margin::same(18))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // P2: cap the identity column so long
                                    // paths elide instead of pushing the
                                    // state chip past the panel edge.
                                    let text_width =
                                        (ui.available_width() - 190.0).max(180.0);
                                    ui.vertical(|ui| {
                                        ui.set_max_width(text_width);
                                        ui.label(
                                            title_text(self.project.display_name()),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&location)
                                                    .text_style(mono_s())
                                                    .color(TEXT_MUTE),
                                            )
                                            .truncate(),
                                        )
                                        .on_hover_text(&location);
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                        let (state, color) = if self.project.is_recovered() {
                                            ("RECOVERED · UNSAVED", WARN)
                                        } else if self.project.is_dirty() {
                                            ("UNSAVED CHANGES", WARN)
                                        } else if self.project.path().is_some() {
                                            ("SAVED LOCALLY ✓", SUCCESS)
                                        } else {
                                            ("LOCAL SESSION", TEXT_MUTE)
                                        };
                                        ui.label(chip_text(state).color(color));
                                    });
                                });
                                ui.add_space(14.0);
                                ui.separator();
                                ui.add_space(10.0);
                                ui.horizontal_wrapped(|ui| {
                                    project_fact(ui, "SCHEMA", "v2");
                                    project_fact(ui, "SOURCES", &summary.sources.to_string());
                                    project_fact(ui, "CASES", &summary.cases.to_string());
                                    project_fact(ui, "IMMUTABLE RUNS", &summary.runs.to_string());
                                    project_fact(ui, "EVIDENCE", &summary.evidence.to_string());
                                    project_fact(
                                        ui,
                                        "BUNDLE OBJECTS",
                                        &format!(
                                            "{} / {} verified",
                                            availability.bundle.available_objects,
                                            availability.bundle.required_objects
                                        ),
                                    );
                                });
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    // Quiet save — the landing's single ember
                                    // action is "Import geometry…" above.
                                    if ui.button("Save project").clicked() {
                                        save_current = true;
                                    }
                                    if ui.button("Save As…").clicked() {
                                        save_as = true;
                                    }
                                    if ui.button("New Project").clicked() {
                                        requested_action =
                                            Some(project_lifecycle::DeferredProjectAction::New);
                                    }
                                    if ui.button("Open Project…").clicked() {
                                        open_dialog = true;
                                    }
                                });
                            });
                        }

                        if !availability.issues.is_empty() {
                            ui.add_space(14.0);
                            Frame::NONE
                                .fill(tint_fill(WARN))
                                .stroke(Stroke::new(1.0, tint_hairline(WARN)))
                                .corner_radius(CornerRadius::same(R1))
                                .inner_margin(Margin::same(14))
                                .show(ui, |ui| {
                                    ui.label(
                                        chip_text(if availability.is_read_only_evidence() {
                                            "READ-ONLY EVIDENCE · DEPENDENCIES UNAVAILABLE"
                                        } else {
                                            "BUNDLE INTEGRITY NOTICE"
                                        })
                                        .color(WARN),
                                    );
                                    ui.label(
                                        RichText::new(
                                            "Stored manifests, calibrated views, warnings, runs, and valid evidence remain inspectable. Reconciliation never rewrites a completed run.",
                                        )
                                        .size(11.0)
                                        .color(TEXT_DIM),
                                    );
                                    ui.collapsing("Dependency & integrity details", |ui| {
                                        for issue in availability.issues.iter().take(12) {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} · {}",
                                                        dependency_kind_label(issue.kind),
                                                        issue.detail
                                                    ))
                                                    .text_style(caption())
                                                    .color(if issue.kind
                                                        == project_lifecycle::DependencyKind::Integrity
                                                    {
                                                        TEXT_DIM
                                                    } else {
                                                        WARN
                                                    }),
                                                );
                                                if issue.relinkable {
                                                    if let Some(digest) =
                                                        issue.content_sha256.as_deref()
                                                    {
                                                        if ui
                                                            .small_button(format!(
                                                                "Relink {}…",
                                                                digest
                                                                    .get(..12)
                                                                    .unwrap_or(digest)
                                                            ))
                                                            .clicked()
                                                        {
                                                            relink_digest =
                                                                Some(digest.to_owned());
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    });
                                });
                        }

                        if !self.project.manifest().cases().is_empty() {
                            ui.add_space(24.0);
                            ui.label(caps("Cases and immutable run history"));
                            ui.label(
                                RichText::new(
                                    "Stored values open their exact run and evidence; stale lineage never rewrites a completed attempt.",
                                )
                                .text_style(caption())
                                .color(TEXT_MUTE),
                            );
                            ui.add_space(8.0);
                            for case in self.project.manifest().cases() {
                                Frame::NONE
                                    .fill(SURFACE_LOW)
                                    .stroke(Stroke::new(1.0, OUTLINE_VARIANT))
                                    .corner_radius(CornerRadius::same(3))
                                    .inner_margin(Margin::same(14))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    RichText::new(case.name())
                                                        .strong()
                                                        .size(13.0)
                                                        .color(TEXT),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} revisions · active {}",
                                                        case.revisions().len(),
                                                        short_id(case.active_revision_id())
                                                    ))
                                                    .text_style(mono_s())
                                                    .color(TEXT_MUTE),
                                                );
                                            });
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if case.stale_stages().is_empty() {
                                                        ui.label(
                                                            RichText::new("DEPENDENCIES CURRENT")
                                                                .strong().text_style(mono_s())
                                                                .color(SUCCESS),
                                                        );
                                                    } else {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "STALE · {}",
                                                                format_stages(
                                                                    case.stale_stages()
                                                                )
                                                            ))
                                                            .strong().text_style(mono_s())
                                                            .color(WARN),
                                                        );
                                                    }
                                                },
                                            );
                                        });
                                        for source_id in
                                            &case.active_revision().source_revision_ids
                                        {
                                            if let Some(source) = self
                                                .project
                                                .manifest()
                                                .source_revisions()
                                                .iter()
                                                .find(|source| {
                                                    source.source_revision_id == *source_id
                                                })
                                            {
                                                let (content_state, content_color) =
                                                    content_state_presentation(
                                                        self.project.content_state(
                                                            &source.content_sha256,
                                                        ),
                                                    );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} source · revision {} · sha256 {} · {}",
                                                        source_kind_label(source.source_kind),
                                                        source.revision,
                                                        short_hash(&source.content_sha256),
                                                        content_state
                                                    ))
                                                    .text_style(mono_s())
                                                    .color(content_color),
                                                );
                                                for warning in source.warnings.iter().take(2) {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "Source warning · {warning}"
                                                        ))
                                                        .text_style(caption())
                                                        .color(WARN),
                                                    );
                                                }
                                            }
                                        }
                                        ui.add_space(8.0);
                                        if case.runs().is_empty() {
                                            ui.label(
                                                RichText::new(
                                                    "No completed run yet. The configured case remains local and saveable.",
                                                )
                                                .text_style(caption())
                                                .color(TEXT_MUTE),
                                            );
                                        }
                                        for run in case.runs().iter().rev() {
                                            let evidence =
                                                self.project.manifest().evidence_for_run(run.run_id());
                                            let stale = case
                                                .stale_stages_for_run(run.run_id())
                                                .unwrap_or_default();
                                            Frame::NONE
                                                .fill(Color32::TRANSPARENT)
                                                .stroke(Stroke::new(
                                                    1.0,
                                                    OUTLINE_VARIANT.gamma_multiply(0.75),
                                                ))
                                                .corner_radius(CornerRadius::same(3))
                                                .inner_margin(Margin::symmetric(11, 9))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.vertical(|ui| {
                                                            let state = if stale.contains(
                                                                &project::DependencyStage::Run,
                                                            ) {
                                                                "STALE INPUT LINEAGE"
                                                            } else {
                                                                "COMPLETE · IMMUTABLE"
                                                            };
                                                            ui.label(
                                                                RichText::new(format!(
                                                                    "{} · {}",
                                                                    short_id(run.run_id()),
                                                                    state
                                                                ))
                                                                .strong().text_style(mono_s())
                                                                .color(if stale.contains(
                                                                    &project::DependencyStage::Run,
                                                                ) {
                                                                    WARN
                                                                } else {
                                                                    BRAND
                                                                }),
                                                            );
                                                            let model_hash = run
                                                                .manifest()
                                                                .model
                                                                .as_ref()
                                                                .and_then(|model| {
                                                                    model.sha256.as_deref()
                                                                })
                                                                .map(short_hash)
                                                                .unwrap_or_else(|| "UNKNOWN".into());
                                                            ui.label(
                                                                RichText::new(format!(
                                                                    "model {model_hash} · {} scalar{} · {} evidence link{}",
                                                                    run.manifest().scalar_outputs.len(),
                                                                    if run.manifest().scalar_outputs.len() == 1 { "" } else { "s" },
                                                                    evidence.len(),
                                                                    if evidence.len() == 1 { "" } else { "s" },
                                                                ))
                                                                .text_style(mono_s())
                                                                .color(TEXT_MUTE),
                                                            );
                                                            if let Some(determinism) =
                                                                &run.manifest().determinism
                                                            {
                                                                ui.label(
                                                                    RichText::new(format!(
                                                                        "rerun of {} · {}",
                                                                        short_id(
                                                                            &determinism
                                                                                .parent_run_id
                                                                        ),
                                                                        determinism_status_label(
                                                                            determinism.status
                                                                        )
                                                                    ))
                                                                    .text_style(mono_s())
                                                                    .color(
                                                                        determinism_status_color(
                                                                            determinism.status,
                                                                        ),
                                                                    ),
                                                                );
                                                                if determinism.status
                                                                    != project::DeterminismStatus::WithinTolerance
                                                                {
                                                                    for difference in determinism
                                                                        .differences
                                                                        .iter()
                                                                        .filter(|difference| {
                                                                            match (
                                                                                difference.abs_difference,
                                                                                difference.abs_tolerance,
                                                                            ) {
                                                                                (Some(delta), Some(tolerance)) => {
                                                                                    delta > tolerance
                                                                                }
                                                                                _ => true,
                                                                            }
                                                                        })
                                                                        .take(2)
                                                                    {
                                                                        ui.label(
                                                                            RichText::new(format!(
                                                                                "{} · parent {} · rerun {} · |Δ| {} · tol {}",
                                                                                difference.key,
                                                                                optional_scalar(
                                                                                    difference
                                                                                        .parent_value
                                                                                ),
                                                                                optional_scalar(
                                                                                    difference
                                                                                        .current_value
                                                                                ),
                                                                                optional_scalar(
                                                                                    difference
                                                                                        .abs_difference
                                                                                ),
                                                                                optional_scalar(
                                                                                    difference
                                                                                        .abs_tolerance
                                                                                ),
                                                                            ))
                                                                            .text_style(mono_s())
                                                                            .color(DATA_RED),
                                                                        );
                                                                    }
                                                                }
                                                            }
                                                            for warning in
                                                                run.manifest().warnings.iter().take(2)
                                                            {
                                                                ui.label(
                                                                    RichText::new(format!(
                                                                        "Warning · {warning}"
                                                                    ))
                                                                    .text_style(caption())
                                                                    .color(WARN),
                                                                );
                                                            }
                                                            for artifact in evidence.iter().take(3)
                                                            {
                                                                let (
                                                                    content_state,
                                                                    content_color,
                                                                ) = content_state_presentation(
                                                                    self.project.content_state(
                                                                        &artifact.content_sha256,
                                                                    ),
                                                                );
                                                                ui.label(
                                                                    RichText::new(format!(
                                                                        "Evidence {} · {} · sha256 {} · {}",
                                                                        short_id(
                                                                            &artifact.evidence_id
                                                                        ),
                                                                        artifact.media_type,
                                                                        short_hash(
                                                                            &artifact
                                                                                .content_sha256
                                                                        ),
                                                                        content_state
                                                                    ))
                                                                    .text_style(mono_s())
                                                                    .color(content_color),
                                                                );
                                                            }
                                                        });
                                                        ui.with_layout(
                                                            Layout::right_to_left(Align::Center),
                                                            |ui| {
                                                                if ui
                                                                    .add_enabled(
                                                                        !evidence.is_empty(),
                                                                        egui::Button::new(
                                                                            "Inspect evidence",
                                                                        ),
                                                                    )
                                                                    .on_disabled_hover_text(
                                                                        "This run has no stored evidence artifact",
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    inspect_run =
                                                                        Some(run.run_id().to_owned());
                                                                }
                                                            },
                                                        );
                                                    });
                                                });
                                            ui.add_space(6.0);
                                        }
                                    });
                                ui.add_space(8.0);
                            }
                        }

                        if !recovery_entries.is_empty() {
                            ui.add_space(24.0);
                            ui.label(caps("Crash recovery"));
                            ui.label(
                                RichText::new(
                                    "Snapshots are separate from explicit saves and never overwrite a project until you choose Save.",
                                )
                                .text_style(caption())
                                .color(TEXT_MUTE),
                            );
                            ui.add_space(8.0);
                            for recovery in &recovery_entries {
                                // §4.2: recovery rows wear the warn recipe —
                                // tinted fill + hairline, not a gold border.
                                Frame::NONE
                                    .fill(tint_fill(WARN))
                                    .stroke(Stroke::new(1.0, tint_hairline(WARN)))
                                    .corner_radius(CornerRadius::same(R1))
                                    .inner_margin(Margin::symmetric(14, 11))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let text_width =
                                                (ui.available_width() - 190.0).max(160.0);
                                            ui.vertical(|ui| {
                                                ui.set_max_width(text_width);
                                                ui.spacing_mut().item_spacing.y = 2.0;
                                                ui.label(
                                                    RichText::new(&recovery.name)
                                                        .text_style(body_strong())
                                                        .color(TEXT),
                                                );
                                                let source = recovery
                                                    .source_path
                                                    .as_ref()
                                                    .map(|path| path.display().to_string())
                                                    .unwrap_or_else(|| {
                                                        "Unsaved project · no original path".into()
                                                    });
                                                let detail = format!(
                                                    "{source} · autosaved {}",
                                                    format_utc(recovery.saved_utc_unix)
                                                );
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(&detail)
                                                            .text_style(mono_s())
                                                            .color(TEXT_MUTE),
                                                    )
                                                    .truncate(),
                                                )
                                                .on_hover_text(&detail);
                                            });
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if ui.button("Discard").clicked() {
                                                        discard_recovery =
                                                            Some(recovery.project_id.clone());
                                                    }
                                                    if ui
                                                        .button("Recover")
                                                        .clicked()
                                                    {
                                                        requested_action = Some(
                                                            project_lifecycle::DeferredProjectAction::Recover(
                                                                recovery.project_id.clone(),
                                                            ),
                                                        );
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(6.0);
                            }
                        }

                        if !first_run {
                            ui.add_space(24.0);
                            ui.label(overline_text("Recent projects"));
                            ui.label(
                                RichText::new(
                                    "Machine-local shortcuts only; portable manifests rely on content hashes, not these paths.",
                                )
                                .text_style(caption())
                                .color(TEXT_MUTE),
                            );
                            ui.add_space(8.0);
                            if recent_projects.is_empty() {
                                ui.label(
                                    RichText::new(
                                        "No recent projects. Save or open a real .reynproj document to add one.",
                                    )
                                    .text_style(caption())
                                    .color(TEXT_MUTE),
                                );
                            }
                            // §4.2: recents as level-0 rows — name
                            // body-strong, path mono-s, opened-when caption,
                            // hover bg-3, no per-row boxes.
                            for recent in &recent_projects {
                                let available = recent.path.is_file();
                                // Reserve a shape slot so the hover wash
                                // paints *behind* the row content.
                                let wash = ui.painter().add(egui::Shape::Noop);
                                let row = Frame::NONE
                                    .inner_margin(Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width() - 20.0);
                                        ui.horizontal(|ui| {
                                            // P6: cap the name/path column so
                                            // long paths elide instead of
                                            // pushing the row past the frame.
                                            let text_width =
                                                (ui.available_width() - 250.0).max(160.0);
                                            ui.vertical(|ui| {
                                                ui.set_max_width(text_width);
                                                ui.spacing_mut().item_spacing.y = 2.0;
                                                ui.label(
                                                    RichText::new(&recent.name)
                                                        .text_style(body_strong())
                                                        .color(if available {
                                                            TEXT
                                                        } else {
                                                            TEXT_MUTE
                                                        }),
                                                );
                                                let path_text =
                                                    recent.path.display().to_string();
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(&path_text)
                                                            .text_style(mono_s())
                                                            .color(TEXT_MUTE),
                                                    )
                                                    .truncate(),
                                                )
                                                .on_hover_text(&path_text);
                                            });
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if ui.button("Remove").clicked() {
                                                        remove_recent = Some(recent.path.clone());
                                                    }
                                                    if ui
                                                        .add_enabled(
                                                            available,
                                                            egui::Button::new("Open"),
                                                        )
                                                        .on_disabled_hover_text(
                                                            "The file no longer exists at this path.",
                                                        )
                                                        .clicked()
                                                    {
                                                        requested_action = Some(
                                                            project_lifecycle::DeferredProjectAction::Open(
                                                                recent.path.clone(),
                                                            ),
                                                        );
                                                    }
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "opened {}",
                                                            format_utc(
                                                                recent.last_opened_utc_unix
                                                            )
                                                        ))
                                                        .text_style(caption())
                                                        .color(TEXT_MUTE),
                                                    );
                                                },
                                            );
                                        });
                                    })
                                    .response;
                                // Hover wash + hairline: the row grammar.
                                if ui.rect_contains_pointer(row.rect) {
                                    ui.painter().set(
                                        wash,
                                        egui::Shape::rect_filled(
                                            row.rect,
                                            CornerRadius::same(R1),
                                            SURFACE_HIGH,
                                        ),
                                    );
                                }
                                ui.painter().hline(
                                    row.rect.x_range(),
                                    row.rect.max.y + 1.0,
                                    Stroke::new(1.0, HAIRLINE),
                                );
                                ui.add_space(2.0);
                            }
                        }
                        ui.add_space(24.0);
                    });
            });

        // Landing drop target (§4.2): dropped STL geometry starts an import,
        // a dropped .reynproj opens through the same deferred (dirty-guarded)
        // path as the Open dialog. Anything else states why it was refused.
        let dropped: Vec<std::path::PathBuf> = ui.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect()
        });
        for path in dropped {
            match path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("stl") => self.import_cad_path(path),
                Some("reynproj") => {
                    requested_action = Some(project_lifecycle::DeferredProjectAction::Open(path));
                }
                _ => {
                    self.project_notice = Some((
                        "Only STL geometry or .reynproj documents can be dropped here.".into(),
                        true,
                    ));
                }
            }
        }

        if save_current {
            self.save_project_dialog();
        }
        if save_as {
            self.save_project_as_dialog();
        }
        if open_dialog {
            self.open_project_dialog(ui.ctx());
        }
        if let Some(project_id) = discard_recovery {
            match self.project.discard_recovery(&project_id) {
                Ok(()) => {
                    self.project_notice = Some(("Recovery snapshot discarded.".into(), false));
                }
                Err(error) => {
                    self.project_notice = Some((
                        format!("Recovery snapshot was not discarded: {error}"),
                        true,
                    ));
                }
            }
        }
        if let Some(path) = remove_recent {
            match self.project.remove_recent(&path) {
                Ok(()) => {
                    self.project_notice = Some(("Recent shortcut removed.".into(), false));
                }
                Err(error) => {
                    self.project_notice =
                        Some((format!("Recent shortcut was not removed: {error}"), true));
                }
            }
        }
        if let Some(action) = requested_action {
            self.request_project_action(action, ui.ctx());
        }
        if let Some(run_id) = inspect_run {
            self.inspect_project_run(&run_id);
        }
        if let Some(digest) = relink_digest {
            self.relink_project_content_dialog(&digest);
        }
    }

    fn handle_project_shortcuts(&mut self, ui: &mut egui::Ui) {
        enum Shortcut {
            New,
            Open,
            Save,
            SaveAs,
            Close,
            Quit,
        }
        let shortcut = ui.input_mut(|input| {
            if input.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::S,
            ) {
                Some(Shortcut::SaveAs)
            } else if input.consume_key(egui::Modifiers::COMMAND, egui::Key::S) {
                Some(Shortcut::Save)
            } else if input.consume_key(egui::Modifiers::COMMAND, egui::Key::O) {
                Some(Shortcut::Open)
            } else if input.consume_key(egui::Modifiers::COMMAND, egui::Key::N) {
                Some(Shortcut::New)
            } else if input.consume_key(egui::Modifiers::COMMAND, egui::Key::W) {
                Some(Shortcut::Close)
            } else if input.consume_key(egui::Modifiers::COMMAND, egui::Key::Q) {
                Some(Shortcut::Quit)
            } else {
                None
            }
        });
        match shortcut {
            Some(Shortcut::New) => {
                self.request_project_action(project_lifecycle::DeferredProjectAction::New, ui.ctx())
            }
            Some(Shortcut::Open) => self.open_project_dialog(ui.ctx()),
            Some(Shortcut::Save) => {
                self.save_project_dialog();
            }
            Some(Shortcut::SaveAs) => {
                self.save_project_as_dialog();
            }
            // Both route through the unsaved-changes guard, never a raw exit.
            Some(Shortcut::Close) | Some(Shortcut::Quit) => self
                .request_project_action(project_lifecycle::DeferredProjectAction::Quit, ui.ctx()),
            None => {}
        }
    }

    fn open_project_dialog(&mut self, ctx: &egui::Context) {
        let mut dialog = rfd::FileDialog::new().add_filter("Reyn project", &["reynproj"]);
        if let Some(directory) = self.project_dialog_directory() {
            dialog = dialog.set_directory(directory);
        }
        let Some(path) = dialog.pick_file() else {
            return;
        };
        self.request_project_action(project_lifecycle::DeferredProjectAction::Open(path), ctx);
    }

    fn save_project_dialog(&mut self) -> bool {
        if self.project.path().is_none() {
            return self.save_project_as_dialog();
        }
        let path = self
            .project
            .path()
            .expect("project path checked above")
            .to_path_buf();
        match self.project.save(now_utc_unix()) {
            Ok(warning) => {
                self.project_name_draft = self.project.display_name().to_owned();
                self.project_notice = Some((
                    warning.unwrap_or_else(|| format!("Saved atomically to {}", path.display())),
                    false,
                ));
                true
            }
            Err(error) => {
                self.project_notice = Some((format!("Project was not saved: {error}"), true));
                false
            }
        }
    }

    fn save_project_as_dialog(&mut self) -> bool {
        let mut dialog = rfd::FileDialog::new().add_filter("Reyn project", &["reynproj"]);
        if let Some(directory) = self.project_dialog_directory() {
            dialog = dialog.set_directory(directory);
        }
        if self.project.display_name() != "Unsaved project" {
            dialog = dialog.set_file_name(format!("{}.reynproj", self.project.display_name()));
        }
        let Some(path) = dialog.save_file() else {
            return false;
        };
        let path = with_project_extension(path);
        match self.project.save_as(&path, now_utc_unix()) {
            Ok(warning) => {
                self.project_name_draft = self.project.display_name().to_owned();
                self.project_notice = Some((
                    warning.unwrap_or_else(|| {
                        format!("Saved a new atomic project at {}", path.display())
                    }),
                    false,
                ));
                true
            }
            Err(error) => {
                self.project_notice = Some((
                    format!("Project was not saved as {}: {error}", path.display()),
                    true,
                ));
                false
            }
        }
    }

    fn relink_project_content_dialog(&mut self, expected_digest: &str) {
        let mut dialog = rfd::FileDialog::new();
        if let Some(directory) = self.project_dialog_directory() {
            dialog = dialog.set_directory(directory);
        }
        let Some(path) = dialog.pick_file() else {
            return;
        };
        match self.project.relink_content(expected_digest, &path) {
            Ok(insert) => {
                self.project_notice = Some((
                    format!(
                        "{} content {} from {}. Immutable runs and evidence records were not changed; save the project to retain the portable object.",
                        if insert.deduplicated {
                            "Verified existing"
                        } else {
                            "Relinked"
                        },
                        short_hash(&insert.content_sha256),
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("selected file")
                    ),
                    false,
                ));
            }
            Err(error) => {
                self.project_notice = Some((
                    format!(
                        "Relink rejected for {}: {error}. The project was not changed.",
                        short_hash(expected_digest)
                    ),
                    true,
                ));
            }
        }
    }

    fn project_dialog_directory(&self) -> Option<std::path::PathBuf> {
        self.project
            .path()
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf)
            .or_else(|| {
                let configured = std::path::PathBuf::from(&self.settings.project_directory);
                configured.is_dir().then_some(configured)
            })
    }

    fn request_project_action(
        &mut self,
        action: project_lifecycle::DeferredProjectAction,
        ctx: &egui::Context,
    ) {
        match self.project_guard.request(action, self.project.is_dirty()) {
            project_lifecycle::GuardRequest::Execute(action) => {
                self.execute_project_action(action, ctx);
            }
            project_lifecycle::GuardRequest::ConfirmationRequired => {}
        }
    }

    fn execute_project_action(
        &mut self,
        action: project_lifecycle::DeferredProjectAction,
        ctx: &egui::Context,
    ) {
        match action {
            project_lifecycle::DeferredProjectAction::New => {
                self.project.new_project(now_utc_unix());
                self.project_name_draft = self.project.display_name().to_owned();
                self.reset_project_runtime();
                self.restart_engine_for_project_change();
                self.nav = Nav::Projects;
                self.project_notice = Some((
                    "New local project created. Choose Save to assign its real name and location."
                        .into(),
                    false,
                ));
            }
            project_lifecycle::DeferredProjectAction::Open(path) => {
                match self.project.open(&path, now_utc_unix()) {
                    Ok(warning) => {
                        self.project_name_draft = self.project.display_name().to_owned();
                        self.reset_project_runtime();
                        self.hydrate_project_runtime();
                        self.restart_engine_for_project_change();
                        self.project_notice = Some((
                            warning.unwrap_or_else(|| {
                                format!(
                                    "Opened {} without requiring the compute engine.",
                                    path.display()
                                )
                            }),
                            false,
                        ));
                    }
                    Err(error) => {
                        self.project_notice = Some((
                            format!(
                                "Could not open {}: {error}. The current project was not changed.",
                                path.display()
                            ),
                            true,
                        ));
                        self.nav = Nav::Projects;
                    }
                }
            }
            project_lifecycle::DeferredProjectAction::Recover(project_id) => {
                match self.project.recover(&project_id, now_utc_unix()) {
                    Ok(()) => {
                        self.project_name_draft = self.project.display_name().to_owned();
                        self.reset_project_runtime();
                        self.hydrate_project_runtime();
                        self.restart_engine_for_project_change();
                        self.project_notice = Some((
                            "Recovered unsaved work. The original project has not been overwritten; Save is required."
                                .into(),
                            false,
                        ));
                    }
                    Err(error) => {
                        self.project_notice =
                            Some((format!("Recovery could not be opened: {error}"), true));
                    }
                }
            }
            project_lifecycle::DeferredProjectAction::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn show_unsaved_changes_dialog(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.project_guard.pending() else {
            return;
        };
        let target = match pending {
            project_lifecycle::DeferredProjectAction::New => "create a new project",
            project_lifecycle::DeferredProjectAction::Open(_) => "open another project",
            project_lifecycle::DeferredProjectAction::Recover(_) => {
                "replace this session with recovered work"
            }
            project_lifecycle::DeferredProjectAction::Quit => "quit Reyn Studio",
        };
        let mut decision = None;
        egui::Window::new("Unsaved project changes")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(430.0, 0.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!(
                        "Save {} before you {target}?",
                        self.project.display_name()
                    ))
                    .size(13.0)
                    .color(TEXT),
                );
                ui.label(
                    RichText::new(
                        "Save writes atomically. Discard removes this session’s recovery snapshot. Cancel leaves everything unchanged.",
                    )
                    .text_style(caption())
                    .color(TEXT_MUTE),
                );
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        decision = Some(project_lifecycle::UnsavedDecision::Cancel);
                    }
                    if ui.button("Discard changes").clicked() {
                        decision = Some(project_lifecycle::UnsavedDecision::Discard);
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Save").color(ON_EMBER)).fill(EMBER),
                        )
                        .clicked()
                    {
                        decision = Some(project_lifecycle::UnsavedDecision::Save);
                    }
                });
            });
        let Some(decision) = decision else {
            return;
        };
        match self.project_guard.resolve(decision) {
            project_lifecycle::GuardResolution::SaveThen(action) => {
                if self.save_project_dialog() {
                    self.execute_project_action(action, ctx);
                }
            }
            project_lifecycle::GuardResolution::Execute(action) => {
                match self.project.discard_active_recovery() {
                    Ok(()) => self.execute_project_action(action, ctx),
                    Err(error) => {
                        self.project_notice = Some((
                            format!(
                                "Unsaved work was not discarded because its recovery snapshot could not be removed: {error}"
                            ),
                            true,
                        ));
                    }
                }
            }
            project_lifecycle::GuardResolution::Cancelled
            | project_lifecycle::GuardResolution::Idle => {}
        }
    }

    fn invalidate_engine_results(&mut self) {
        self.invalidate_cad_section();
        self.particles.clear();
        self.volume_data = std::sync::Arc::new(vec![0; 8]);
        self.volume_dims = [2, 2, 2];
        self.volume_version = self.volume_version.wrapping_add(1);
        self.render_volume = false;
        self.insights3d.clear();
        self.f2d = None;
        self.f2d_tex.clear();
        self.f2d_sig = u64::MAX;
        self.f2d_dirty = false;
        self.f2d_pending = false;
        self.f2d_req_at = None;
        self.bench = None;
        self.bench_running = false;
        self.bench_selected = None;
        self.bench_inspector = None;
        self.bench_inspector_pending = false;
        self.bench_error = None;
        self.bench_inspector_error = None;
        self.bench_tex.clear();
        self.active_benchmark_run_id = None;
        if let Some(cad) = &mut self.cad {
            cad.surf = None;
        }
        self.surface_on = false;
    }

    fn reset_project_runtime(&mut self) {
        self.invalidate_engine_results();
        self.live = false;
        self.live_timer = 0.0;
        self.seed = 1;
        self.current_model = "flow3d_obs_v1.pth".into();
        self.f2d_model = "obstacle_v2_shapes.pth".into();
        self.f2d_var = FieldVar::Vorticity;
        self.f2d_horizon = 8;
        self.f2d_truth = false;
        self.f2d_method = PMethod::Spectral;
        self.f2d_tol_exp = 5;
        self.f2d_boundary = PBoundary::Periodic;
        self.f2d_scale = 1.0;
        self.f2d_signed = true;
        self.cad = None;
        self.f2d_painted = None;
        self.paint = painter::PaintField::default();
        self.paint_tex = None;
        self.paint_dirty = true;
        self.paint_last = None;
        self.bench_seeds = 3;
        self.bench_seed_start = 70000;
        self.bench_var = InspectorVariable::Velocity;
        self.reset_controls();
        self.cam = viewport::Camera::default();
    }

    fn restart_engine_for_project_change(&mut self) {
        // Replacing the handle drops the prior receiver, so an in-flight result
        // from the previous project cannot repopulate a cleared viewport.
        self.engine = engine::EngineHandle::spawn_with_config(self.settings.engine_config());
        self.engine_ok = false;
        self.engine_status = "○ Project context changed · revalidating engine…".into();
        self.library.busy = true;
        let _ = self.engine.tx.send(engine::Cmd::ListModels);
    }

    fn prepare_benchmark_case(&mut self, clear_selection: bool) -> Result<String, String> {
        let model = self
            .models
            .iter()
            .find(|model| model.id == self.f2d_model)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "model {} is not present in the validated inventory",
                    self.f2d_model
                )
            })?;
        if !is_sha256(&model.checkpoint_sha256) {
            return Err(format!(
                "{} has no valid checkpoint SHA-256; an evidence run cannot be created",
                model.name
            ));
        }
        let now = now_utc_unix();
        let checkpoint_sha256 = model.checkpoint_sha256.to_ascii_lowercase();
        let existing_source = self
            .project
            .manifest()
            .source_by_digest(&checkpoint_sha256)
            .cloned();
        let bundled_model_bytes = if self.project.content_bytes(&checkpoint_sha256).is_some() {
            None
        } else {
            let model_id_path = std::path::PathBuf::from(&model.id);
            let model_path = if model_id_path.is_absolute() {
                model_id_path
            } else {
                std::path::PathBuf::from(&self.settings.research_dir).join(model_id_path)
            };
            let bytes = std::fs::read(&model_path).map_err(|error| {
                format!(
                    "checkpoint {} cannot be bundled from {}: {error}",
                    model.name,
                    model_path.display()
                )
            })?;
            let actual = project_sha256(&bytes);
            if actual != checkpoint_sha256 {
                return Err(format!(
                    "checkpoint {} changed after validation: expected {}, received {}",
                    model.name,
                    short_hash(&checkpoint_sha256),
                    short_hash(&actual)
                ));
            }
            Some(bytes)
        };
        let source_id = existing_source
            .as_ref()
            .map(|source| source.source_revision_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let parent_source = self
            .project
            .manifest()
            .source_revisions()
            .iter()
            .rev()
            .find(|source| {
                source.source_kind == project::SourceKind::Model
                    && source.uri_hint.as_deref() == Some(model.name.as_str())
            })
            .cloned();
        let case_id = self
            .project
            .manifest()
            .case_by_contract_kind("benchmark_suite")
            .map(|case| case.case_id().to_owned())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let contract = serde_json::json!({
            "kind": "benchmark_suite",
            "physics_contract": model.physics_contract,
            "model_id": model.id,
            "model_checkpoint_sha256": checkpoint_sha256,
            "model_source_sha256": model.source_digest,
            "dimension": model.dimension,
            "input_channels": model.in_channels,
            "output_channels": model.out_channels,
            "maximum_supported_horizon": model.max_steps,
            "reference_semantics": "numerical solver reference",
        });
        let discretization = serde_json::json!({
            "grid": model.grid,
            "dimension": model.dimension,
            "domain": INSPECTOR_DOMAIN,
        });
        let horizons = vec![1_u32, 4, 8, 16];
        let seeds: Vec<u32> = (0..self.bench_seeds)
            .map(|offset| self.bench_seed_start.saturating_add(offset))
            .collect();
        let outputs = serde_json::json!({
            "seeds": seeds,
            "horizons": horizons,
            "scalars": ["relative_l2", "persistence_relative_l2", "improvement_ratio"],
            "selected_cell_evidence_schema": INSPECTOR_SCHEMA,
        });
        let checkpoint_sha256_for_source = checkpoint_sha256.clone();
        let new_source = existing_source.is_none().then(|| {
            let mut warnings = model.unknown_fields.clone();
            if model.status != "clean" && !model.status_detail.trim().is_empty() {
                warnings.push(model.status_detail.clone());
            }
            warnings.sort();
            warnings.dedup();
            project::SourceRevision {
                source_revision_id: source_id.clone(),
                source_kind: project::SourceKind::Model,
                revision: parent_source
                    .as_ref()
                    .map_or(1, |parent| parent.revision.saturating_add(1)),
                imported_utc_unix: now,
                uri_hint: Some(model.name.clone()),
                byte_size: model.size_bytes,
                content_sha256: checkpoint_sha256_for_source,
                declared_units: None,
                frame: None,
                transform_4x4: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
                parent_revision_id: parent_source.map(|parent| parent.source_revision_id),
                warnings,
            }
        });
        let existing_case = self.project.manifest().case(&case_id).cloned();
        let revision_id = uuid::Uuid::new_v4().to_string();
        let source_id_for_edit = source_id.clone();
        let case_id_for_edit = case_id.clone();
        self.project
            .transact(now, move |manifest| {
                if let Some(source) = new_source {
                    manifest.add_source_revision(source, now)?;
                }
                if let Some(case) = existing_case {
                    let active = case.active_revision();
                    if active.source_revision_ids != [source_id_for_edit.clone()]
                        || active.contract != contract
                        || active.discretization != discretization
                        || active.outputs != outputs
                    {
                        manifest.append_case_revision(
                            &case_id_for_edit,
                            project::CaseRevision {
                                case_revision_id: revision_id,
                                parent_revision_id: Some(active.case_revision_id.clone()),
                                created_utc_unix: now,
                                source_revision_ids: vec![source_id_for_edit],
                                contract,
                                discretization,
                                outputs,
                            },
                            now,
                        )?;
                    }
                } else {
                    manifest.create_case(
                        case_id_for_edit.clone(),
                        "Model qualification",
                        project::CaseRevision {
                            case_revision_id: revision_id,
                            parent_revision_id: None,
                            created_utc_unix: now,
                            source_revision_ids: vec![source_id_for_edit],
                            contract,
                            discretization,
                            outputs,
                        },
                        now,
                    )?;
                }
                if clear_selection {
                    manifest.set_selection(
                        project::ProjectSelection {
                            active_case_id: Some(case_id_for_edit.clone()),
                            ..Default::default()
                        },
                        now,
                    )?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        if let Some(bytes) = bundled_model_bytes {
            self.project
                .add_content_with_digest(
                    bytes,
                    "application/vnd.pytorch.checkpoint",
                    &checkpoint_sha256,
                )
                .map_err(|error| format!("checkpoint bundle: {error}"))?;
        }
        Ok(case_id)
    }

    fn persist_benchmark_run(&mut self, benchmark: &engine::BenchResult) -> Result<String, String> {
        let case_id = self.prepare_benchmark_case(false)?;
        let now = now_utc_unix();
        let model = self
            .models
            .iter()
            .find(|model| model.id == benchmark.model)
            .cloned()
            .ok_or_else(|| format!("model {} left the validated inventory", benchmark.model))?;
        let case = self
            .project
            .manifest()
            .case(&case_id)
            .cloned()
            .ok_or_else(|| "benchmark case was not created".to_string())?;
        let active_revision = case.active_revision().clone();
        let parent = case.latest_run_for_active_revision().cloned();
        let snapshot =
            serde_json::to_value(benchmark).map_err(|error| format!("suite snapshot: {error}"))?;
        let snapshot_bytes =
            serde_json::to_vec(&snapshot).map_err(|error| format!("suite snapshot: {error}"))?;
        let output_sha256 = project_sha256(&snapshot_bytes);
        let mut scalar_outputs = vec![project::ScalarOutput {
            key: "global_relative_l2".into(),
            value: benchmark.global_rel as f64,
            units: "dimensionless".into(),
            abs_tolerance: 1e-6,
        }];
        for (seed_index, seed) in benchmark.seeds.iter().enumerate() {
            for (horizon_index, horizon) in benchmark.horizons.iter().enumerate() {
                scalar_outputs.push(project::ScalarOutput {
                    key: format!("seed_{seed}.horizon_{horizon}.relative_l2"),
                    value: benchmark.rel[seed_index][horizon_index] as f64,
                    units: "dimensionless".into(),
                    abs_tolerance: 1e-6,
                });
                scalar_outputs.push(project::ScalarOutput {
                    key: format!("seed_{seed}.horizon_{horizon}.persistence_relative_l2"),
                    value: benchmark.persist[seed_index][horizon_index] as f64,
                    units: "dimensionless".into(),
                    abs_tolerance: 1e-6,
                });
            }
        }
        let mut warnings = benchmark.provenance.flags.clone();
        warnings.extend(
            benchmark
                .provenance
                .legacy_unknown
                .iter()
                .map(|field| format!("UNKNOWN provenance field: {field}")),
        );
        if benchmark.provenance.verdict == "unknown" {
            warnings.push(
                "RNG-stream collision status is UNKNOWN because required metadata is missing"
                    .into(),
            );
        }
        warnings.sort();
        warnings.dedup();
        let comparison_max = benchmark
            .rel
            .iter()
            .flatten()
            .chain(benchmark.persist.iter().flatten())
            .copied()
            .fold(1e-6_f32, f32::max) as f64;
        let suite_view = project::CalibratedView {
            view_id: "benchmark.relative_l2.compare".into(),
            quantity: "model and persistence relative L2 error".into(),
            units: "dimensionless".into(),
            scale_min: 0.0,
            scale_max: comparison_max,
            source_class: project::EvidenceSourceClass::Derived,
            method: "shared scale over model-vs-solver-reference and persistence errors".into(),
        };
        let run_id = uuid::Uuid::new_v4().to_string();
        let evidence_id = uuid::Uuid::new_v4().to_string();
        let parent_run_id = parent.as_ref().map(|run| run.run_id().to_owned());
        let mut run_manifest = project::RunManifest {
            schema_version: project::PROJECT_SCHEMA_VERSION,
            app: project::VersionedComponent {
                name: "Reyn Studio".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                sha256: None,
            },
            engine: Some(project::VersionedComponent {
                name: "Reyn Python engine".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                sha256: None,
            }),
            model: Some(project::VersionedComponent {
                name: model.name.clone(),
                version: format!("epoch {}", benchmark.epoch),
                sha256: Some(model.checkpoint_sha256.to_ascii_lowercase()),
            }),
            solver: Some(project::VersionedComponent {
                name: "numerical solver reference".into(),
                version: "engine-declared benchmark protocol".into(),
                sha256: None,
            }),
            converter: None,
            exact_contract: active_revision.contract.clone(),
            exact_settings: serde_json::json!({
                "seeds": benchmark.seeds,
                "horizons": benchmark.horizons,
                "grid": benchmark.grid,
                "dt_frame": benchmark.dt_frame,
            }),
            seeds: benchmark.seeds.iter().map(|seed| *seed as u64).collect(),
            device: self.settings.compute_device.engine_value().into(),
            runtime_ms: (benchmark.runtime_s.max(0.0) * 1000.0).round() as u64,
            stop_reason: "complete".into(),
            warnings: warnings.clone(),
            waivers: Vec::new(),
            missing_dependencies: Vec::new(),
            output_sha256: vec![output_sha256.clone()],
            scalar_outputs,
            determinism: None,
        };
        if let Some(parent) = &parent {
            run_manifest.compare_scalars_against(parent);
        }
        let run = project::RunRecord::new(
            run_id.clone(),
            parent_run_id,
            active_revision.case_revision_id,
            now,
            now,
            project::LifecycleState::Complete,
            run_manifest,
            vec![suite_view.clone()],
        );
        let evidence = project::EvidenceArtifact {
            evidence_id: evidence_id.clone(),
            run_ids: vec![run_id.clone()],
            created_utc_unix: now,
            source_class: project::EvidenceSourceClass::Derived,
            media_type: "application/vnd.reyn.benchmark-suite+json".into(),
            byte_size: snapshot_bytes.len() as u64,
            content_sha256: output_sha256.clone(),
            derivation_method: Some("seed_horizon_benchmark_suite".into()),
            derivation_version: Some("1".into()),
            warnings,
            metadata: serde_json::json!({
                "kind": "benchmark_suite",
                "snapshot": snapshot,
                "reference_semantics": "numerical solver reference",
                "integrity_only": true,
            }),
            calibrated_views: vec![suite_view.clone()],
        };
        let selected_view_id = suite_view.view_id;
        let case_id_for_edit = case_id.clone();
        let run_id_for_edit = run_id.clone();
        self.project
            .transact(now, move |manifest| {
                manifest.append_run(&case_id_for_edit, run, now)?;
                manifest.append_evidence(evidence, now)?;
                manifest.set_selection(
                    project::ProjectSelection {
                        active_case_id: Some(case_id_for_edit),
                        selected_run_id: Some(run_id_for_edit),
                        selected_evidence_id: Some(evidence_id),
                        selected_view_id: Some(selected_view_id),
                    },
                    now,
                )?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        self.project
            .add_content_with_digest(
                snapshot_bytes,
                "application/vnd.reyn.benchmark-suite+json",
                &output_sha256,
            )
            .map_err(|error| format!("suite artifact bundle: {error}"))?;
        Ok(run_id)
    }

    fn persist_benchmark_inspector(
        &mut self,
        inspector: &engine::BenchInspector,
    ) -> Result<(), String> {
        let run_id = self
            .active_benchmark_run_id
            .clone()
            .or_else(|| self.project.manifest().selection().selected_run_id.clone())
            .ok_or_else(|| "no immutable benchmark run is selected".to_string())?;
        let case_id = self
            .project
            .manifest()
            .cases()
            .iter()
            .find(|case| case.runs().iter().any(|run| run.run_id() == run_id))
            .map(|case| case.case_id().to_owned())
            .ok_or_else(|| format!("selected run {run_id} is not in this project"))?;
        let snapshot =
            serde_json::to_value(inspector).map_err(|error| format!("cell snapshot: {error}"))?;
        let snapshot_bytes =
            serde_json::to_vec(&snapshot).map_err(|error| format!("cell snapshot: {error}"))?;
        let mut calibrated_views = Vec::new();
        for variable in InspectorVariable::ALL {
            let Some((comparison_scale, error_scale)) = inspector.maps.scales(variable) else {
                continue;
            };
            let comparison_min = if variable.signed() {
                -(comparison_scale as f64)
            } else {
                0.0
            };
            let error_min = if variable.signed() {
                -(error_scale as f64)
            } else {
                0.0
            };
            calibrated_views.extend([
                project::CalibratedView {
                    view_id: format!("benchmark.{}.model", variable.key()),
                    quantity: variable.label().into(),
                    units: variable.unit_label().into(),
                    scale_min: comparison_min,
                    scale_max: comparison_scale as f64,
                    source_class: match variable {
                        InspectorVariable::Velocity => {
                            project::EvidenceSourceClass::ModelPrediction
                        }
                        InspectorVariable::Pressure => project::EvidenceSourceClass::Recovered,
                        InspectorVariable::Vorticity | InspectorVariable::Divergence => {
                            project::EvidenceSourceClass::Derived
                        }
                    },
                    method: variable.method_note().into(),
                },
                project::CalibratedView {
                    view_id: format!("benchmark.{}.solver_reference", variable.key()),
                    quantity: variable.label().into(),
                    units: variable.unit_label().into(),
                    scale_min: comparison_min,
                    scale_max: comparison_scale as f64,
                    source_class: match variable {
                        InspectorVariable::Velocity => {
                            project::EvidenceSourceClass::SolverReference
                        }
                        InspectorVariable::Pressure => project::EvidenceSourceClass::Recovered,
                        InspectorVariable::Vorticity | InspectorVariable::Divergence => {
                            project::EvidenceSourceClass::Derived
                        }
                    },
                    method: variable.method_note().into(),
                },
                project::CalibratedView {
                    view_id: format!("benchmark.{}.error", variable.key()),
                    quantity: format!("{} error", variable.label()),
                    units: variable.unit_label().into(),
                    scale_min: error_min,
                    scale_max: error_scale as f64,
                    source_class: project::EvidenceSourceClass::Derived,
                    method: variable.method_note().into(),
                },
            ]);
        }
        let evidence_id = uuid::Uuid::new_v4().to_string();
        let selected_view_id = format!("benchmark.{}.model", self.bench_var.key());
        let snapshot_sha256 = project_sha256(&snapshot_bytes);
        let evidence = project::EvidenceArtifact {
            evidence_id: evidence_id.clone(),
            run_ids: vec![run_id.clone()],
            created_utc_unix: now_utc_unix(),
            source_class: project::EvidenceSourceClass::Derived,
            media_type: "application/vnd.reyn.benchmark-cell+json".into(),
            byte_size: snapshot_bytes.len() as u64,
            content_sha256: snapshot_sha256.clone(),
            derivation_method: Some("selected_cell_spatial_and_spectral_evidence".into()),
            derivation_version: Some(INSPECTOR_PROTOCOL_VERSION.to_string()),
            warnings: Vec::new(),
            metadata: serde_json::json!({
                "kind": "benchmark_inspector",
                "snapshot": snapshot,
                "seed": inspector.seed,
                "horizon": inspector.horizon,
                "selected_variable": self.bench_var.key(),
                "reference_semantics": "numerical solver reference",
            }),
            calibrated_views,
        };
        let now = now_utc_unix();
        self.project
            .transact(now, move |manifest| {
                manifest.append_evidence(evidence, now)?;
                manifest.set_selection(
                    project::ProjectSelection {
                        active_case_id: Some(case_id),
                        selected_run_id: Some(run_id),
                        selected_evidence_id: Some(evidence_id),
                        selected_view_id: Some(selected_view_id),
                    },
                    now,
                )?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        self.project
            .add_content_with_digest(
                snapshot_bytes,
                "application/vnd.reyn.benchmark-cell+json",
                &snapshot_sha256,
            )
            .map_err(|error| format!("selected-cell artifact bundle: {error}"))?;
        Ok(())
    }

    fn stored_artifact_snapshot(
        &self,
        artifact: &project::EvidenceArtifact,
    ) -> Result<serde_json::Value, String> {
        match self.project.content_state(&artifact.content_sha256) {
            project::ContentState::Available => {
                let bytes = self
                    .project
                    .content_bytes(&artifact.content_sha256)
                    .ok_or_else(|| "verified bundle object became unavailable".to_string())?;
                serde_json::from_slice(bytes).map_err(|error| {
                    format!(
                        "verified artifact {} is not valid JSON: {error}",
                        short_hash(&artifact.content_sha256)
                    )
                })
            }
            project::ContentState::Corrupt => Err(format!(
                "artifact {} failed content verification; its immutable manifest remains inspectable",
                short_hash(&artifact.content_sha256)
            )),
            project::ContentState::Missing => artifact
                .metadata
                .get("snapshot")
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "artifact {} is missing from the portable bundle",
                        short_hash(&artifact.content_sha256)
                    )
                }),
        }
    }

    fn hydrate_engineering_run(&mut self, run_id: &str) -> Result<bool, String> {
        let Some((case_record, run)) = self.project.manifest().cases().iter().find_map(|case| {
            case.runs()
                .iter()
                .find(|run| run.run_id() == run_id)
                .map(|run| (case.clone(), run.clone()))
        }) else {
            return Ok(false);
        };
        let contract = run.manifest().exact_contract.clone();
        if contract.get("kind").and_then(serde_json::Value::as_str)
            != Some(engineering::EXTERNAL_FLOW_CONTRACT)
        {
            return Ok(false);
        }
        let result_artifact = self
            .project
            .manifest()
            .evidence_for_run(run_id)
            .into_iter()
            .rev()
            .find(|artifact| {
                artifact
                    .metadata
                    .get("schema")
                    .and_then(serde_json::Value::as_str)
                    == Some(engineering::ENGINEERING_RESULT_SCHEMA)
            })
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Engineering run {} has no result evidence artifact.",
                    short_id(run_id)
                )
            })?;
        let summary = self.stored_artifact_snapshot(&result_artifact)?;
        let field_sha256 = summary
            .get("field_sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                "Engineering result does not identify its field artifact.".to_string()
            })?;
        let field_bytes = self.project.content_bytes(field_sha256).ok_or_else(|| {
            format!(
                "Engineering field {} is unavailable; the run remains readable from its manifest.",
                short_hash(field_sha256)
            )
        })?;
        let field = engineering::decode_engineering_field(field_bytes)?;
        let operating: engineering::OperatingPoint = serde_json::from_value(
            contract
                .get("operating_point")
                .cloned()
                .ok_or_else(|| "Engineering contract has no operating point.".to_string())?,
        )
        .map_err(|error| format!("Engineering operating point is invalid: {error}"))?;
        let preflight: engineering::GeometryPreflight = serde_json::from_value(
            contract
                .get("preflight")
                .cloned()
                .ok_or_else(|| "Engineering contract has no preflight record.".to_string())?,
        )
        .map_err(|error| format!("Engineering preflight is invalid: {error}"))?;
        let model = contract
            .get("model")
            .ok_or_else(|| "Engineering contract has no model record.".to_string())?;
        let model_id = model
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let model_support: engineering::ModelSupport =
            serde_json::from_value(model.get("support").cloned().unwrap_or_default())
                .unwrap_or_default();
        let source_revision_id = contract
            .get("source_revision_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let source = source_revision_id.as_deref().and_then(|source_id| {
            self.project
                .manifest()
                .source_revisions()
                .iter()
                .find(|source| source.source_revision_id == source_id)
        });
        let source_name = source
            .and_then(|source| source.uri_hint.as_deref())
            .and_then(|hint| std::path::Path::new(hint).file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("stored_geometry.stl")
            .to_string();
        let vec3 = |key: &str| -> [f64; 3] {
            serde_json::from_value(summary.get(key).cloned().unwrap_or_default())
                .unwrap_or([0.0; 3])
        };
        let warnings = summary
            .get("warnings")
            .and_then(serde_json::Value::as_array)
            .map(|warnings| {
                warnings
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let cp_min = field.cp.iter().copied().fold(f32::INFINITY, f32::min) as f64;
        let cp_max = field.cp.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
        let result = engineering::EngineeringResult {
            method: summary
                .get("method")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(engineering::SURFACE_LOAD_METHOD)
                .into(),
            cp_min,
            cp_max,
            force_coefficients: vec3("force_coefficients"),
            moment_coefficients: vec3("moment_coefficients"),
            force_newtons: vec3("force_newtons"),
            moment_newton_meters: vec3("moment_newton_meters"),
            surface_area_m2: summary
                .get("surface_area_m2")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            pressure_force_fraction: summary
                .get("pressure_force_fraction")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            load_hotspot: vec3("load_hotspot_m"),
            suction_hotspot: vec3("suction_hotspot_m"),
            divergence_rms: summary
                .get("divergence_rms")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            wake_deficit_peak: summary
                .get("wake_deficit_peak")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            wake_deficit_mean: summary
                .get("wake_deficit_mean")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            warnings,
        };
        let shape = vec![3usize, field.n, field.n, field.n];
        self.particles = flow::from_field(&shape, &field.velocity);
        if let Some((volume, dims)) = flow::vorticity_volume(&shape, &field.velocity) {
            self.volume_data = std::sync::Arc::new(volume);
            self.volume_dims = dims;
            self.volume_version = self.volume_version.wrapping_add(1);
        }
        let mut insights = flow::insights3d(&shape, &field.velocity);
        insights.extend(cad::surface_insights(&field.mask, &field.cp, field.n));
        self.insights3d = insights;
        let cp_scale = field
            .cp
            .iter()
            .fold(0.0f32, |scale, value| scale.max(value.abs()))
            .max(1e-6);
        let mut mask_u8 = vec![0u8; field.n * field.n * field.n];
        let mut cp_u8 = vec![128u8; field.n * field.n * field.n];
        for i in 0..field.n {
            for j in 0..field.n {
                for k in 0..field.n {
                    let source_index = i * field.n * field.n + j * field.n + k;
                    let target_index = (k * field.n + j) * field.n + i;
                    mask_u8[target_index] =
                        (field.mask[source_index].clamp(0.0, 1.0) * 255.0) as u8;
                    let normalized = (field.cp[source_index] / cp_scale) * 0.5 + 0.5;
                    cp_u8[target_index] = (normalized.clamp(0.0, 1.0) * 255.0) as u8;
                }
            }
        }
        self.cad_version = self.cad_version.wrapping_add(1);
        let surface = gpu::SurfaceData {
            mask: std::sync::Arc::new(mask_u8),
            pressure: std::sync::Arc::new(cp_u8),
            dims: [field.n as u32; 3],
            version: self.cad_version,
        };
        let workflow = engineering::ExternalFlowCase {
            stage: engineering::CaseStage::Results,
            case_id: case_record.case_id().to_owned(),
            name: case_record.name().to_owned(),
            source_name: source_name.clone(),
            source_revision_id,
            case_revision_id: Some(run.case_revision_id().to_owned()),
            model_id: model_id.clone(),
            model_sha256: model
                .get("sha256")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            model_max_steps: model
                .get("max_steps")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as u32,
            model_support,
            preflight,
            operating,
            result: Some(result),
            parent_run_id: run.parent_run_id().map(str::to_owned),
        };
        let horizon = summary
            .get("horizon")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(workflow.operating.horizon_steps as u64) as u32;
        self.invalidate_cad_section();
        self.cad = Some(CadCase {
            mask: std::sync::Arc::new(field.mask),
            model: model_id,
            steps: horizon,
            surf: Some(surface),
            name: source_name,
            workflow,
            velocity: field.velocity,
            pressure: field.pressure_pa,
            cp: field.cp,
            traction: field.traction_pa,
            result_grid: field.n,
            active_run_id: Some(run_id.to_owned()),
            pending: false,
            pending_request_id: None,
            pending_run: None,
        });
        self.surface_on = true;
        self.volumetric = true;
        self.render_volume = true;
        self.nav = Nav::Results;
        Ok(true)
    }

    fn hydrate_external_flow_runtime(&mut self, selection: &project::ProjectSelection) -> bool {
        let snapshot = {
            let Some(case_record) = self.project.manifest().cases().iter().find(|case| {
                case.active_revision()
                    .contract
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    == Some(engineering::EXTERNAL_FLOW_CONTRACT)
            }) else {
                return false;
            };
            let revision = case_record.active_revision();
            let Some(source_id) = revision.source_revision_ids.first() else {
                return false;
            };
            let Some(source) = self
                .project
                .manifest()
                .source_revisions()
                .iter()
                .find(|source| source.source_revision_id == *source_id)
            else {
                return false;
            };
            let source_bytes = self
                .project
                .content_bytes(&source.content_sha256)
                .map(<[u8]>::to_vec);
            let selected_run_id = selection
                .selected_run_id
                .as_ref()
                .filter(|run_id| case_record.runs().iter().any(|run| run.run_id() == *run_id))
                .cloned()
                .or_else(|| case_record.runs().last().map(|run| run.run_id().to_owned()));
            let result_metadata = selected_run_id.as_deref().and_then(|run_id| {
                self.project
                    .manifest()
                    .evidence_for_run(run_id)
                    .into_iter()
                    .rev()
                    .find(|artifact| {
                        artifact
                            .metadata
                            .get("schema")
                            .and_then(serde_json::Value::as_str)
                            == Some(engineering::ENGINEERING_RESULT_SCHEMA)
                    })
                    .map(|artifact| artifact.metadata.clone())
            });
            (
                case_record.case_id().to_owned(),
                case_record.name().to_owned(),
                revision.case_revision_id.clone(),
                revision.contract.clone(),
                source.clone(),
                source_bytes,
                selected_run_id,
                result_metadata,
            )
        };
        let (
            case_id,
            case_name,
            case_revision_id,
            contract,
            source,
            source_bytes,
            selected_run_id,
            result_metadata,
        ) = snapshot;
        let Some(source_bytes) = source_bytes else {
            self.project_notice = Some((
                format!(
                    "External-flow case {} is available in read-only evidence mode, but geometry object {} must be relinked.",
                    case_name,
                    short_hash(&source.content_sha256)
                ),
                true,
            ));
            return false;
        };
        let mesh = match cad::parse_stl(&source_bytes) {
            Ok(mesh) => mesh,
            Err(error) => {
                self.project_notice = Some((
                    format!("Stored geometry could not be reconstructed: {error}"),
                    true,
                ));
                return false;
            }
        };
        let preflight = match serde_json::from_value::<engineering::GeometryPreflight>(
            contract
                .get("preflight")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        ) {
            Ok(preflight) => preflight,
            Err(error) => {
                self.project_notice = Some((
                    format!("Stored external-flow preflight is malformed: {error}"),
                    true,
                ));
                return false;
            }
        };
        let operating = match serde_json::from_value::<engineering::OperatingPoint>(
            contract
                .get("operating_point")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        ) {
            Ok(operating) => operating,
            Err(error) => {
                self.project_notice = Some((
                    format!("Stored external-flow operating point is malformed: {error}"),
                    true,
                ));
                return false;
            }
        };
        let model = contract.get("model").cloned().unwrap_or_default();
        let model_id = model
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let model_sha256 = model
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let model_max_steps = model
            .get("max_steps")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32;
        let model_support = serde_json::from_value::<engineering::ModelSupport>(
            model
                .get("support")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        )
        .unwrap_or_default();
        let result = result_metadata.as_ref().map(|metadata| {
            let vector = |key: &str| -> [f64; 3] {
                std::array::from_fn(|axis| {
                    metadata
                        .get(key)
                        .and_then(|value| value.get(axis))
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0)
                })
            };
            engineering::EngineeringResult {
                method: metadata
                    .get("method")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(engineering::SURFACE_LOAD_METHOD)
                    .into(),
                cp_min: metadata
                    .pointer("/cp/minimum")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                cp_max: metadata
                    .pointer("/cp/maximum")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                force_coefficients: vector("force_coefficients"),
                moment_coefficients: vector("moment_coefficients"),
                force_newtons: vector("force_newtons"),
                moment_newton_meters: vector("moment_newton_meters"),
                surface_area_m2: metadata
                    .get("surface_area_m2")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                pressure_force_fraction: metadata
                    .get("pressure_force_fraction")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                load_hotspot: vector("load_hotspot_m"),
                suction_hotspot: vector("suction_hotspot_m"),
                divergence_rms: metadata
                    .get("divergence_rms")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                wake_deficit_peak: metadata
                    .get("wake_deficit_peak")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                wake_deficit_mean: metadata
                    .get("wake_deficit_mean")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                warnings: metadata
                    .get("warnings")
                    .and_then(serde_json::Value::as_array)
                    .map(|warnings| {
                        warnings
                            .iter()
                            .filter_map(|warning| warning.as_str().map(str::to_owned))
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        });
        let field_blob = result_metadata
            .as_ref()
            .and_then(|metadata| metadata.get("field_sha256"))
            .and_then(serde_json::Value::as_str)
            .and_then(|digest| self.project.content_bytes(digest))
            .and_then(|bytes| engineering::decode_engineering_field(bytes).ok());
        let voxel = if field_blob.is_none() {
            match cad::voxelize(&mesh, preflight.target_grid) {
                Ok(voxel) => Some(voxel),
                Err(error) => {
                    self.project_notice = Some((
                        format!("Stored case geometry could not be voxelized: {error}"),
                        true,
                    ));
                    return false;
                }
            }
        } else {
            None
        };
        let workflow = engineering::ExternalFlowCase {
            stage: if result.is_some() {
                engineering::CaseStage::Results
            } else {
                engineering::CaseStage::Setup
            },
            case_id,
            name: case_name,
            source_name: source
                .uri_hint
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("stored_geometry.stl")
                .into(),
            source_revision_id: Some(source.source_revision_id),
            case_revision_id: Some(case_revision_id),
            model_id: model_id.clone(),
            model_sha256,
            model_max_steps,
            model_support,
            preflight,
            operating,
            result,
            parent_run_id: selected_run_id.clone(),
        };
        let (mask, pressure, cp, traction, result_grid, velocity) = if let Some(field) = field_blob
        {
            (
                field.mask,
                field.pressure_pa,
                field.cp,
                field.traction_pa,
                field.n,
                Some(field.velocity),
            )
        } else {
            let voxel = voxel.expect("voxel is present without a field blob");
            (voxel.mask, Vec::new(), Vec::new(), Vec::new(), 0, None)
        };
        let mut surf = None;
        if result_grid > 0 && cp.len() == result_grid * result_grid * result_grid {
            let scale = cp
                .iter()
                .fold(0.0f32, |maximum, value| maximum.max(value.abs()))
                .max(1e-6);
            let mut mask_u8 = vec![0u8; mask.len()];
            let mut cp_u8 = vec![128u8; cp.len()];
            for i in 0..result_grid {
                for j in 0..result_grid {
                    for k in 0..result_grid {
                        let source_index = i * result_grid * result_grid + j * result_grid + k;
                        let target_index = (k * result_grid + j) * result_grid + i;
                        mask_u8[target_index] = (mask[source_index].clamp(0.0, 1.0) * 255.0) as u8;
                        cp_u8[target_index] = (((cp[source_index] / scale) * 0.5 + 0.5)
                            .clamp(0.0, 1.0)
                            * 255.0) as u8;
                    }
                }
            }
            self.cad_version = self.cad_version.wrapping_add(1);
            surf = Some(gpu::SurfaceData {
                mask: std::sync::Arc::new(mask_u8),
                pressure: std::sync::Arc::new(cp_u8),
                dims: [result_grid as u32; 3],
                version: self.cad_version,
            });
        }
        if let Some(velocity) = velocity.as_deref() {
            let shape = vec![3usize, result_grid, result_grid, result_grid];
            let particles = flow::from_field(&shape, velocity);
            if !particles.is_empty() {
                self.particles = particles;
            }
            if let Some((volume, dimensions)) = flow::vorticity_volume(&shape, velocity) {
                self.volume_data = std::sync::Arc::new(volume);
                self.volume_dims = dimensions;
                self.volume_version = self.volume_version.wrapping_add(1);
            }
            let mut insights = flow::insights3d(&shape, velocity);
            insights.extend(cad::surface_insights(&mask, &cp, result_grid));
            self.insights3d = insights;
            self.surface_on = true;
            self.render_volume = true;
        }
        self.current_model = model_id.clone();
        self.invalidate_cad_section();
        self.cad = Some(CadCase {
            mask: std::sync::Arc::new(mask),
            model: model_id,
            steps: workflow.operating.horizon_steps,
            surf,
            name: workflow.source_name.clone(),
            workflow,
            velocity: velocity.unwrap_or_default(),
            pressure,
            cp,
            traction,
            result_grid,
            active_run_id: selected_run_id,
            pending: false,
            pending_request_id: None,
            pending_run: None,
        });
        self.nav = if self
            .cad
            .as_ref()
            .and_then(|case| case.workflow.result.as_ref())
            .is_some()
        {
            Nav::Evidence
        } else {
            Nav::Case
        };
        true
    }

    fn hydrate_project_runtime(&mut self) {
        self.nav = Nav::Projects;
        self.bench = None;
        self.bench_selected = None;
        self.bench_inspector = None;
        self.bench_inspector_pending = false;
        self.bench_error = None;
        self.bench_inspector_error = None;
        self.bench_tex.clear();
        self.active_benchmark_run_id = None;

        let selection = self.project.manifest().selection().clone();
        let Some(run_id) = selection.selected_run_id.clone() else {
            let _ = self.hydrate_external_flow_runtime(&selection);
            return;
        };
        match self.hydrate_engineering_run(&run_id) {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                self.project_notice = Some((error, true));
                return;
            }
        }
        let evidence = self.project.manifest().evidence_for_run(&run_id);
        let suite = evidence.iter().rev().find(|artifact| {
            artifact
                .metadata
                .get("kind")
                .and_then(serde_json::Value::as_str)
                == Some("benchmark_suite")
        });
        let Some(suite) = suite else {
            return;
        };
        let snapshot = match self.stored_artifact_snapshot(suite) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.project_notice = Some((error, true));
                return;
            }
        };
        let Ok(benchmark) = serde_json::from_value::<engine::BenchResult>(snapshot) else {
            self.project_notice = Some((
                "Stored benchmark evidence could not be decoded; the immutable manifest remains available."
                    .into(),
                true,
            ));
            return;
        };
        self.f2d_model = benchmark.model.clone();
        if let Some(first_seed) = benchmark.seeds.first() {
            self.bench_seed_start = *first_seed;
        }
        self.bench_seeds = benchmark.seeds.len().clamp(1, u32::MAX as usize) as u32;
        self.active_benchmark_run_id = Some(run_id.clone());
        self.bench = Some(benchmark);
        self.nav = Nav::Benchmark;

        let selected_evidence = selection
            .selected_evidence_id
            .as_deref()
            .and_then(|evidence_id| self.project.manifest().evidence_artifact(evidence_id));
        let inspector_evidence = selected_evidence.filter(|artifact| {
            artifact
                .metadata
                .get("kind")
                .and_then(serde_json::Value::as_str)
                == Some("benchmark_inspector")
        });
        let Some(inspector_evidence) = inspector_evidence else {
            return;
        };
        let snapshot = match self.stored_artifact_snapshot(inspector_evidence) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.project_notice = Some((error, true));
                return;
            }
        };
        let Ok(inspector) = serde_json::from_value::<engine::BenchInspector>(snapshot) else {
            self.project_notice = Some((
                "Stored selected-cell evidence could not be decoded; suite scalars remain inspectable."
                    .into(),
                true,
            ));
            return;
        };
        if let Some(benchmark) = &self.bench {
            let seed_index = benchmark
                .seeds
                .iter()
                .position(|seed| *seed == inspector.seed);
            let horizon_index = benchmark
                .horizons
                .iter()
                .position(|horizon| *horizon == inspector.horizon);
            self.bench_selected = seed_index.zip(horizon_index);
        }
        if let Some(view_id) = selection.selected_view_id {
            if let Some(variable) = view_id
                .split('.')
                .nth(1)
                .and_then(InspectorVariable::from_key)
            {
                self.bench_var = variable;
            }
        }
        self.bench_inspector = Some(inspector);
    }

    fn inspect_project_run(&mut self, run_id: &str) {
        let Some(case_id) = self
            .project
            .manifest()
            .cases()
            .iter()
            .find(|case| case.runs().iter().any(|run| run.run_id() == run_id))
            .map(|case| case.case_id().to_owned())
        else {
            return;
        };
        let evidence = self.project.manifest().evidence_for_run(run_id);
        let selected = evidence
            .iter()
            .rev()
            .find(|artifact| {
                artifact
                    .metadata
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    == Some("benchmark_inspector")
            })
            .or_else(|| evidence.last())
            .copied();
        let selected_evidence_id = selected.map(|artifact| artifact.evidence_id.clone());
        let selected_view_id = selected
            .and_then(|artifact| artifact.calibrated_views.first())
            .map(|view| view.view_id.clone())
            .or_else(|| {
                self.project
                    .manifest()
                    .run(run_id)
                    .and_then(|run| run.calibrated_views().first())
                    .map(|view| view.view_id.clone())
            });
        let now = now_utc_unix();
        let run_id = run_id.to_owned();
        if let Err(error) = self.project.transact(now, move |manifest| {
            manifest.set_selection(
                project::ProjectSelection {
                    active_case_id: Some(case_id),
                    selected_run_id: Some(run_id),
                    selected_evidence_id,
                    selected_view_id,
                },
                now,
            )
        }) {
            self.project_notice =
                Some((format!("Stored run could not be selected: {error}"), true));
            return;
        }
        self.hydrate_project_runtime();
    }

    fn persist_benchmark_view_selection(&mut self) {
        let current = self.project.manifest().selection().clone();
        if current.selected_run_id.is_none() || current.selected_evidence_id.is_none() {
            return;
        }
        let selected_view_id = format!("benchmark.{}.model", self.bench_var.key());
        let now = now_utc_unix();
        if let Err(error) = self.project.transact(now, move |manifest| {
            manifest.set_selection(
                project::ProjectSelection {
                    selected_view_id: Some(selected_view_id),
                    ..current
                },
                now,
            )
        }) {
            self.project_notice = Some((
                format!("Selected calibrated view was not saved: {error}"),
                true,
            ));
        }
    }

    fn regenerate(&mut self) {
        self.seed = self.seed.wrapping_add(1);
        if self.engine_ok {
            let _ = self.engine.tx.send(engine::Cmd::Predict {
                model: self.current_model.clone(),
                seed: self.seed,
            });
            self.engine_status = "● Predicting…".into();
        } else {
            self.particles = flow::generate(6000, self.seed);
            let (vol, dims) = flow::procedural_volume(48, self.seed);
            self.volume_data = std::sync::Arc::new(vol);
            self.volume_dims = dims;
            self.volume_version = self.volume_version.wrapping_add(1);
            self.insights3d.clear(); // procedural placeholder: no real field to annotate
        }
    }

    /// Request a 2D prediction, coalesced to one in-flight request (TimeJump can
    /// fire many per second while dragging; stale ones are dropped, the newest
    /// re-fires when the current result lands).
    fn request_2d(&mut self) {
        if !self.engine_ok {
            return;
        }
        if self.f2d_pending {
            self.f2d_dirty = true;
            return;
        }
        let cmd = if let Some(ic) = &self.f2d_painted {
            // painted IC: stateless — the projected field rides with every scrub
            engine::Cmd::PredictIC {
                model: self.f2d_model.clone(),
                steps: self.f2d_horizon,
                ic: ic.clone(),
            }
        } else {
            engine::Cmd::Predict2D {
                model: self.f2d_model.clone(),
                steps: self.f2d_horizon,
                seed: 1,
                want_truth: self.f2d_truth,
                method: match self.f2d_method {
                    PMethod::Spectral => "spectral",
                    PMethod::Fd => "fd",
                }
                .into(),
                tolerance: 10f32.powi(-self.f2d_tol_exp),
                boundary: match self.f2d_boundary {
                    PBoundary::Periodic => "periodic",
                    PBoundary::Dirichlet => "dirichlet",
                }
                .into(),
            }
        };
        let _ = self.engine.tx.send(cmd);
        self.f2d_pending = true;
        self.f2d_req_at = Some(std::time::Instant::now());
    }

    /// Rebuild the colormapped textures only when the field, variable, or overlay
    /// changed (not every frame). Model prediction and solver reference share one
    /// colormap scale so the two panels are visually comparable (independent
    /// normalization would hide amplitude errors).
    fn ensure_f2d_textures(&mut self, ctx: &egui::Context) {
        let Some(f) = &self.f2d else { return };
        let var_id = match self.f2d_var {
            FieldVar::Velocity => 0,
            FieldVar::Vorticity => 1,
            FieldVar::Pressure => 2,
        };
        let sig = self.f2d_gen.wrapping_mul(131) ^ (var_id << 1) ^ ((self.f2d_truth as u64) << 4);
        if sig == self.f2d_sig && !self.f2d_tex.is_empty() {
            return;
        }
        let (mut scale, signed) = field2d::scalar_scale(&f.ai, f.n, self.f2d_var);
        if self.f2d_truth {
            if let Some(truth) = &f.truth {
                scale = scale.max(field2d::scalar_scale(truth, f.n, self.f2d_var).0);
            }
        }
        let opts = egui::TextureOptions::NEAREST;
        let mut tex = vec![ctx.load_texture(
            "f2d.ai",
            field2d::image_scaled(f, &f.ai, self.f2d_var, scale),
            opts,
        )];
        if self.f2d_truth {
            if let Some(truth) = &f.truth {
                tex.push(ctx.load_texture(
                    "f2d.truth",
                    field2d::image_scaled(f, truth, self.f2d_var, scale),
                    opts,
                ));
                tex.push(ctx.load_texture(
                    "f2d.err",
                    field2d::error_image(f, &f.ai, truth, self.f2d_var),
                    opts,
                ));
            }
        }
        self.f2d_tex = tex;
        self.f2d_sig = sig;
        self.f2d_scale = scale;
        self.f2d_signed = signed;
    }

    /// Import an STL, voxelize it onto the 3D model's grid, and send it to the
    /// engine (which develops the flow with the real solver, then predicts).
    fn import_cad(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl", "STL"])
            .pick_file()
        else {
            return;
        };
        self.import_cad_path(path);
    }

    /// Shared import path for the file dialog and the landing drop target.
    fn import_cad_path(&mut self, path: std::path::PathBuf) {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("mesh")
            .to_string();
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.project_notice = Some((format!("Geometry could not be read: {e}"), true));
                return;
            }
        };
        let source_sha256 = format!("{:x}", Sha256::digest(&bytes));
        let mesh = match cad::parse_stl(&bytes) {
            Ok(mesh) => mesh,
            Err(error) => {
                self.project_notice = Some((format!("STL import blocked: {error}"), true));
                return;
            }
        };
        let mesh_diagnostics = cad::diagnose_mesh(&mesh);
        // pick the best geometry-conditioned 3D checkpoint we have (grid must match)
        let (model, n) = if self
            .models
            .iter()
            .any(|model| model.id == "flow3d_obs_unified_64.pth")
        {
            ("flow3d_obs_unified_64.pth".to_string(), 64)
        } else {
            ("flow3d_obs_v1.pth".to_string(), 32)
        };
        let model_card = self.models.iter().find(|card| card.id == model).cloned();
        match cad::voxelize(&mesh, n) {
            Ok(vm) => {
                let mask = std::sync::Arc::new(vm.mask);
                let source_revision_id = format!("source-{}", uuid::Uuid::new_v4());
                let case_id = self
                    .cad
                    .as_ref()
                    .map(|case| case.workflow.case_id.clone())
                    .unwrap_or_else(|| format!("case-{}", uuid::Uuid::new_v4()));
                let case_revision_id = format!("case-revision-{}", uuid::Uuid::new_v4());
                let parent_source_revision_id = self
                    .cad
                    .as_ref()
                    .and_then(|case| case.workflow.source_revision_id.clone());
                let parent_case_revision_id = self
                    .cad
                    .as_ref()
                    .and_then(|case| case.workflow.case_revision_id.clone());
                let source_revision_number = parent_source_revision_id
                    .as_ref()
                    .and_then(|parent| {
                        self.project
                            .manifest()
                            .source_revisions()
                            .iter()
                            .find(|source| source.source_revision_id == *parent)
                            .map(|source| source.revision + 1)
                    })
                    .unwrap_or(1);
                let mut warnings = Vec::new();
                if mesh_diagnostics.inconsistent_winding_edges > 0 {
                    warnings.push(format!(
                        "{} edges have inconsistent winding.",
                        mesh_diagnostics.inconsistent_winding_edges
                    ));
                }
                if mesh_diagnostics.components > 1 {
                    warnings.push(format!(
                        "{} disconnected STL components detected.",
                        mesh_diagnostics.components
                    ));
                }
                if let Some(previous) = &self.cad {
                    let prior = &previous.workflow.preflight;
                    if prior.source_sha256 != source_sha256 {
                        warnings.push(format!(
                            "REIMPORT CHANGE · source SHA-256 {} → {}.",
                            short_hash(&prior.source_sha256),
                            short_hash(&source_sha256)
                        ));
                    }
                    if prior.triangles != mesh_diagnostics.triangles {
                        warnings.push(format!(
                            "REIMPORT CHANGE · triangle count {} → {}.",
                            prior.triangles, mesh_diagnostics.triangles
                        ));
                    }
                    let next_extents = mesh_diagnostics.extents.map(f64::from);
                    if prior.source_extents != next_extents {
                        warnings.push(format!(
                            "REIMPORT CHANGE · source extents {:?} → {:?}; region identity is not preserved for STL.",
                            prior.source_extents, next_extents
                        ));
                    }
                    if prior.boundary_edges != mesh_diagnostics.boundary_edges
                        || prior.non_manifold_edges != mesh_diagnostics.non_manifold_edges
                        || prior.degenerate_triangles != mesh_diagnostics.degenerate_triangles
                    {
                        warnings.push(format!(
                            "REIMPORT CHANGE · defects open/non-manifold/degenerate {}·{}·{} → {}·{}·{}.",
                            prior.boundary_edges,
                            prior.non_manifold_edges,
                            prior.degenerate_triangles,
                            mesh_diagnostics.boundary_edges,
                            mesh_diagnostics.non_manifold_edges,
                            mesh_diagnostics.degenerate_triangles
                        ));
                    }
                }
                let preflight = engineering::GeometryPreflight {
                    source_sha256: source_sha256.clone(),
                    source_bytes: bytes.len() as u64,
                    triangles: mesh_diagnostics.triangles,
                    components: mesh_diagnostics.components,
                    degenerate_triangles: mesh_diagnostics.degenerate_triangles,
                    boundary_edges: mesh_diagnostics.boundary_edges,
                    non_manifold_edges: mesh_diagnostics.non_manifold_edges,
                    source_extents: mesh_diagnostics.extents.map(f64::from),
                    proposed_scale: vm.transform_4x4[0],
                    solver_characteristic_length: vm.char_len as f64,
                    transform_4x4: vm.transform_4x4,
                    target_grid: vm.n,
                    solid_voxels: vm.solid_voxels,
                    voxel_components: vm.components,
                    minimum_cells_across: vm.minimum_cells_across,
                    boundary_clearance_cells: vm.boundary_clearance_cells,
                    warnings: warnings.clone(),
                    waivers: Vec::new(),
                    transform_approved: false,
                };
                let reference_length = mesh_diagnostics.extents[1]
                    .max(mesh_diagnostics.extents[2])
                    .max(1e-6) as f64;
                let workflow = engineering::ExternalFlowCase {
                    stage: engineering::CaseStage::Preflight,
                    case_id: case_id.clone(),
                    name: name.trim_end_matches(".stl").to_string(),
                    source_name: name.clone(),
                    source_revision_id: Some(source_revision_id.clone()),
                    case_revision_id: Some(case_revision_id.clone()),
                    model_id: model.clone(),
                    model_sha256: model_card
                        .as_ref()
                        .map(|card| card.checkpoint_sha256.clone()),
                    model_max_steps: model_card.as_ref().map(|card| card.max_steps).unwrap_or(64),
                    model_support: model_card
                        .as_ref()
                        .map(|card| engineering::ModelSupport {
                            status: card.status.clone(),
                            dimension: card.dimension,
                            grid: card.grid,
                            input_channels: card.in_channels,
                            output_channels: card.out_channels,
                            scenario: card.scenario.clone(),
                            physics_contract: card.physics_contract.clone(),
                        })
                        .unwrap_or_default(),
                    preflight,
                    operating: engineering::OperatingPoint {
                        reference_length,
                        // Settings › Workflow default, clamped to the model.
                        horizon_steps: self.settings.default_horizon_steps.clamp(
                            1,
                            model_card
                                .as_ref()
                                .map(|card| card.max_steps.max(1))
                                .unwrap_or(64),
                        ),
                        ..Default::default()
                    },
                    result: None,
                    parent_run_id: self
                        .cad
                        .as_ref()
                        .and_then(|case| case.active_run_id.clone()),
                };
                if let Err(error) =
                    self.project
                        .add_content_with_digest(bytes.clone(), "model/stl", &source_sha256)
                {
                    self.project_notice =
                        Some((format!("Geometry content was not stored: {error}"), true));
                    return;
                }
                let source = project::SourceRevision {
                    source_revision_id: source_revision_id.clone(),
                    source_kind: project::SourceKind::Geometry,
                    revision: source_revision_number,
                    imported_utc_unix: now_utc_unix(),
                    uri_hint: Some(path.display().to_string()),
                    byte_size: bytes.len() as u64,
                    content_sha256: source_sha256,
                    declared_units: None,
                    frame: Some("source frame; preprocessing transform pending approval".into()),
                    transform_4x4: vm.transform_4x4,
                    parent_revision_id: parent_source_revision_id,
                    warnings,
                };
                let revision = project::CaseRevision {
                    case_revision_id: case_revision_id.clone(),
                    parent_revision_id: parent_case_revision_id,
                    created_utc_unix: now_utc_unix(),
                    source_revision_ids: vec![source_revision_id],
                    contract: workflow.exact_contract(),
                    discretization: serde_json::json!({
                        "grid": [vm.n, vm.n, vm.n],
                        "solid_voxels": vm.solid_voxels,
                        "minimum_cells_across": vm.minimum_cells_across,
                        "boundary_clearance_cells": vm.boundary_clearance_cells,
                        "transform_4x4": vm.transform_4x4,
                    }),
                    outputs: serde_json::json!({
                        "velocity": "model_prediction",
                        "pressure": "recovered",
                        "surface_loads": engineering::SURFACE_LOAD_METHOD,
                    }),
                };
                let existing_case = self
                    .project
                    .manifest()
                    .cases()
                    .iter()
                    .any(|case| case.case_id() == case_id);
                let persist_result = self.project.transact(now_utc_unix(), |manifest| {
                    manifest.add_source_revision(source, now_utc_unix())?;
                    if existing_case {
                        manifest.append_case_revision(&case_id, revision, now_utc_unix())?;
                    } else {
                        manifest.create_case(
                            case_id.clone(),
                            workflow.name.clone(),
                            revision,
                            now_utc_unix(),
                        )?;
                    }
                    Ok(())
                });
                if let Err(error) = persist_result {
                    self.project_notice =
                        Some((format!("Case revision was not recorded: {error}"), true));
                    return;
                }
                self.invalidate_cad_section();
                self.cad = Some(CadCase {
                    mask: mask.clone(),
                    model: model.clone(),
                    steps: workflow.operating.horizon_steps,
                    surf: None,
                    name: name.clone(),
                    workflow,
                    velocity: Vec::new(),
                    pressure: Vec::new(),
                    cp: Vec::new(),
                    traction: Vec::new(),
                    result_grid: 0,
                    active_run_id: None,
                    pending: false,
                    pending_request_id: None,
                    pending_run: None,
                });
                self.nav = Nav::Case;
                self.engine_status = format!(
                    "● {name}: {} triangles → {} solid voxels @ {}³ · preflight required",
                    mesh_diagnostics.triangles, vm.solid_voxels, vm.n
                );
                self.project_notice = Some((
                    "Geometry revision stored. Confirm units, transform, preflight, and operating point before execution."
                        .into(),
                    false,
                ));
            }
            Err(e) => {
                self.project_notice = Some((format!("Voxel preflight blocked: {e}"), true));
            }
        }
    }

    fn import_model(&mut self) {
        if !self.engine_ok {
            self.library.notice = Some((
                "Engine unavailable; checkpoint validation cannot run.".into(),
                true,
            ));
            self.nav = Nav::Models;
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("checkpoint", &["pth"])
            .set_directory(&self.settings.research_dir)
            .pick_file()
        {
            self.library.busy = true;
            self.library.validation = None;
            self.library.notice = Some(("Validating checkpoint contract…".into(), false));
            let _ = self.engine.tx.send(engine::Cmd::ImportModel {
                path: path.to_string_lossy().into_owned(),
            });
            self.nav = Nav::Models;
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
                self.current_model, count, hel, ens, q, self.density_lo, self.opacity
            );
            let _ = std::fs::write(&path, csv);
            self.engine_status = format!(
                "● Exported {}",
                path.file_name().and_then(|s| s.to_str()).unwrap_or("file")
            );
        }
    }

    fn reset_controls(&mut self) {
        self.slice = [true, false, false];
        self.slice_pos = [0.5, 0.0, 0.0];
        self.section_axis = engineering_section::SectionAxis::X;
        self.section_quantity = engineering_section::SectionQuantity::PhysicalCp;
        self.density_lo = 0.85;
        self.opacity = 0.75;
        self.shadows = true;
        self.streamlines = false;
    }

    fn activate_model(&mut self, model_id: &str) {
        let Some(model) = self
            .models
            .iter()
            .find(|model| model.id == model_id)
            .cloned()
        else {
            self.library.notice =
                Some(("Model is no longer in the engine inventory.".into(), true));
            return;
        };
        if model.status == "invalid" {
            self.library.notice =
                Some(("Rejected checkpoints cannot be made active.".into(), true));
            return;
        }
        self.current_model = model.id.clone();
        self.seed = self.seed.wrapping_add(1);
        if model.dimension == 2 {
            self.f2d_model = model.id;
            self.f2d = None;
            self.f2d_tex.clear();
            self.f2d_sig = u64::MAX;
            self.bench = None;
            self.bench_selected = None;
            self.bench_inspector = None;
            self.bench_inspector_pending = false;
            self.bench_tex.clear();
            self.active_benchmark_run_id = None;
            if self
                .project
                .manifest()
                .case_by_contract_kind("benchmark_suite")
                .is_some()
            {
                match self.prepare_benchmark_case(true) {
                    Ok(_) => {
                        self.project_notice = Some((
                            "Active model changed; dependent benchmark stages are stale and completed runs remain immutable."
                                .into(),
                            false,
                        ));
                    }
                    Err(error) => {
                        self.project_notice = Some((
                            format!("Active model changed, but its case revision was not recorded: {error}"),
                            true,
                        ));
                    }
                }
            }
            if self.settings.developer_research_sandbox {
                self.nav = Nav::Fields2D;
                self.request_2d();
            } else {
                self.nav = Nav::Models;
            }
        } else {
            self.nav = Nav::Models;
        }
        if self.engine_ok && model.dimension == 3 && self.settings.developer_research_sandbox {
            let _ = self.engine.tx.send(engine::Cmd::Predict {
                model: self.current_model.clone(),
                seed: self.seed,
            });
            self.engine_status = "● Predicting…".into();
        }
        self.library.notice = Some((format!("{} is now active", self.current_model), false));
    }

    fn handle_library_action(&mut self, action: library::LibraryAction) {
        match action {
            library::LibraryAction::Activate(model) => self.activate_model(&model),
            library::LibraryAction::Delete(model) => {
                self.library.busy = true;
                self.library.notice = Some(("Deleting managed checkpoint…".into(), false));
                let _ = self.engine.tx.send(engine::Cmd::DeleteModel { model });
            }
            library::LibraryAction::Import => self.import_model(),
            library::LibraryAction::Refresh => {
                self.library.busy = true;
                self.library.notice = Some(("Refreshing checkpoint metadata…".into(), false));
                let _ = self.engine.tx.send(engine::Cmd::ListModels);
            }
        }
    }

    fn handle_settings_action(&mut self, ctx: &egui::Context, action: settings::SettingsAction) {
        match action {
            settings::SettingsAction::RestoreDefaults => {
                // Preferences reset; user data (signing identity, saved
                // operating-point presets) is preserved as promised in the
                // confirmation copy.
                let mut defaults = settings::AppSettings::default();
                defaults.signing_key_reference = self.settings.signing_key_reference.clone();
                defaults.signing_public_key_base64 =
                    self.settings.signing_public_key_base64.clone();
                defaults.signing_key_fingerprint_sha256 =
                    self.settings.signing_key_fingerprint_sha256.clone();
                defaults.revoked_signing_key_fingerprints =
                    self.settings.revoked_signing_key_fingerprints.clone();
                defaults.operating_presets = self.settings.operating_presets.clone();
                self.settings_draft = defaults;
                self.settings_notice = Some(("Defaults staged; save to apply.".into(), false));
            }
            settings::SettingsAction::CreateSigningKey => {
                let provider = signing::NativeKeychainProvider;
                match provider.create_key() {
                    Ok(key) => {
                        let mut candidate = self.settings.clone();
                        candidate.set_signing_key(&key);
                        match candidate.save() {
                            Ok(path) => {
                                self.settings = candidate.clone();
                                self.settings_draft = candidate;
                                self.settings_notice = Some((
                                    format!(
                                        "Ed25519 key created in the macOS Keychain; public settings saved to {}",
                                        path.display()
                                    ),
                                    false,
                                ));
                            }
                            Err(error) => {
                                let _ = provider.delete_key(&key.key_id);
                                self.settings_notice = Some((
                                    format!(
                                        "Key settings were not saved; the new Keychain item was removed: {error}"
                                    ),
                                    true,
                                ));
                            }
                        }
                    }
                    Err(error) => {
                        self.settings_notice =
                            Some((format!("Signing key was not created: {error}"), true));
                    }
                }
            }
            settings::SettingsAction::RevokeSigningKey => {
                let key_reference = self.settings.signing_key_reference.clone();
                let mut candidate = self.settings.clone();
                candidate.revoke_signing_key();
                match candidate.save() {
                    Ok(path) => {
                        self.settings = candidate.clone();
                        self.settings_draft = candidate;
                        let deletion = signing::NativeKeychainProvider.delete_key(&key_reference);
                        self.settings_notice = Some(match deletion {
                            Ok(()) => (
                                format!(
                                    "Key fingerprint revoked and private Keychain item deleted; settings saved to {}",
                                    path.display()
                                ),
                                false,
                            ),
                            Err(error) => (
                                format!(
                                    "Fingerprint is revoked and cannot sign. Keychain deletion needs attention: {error}"
                                ),
                                true,
                            ),
                        });
                    }
                    Err(error) => {
                        self.settings_notice = Some((
                            format!("Key was not revoked because settings could not save: {error}"),
                            true,
                        ));
                    }
                }
            }
            settings::SettingsAction::Save => {
                self.settings_draft.telemetry = false;
                self.settings_draft.normalize();
                if let Err(error) = self.settings_draft.validate_runtime() {
                    self.settings_notice = Some((error, true));
                    return;
                }
                let restart_engine =
                    settings::runtime_changed(&self.settings, &self.settings_draft);
                let display_changed = self.settings.colormap != self.settings_draft.colormap
                    || self.settings.cp_range_mode != self.settings_draft.cp_range_mode
                    || self.settings.cp_pinned_extent != self.settings_draft.cp_pinned_extent
                    || self.settings.unit_system != self.settings_draft.unit_system
                    || self.settings.significant_digits != self.settings_draft.significant_digits
                    || self.settings.number_notation != self.settings_draft.number_notation;
                match self.settings_draft.save() {
                    Ok(path) => {
                        self.settings = self.settings_draft.clone();
                        if !nav_is_available(self.nav, self.settings.developer_research_sandbox) {
                            self.nav = Nav::Projects;
                        }
                        apply_with_contrast(
                            ctx,
                            self.settings.theme == settings::ThemeMode::HighContrast,
                        );
                        set_reduced_motion(ctx, self.settings.reduced_motion);
                        ctx.set_zoom_factor(self.settings.ui_scale);
                        field2d::set_view_colormap(self.settings.colormap);
                        self.input_units = self.settings.input_units;
                        if display_changed {
                            // Force colormapped textures to rebuild with the
                            // new appearance preferences.
                            self.invalidate_field_textures();
                        }
                        self.settings_notice =
                            Some((format!("Saved locally to {}", path.display()), false));
                        if restart_engine {
                            self.restart_engine();
                        }
                    }
                    Err(error) => {
                        self.settings_notice =
                            Some((format!("Settings were not saved: {error}"), true));
                    }
                }
            }
        }
    }

    fn restart_engine(&mut self) {
        // Drop every engine-backed result before replacing the receiver. This
        // prevents fields, benchmark cells, or recovered surfaces from the old
        // runtime from presenting as revalidated output.
        self.invalidate_engine_results();
        self.engine = engine::EngineHandle::spawn_with_config(self.settings.engine_config());
        self.engine_ok = false;
        self.engine_status = "○ Restarting engine…".into();
        self.library.busy = true;
        let _ = self.engine.tx.send(engine::Cmd::ListModels);
    }

    fn viewport(&mut self, ui: &mut egui::Ui) {
        // C7: the near-black well is for calibrated render viewports only;
        // document screens sit on the BG surface so the elevation ladder
        // (BG → SURFACE → SURFACE_HIGH) stays legible.
        let is_render_screen = matches!(
            self.nav,
            Nav::Results | Nav::Metrics | Nav::Fields2D | Nav::FlowPainter | Nav::Benchmark
        );
        let canvas_fill = if is_render_screen {
            // Settings › Viewport: theme-sanctioned well surfaces only.
            self.settings.viewport_background.color()
        } else {
            BG
        };
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(canvas_fill))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                self.last_render_rect = is_render_screen.then_some(rect);
                // Screen-switch crossfade (§3.7): a BG-colored veil fades out
                // *over* the incoming screen (drawn last, so it also covers
                // the wgpu 3D pass — no full-brightness pop, QA C8); skipped
                // entirely under reduced motion.
                if self.nav != self.last_nav {
                    self.last_nav = self.nav;
                    self.nav_changed_at = ui.input(|input| input.time);
                }
                let veil = if reduced_motion(ui.ctx()) {
                    0.0
                } else {
                    let progress = ((ui.input(|input| input.time) - self.nav_changed_at) / 0.18)
                        .clamp(0.0, 1.0) as f32;
                    if progress < 1.0 {
                        ui.ctx().request_repaint();
                    }
                    1.0 - egui::emath::easing::cubic_out(progress)
                };
                // The decorative 40px background grid is retired (audit A20):
                // it carried no units or scale and read as ornament under
                // real measurement views.

                if self.nav == Nav::Projects {
                    self.project_view(ui);
                }
                if self.nav == Nav::Case {
                    self.engineering_case_view(ui);
                }
                if matches!(self.nav, Nav::Results | Nav::Metrics) {
                    if self.nav == Nav::Results && !self.volumetric {
                        self.cad_section_view(ui, rect);
                    } else {
                        let opts = viewport::ViewOpts {
                            opacity: self.opacity,
                            density_lo: self.density_lo,
                            density_hi: self.density_hi,
                            slice: [
                                if self.slice[0] {
                                    Some(self.slice_pos[0])
                                } else {
                                    None
                                },
                                if self.slice[1] {
                                    Some(self.slice_pos[1])
                                } else {
                                    None
                                },
                                if self.slice[2] {
                                    Some(self.slice_pos[2])
                                } else {
                                    None
                                },
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
                            surface: if self.surface_on {
                                self.cad.as_ref().and_then(|c| c.surf.clone())
                            } else {
                                None
                            },
                            markers: if self.insights_on {
                                self.insights3d
                                    .iter()
                                    .map(|ins| viewport::Marker3D {
                                        pos: ins.pos,
                                        color: insight3d_color(ins.kind),
                                        text: format!("{} {:.3}", ins.kind.glyph(), ins.value),
                                    })
                                    .collect()
                            } else {
                                Vec::new()
                            },
                            orbit_sensitivity: self.settings.orbit_sensitivity,
                            invert_scroll_zoom: self.settings.invert_scroll_zoom,
                            show_domain_bounds: self.settings.show_domain_bounds,
                        };
                        viewport::show(ui, rect, &mut self.cam, &opts, &self.particles);
                    }
                }
                if self.nav == Nav::Fields2D {
                    self.field2d_view(ui, rect);
                }
                if self.nav == Nav::FlowPainter {
                    self.painter_view(ui, rect);
                }
                if self.nav == Nav::Benchmark {
                    self.bench_view(ui, rect);
                }
                if self.nav == Nav::Models {
                    let action = library::show_library(
                        ui,
                        &self.models,
                        &self.current_model,
                        &mut self.library,
                    );
                    if let Some(action) = action {
                        self.handle_library_action(action);
                    }
                }
                if self.nav == Nav::Settings {
                    let action = settings::show_settings(
                        ui,
                        &self.settings,
                        &mut self.settings_draft,
                        &mut self.settings_ui,
                    );
                    if let Some(action) = action {
                        self.handle_settings_action(ui.ctx(), action);
                    }
                }
                if self.nav == Nav::Evidence {
                    self.engineering_evidence_view(ui);
                }

                let p = ui.painter_at(rect);
                // §4.4 applicability banner: a slim strip above the viewport
                // naming model support status for *this* case — the verdict
                // reads before the pictures (glyph + words, never color-only).
                let mut banner_offset = 0.0;
                if self.nav == Nav::Results {
                    if let Some(case) = self
                        .cad
                        .as_ref()
                        .filter(|case| case.workflow.result.is_some())
                    {
                        let status = case.workflow.model_support.status.as_str();
                        let (glyph, color, text) = if status == "clean" {
                            (
                                "✓",
                                OK,
                                format!(
                                    "Within declared fixed-body support · horizon {} of 1–{} ✓",
                                    case.workflow.operating.horizon_steps,
                                    case.workflow.model_max_steps
                                ),
                            )
                        } else if status.is_empty() {
                            (
                                "○",
                                WARN,
                                "No applicability envelope declared — UNKNOWN".to_owned(),
                            )
                        } else {
                            (
                                "!",
                                WARN,
                                "Model metadata review — support contract compatible, provenance incomplete"
                                    .to_owned(),
                            )
                        };
                        let banner =
                            Rect::from_min_size(rect.min, Vec2::new(rect.width(), 28.0));
                        p.rect_filled(banner, CornerRadius::ZERO, SURFACE);
                        p.hline(banner.x_range(), banner.max.y, Stroke::new(1.0, HAIRLINE));
                        p.text(
                            egui::pos2(banner.min.x + 16.0, banner.center().y),
                            Align2::LEFT_CENTER,
                            glyph,
                            FontId::proportional(12.0),
                            color,
                        );
                        p.text(
                            egui::pos2(banner.min.x + 34.0, banner.center().y),
                            Align2::LEFT_CENTER,
                            text,
                            mono_s().resolve(ui.style()),
                            TEXT_DIM,
                        );
                        banner_offset = 28.0;
                    }
                }
                // camera chip — live azimuth / elevation / zoom (3D only)
                if matches!(self.nav, Nav::Results | Nav::Metrics)
                    && !(self.nav == Nav::Results && !self.volumetric)
                {
                    let cam_text = format!(
                        "Perspective  ·  az {:>3.0}°  el {:>3.0}°  ·  zoom {:.2}×",
                        self.cam.yaw.to_degrees().rem_euclid(360.0),
                        self.cam.pitch.to_degrees(),
                        viewport::Camera::default().dist / self.cam.dist
                    );
                    let cg = p.layout_no_wrap(cam_text, mono_s().resolve(ui.style()), TEXT_DIM);
                    let chip = Rect::from_min_size(
                        rect.min + Vec2::new(16.0, 16.0 + banner_offset),
                        Vec2::new(cg.size().x + 24.0, 30.0),
                    );
                    p.rect_filled(chip, CornerRadius::same(3), SURFACE);
                    p.rect_stroke(
                        chip,
                        CornerRadius::same(3),
                        Stroke::new(1.0, OUTLINE_VARIANT),
                        egui::StrokeKind::Inside,
                    );
                    p.galley(
                        egui::pos2(chip.min.x + 12.0, chip.center().y - cg.size().y / 2.0),
                        cg,
                        TEXT_DIM,
                    );
                }

                // The floating engine pill is retired: engine state lives in
                // the status bar, the single status home (§4.1).

                // Interaction hint — dropped on short viewports where it
                // would collide with the section legend (QA R7), and
                // suppressible from Settings › Viewport.
                if matches!(self.nav, Nav::Results | Nav::Metrics)
                    && rect.height() >= 420.0
                    && self.settings.show_viewport_hints
                {
                    p.text(
                        rect.center_bottom() - Vec2::new(0.0, 22.0),
                        Align2::CENTER_CENTER,
                        if self.nav == Nav::Results {
                            if self.volumetric {
                                "drag to orbit · scroll to zoom · section planes remain linked to this case"
                            } else {
                                "stored engineering section · hover to inspect · geometry from active run mask"
                            }
                        } else {
                            "research sandbox · drag to orbit · scroll to zoom · G to regenerate"
                        },
                        egui::TextStyle::Small.resolve(ui.style()),
                        TEXT_MUTE,
                    );
                }

                // Crossfade veil: painted last so it sits over everything in
                // this panel, including the wgpu paint callback (QA C8).
                if veil > 0.0 {
                    ui.painter()
                        .rect_filled(rect, CornerRadius::ZERO, canvas_fill.gamma_multiply(veil));
                }
            });
    }

    fn ensure_cad_section_texture(&mut self, ctx: &egui::Context) {
        let Some(case) = self.cad.as_ref() else {
            self.invalidate_cad_section();
            self.section_error = Some("No active engineering result is available.".into());
            return;
        };
        let axis_index = self.section_axis.id() as usize;
        let location = self.slice_pos[axis_index];
        let index = match engineering_section::section_index(case.result_grid, location) {
            Ok(index) => index,
            Err(error) => {
                self.invalidate_cad_section();
                self.section_error = Some(error);
                return;
            }
        };
        let signature = self.cad_version.wrapping_mul(131)
            ^ self.section_axis.id().wrapping_mul(17)
            ^ self.section_quantity.id().wrapping_mul(31)
            ^ (index as u64).wrapping_mul(47);
        if signature == self.section_sig
            && (self.section_tex.is_some() || self.section_error.is_some())
        {
            return;
        }
        let reference_length_m = case.workflow.operating.reference_length
            * case
                .workflow
                .operating
                .length_unit
                .meters_per_unit()
                .unwrap_or(0.0);
        let input = engineering_section::SectionInput {
            n: case.result_grid,
            velocity: &case.velocity,
            pressure_pa: &case.pressure,
            mask: case.mask.as_ref(),
            cp: &case.cp,
            traction_pa: &case.traction,
            free_stream_mps: case.workflow.operating.velocity as f32,
            reference_pressure_pa: case.workflow.operating.reference_pressure as f32,
            reference_length_m: reference_length_m as f32,
            solver_characteristic_length: case.workflow.preflight.solver_characteristic_length
                as f32,
        };
        match engineering_section::extract_section(
            &input,
            self.section_axis,
            location,
            self.section_quantity,
        ) {
            Ok(mut section) => {
                // Settings › Appearance: pinned symmetric Cp range so
                // sections compare across runs. The stored field is untouched.
                if self.section_quantity == engineering_section::SectionQuantity::PhysicalCp
                    && self.settings.cp_range_mode == settings::CpRangeMode::Pinned
                {
                    section.scale = section.scale.pinned(self.settings.cp_pinned_extent as f32);
                }
                let image = engineering_section_image(&section);
                self.section_tex = Some(ctx.load_texture(
                    format!("engineering.section.{signature}"),
                    image,
                    egui::TextureOptions::NEAREST,
                ));
                self.section_data = Some(section);
                self.section_error = None;
                self.section_sig = signature;
            }
            Err(error) => {
                self.section_tex = None;
                self.section_data = None;
                self.section_error = Some(error);
                self.section_sig = signature;
            }
        }
    }

    /// Geometry-linked 2D evidence from the active immutable engineering field.
    /// No standalone/sandbox Field2D state participates in this path.
    fn cad_section_view(&mut self, ui: &mut egui::Ui, rect: Rect) {
        self.ensure_cad_section_texture(ui.ctx());
        let painter = ui.painter_at(rect);
        let (Some(texture), Some(section)) =
            (self.section_tex.as_ref(), self.section_data.as_ref())
        else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                self.section_error
                    .as_deref()
                    .unwrap_or("Stored engineering section is unavailable."),
                FontId::proportional(14.0),
                GOLD,
            );
            return;
        };

        let header_origin = rect.min + Vec2::new(18.0, 17.0);
        painter.text(
            header_origin,
            Align2::LEFT_TOP,
            format!(
                "{} · {}",
                section.quantity.label(),
                section.quantity.units()
            ),
            FontId::proportional(15.0),
            TEXT,
        );
        painter.text(
            header_origin + Vec2::new(0.0, 23.0),
            Align2::LEFT_TOP,
            section.quantity.source(),
            FontId::monospace(10.0),
            GOLD,
        );
        painter.text(
            header_origin + Vec2::new(0.0, 41.0),
            Align2::LEFT_TOP,
            section.quantity.method(),
            FontId::proportional(10.5),
            TEXT_DIM,
        );

        let available = Rect::from_min_max(
            rect.min + Vec2::new(46.0, 88.0),
            rect.max - Vec2::new(46.0, 102.0),
        );
        let side = available.width().min(available.height()).max(1.0);
        let panel = Rect::from_center_size(available.center(), Vec2::splat(side));
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture.id(), panel, uv, Color32::WHITE);
        painter.rect_stroke(
            panel,
            CornerRadius::same(2),
            Stroke::new(1.0, OUTLINE),
            egui::StrokeKind::Outside,
        );
        painter.text(
            egui::pos2(panel.center().x, panel.max.y + 12.0),
            Align2::CENTER_TOP,
            format!("+{} →", section.axis.horizontal_axis()),
            FontId::monospace(10.0),
            TEXT_DIM,
        );
        painter.text(
            egui::pos2(panel.min.x - 10.0, panel.min.y),
            Align2::RIGHT_TOP,
            format!("↑ +{}", section.axis.vertical_axis()),
            FontId::monospace(10.0),
            TEXT_DIM,
        );
        painter.text(
            egui::pos2(panel.max.x, panel.min.y - 8.0),
            Align2::RIGHT_BOTTOM,
            format!(
                "{} SECTION · location {:.3} · cell {}/{}",
                section.axis.label(),
                section.location,
                section.index,
                section.n - 1
            ),
            FontId::monospace(10.0),
            TEXT_MUTE,
        );
        let geometry_label = "■ STORED CAD MASK · solid section";
        painter.text(
            egui::pos2(panel.min.x, panel.min.y - 8.0),
            Align2::LEFT_BOTTOM,
            geometry_label,
            FontId::monospace(9.5),
            BRAND,
        );
        engineering_section_legend(&painter, panel, section);

        if let Some(position) = ui.input(|input| input.pointer.hover_pos()) {
            if panel.contains(position) {
                draw_engineering_section_probe(&painter, panel, rect, section, position);
            }
        }
    }

    /// Central 2D field render: colormapped model prediction and, when enabled,
    /// solver reference plus derived error on a shared scale. The instrument
    /// layer adds recovered-pressure extrema, max |ω|, max speed, max error, a
    /// live hover probe, and a calibrated legend.
    fn field2d_view(&mut self, ui: &mut egui::Ui, rect: Rect) {
        self.ensure_f2d_textures(ui.ctx());
        let p = ui.painter_at(rect);
        if self.f2d_tex.is_empty() {
            let t = if self.f2d_pending {
                "predicting…"
            } else {
                "no field"
            };
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                t,
                FontId::proportional(15.0),
                TEXT_MUTE,
            );
            return;
        }
        let pad = 30.0;
        let avail = Rect::from_min_max(
            rect.min + Vec2::splat(pad),
            rect.max - Vec2::new(pad, pad + 52.0),
        );
        let n = self.f2d_tex.len();
        let gap = 16.0;
        let cell_w = (avail.width() - gap * (n as f32 - 1.0)) / n as f32;
        let side = cell_w.min(avail.height());
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let titles = ["Model prediction", "Solver reference", "|error|"];
        let mut panels: Vec<Rect> = Vec::with_capacity(n);
        for (k, tex) in self.f2d_tex.iter().enumerate() {
            let x0 = avail.min.x + k as f32 * (cell_w + gap) + (cell_w - side) / 2.0;
            let y0 = avail.min.y + (avail.height() - side) / 2.0;
            let r = Rect::from_min_size(egui::pos2(x0, y0), Vec2::splat(side));
            p.image(tex.id(), r, uv, Color32::WHITE);
            p.rect_stroke(
                r,
                CornerRadius::same(3),
                Stroke::new(1.0, OUTLINE_VARIANT),
                egui::StrokeKind::Outside,
            );
            if n > 1 {
                p.text(
                    egui::pos2(r.center().x, r.max.y + 12.0),
                    Align2::CENTER_CENTER,
                    titles[k.min(2)],
                    FontId::proportional(12.0),
                    TEXT_DIM,
                );
            }
            panels.push(r);
        }

        let Some(f) = self.f2d.as_ref() else { return };

        // calibrated legend under the first panel (shared model/reference scale)
        legend_bar(&p, panels[0], self.f2d_scale, self.f2d_signed);

        // Auto-pinned critical points: field classes on the model panel and the
        // error hotspot on the |error| panel when the reference is shown.
        if self.insights_on {
            let truth = if self.f2d_truth {
                f.truth.as_deref()
            } else {
                None
            };
            let all = field2d::insights(f, &f.ai, truth, self.f2d_var);
            let mut chips: Vec<Rect> = Vec::new();
            for ins in &all {
                let class = ins.kind as usize;
                if !self.insight_classes.get(class).copied().unwrap_or(true) {
                    continue;
                }
                let panel = match ins.kind {
                    field2d::InsightKind::MaxError => match panels.get(2) {
                        Some(r) => *r,
                        None => continue,
                    },
                    _ => panels[0],
                };
                draw_insight(&p, panel, f.n, ins, &mut chips);
            }
        }

        // live probe: hover any field panel for the full readout at that cell
        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
            for (k, r) in panels.iter().enumerate().take(2) {
                if r.contains(pos) {
                    let src: &[f32] = if k == 1 {
                        match f.truth.as_deref() {
                            Some(t) => t,
                            None => &f.ai,
                        }
                    } else {
                        &f.ai
                    };
                    draw_probe(&p, *r, rect, f.n, src, pos);
                    break;
                }
            }
        }

        let cap = format!(
            "{}  ·  {}  ·  t = {:.2}s ({} steps)",
            f.scenario,
            self.f2d_var.label(),
            f.horizon as f32 * f.dt_frame,
            f.horizon
        );
        p.text(
            rect.center_bottom() - Vec2::new(0.0, 14.0),
            Align2::CENTER_CENTER,
            cap,
            FontId::proportional(12.5),
            TEXT_MUTE,
        );
    }

    /// N4 — the painting canvas: ω as a diverging-colormap texture, left drag
    /// paints +ω, right drag −ω, with stroke interpolation and a brush preview.
    fn painter_view(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let n = painter::N;
        // canvas texture (rebuilt only when a stroke/preset changed it)
        if self.paint_dirty || self.paint_tex.is_none() {
            let scale = self
                .paint
                .omega
                .iter()
                .fold(1e-3f32, |m, &w| m.max(w.abs()));
            let pixels: Vec<Color32> = self
                .paint
                .omega
                .iter()
                .map(|&w| field2d::colormap_color(w / scale, true))
                .collect();
            let img = egui::ColorImage {
                size: [n, n],
                pixels,
                source_size: Vec2::new(n as f32, n as f32),
            };
            self.paint_tex = Some(ui.ctx().load_texture(
                "painter.omega",
                img,
                egui::TextureOptions::NEAREST,
            ));
            self.paint_dirty = false;
        }

        let pad = 30.0;
        let avail = Rect::from_min_max(
            rect.min + Vec2::splat(pad),
            rect.max - Vec2::new(pad, pad + 24.0),
        );
        let side = avail.width().min(avail.height());
        let panel = Rect::from_center_size(avail.center(), Vec2::splat(side));
        let p = ui.painter_at(rect);
        if let Some(tex) = &self.paint_tex {
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            p.image(tex.id(), panel, uv, Color32::WHITE);
        }
        p.rect_stroke(
            panel,
            CornerRadius::same(3),
            Stroke::new(1.0, OUTLINE_VARIANT),
            egui::StrokeKind::Outside,
        );

        // painting interaction (primary = +ω, secondary = −ω), stroke-interpolated
        let resp = ui.interact(panel, ui.id().with("paint.canvas"), Sense::click_and_drag());
        let sign = if resp.dragged_by(egui::PointerButton::Primary) {
            Some(1.0f32)
        } else if resp.dragged_by(egui::PointerButton::Secondary) {
            Some(-1.0f32)
        } else {
            None
        };
        if let (Some(sign), Some(pos)) = (sign, resp.interact_pointer_pos()) {
            let gx = ((pos.x - panel.min.x) / panel.width() * n as f32).clamp(0.0, n as f32 - 1.0);
            let gy = ((pos.y - panel.min.y) / panel.height() * n as f32).clamp(0.0, n as f32 - 1.0);
            let step = (self.brush_radius * 0.4).max(0.75);
            let (mut sx, mut sy) = self.paint_last.unwrap_or((gx, gy));
            let dist = ((gx - sx).powi(2) + (gy - sy).powi(2)).sqrt();
            let steps = (dist / step).ceil().max(1.0) as usize;
            for k in 1..=steps {
                let t = k as f32 / steps as f32;
                let (x, y) = (sx + (gx - sx) * t, sy + (gy - sy) * t);
                self.paint.stamp(
                    x,
                    y,
                    self.brush_radius,
                    self.brush_strength * 0.25,
                    sign,
                    self.paint_sym,
                );
            }
            (sx, sy) = (gx, gy);
            self.paint_last = Some((sx, sy));
            self.paint_dirty = true;
        } else {
            self.paint_last = None;
        }
        // brush preview ring under the cursor
        if let Some(pos) = resp.hover_pos() {
            let r = self.brush_radius / n as f32 * panel.width();
            p.circle_stroke(pos, r, Stroke::new(1.0, TEXT_DIM.gamma_multiply(0.8)));
        }

        // live diagnostics chip (top-left, instrument style)
        let (proj_line, proj_col) = if self.paint.velocity.is_some() {
            (
                format!(
                    "div {:.1e} · energy {:.3} · CG {}",
                    self.paint.div_max, self.paint.energy, self.paint.cg_iters
                ),
                if self.paint.div_max < 1e-6 {
                    SUCCESS
                } else {
                    GOLD
                },
            )
        } else {
            ("unprojected — apply Leray projection".into(), GOLD)
        };
        let lines = [
            (
                format!("enstrophy {:.4}", self.paint.mean_enstrophy()),
                TEXT_DIM,
            ),
            (proj_line, proj_col),
        ];
        let font = FontId::monospace(11.0);
        let galleys: Vec<_> = lines
            .iter()
            .map(|(t, c)| p.layout_no_wrap(t.clone(), font.clone(), *c))
            .collect();
        let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + 24.0;
        let chip = Rect::from_min_size(rect.min + Vec2::new(16.0, 16.0), Vec2::new(w, 44.0));
        p.rect_filled(chip, CornerRadius::same(3), SURFACE);
        p.rect_stroke(
            chip,
            CornerRadius::same(3),
            Stroke::new(1.0, OUTLINE_VARIANT),
            egui::StrokeKind::Inside,
        );
        for (k, g) in galleys.into_iter().enumerate() {
            p.galley(
                chip.min + Vec2::new(12.0, 7.0 + k as f32 * 16.0),
                g,
                lines[k].1,
            );
        }

        p.text(
            rect.center_bottom() - Vec2::new(0.0, 14.0),
            Align2::CENTER_CENTER,
            format!("left drag +ω  ·  right drag −ω  ·  {n}² grid"),
            FontId::proportional(12.5),
            TEXT_MUTE,
        );
    }

    fn controls_painter(&mut self, ui: &mut egui::Ui) {
        ui.label(title_text("Flow Painter"));
        ui.add_space(4.0);
        ui.label(
            RichText::new("paint vorticity → project divergence-free → generate")
                .size(11.5)
                .color(TEXT_MUTE),
        );
        ui.add_space(16.0);

        ui.label(caps("Brush"));
        ui.add_space(8.0);
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Radius").color(TEXT_DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(mono(&format!("{:.0} cells", self.brush_radius), TEXT_DIM).size(12.0));
                });
            });
            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
            ui.add(
                egui::Slider::new(&mut self.brush_radius, 2.0..=24.0)
                    .show_value(false)
                    .trailing_fill(true),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Strength").color(TEXT_DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(mono(&format!("{:.1}", self.brush_strength), TEXT_DIM).size(12.0));
                });
            });
            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
            ui.add(
                egui::Slider::new(&mut self.brush_strength, 0.2..=3.0)
                    .show_value(false)
                    .trailing_fill(true),
            );
        });

        ui.add_space(14.0);
        ui.label(caps("Symmetry"));
        ui.add_space(8.0);
        card(ui, |ui| {
            ui.checkbox(
                &mut self.paint_sym.mirror_h,
                RichText::new("Mirror left–right").color(TEXT_DIM),
            );
            ui.checkbox(
                &mut self.paint_sym.mirror_v,
                RichText::new("Mirror top–bottom").color(TEXT_DIM),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Radial fold").color(TEXT_DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = if self.paint_sym.radial <= 1 {
                        "off".into()
                    } else {
                        format!("{}-fold", self.paint_sym.radial)
                    };
                    ui.label(mono(&label, TEXT_DIM).size(12.0));
                });
            });
            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
            ui.add(
                egui::Slider::new(&mut self.paint_sym.radial, 1..=8)
                    .show_value(false)
                    .trailing_fill(true),
            );
            ui.label(
                RichText::new("mirrors counter-rotate (ω is a pseudoscalar)")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
        });

        ui.add_space(14.0);
        ui.label(caps("Presets"));
        ui.add_space(8.0);
        card(ui, |ui| {
            let w = ui.available_width();
            if action_button(
                ui,
                None,
                "Vortex Pair",
                SURFACE_HIGH,
                TEXT,
                Some(OUTLINE),
                30.0,
                w,
            ) {
                self.paint.preset_vortex_pair();
                self.paint_dirty = true;
            }
            ui.add_space(4.0);
            if action_button(
                ui,
                None,
                "Shear Layer",
                SURFACE_HIGH,
                TEXT,
                Some(OUTLINE),
                30.0,
                w,
            ) {
                self.paint.preset_shear_layer();
                self.paint_dirty = true;
            }
            ui.add_space(4.0);
            if action_button(
                ui,
                None,
                "Kármán Street",
                SURFACE_HIGH,
                TEXT,
                Some(OUTLINE),
                30.0,
                w,
            ) {
                self.paint.preset_karman_street();
                self.paint_dirty = true;
            }
            ui.add_space(4.0);
            if action_button(
                ui,
                None,
                "Clear Canvas",
                Color32::TRANSPARENT,
                TEXT_MUTE,
                Some(OUTLINE_VARIANT),
                28.0,
                w,
            ) {
                self.paint.clear();
                self.paint_dirty = true;
            }
        });

        ui.add_space(14.0);
        ui.label(caps("Projection"));
        ui.add_space(8.0);
        card(ui, |ui| {
            if action_button(
                ui,
                None,
                "Apply Leray projection",
                SURFACE_HIGH,
                TEXT,
                Some(OUTLINE),
                32.0,
                ui.available_width(),
            ) {
                self.paint.project(1e-10, 4000);
            }
            if self.paint.velocity.is_some() {
                ui.add_space(8.0);
                let ok = self.paint.div_max < 1e-6;
                diag(
                    ui,
                    "Divergence check",
                    &format!("{:.1e}", self.paint.div_max),
                    if ok { SUCCESS } else { GOLD },
                );
                diag(
                    ui,
                    "Total energy",
                    &format!("{:.4}", self.paint.energy),
                    BRAND,
                );
                diag(
                    ui,
                    "Mean enstrophy",
                    &format!("{:.4}", self.paint.mean_enstrophy()),
                    BRAND,
                );
                diag(
                    ui,
                    "CG iterations",
                    &format!("{}", self.paint.cg_iters),
                    TEXT_DIM,
                );
            } else {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("ω → ψ → u: divergence-free by construction")
                        .text_style(caption())
                        .color(TEXT_MUTE),
                );
            }
        });

        ui.add_space(16.0);
        if action_button(
            ui,
            Some(ph::PLAY),
            "Generate flow",
            EMBER,
            ON_EMBER,
            None,
            44.0,
            ui.available_width(),
        ) {
            self.generate_flow();
        }
        ui.add_space(4.0);
        ui.label(
            RichText::new("auto-projects if needed · prefers the unified (mask-free) model")
                .text_style(caption())
                .color(TEXT_MUTE),
        );
    }

    /// Commit the painted IC: project if stale, hand it to the model (the
    /// unified checkpoint when available — the one trained for mask-free flow),
    /// and jump to Fields (2D) where TimeJump scrubs it.
    fn generate_flow(&mut self) {
        if self.paint.mean_enstrophy() <= 0.0 {
            return;
        } // empty canvas
        if self.paint.velocity.is_none() {
            self.paint.project(1e-10, 4000);
        }
        let Some(ic) = self.paint.ic_payload() else {
            return;
        };
        self.f2d_painted = Some(std::sync::Arc::new(ic));
        if let Some(model) = self
            .models
            .iter()
            .find(|model| model.dimension == 2 && model.id.contains("unified"))
        {
            self.f2d_model = model.id.clone();
        }
        self.f2d_truth = false; // a painted IC has no solver reference
        self.f2d = None;
        self.f2d_tex.clear();
        self.f2d_sig = u64::MAX;
        self.nav = Nav::Fields2D;
        self.request_2d();
    }

    fn select_bench_cell(&mut self, seed_index: usize, horizon_index: usize) {
        if self.bench_inspector_pending {
            return;
        }
        let Some(bench) = self.bench.as_ref() else {
            return;
        };
        let Some(&seed) = bench.seeds.get(seed_index) else {
            return;
        };
        let Some(&horizon) = bench.horizons.get(horizon_index) else {
            return;
        };
        self.bench_selected = Some((seed_index, horizon_index));
        self.bench_inspector = None;
        self.bench_inspector_error = None;
        self.bench_tex.clear();
        if !self.engine_ok {
            self.bench_inspector_error =
                Some("engine unavailable; suite values remain inspectable".into());
            return;
        }
        if self
            .engine
            .tx
            .send(engine::Cmd::InspectBenchmarkCell {
                model: bench.model.clone(),
                seed,
                horizon,
            })
            .is_ok()
        {
            self.bench_inspector_pending = true;
        } else {
            self.bench_inspector_error = Some("engine worker is no longer available".into());
        }
    }

    fn bench_keyboard(&mut self, ui: &egui::Ui) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let Some(bench) = self.bench.as_ref() else {
            return;
        };
        if bench.seeds.is_empty() || bench.horizons.is_empty() || self.bench_inspector_pending {
            return;
        }
        let (mut seed_index, mut horizon_index) = self.bench_selected.unwrap_or((0, 0));
        let old = (seed_index, horizon_index);
        ui.input(|input| {
            if input.key_pressed(egui::Key::ArrowUp) {
                seed_index = seed_index.saturating_sub(1);
            }
            if input.key_pressed(egui::Key::ArrowDown) {
                seed_index = (seed_index + 1).min(bench.seeds.len() - 1);
            }
            if input.key_pressed(egui::Key::ArrowLeft) {
                horizon_index = horizon_index.saturating_sub(1);
            }
            if input.key_pressed(egui::Key::ArrowRight) {
                horizon_index = (horizon_index + 1).min(bench.horizons.len() - 1);
            }
        });
        if (seed_index, horizon_index) != old {
            self.select_bench_cell(seed_index, horizon_index);
        } else if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            self.bench_inspector = None;
            self.bench_tex.clear();
            self.select_bench_cell(seed_index, horizon_index);
        }
    }

    fn ensure_bench_textures(&mut self, ctx: &egui::Context) {
        if self.bench_tex.len() == 3 {
            return;
        }
        let Some(cell) = self.bench_inspector.as_ref() else {
            return;
        };
        let Some(maps) = cell.maps.get(self.bench_var) else {
            return;
        };
        let Some((comparison_scale, error_scale)) = cell.maps.scales(self.bench_var) else {
            return;
        };
        let signed = self.bench_var.signed();
        let key = self.bench_var.key();
        let n = cell.maps.n();
        debug_assert_eq!(n, cell.n);
        let opts = egui::TextureOptions::NEAREST;
        self.bench_tex = vec![
            ctx.load_texture(
                format!("bench.model.{key}.{}.{}", cell.seed, cell.horizon),
                benchmark_map_image(&maps.model, n, comparison_scale, signed),
                opts,
            ),
            ctx.load_texture(
                format!("bench.reference.{key}.{}.{}", cell.seed, cell.horizon),
                benchmark_map_image(&maps.reference, n, comparison_scale, signed),
                opts,
            ),
            ctx.load_texture(
                format!("bench.error.{key}.{}.{}", cell.seed, cell.horizon),
                benchmark_map_image(&maps.error, n, error_scale, signed),
                opts,
            ),
        ];
    }

    /// N5 — the Model Suite Analysis: seeds × horizons RelL2 vs the solver, with
    /// the persistence floor per cell. The Reyn Verify seed.
    fn bench_view(&mut self, ui: &mut egui::Ui, rect: Rect) {
        self.ensure_bench_textures(ui.ctx());
        let p = ui.painter_at(rect);
        let Some(b) = self.bench.as_ref() else {
            let (title, detail, color) = if self.bench_running {
                (
                    "RUNNING MODEL SUITE",
                    "generating solver trajectories and classifying provenance; the interface remains live",
                    EMBER,
                )
            } else if let Some(error) = &self.bench_error {
                ("SUITE UNAVAILABLE", error.as_str(), DATA_RED)
            } else if !self.engine_ok {
                ("ENGINE UNAVAILABLE", "Benchmark Lab remains responsive; restore the Python engine, then run a suite.", GOLD)
            } else {
                (
                    "NO SUITE EVIDENCE",
                    "Choose exact benchmark seeds and run the full suite from the right panel.",
                    TEXT_MUTE,
                )
            };
            let empty =
                Rect::from_center_size(rect.center(), Vec2::new(rect.width().min(560.0), 116.0));
            p.rect_filled(empty, CornerRadius::same(3), SURFACE);
            p.rect_stroke(
                empty,
                CornerRadius::same(3),
                Stroke::new(1.0, OUTLINE_VARIANT),
                egui::StrokeKind::Inside,
            );
            p.text(
                empty.center_top() + Vec2::new(0.0, 27.0),
                Align2::CENTER_TOP,
                title,
                FontId::monospace(12.0),
                color,
            );
            p.text(
                empty.center() + Vec2::new(0.0, 14.0),
                Align2::CENTER_CENTER,
                detail,
                FontId::proportional(12.0),
                TEXT_MUTE,
            );
            return;
        };
        let pad = 30.0;
        let x0 = rect.min.x + pad;
        let mut y = rect.min.y + 42.0;
        let content_w = (rect.width() - 2.0 * pad).max(600.0);

        // header: model · grid · epoch · global RelL2 · runtime
        let stem = b.model.trim_end_matches(".pth");
        p.text(
            egui::pos2(x0, y),
            Align2::LEFT_TOP,
            format!("Model Suite Analysis — {stem}"),
            FontId::proportional(18.0),
            TEXT,
        );
        y += 26.0;
        let ratio_all = {
            let (mut r, mut q) = (0f32, 0f32);
            for (rr, pp) in b.rel.iter().flatten().zip(b.persist.iter().flatten()) {
                r += rr;
                q += pp;
            }
            if r > 0.0 {
                q / r
            } else {
                0.0
            }
        };
        let head = format!(
            "{}² grid · epoch {} · global RelL2 {:.4} · {:.1}× persistence · suite {:.1}s",
            b.grid, b.epoch, b.global_rel, ratio_all, b.runtime_s
        );
        p.text(
            egui::pos2(x0, y),
            Align2::LEFT_TOP,
            head,
            FontId::monospace(12.0),
            TEXT_DIM,
        );
        y += 36.0;

        // Table at left, metadata-backed provenance at right.
        let gap = 20.0;
        let provenance_w = (content_w * 0.31).clamp(250.0, 320.0);
        let table_w = (content_w - provenance_w - gap).max(390.0);
        let row_label_w = 104.0;
        let cell_w = ((table_w - row_label_w) / b.horizons.len() as f32).max(58.0);
        let cell_h = 46.0;
        for (hi, h) in b.horizons.iter().enumerate() {
            p.text(
                egui::pos2(
                    x0 + row_label_w + hi as f32 * cell_w + (cell_w - 6.0) / 2.0,
                    y,
                ),
                Align2::CENTER_TOP,
                format!("{h}× · {:.2}s", *h as f32 * b.dt_frame),
                FontId::monospace(10.5),
                TEXT_MUTE,
            );
        }
        y += 20.0;
        let rows_top = y;
        let mut clicked = None;
        for (si, seed) in b.seeds.iter().enumerate() {
            let row_y = y + si as f32 * (cell_h + 6.0);
            let stream = b
                .provenance
                .benchmark_seeds
                .iter()
                .find(|record| record.seed == *seed)
                .map(|record| record.stream.as_str())
                .unwrap_or("unknown");
            p.text(
                egui::pos2(x0, row_y + 8.0),
                Align2::LEFT_TOP,
                format!("seed {seed}"),
                FontId::monospace(11.5),
                TEXT_DIM,
            );
            p.text(
                egui::pos2(x0, row_y + 27.0),
                Align2::LEFT_TOP,
                stream_label(stream),
                FontId::monospace(8.5),
                stream_color(stream),
            );
            for hi in 0..b.horizons.len() {
                let (rel, per) = (b.rel[si][hi], b.persist[si][hi]);
                let ratio = per / rel.max(1e-9);
                let col = if ratio >= 5.0 {
                    SUCCESS
                } else if ratio >= 2.0 {
                    GOLD
                } else if ratio >= 1.0 {
                    EMBER
                } else {
                    DATA_RED
                };
                let r = Rect::from_min_size(
                    egui::pos2(x0 + row_label_w + hi as f32 * cell_w, row_y),
                    Vec2::new(cell_w - 6.0, cell_h),
                );
                let response = ui.interact(
                    r,
                    ui.make_persistent_id(("benchmark.cell", *seed, b.horizons[hi])),
                    Sense::click(),
                ).on_hover_text(format!(
                    "seed {seed} · horizon {}×\nRelL2 {rel:.6} · persistence {per:.6}\nClick to inspect real field and spectral evidence",
                    b.horizons[hi]
                ));
                if response.clicked() {
                    clicked = Some((si, hi));
                }
                p.rect_filled(r, CornerRadius::same(3), col.gamma_multiply(0.16));
                let selected = self.bench_selected == Some((si, hi));
                let stroke = if selected {
                    Stroke::new(2.0, BRAND)
                } else if response.hovered() {
                    Stroke::new(1.5, TEXT_DIM)
                } else {
                    Stroke::new(1.0, col.gamma_multiply(0.8))
                };
                p.rect_stroke(r, CornerRadius::same(3), stroke, egui::StrokeKind::Inside);
                if selected {
                    p.text(
                        r.right_top() + Vec2::new(-5.0, 4.0),
                        Align2::RIGHT_TOP,
                        "SEL",
                        FontId::monospace(7.5),
                        BRAND,
                    );
                }
                p.text(
                    egui::pos2(r.center().x, r.min.y + 9.0),
                    Align2::CENTER_TOP,
                    format!("{rel:.4}"),
                    FontId::monospace(12.5),
                    TEXT,
                );
                p.text(
                    egui::pos2(r.center().x, r.max.y - 9.0),
                    Align2::CENTER_BOTTOM,
                    format!("{ratio:.1}× persist"),
                    FontId::monospace(10.0),
                    col,
                );
            }
        }
        let rows_bottom = rows_top + b.seeds.len() as f32 * (cell_h + 6.0);
        let legend_y = rows_bottom + 12.0;
        p.text(
            egui::pos2(x0, legend_y),
            Align2::LEFT_TOP,
            "cell = fresh-test RelL2 when row says FRESH TEST · fill = margin over persistence",
            FontId::proportional(10.5),
            TEXT_MUTE,
        );
        p.text(
            egui::pos2(x0, legend_y + 16.0),
            Align2::LEFT_TOP,
            "click a cell · arrow keys move selection · Enter reloads evidence",
            FontId::proportional(10.5),
            TEXT_MUTE,
        );

        let provenance_rect = Rect::from_min_size(
            egui::pos2(x0 + table_w + gap, y - 20.0),
            Vec2::new(provenance_w, (rows_bottom - y + 52.0).max(286.0)),
        );
        paint_bench_provenance(&p, provenance_rect, &b.provenance);

        let inspector_y = (legend_y + 48.0).max(provenance_rect.max.y + 22.0);
        p.text(
            egui::pos2(x0, inspector_y),
            Align2::LEFT_TOP,
            format!("CELL INSPECTOR · {}", self.bench_var.label().to_uppercase()),
            FontId::monospace(10.0),
            TEXT_MUTE,
        );
        if let Some((si, hi)) = self.bench_selected {
            if let (Some(seed), Some(horizon)) = (b.seeds.get(si), b.horizons.get(hi)) {
                p.text(
                    egui::pos2(x0 + 230.0, inspector_y),
                    Align2::LEFT_TOP,
                    format!("seed {seed} × {horizon} steps"),
                    FontId::monospace(10.0),
                    TEXT_DIM,
                );
            }
        }
        let evidence_top = inspector_y + 22.0;
        if self.bench_inspector_pending {
            paint_evidence_state(
                &p,
                Rect::from_min_size(egui::pos2(x0, evidence_top), Vec2::new(content_w, 76.0)),
                "LOADING CELL EVIDENCE",
                "model/reference fields, derived error, divergence, and spectra are computed on demand",
                EMBER,
            );
        } else if let Some(error) = &self.bench_inspector_error {
            paint_evidence_state(
                &p,
                Rect::from_min_size(egui::pos2(x0, evidence_top), Vec2::new(content_w, 76.0)),
                "INSPECTOR UNAVAILABLE",
                error,
                DATA_RED,
            );
        } else if let Some(cell) = self.bench_inspector.as_ref() {
            let (comparison_scale, error_scale) = cell
                .maps
                .scales(self.bench_var)
                .expect("validated inspector map scales");
            let (mean_error, p95_error, max_error) = cell
                .maps
                .error_stats(self.bench_var)
                .expect("validated inspector error statistics");
            let metrics =
                Rect::from_min_size(egui::pos2(x0, evidence_top), Vec2::new(content_w, 58.0));
            p.rect_filled(metrics, CornerRadius::same(3), SURFACE);
            p.rect_stroke(
                metrics,
                CornerRadius::same(3),
                Stroke::new(1.0, OUTLINE_VARIANT),
                egui::StrokeKind::Inside,
            );
            let metric_w = metrics.width() / 4.0;
            let metric_values = [
                ("MODEL ERROR · RelL2", format!("{:.5}", cell.rel_l2), TEXT),
                (
                    "PERSISTENCE ERROR",
                    format!("{:.5}", cell.persist_rel_l2),
                    TEXT_DIM,
                ),
                (
                    "IMPROVEMENT",
                    format!("{:.2}×", cell.improvement_ratio),
                    if cell.improvement_ratio >= 2.0 {
                        SUCCESS
                    } else if cell.improvement_ratio >= 1.0 {
                        GOLD
                    } else {
                        DATA_RED
                    },
                ),
                (
                    rng_stream_proposition(&cell.provenance_status),
                    stream_label(&cell.seed_stream).to_owned(),
                    provenance_color(&cell.provenance_status),
                ),
            ];
            for (index, (label, value, color)) in metric_values.iter().enumerate() {
                let mx = metrics.min.x + index as f32 * metric_w + 12.0;
                p.text(
                    egui::pos2(mx, metrics.min.y + 9.0),
                    Align2::LEFT_TOP,
                    *label,
                    FontId::monospace(8.5),
                    TEXT_MUTE,
                );
                p.text(
                    egui::pos2(mx, metrics.min.y + 29.0),
                    Align2::LEFT_TOP,
                    value,
                    FontId::monospace(12.0),
                    *color,
                );
            }

            let plots_top = metrics.max.y + 26.0;
            let map_gap = 10.0;
            let map_side = ((table_w - 2.0 * map_gap) / 3.0)
                .min((rect.max.y - plots_top - 38.0).max(72.0))
                .min(148.0);
            let comparison_range = if self.bench_var.signed() {
                format!("shared −{comparison_scale:.3} → +{comparison_scale:.3}")
            } else {
                format!("shared 0 → {comparison_scale:.3}")
            };
            let error_range = if self.bench_var.signed() {
                format!("−{error_scale:.4} → +{error_scale:.4}")
            } else {
                format!("0 → {error_scale:.4}")
            };
            let labels = [
                (
                    format!(
                        "{} · {}",
                        self.bench_var.model_source_label(),
                        self.bench_var.symbol()
                    ),
                    format!("{comparison_range} · {}", self.bench_var.unit_label()),
                ),
                (
                    format!(
                        "{} · {}",
                        self.bench_var.reference_source_label(),
                        self.bench_var.symbol()
                    ),
                    format!("{comparison_range} · {}", self.bench_var.unit_label()),
                ),
                (
                    format!("DERIVED ERROR · {}", self.bench_var.error_symbol()),
                    format!("{error_range} · {}", self.bench_var.unit_label()),
                ),
            ];
            for index in 0..3 {
                let image_rect = Rect::from_min_size(
                    egui::pos2(x0 + index as f32 * (map_side + map_gap), plots_top),
                    Vec2::splat(map_side),
                );
                p.text(
                    image_rect.left_top() - Vec2::new(0.0, 16.0),
                    Align2::LEFT_TOP,
                    &labels[index].0,
                    FontId::monospace(8.5),
                    TEXT_MUTE,
                );
                p.rect_filled(image_rect, CornerRadius::same(2), SURFACE_LOWEST);
                if let Some(texture) = self.bench_tex.get(index) {
                    p.image(
                        texture.id(),
                        image_rect,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                p.rect_stroke(
                    image_rect,
                    CornerRadius::same(2),
                    Stroke::new(1.0, OUTLINE_VARIANT),
                    egui::StrokeKind::Inside,
                );
                p.text(
                    image_rect.left_bottom() + Vec2::new(0.0, 5.0),
                    Align2::LEFT_TOP,
                    &labels[index].1,
                    FontId::monospace(8.5),
                    TEXT_MUTE,
                );
            }
            let stats_y = plots_top + map_side + 21.0;
            p.text(
                egui::pos2(x0, stats_y),
                Align2::LEFT_TOP,
                format!(
                    "{} |error| mean {:.4} · p95 {:.4} · max {:.4}",
                    self.bench_var.label(),
                    mean_error,
                    p95_error,
                    max_error
                ),
                FontId::monospace(9.0),
                TEXT_DIM,
            );

            let spectrum_rect = Rect::from_min_size(
                egui::pos2(x0 + table_w + gap, plots_top),
                Vec2::new(provenance_w, map_side),
            );
            p.text(
                spectrum_rect.left_top() - Vec2::new(0.0, 16.0),
                Align2::LEFT_TOP,
                "KINETIC-ENERGY SPECTRUM · E(k)",
                FontId::monospace(8.5),
                TEXT_MUTE,
            );
            paint_energy_spectrum(&p, spectrum_rect, cell);
            p.text(
                spectrum_rect.left_bottom() + Vec2::new(0.0, 5.0),
                Align2::LEFT_TOP,
                format!(
                    "spectrum RelL2 {:.4} · model div RMS {:.2e} · solver ref {:.2e} · Δdiv {:.2e}",
                    cell.spectrum_rel_l2,
                    cell.divergence_model_rms,
                    cell.divergence_truth_rms,
                    cell.divergence_error_rms
                ),
                FontId::monospace(8.0),
                TEXT_MUTE,
            );
        } else {
            paint_evidence_state(
                &p,
                Rect::from_min_size(egui::pos2(x0, evidence_top), Vec2::new(content_w, 76.0)),
                "SELECT A CELL",
                "Click a seed × horizon cell to request measured evidence from the engine.",
                TEXT_MUTE,
            );
        }

        if let Some((seed_index, horizon_index)) = clicked {
            self.select_bench_cell(seed_index, horizon_index);
        }
    }

    fn controls_bench(&mut self, ui: &mut egui::Ui) {
        let original_inputs = (
            self.f2d_model.clone(),
            self.bench_seed_start,
            self.bench_seeds,
        );
        ui.label(title_text("Benchmark Lab"));
        ui.add_space(4.0);
        ui.label(
            RichText::new("the Reyn Verify seed — honest suite runs")
                .size(11.5)
                .color(TEXT_MUTE),
        );
        ui.add_space(16.0);

        let stem = |m: &str| m.trim_end_matches(".pth").to_string();
        let models: Vec<String> = self
            .models
            .iter()
            .filter(|model| model.dimension == 2 && model.status != "invalid")
            .map(|model| model.id.clone())
            .collect();
        let mut pick = self.f2d_model.clone();
        egui::ComboBox::from_id_salt("bench.model")
            .selected_text(RichText::new(stem(&pick)).color(TEXT).size(12.5))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for m in &models {
                    ui.selectable_value(&mut pick, m.clone(), stem(m));
                }
            });
        if pick != self.f2d_model {
            self.f2d_model = pick;
            self.bench = None;
            self.bench_selected = None;
            self.bench_inspector = None;
            self.bench_inspector_pending = false;
            self.bench_inspector_error = None;
            self.bench_tex.clear();
        }

        ui.add_space(12.0);
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Exact seed start").color(TEXT_DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.bench_seed_start)
                            .range(0..=999_999)
                            .speed(1),
                    );
                });
            });
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Seed count").color(TEXT_DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(mono(&format!("{}", self.bench_seeds), TEXT_DIM).size(12.0));
                });
            });
            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
            ui.add(
                egui::Slider::new(&mut self.bench_seeds, 2..=6)
                    .show_value(false)
                    .trailing_fill(true),
            );
            let last_seed = self.bench_seed_start.saturating_add(self.bench_seeds - 1);
            ui.label(
                RichText::new(format!(
                    "{}…{} exactly · no hidden offset",
                    self.bench_seed_start, last_seed
                ))
                .text_style(caption())
                .color(TEXT_MUTE),
            );
            ui.label(
                RichText::new("horizons 1× · 4× · 8× · 16×")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
        });

        let inputs_changed = original_inputs
            != (
                self.f2d_model.clone(),
                self.bench_seed_start,
                self.bench_seeds,
            );
        if inputs_changed {
            self.bench = None;
            self.bench_selected = None;
            self.bench_inspector = None;
            self.bench_inspector_pending = false;
            self.bench_error = None;
            self.bench_inspector_error = None;
            self.bench_tex.clear();
            self.active_benchmark_run_id = None;
            match self.prepare_benchmark_case(true) {
                Ok(_) => {
                    self.project_notice = Some((
                        "Benchmark inputs changed; only dependent run and evidence stages are stale. Completed runs remain inspectable."
                            .into(),
                        false,
                    ));
                }
                Err(error) => {
                    self.project_notice =
                        Some((format!("Benchmark case was not revised: {error}"), true));
                }
            }
        }

        ui.add_space(12.0);
        let selected_model_available = self.models.iter().any(|model| {
            model.id == self.f2d_model
                && model.status != "invalid"
                && is_sha256(&model.checkpoint_sha256)
        });
        // Disabled-with-reason (X3): the button never renders as a live
        // control while blocked, and states why on hover.
        let (run_label, run_gate) = if self.bench_running {
            ("Running…", Some("The benchmark suite is already running."))
        } else if self.bench_inspector_pending {
            ("Inspecting cell…", Some("A suite cell is being inspected."))
        } else if !self.engine_ok {
            (
                "Engine unavailable",
                Some("Restore the Python engine, then run a suite."),
            )
        } else if !selected_model_available {
            (
                "Model unavailable",
                Some(
                    "The selected checkpoint is invalid or missing; pick another in Model Library.",
                ),
            )
        } else {
            ("Run full suite", None)
        };
        if action_button_gated(
            ui,
            Some(ph::PLAY),
            run_label,
            EMBER,
            ON_EMBER,
            None,
            40.0,
            ui.available_width(),
            run_gate,
        ) {
            if let Err(error) = self.prepare_benchmark_case(true) {
                self.bench_error = Some(format!(
                    "Run blocked because its project lineage could not be recorded: {error}"
                ));
                return;
            }
            self.bench = None;
            self.bench_selected = None;
            self.bench_inspector = None;
            self.bench_inspector_pending = false;
            self.bench_error = None;
            self.bench_inspector_error = None;
            self.bench_tex.clear();
            self.bench_running = true;
            let seeds = (0..self.bench_seeds)
                .map(|offset| self.bench_seed_start.saturating_add(offset))
                .collect();
            if self
                .engine
                .tx
                .send(engine::Cmd::RunBenchmark {
                    model: self.f2d_model.clone(),
                    seeds,
                    horizons: vec![1, 4, 8, 16],
                })
                .is_err()
            {
                self.bench_running = false;
                self.bench_error = Some("engine worker is no longer available".into());
            }
        }

        if self.bench_inspector.is_some() {
            ui.add_space(12.0);
            ui.label(caps("Cell Inspector Variable"));
            ui.add_space(8.0);
            let mut selected = self.bench_var;
            let inspector_maps = &self
                .bench_inspector
                .as_ref()
                .expect("inspector is present")
                .maps;
            card(ui, |ui| {
                egui::ComboBox::from_id_salt("bench.inspector.variable")
                    .selected_text(format!("{} · {}", selected.symbol(), selected.label()))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for variable in InspectorVariable::ALL {
                            ui.selectable_value(
                                &mut selected,
                                variable,
                                format!("{} · {}", variable.symbol(), variable.label()),
                            );
                        }
                    });
                ui.label(
                    RichText::new(selected.method_note())
                        .text_style(caption())
                        .color(TEXT_MUTE),
                );
                ui.label(
                    RichText::new("model and solver reference share one calibrated scale")
                        .text_style(caption())
                        .color(TEXT_MUTE),
                );
                if let Some((model_rms, reference_rms, error_rms)) = inspector_maps.rms(selected) {
                    ui.label(
                        RichText::new(format!(
                            "RMS [{}] · model {model_rms:.2e} · reference {reference_rms:.2e} · error {error_rms:.2e}",
                            selected.unit_label()
                        ))
                        .text_style(mono_s())
                        .color(TEXT_DIM),
                    );
                }
            });
            if selected != self.bench_var {
                self.bench_var = selected;
                self.bench_tex.clear();
                self.persist_benchmark_view_selection();
            }
        }

        if self.bench.is_some() {
            ui.add_space(12.0);
            ui.label(caps("Evidence Export"));
            ui.add_space(8.0);
            card(ui, |ui| {
                if action_button(
                    ui,
                    Some(ph::DOWNLOAD_SIMPLE),
                    "Export CSV",
                    SURFACE_HIGH,
                    TEXT,
                    Some(OUTLINE),
                    30.0,
                    ui.available_width(),
                ) {
                    self.export_bench_csv();
                }
                ui.add_space(4.0);
                if action_button(
                    ui,
                    Some(ph::DOWNLOAD_SIMPLE),
                    "Report Card (JSON)",
                    SURFACE_HIGH,
                    TEXT,
                    Some(OUTLINE),
                    30.0,
                    ui.available_width(),
                ) {
                    self.export_report_card();
                }
                ui.add_space(4.0);
                if action_button(
                    ui,
                    Some(ph::DOWNLOAD_SIMPLE),
                    "Report Card (PNG + JSON)",
                    SURFACE_HIGH,
                    TEXT,
                    Some(OUTLINE),
                    30.0,
                    ui.available_width(),
                ) {
                    self.export_visual_report(crate::benchmark_export::ExportFormat::Png);
                }
                ui.add_space(4.0);
                if action_button(
                    ui,
                    Some(ph::DOWNLOAD_SIMPLE),
                    "Report Card (PDF + JSON)",
                    SURFACE_HIGH,
                    TEXT,
                    Some(OUTLINE),
                    30.0,
                    ui.available_width(),
                ) {
                    self.export_visual_report(crate::benchmark_export::ExportFormat::Pdf);
                }
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                let configured_key = self.settings.configured_signing_key();
                let signing_ready = configured_key.is_ok_and(|key| key.is_some());
                if action_button(
                    ui,
                    Some(ph::DOWNLOAD_SIMPLE),
                    if signing_ready {
                        "Sign JSON + PNG + PDF…"
                    } else {
                        "Signing key unavailable"
                    },
                    if signing_ready { EMBER } else { SURFACE_HIGH },
                    if signing_ready { ON_EMBER } else { TEXT_MUTE },
                    None,
                    32.0,
                    ui.available_width(),
                ) && signing_ready
                {
                    self.export_signed_report_bundle();
                }
                ui.add_space(4.0);
                if action_button(
                    ui,
                    None,
                    "Verify detached signature…",
                    SURFACE_HIGH,
                    TEXT,
                    Some(OUTLINE),
                    30.0,
                    ui.available_width(),
                ) {
                    self.verify_detached_signature();
                }
                ui.label(
                    RichText::new(if signing_ready {
                        "READY · Ed25519 signs the canonical payload hash · private key remains in macOS Keychain"
                    } else if self.settings.signing_key_is_revoked() {
                        "REVOKED · this fingerprint cannot produce a signed claim"
                    } else {
                        "UNSIGNED · SHA-256 integrity only · configure a valid key in Settings"
                    })
                    .text_style(caption())
                    .color(TEXT_MUTE),
                );
                if let Some((message, is_error)) = &self.signing_notice {
                    ui.label(
                        RichText::new(message)
                            .text_style(caption())
                            .color(if *is_error { DATA_RED } else { SUCCESS }),
                    );
                }
            });
        }
        ui.add_space(12.0);
        ui.label(
            RichText::new(
                "N5.3 · velocity / vorticity / recovered-pressure / spatial-divergence evidence",
            )
            .text_style(caption())
            .color(TEXT_MUTE),
        );
    }

    fn export_bench_csv(&mut self) {
        let Some(b) = self.bench.as_ref() else { return };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("reyn_benchmark.csv")
            .save_file()
        else {
            return;
        };
        let csv = benchmark_csv(b);
        let _ = std::fs::write(&path, csv);
        self.engine_status = format!(
            "● Exported {}",
            path.file_name().and_then(|s| s.to_str()).unwrap_or("csv")
        );
    }

    /// Machine-readable evidence with a SHA-256 hash over the canonical payload.
    /// This export does not produce an organization-key authenticity signature.
    fn export_report_card(&mut self) {
        let (run_id, model_sha256, timestamp_unix) = match self.benchmark_export_identity() {
            Ok(identity) => identity,
            Err(error) => {
                self.engine_status = format!("● Report export unavailable · {error}");
                return;
            }
        };
        let Some(b) = self.bench.as_ref() else { return };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name("reyn_report_card.json")
            .save_file()
        else {
            return;
        };
        let (json, hex) = benchmark_report_card(
            b,
            self.bench_inspector.as_ref(),
            &run_id,
            &model_sha256,
            timestamp_unix,
        );
        if let Err(error) = crate::benchmark_export::verify_canonical_report(&json) {
            self.engine_status = format!("● Report export failed verification · {error}");
            return;
        }
        if let Err(error) = std::fs::write(&path, json) {
            self.engine_status = format!("● Report export failed · {error}");
            return;
        }
        self.engine_status = format!(
            "● Report card {} · canonical sha256 {}… · UNSIGNED",
            path.file_name().and_then(|s| s.to_str()).unwrap_or("json"),
            &hex[..12]
        );
    }

    fn benchmark_export_identity(&self) -> Result<(String, String, u64), String> {
        let benchmark = self
            .bench
            .as_ref()
            .ok_or_else(|| "no benchmark suite is available".to_string())?;
        let run_id = self
            .active_benchmark_run_id
            .clone()
            .ok_or_else(|| "the suite is not linked to an immutable run".to_string())?;
        let model_sha256 = self
            .models
            .iter()
            .find(|model| model.id == benchmark.model)
            .map(|model| model.checkpoint_sha256.to_ascii_lowercase())
            .filter(|digest| is_sha256(digest))
            .ok_or_else(|| "the selected model has no verified checkpoint SHA-256".to_string())?;
        let timestamp_unix = self
            .project
            .manifest()
            .run(&run_id)
            .map(project::RunRecord::completed_utc_unix)
            .ok_or_else(|| "the immutable benchmark run is no longer available".to_string())?;
        Ok((run_id, model_sha256, timestamp_unix))
    }

    fn export_visual_report(&mut self, format: crate::benchmark_export::ExportFormat) {
        let (run_id, model_sha256, timestamp_unix) = match self.benchmark_export_identity() {
            Ok(identity) => identity,
            Err(error) => {
                self.engine_status = format!("● Report export unavailable · {error}");
                return;
            }
        };
        let Some(benchmark) = self.bench.as_ref() else {
            return;
        };
        let label = match format {
            crate::benchmark_export::ExportFormat::Png => "PNG",
            crate::benchmark_export::ExportFormat::Pdf => "PDF",
        };
        let extension = format.extension();
        let Some(path) = rfd::FileDialog::new()
            .add_filter(label, &[extension])
            .set_file_name(format!("reyn_report_card.{extension}"))
            .save_file()
        else {
            return;
        };
        let (json, canonical_hash) = benchmark_report_card(
            benchmark,
            self.bench_inspector.as_ref(),
            &run_id,
            &model_sha256,
            timestamp_unix,
        );
        let artifact = match crate::benchmark_export::export_report(&json, format) {
            Ok(artifact) => artifact,
            Err(error) => {
                self.engine_status = format!("● {label} report export failed · {error}");
                return;
            }
        };
        let json_path = path.with_extension("json");
        if let Err(error) = std::fs::write(&json_path, json) {
            self.engine_status =
                format!("● {label} report export failed · canonical JSON: {error}");
            return;
        }
        if let Err(error) = std::fs::write(&path, &artifact.bytes) {
            self.engine_status = format!(
                "● {label} rendering failed · canonical JSON saved as {} · {error}",
                json_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("report.json")
            );
            return;
        }
        self.engine_status = format!(
            "● {label} + JSON exported · {} · canonical {}… · artifact {}… · UNSIGNED",
            artifact.media_type(),
            &canonical_hash[..12],
            &artifact.content_sha256[..12],
        );
    }

    fn export_signed_report_bundle(&mut self) {
        let (run_id, model_sha256, timestamp_unix) = match self.benchmark_export_identity() {
            Ok(identity) => identity,
            Err(error) => {
                self.signing_notice = Some((format!("UNSIGNED · {error}"), true));
                return;
            }
        };
        let configured_key = match self.settings.configured_signing_key() {
            Ok(Some(key)) => key,
            Ok(None) => {
                self.signing_notice =
                    Some(("UNSIGNED · no signing key is configured".into(), true));
                return;
            }
            Err(error) => {
                self.signing_notice = Some((format!("UNSIGNED · {error}"), true));
                return;
            }
        };
        let Some(benchmark) = self.bench.as_ref() else {
            return;
        };
        let Some(selected_path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name("reyn_report_card.json")
            .save_file()
        else {
            return;
        };
        let report_path = selected_path.with_extension("json");
        let signature_path = selected_path.with_extension("sig.json");
        let png_path = selected_path.with_extension("png");
        let pdf_path = selected_path.with_extension("pdf");
        let (json, canonical_payload_sha256) = benchmark_report_card(
            benchmark,
            self.bench_inspector.as_ref(),
            &run_id,
            &model_sha256,
            timestamp_unix,
        );
        if let Err(error) = crate::benchmark_export::verify_canonical_report(&json) {
            self.signing_notice =
                Some((format!("UNSIGNED · canonical report failed: {error}"), true));
            return;
        }
        let lineage = signing::SigningLineage {
            run_id,
            report_schema: crate::benchmark_export::REPORT_CARD_SCHEMA.into(),
            canonical_report_sha256: signing::sha256_hex(json.as_bytes()),
            canonical_payload_sha256,
            created_utc_unix: timestamp_unix,
        };
        let signature = match signing::sign_canonical_payload(
            &signing::NativeKeychainProvider,
            &configured_key,
            self.settings.signing_key_is_revoked(),
            &lineage,
        ) {
            Ok(signature) => signature,
            Err(error) => {
                self.signing_notice = Some((format!("UNSIGNED · signing failed: {error}"), true));
                return;
            }
        };
        let signature_json = match signature.to_json() {
            Ok(json) => json,
            Err(error) => {
                self.signing_notice = Some((format!("UNSIGNED · sidecar failed: {error}"), true));
                return;
            }
        };
        let png = match crate::benchmark_export::export_report_with_signature(
            &json,
            crate::benchmark_export::ExportFormat::Png,
            &signature,
        ) {
            Ok(artifact) => artifact,
            Err(error) => {
                self.signing_notice =
                    Some((format!("UNSIGNED · signed PNG failed: {error}"), true));
                return;
            }
        };
        let pdf = match crate::benchmark_export::export_report_with_signature(
            &json,
            crate::benchmark_export::ExportFormat::Pdf,
            &signature,
        ) {
            Ok(artifact) => artifact,
            Err(error) => {
                self.signing_notice =
                    Some((format!("UNSIGNED · signed PDF failed: {error}"), true));
                return;
            }
        };
        for (path, bytes) in [
            (&report_path, json.as_bytes()),
            (&signature_path, signature_json.as_bytes()),
            (&png_path, png.bytes.as_slice()),
            (&pdf_path, pdf.bytes.as_slice()),
        ] {
            if let Err(error) = std::fs::write(path, bytes) {
                self.signing_notice = Some((
                    format!("Signed bundle write failed at {}: {error}", path.display()),
                    true,
                ));
                return;
            }
        }
        let project_status = match self.persist_signed_report(&json, &signature_json, &signature) {
            Ok(true) => "signature and report appended as derived project evidence",
            Ok(false) => "matching signature already exists; immutable evidence was not duplicated",
            Err(error) => {
                self.project_notice = Some((
                    format!(
                        "Signed files were exported, but project evidence was not appended: {error}"
                    ),
                    true,
                ));
                "external bundle verified; project append failed"
            }
        };
        let sidecar_short = signature
            .content_sha256()
            .map(|digest| short_hash(&digest))
            .unwrap_or_else(|_| "UNKNOWN".into());
        self.signing_notice = Some((
            format!(
                "VERIFIED CONFIGURED KEY · Ed25519 · {} · sidecar {} · {project_status}",
                short_hash(&configured_key.key_fingerprint_sha256),
                sidecar_short,
            ),
            false,
        ));
    }

    fn verify_detached_signature(&mut self) {
        let Some(report_path) = rfd::FileDialog::new()
            .add_filter("Canonical report JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        let signature_path = report_path.with_extension("sig.json");
        let report = match std::fs::read_to_string(&report_path) {
            Ok(report) => report,
            Err(error) => {
                self.signing_notice = Some((format!("VERIFY FAILED · report read: {error}"), true));
                return;
            }
        };
        let signature_json = match std::fs::read_to_string(&signature_path) {
            Ok(signature) => signature,
            Err(error) => {
                self.signing_notice = Some((
                    format!(
                        "VERIFY FAILED · expected detached sidecar {}: {error}",
                        signature_path.display()
                    ),
                    true,
                ));
                return;
            }
        };
        let signature = match signing::SignedEvidenceArtifact::from_json(&signature_json) {
            Ok(signature) => signature,
            Err(error) => {
                self.signing_notice =
                    Some((format!("VERIFY FAILED · malformed sidecar: {error}"), true));
                return;
            }
        };
        let trusted: Vec<String> = self
            .settings
            .configured_signing_key()
            .ok()
            .flatten()
            .map(|key| vec![key.key_fingerprint_sha256])
            .unwrap_or_default();
        let policy = signing::VerificationPolicy::new(
            trusted,
            self.settings.revoked_signing_key_fingerprints.clone(),
        );
        let outcome =
            crate::benchmark_export::verify_report_signature(&report, &signature, &policy);
        self.signing_notice = Some((
            format!(
                "{:?} · {} · key {} · fingerprint {}",
                outcome.status,
                outcome.detail,
                outcome.key_id.as_deref().unwrap_or("UNKNOWN"),
                outcome
                    .key_fingerprint_sha256
                    .as_deref()
                    .map(short_hash)
                    .unwrap_or_else(|| "UNKNOWN".into()),
            ),
            !outcome.status.is_cryptographically_valid(),
        ));
    }

    fn persist_signed_report(
        &mut self,
        canonical_report_json: &str,
        signature_json: &str,
        signature: &signing::SignedEvidenceArtifact,
    ) -> Result<bool, String> {
        let report_sha256 = signing::sha256_hex(canonical_report_json.as_bytes());
        let signature_sha256 = signing::sha256_hex(signature_json.as_bytes());
        let canonical_payload_sha256 = &signature.source.canonical_payload_sha256;
        let key_fingerprint = &signature.authenticity.key_fingerprint_sha256;
        if self.project.manifest().evidence().iter().any(|artifact| {
            artifact.source_class == project::EvidenceSourceClass::AuthenticitySignature
                && artifact
                    .metadata
                    .get("canonical_payload_sha256")
                    .and_then(serde_json::Value::as_str)
                    == Some(canonical_payload_sha256)
                && artifact
                    .metadata
                    .get("key_fingerprint_sha256")
                    .and_then(serde_json::Value::as_str)
                    == Some(key_fingerprint)
        }) {
            return Ok(false);
        }
        self.project
            .add_content_with_digest(
                canonical_report_json.as_bytes().to_vec(),
                "application/vnd.reyn.benchmark-report+json",
                &report_sha256,
            )
            .map_err(|error| format!("canonical report bundle: {error}"))?;
        self.project
            .add_content_with_digest(
                signature_json.as_bytes().to_vec(),
                "application/vnd.reyn.evidence-signature+json",
                &signature_sha256,
            )
            .map_err(|error| format!("signature sidecar bundle: {error}"))?;

        let report_evidence_id = format!("benchmark-report-{report_sha256}");
        let signature_evidence_id =
            format!("benchmark-signature-{key_fingerprint}-{canonical_payload_sha256}");
        let report_exists = self
            .project
            .manifest()
            .evidence_artifact(&report_evidence_id)
            .is_some();
        let run_id = signature.source.run_id.clone();
        let created_utc_unix = signature.created_utc_unix;
        let report = project::EvidenceArtifact {
            evidence_id: report_evidence_id.clone(),
            run_ids: vec![run_id.clone()],
            created_utc_unix,
            source_class: project::EvidenceSourceClass::Derived,
            media_type: "application/vnd.reyn.benchmark-report+json".into(),
            byte_size: canonical_report_json.len() as u64,
            content_sha256: report_sha256.clone(),
            derivation_method: Some("canonical_benchmark_report".into()),
            derivation_version: Some("1".into()),
            warnings: vec![
                "The canonical report remains immutable and explicitly UNSIGNED; authenticity is carried by a separate signature evidence artifact."
                    .into(),
            ],
            metadata: serde_json::json!({
                "kind": "canonical_benchmark_report",
                "canonical_payload_sha256": canonical_payload_sha256,
                "authenticity": "detached_signature",
            }),
            calibrated_views: Vec::new(),
        };
        let signature_evidence = project::EvidenceArtifact {
            evidence_id: signature_evidence_id.clone(),
            run_ids: vec![run_id.clone()],
            created_utc_unix,
            source_class: project::EvidenceSourceClass::AuthenticitySignature,
            media_type: "application/vnd.reyn.evidence-signature+json".into(),
            byte_size: signature_json.len() as u64,
            content_sha256: signature_sha256,
            derivation_method: Some("ed25519_canonical_payload_signature".into()),
            derivation_version: Some("1".into()),
            warnings: vec![
                "Signature validity and organization-key trust are separate; compare the fingerprint independently and apply current revocations."
                    .into(),
            ],
            metadata: serde_json::json!({
                "kind": "benchmark_signature",
                "signature_schema": signing::SIGNATURE_SCHEMA,
                "parent_evidence_id": report_evidence_id,
                "canonical_report_sha256": report_sha256,
                "canonical_payload_sha256": canonical_payload_sha256,
                "algorithm": signing::SIGNATURE_ALGORITHM,
                "key_id": signature.authenticity.key_id,
                "key_fingerprint_sha256": key_fingerprint,
                "verification_at_creation": "valid",
            }),
            calibrated_views: Vec::new(),
        };
        let now = now_utc_unix();
        self.project
            .transact(now, move |manifest| {
                if !report_exists {
                    manifest.append_evidence(report, now)?;
                }
                manifest.append_evidence(signature_evidence, now)?;
                manifest.set_selection(
                    project::ProjectSelection {
                        active_case_id: manifest
                            .cases()
                            .iter()
                            .find(|case| case.runs().iter().any(|run| run.run_id() == run_id))
                            .map(|case| case.case_id().to_owned()),
                        selected_run_id: Some(run_id),
                        selected_evidence_id: Some(signature_evidence_id),
                        selected_view_id: None,
                    },
                    now,
                )?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        Ok(true)
    }

    fn controls_2d(&mut self, ui: &mut egui::Ui) {
        ui.label(title_text("Pressure Recovery (2D)"));
        ui.add_space(8.0);
        // model selector — the obstacle-family 2D checkpoints (all work with predict2d)
        let stem = |m: &str| m.trim_end_matches(".pth").to_string();
        let models: Vec<String> = self
            .models
            .iter()
            .filter(|model| model.dimension == 2 && model.status != "invalid")
            .map(|model| model.id.clone())
            .collect();
        let mut pick = self.f2d_model.clone();
        egui::ComboBox::from_id_salt("f2d.model")
            .selected_text(RichText::new(stem(&pick)).color(TEXT).size(12.5))
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
        // Concentric radii: thumb r-1 = 4 inside container r-2 = 6 (§3.5).
        Frame::NONE
            .fill(SURFACE)
            .corner_radius(CornerRadius::same(R2))
            .stroke(Stroke::new(1.0, HAIRLINE))
            .inner_margin(2)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for v in [FieldVar::Velocity, FieldVar::Vorticity, FieldVar::Pressure] {
                        if seg(ui, v.label(), self.f2d_var == v) {
                            self.f2d_var = v;
                        }
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
                    ui.label(
                        mono(
                            &format!(
                                "t = {:.2}s · {} steps",
                                self.f2d_horizon as f32 * dt,
                                self.f2d_horizon
                            ),
                            EMBER,
                        )
                        .size(12.0),
                    );
                });
            });
            ui.add_space(6.0);
            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
            let resp = ui.add(
                egui::Slider::new(&mut self.f2d_horizon, 1..=32)
                    .show_value(false)
                    .trailing_fill(true),
            );
            if resp.changed() {
                self.request_2d();
            }
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
        ui.label(caps("Consistency check"));
        ui.add_space(8.0);
        card(ui, |ui| match self.f2d.as_ref().and_then(|f| f.semigroup) {
            Some(s) => {
                let pct = (1.0 - s).clamp(0.0, 1.0) * 100.0;
                let col = if pct >= 98.0 {
                    SUCCESS
                } else if pct >= 90.0 {
                    GOLD
                } else {
                    EMBER
                };
                diag(ui, "Self-consistency", &format!("{:.1}%", pct), col);
                ui.label(
                    RichText::new("semigroup: predict h  vs  h/2 then h/2")
                        .text_style(caption())
                        .color(TEXT_MUTE),
                );
            }
            None => {
                diag(ui, "Self-consistency", "— (odd h)", TEXT_MUTE);
            }
        });

        ui.add_space(16.0);
        ui.label(caps("Solver reference overlay"));
        ui.add_space(8.0);
        card(ui, |ui| {
            if self.f2d_painted.is_some() {
                ui.label(
                    RichText::new("painted IC — no solver reference exists")
                        .color(GOLD)
                        .size(12.0),
                );
                ui.label(
                    RichText::new(
                        "semigroup self-consistency is available; it is not an accuracy claim",
                    )
                    .text_style(caption())
                    .color(TEXT_MUTE),
                );
                ui.add_space(6.0);
                if action_button(
                    ui,
                    None,
                    "Release painted IC",
                    Color32::TRANSPARENT,
                    TEXT_MUTE,
                    Some(OUTLINE_VARIANT),
                    28.0,
                    ui.available_width(),
                ) {
                    self.f2d_painted = None;
                    self.f2d = None;
                    self.f2d_tex.clear();
                    self.f2d_sig = u64::MAX;
                    self.request_2d();
                }
                return;
            }
            if ui
                .checkbox(
                    &mut self.f2d_truth,
                    RichText::new("Compare to solver reference").color(TEXT_DIM),
                )
                .changed()
            {
                self.request_2d();
            }
            if self.f2d_truth {
                if let Some((rel, per)) = self.f2d.as_ref().and_then(|f| f.rel_l2.zip(f.persist)) {
                    ui.add_space(6.0);
                    diag(
                        ui,
                        "RelL2 vs solver reference",
                        &format!("{:.4}", rel),
                        if rel < per { SUCCESS } else { EMBER },
                    );
                    diag(ui, "Persistence floor", &format!("{:.4}", per), TEXT_DIM);
                    let x = per / rel.max(1e-6);
                    diag(
                        ui,
                        "Beats persistence",
                        &format!("{:.1}×", x),
                        if x > 1.0 { SUCCESS } else { EMBER },
                    );
                }
            }
        });

        ui.add_space(16.0);
        ui.label(caps("Field Insights"));
        ui.add_space(8.0);
        card(ui, |ui| {
            ui.checkbox(
                &mut self.insights_on,
                RichText::new("Pin critical points").color(TEXT_DIM),
            );
            if self.insights_on {
                ui.add_space(4.0);
                let kinds = [
                    field2d::InsightKind::PeakPressure,
                    field2d::InsightKind::SuctionPeak,
                    field2d::InsightKind::MaxVorticity,
                    field2d::InsightKind::MaxSpeed,
                    field2d::InsightKind::MaxError,
                ];
                for (k, kind) in kinds.iter().enumerate() {
                    let needs_truth = *kind == field2d::InsightKind::MaxError;
                    ui.add_enabled_ui(!needs_truth || self.f2d_truth, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.insight_classes[k], "");
                            ui.label(RichText::new("●").color(insight_color(*kind)).size(9.0));
                            ui.label(RichText::new(kind.label()).color(TEXT_DIM).size(12.5));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(mono(kind.glyph(), TEXT_MUTE).size(11.0));
                            });
                        });
                    });
                }
            }
            ui.add_space(2.0);
            ui.label(
                RichText::new("hover a field panel to probe u · v · |v| · ω · recovered p/ρ")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
        });

        ui.add_space(16.0);
        ui.label(caps("Pressure Recovery"));
        ui.add_space(8.0);
        let mut recompute = false;
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if seg(ui, "Spectral", self.f2d_method == PMethod::Spectral)
                    && self.f2d_method != PMethod::Spectral
                {
                    self.f2d_method = PMethod::Spectral;
                    recompute = true;
                }
                if seg(ui, "FD (iterative)", self.f2d_method == PMethod::Fd)
                    && self.f2d_method != PMethod::Fd
                {
                    self.f2d_method = PMethod::Fd;
                    recompute = true;
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
                if ui
                    .add(
                        egui::Slider::new(&mut self.f2d_tol_exp, 2..=8)
                            .show_value(false)
                            .trailing_fill(true),
                    )
                    .changed()
                {
                    recompute = true;
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if seg(ui, "Periodic", self.f2d_boundary == PBoundary::Periodic)
                        && self.f2d_boundary != PBoundary::Periodic
                    {
                        self.f2d_boundary = PBoundary::Periodic;
                        recompute = true;
                    }
                    if seg(ui, "Dirichlet", self.f2d_boundary == PBoundary::Dirichlet)
                        && self.f2d_boundary != PBoundary::Dirichlet
                    {
                        self.f2d_boundary = PBoundary::Dirichlet;
                        recompute = true;
                    }
                });
            }
            ui.add_space(10.0);
            if action_button(
                ui,
                None,
                "RECOMPUTE RECOVERED PRESSURE",
                SURFACE_HIGH,
                TEXT,
                Some(OUTLINE),
                32.0,
                ui.available_width(),
            ) {
                recompute = true;
            }
            if let Some(f) = self.f2d.as_ref() {
                ui.add_space(10.0);
                let good = f.p_residual < 1e-3;
                diag(
                    ui,
                    &format!("Recovery error · {}", f.p_method),
                    &format!("{:.1e}", f.p_residual),
                    if good { SUCCESS } else { GOLD },
                );
                if f.p_iters > 0 {
                    diag(ui, "CG iterations", &format!("{}", f.p_iters), TEXT_DIM);
                }
                diag(
                    ui,
                    "Recovered max / min",
                    &format!("{:.2} / {:.2}", f.peak_p, f.low_p),
                    BRAND,
                );
            }
        });
        if recompute {
            self.request_2d();
        }
    }
}

fn benchmark_map_image(values: &[f32], n: usize, scale: f32, signed: bool) -> egui::ColorImage {
    if !signed {
        return field2d::magnitude_image(values, n, scale);
    }
    let pixels = values
        .iter()
        .map(|value| field2d::colormap_color((*value / scale.max(1e-12)).clamp(-1.0, 1.0), true))
        .collect();
    egui::ColorImage {
        size: [n, n],
        pixels,
        source_size: egui::Vec2::splat(n as f32),
    }
}

fn is_sha256(digest: &str) -> bool {
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn project_sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn benchmark_csv(benchmark: &engine::BenchResult) -> String {
    let mut csv = String::from("seed,provenance_stream,provenance_status,overlaps_reserved_stream");
    for horizon in &benchmark.horizons {
        csv += &format!(",rel_{horizon}x,persist_{horizon}x,improvement_{horizon}x");
    }
    csv.push('\n');
    for (seed_index, seed) in benchmark.seeds.iter().enumerate() {
        let record = benchmark
            .provenance
            .benchmark_seeds
            .iter()
            .find(|record| record.seed == *seed);
        let stream = record
            .map(|record| record.stream.as_str())
            .unwrap_or("unknown");
        let overlap = record.map(|record| record.overlap).unwrap_or(false);
        csv += &format!(
            "{seed},{stream},{},{}",
            benchmark.provenance.verdict, overlap
        );
        for horizon_index in 0..benchmark.horizons.len() {
            let rel = benchmark.rel[seed_index][horizon_index];
            let persist = benchmark.persist[seed_index][horizon_index];
            csv += &format!(",{rel:.6},{persist:.6},{:.6}", persist / rel.max(1e-12));
        }
        csv.push('\n');
    }
    csv
}

fn rng_stream_proposition(status: &str) -> &'static str {
    match status {
        "clean" => "no collision in checked RNG streams",
        "flagged" => "a checked RNG stream or provenance-contract conflict was found",
        _ => "RNG-stream collision status is unknown because required metadata is missing",
    }
}

fn benchmark_provenance_json(provenance: &engine::BenchProvenance) -> serde_json::Value {
    let checked_proposition = rng_stream_proposition(&provenance.verdict);
    let seeds: Vec<_> = provenance
        .benchmark_seeds
        .iter()
        .map(|record| {
            serde_json::json!({
                "seed": record.seed,
                "stream": record.stream,
                "overlap": record.overlap,
            })
        })
        .collect();
    serde_json::json!({
        "verdict": provenance.verdict,
        "checked_proposition": checked_proposition,
        "training_stream_seed": provenance.training_seed,
        "mixed_fork_stream_seed": provenance.mixed_fork_seed,
        "mixed_fork_used": provenance.mixed_fork_used,
        "validation_checkpoint_selection_stream_seed": provenance.validation_seed,
        "dataset": provenance.dataset,
        "benchmark_seeds": seeds,
        "overlap_count": provenance.overlap_count,
        "overlap_pct": provenance.overlap_pct,
        "checkpoint_epoch": provenance.epoch,
        "declared_epochs": provenance.declared_epochs,
        "checkpoint_role": provenance.checkpoint_role,
        "final_epoch_status": provenance.final_epoch_status,
        "selection_metric": provenance.selection_metric,
        "selection_stream": provenance.selection_stream,
        "source_fingerprint_present": provenance.source_fingerprint_present,
        "source_fingerprint_digest": provenance.source_fingerprint_digest,
        "legacy_unknown": provenance.legacy_unknown,
        "flags": provenance.flags,
        "validation_is_independent_test": false,
    })
}

fn benchmark_spatial_evidence_json(cell: &engine::BenchInspector) -> serde_json::Value {
    let variables: Vec<_> = InspectorVariable::ALL
        .iter()
        .filter_map(|&variable| {
            let (comparison_scale, error_scale) = cell.maps.scales(variable)?;
            let (mean, p95, max) = cell.maps.error_stats(variable)?;
            let (model_rms, reference_rms, error_rms) = cell.maps.rms(variable)?;
            Some(serde_json::json!({
                "key": variable.key(),
                "label": variable.label(),
                "symbol": variable.symbol(),
                "unit": variable.unit_key(),
                "signed": variable.signed(),
                "method": variable.method_note(),
                "sources": {
                    "model": variable.model_source(),
                    "solver_reference": variable.reference_source(),
                    "error": "DERIVED",
                },
                "comparison_scale": comparison_scale,
                "error_scale": error_scale,
                "rms": {
                    "model": model_rms,
                    "solver_reference": reference_rms,
                    "error": error_rms,
                },
                "absolute_error": {
                    "mean": mean,
                    "p95": p95,
                    "max": max,
                },
            }))
        })
        .collect();
    serde_json::json!({
        "schema": INSPECTOR_SCHEMA,
        "protocol_version": INSPECTOR_PROTOCOL_VERSION,
        "layout": INSPECTOR_LAYOUT,
        "domain": INSPECTOR_DOMAIN,
        "derivative": INSPECTOR_DERIVATIVE,
        "pressure": INSPECTOR_PRESSURE,
        "maps_embedded": false,
        "variables": variables,
    })
}

fn benchmark_report_card(
    benchmark: &engine::BenchResult,
    inspector: Option<&engine::BenchInspector>,
    run_id: &str,
    model_checkpoint_sha256: &str,
    timestamp_unix: u64,
) -> (String, String) {
    let selected_cell = inspector
        .filter(|cell| {
            benchmark.seeds.contains(&cell.seed) && benchmark.horizons.contains(&cell.horizon)
        })
        .map(|cell| {
            serde_json::json!({
                "seed": cell.seed,
                "horizon": cell.horizon,
                "seed_stream": cell.seed_stream,
                "provenance_status": cell.provenance_status,
                "provenance_checked_proposition": rng_stream_proposition(&cell.provenance_status),
                "rel_l2": cell.rel_l2,
                "persistence_rel_l2": cell.persist_rel_l2,
                "improvement_ratio": cell.improvement_ratio,
                "error_magnitude": {
                    "mean": cell.mean_abs_error,
                    "p95": cell.p95_abs_error,
                    "max": cell.max_abs_error,
                },
                "divergence_rms": {
                    "model": cell.divergence_model_rms,
                    "solver_reference": cell.divergence_truth_rms,
                    "error": cell.divergence_error_rms,
                },
                "energy_spectrum": {
                    "relative_l2": cell.spectrum_rel_l2,
                    "wavenumber": cell.spectrum_k,
                    "model": cell.spectrum_model,
                    "solver_reference": cell.spectrum_truth,
                },
                "spatial_variable_evidence": benchmark_spatial_evidence_json(cell),
            })
        });
    let warnings: Vec<_> = benchmark
        .provenance
        .flags
        .iter()
        .cloned()
        .chain(
            benchmark
                .provenance
                .legacy_unknown
                .iter()
                .map(|field| format!("UNKNOWN provenance field: {field}")),
        )
        .collect();
    let mut payload = serde_json::json!({
        "report_schema": crate::benchmark_export::REPORT_CARD_SCHEMA,
        "product": "reyn-studio",
        "version": env!("CARGO_PKG_VERSION"),
        "run_id": run_id,
        "protocol_id": "reyn.benchmark.seed-horizon.v1",
        "model": benchmark.model,
        "model_checkpoint_sha256": model_checkpoint_sha256,
        "grid": benchmark.grid,
        "epoch": benchmark.epoch,
        "timestamp_unix": timestamp_unix,
        "seeds": benchmark.seeds,
        "horizons": benchmark.horizons,
        "rel_l2": benchmark.rel,
        "persistence": benchmark.persist,
        "global_rel_l2": benchmark.global_rel,
        "runtime_s": benchmark.runtime_s,
        "protocol": "exact benchmark RNG seeds; training, mixed-fork (+10000), and validation/checkpoint-selection (+50000) streams classified from checkpoint metadata; validation is never independent test evidence",
        "provenance": benchmark_provenance_json(&benchmark.provenance),
        "selected_cell_evidence": selected_cell,
        "warnings": warnings,
        "limitations": [
            "a numerical solver output is a solver reference, not automatically physical truth",
            "recovered pressure is density-normalized and is not physical Cp without a recorded reference state",
            "RNG-stream non-collision is not field-space or trajectory non-overlap",
            "consistency, provenance, integrity, authenticity, and independent accuracy/validation are separate claims",
        ],
        "integrity_algorithm": "SHA-256",
        "authenticity": {
            "status": "UNSIGNED",
            "signature": null,
            "reason": "no organization-key signature was produced",
        },
        "verification": {
            "canonical_payload": "remove only integrity_sha256, then serialize compact UTF-8 JSON with object keys in lexical order",
            "integrity": "compute SHA-256 over the canonical payload and compare with integrity_sha256",
            "authenticity": "UNSIGNED; no organization identity is asserted by the JSON, PNG, or PDF",
        },
        "canonicalization": "SHA-256 over compact UTF-8 JSON for this object before integrity_sha256 is inserted; object keys use serde_json lexical order",
    });
    // Normalize f32-backed JSON numbers through the exact UTF-8 representation
    // a verifier will parse. Hashing the pre-serialization Value can otherwise
    // differ after a selected-cell payload is written and read as JSON.
    payload = serde_json::from_slice(
        &serde_json::to_vec(&payload).expect("report payload is JSON-serializable"),
    )
    .expect("serialized report payload parses");
    // Normalize through JSON once so a verifier that parses the exported
    // pretty JSON reconstructs byte-identical canonical numbers.
    let first_pass = serde_json::to_vec(&payload).expect("report payload is JSON-serializable");
    payload = serde_json::from_slice(&first_pass).expect("serialized report payload is valid JSON");
    let canonical = serde_json::to_vec(&payload).expect("report payload is JSON-serializable");
    use sha2::Digest;
    let digest = sha2::Sha256::digest(&canonical);
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    payload
        .as_object_mut()
        .expect("report payload is an object")
        .insert(
            "integrity_sha256".into(),
            serde_json::Value::String(hex.clone()),
        );
    let mut report =
        serde_json::to_string_pretty(&payload).expect("report payload is JSON-serializable");
    report.push('\n');
    (report, hex)
}

fn stream_label(stream: &str) -> &'static str {
    match stream {
        "training" => "TRAINING",
        "mixed_fork" => "MIXED FORK",
        "validation_selection" => "VALIDATION / SELECTION",
        "fresh_test" => "FRESH TEST",
        _ => "UNKNOWN",
    }
}

fn stream_color(stream: &str) -> Color32 {
    match stream {
        "fresh_test" => SUCCESS,
        "training" | "mixed_fork" | "validation_selection" => DATA_RED,
        _ => GOLD,
    }
}

fn provenance_color(status: &str) -> Color32 {
    match status {
        "clean" => SUCCESS,
        "flagged" => DATA_RED,
        _ => GOLD,
    }
}

fn paint_bench_provenance(p: &egui::Painter, rect: Rect, provenance: &engine::BenchProvenance) {
    p.rect_filled(rect, CornerRadius::same(3), SURFACE);
    p.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0, OUTLINE_VARIANT),
        egui::StrokeKind::Inside,
    );
    let x = rect.min.x + 14.0;
    let mut y = rect.min.y + 13.0;
    p.text(
        egui::pos2(x, y),
        Align2::LEFT_TOP,
        "LEAK & PROVENANCE",
        FontId::monospace(9.5),
        TEXT_MUTE,
    );
    let mark = match provenance.verdict.as_str() {
        "clean" => "✓ RNG STREAMS CLEAN",
        "flagged" => "! FLAGGED",
        _ => "? UNKNOWN",
    };
    p.text(
        egui::pos2(rect.max.x - 14.0, y),
        Align2::RIGHT_TOP,
        mark,
        FontId::monospace(10.0),
        provenance_color(&provenance.verdict),
    );
    y += 28.0;

    let mut row = |label: &str, value: String, color: Color32| {
        p.text(
            egui::pos2(x, y),
            Align2::LEFT_TOP,
            label,
            FontId::proportional(9.5),
            TEXT_MUTE,
        );
        p.text(
            egui::pos2(rect.max.x - 14.0, y),
            Align2::RIGHT_TOP,
            value,
            FontId::monospace(9.5),
            color,
        );
        y += 22.0;
    };
    let seed = |value: Option<i64>| {
        value
            .map(|number| number.to_string())
            .unwrap_or_else(|| "unknown".into())
    };
    row("training stream", seed(provenance.training_seed), TEXT_DIM);
    row(
        "mixed-fork stream",
        format!(
            "{} · {}",
            seed(provenance.mixed_fork_seed),
            if provenance.mixed_fork_used {
                "used"
            } else {
                "reserved"
            }
        ),
        if provenance.mixed_fork_used {
            TEXT_DIM
        } else {
            TEXT_MUTE
        },
    );
    row(
        "validation / selection",
        seed(provenance.validation_seed),
        GOLD,
    );
    let fresh = provenance
        .benchmark_seeds
        .iter()
        .filter(|record| record.stream == "fresh_test")
        .count();
    row(
        "benchmark seeds",
        format!(
            "{fresh} fresh · {} overlap ({:.0}%)",
            provenance.overlap_count, provenance.overlap_pct
        ),
        if provenance.overlap_count == 0 {
            SUCCESS
        } else {
            DATA_RED
        },
    );
    let epoch = match (provenance.epoch, provenance.declared_epochs) {
        (Some(epoch), Some(declared)) => format!("{epoch}/{declared}"),
        (Some(epoch), None) => format!("{epoch}/?"),
        _ => "unknown".into(),
    };
    row(
        "final epoch",
        format!(
            "{epoch} · {}",
            provenance.final_epoch_status.replace('_', " ")
        ),
        if provenance.final_epoch_status == "fixed_final" {
            SUCCESS
        } else {
            GOLD
        },
    );
    let source = provenance
        .source_fingerprint_digest
        .as_deref()
        .map(|digest| {
            let short: String = digest.chars().take(10).collect();
            format!("present · {short}…")
        })
        .unwrap_or_else(|| "absent · legacy unknown".into());
    row(
        "source fingerprint",
        source,
        if provenance.source_fingerprint_present {
            SUCCESS
        } else {
            GOLD
        },
    );
    row(
        "checkpoint selection",
        format!(
            "{} · {}",
            provenance.selection_stream, provenance.checkpoint_role
        ),
        if provenance.selection_stream == "validation" {
            GOLD
        } else {
            TEXT_DIM
        },
    );
    p.text(
        egui::pos2(x, y + 1.0),
        Align2::LEFT_TOP,
        "Validation selects checkpoints; it is not independent testing.",
        FontId::proportional(9.0),
        GOLD,
    );
    y += 25.0;
    let finding = if !provenance.flags.is_empty() {
        format!("! {} protocol flag(s)", provenance.flags.len())
    } else if !provenance.legacy_unknown.is_empty() {
        format!(
            "? {} legacy metadata gap(s)",
            provenance.legacy_unknown.len()
        )
    } else {
        "✓ no collision in checked RNG streams".into()
    };
    p.text(
        egui::pos2(x, y),
        Align2::LEFT_TOP,
        finding,
        FontId::monospace(9.0),
        provenance_color(&provenance.verdict),
    );
}

fn paint_evidence_state(p: &egui::Painter, rect: Rect, title: &str, detail: &str, color: Color32) {
    p.rect_filled(rect, CornerRadius::same(3), SURFACE);
    p.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0, OUTLINE_VARIANT),
        egui::StrokeKind::Inside,
    );
    p.text(
        rect.left_center() + Vec2::new(16.0, -12.0),
        Align2::LEFT_CENTER,
        title,
        FontId::monospace(10.0),
        color,
    );
    p.text(
        rect.left_center() + Vec2::new(16.0, 13.0),
        Align2::LEFT_CENTER,
        detail,
        FontId::proportional(10.5),
        TEXT_MUTE,
    );
}

fn paint_energy_spectrum(p: &egui::Painter, rect: Rect, cell: &engine::BenchInspector) {
    p.rect_filled(rect, CornerRadius::same(2), SURFACE_LOWEST);
    p.rect_stroke(
        rect,
        CornerRadius::same(2),
        Stroke::new(1.0, OUTLINE_VARIANT),
        egui::StrokeKind::Inside,
    );
    if cell.spectrum_k.len() < 2 {
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "no spectral bins",
            FontId::monospace(9.0),
            TEXT_MUTE,
        );
        return;
    }
    let plot = Rect::from_min_max(
        rect.min + Vec2::new(34.0, 13.0),
        rect.max - Vec2::new(9.0, 23.0),
    );
    let k_min = cell.spectrum_k[0].max(1e-6).log10();
    let k_max = cell
        .spectrum_k
        .last()
        .copied()
        .unwrap_or(1.0)
        .max(1e-6)
        .log10();
    let (mut e_min, mut e_max) = (f32::INFINITY, f32::NEG_INFINITY);
    for energy in cell.spectrum_model.iter().chain(&cell.spectrum_truth) {
        let value = energy.max(1e-20).log10();
        e_min = e_min.min(value);
        e_max = e_max.max(value);
    }
    if !e_min.is_finite() || !e_max.is_finite() {
        return;
    }
    if (e_max - e_min).abs() < 1e-6 {
        e_min -= 1.0;
        e_max += 1.0;
    }
    let point = |k: f32, energy: f32| {
        let tx = (k.max(1e-6).log10() - k_min) / (k_max - k_min).max(1e-6);
        let ty = (energy.max(1e-20).log10() - e_min) / (e_max - e_min);
        egui::pos2(
            egui::lerp(plot.x_range(), tx.clamp(0.0, 1.0)),
            egui::lerp(plot.y_range(), 1.0 - ty.clamp(0.0, 1.0)),
        )
    };
    for fraction in [0.0, 0.5, 1.0] {
        let gy = egui::lerp(plot.y_range(), fraction);
        p.line_segment(
            [egui::pos2(plot.min.x, gy), egui::pos2(plot.max.x, gy)],
            Stroke::new(1.0, OUTLINE_VARIANT.gamma_multiply(0.45)),
        );
    }
    let model: Vec<_> = cell
        .spectrum_k
        .iter()
        .zip(&cell.spectrum_model)
        .map(|(&k, &energy)| point(k, energy))
        .collect();
    let truth: Vec<_> = cell
        .spectrum_k
        .iter()
        .zip(&cell.spectrum_truth)
        .map(|(&k, &energy)| point(k, energy))
        .collect();
    p.add(egui::Shape::line(model, Stroke::new(1.5, EMBER)));
    p.add(egui::Shape::line(truth.clone(), Stroke::new(1.5, TERTIARY)));
    for marker in truth.iter().step_by((truth.len() / 6).max(1)) {
        p.circle_stroke(*marker, 2.0, Stroke::new(1.0, TERTIARY));
    }
    p.text(
        rect.min + Vec2::new(8.0, 8.0),
        Align2::LEFT_TOP,
        "MODEL —",
        FontId::monospace(7.5),
        EMBER,
    );
    p.text(
        rect.min + Vec2::new(70.0, 8.0),
        Align2::LEFT_TOP,
        "SOLVER REF ○",
        FontId::monospace(7.5),
        TERTIARY,
    );
    p.text(
        egui::pos2(plot.center().x, rect.max.y - 4.0),
        Align2::CENTER_BOTTOM,
        format!(
            "wavenumber k · 1 → {:.0}",
            cell.spectrum_k.last().unwrap_or(&1.0)
        ),
        FontId::monospace(7.5),
        TEXT_MUTE,
    );
    p.text(
        egui::pos2(rect.min.x + 5.0, plot.center().y),
        Align2::LEFT_CENTER,
        "log E",
        FontId::monospace(7.5),
        TEXT_MUTE,
    );
}

// -- field instrument layer (insights · probe · legend) -----------------------

/// DESIGN.md reserves gold / blue / green / red for data semantics — this is
/// that vocabulary. Ember (the action color) marks the model-error hotspot.
fn insight_color(kind: field2d::InsightKind) -> Color32 {
    match kind {
        field2d::InsightKind::PeakPressure => DATA_RED,
        field2d::InsightKind::SuctionPeak => TERTIARY,
        field2d::InsightKind::MaxVorticity => GOLD,
        field2d::InsightKind::MaxSpeed => SUCCESS,
        field2d::InsightKind::MaxError => EMBER,
    }
}

/// Same vocabulary in 3D: gold rotation, green speed, blue for the vortex core
/// (Q-max ≈ the low-pressure core — the same semantic as 2D's suction blue).
fn insight3d_color(kind: flow::Insight3DKind) -> Color32 {
    match kind {
        flow::Insight3DKind::VortexCore => TERTIARY,
        flow::Insight3DKind::MaxVorticity => GOLD,
        flow::Insight3DKind::MaxSpeed => SUCCESS,
        flow::Insight3DKind::SurfLoad => DATA_RED, // stagnation: where the flow pushes
        flow::Insight3DKind::SurfSuction => TERTIARY, // suction peak: the weak point
    }
}

/// One pinned 2D critical point: grid cell → panel position, then the shared
/// instrument marker (`viewport::insight_marker`).
fn draw_insight(
    p: &egui::Painter,
    panel: Rect,
    n: usize,
    ins: &field2d::Insight,
    chips: &mut Vec<Rect>,
) {
    let c = egui::pos2(
        panel.min.x + (ins.j as f32 + 0.5) / n as f32 * panel.width(),
        panel.min.y + (ins.i as f32 + 0.5) / n as f32 * panel.height(),
    );
    viewport::insight_marker(
        p,
        panel,
        c,
        insight_color(ins.kind),
        &format!("{} {:.3}", ins.kind.glyph(), ins.value),
        chips,
    );
}

fn blend_color(base: Color32, overlay: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let channel = |base: u8, overlay: u8| {
        (base as f32 + (overlay as f32 - base as f32) * amount).round() as u8
    };
    Color32::from_rgb(
        channel(base.r(), overlay.r()),
        channel(base.g(), overlay.g()),
        channel(base.b(), overlay.b()),
    )
}

fn engineering_section_image(section: &engineering_section::SectionPlane) -> egui::ColorImage {
    let n = section.n;
    let is_boundary = |row: usize, column: usize| {
        let solid = section.mask_value(row, column) >= 0.5;
        [
            row.checked_sub(1).map(|neighbor| (neighbor, column)),
            (row + 1 < n).then_some((row + 1, column)),
            column.checked_sub(1).map(|neighbor| (row, neighbor)),
            (column + 1 < n).then_some((row, column + 1)),
        ]
        .into_iter()
        .flatten()
        .any(|(neighbor_row, neighbor_column)| {
            (section.mask_value(neighbor_row, neighbor_column) >= 0.5) != solid
        })
    };
    let mut pixels = Vec::with_capacity(n * n);
    for row in 0..n {
        for column in 0..n {
            let mask = section.mask_value(row, column).clamp(0.0, 1.0);
            let base = field2d::colormap_color(
                section.scale.normalize(section.value(row, column)),
                section.quantity.signed(),
            );
            let color = if is_boundary(row, column) {
                BRAND
            } else if mask >= 0.5 {
                SURFACE_LOWEST
            } else if mask > 0.01 {
                blend_color(base, SURFACE_LOWEST, mask * 0.7)
            } else {
                base
            };
            pixels.push(color);
        }
    }
    egui::ColorImage {
        size: [n, n],
        pixels,
        source_size: Vec2::new(n as f32, n as f32),
    }
}

fn format_section_value(value: f32, units: &str) -> String {
    let magnitude = value.abs();
    if magnitude >= 10_000.0 || (magnitude > 0.0 && magnitude < 0.001) {
        format!("{value:.4e} {units}")
    } else {
        format!("{value:.4} {units}")
    }
}

fn engineering_section_legend(
    painter: &egui::Painter,
    panel: Rect,
    section: &engineering_section::SectionPlane,
) {
    let bar = Rect::from_min_size(
        egui::pos2(panel.min.x, panel.max.y + 36.0),
        Vec2::new(panel.width(), 8.0),
    );
    let signed = section.quantity.signed();
    let strips = 64;
    for strip in 0..strips {
        let fraction = (strip as f32 + 0.5) / strips as f32;
        let normalized = if signed {
            fraction * 2.0 - 1.0
        } else {
            fraction
        };
        let x0 = bar.min.x + bar.width() * strip as f32 / strips as f32;
        let x1 = bar.min.x + bar.width() * (strip + 1) as f32 / strips as f32;
        painter.rect_filled(
            Rect::from_min_max(egui::pos2(x0, bar.min.y), egui::pos2(x1, bar.max.y)),
            CornerRadius::ZERO,
            field2d::colormap_color(normalized, signed),
        );
    }
    painter.rect_stroke(
        bar,
        CornerRadius::ZERO,
        Stroke::new(1.0, OUTLINE_VARIANT),
        egui::StrokeKind::Outside,
    );
    let font = FontId::monospace(9.5);
    let label_y = bar.max.y + 4.0;
    painter.text(
        egui::pos2(bar.min.x, label_y),
        Align2::LEFT_TOP,
        format_section_value(section.scale.legend_minimum(), section.quantity.units()),
        font.clone(),
        TEXT_MUTE,
    );
    if let Some(center) = section.scale.center {
        painter.text(
            egui::pos2(bar.center().x, label_y),
            Align2::CENTER_TOP,
            format_section_value(center, section.quantity.units()),
            font.clone(),
            TEXT_MUTE,
        );
    }
    painter.text(
        egui::pos2(bar.max.x, label_y),
        Align2::RIGHT_TOP,
        format_section_value(section.scale.legend_maximum(), section.quantity.units()),
        font,
        TEXT_MUTE,
    );
}

fn draw_engineering_section_probe(
    painter: &egui::Painter,
    panel: Rect,
    clip: Rect,
    section: &engineering_section::SectionPlane,
    position: egui::Pos2,
) {
    let column = (((position.x - panel.min.x) / panel.width() * section.n as f32) as usize)
        .min(section.n - 1);
    let row = (((position.y - panel.min.y) / panel.height() * section.n as f32) as usize)
        .min(section.n - 1);
    let center = egui::pos2(
        panel.min.x + (column as f32 + 0.5) / section.n as f32 * panel.width(),
        panel.min.y + (row as f32 + 0.5) / section.n as f32 * panel.height(),
    );
    let crosshair = Stroke::new(1.0, OUTLINE.gamma_multiply(0.55));
    painter.line_segment(
        [
            egui::pos2(panel.min.x, center.y),
            egui::pos2(panel.max.x, center.y),
        ],
        crosshair,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, panel.min.y),
            egui::pos2(center.x, panel.max.y),
        ],
        crosshair,
    );
    painter.circle_stroke(center, 4.0, Stroke::new(1.0, TEXT_DIM));

    let mask = section.mask_value(row, column);
    let lines = [
        (
            format!(
                "{} cell {} · pixel {},{}",
                section.axis.label(),
                section.index,
                column,
                row
            ),
            TEXT_MUTE,
        ),
        (
            format_section_value(section.value(row, column), section.quantity.units()),
            TEXT,
        ),
        (
            if mask >= 0.5 {
                format!("stored mask {mask:.3} · SOLID GEOMETRY")
            } else {
                format!("stored mask {mask:.3} · FLUID")
            },
            if mask >= 0.5 { BRAND } else { TEXT_DIM },
        ),
    ];
    let font = FontId::monospace(10.0);
    let galleys = lines
        .iter()
        .map(|(text, color)| painter.layout_no_wrap(text.clone(), font.clone(), *color))
        .collect::<Vec<_>>();
    let width = galleys
        .iter()
        .map(|galley| galley.size().x)
        .fold(0.0, f32::max)
        + 16.0;
    let line_height = galleys[0].size().y + 3.0;
    let size = Vec2::new(width, line_height * galleys.len() as f32 + 10.0);
    let mut origin = position + Vec2::new(14.0, 14.0);
    if origin.x + size.x > clip.max.x - 8.0 {
        origin.x = position.x - size.x - 14.0;
    }
    if origin.y + size.y > clip.max.y - 8.0 {
        origin.y = position.y - size.y - 14.0;
    }
    let chip = Rect::from_min_size(origin, size);
    painter.rect_filled(chip, CornerRadius::same(2), SURFACE);
    painter.rect_stroke(
        chip,
        CornerRadius::same(2),
        Stroke::new(1.0, OUTLINE_VARIANT),
        egui::StrokeKind::Inside,
    );
    for (line, galley) in galleys.into_iter().enumerate() {
        painter.galley(
            origin + Vec2::new(8.0, 5.0 + line as f32 * line_height),
            galley,
            lines[line].1,
        );
    }
}

/// Calibrated colormap legend: gradient bar + labeled range (shared
/// model/reference scale, so the two panels read on the same instrument).
fn legend_bar(p: &egui::Painter, panel: Rect, scale: f32, signed: bool) {
    let bar = Rect::from_min_size(
        egui::pos2(panel.min.x, panel.max.y + 24.0),
        Vec2::new(panel.width(), 8.0),
    );
    let strips = 48;
    for k in 0..strips {
        let t = (k as f32 + 0.5) / strips as f32;
        let val = if signed { t * 2.0 - 1.0 } else { t };
        let x0 = bar.min.x + bar.width() * k as f32 / strips as f32;
        let x1 = bar.min.x + bar.width() * (k + 1) as f32 / strips as f32;
        p.rect_filled(
            Rect::from_min_max(egui::pos2(x0, bar.min.y), egui::pos2(x1, bar.max.y)),
            CornerRadius::ZERO,
            field2d::colormap_color(val, signed),
        );
    }
    p.rect_stroke(
        bar,
        CornerRadius::ZERO,
        Stroke::new(1.0, OUTLINE_VARIANT),
        egui::StrokeKind::Outside,
    );
    let y = bar.max.y + 4.0;
    let font = FontId::monospace(10.0);
    if signed {
        p.text(
            egui::pos2(bar.min.x, y),
            Align2::LEFT_TOP,
            format!("-{scale:.2}"),
            font.clone(),
            TEXT_MUTE,
        );
        p.text(
            egui::pos2(bar.center().x, y),
            Align2::CENTER_TOP,
            "0",
            font.clone(),
            TEXT_MUTE,
        );
        p.text(
            egui::pos2(bar.max.x, y),
            Align2::RIGHT_TOP,
            format!("+{scale:.2}"),
            font,
            TEXT_MUTE,
        );
    } else {
        p.text(
            egui::pos2(bar.min.x, y),
            Align2::LEFT_TOP,
            "0",
            font.clone(),
            TEXT_MUTE,
        );
        p.text(
            egui::pos2(bar.max.x, y),
            Align2::RIGHT_TOP,
            format!("{scale:.2}"),
            font,
            TEXT_MUTE,
        );
    }
}

/// Live hover probe: crosshair at the cell + a full readout chip (u, v, |v|,
/// ω, recovered p/ρ) beside the cursor. Native-side sampling — zero latency, no engine
/// round-trip.
fn draw_probe(p: &egui::Painter, panel: Rect, clip: Rect, n: usize, src: &[f32], pos: egui::Pos2) {
    let j = (((pos.x - panel.min.x) / panel.width() * n as f32) as usize).min(n - 1);
    let i = (((pos.y - panel.min.y) / panel.height() * n as f32) as usize).min(n - 1);
    let cx = panel.min.x + (j as f32 + 0.5) / n as f32 * panel.width();
    let cy = panel.min.y + (i as f32 + 0.5) / n as f32 * panel.height();
    let cross = Stroke::new(1.0, OUTLINE.gamma_multiply(0.45));
    p.line_segment(
        [egui::pos2(panel.min.x, cy), egui::pos2(panel.max.x, cy)],
        cross,
    );
    p.line_segment(
        [egui::pos2(cx, panel.min.y), egui::pos2(cx, panel.max.y)],
        cross,
    );
    p.circle_stroke(egui::pos2(cx, cy), 4.0, Stroke::new(1.0, TEXT_DIM));

    let s = field2d::probe(src, n, i, j);
    let font = FontId::monospace(10.5);
    let lines = [
        (format!("cell {j},{i}"), TEXT_MUTE),
        (format!("u   {:+.3}", s.u), TEXT_DIM),
        (format!("v   {:+.3}", s.v), TEXT_DIM),
        (format!("|v| {:.3}", s.speed), TEXT),
        (format!("ω   {:+.3}", s.omega), TEXT),
        (format!("p/ρ {:+.3}", s.p), TEXT),
    ];
    let galleys: Vec<_> = lines
        .iter()
        .map(|(t, c)| p.layout_no_wrap(t.clone(), font.clone(), *c))
        .collect();
    let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + 16.0;
    let line_h = galleys[0].size().y + 2.0;
    let h = line_h * galleys.len() as f32 + 10.0;
    let mut anchor = pos + Vec2::new(16.0, 12.0);
    if anchor.x + w > clip.max.x - 4.0 {
        anchor.x = pos.x - 16.0 - w;
    }
    if anchor.y + h > clip.max.y - 4.0 {
        anchor.y = pos.y - 12.0 - h;
    }
    let chip = Rect::from_min_size(anchor, Vec2::new(w, h));
    p.rect_filled(chip, CornerRadius::same(3), SURFACE);
    p.rect_stroke(
        chip,
        CornerRadius::same(3),
        Stroke::new(1.0, OUTLINE),
        egui::StrokeKind::Inside,
    );
    let colors = [TEXT_MUTE, TEXT_DIM, TEXT_DIM, TEXT, TEXT, TEXT];
    for (k, g) in galleys.into_iter().enumerate() {
        p.galley(
            chip.min + Vec2::new(8.0, 5.0 + k as f32 * line_h),
            g,
            colors[k],
        );
    }
}

// -- reusable widgets --------------------------------------------------------
fn with_project_extension(mut path: std::path::PathBuf) -> std::path::PathBuf {
    let is_project = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("reynproj"));
    if !is_project {
        path.set_extension("reynproj");
    }
    path
}

fn project_fact(ui: &mut egui::Ui, label: &str, value: &str) {
    Frame::NONE
        .fill(SURFACE_LOW)
        .corner_radius(CornerRadius::same(2))
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("{label}  {value}"))
                    .text_style(mono_s())
                    .color(TEXT_DIM),
            );
        });
}

fn seg(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    // Selection is tonal (bg-4 thumb + primary text), not accent-filled —
    // ember stays reserved for the screen's one primary action.
    let (fill, color) = if active {
        (SURFACE_HIGHEST, TEXT)
    } else {
        (Color32::TRANSPARENT, TEXT_DIM)
    };
    ui.add(
        egui::Button::new(RichText::new(label).text_style(body_strong()).color(color))
            .fill(fill)
            .corner_radius(CornerRadius::same(R1))
            .stroke(Stroke::NONE),
    )
    .clicked()
}

fn short_id(value: &str) -> String {
    value.chars().take(8).collect()
}

fn short_hash(value: &str) -> String {
    let prefix: String = value.chars().take(12).collect();
    if value.chars().count() > 12 {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn format_stages(stages: &[project::DependencyStage]) -> String {
    stages
        .iter()
        .map(|stage| match stage {
            project::DependencyStage::Contract => "contract",
            project::DependencyStage::Discretization => "discretization",
            project::DependencyStage::Run => "run",
            project::DependencyStage::Evidence => "evidence",
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn source_kind_label(kind: project::SourceKind) -> &'static str {
    match kind {
        project::SourceKind::Geometry => "GEOMETRY",
        project::SourceKind::Model => "MODEL",
        project::SourceKind::Reference => "REFERENCE",
    }
}

fn dependency_kind_label(kind: project_lifecycle::DependencyKind) -> &'static str {
    match kind {
        project_lifecycle::DependencyKind::Engine => "ENGINE",
        project_lifecycle::DependencyKind::Model => "MODEL",
        project_lifecycle::DependencyKind::Source => "SOURCE",
        project_lifecycle::DependencyKind::Artifact => "ARTIFACT",
        project_lifecycle::DependencyKind::Integrity => "INTEGRITY",
    }
}

fn content_state_presentation(state: project::ContentState) -> (&'static str, Color32) {
    match state {
        project::ContentState::Available => ("BUNDLED · VERIFIED", SUCCESS),
        project::ContentState::Missing => ("BUNDLE OBJECT MISSING", WARN),
        project::ContentState::Corrupt => ("BUNDLE OBJECT CORRUPT", DATA_RED),
    }
}

fn determinism_status_label(status: project::DeterminismStatus) -> &'static str {
    match status {
        project::DeterminismStatus::WithinTolerance => "WITHIN DECLARED TOLERANCE",
        project::DeterminismStatus::Difference => "DIFFERENCE EXPOSED",
        project::DeterminismStatus::NotComparable => "NOT COMPARABLE",
    }
}

fn determinism_status_color(status: project::DeterminismStatus) -> Color32 {
    match status {
        project::DeterminismStatus::WithinTolerance => SUCCESS,
        project::DeterminismStatus::Difference => DATA_RED,
        project::DeterminismStatus::NotComparable => WARN,
    }
}

fn optional_scalar(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.6e}"))
        .unwrap_or_else(|| "missing".into())
}

/// Lifecycle state glyph for a workflow stage: shape + color always travel
/// together (never color-only, SCI honesty rules).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StageGlyph {
    Complete,
    Partial,
    Empty,
}

impl StageGlyph {
    fn symbol(self) -> &'static str {
        match self {
            Self::Complete => "●",
            Self::Partial => "◐",
            Self::Empty => "○",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Complete => SUCCESS,
            // WARN is the status hue (§3.3); GOLD stays reserved for data.
            Self::Partial => WARN,
            Self::Empty => TEXT_MUTE,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct StageState {
    glyph: StageGlyph,
    /// `Some(reason)` disables the stage; the reason renders inline (A9) and
    /// the click never fires — no silent redirect (A5).
    blocked: Option<&'static str>,
}

/// Pure lifecycle model for the workflow rail (§4.1). Order: Project, Case
/// Setup, Results, Evidence. Results is the only gated stage — it requires a
/// completed immutable run in the open case.
fn workflow_stage_states(
    project_has_content: bool,
    case_open: bool,
    has_result: bool,
    has_runs: bool,
) -> [StageState; 4] {
    let case_glyph = if has_result {
        StageGlyph::Complete
    } else if case_open {
        StageGlyph::Partial
    } else {
        StageGlyph::Empty
    };
    [
        StageState {
            glyph: if project_has_content {
                StageGlyph::Complete
            } else {
                StageGlyph::Empty
            },
            blocked: None,
        },
        StageState {
            glyph: case_glyph,
            blocked: None,
        },
        StageState {
            glyph: if has_result {
                StageGlyph::Complete
            } else {
                StageGlyph::Empty
            },
            blocked: (!has_result).then_some("needs a completed run"),
        },
        StageState {
            glyph: if has_runs {
                StageGlyph::Complete
            } else {
                StageGlyph::Empty
            },
            blocked: None,
        },
    ]
}

/// Workflow-stage nav row: icon + label + right-aligned state glyph, an
/// inline blocked-reason line when gated, animated hover, ember edge marker
/// when active, focus ring for keyboard navigation.
fn stage_row(ui: &mut egui::Ui, icon: &str, label: &str, active: bool, state: StageState) -> bool {
    let blocked = state.blocked.is_some();
    // §3.2 rhythm: nav rows are 32px; a blocked row adds one caption line.
    let height = if blocked { 48.0 } else { 32.0 };
    // Blocked rows stay focusable (Sense::click) so the inline reason is
    // reachable by keyboard; activation is ignored below (QA C11).
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    let hover = motion_t(
        ui.ctx(),
        resp.id.with("hover"),
        resp.hovered() && !active && !blocked,
        0.14,
    );
    let label_font = body_strong().resolve(ui.style());
    let small_font = egui::TextStyle::Small.resolve(ui.style());
    let painter = ui.painter();
    let fill = if active {
        SURFACE_HIGH
    } else {
        SURFACE.gamma_multiply(hover)
    };
    painter.rect_filled(rect, CornerRadius::same(R1), fill);
    if active {
        // 2px ember edge marker — ember is a mark, never a nav fill (§3.3).
        painter.rect_filled(
            Rect::from_min_size(rect.min + Vec2::new(0.0, 6.0), Vec2::new(2.0, 32.0 - 12.0)),
            CornerRadius::same(1),
            EMBER,
        );
    }
    if resp.has_focus() {
        painter.rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(R1),
            focus_stroke(),
            egui::StrokeKind::Outside,
        );
    }
    let fg = if blocked {
        TEXT_MUTE
    } else if active || resp.hovered() {
        TEXT
    } else {
        TEXT_DIM
    };
    let row_y = rect.min.y + 16.0;
    painter.text(
        egui::pos2(rect.min.x + 12.0, row_y),
        Align2::LEFT_CENTER,
        icon,
        FontId::proportional(15.0),
        fg,
    );
    painter.text(
        egui::pos2(rect.min.x + 36.0, row_y),
        Align2::LEFT_CENTER,
        label,
        label_font,
        fg,
    );
    painter.text(
        egui::pos2(rect.max.x - 12.0, row_y),
        Align2::RIGHT_CENTER,
        state.glyph.symbol(),
        FontId::proportional(11.0),
        state.glyph.color(),
    );
    if let Some(reason) = state.blocked {
        painter.text(
            egui::pos2(rect.min.x + 36.0, rect.min.y + 36.0),
            Align2::LEFT_CENTER,
            format!("○ {reason}"),
            small_font,
            TEXT_MUTE,
        );
    }
    !blocked && resp.clicked()
}

/// Destination nav row (Model Library, Settings, sandbox tools): no lifecycle
/// glyph, same active/hover/focus grammar as [`stage_row`].
fn nav_row(ui: &mut egui::Ui, icon: &str, label: &str, active: bool) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::click());
    let hover = motion_t(
        ui.ctx(),
        resp.id.with("hover"),
        resp.hovered() && !active,
        0.14,
    );
    let label_font = body_strong().resolve(ui.style());
    let painter = ui.painter();
    let fill = if active {
        SURFACE_HIGH
    } else {
        SURFACE.gamma_multiply(hover)
    };
    painter.rect_filled(rect, CornerRadius::same(R1), fill);
    if active {
        painter.rect_filled(
            Rect::from_min_size(
                rect.min + Vec2::new(0.0, 6.0),
                Vec2::new(2.0, rect.height() - 12.0),
            ),
            CornerRadius::same(1),
            EMBER,
        );
    }
    if resp.has_focus() {
        painter.rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(R1),
            focus_stroke(),
            egui::StrokeKind::Outside,
        );
    }
    let fg = if active || resp.hovered() {
        TEXT
    } else {
        TEXT_DIM
    };
    painter.text(
        egui::pos2(rect.min.x + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        icon,
        FontId::proportional(15.0),
        fg,
    );
    painter.text(
        egui::pos2(rect.min.x + 36.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        label_font,
        fg,
    );
    resp.clicked()
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", url])
        .spawn();
}

/// Centered icon+label button. `border` gives a ghost style. Hover and press
/// animate per §3.7 (ease-out fills, 1px press inset) and keyboard focus
/// draws the ember focus ring.
fn action_button(
    ui: &mut egui::Ui,
    icon: Option<&str>,
    label: &str,
    fill: Color32,
    fg: Color32,
    border: Option<Color32>,
    height: f32,
    width: f32,
) -> bool {
    action_button_gated(ui, icon, label, fill, fg, border, height, width, None)
}

/// [`action_button`] with disabled-with-reason support: when `disabled_reason`
/// is set, the button renders quiet/tonal, never fires, and explains itself
/// on hover (UX-AC-01 — a control is never dead without a reason).
#[allow(clippy::too_many_arguments)]
fn action_button_gated(
    ui: &mut egui::Ui,
    icon: Option<&str>,
    label: &str,
    fill: Color32,
    fg: Color32,
    border: Option<Color32>,
    height: f32,
    width: f32,
    disabled_reason: Option<&str>,
) -> bool {
    let disabled = disabled_reason.is_some();
    let (fill, fg, border) = if disabled {
        (SURFACE_HIGH, TEXT_MUTE, Some(HAIRLINE))
    } else {
        (fill, fg, border)
    };
    let sense = if disabled {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), sense);
    if let Some(reason) = disabled_reason {
        let resp = resp.clone().on_hover_text(reason);
        let press = 0.0;
        let painter = ui.painter();
        let draw_rect = rect.shrink(press);
        painter.rect_filled(draw_rect, CornerRadius::same(R1), fill);
        if let Some(b) = border {
            painter.rect_stroke(
                draw_rect,
                CornerRadius::same(R1),
                Stroke::new(1.0, b),
                egui::StrokeKind::Inside,
            );
        }
        let font = body_strong().resolve(ui.style());
        let icon_w = if icon.is_some() { 22.0 } else { 0.0 };
        let galley = elided_singleline(painter, label, font, fg, draw_rect.width() - icon_w - 20.0);
        let start = draw_rect.center().x - (icon_w + galley.size().x) / 2.0;
        if let Some(glyph) = icon {
            painter.text(
                egui::pos2(start, draw_rect.center().y),
                Align2::LEFT_CENTER,
                glyph,
                FontId::proportional(14.0),
                fg,
            );
        }
        let gpos = egui::pos2(start + icon_w, draw_rect.center().y - galley.size().y / 2.0);
        painter.galley(gpos, galley, fg);
        let _ = resp;
        return false;
    }
    let hover = motion_t(ui.ctx(), resp.id.with("hover"), resp.hovered(), 0.12);
    let press = motion_t(
        ui.ctx(),
        resp.id.with("press"),
        resp.is_pointer_button_down_on(),
        0.08,
    );
    let bg = fill.lerp_to_gamma(fill.gamma_multiply(1.14), hover);
    let font = body_strong().resolve(ui.style());
    let draw_rect = rect.shrink(press);
    let painter = ui.painter();
    painter.rect_filled(draw_rect, CornerRadius::same(R1), bg);
    if let Some(b) = border {
        painter.rect_stroke(
            draw_rect,
            CornerRadius::same(R1),
            Stroke::new(1.0, b),
            egui::StrokeKind::Inside,
        );
    }
    if resp.has_focus() {
        painter.rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(R1),
            focus_stroke(),
            egui::StrokeKind::Outside,
        );
    }
    let icon_w = if icon.is_some() { 22.0 } else { 0.0 };
    let galley = elided_singleline(painter, label, font, fg, draw_rect.width() - icon_w - 20.0);
    let start = draw_rect.center().x - (icon_w + galley.size().x) / 2.0;
    if let Some(glyph) = icon {
        painter.text(
            egui::pos2(start, draw_rect.center().y),
            Align2::LEFT_CENTER,
            glyph,
            FontId::proportional(14.0),
            fg,
        );
    }
    let gpos = egui::pos2(start + icon_w, draw_rect.center().y - galley.size().y / 2.0);
    painter.galley(gpos, galley, fg);
    resp.clicked()
}

/// Single-line galley that elides with `…` instead of spilling outside its
/// button rect (QA X3: `layout_no_wrap` let long labels overflow).
fn elided_singleline(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = max_width.max(0.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    painter.layout_job(job)
}

fn internal_flow_reference_card(ui: &mut egui::Ui) {
    let contract = engineering::InternalFlowContract::default();
    let exact = contract.exact_contract();
    card(ui, |ui| {
        ui.label(
            RichText::new("Internal / HVAC · reference contract")
                .text_style(title())
                .color(TEXT),
        );
        ui.label(
            RichText::new(&contract.intent)
                .text_style(caption())
                .color(TEXT_MUTE),
        );
        ui.add_space(8.0);
        diag(
            ui,
            "Contract",
            exact["contract_kind"]
                .as_str()
                .unwrap_or(engineering::INTERNAL_FLOW_CONTRACT),
            TEXT_DIM,
        );
        ui.label(caps("Required future assignments"));
        ui.label(
            RichText::new(contract.required_assignments.join(" · "))
                .text_style(caption())
                .color(TEXT_DIM),
        );
        ui.add_space(8.0);
        for blocker in contract.execution_blockers() {
            ui.label(
                RichText::new(format!("EXECUTION BLOCKED · {blocker}"))
                    .text_style(mono_s())
                    .color(WARN),
            );
        }
        ui.add_space(8.0);
        // CS8: a plain status line, not a button-shaped non-action.
        ui.label(
            RichText::new("No compatible solver/model is available for this contract.")
                .text_style(caption())
                .color(TEXT_MUTE),
        );
    });
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) {
    let response = Frame::NONE
        .fill(SURFACE)
        .corner_radius(CornerRadius::same(R2))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    // §3.4 level 1: raised by tone, not outline — no outer border; a 1px
    // inner top-edge light (white @ 4%) reads as machined elevation.
    let rect = response.response.rect;
    let y = rect.top() + 0.5;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + R2 as f32, y),
            egui::pos2(rect.right() - R2 as f32, y),
        ],
        Stroke::new(1.0, Color32::from_white_alpha(10)),
    );
}

/// The clipboard results summary: a tab-separated table (label, value, unit,
/// source class) carrying named coefficients, physical loads in the display
/// system, reference values, and the provenance identifiers. Requires a
/// completed result; the caller only offers the button when one exists.
fn results_summary_tsv(
    case: &engineering::ExternalFlowCase,
    run_id: Option<&str>,
    system: units::UnitSystem,
    format: units::ValueFormat,
) -> String {
    use units::Quantity;
    let mut out = String::from("label\tvalue\tunit\tsource\n");
    let mut row = |label: &str, value: String, unit: &str, source: &str| {
        out.push_str(&format!("{label}\t{value}\t{unit}\t{source}\n"));
    };
    let fmt = |value: f64| units::format_value(value, format);
    row("case", case.name.clone(), "–", "PROVENANCE");
    row(
        "run",
        run_id.unwrap_or("draft (no immutable run)").into(),
        "–",
        "PROVENANCE",
    );
    row("model", case.model_id.clone(), "–", "PROVENANCE");
    row(
        "source_sha256",
        case.preflight.source_sha256.clone(),
        "–",
        "PROVENANCE",
    );
    let Some(result) = case.result.as_ref() else {
        return out;
    };
    row("Cd (+X drag)", fmt(result.force_coefficients[0]), "1", "MODEL");
    row("Cs (+Y side)", fmt(result.force_coefficients[1]), "1", "MODEL");
    row(
        "Cl (+Z vertical)",
        fmt(result.force_coefficients[2]),
        "1",
        "MODEL",
    );
    let axes = ["x", "y", "z"];
    for (axis, label) in axes.iter().enumerate() {
        let (value, unit) = units::display_value(Quantity::Force, result.force_newtons[axis], system);
        row(&format!("F{label}"), fmt(value), unit, "MODEL");
    }
    for (axis, label) in axes.iter().enumerate() {
        row(
            &format!("Cm{label}"),
            fmt(result.moment_coefficients[axis]),
            "1",
            "MODEL",
        );
    }
    for (axis, label) in axes.iter().enumerate() {
        let (value, unit) =
            units::display_value(Quantity::Moment, result.moment_newton_meters[axis], system);
        row(&format!("M{label}"), fmt(value), unit, "MODEL");
    }
    row("Cp_min", fmt(result.cp_min), "1", "RECOVERED");
    row("Cp_max", fmt(result.cp_max), "1", "RECOVERED");
    let (area, area_unit) = units::display_value(Quantity::Area, result.surface_area_m2, system);
    row("diffuse_surface_area", fmt(area), area_unit, "MODEL");
    let operating = &case.operating;
    let (speed, speed_unit) = units::display_value(Quantity::Velocity, operating.velocity, system);
    row("V_inf", fmt(speed), speed_unit, "REFERENCE");
    let (density, density_unit) =
        units::display_value(Quantity::Density, operating.density, system);
    row("rho_inf", fmt(density), density_unit, "REFERENCE");
    if let Some(q) = operating.dynamic_pressure() {
        let (value, unit) = units::display_value(Quantity::Pressure, q, system);
        row("q_inf", fmt(value), unit, "REFERENCE");
    }
    row(
        "L_ref",
        fmt(operating.reference_length),
        operating.length_unit.symbol(),
        "REFERENCE",
    );
    if let Some(reynolds) = operating.reynolds() {
        row("Re", fmt(reynolds), "1", "REFERENCE");
    }
    out
}

/// Encode an egui `ColorImage` as RGBA PNG bytes, nearest-neighbor upscaling
/// small images until the long edge reaches `min_edge` (0 = never upscale) so
/// exported grid sections stay inspectable at native data resolution.
fn color_image_png_bytes(image: &egui::ColorImage, min_edge: usize) -> Result<Vec<u8>, String> {
    let [width, height] = image.size;
    if width == 0 || height == 0 {
        return Err("the image is empty".into());
    }
    let factor = if min_edge == 0 {
        1
    } else {
        min_edge.div_ceil(width.max(height)).max(1)
    };
    let (out_width, out_height) = (width * factor, height * factor);
    let mut rgba = Vec::with_capacity(out_width * out_height * 4);
    for row in 0..out_height {
        for column in 0..out_width {
            let pixel = image.pixels[(row / factor) * width + (column / factor)];
            rgba.extend_from_slice(&[pixel.r(), pixel.g(), pixel.b(), pixel.a()]);
        }
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, out_width as u32, out_height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| error.to_string())?;
        writer
            .write_image_data(&rgba)
            .map_err(|error| error.to_string())?;
    }
    Ok(bytes)
}

/// Unit-aware SI-backed numeric input: a DragValue in the chosen display unit
/// plus a unit selector. Storage stays SI; switching the unit only changes
/// presentation. Returns true when the SI value changed.
fn unit_value_input<U: units::InputUnit>(
    ui: &mut egui::Ui,
    id: &str,
    si_value: &mut f64,
    unit: &mut U,
    si_speed: f64,
    si_range: std::ops::RangeInclusive<f64>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let active = *unit;
        let mut display = active.unit_from_si(*si_value);
        let response = ui.add(
            egui::DragValue::new(&mut display)
                .speed(si_speed * active.unit_from_si(1.0))
                .range(
                    active.unit_from_si(*si_range.start())..=active.unit_from_si(*si_range.end()),
                ),
        );
        if response.changed() {
            *si_value = active
                .unit_to_si(display)
                .clamp(*si_range.start(), *si_range.end());
            changed = true;
        }
        egui::ComboBox::from_id_salt(id)
            .selected_text(active.unit_symbol())
            .width(84.0)
            .show_ui(ui, |ui| {
                for &candidate in U::all() {
                    ui.selectable_value(unit, candidate, candidate.unit_symbol());
                }
            });
        // Honest storage: whenever the entry unit is not SI, the exact stored
        // SI value stays visible beside the field.
        if (active.unit_to_si(1.0) - 1.0).abs() > 1e-12 {
            ui.label(
                RichText::new(format!("= {si_value:.6} SI"))
                    .text_style(mono_s())
                    .color(TEXT_MUTE),
            );
        }
    });
    changed
}

fn diag(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_DIM));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Overflow guard: the value elides into the remaining width
            // instead of overprinting the label; full value on hover.
            ui.add(egui::Label::new(mono(value, color)).truncate())
                .on_hover_text(RichText::new(value).monospace());
        });
    });
    ui.add_space(7.0);
}

/// §4.4 measurement-table row: label in body text, value in mono
/// right-aligned, a shared unit column, and a source-class chip
/// (`MODEL` / `RECOVERED`, N5X-EV-02) on every row. Returns the row
/// response so callers can attach method notes on hover.
fn measure_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    unit: &str,
    source: &str,
    source_color: Color32,
) -> egui::Response {
    let response = ui
        .horizontal(|ui| {
            ui.label(RichText::new(label).color(TEXT_DIM));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(source)
                        .text_style(mono_chip())
                        .color(source_color),
                );
                ui.add_space(4.0);
                // Fixed-width unit column keeps the value edges ragged-left,
                // aligned-right — tabular reading order.
                let unit_galley = ui.painter().layout_no_wrap(
                    unit.to_owned(),
                    mono_s().resolve(ui.style()),
                    TEXT_MUTE,
                );
                let (unit_rect, _) =
                    ui.allocate_exact_size(Vec2::new(28.0, unit_galley.size().y), Sense::hover());
                ui.painter().galley(
                    egui::pos2(
                        unit_rect.min.x,
                        unit_rect.center().y - unit_galley.size().y / 2.0,
                    ),
                    unit_galley,
                    TEXT_MUTE,
                );
                ui.label(mono(value, TEXT));
            });
        })
        .response;
    ui.add_space(7.0);
    response
}

/// §4.2 landing drop zone: a level-1 card with a dashed hairline that
/// brightens while files hover. Actual drop handling lives in
/// `project_view` (dropped_files), so this stays purely presentational.
fn drop_target(ui: &mut egui::Ui, width: f32) {
    let hovering_files = ui.input(|input| !input.raw.hovered_files.is_empty());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 96.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(R2),
        if hovering_files {
            SURFACE_HIGH
        } else {
            SURFACE_LOW
        },
    );
    let border = if hovering_files { EMBER } else { OUTLINE };
    for [start, end] in [
        [rect.left_top(), rect.right_top()],
        [rect.right_top(), rect.right_bottom()],
        [rect.right_bottom(), rect.left_bottom()],
        [rect.left_bottom(), rect.left_top()],
    ] {
        painter.extend(egui::Shape::dashed_line(
            &[start, end],
            Stroke::new(1.0, border),
            5.0,
            4.0,
        ));
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "Drop an STL or .reynproj file here",
        caption().resolve(ui.style()),
        TEXT_MUTE,
    );
}

/// §4.5 ledger row: level-0 (no box) — label left, value right in `mono-s`,
/// hairline underneath. Long values truncate to a 12-char prefix with the
/// full value on hover and a copy affordance; absent values state their
/// honest word (`unknown` / `not completed`) in WARN, never blank.
fn ledger_row(
    ui: &mut egui::Ui,
    label: &str,
    value: Option<&str>,
    accent: Color32,
    absent_word: &str,
) {
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        ui.label(RichText::new(label).color(TEXT_DIM));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| match value {
            Some(full) => {
                if ui
                    .add(
                        egui::Button::new(RichText::new(ph::COPY).size(12.0).color(TEXT_MUTE))
                            .frame(false),
                    )
                    .on_hover_text("Copy full value")
                    .clicked()
                {
                    ui.ctx().copy_text(full.to_owned());
                }
                ui.label(
                    RichText::new(short_hash(full))
                        .text_style(mono_s())
                        .color(accent),
                )
                .on_hover_text(full);
            }
            None => {
                ui.label(RichText::new(absent_word).text_style(mono_s()).color(WARN));
            }
        });
    });
    let y = ui.cursor().min.y;
    ui.painter()
        .hline(ui.max_rect().x_range(), y, Stroke::new(1.0, HAIRLINE));
    ui.add_space(4.0);
}

/// §4.5 run-ledger row: run ID + created (UTC) + lifecycle state as
/// glyph+word chip. The whole row is clickable and deep-links to the
/// immutable run's evidence. Returns true when clicked.
fn run_ledger_row(
    ui: &mut egui::Ui,
    run_id: &str,
    created_utc_unix: u64,
    state: project::LifecycleState,
    active: bool,
) -> bool {
    use project::LifecycleState as LS;
    let (state_word, state_color) = match state {
        LS::Complete => ("COMPLETE", OK),
        LS::Failed => ("FAILED", DANGER),
        LS::Stale => ("STALE", WARN),
        LS::Running => ("RUNNING", EMBER),
        LS::EvidenceLocked => ("EVIDENCE-LOCKED", TEXT_DIM),
        LS::Ready => ("READY", TEXT_DIM),
        LS::Draft => ("DRAFT", TEXT_MUTE),
    };
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::click());
    let hover = motion_t(ui.ctx(), resp.id.with("hover"), resp.hovered(), 0.14);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(R1), SURFACE.gamma_multiply(hover));
    if active {
        painter.rect_filled(
            Rect::from_min_size(
                rect.min + Vec2::new(0.0, 6.0),
                Vec2::new(2.0, rect.height() - 12.0),
            ),
            CornerRadius::same(1),
            EMBER,
        );
    }
    let mono_font = mono_s().resolve(ui.style());
    painter.text(
        egui::pos2(rect.min.x + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        short_id(run_id),
        mono_font.clone(),
        if active { TEXT } else { TEXT_DIM },
    );
    painter.text(
        egui::pos2(rect.min.x + 110.0, rect.center().y),
        Align2::LEFT_CENTER,
        format_utc(created_utc_unix),
        mono_font.clone(),
        TEXT_MUTE,
    );
    painter.text(
        egui::pos2(rect.max.x - 12.0, rect.center().y),
        Align2::RIGHT_CENTER,
        state_word,
        mono_font,
        state_color,
    );
    let resp = resp.on_hover_text(format!("{run_id} — open this run's evidence"));
    painter.hline(rect.x_range(), rect.max.y + 1.0, Stroke::new(1.0, HAIRLINE));
    resp.clicked()
}

/// Unix seconds → "YYYY-MM-DD HH:MM UTC" without a date dependency
/// (civil-from-days, Hinnant's algorithm).
pub(crate) fn format_utc(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    let (hh, mm) = (secs / 3600, (secs % 3600) / 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}-{month:02}-{day:02} {hh:02}:{mm:02} UTC")
}

/// §4.4 inspector accordion group: an overline header row with a caret;
/// the body keeps the quiet control styling. State persists per id.
fn inspector_group(
    ui: &mut egui::Ui,
    id_salt: &str,
    title: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let id = ui.make_persistent_id(("inspector", id_salt));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        id,
        default_open,
    );
    let open = state.is_open();
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
    if resp.clicked() {
        state.toggle(ui);
    }
    let hover = motion_t(ui.ctx(), resp.id.with("hover"), resp.hovered(), 0.14);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(R1), SURFACE.gamma_multiply(hover));
    let overline_galley = painter.layout_no_wrap(
        title.to_uppercase(),
        overline().resolve(ui.style()),
        if open { TEXT_DIM } else { TEXT_MUTE },
    );
    painter.galley(
        egui::pos2(
            rect.min.x + 4.0,
            rect.center().y - overline_galley.size().y / 2.0,
        ),
        overline_galley,
        TEXT_DIM,
    );
    painter.text(
        egui::pos2(rect.max.x - 8.0, rect.center().y),
        Align2::RIGHT_CENTER,
        if open { "▾" } else { "▸" },
        FontId::proportional(11.0),
        TEXT_MUTE,
    );
    state.show_body_unindented(ui, |ui| {
        ui.add_space(4.0);
        body(ui);
        ui.add_space(8.0);
    });
    ui.add_space(4.0);
}

/// One glyph+word status line (§ honesty: never color-only).
fn alert_line(ui: &mut egui::Ui, color: Color32, glyph: &str, message: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(RichText::new(glyph).text_style(mono_s()).color(color));
        ui.label(RichText::new(message).text_style(caption()).color(TEXT_DIM));
    });
    ui.add_space(4.0);
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.2} GiB", value / GIB)
    } else if value >= MIB {
        format!("{:.1} MiB", value / MIB)
    } else {
        format!("{:.0} KiB", value / KIB)
    }
}

/// One stage in the Case Setup spine (§4.3): a 40px header row with a status
/// glyph, stage name, and one-line verdict summary; clicking anywhere on the
/// row toggles the body. The body hangs off a vertical connector line that
/// continues the glyph column, so the workflow reads top-to-bottom as one
/// spine rather than a stack of cards.
#[allow(clippy::too_many_arguments)]
fn spine_stage(
    ui: &mut egui::Ui,
    id_salt: &str,
    glyph: &str,
    glyph_color: Color32,
    title: &str,
    summary: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let id = ui.make_persistent_id(("case-spine", id_salt));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        id,
        default_open,
    );
    let open = state.is_open();

    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::click());
    if resp.clicked() {
        state.toggle(ui);
    }
    let hover = motion_t(ui.ctx(), resp.id.with("hover"), resp.hovered(), 0.14);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(R1), SURFACE.gamma_multiply(hover));
    if resp.has_focus() {
        painter.rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(R1),
            focus_stroke(),
            egui::StrokeKind::Outside,
        );
    }
    let row_y = rect.center().y;
    painter.text(
        egui::pos2(rect.min.x + 9.0, row_y),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(13.0),
        glyph_color,
    );
    painter.text(
        egui::pos2(rect.min.x + 28.0, row_y),
        Align2::LEFT_CENTER,
        title,
        body_strong().resolve(ui.style()),
        if open || resp.hovered() {
            TEXT
        } else {
            TEXT_DIM
        },
    );
    // Caret + summary, right-aligned. The summary is the verdict at a glance;
    // the caret signals the row is expandable.
    let caret = if open { "▾" } else { "▸" };
    painter.text(
        egui::pos2(rect.max.x - 8.0, row_y),
        Align2::RIGHT_CENTER,
        caret,
        FontId::proportional(11.0),
        TEXT_MUTE,
    );
    let summary_font = mono_s().resolve(ui.style());
    let summary_galley = painter.layout_no_wrap(summary.to_owned(), summary_font, TEXT_MUTE);
    let title_end = rect.min.x + 28.0 + 140.0;
    let summary_x = (rect.max.x - 24.0 - summary_galley.size().x).max(title_end);
    painter.galley(
        egui::pos2(summary_x, row_y - summary_galley.size().y / 2.0),
        summary_galley,
        TEXT_MUTE,
    );

    state.show_body_unindented(ui, |ui| {
        let left = ui.max_rect().min.x;
        let top = ui.cursor().min.y;
        let inner = Frame::NONE
            .inner_margin(Margin {
                left: 28,
                right: 0,
                top: 4,
                bottom: 12,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                body(ui);
            });
        // Connector line continues the glyph column through the open body.
        ui.painter().vline(
            left + 9.0,
            egui::Rangef::new(top, inner.response.rect.bottom() - 8.0),
            Stroke::new(1.0, HAIRLINE),
        );
    });
    ui.add_space(4.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_RUN_ID: &str = "61f596e7-8414-488e-b764-0a1dfe671d1a";
    const FIXTURE_MODEL_SHA256: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";

    fn benchmark_fixture() -> engine::BenchResult {
        engine::BenchResult {
            model: "fixture.pth".into(),
            seeds: vec![70000],
            horizons: vec![1, 4],
            rel: vec![vec![0.1, 0.2]],
            persist: vec![vec![0.4, 0.6]],
            global_rel: 0.15,
            runtime_s: 1.25,
            grid: 32,
            epoch: 20,
            dt_frame: 0.04,
            provenance: engine::BenchProvenance {
                verdict: "unknown".into(),
                training_seed: Some(0),
                mixed_fork_seed: Some(10000),
                mixed_fork_used: false,
                validation_seed: Some(50000),
                dataset: "standard".into(),
                benchmark_seeds: vec![engine::BenchSeedProvenance {
                    seed: 70000,
                    stream: "fresh_test".into(),
                    overlap: false,
                }],
                overlap_count: 0,
                overlap_pct: 0.0,
                epoch: Some(20),
                declared_epochs: Some(20),
                checkpoint_role: "legacy/unknown".into(),
                final_epoch_status: "final_epoch_role_unknown".into(),
                selection_metric: "val_multi_horizon_rel_l2".into(),
                selection_stream: "validation".into(),
                source_fingerprint_present: false,
                source_fingerprint_digest: None,
                legacy_unknown: vec![
                    "checkpoint role absent".into(),
                    "source fingerprint absent".into(),
                ],
                flags: Vec::new(),
            },
        }
    }

    fn inspector_fixture() -> engine::BenchInspector {
        let variables: Vec<String> = InspectorVariable::ALL
            .iter()
            .map(|variable| variable.key().into())
            .collect();
        let signed: Vec<bool> = InspectorVariable::ALL
            .iter()
            .map(|variable| variable.signed())
            .collect();
        let values: Vec<f32> = (0..48).map(|value| value as f32).collect();
        engine::BenchInspector {
            seed: 70000,
            horizon: 1,
            n: 2,
            maps: crate::benchmark_evidence::InspectorMaps::from_protocol(
                INSPECTOR_SCHEMA,
                2,
                &variables,
                &signed,
                &values,
            )
            .unwrap(),
            seed_stream: "fresh_test".into(),
            provenance_status: "unknown".into(),
            rel_l2: 0.1,
            persist_rel_l2: 0.4,
            improvement_ratio: 4.0,
            mean_abs_error: 1.0,
            p95_abs_error: 2.0,
            max_abs_error: 3.0,
            divergence_model_rms: 1e-3,
            divergence_truth_rms: 1e-4,
            divergence_error_rms: 1.1e-3,
            spectrum_rel_l2: 0.2,
            spectrum_k: vec![1.0],
            spectrum_model: vec![2.0],
            spectrum_truth: vec![1.8],
        }
    }

    #[test]
    fn evidence_exports_include_provenance_and_valid_canonical_hash() {
        let benchmark = benchmark_fixture();
        let csv = benchmark_csv(&benchmark);
        assert!(
            csv.starts_with("seed,provenance_stream,provenance_status,overlaps_reserved_stream")
        );
        assert!(csv.contains("70000,fresh_test,unknown,false"));

        let (report, expected_hash) =
            benchmark_report_card(&benchmark, None, FIXTURE_RUN_ID, FIXTURE_MODEL_SHA256, 42);
        let mut value: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(
            value["report_schema"],
            crate::benchmark_export::REPORT_CARD_SCHEMA
        );
        assert_eq!(value["run_id"], FIXTURE_RUN_ID);
        assert_eq!(value["model_checkpoint_sha256"], FIXTURE_MODEL_SHA256);
        assert_eq!(
            crate::benchmark_export::verify_canonical_report(&report).unwrap(),
            expected_hash
        );
        assert_eq!(value["provenance"]["validation_is_independent_test"], false);
        assert_eq!(
            value["provenance"]["benchmark_seeds"][0]["stream"],
            "fresh_test"
        );
        assert_eq!(value["integrity_algorithm"], "SHA-256");
        assert_eq!(value["authenticity"]["status"], "UNSIGNED");
        assert!(value["authenticity"]["signature"].is_null());
        let embedded = value
            .as_object_mut()
            .unwrap()
            .remove("integrity_sha256")
            .unwrap();
        assert_eq!(embedded.as_str(), Some(expected_hash.as_str()));
        use sha2::Digest;
        let actual = sha2::Sha256::digest(serde_json::to_vec(&value).unwrap());
        let actual_hex: String = actual.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(actual_hex, expected_hash);

        let mut clean = benchmark_fixture();
        clean.provenance.verdict = "clean".into();
        let (clean_report, _) =
            benchmark_report_card(&clean, None, FIXTURE_RUN_ID, FIXTURE_MODEL_SHA256, 42);
        let clean_value: serde_json::Value = serde_json::from_str(&clean_report).unwrap();
        assert_eq!(
            clean_value["provenance"]["checked_proposition"],
            "no collision in checked RNG streams"
        );
    }

    #[test]
    fn report_card_carries_spatial_variable_method_provenance() {
        let benchmark = benchmark_fixture();
        let mut inspector = inspector_fixture();
        inspector.provenance_status = "clean".into();
        let (report, _) = benchmark_report_card(
            &benchmark,
            Some(&inspector),
            FIXTURE_RUN_ID,
            FIXTURE_MODEL_SHA256,
            42,
        );
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        let spatial = &value["selected_cell_evidence"]["spatial_variable_evidence"];

        assert_eq!(
            value["selected_cell_evidence"]["provenance_checked_proposition"],
            "no collision in checked RNG streams"
        );
        assert_eq!(spatial["schema"], INSPECTOR_SCHEMA);
        assert_eq!(
            spatial["protocol_version"].as_u64(),
            Some(INSPECTOR_PROTOCOL_VERSION)
        );
        assert_eq!(spatial["layout"], INSPECTOR_LAYOUT);
        assert_eq!(spatial["domain"], "periodic_2pi");
        assert_eq!(spatial["maps_embedded"], false);
        assert_eq!(spatial["variables"].as_array().unwrap().len(), 4);
        assert_eq!(spatial["variables"][3]["key"], "divergence");
        assert_eq!(spatial["variables"][3]["signed"], true);
        assert_eq!(spatial["variables"][2]["label"], "Recovered pressure");
        assert_eq!(
            spatial["variables"][2]["unit"],
            "solver_velocity_unit_squared"
        );
        assert_eq!(
            spatial["variables"][2]["sources"]["solver_reference"],
            "RECOVERED_FROM_SOLVER_REFERENCE"
        );
        for format in [
            crate::benchmark_export::ExportFormat::Png,
            crate::benchmark_export::ExportFormat::Pdf,
        ] {
            let artifact = crate::benchmark_export::export_report(&report, format).unwrap();
            assert_eq!(
                artifact.canonical_payload_sha256,
                crate::benchmark_export::verify_canonical_report(&report).unwrap()
            );
            artifact.verify().unwrap();
        }
    }

    #[test]
    fn signed_benchmark_maps_keep_negative_and_positive_evidence_distinct() {
        let image = benchmark_map_image(&[-2.0, 0.0, 2.0, 1.0], 2, 2.0, true);
        assert_eq!(image.size, [2, 2]);
        assert_ne!(image.pixels[0], image.pixels[2]);
        assert_eq!(image.pixels[1], field2d::colormap_color(0.0, true));
    }

    #[test]
    fn engineering_navigation_hides_research_tools_by_default() {
        for nav in [Nav::Projects, Nav::Case, Nav::Results, Nav::Evidence] {
            assert!(nav_is_available(nav, false));
        }
        for nav in [
            Nav::FlowPainter,
            Nav::Fields2D,
            Nav::Metrics,
            Nav::Benchmark,
        ] {
            assert!(!nav_is_available(nav, false));
            assert!(nav_is_available(nav, true));
        }
        assert!(!settings::AppSettings::default().developer_research_sandbox);
    }

    /// Nav stage model (§4.1): Results is gated with an explicit reason (no
    /// silent redirect), and stage glyphs reflect the lifecycle honestly.
    #[test]
    fn workflow_stage_states_gate_results_with_a_reason() {
        // Fresh session: nothing complete, Results blocked with a reason.
        let fresh = workflow_stage_states(false, false, false, false);
        assert_eq!(fresh[0].glyph, StageGlyph::Empty);
        assert_eq!(fresh[1].glyph, StageGlyph::Empty);
        assert!(fresh[2].blocked.is_some());
        assert_eq!(fresh[3].glyph, StageGlyph::Empty);

        // Case open but no completed run: Case Setup is partial, Results
        // still blocked.
        let case_open = workflow_stage_states(true, true, false, false);
        assert_eq!(case_open[0].glyph, StageGlyph::Complete);
        assert_eq!(case_open[1].glyph, StageGlyph::Partial);
        assert!(case_open[2].blocked.is_some());

        // Completed run: everything reachable, nothing blocked.
        let complete = workflow_stage_states(true, true, true, true);
        assert_eq!(complete[1].glyph, StageGlyph::Complete);
        assert_eq!(complete[2].glyph, StageGlyph::Complete);
        assert!(complete[2].blocked.is_none());
        assert_eq!(complete[3].glyph, StageGlyph::Complete);
    }

    #[test]
    fn legacy_scientific_and_placeholder_labels_do_not_reenter_shell() {
        let source = include_str!("app.rs");
        let forbidden = [
            ["Surface Loads (", "C", "p", ")"].concat(),
            ["Truth", " Overlay"].concat(),
            ["Solver ", "truth"].concat(),
            ["Project ", "Alpha"].concat(),
            ["Live ", "Session"].concat(),
            ["\"Sup", "port\""].concat(),
        ];
        for label in forbidden {
            assert!(
                !source.contains(&label),
                "legacy shell label returned: {label}"
            );
        }
    }
}
