//! N6 model-library UI — "instrument inventory" (§4.6). Checkpoint facts come
//! from the Python engine; this module only presents them and emits explicit
//! user actions. The screen owns everything: header (title + active chip +
//! the single ember action), toolbar (search · dimension · health filter
//! chips), an inline import-flow feedback area, and a reflowing card grid.
use crate::engine::{ModelCard, ModelValidation};
use crate::theme::*;
use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Sense, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DimensionFilter {
    #[default]
    All,
    TwoD,
    ThreeD,
}

impl DimensionFilter {
    const ALL: [Self; 3] = [Self::All, Self::TwoD, Self::ThreeD];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::TwoD => "2D",
            Self::ThreeD => "3D",
        }
    }
}

/// Health filter — the old passive counters as actionable chips (§4.6, A19).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HealthFilter {
    #[default]
    All,
    NeedsReview,
    Rejected,
}

#[derive(Default)]
pub struct LibraryState {
    pub search: String,
    pub dimension: DimensionFilter,
    pub health: HealthFilter,
    pub pending_delete: Option<String>,
    pub busy: bool,
    pub notice: Option<(String, bool)>,
    pub validation: Option<ModelValidation>,
}

pub enum LibraryAction {
    Activate(String),
    Delete(String),
    Import,
    Refresh,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Health {
    Ready,
    NeedsReview,
    Rejected,
}

fn health(status: &str) -> Health {
    match status {
        "clean" => Health::Ready,
        "invalid" => Health::Rejected,
        _ => Health::NeedsReview,
    }
}

fn status_color(status: &str) -> Color32 {
    match health(status) {
        Health::Ready => OK,
        Health::Rejected => DANGER,
        Health::NeedsReview => WARN,
    }
}

/// Scientific-state token (mono-chip caps); never color-only — the dot glyph
/// and the word travel together.
fn status_label(status: &str) -> &'static str {
    match health(status) {
        Health::Ready => "CONTRACT OK",
        Health::Rejected => "REJECTED",
        Health::NeedsReview => "METADATA GAPS",
    }
}

fn visible(model: &ModelCard, state: &LibraryState) -> bool {
    let dimension_matches = match state.dimension {
        DimensionFilter::All => true,
        DimensionFilter::TwoD => model.dimension == 2,
        DimensionFilter::ThreeD => model.dimension == 3,
    };
    let health_matches = match state.health {
        HealthFilter::All => true,
        HealthFilter::NeedsReview => health(&model.status) == Health::NeedsReview,
        HealthFilter::Rejected => health(&model.status) == Health::Rejected,
    };
    let needle = state.search.trim().to_lowercase();
    dimension_matches
        && health_matches
        && (needle.is_empty()
            || model.name.to_lowercase().contains(&needle)
            || model.scenario.to_lowercase().contains(&needle))
}

fn health_counts(models: &[ModelCard]) -> (usize, usize, usize) {
    let review = models
        .iter()
        .filter(|model| health(&model.status) == Health::NeedsReview)
        .count();
    let rejected = models
        .iter()
        .filter(|model| health(&model.status) == Health::Rejected)
        .count();
    (models.len(), review, rejected)
}

/// "38 checkpoints — 9 need metadata review" (loudest fact first, §3.8).
fn summary_phrase(total: usize, review: usize) -> String {
    let mut phrase = format!("{total} checkpoint{}", if total == 1 { "" } else { "s" });
    if review > 0 {
        phrase.push_str(&format!(" — {review} need metadata review"));
    }
    phrase
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    if bytes as f64 >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB)
    } else {
        format!("{:.1} MiB", bytes as f64 / MIB)
    }
}

fn format_modified(unix: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(unix);
    let days = now.saturating_sub(unix) / 86_400;
    match days {
        0 => "today".into(),
        1 => "1 day ago".into(),
        value if value < 365 => format!("{value} days ago"),
        value => format!("{:.1} years ago", value as f32 / 365.25),
    }
}

fn compact_role(role: &str) -> &str {
    match role {
        "fixed_final" => "fixed final",
        "best_validation" => "validation selected",
        "legacy/unknown" => "UNKNOWN",
        other => other,
    }
}

fn known_u32(value: u32) -> String {
    if value == 0 {
        "UNKNOWN".into()
    } else {
        value.to_string()
    }
}

fn known_text(value: &str) -> &str {
    if value.is_empty() || value == "unknown" || value == "legacy/unknown" {
        "UNKNOWN"
    } else {
        value
    }
}

fn epoch_text(model: &ModelCard) -> String {
    match (model.epoch, model.declared_epochs) {
        (0, 0) => "UNKNOWN".into(),
        (epoch, 0) => format!("{epoch} / UNKNOWN"),
        (0, declared) => format!("UNKNOWN / {declared}"),
        (epoch, declared) => format!("{epoch} / {declared}"),
    }
}

fn support_summary(model: &ModelCard) -> String {
    if model.dimension == 0 {
        return "UNKNOWN contract".into();
    }
    let grid = if model.grid == 0 {
        "UNKNOWN grid".into()
    } else if model.dimension == 3 {
        format!("{}³ grid", model.grid)
    } else {
        format!("{}² grid", model.grid)
    };
    let horizon = if model.max_steps == 0 {
        // Matches `contract_line`'s wording — "hUNKNOWN" read as a token.
        "horizon UNKNOWN".to_owned()
    } else {
        format!("h{}", model.max_steps)
    };
    format!(
        "{}D · {grid} · {} → {} ch · {horizon}",
        model.dimension,
        known_u32(model.in_channels),
        known_u32(model.out_channels),
    )
}

/// Plain-language contract line (§4.6 card row 2). Same data as
/// `support_summary`, human-ordered; UNKNOWN stays UNKNOWN.
fn contract_line(model: &ModelCard) -> String {
    if model.dimension == 0 {
        return "Contract UNKNOWN — no field dimension declared".into();
    }
    let grid = if model.grid == 0 {
        "UNKNOWN grid".into()
    } else if model.dimension == 3 {
        format!("{}³ grid", model.grid)
    } else {
        format!("{}² grid", model.grid)
    };
    format!(
        "{}D velocity field · {grid} · {} → {} channels · horizon {}",
        model.dimension,
        known_u32(model.in_channels),
        known_u32(model.out_channels),
        known_u32(model.max_steps)
    )
}

/// Filenames are data, not titles (§3.1): the card title is the humanized
/// name; the exact filename renders in mono-s beneath it.
fn humanize_name(name: &str) -> String {
    let stem = name.strip_suffix(".pth").unwrap_or(name);
    let spaced: String = stem
        .chars()
        .map(|character| match character {
            '_' | '-' => ' ',
            other => other,
        })
        .collect();
    let mut characters = spaced.chars();
    let humanized = match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => spaced,
    };
    if humanized.chars().count() <= 34 {
        return humanized;
    }
    let truncated: String = humanized.chars().take(33).collect();
    format!("{truncated}…")
}

fn compact_name(name: &str) -> String {
    let stem = name.strip_suffix(".pth").unwrap_or(name);
    if stem.chars().count() <= 38 {
        return stem.into();
    }
    let start: String = stem.chars().take(25).collect();
    let end: String = stem
        .chars()
        .rev()
        .take(10)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{start}…{end}")
}

/// Toolbar filter chip: label + optional count dot color; tonal when active.
fn filter_chip(ui: &mut egui::Ui, label: &str, active: bool, accent: Option<Color32>) -> bool {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        body_strong().resolve(ui.style()),
        TEXT_DIM,
    );
    let width = galley.size().x + 22.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::click());
    let hover = motion_t(
        ui.ctx(),
        response.id.with("hover"),
        response.hovered(),
        0.12,
    );
    let fill = if active {
        SURFACE_HIGHEST
    } else {
        SURFACE.lerp_to_gamma(SURFACE_HIGH, hover)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(R1), fill);
    painter.rect_stroke(
        rect,
        CornerRadius::same(R1),
        Stroke::new(1.0, if active { OUTLINE } else { HAIRLINE }),
        egui::StrokeKind::Inside,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(R1),
            focus_stroke(),
            egui::StrokeKind::Outside,
        );
    }
    let mut text_x = rect.min.x + 11.0;
    if let Some(color) = accent {
        painter.circle_filled(egui::pos2(rect.min.x + 13.0, rect.center().y), 3.0, color);
        text_x += 10.0;
    }
    painter.galley(
        egui::pos2(text_x, rect.center().y - galley.size().y / 2.0),
        galley,
        if active { TEXT } else { TEXT_DIM },
    );
    response.clicked()
}

/// One calm feedback area for the import flow (notice + structured
/// validation), rendered inline under the toolbar — never a floating console.
fn import_feedback(ui: &mut egui::Ui, state: &mut LibraryState) {
    let mut clear_notice = false;
    if let Some((message, is_error)) = &state.notice {
        let hue = if *is_error { DANGER } else { OK };
        let glyph = if *is_error { "!" } else { "✓" };
        Frame::NONE
            .fill(tint_fill(hue))
            .stroke(Stroke::new(1.0, tint_hairline(hue)))
            .corner_radius(CornerRadius::same(R1))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(glyph).text_style(caption()).color(hue));
                    ui.label(RichText::new(message).text_style(caption()).color(TEXT_DIM));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Dismiss").clicked() {
                            clear_notice = true;
                        }
                    });
                });
            });
        ui.add_space(10.0);
    }
    if clear_notice {
        state.notice = None;
    }
    let mut clear_validation = false;
    if let Some(validation) = &state.validation {
        // Single owner of the import rejection: plain-language line first,
        // verbatim structured codes beneath (SCI semantics preserved).
        Frame::NONE
            .fill(tint_fill(DANGER))
            .stroke(Stroke::new(1.0, tint_hairline(DANGER)))
            .corner_radius(CornerRadius::same(R1))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("×").text_style(caption()).color(DANGER));
                    ui.label(
                        RichText::new("Import rejected — the active checkpoint is unchanged.")
                            .text_style(body_strong())
                            .color(TEXT),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Dismiss").clicked() {
                            clear_validation = true;
                        }
                    });
                });
                if validation.issues.is_empty() {
                    ui.label(
                        RichText::new(&validation.summary)
                            .text_style(caption())
                            .color(TEXT_DIM),
                    );
                } else {
                    ui.add_space(4.0);
                    ui.label(overline_text("Structured validation"));
                    for issue in &validation.issues {
                        ui.label(
                            RichText::new(format!("{} · {}", issue.code, issue.message))
                                .text_style(mono_s())
                                .color(TEXT_DIM),
                        )
                        .on_hover_text(format!("Field: {} · {}", issue.field, issue.severity));
                    }
                }
            });
        ui.add_space(10.0);
    }
    if clear_validation {
        state.validation = None;
    }
}

/// Skeleton card rows while the engine inventory loads (§3.7 shimmer).
fn skeleton_grid(ui: &mut egui::Ui, columns: usize) {
    let time = ui.input(|input| input.time);
    let animate = !reduced_motion(ui.ctx());
    for row in 0..2 {
        ui.columns(columns, |cells| {
            for (column, cell) in cells.iter_mut().enumerate() {
                let phase = (row * columns + column) as f64 * 0.35;
                let pulse = if animate {
                    (((time * std::f64::consts::TAU / 1.1 + phase).sin() * 0.5 + 0.5) * 0.16) as f32
                } else {
                    0.08
                };
                Frame::NONE
                    .fill(SURFACE)
                    .corner_radius(CornerRadius::same(R2))
                    .inner_margin(Margin::same(16))
                    .show(cell, |ui| {
                        ui.set_width(ui.available_width());
                        for width_fraction in [0.55, 0.85, 0.7, 0.4] {
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width() * width_fraction, 13.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(3),
                                Color32::from_white_alpha((10.0 + pulse * 255.0 * 0.12) as u8),
                            );
                            ui.add_space(9.0);
                        }
                    });
            }
        });
        ui.add_space(12.0);
    }
    if animate {
        ui.ctx().request_repaint();
    }
}

pub fn show_library(
    ui: &mut egui::Ui,
    models: &[ModelCard],
    current_model: &str,
    state: &mut LibraryState,
) -> Option<LibraryAction> {
    let mut action = None;
    content_column(ui, CONTENT_MAX_WIDTH, |ui| {
        ui.add_space(26.0);

        // Header: the ONLY "Model Library" title; right side carries the
        // active-checkpoint chip and the screen's single ember action.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(display_text("Model Library"));
                ui.label(
                    RichText::new(
                        "Local checkpoints, declared contracts, and provenance gaps—before inference.",
                    )
                    .text_style(caption())
                    .color(TEXT_MUTE),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let import_label = if state.busy {
                    "Validating…"
                } else {
                    "Import checkpoint…"
                };
                if ui
                    .add_enabled(
                        !state.busy,
                        egui::Button::new(RichText::new(import_label).color(ON_EMBER))
                            .fill(EMBER)
                            .min_size(Vec2::new(150.0, 32.0)),
                    )
                    .clicked()
                {
                    action = Some(LibraryAction::Import);
                }
                ui.add_space(8.0);
                if let Some(model) = models.iter().find(|model| model.id == current_model) {
                    Frame::NONE
                        .fill(SURFACE)
                        .stroke(Stroke::new(1.0, HAIRLINE))
                        .corner_radius(CornerRadius::same(R1))
                        .inner_margin(Margin::symmetric(9, 5))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("◆ {}", compact_name(&model.name)))
                                    .text_style(mono_s())
                                    .color(TEXT_DIM),
                            )
                            .on_hover_text(format!(
                                "Active checkpoint\n{}\n{}\nsha256 {}",
                                model.name,
                                support_summary(model),
                                model.checkpoint_sha256
                            ));
                        });
                }
            });
        });
        ui.add_space(16.0);

        // Toolbar: search · dimension · health filter chips · refresh.
        let (total, review, rejected) = health_counts(models);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.search)
                    .hint_text("Filter by name or regime…")
                    .desired_width(230.0),
            );
            ui.add_space(6.0);
            Frame::NONE
                .fill(SURFACE)
                .corner_radius(CornerRadius::same(R2))
                .stroke(Stroke::new(1.0, HAIRLINE))
                .inner_margin(2)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for dimension in DimensionFilter::ALL {
                            let selected = state.dimension == dimension;
                            let (fill, color) = if selected {
                                (SURFACE_HIGHEST, TEXT)
                            } else {
                                (Color32::TRANSPARENT, TEXT_DIM)
                            };
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(dimension.label())
                                            .text_style(body_strong())
                                            .color(color),
                                    )
                                    .fill(fill)
                                    .corner_radius(CornerRadius::same(R1))
                                    .stroke(Stroke::NONE),
                                )
                                .clicked()
                            {
                                state.dimension = dimension;
                            }
                        }
                    });
                });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !state.busy,
                        egui::Button::new(
                            RichText::new(egui_phosphor::regular::ARROWS_CLOCKWISE).size(14.0),
                        ),
                    )
                    .on_hover_text("Refresh inventory")
                    .clicked()
                {
                    action = Some(LibraryAction::Refresh);
                }
            });
        });
        // Health chips on their own wrapped line (QA L1): they can no longer
        // collide with the refresh button at narrow window widths.
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if filter_chip(
                ui,
                &summary_phrase(total, 0),
                state.health == HealthFilter::All,
                None,
            ) {
                state.health = HealthFilter::All;
            }
            if review > 0
                && filter_chip(
                    ui,
                    &format!("{review} need metadata review"),
                    state.health == HealthFilter::NeedsReview,
                    Some(WARN),
                )
            {
                state.health = if state.health == HealthFilter::NeedsReview {
                    HealthFilter::All
                } else {
                    HealthFilter::NeedsReview
                };
            }
            if rejected > 0
                && filter_chip(
                    ui,
                    &format!("{rejected} rejected"),
                    state.health == HealthFilter::Rejected,
                    Some(DANGER),
                )
            {
                state.health = if state.health == HealthFilter::Rejected {
                    HealthFilter::All
                } else {
                    HealthFilter::Rejected
                };
            }
        });
        ui.add_space(12.0);

        // Import-flow feedback lives inline, right under the toolbar.
        import_feedback(ui, state);

        let columns = ((ui.available_width() + 12.0) / 312.0)
            .floor()
            .clamp(1.0, 4.0) as usize;

        if state.busy && models.is_empty() {
            skeleton_grid(ui, columns);
            return;
        }
        if models.is_empty() {
            Frame::NONE
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, HAIRLINE))
                .corner_radius(CornerRadius::same(R2))
                .inner_margin(Margin::same(28))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(title_text("No checkpoints found"));
                    ui.label(
                        RichText::new(
                            "Import a PyTorch .pth checkpoint. Reyn validates its model contract before adding it to the managed library.",
                        )
                        .text_style(caption())
                        .color(TEXT_MUTE),
                    );
                    ui.add_space(10.0);
                    if ui.button("Import checkpoint…").clicked() {
                        action = Some(LibraryAction::Import);
                    }
                });
            return;
        }

        let filtered: Vec<ModelCard> = models
            .iter()
            .filter(|model| visible(model, state))
            .cloned()
            .collect();
        if filtered.is_empty() {
            ui.label(
                RichText::new("No models match the current search and filters.")
                    .text_style(caption())
                    .color(TEXT_MUTE),
            );
            if ui.button("Clear filters").clicked() {
                state.search.clear();
                state.dimension = DimensionFilter::All;
                state.health = HealthFilter::All;
            }
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for row in filtered.chunks(columns) {
                    ui.columns(columns, |cells| {
                        for (column, model) in row.iter().enumerate() {
                            if let Some(card_action) =
                                model_card(&mut cells[column], model, current_model, state)
                            {
                                action = Some(card_action);
                            }
                        }
                    });
                    ui.add_space(12.0);
                }
                // Bottom breathing room so the last row never touches the
                // panel edge (QA L6).
                ui.add_space(24.0);
            });
    });
    action
}

fn model_card(
    ui: &mut egui::Ui,
    model: &ModelCard,
    current_model: &str,
    state: &mut LibraryState,
) -> Option<LibraryAction> {
    let mut action = None;
    let active = model.id == current_model;
    let card_health = health(&model.status);
    let rejected = card_health == Health::Rejected;
    // Rejected cards recede: dimmed text, red only on the dot + status chip.
    let (title_color, body_color, detail_color) = if rejected {
        (TEXT_DIM, TEXT_MUTE, TEXT_MUTE)
    } else {
        (TEXT, TEXT_DIM, TEXT_MUTE)
    };
    let frame_response = Frame::NONE
        .fill(if active { SURFACE_HIGH } else { SURFACE })
        .stroke(Stroke::new(1.0, if active { OUTLINE } else { HAIRLINE }))
        .corner_radius(CornerRadius::same(R2))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_height(200.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("●")
                        .text_style(caption())
                        .color(status_color(&model.status)),
                );
                // Title truncates; the status chip keeps priority (QA L4).
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let chip = ui.label(
                        chip_text(status_label(&model.status)).color(status_color(&model.status)),
                    );
                    if card_health == Health::Ready {
                        chip.on_hover_text(
                            "Declared contract validates against engine requirements.",
                        );
                    }
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(humanize_name(&model.name))
                                    .text_style(title())
                                    .color(title_color),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&model.name);
                    });
                });
            });
            ui.label(
                RichText::new(compact_name(&model.name))
                    .text_style(mono_s())
                    .color(detail_color),
            )
            .on_hover_text(&model.name);
            ui.add_space(6.0);
            ui.label(RichText::new(contract_line(model)).color(body_color));
            if !model.status_detail.is_empty() && card_health != Health::Ready {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(&model.status_detail)
                        .text_style(caption())
                        .color(if rejected { DANGER } else { WARN }),
                );
            }
            ui.add_space(6.0);
            ui.collapsing("Details & provenance", |ui| {
                egui::Grid::new(("model.facts", &model.id))
                    .num_columns(2)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        fact(ui, "Regime", known_text(&model.scenario));
                        ui.end_row();
                        fact(ui, "Epoch", &epoch_text(model));
                        ui.end_row();
                        fact(ui, "Role", known_text(compact_role(&model.checkpoint_role)));
                        ui.end_row();
                        fact(ui, "Contract", &support_summary(model));
                        ui.end_row();
                        fact(ui, "Size", &format_bytes(model.size_bytes));
                        ui.end_row();
                        fact(ui, "Modified", &format_modified(model.modified_unix));
                        ui.end_row();
                    });
                ui.add_space(5.0);
                ui.label(overline_text("Declared envelope"));
                if model.support.is_empty() {
                    ui.label(
                        RichText::new("UNKNOWN — no applicability envelope declared")
                            .text_style(caption())
                            .color(WARN),
                    );
                } else {
                    for support in &model.support {
                        ui.label(
                            RichText::new(format!("• {support}"))
                                .text_style(caption())
                                .color(TEXT_DIM),
                        );
                    }
                }
                if model.physics_contract != "unknown" && model.physics_contract != "legacy/unknown"
                {
                    ui.label(
                        RichText::new(format!("• physics contract: {}", model.physics_contract))
                            .text_style(caption())
                            .color(TEXT_DIM),
                    );
                }
                ui.add_space(5.0);
                ui.label(overline_text("Limitations"));
                if model.limitations.is_empty() {
                    ui.label(
                        RichText::new("UNKNOWN — checkpoint declares no limitations")
                            .text_style(caption())
                            .color(WARN),
                    );
                } else {
                    for limitation in &model.limitations {
                        ui.label(
                            RichText::new(format!("• {limitation}"))
                                .text_style(caption())
                                .color(TEXT_DIM),
                        );
                    }
                }
                ui.add_space(5.0);
                ui.label(overline_text("Benchmark reports"));
                if model.benchmark_report_hashes.is_empty() {
                    ui.label(
                        RichText::new("NONE DECLARED")
                            .text_style(mono_s())
                            .color(TEXT_MUTE),
                    );
                } else {
                    for digest in &model.benchmark_report_hashes {
                        ui.label(RichText::new(digest).text_style(mono_s()).color(BRAND))
                            .on_hover_text("Canonical report SHA-256");
                    }
                }
                if !model.unknown_fields.is_empty() {
                    ui.add_space(5.0);
                    ui.label(overline_text("Unknown legacy fields"));
                    ui.label(
                        RichText::new(model.unknown_fields.join(" · "))
                            .text_style(mono_s())
                            .color(WARN),
                    );
                }
            });
            // Footer flows after content (QA L2): no bottom_up inside a
            // height-unbounded grid cell, which stretched cards and let the
            // footer overlap opened disclosure content.
            ui.add_space(10.0);
            {
                if state.pending_delete.as_deref() == Some(&model.id) {
                    // Inline destructive confirmation — danger fill is one of
                    // the two sanctioned full-saturation uses (§3.3).
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Delete checkpoint").color(Color32::WHITE),
                                )
                                .fill(DANGER),
                            )
                            .clicked()
                        {
                            action = Some(LibraryAction::Delete(model.id.clone()));
                        }
                        if ui.button("Cancel").clicked() {
                            state.pending_delete = None;
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        if active {
                            // Ember budget (QA L5): the edge marker carries
                            // the accent; the chip is a quiet state token.
                            ui.label(chip_text("◆ ACTIVE").color(TEXT_DIM));
                        } else {
                            if ui
                                .add_enabled(!rejected, egui::Button::new("Set active"))
                                .clicked()
                            {
                                action = Some(LibraryAction::Activate(model.id.clone()));
                            }
                            if rejected {
                                // Disabled-with-reason, always inline (A9).
                                ui.label(
                                    RichText::new("Rejected checkpoints can't be activated")
                                        .text_style(caption())
                                        .color(TEXT_MUTE),
                                );
                            }
                        }
                        if model.managed
                            && ui
                                .add_enabled(
                                    !active,
                                    egui::Button::new(RichText::new("Delete").color(TEXT_DIM)),
                                )
                                .on_disabled_hover_text("Activate another model before deleting")
                                .clicked()
                        {
                            state.pending_delete = Some(model.id.clone());
                        }
                    });
                }
            }
        });
    if active {
        // Active card: 2px ember edge marker, not a brighter fill (§4.6).
        let rect = frame_response.response.rect;
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                rect.min + Vec2::new(0.0, 10.0),
                Vec2::new(2.0, rect.height() - 20.0),
            ),
            CornerRadius::same(1),
            EMBER,
        );
    }
    action
}

fn fact(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).text_style(caption()).color(TEXT_MUTE));
    ui.label(RichText::new(value).text_style(mono_s()).color(TEXT_DIM));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(name: &str, dimension: u32) -> ModelCard {
        ModelCard {
            id: name.into(),
            name: name.into(),
            managed: false,
            size_bytes: 10,
            modified_unix: 0,
            status: "clean".into(),
            status_detail: String::new(),
            dimension,
            grid: 128,
            in_channels: 4,
            out_channels: 2,
            max_steps: 64,
            epoch: 40,
            declared_epochs: 40,
            checkpoint_role: "fixed_final".into(),
            scenario: "obstacle".into(),
            source_digest: Some("abc".into()),
            checkpoint_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .into(),
            physics_contract: "legacy/unknown".into(),
            support: vec!["3D grid".into()],
            limitations: vec![],
            benchmark_report_hashes: vec![],
            unknown_fields: vec![],
        }
    }

    #[test]
    fn library_filter_combines_dimension_and_query() {
        let mut state = LibraryState {
            search: "wake".into(),
            dimension: DimensionFilter::ThreeD,
            ..Default::default()
        };
        assert!(visible(&model("wake-3d.pth", 3), &state));
        assert!(!visible(&model("wake-2d.pth", 2), &state));
        state.search = "free".into();
        assert!(!visible(&model("wake-3d.pth", 3), &state));
    }

    #[test]
    fn health_filter_chips_partition_the_inventory() {
        let mut rejected = model("rejected.pth", 3);
        rejected.status = "invalid".into();
        let mut review = model("review.pth", 3);
        review.status = "review".into();
        let clean = model("clean.pth", 3);

        let mut state = LibraryState {
            health: HealthFilter::Rejected,
            ..Default::default()
        };
        assert!(visible(&rejected, &state));
        assert!(!visible(&review, &state));
        assert!(!visible(&clean, &state));

        state.health = HealthFilter::NeedsReview;
        assert!(!visible(&rejected, &state));
        assert!(visible(&review, &state));
        assert!(!visible(&clean, &state));

        state.health = HealthFilter::All;
        assert!(visible(&rejected, &state));
        assert!(visible(&review, &state));
        assert!(visible(&clean, &state));

        let counts = health_counts(&[rejected, review, clean]);
        assert_eq!(counts, (3, 1, 1));
        assert_eq!(
            summary_phrase(38, 9),
            "38 checkpoints — 9 need metadata review"
        );
        assert_eq!(summary_phrase(1, 0), "1 checkpoint");
    }

    #[test]
    fn byte_format_is_engineering_legible() {
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2.00 GiB");
        assert!(compact_name("obstacle_physics_h64_mixed_delta_seed0_epoch40.pth").contains('…'));
    }

    #[test]
    fn absent_metadata_is_rendered_as_unknown() {
        let mut legacy = model("legacy.pth", 2);
        legacy.grid = 0;
        legacy.max_steps = 0;
        legacy.epoch = 0;
        legacy.declared_epochs = 0;
        legacy.scenario = "unknown".into();
        assert!(support_summary(&legacy).contains("UNKNOWN grid"));
        // QA L3: "hUNKNOWN" read as one token; UNKNOWN stays explicit but
        // legible and matches contract_line's wording.
        assert!(support_summary(&legacy).contains("horizon UNKNOWN"));
        assert!(!support_summary(&legacy).contains("hUNKNOWN"));
        assert!(contract_line(&legacy).contains("UNKNOWN grid"));
        assert!(contract_line(&legacy).contains("horizon UNKNOWN"));
        assert_eq!(epoch_text(&legacy), "UNKNOWN");
        assert_eq!(known_text(&legacy.scenario), "UNKNOWN");
    }

    #[test]
    fn card_titles_humanize_filenames_but_keep_the_exact_name_in_mono() {
        assert_eq!(humanize_name("direct_v1_latest.pth"), "Direct v1 latest");
        assert!(humanize_name("obstacle_physics_h64_mixed_delta_seed0_epoch40.pth").ends_with('…'));
        // The exact filename remains available (data, not title).
        assert_eq!(compact_name("direct_v1_latest.pth"), "direct_v1_latest");
    }
}
