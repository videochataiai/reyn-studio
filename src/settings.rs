//! N6 persistent desktop settings. Runtime choices are explicit and local;
//! telemetry remains opt-in and is never enabled by a default migration.
//!
//! Every stored value keeps a serde default so settings files written by any
//! older build load cleanly. Display units, formatting, and viewport
//! preferences never change stored evidence — run manifests and versioned
//! exports remain SI regardless of these preferences.
use crate::engine::EngineConfig;
use crate::engineering_section::{SectionAxis, SectionQuantity};
use crate::field2d::FieldColormap;
use crate::signing::{PublicKeyRecord, SIGNATURE_ALGORITHM};
use crate::theme::*;
use crate::units::{self, InputUnitPrefs, NumberNotation, UnitSystem, ValueFormat};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, Stroke};
use egui_phosphor::regular as ph;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComputeDevice {
    Auto,
    Mps,
    Cpu,
}

impl ComputeDevice {
    pub const ALL: [Self; 3] = [Self::Auto, Self::Mps, Self::Cpu];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Mps => "Apple Metal (MPS)",
            Self::Cpu => "CPU",
        }
    }

    pub fn engine_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Mps => "mps",
            Self::Cpu => "cpu",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    Instrument,
    HighContrast,
}

impl ThemeMode {
    pub const ALL: [Self; 2] = [Self::Instrument, Self::HighContrast];

    pub fn label(self) -> &'static str {
        match self {
            Self::Instrument => "Instrument Dark",
            Self::HighContrast => "Instrument High Contrast",
        }
    }
}

/// Colormap range behavior for signed section views (Cp).
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CpRangeMode {
    /// Symmetric range from the section's own extrema.
    #[default]
    Auto,
    /// Symmetric range pinned to ±extent so sections compare across runs.
    Pinned,
}

impl CpRangeMode {
    pub const ALL: [Self; 2] = [Self::Auto, Self::Pinned];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto (per section)",
            Self::Pinned => "Pinned symmetric range",
        }
    }
}

/// Theme-sanctioned surfaces for the render viewport well.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViewportBackground {
    /// The darkest calibrated well (`BG_VIEWPORT`) — data glows here.
    #[default]
    InstrumentWell,
    /// The app canvas (`BG_0`) — slightly lifted, matches document screens.
    Canvas,
}

impl ViewportBackground {
    pub const ALL: [Self; 2] = [Self::InstrumentWell, Self::Canvas];

    pub fn label(self) -> &'static str {
        match self {
            Self::InstrumentWell => "Instrument well (darkest)",
            Self::Canvas => "App canvas",
        }
    }

    pub fn color(self) -> egui::Color32 {
        match self {
            Self::InstrumentWell => BG_VIEWPORT,
            Self::Canvas => BG_0,
        }
    }
}

/// A named, user-savable operating point (fluid state + speed). Applying a
/// preset only fills the case's draft fields — every gate still runs.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct OperatingPointPreset {
    pub name: String,
    pub velocity_mps: f64,
    pub density_kg_m3: f64,
    pub viscosity_pa_s: f64,
    pub reference_pressure_pa: f64,
}

/// Built-in reference fluids at standard conditions. These are textbook
/// values, clearly named; they fill inputs and never bypass validation.
pub fn built_in_presets() -> Vec<OperatingPointPreset> {
    vec![
        OperatingPointPreset {
            name: "Wind tunnel · air 15 °C · 30 m/s".into(),
            velocity_mps: 30.0,
            density_kg_m3: 1.225,
            viscosity_pa_s: 1.81e-5,
            reference_pressure_pa: 101_325.0,
        },
        OperatingPointPreset {
            name: "Low-speed air · 15 °C · 5 m/s".into(),
            velocity_mps: 5.0,
            density_kg_m3: 1.225,
            viscosity_pa_s: 1.81e-5,
            reference_pressure_pa: 101_325.0,
        },
        OperatingPointPreset {
            name: "Water · 20 °C · 1 m/s".into(),
            velocity_mps: 1.0,
            density_kg_m3: 998.2,
            viscosity_pa_s: 1.002e-3,
            reference_pressure_pa: 101_325.0,
        },
    ]
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AppSettings {
    pub compute_device: ComputeDevice,
    pub python_path: String,
    pub research_dir: String,
    pub project_directory: String,
    pub autosave_interval_seconds: u32,
    pub theme: ThemeMode,
    /// Replace interface transitions with instant state changes (§3.7).
    pub reduced_motion: bool,
    pub telemetry: bool,
    /// Permanently exposes experimental prediction tools. The default product
    /// path remains the case-centered engineering workflow.
    pub developer_research_sandbox: bool,
    /// Persistent organization-key identifier only. Until the signing backend
    /// verifies a key, this reference never implies authenticity or "signed".
    pub signing_key_reference: String,
    /// Portable public verification material. Private bytes remain in the
    /// native key provider and are never serialized into settings.
    pub signing_public_key_base64: String,
    pub signing_key_fingerprint_sha256: String,
    pub revoked_signing_key_fingerprints: Vec<String>,

    // -- Units & formatting (display/input side only; storage stays SI) -----
    pub unit_system: UnitSystem,
    pub significant_digits: u8,
    pub number_notation: NumberNotation,
    pub input_units: InputUnitPrefs,

    // -- Appearance ----------------------------------------------------------
    /// Interface zoom factor (egui zoom; ⌘+/⌘− adjust it live, this persists).
    pub ui_scale: f32,
    pub colormap: FieldColormap,
    pub cp_range_mode: CpRangeMode,
    pub cp_pinned_extent: f64,
    pub default_section_axis: SectionAxis,
    pub default_section_quantity: SectionQuantity,

    // -- Viewport -------------------------------------------------------------
    pub orbit_sensitivity: f32,
    pub invert_scroll_zoom: bool,
    pub show_domain_bounds: bool,
    pub show_viewport_hints: bool,
    pub viewport_background: ViewportBackground,

    // -- Workflow defaults ----------------------------------------------------
    pub default_horizon_steps: u32,
    /// Empty means "use the system default / last location".
    pub default_export_directory: String,
    pub operating_presets: Vec<OperatingPointPreset>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let config = EngineConfig::default();
        Self {
            compute_device: ComputeDevice::Auto,
            python_path: config.python_path,
            research_dir: config.research_dir,
            project_directory: default_project_directory(),
            autosave_interval_seconds: 120,
            theme: ThemeMode::Instrument,
            reduced_motion: false,
            telemetry: false,
            developer_research_sandbox: false,
            signing_key_reference: String::new(),
            signing_public_key_base64: String::new(),
            signing_key_fingerprint_sha256: String::new(),
            revoked_signing_key_fingerprints: Vec::new(),
            unit_system: UnitSystem::Si,
            significant_digits: 5,
            number_notation: NumberNotation::Auto,
            input_units: InputUnitPrefs::default(),
            ui_scale: 1.0,
            colormap: FieldColormap::Ember,
            cp_range_mode: CpRangeMode::Auto,
            cp_pinned_extent: 1.5,
            default_section_axis: SectionAxis::X,
            default_section_quantity: SectionQuantity::PhysicalCp,
            orbit_sensitivity: 1.0,
            invert_scroll_zoom: false,
            show_domain_bounds: true,
            show_viewport_hints: true,
            viewport_background: ViewportBackground::InstrumentWell,
            default_horizon_steps: 4,
            default_export_directory: String::new(),
            operating_presets: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn load() -> (Self, Option<String>) {
        let Some(path) = config_path() else {
            return (
                Self::default(),
                Some("settings directory unavailable; using session defaults".into()),
            );
        };
        if !path.is_file() {
            return (Self::default(), None);
        }
        match std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| serde_json::from_str::<Self>(&text).map_err(|error| error.to_string()))
        {
            Ok(mut settings) => {
                settings.telemetry = false;
                settings.normalize();
                (settings, None)
            }
            Err(error) => (
                Self::default(),
                Some(format!(
                    "settings could not be read; defaults restored: {error}"
                )),
            ),
        }
    }

    /// Clamp numeric preferences into their supported ranges so a hand-edited
    /// or older settings file can never push the UI into a broken state.
    pub fn normalize(&mut self) {
        self.significant_digits = self
            .significant_digits
            .clamp(units::MIN_SIGNIFICANT_DIGITS, units::MAX_SIGNIFICANT_DIGITS);
        if !self.ui_scale.is_finite() {
            self.ui_scale = 1.0;
        }
        self.ui_scale = self.ui_scale.clamp(0.8, 1.4);
        if !self.orbit_sensitivity.is_finite() {
            self.orbit_sensitivity = 1.0;
        }
        self.orbit_sensitivity = self.orbit_sensitivity.clamp(0.4, 2.0);
        if !self.cp_pinned_extent.is_finite() || self.cp_pinned_extent <= 0.0 {
            self.cp_pinned_extent = 1.5;
        }
        self.cp_pinned_extent = self.cp_pinned_extent.clamp(0.05, 100.0);
        self.default_horizon_steps = self.default_horizon_steps.clamp(1, 256);
        self.autosave_interval_seconds = self.autosave_interval_seconds.clamp(30, 3600);
        self.operating_presets
            .retain(|preset| !preset.name.trim().is_empty());
    }

    /// The active display format for numeric values.
    pub fn value_format(&self) -> ValueFormat {
        ValueFormat {
            significant_digits: self.significant_digits,
            notation: self.number_notation,
        }
    }

    pub fn engine_config(&self) -> EngineConfig {
        EngineConfig {
            research_dir: self.research_dir.clone(),
            python_path: self.python_path.clone(),
            device: self.compute_device.engine_value().into(),
        }
    }

    pub fn validate_runtime(&self) -> Result<(), String> {
        if self.python_path.trim().is_empty() {
            return Err("Python executable cannot be empty".into());
        }
        let python = Path::new(self.python_path.trim());
        if (python.is_absolute() || self.python_path.contains('/')) && !python.is_file() {
            return Err(format!(
                "Python executable was not found: {}",
                self.python_path
            ));
        }
        let research = Path::new(self.research_dir.trim());
        if !research.is_dir() {
            return Err(format!(
                "Research checkout was not found: {}",
                self.research_dir
            ));
        }
        if !research.join("time_moe_operator.py").is_file() {
            return Err("Research checkout does not contain time_moe_operator.py".into());
        }
        if !self.default_export_directory.trim().is_empty()
            && !Path::new(self.default_export_directory.trim()).is_dir()
        {
            return Err(format!(
                "Default export directory was not found: {}",
                self.default_export_directory
            ));
        }
        Ok(())
    }

    pub fn save(&self) -> Result<PathBuf, String> {
        let path = config_path().ok_or_else(|| "settings directory unavailable".to_string())?;
        save_to(self, &path)?;
        Ok(path)
    }

    pub fn configured_signing_key(&self) -> Result<Option<PublicKeyRecord>, String> {
        let fields_empty = self.signing_key_reference.trim().is_empty()
            && self.signing_public_key_base64.trim().is_empty()
            && self.signing_key_fingerprint_sha256.trim().is_empty();
        if fields_empty {
            return Ok(None);
        }
        if self.signing_key_reference.trim().is_empty()
            || self.signing_public_key_base64.trim().is_empty()
            || self.signing_key_fingerprint_sha256.trim().is_empty()
        {
            return Err(
                "signing-key reference is incomplete; no signed claim can be produced".into(),
            );
        }
        let record = PublicKeyRecord {
            key_id: self.signing_key_reference.clone(),
            algorithm: SIGNATURE_ALGORITHM.into(),
            public_key_base64: self.signing_public_key_base64.clone(),
            key_fingerprint_sha256: self.signing_key_fingerprint_sha256.clone(),
        };
        record.validate().map_err(|error| error.to_string())?;
        if self
            .revoked_signing_key_fingerprints
            .iter()
            .any(|fingerprint| fingerprint == &record.key_fingerprint_sha256)
        {
            return Err("configured signing key is revoked".into());
        }
        Ok(Some(record))
    }

    pub fn signing_key_is_revoked(&self) -> bool {
        !self.signing_key_fingerprint_sha256.is_empty()
            && self
                .revoked_signing_key_fingerprints
                .iter()
                .any(|fingerprint| fingerprint == &self.signing_key_fingerprint_sha256)
    }

    pub fn set_signing_key(&mut self, key: &PublicKeyRecord) {
        self.signing_key_reference = key.key_id.clone();
        self.signing_public_key_base64 = key.public_key_base64.clone();
        self.signing_key_fingerprint_sha256 = key.key_fingerprint_sha256.clone();
    }

    pub fn revoke_signing_key(&mut self) {
        if !self.signing_key_fingerprint_sha256.is_empty()
            && !self
                .revoked_signing_key_fingerprints
                .contains(&self.signing_key_fingerprint_sha256)
        {
            self.revoked_signing_key_fingerprints
                .push(self.signing_key_fingerprint_sha256.clone());
            self.revoked_signing_key_fingerprints.sort();
            self.revoked_signing_key_fingerprints.dedup();
        }
    }
}

fn default_project_directory() -> String {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Documents/Reyn Studio Projects"))
        .unwrap_or_else(|| PathBuf::from("Reyn Studio Projects"))
        .display()
        .to_string()
}

pub fn config_path() -> Option<PathBuf> {
    if let Some(override_dir) = std::env::var_os("REYN_STUDIO_CONFIG_DIR") {
        return Some(PathBuf::from(override_dir).join("settings.json"));
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME").map(|home| {
            PathBuf::from(home).join("Library/Application Support/Reyn Studio/settings.json")
        });
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Reyn Studio/settings.json"));
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|path| path.join("reyn-studio/settings.json"))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SettingsAction {
    Save,
    RestoreDefaults,
    CreateSigningKey,
    RevokeSigningKey,
}

/// Settings screen categories — a left rail selects one; only its sections
/// render, so depth never becomes a control wall (§3, progressive disclosure).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsCategory {
    #[default]
    Compute,
    Units,
    Appearance,
    Viewport,
    Workflow,
    Shortcuts,
    Storage,
    Signing,
    Developer,
}

impl SettingsCategory {
    pub const ALL: [Self; 9] = [
        Self::Compute,
        Self::Units,
        Self::Appearance,
        Self::Viewport,
        Self::Workflow,
        Self::Shortcuts,
        Self::Storage,
        Self::Signing,
        Self::Developer,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Compute => "Compute & engine",
            Self::Units => "Units & formatting",
            Self::Appearance => "Appearance",
            Self::Viewport => "Viewport & camera",
            Self::Workflow => "Workflow defaults",
            Self::Shortcuts => "Keyboard shortcuts",
            Self::Storage => "Storage & recovery",
            Self::Signing => "Signing & privacy",
            Self::Developer => "Developer",
        }
    }
}

/// Per-session UI state for the Settings screen (never persisted).
#[derive(Default)]
pub struct SettingsUiState {
    pub category: SettingsCategory,
    pub confirm_restore_defaults: bool,
    pub revoke_signing_key_armed: bool,
}

pub fn show_settings(
    ui: &mut egui::Ui,
    saved: &AppSettings,
    draft: &mut AppSettings,
    state: &mut SettingsUiState,
) -> Option<SettingsAction> {
    // S6/G3: the same centered content column as every other document screen
    // (980 max width, symmetric ≥34px gutters — the founder's margins fix).
    content_column(ui, CONTENT_MAX_WIDTH, |ui| {
        let mut action = None;
        ui.add_space(24.0);
        ui.label(display_text("Desktop settings"));
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Units, appearance, viewport, workflow, runtime, storage, and signing-key state. Changes stay on this machine; stored evidence remains SI.",
            )
            .text_style(caption())
            .color(TEXT_MUTE),
        );
        ui.add_space(16.0);

        // Reserve room for the action row below the scroll region so the save
        // controls can never be pushed under the status bar.
        let footer_height = 56.0;
        let body_height = (ui.available_height() - footer_height).max(120.0);
        ui.horizontal_top(|ui| {
            // Category rail — quiet list rows; the active row gets the level-1
            // fill and a 2px quiet edge marker (ember stays on Save).
            ui.vertical(|ui| {
                ui.set_width(188.0);
                for category in SettingsCategory::ALL {
                    let active = state.category == category;
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 30.0),
                        egui::Sense::click(),
                    );
                    let painter = ui.painter();
                    if active {
                        painter.rect_filled(rect, CornerRadius::same(R1), SURFACE_HIGH);
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                rect.min + egui::vec2(0.0, 6.0),
                                egui::vec2(2.0, rect.height() - 12.0),
                            ),
                            CornerRadius::same(1),
                            OUTLINE,
                        );
                    } else if resp.hovered() {
                        painter.rect_filled(rect, CornerRadius::same(R1), SURFACE);
                    }
                    painter.text(
                        egui::pos2(rect.min.x + 12.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        category.label(),
                        body_strong().resolve(ui.style()),
                        if active { TEXT } else { TEXT_DIM },
                    );
                    if resp.clicked() {
                        state.category = category;
                    }
                    ui.add_space(2.0);
                }
            });
            ui.add_space(6.0);
            // Meeting-edge hairline between rail and content (§3.4 level 0).
            let x = ui.cursor().min.x;
            ui.painter().vline(
                x,
                egui::Rangef::new(ui.cursor().min.y, ui.cursor().min.y + body_height),
                Stroke::new(1.0, HAIRLINE),
            );
            ui.add_space(14.0);
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(body_height)
                    .show(ui, |ui| {
                        match state.category {
                            SettingsCategory::Compute => {
                                category_compute(ui, draft, &mut action)
                            }
                            SettingsCategory::Units => category_units(ui, draft),
                            SettingsCategory::Appearance => category_appearance(ui, draft),
                            SettingsCategory::Viewport => category_viewport(ui, draft),
                            SettingsCategory::Workflow => category_workflow(ui, draft),
                            SettingsCategory::Shortcuts => category_shortcuts(ui, draft),
                            SettingsCategory::Storage => category_storage(ui, draft),
                            SettingsCategory::Signing => {
                                category_signing(ui, draft, state, &mut action)
                            }
                            SettingsCategory::Developer => category_developer(ui, draft),
                        }
                        ui.add_space(16.0);
                    });
            });
        });

        // Action row — always visible beneath the scroll region, above the
        // status bar, separated by a full-span hairline (§3.4 level 0).
        ui.add_space(8.0);
        let cursor_y = ui.cursor().min.y - 4.0;
        ui.painter().hline(
            ui.max_rect().x_range(),
            cursor_y,
            Stroke::new(1.0, HAIRLINE),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let dirty = draft != saved;
            if ui
                .add_enabled(
                    dirty,
                    egui::Button::new(
                        RichText::new(if runtime_changed(saved, draft) {
                            "Save & restart engine"
                        } else {
                            "Save settings"
                        })
                        .color(ON_EMBER),
                    )
                    .fill(EMBER)
                    .min_size(egui::vec2(0.0, 28.0)),
                )
                .clicked()
            {
                action = Some(SettingsAction::Save);
            }
            if state.confirm_restore_defaults {
                ui.label(
                    RichText::new("Reset every preference? Signing keys and saved presets are kept.")
                        .text_style(caption())
                        .color(WARN),
                );
                if ui
                    .add(egui::Button::new("Confirm reset").min_size(egui::vec2(0.0, 28.0)))
                    .clicked()
                {
                    action = Some(SettingsAction::RestoreDefaults);
                    state.confirm_restore_defaults = false;
                }
                if ui
                    .add(egui::Button::new("Cancel").min_size(egui::vec2(0.0, 28.0)))
                    .clicked()
                {
                    state.confirm_restore_defaults = false;
                }
            } else if ui
                .add(egui::Button::new("Restore defaults…").min_size(egui::vec2(0.0, 28.0)))
                .clicked()
            {
                state.confirm_restore_defaults = true;
            }
            if dirty {
                ui.label(
                    RichText::new("Unsaved changes")
                        .text_style(caption())
                        .color(WARN),
                );
            }
        });
        action
    })
}

// ---------------------------------------------------------------------------
// Category bodies
// ---------------------------------------------------------------------------

fn category_compute(ui: &mut egui::Ui, draft: &mut AppSettings, _action: &mut Option<SettingsAction>) {
    section(ui, "Compute & engine", |ui| {
        setting_row_reset(
            ui,
            "Compute device",
            "Reloads the Python sidecar; Automatic prefers Metal on Apple Silicon.",
            &mut draft.compute_device,
            AppSettings::default().compute_device,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.device")
                    .selected_text(value.label())
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        for device in ComputeDevice::ALL {
                            ui.selectable_value(value, device, device.label());
                        }
                    });
            },
        );
        ui.separator();
        setting_row(
            ui,
            "Python executable",
            "Pinned interpreter used to launch the bundled inference engine.",
            |ui| path_control(ui, &mut draft.python_path, PathPick::File),
        );
        ui.separator();
        setting_row(
            ui,
            "Research checkout",
            "Checkpoint library and solver modules used by this development build.",
            |ui| path_control(ui, &mut draft.research_dir, PathPick::Folder),
        );
    });
}

fn category_units(ui: &mut egui::Ui, draft: &mut AppSettings) {
    section(ui, "Displayed results", |ui| {
        setting_row_reset(
            ui,
            "Unit system",
            "Applies to Results, Case Setup summaries, and reports. Run manifests, evidence, and the versioned FEA CSV stay SI.",
            &mut draft.unit_system,
            AppSettings::default().unit_system,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.units")
                    .selected_text(value.label())
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for system in UnitSystem::ALL {
                            ui.selectable_value(value, system, system.label());
                        }
                    });
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Significant digits",
            "Displayed precision for measured and derived values (3–8).",
            &mut draft.significant_digits,
            AppSettings::default().significant_digits,
            |ui, value| {
                ui.add(egui::DragValue::new(value).range(
                    units::MIN_SIGNIFICANT_DIGITS..=units::MAX_SIGNIFICANT_DIGITS,
                ));
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Notation",
            "Automatic keeps values fixed inside 10⁻³…10⁵ and scientific outside.",
            &mut draft.number_notation,
            AppSettings::default().number_notation,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.notation")
                    .selected_text(value.label())
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        for notation in NumberNotation::ALL {
                            ui.selectable_value(value, notation, notation.label());
                        }
                    });
            },
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "Preview · {} · {}",
                units::format_quantity(
                    units::Quantity::Velocity,
                    30.0,
                    draft.unit_system,
                    draft.value_format()
                ),
                units::format_quantity(
                    units::Quantity::Pressure,
                    101_325.0,
                    draft.unit_system,
                    draft.value_format()
                ),
            ))
            .text_style(mono_s())
            .color(TEXT_MUTE),
        );
    });

    ui.add_space(12.0);
    section(ui, "Case input units", |ui| {
        ui.label(
            RichText::new(
                "Default entry units for the operating point. Values convert to SI on entry; \
                 the unit can also be switched per field in Case Setup.",
            )
            .text_style(caption())
            .color(TEXT_MUTE),
        );
        ui.add_space(6.0);
        setting_row_reset(
            ui,
            "Free-stream speed",
            "m/s, km/h, mph, ft/s, or knots.",
            &mut draft.input_units.velocity,
            InputUnitPrefs::default().velocity,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.input.velocity")
                    .selected_text(value.symbol())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for unit in units::VelocityUnit::ALL {
                            ui.selectable_value(value, unit, unit.symbol());
                        }
                    });
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Pressure",
            "Pa, kPa, psi, or atm.",
            &mut draft.input_units.pressure,
            InputUnitPrefs::default().pressure,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.input.pressure")
                    .selected_text(value.symbol())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for unit in units::PressureUnit::ALL {
                            ui.selectable_value(value, unit, unit.symbol());
                        }
                    });
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Density",
            "kg/m³ or lbm/ft³.",
            &mut draft.input_units.density,
            InputUnitPrefs::default().density,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.input.density")
                    .selected_text(value.symbol())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for unit in units::DensityUnit::ALL {
                            ui.selectable_value(value, unit, unit.symbol());
                        }
                    });
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Dynamic viscosity",
            "Pa·s or mPa·s (centipoise).",
            &mut draft.input_units.viscosity,
            InputUnitPrefs::default().viscosity,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.input.viscosity")
                    .selected_text(value.symbol())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for unit in units::ViscosityUnit::ALL {
                            ui.selectable_value(value, unit, unit.symbol());
                        }
                    });
            },
        );
    });
}

fn category_appearance(ui: &mut egui::Ui, draft: &mut AppSettings) {
    section(ui, "Interface", |ui| {
        setting_row_reset(
            ui,
            "Theme",
            "Both modes preserve the calibrated ember data vocabulary.",
            &mut draft.theme,
            AppSettings::default().theme,
            |ui, value| {
                ui.horizontal(|ui| {
                    theme_preview(ui, *value);
                    egui::ComboBox::from_id_salt("settings.theme")
                        .selected_text(value.label())
                        .width(190.0)
                        .show_ui(ui, |ui| {
                            for theme in ThemeMode::ALL {
                                ui.selectable_value(value, theme, theme.label());
                            }
                        });
                });
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Interface scale",
            "Zoom for all interface text and controls. ⌘+ / ⌘− adjust it live; this value persists.",
            &mut draft.ui_scale,
            AppSettings::default().ui_scale,
            |ui, value| {
                ui.add(
                    egui::Slider::new(value, 0.8..=1.4)
                        .step_by(0.05)
                        .custom_formatter(|scale, _| format!("{:.0}%", scale * 100.0)),
                );
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Reduce motion",
            "Replaces interface transitions (hover fades, screen crossfades, shimmer) with instant state changes.",
            &mut draft.reduced_motion,
            AppSettings::default().reduced_motion,
            |ui, value| {
                ui.checkbox(value, "Enable");
            },
        );
    });

    ui.add_space(12.0);
    section(ui, "Field views", |ui| {
        setting_row_reset(
            ui,
            "Colormap",
            "Applies to interactive section and 2D field views. Deterministic evidence exports keep the calibrated instrument map.",
            &mut draft.colormap,
            AppSettings::default().colormap,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.colormap")
                    .selected_text(value.label())
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for map in FieldColormap::ALL {
                            ui.selectable_value(value, map, map.label());
                        }
                    });
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Cp color range",
            "Auto scales each section to its own extrema; pinned keeps a fixed ±range so sections compare across runs.",
            &mut draft.cp_range_mode,
            AppSettings::default().cp_range_mode,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.cp-range")
                    .selected_text(value.label())
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for mode in CpRangeMode::ALL {
                            ui.selectable_value(value, mode, mode.label());
                        }
                    });
            },
        );
        if draft.cp_range_mode == CpRangeMode::Pinned {
            ui.separator();
            setting_row_reset(
                ui,
                "Pinned Cp extent",
                "Sections render as ±extent around Cp = 0.",
                &mut draft.cp_pinned_extent,
                AppSettings::default().cp_pinned_extent,
                |ui, value| {
                    ui.add(
                        egui::DragValue::new(value)
                            .speed(0.05)
                            .range(0.05..=100.0)
                            .prefix("± "),
                    );
                },
            );
        }
        ui.separator();
        // Two-part control on one row; the tuple lens gives the pair a single
        // reset affordance and writes back both halves.
        let mut section_default = (draft.default_section_axis, draft.default_section_quantity);
        setting_row_reset(
            ui,
            "Default section view",
            "The section plane and quantity a new engineering result opens with.",
            &mut section_default,
            (
                AppSettings::default().default_section_axis,
                AppSettings::default().default_section_quantity,
            ),
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.section-quantity")
                    .selected_text(value.1.label())
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        for quantity in SectionQuantity::ALL {
                            ui.selectable_value(&mut value.1, quantity, quantity.label());
                        }
                    });
                egui::ComboBox::from_id_salt("settings.section-axis")
                    .selected_text(value.0.label())
                    .width(52.0)
                    .show_ui(ui, |ui| {
                        for axis in SectionAxis::ALL {
                            ui.selectable_value(&mut value.0, axis, axis.label());
                        }
                    });
            },
        );
        draft.default_section_axis = section_default.0;
        draft.default_section_quantity = section_default.1;
    });
}

fn category_viewport(ui: &mut egui::Ui, draft: &mut AppSettings) {
    section(ui, "Camera", |ui| {
        setting_row_reset(
            ui,
            "Orbit sensitivity",
            "Degrees of rotation per pixel of drag, as a multiplier of the calibrated default.",
            &mut draft.orbit_sensitivity,
            AppSettings::default().orbit_sensitivity,
            |ui, value| {
                ui.add(
                    egui::Slider::new(value, 0.4..=2.0)
                        .step_by(0.1)
                        .custom_formatter(|factor, _| format!("{factor:.1}×")),
                );
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Invert scroll zoom",
            "Scroll up moves the camera away instead of closer.",
            &mut draft.invert_scroll_zoom,
            AppSettings::default().invert_scroll_zoom,
            |ui, value| {
                ui.checkbox(value, "Invert");
            },
        );
    });
    ui.add_space(12.0);
    section(ui, "Overlays & background", |ui| {
        setting_row_reset(
            ui,
            "Domain bounds",
            "Draw the [-1, 1]³ solver-domain wireframe in the 3D viewport.",
            &mut draft.show_domain_bounds,
            AppSettings::default().show_domain_bounds,
            |ui, value| {
                ui.checkbox(value, "Show");
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Interaction hints",
            "The one-line orbit/zoom hint at the bottom of render viewports.",
            &mut draft.show_viewport_hints,
            AppSettings::default().show_viewport_hints,
            |ui, value| {
                ui.checkbox(value, "Show");
            },
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Viewport background",
            "Theme-sanctioned surfaces only; the calibrated data vocabulary is unchanged.",
            &mut draft.viewport_background,
            AppSettings::default().viewport_background,
            |ui, value| {
                egui::ComboBox::from_id_salt("settings.viewport-bg")
                    .selected_text(value.label())
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for background in ViewportBackground::ALL {
                            ui.selectable_value(value, background, background.label());
                        }
                    });
            },
        );
    });
}

fn category_workflow(ui: &mut egui::Ui, draft: &mut AppSettings) {
    section(ui, "New-case defaults", |ui| {
        setting_row_reset(
            ui,
            "Default prediction horizon",
            "Initial horizon for a newly imported case, clamped to the selected model's support.",
            &mut draft.default_horizon_steps,
            AppSettings::default().default_horizon_steps,
            |ui, value| {
                ui.add(egui::DragValue::new(value).range(1..=256).suffix(" steps"));
            },
        );
        ui.separator();
        setting_row(
            ui,
            "Default export directory",
            "Starting location for CSV, PNG, and report export dialogs. Empty uses the system default.",
            |ui| path_control(ui, &mut draft.default_export_directory, PathPick::Folder),
        );
    });
    ui.add_space(12.0);
    section(ui, "Operating-point presets", |ui| {
        ui.label(
            RichText::new(
                "Save the current operating point from Case Setup (\"Save as preset…\"). \
                 Presets fill draft inputs only — every readiness gate still runs.",
            )
            .text_style(caption())
            .color(TEXT_MUTE),
        );
        ui.add_space(8.0);
        for preset in built_in_presets() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&preset.name).text_style(body_strong()).color(TEXT_DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("built-in")
                            .text_style(caption())
                            .color(TEXT_MUTE),
                    );
                    ui.label(
                        RichText::new(preset_summary(&preset))
                            .text_style(mono_s())
                            .color(TEXT_MUTE),
                    );
                });
            });
        }
        if !draft.operating_presets.is_empty() {
            ui.add_space(4.0);
            ui.separator();
        }
        let mut remove_index = None;
        for (index, preset) in draft.operating_presets.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&preset.name).text_style(body_strong()).color(TEXT));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .small_button("Delete")
                        .on_hover_text("Remove this saved preset")
                        .clicked()
                    {
                        remove_index = Some(index);
                    }
                    ui.label(
                        RichText::new(preset_summary(preset))
                            .text_style(mono_s())
                            .color(TEXT_MUTE),
                    );
                });
            });
        }
        if let Some(index) = remove_index {
            draft.operating_presets.remove(index);
        }
        if draft.operating_presets.is_empty() {
            ui.add_space(4.0);
            ui.label(
                RichText::new("No saved presets yet.")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
        }
    });
}

fn preset_summary(preset: &OperatingPointPreset) -> String {
    format!(
        "{} m/s · {} kg/m³ · {:.3e} Pa·s",
        preset.velocity_mps, preset.density_kg_m3, preset.viscosity_pa_s
    )
}

fn category_shortcuts(ui: &mut egui::Ui, draft: &mut AppSettings) {
    section(ui, "Keyboard reference", |ui| {
        ui.label(
            RichText::new("Bindings are fixed in this build; rebinding is planned.")
                .text_style(caption())
                .color(TEXT_MUTE),
        );
        ui.add_space(8.0);
        let mut rows: Vec<(&str, &str)> = vec![
            ("⌘K", "Command palette — navigate or act"),
            ("⌘N", "New project (guarded by unsaved changes)"),
            ("⌘O", "Open project…"),
            ("⌘S", "Save project"),
            ("⇧⌘S", "Save project as…"),
            ("⌘W / ⌘Q", "Close / quit through the unsaved-changes guard"),
            ("⌘+ / ⌘− / ⌘0", "Interface zoom in / out / reset (live)"),
            ("Drag", "Orbit the 3D viewport"),
            ("Scroll", "Zoom the 3D viewport"),
        ];
        if draft.developer_research_sandbox {
            rows.push(("G", "Regenerate procedural flow (research sandbox)"));
            rows.push(("← ↑ → ↓", "Move the selected benchmark cell (Benchmark Lab)"));
        }
        for (keys, description) in rows {
            ui.horizontal(|ui| {
                ui.allocate_ui(egui::vec2(120.0, 18.0), |ui| {
                    ui.label(RichText::new(keys).text_style(mono_s()).color(TEXT));
                });
                ui.label(
                    RichText::new(description)
                        .text_style(caption())
                        .color(TEXT_DIM),
                );
            });
            ui.add_space(2.0);
        }
    });
}

fn category_storage(ui: &mut egui::Ui, draft: &mut AppSettings) {
    section(ui, "Storage & recovery", |ui| {
        setting_row(
            ui,
            "Project directory",
            "Default location for local project bundles; portable content hashes remain authoritative.",
            |ui| path_control(ui, &mut draft.project_directory, PathPick::Folder),
        );
        ui.separator();
        setting_row_reset(
            ui,
            "Autosave interval",
            "Recovery snapshots are separate from explicit project saves.",
            &mut draft.autosave_interval_seconds,
            AppSettings::default().autosave_interval_seconds,
            |ui, value| {
                ui.add(egui::DragValue::new(value).range(30..=3600).suffix(" s"));
            },
        );
    });
}

fn category_signing(
    ui: &mut egui::Ui,
    draft: &mut AppSettings,
    state: &mut SettingsUiState,
    action: &mut Option<SettingsAction>,
) {
    section(ui, "Signing & integrity", |ui| {
        let key_state = if draft.signing_key_is_revoked() {
            ("REVOKED", DATA_RED)
        } else {
            match draft.configured_signing_key() {
                Ok(Some(_)) => ("READY · ED25519", SUCCESS),
                Ok(None) => ("NOT CONFIGURED", WARN),
                Err(_) => ("INVALID REFERENCE", DATA_RED),
            }
        };
        setting_row(
            ui,
            "Organization key",
            "Private bytes stay in the non-synchronizing macOS Keychain. Projects and reports receive only the public key and fingerprint.",
            |ui| {
                if draft.signing_key_reference.trim().is_empty() {
                    if ui.button("Create local key…").clicked() {
                        *action = Some(SettingsAction::CreateSigningKey);
                    }
                } else {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&draft.signing_key_reference)
                                .text_style(mono_s())
                                .color(TEXT_DIM),
                        );
                        ui.label(
                            RichText::new(short_fingerprint(
                                &draft.signing_key_fingerprint_sha256,
                            ))
                            .text_style(mono_s())
                            .color(TEXT_MUTE),
                        );
                    });
                }
            },
        );
        ui.separator();
        setting_row(
            ui,
            "Authenticity state",
            "A valid signature proves possession of this key. Recipients must compare its fingerprint through an independent channel.",
            |ui| {
                ui.label(
                    RichText::new(key_state.0)
                        .monospace()
                        .size(11.0)
                        .strong()
                        .color(key_state.1),
                );
            },
        );
        if !draft.signing_key_reference.trim().is_empty() && !draft.signing_key_is_revoked() {
            ui.separator();
            if state.revoke_signing_key_armed {
                ui.label(
                    RichText::new(
                        "Revocation is permanent for this local fingerprint. Existing signatures remain cryptographically valid but must verify as REVOKED when this list is supplied.",
                    )
                    .text_style(caption())
                    .color(DATA_RED),
                );
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.revoke_signing_key_armed = false;
                    }
                    if ui.button("Confirm revoke").clicked() {
                        *action = Some(SettingsAction::RevokeSigningKey);
                        state.revoke_signing_key_armed = false;
                    }
                });
            } else if ui.button("Revoke key…").clicked() {
                state.revoke_signing_key_armed = true;
            }
        }
    });
    ui.add_space(12.0);
    section(ui, "Privacy", |ui| {
        setting_row(
            ui,
            "Anonymous telemetry",
            "No analytics endpoint is bundled. Model paths and fields never leave the machine.",
            |ui| {
                draft.telemetry = false;
                ui.label(
                    RichText::new("OFF")
                        .monospace()
                        .size(11.0)
                        .strong()
                        .color(SUCCESS),
                );
            },
        );
    });
}

fn category_developer(ui: &mut egui::Ui, draft: &mut AppSettings) {
    section(ui, "Developer", |ui| {
        setting_row_reset(
            ui,
            "Research Sandbox",
            "Expose procedural 3D prediction, Flow Painter, standalone 2D fields, and Benchmark Lab. These tools are experimental and remain outside engineering case evidence.",
            &mut draft.developer_research_sandbox,
            AppSettings::default().developer_research_sandbox,
            |ui, value| {
                ui.checkbox(value, "Enable");
            },
        );
        if draft.developer_research_sandbox {
            ui.add_space(8.0);
            ui.label(
                RichText::new("Developer mode · sandbox outputs are not case results")
                    .text_style(caption())
                    .color(WARN),
            );
        }
    });
}

/// Honest engine-state color for the runtime rail: derived from the real
/// `engine_ok` flag, never assumed healthy (PRD: no fake state). The status
/// string carries the glyph + words; this pairs the matching semantic hue.
pub fn engine_state_color(engine_ok: bool) -> egui::Color32 {
    if engine_ok {
        SUCCESS
    } else {
        WARN
    }
}

pub fn show_controls(
    ui: &mut egui::Ui,
    saved: &AppSettings,
    engine_status: &str,
    engine_ok: bool,
    notice: Option<&(String, bool)>,
) {
    ui.label(title_text("Runtime"));
    ui.label(
        RichText::new("local engine configuration")
            .size(11.5)
            .color(TEXT_MUTE),
    );
    ui.add_space(16.0);

    if let Some((message, is_error)) = notice {
        // Calm alert recipe (§3.3): tint fill + same-hue hairline; full color
        // only on the leading glyph.
        let hue = if *is_error { DANGER } else { OK };
        let glyph = if *is_error { "!" } else { "✓" };
        Frame::NONE
            .fill(tint_fill(hue))
            .stroke(Stroke::new(1.0, tint_hairline(hue)))
            .corner_radius(CornerRadius::same(R1))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.label(RichText::new(glyph).size(11.0).strong().color(hue));
                    ui.label(RichText::new(message).size(11.0).color(TEXT_DIM));
                });
            });
        ui.add_space(12.0);
    }

    runtime_fact(ui, "ENGINE", engine_status, engine_state_color(engine_ok));
    runtime_fact(ui, "DEVICE POLICY", saved.compute_device.label(), BRAND);
    runtime_fact(ui, "THEME", saved.theme.label(), TEXT_DIM);
    runtime_fact(
        ui,
        "UNITS",
        match saved.unit_system {
            UnitSystem::Si => "SI",
            UnitSystem::Imperial => "Imperial (display only)",
        },
        TEXT_DIM,
    );
    runtime_fact(ui, "TELEMETRY", "Off", SUCCESS);
    runtime_fact(
        ui,
        "SIGNING KEY",
        if saved.signing_key_is_revoked() {
            "Revoked"
        } else if saved
            .configured_signing_key()
            .is_ok_and(|key| key.is_some())
        {
            "Ready · Ed25519"
        } else if saved.signing_key_reference.trim().is_empty() {
            "Not configured"
        } else {
            "Invalid reference"
        },
        if saved.signing_key_is_revoked() || saved.configured_signing_key().is_err() {
            DATA_RED
        } else if saved
            .configured_signing_key()
            .is_ok_and(|key| key.is_some())
        {
            SUCCESS
        } else {
            WARN
        },
    );
    ui.add_space(14.0);

    Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, HAIRLINE))
        .corner_radius(CornerRadius::same(R2))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.label(overline_text("Settings file"));
            let path_text = config_path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unavailable".into());
            // S8: elide instead of wrapping mid-path; full path on hover.
            ui.add(
                egui::Label::new(
                    RichText::new(&path_text)
                        .text_style(mono_s())
                        .color(TEXT_DIM),
                )
                .truncate(),
            )
            .on_hover_text(RichText::new(path_text).monospace());
        });
}

pub fn runtime_changed(saved: &AppSettings, draft: &AppSettings) -> bool {
    saved.compute_device != draft.compute_device
        || saved.python_path != draft.python_path
        || saved.research_dir != draft.research_dir
}

fn section(ui: &mut egui::Ui, section_title: &str, add: impl FnOnce(&mut egui::Ui)) {
    Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, HAIRLINE))
        .corner_radius(CornerRadius::same(R2))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(title_text(section_title));
            ui.add_space(10.0);
            add(ui);
        });
}

fn setting_row(ui: &mut egui::Ui, label: &str, detail: &str, control: impl FnOnce(&mut egui::Ui)) {
    // S3: the label column yields to the control at narrow widths instead of
    // overlapping it (min 200px reserved for the control), with row padding.
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let label_width = (ui.available_width() - 216.0).clamp(140.0, 360.0);
        ui.vertical(|ui| {
            ui.set_max_width(label_width);
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(RichText::new(label).text_style(body_strong()).color(TEXT));
            ui.label(RichText::new(detail).text_style(caption()).color(TEXT_MUTE));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), control);
    });
    ui.add_space(4.0);
}

/// A setting row with a per-setting "reset to default" affordance: the ↺
/// appears only while the draft value differs from the shipped default, in a
/// fixed slot so controls stay aligned.
fn setting_row_reset<T: PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    detail: &str,
    value: &mut T,
    default: T,
    control: impl FnOnce(&mut egui::Ui, &mut T),
) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let label_width = (ui.available_width() - 240.0).clamp(140.0, 360.0);
        ui.vertical(|ui| {
            ui.set_max_width(label_width);
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(RichText::new(label).text_style(body_strong()).color(TEXT));
            ui.label(RichText::new(detail).text_style(caption()).color(TEXT_MUTE));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let modified = *value != default;
            let reset = ui.add_visible(
                modified,
                egui::Button::new(
                    RichText::new(ph::ARROW_COUNTER_CLOCKWISE)
                        .size(13.0)
                        .color(TEXT_MUTE),
                )
                .frame(false),
            );
            if modified && reset.on_hover_text("Reset to default").clicked() {
                *value = default;
                return;
            }
            control(ui, value);
        });
    });
    ui.add_space(4.0);
}

#[derive(Clone, Copy)]
enum PathPick {
    File,
    Folder,
}

/// Right-aligned path field: Browse… on the right edge, then a flexible-width
/// monospace field that uses the remaining row width (no more hard 250px clip)
/// with the full path on hover.
fn path_control(ui: &mut egui::Ui, value: &mut String, pick: PathPick) {
    if ui.button("Browse…").clicked() {
        let dialog = rfd::FileDialog::new();
        let picked = match pick {
            PathPick::File => dialog.pick_file(),
            PathPick::Folder => dialog.pick_folder(),
        };
        if let Some(path) = picked {
            *value = path.display().to_string();
        }
    }
    let width = (ui.available_width() - 8.0).clamp(220.0, 560.0);
    let full_path = value.clone();
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(width)
            .font(egui::TextStyle::Monospace),
    )
    .on_hover_text(RichText::new(full_path).monospace());
}

/// Three-swatch preview of a theme mode: surface, text, and ember accent.
fn theme_preview(ui: &mut egui::Ui, mode: ThemeMode) {
    let text = match mode {
        ThemeMode::Instrument => TEXT,
        ThemeMode::HighContrast => egui::Color32::WHITE,
    };
    // S5: swatches sit on a darker well with visible outlines so the surface
    // swatch reads instead of vanishing into the card fill.
    let (rect, _) = ui.allocate_exact_size(egui::vec2(60.0, 22.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(4), BG_0);
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, OUTLINE),
        egui::StrokeKind::Inside,
    );
    for (index, color) in [BG_1, text, EMBER].into_iter().enumerate() {
        let swatch = egui::Rect::from_min_size(
            rect.min + egui::vec2(3.0 + index as f32 * 18.0, 3.0),
            egui::vec2(16.0, 16.0),
        );
        painter.rect_filled(swatch, CornerRadius::same(3), color);
        painter.rect_stroke(
            swatch,
            CornerRadius::same(3),
            Stroke::new(1.0, OUTLINE),
            egui::StrokeKind::Inside,
        );
    }
}

fn runtime_fact(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.label(overline_text(label));
    ui.label(RichText::new(value).text_style(mono_s()).color(color));
    ui.add_space(11.0);
}

fn short_fingerprint(fingerprint: &str) -> String {
    if fingerprint.len() >= 24 {
        format!(
            "{}…{}",
            &fingerprint[..12],
            &fingerprint[fingerprint.len() - 8..]
        )
    } else if fingerprint.is_empty() {
        "fingerprint unavailable".into()
    } else {
        fingerprint.into()
    }
}

fn save_to(settings: &AppSettings, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "settings path has no parent directory".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Honesty guard (PRD: no fake state): the runtime rail's engine color
    /// must track the real `engine_ok` flag — never green while down.
    #[test]
    fn engine_state_color_is_never_green_when_engine_is_down() {
        assert_eq!(engine_state_color(true), SUCCESS);
        assert_eq!(engine_state_color(false), WARN);
        assert_ne!(engine_state_color(false), SUCCESS);
    }

    #[test]
    fn settings_round_trip_keeps_telemetry_off() {
        let root = std::env::temp_dir().join(format!(
            "reyn-settings-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let path = root.join("settings.json");
        let settings = AppSettings {
            compute_device: ComputeDevice::Cpu,
            python_path: "/usr/bin/python3".into(),
            research_dir: "/tmp/reyn-research".into(),
            project_directory: "/tmp/reyn-projects".into(),
            autosave_interval_seconds: 300,
            theme: ThemeMode::HighContrast,
            reduced_motion: true,
            telemetry: false,
            developer_research_sandbox: true,
            signing_key_reference: "org-key-2026".into(),
            unit_system: UnitSystem::Imperial,
            significant_digits: 6,
            number_notation: NumberNotation::Scientific,
            input_units: InputUnitPrefs {
                velocity: units::VelocityUnit::MilesPerHour,
                pressure: units::PressureUnit::Psi,
                density: units::DensityUnit::PoundsMassPerCubicFoot,
                viscosity: units::ViscosityUnit::MillipascalSecond,
            },
            ui_scale: 1.2,
            colormap: FieldColormap::Viridis,
            cp_range_mode: CpRangeMode::Pinned,
            cp_pinned_extent: 2.0,
            default_section_axis: SectionAxis::Z,
            default_section_quantity: SectionQuantity::VelocityMagnitude,
            orbit_sensitivity: 1.4,
            invert_scroll_zoom: true,
            show_domain_bounds: false,
            show_viewport_hints: false,
            viewport_background: ViewportBackground::Canvas,
            default_horizon_steps: 8,
            default_export_directory: "/tmp".into(),
            operating_presets: vec![OperatingPointPreset {
                name: "Tunnel B · 22 m/s".into(),
                velocity_mps: 22.0,
                density_kg_m3: 1.2,
                viscosity_pa_s: 1.8e-5,
                reference_pressure_pa: 101_000.0,
            }],
            ..AppSettings::default()
        };
        save_to(&settings, &path).unwrap();
        let loaded: AppSettings = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(loaded, settings);
        assert!(!loaded.telemetry);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_partial_settings_receive_safe_defaults() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"python_path":"python3","research_dir":"/tmp"}"#).unwrap();
        assert_eq!(settings.compute_device, ComputeDevice::Auto);
        assert_eq!(settings.theme, ThemeMode::Instrument);
        assert!(!settings.reduced_motion);
        assert!(!settings.telemetry);
        assert!(!settings.developer_research_sandbox);
        assert_eq!(settings.autosave_interval_seconds, 120);
        assert!(settings.signing_key_reference.is_empty());
        assert!(settings.signing_public_key_base64.is_empty());
        assert!(settings.signing_key_fingerprint_sha256.is_empty());
        assert!(settings.revoked_signing_key_fingerprints.is_empty());
        // New preference groups all arrive at their shipped defaults.
        assert_eq!(settings.unit_system, UnitSystem::Si);
        assert_eq!(settings.significant_digits, 5);
        assert_eq!(settings.number_notation, NumberNotation::Auto);
        assert_eq!(settings.input_units, InputUnitPrefs::default());
        assert_eq!(settings.ui_scale, 1.0);
        assert_eq!(settings.colormap, FieldColormap::Ember);
        assert_eq!(settings.cp_range_mode, CpRangeMode::Auto);
        assert_eq!(settings.default_section_axis, SectionAxis::X);
        assert_eq!(
            settings.default_section_quantity,
            SectionQuantity::PhysicalCp
        );
        assert_eq!(settings.orbit_sensitivity, 1.0);
        assert!(!settings.invert_scroll_zoom);
        assert!(settings.show_domain_bounds);
        assert!(settings.show_viewport_hints);
        assert_eq!(
            settings.viewport_background,
            ViewportBackground::InstrumentWell
        );
        assert_eq!(settings.default_horizon_steps, 4);
        assert!(settings.default_export_directory.is_empty());
        assert!(settings.operating_presets.is_empty());
    }

    #[test]
    fn normalize_clamps_out_of_range_preferences() {
        let mut settings = AppSettings {
            significant_digits: 42,
            ui_scale: 9.0,
            orbit_sensitivity: f32::NAN,
            cp_pinned_extent: -3.0,
            default_horizon_steps: 0,
            autosave_interval_seconds: 1,
            operating_presets: vec![OperatingPointPreset::default()],
            ..AppSettings::default()
        };
        settings.normalize();
        assert_eq!(settings.significant_digits, units::MAX_SIGNIFICANT_DIGITS);
        assert_eq!(settings.ui_scale, 1.4);
        assert_eq!(settings.orbit_sensitivity, 1.0);
        assert_eq!(settings.cp_pinned_extent, 1.5);
        assert_eq!(settings.default_horizon_steps, 1);
        assert_eq!(settings.autosave_interval_seconds, 30);
        assert!(settings.operating_presets.is_empty(), "nameless presets dropped");
    }

    #[test]
    fn runtime_restart_is_only_requested_for_engine_changes() {
        let saved = AppSettings::default();
        let mut draft = saved.clone();
        draft.theme = ThemeMode::HighContrast;
        draft.unit_system = UnitSystem::Imperial;
        draft.ui_scale = 1.2;
        assert!(!runtime_changed(&saved, &draft));
        draft.compute_device = ComputeDevice::Cpu;
        assert!(runtime_changed(&saved, &draft));
    }

    #[test]
    fn storage_and_signing_state_persist_without_engine_restart() {
        let saved = AppSettings::default();
        let mut draft = saved.clone();
        draft.project_directory = "/tmp/portable-reyn-projects".into();
        draft.autosave_interval_seconds = 45;
        draft.signing_key_reference = "verification-key-a".into();
        draft.developer_research_sandbox = true;
        assert_ne!(draft, saved);
        assert!(!runtime_changed(&saved, &draft));

        let json = serde_json::to_string(&draft).unwrap();
        let loaded: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.project_directory, draft.project_directory);
        assert_eq!(loaded.autosave_interval_seconds, 45);
        assert_eq!(loaded.signing_key_reference, "verification-key-a");
        assert!(loaded.developer_research_sandbox);
        assert!(loaded.configured_signing_key().is_err());
    }

    #[test]
    fn public_key_settings_round_trip_and_revocation_blocks_signing_state() {
        use base64::Engine as _;
        use sha2::Digest as _;

        let provider = crate::signing::DeterministicTestProvider::new("settings-public");
        let key = provider.public_key_record();
        let mut settings = AppSettings::default();
        settings.set_signing_key(&key);
        assert_eq!(
            settings.configured_signing_key().unwrap(),
            Some(key.clone())
        );
        let json = serde_json::to_string_pretty(&settings).unwrap();
        let seed: [u8; 32] =
            sha2::Sha256::digest(b"reyn.test-only.ed25519.v1\0settings-public").into();
        assert!(!json.contains(&base64::engine::general_purpose::STANDARD.encode(seed)));
        assert!(!json.contains(
            &seed
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ));
        assert!(json.contains(&key.public_key_base64));

        let mut restored: AppSettings = serde_json::from_str(&json).unwrap();
        restored.revoke_signing_key();
        assert!(restored.signing_key_is_revoked());
        assert!(restored.configured_signing_key().is_err());
    }
}
