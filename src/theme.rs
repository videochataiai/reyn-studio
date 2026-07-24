//! "Calm instrument" design tokens — warm-dark ember identity, recalibrated
//! (docs/DESIGN_OVERHAUL.md §3). Surfaces are warm-neutral with real value
//! steps, text is a warm off-white triad, and chroma is spent only on the
//! ember accent, semantic status, and data colors.
#![allow(dead_code)] // full token set kept; some tokens used by later views
use egui::{Color32, RichText, TextStyle};

// ---------------------------------------------------------------------------
// Surface ladder (§3.3) — hue ≈ 25°, low chroma, larger value steps.
// ---------------------------------------------------------------------------
/// 3D/2D scientific canvas — darkest; data glows here.
pub const BG_VIEWPORT: Color32 = Color32::from_rgb(0x0E, 0x0C, 0x0A);
/// App canvas.
pub const BG_0: Color32 = Color32::from_rgb(0x15, 0x12, 0x10);
/// Rails, top bar, status bar.
pub const BG_1: Color32 = Color32::from_rgb(0x1C, 0x19, 0x16);
/// Cards, inputs (level-1 elevation).
pub const BG_2: Color32 = Color32::from_rgb(0x24, 0x20, 0x19);
/// Hover, raised, menu base.
pub const BG_3: Color32 = Color32::from_rgb(0x2D, 0x28, 0x22);
/// Overlays, active fills.
pub const BG_4: Color32 = Color32::from_rgb(0x37, 0x31, 0x2A);

// Legacy surface names — aliased in place so every call site inherits.
pub const BG: Color32 = BG_0;
pub const SURFACE_LOWEST: Color32 = BG_1; // nav/rails, top bar
pub const SURFACE_LOW: Color32 = BG_1;
pub const SURFACE: Color32 = BG_2; // cards
pub const SURFACE_HIGH: Color32 = BG_3; // inputs/elevated
pub const SURFACE_HIGHEST: Color32 = BG_4;

/// Separators and quiet borders — visible, not loud.
pub const HAIRLINE: Color32 = Color32::from_rgb(0x3B, 0x34, 0x2C);
/// Interactive borders, focus-adjacent.
pub const OUTLINE: Color32 = Color32::from_rgb(0x57, 0x4C, 0x41);
pub const OUTLINE_VARIANT: Color32 = HAIRLINE;

// ---------------------------------------------------------------------------
// Text triad (§3.3) — warm off-white; the brown cast lives in surfaces only.
// ---------------------------------------------------------------------------
/// Titles, values (≈13:1 on bg-1/bg-2).
pub const TEXT: Color32 = Color32::from_rgb(0xF1, 0xEC, 0xE6);
/// Body, labels (≈8:1).
pub const TEXT_DIM: Color32 = Color32::from_rgb(0xC6, 0xBC, 0xB1);
/// Captions, placeholders — ≥11.5px sizes only (≈4.6:1).
pub const TEXT_MUTE: Color32 = Color32::from_rgb(0x8F, 0x84, 0x78);

// ---------------------------------------------------------------------------
// Accent — spent, not sprayed. Ember marks exactly one primary action per
// screen plus the running-state indicator; never a nav fill or panel border.
// ---------------------------------------------------------------------------
pub const EMBER: Color32 = Color32::from_rgb(0xFF, 0x7A, 0x1A);
/// Wordmark + rare brand moments only.
pub const BRAND: Color32 = Color32::from_rgb(0xFF, 0xB6, 0x8E);
pub const ON_EMBER: Color32 = Color32::from_rgb(0x2A, 0x14, 0x00);

// ---------------------------------------------------------------------------
// Semantic status (§3.3) — never color-only; every status pairs glyph + word.
// ---------------------------------------------------------------------------
/// Pass with a named proposition.
pub const OK: Color32 = Color32::from_rgb(0x3F, 0xBF, 0x8A);
/// UNKNOWN / metadata gaps / stale.
pub const WARN: Color32 = Color32::from_rgb(0xE3, 0xA9, 0x3C);
/// Failed gate, destructive actions, REJECTED.
pub const DANGER: Color32 = Color32::from_rgb(0xE5, 0x54, 0x4B);
/// Neutral notices; also the tertiary data blue.
pub const INFO: Color32 = Color32::from_rgb(0x8A, 0xCE, 0xFF);

pub const SUCCESS: Color32 = OK;
pub const DATA_RED: Color32 = DANGER;

// Data colors (viewport/plots — meanings unchanged).
pub const GOLD: Color32 = Color32::from_rgb(0xF7, 0xBE, 0x1D); // secondary data
pub const TERTIARY: Color32 = INFO; // data blue

// ---------------------------------------------------------------------------
// Radius tokens (§3.5).
// ---------------------------------------------------------------------------
/// Buttons, inputs, chips, segmented items.
pub const R1: u8 = 4;
/// Cards, menus, popovers.
pub const R2: u8 = 6;

// ---------------------------------------------------------------------------
// Alert recipe (§3.3): tint fill at ~12% + same-hue hairline at ~30%; full
// color only on the icon/keyword. Full-saturation fills are allowed in
// exactly two places: the ember primary button and the danger confirm button.
// ---------------------------------------------------------------------------
pub fn tint_fill(color: Color32) -> Color32 {
    color.gamma_multiply(0.12)
}

pub fn tint_hairline(color: Color32) -> Color32 {
    color.gamma_multiply(0.30)
}

// ---------------------------------------------------------------------------
// Type scale (§3.1) — named text styles installed by `apply()`.
// ---------------------------------------------------------------------------
/// Screen title — one per screen, content-owned.
pub fn display() -> TextStyle {
    TextStyle::Name("display".into())
}

/// Card/panel titles, dialog titles.
pub fn title() -> TextStyle {
    TextStyle::Name("title".into())
}

/// Emphasis, buttons, nav labels, table headers.
pub fn body_strong() -> TextStyle {
    TextStyle::Name("body-strong".into())
}

/// Section eyebrows ONLY — the single sanctioned caps style.
pub fn overline() -> TextStyle {
    TextStyle::Name("overline".into())
}

/// Dense evidence detail, hash suffixes.
pub fn mono_s() -> TextStyle {
    TextStyle::Name("mono-s".into())
}

/// Scientific state tokens only: MODEL, RECOVERED, UNKNOWN, UNSIGNED…
pub fn mono_chip() -> TextStyle {
    TextStyle::Name("mono-chip".into())
}

/// Default UI text.
pub fn body() -> TextStyle {
    TextStyle::Body
}

/// Helper text, timestamps prose.
pub fn caption() -> TextStyle {
    TextStyle::Small
}

pub fn display_text(text: &str) -> RichText {
    RichText::new(text)
        .text_style(display())
        .extra_letter_spacing(-0.2)
        .color(TEXT)
}

pub fn title_text(text: &str) -> RichText {
    RichText::new(text)
        .text_style(title())
        .extra_letter_spacing(-0.1)
        .color(TEXT)
}

/// Tracked-caps section eyebrow — the only sanctioned caps style besides
/// `chip_text` scientific-state tokens.
pub fn overline_text(text: &str) -> RichText {
    RichText::new(text.to_uppercase())
        .text_style(overline())
        .extra_letter_spacing(0.8)
        .color(TEXT_MUTE)
}

/// Scientific-state token (caller supplies the semantic color).
pub fn chip_text(text: &str) -> RichText {
    RichText::new(text)
        .text_style(mono_chip())
        .extra_letter_spacing(0.5)
}

// ---------------------------------------------------------------------------
// Motion (§3.7) — 120–220ms ease-out, interruptible, reduced-motion aware.
// ---------------------------------------------------------------------------

fn reduced_motion_id() -> egui::Id {
    egui::Id::new("reyn.reduced-motion")
}

/// Record the user's reduced-motion preference on the context; widgets read
/// it through [`motion_t`]. Also zeroes egui's built-in animation time so
/// collapses/toggles snap when reduced.
pub fn set_reduced_motion(ctx: &egui::Context, reduced: bool) {
    ctx.data_mut(|data| data.insert_temp(reduced_motion_id(), reduced));
    ctx.all_styles_mut(|style| style.animation_time = if reduced { 0.0 } else { 0.16 });
}

pub fn reduced_motion(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp(reduced_motion_id()).unwrap_or(false))
}

/// Ease-out animated bool for hover/press states. Returns the target value
/// instantly when the user prefers reduced motion.
pub fn motion_t(ctx: &egui::Context, id: egui::Id, target: bool, seconds: f32) -> f32 {
    if reduced_motion(ctx) {
        return if target { 1.0 } else { 0.0 };
    }
    ctx.animate_bool_with_time_and_easing(id, target, seconds, egui::emath::easing::cubic_out)
}

/// Keyboard focus ring — ember at ~60%, drawn outside the widget rect.
pub fn focus_stroke() -> egui::Stroke {
    egui::Stroke::new(2.0, EMBER.gamma_multiply(0.6))
}

/// Shared document-screen rhythm (§3.2): every content screen caps its column
/// at the same width and keeps at least this side gutter, so headers align
/// across Projects, Case, Evidence, Library, and Settings (QA G3/G4).
pub const CONTENT_MAX_WIDTH: f32 = 980.0;
pub const CONTENT_MIN_GUTTER: f32 = 34.0;

/// Centered content column (§3.2): content max-width with symmetric gutters
/// instead of a left-anchored block and one dead right gutter. The gutters
/// never collapse below [`CONTENT_MIN_GUTTER`], and the width is capped —
/// never expanded — so narrow panels wrap at the clip edge instead of
/// pushing text off-screen.
pub fn content_column<R>(
    ui: &mut egui::Ui,
    max_width: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let full = ui.available_width();
    let width = (full - 2.0 * CONTENT_MIN_GUTTER).min(max_width).max(120.0);
    let gutter = ((full - width) / 2.0).max(0.0);
    ui.horizontal_top(|ui| {
        ui.add_space(gutter);
        ui.vertical(|ui| {
            ui.set_width(width);
            add(ui)
        })
        .inner
    })
    .inner
}

/// Apply the instrument theme to an egui context (colors, type scale,
/// spacing, rounding, shadows).
pub fn apply(ctx: &egui::Context) {
    apply_with_contrast(ctx, false);
}

/// Preserve the instrument palette while strengthening text and control
/// boundaries for users who need additional contrast.
pub fn apply_with_contrast(ctx: &egui::Context, high_contrast: bool) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(if high_contrast { Color32::WHITE } else { TEXT });
    visuals.panel_fill = BG;
    visuals.window_fill = BG_3; // overlay level (menus, dialogs)
    visuals.extreme_bg_color = BG_2; // text edit backgrounds (inputs)
    visuals.faint_bg_color = BG_1;
    visuals.hyperlink_color = INFO;
    visuals.selection.bg_fill = EMBER.linear_multiply(0.35);
    visuals.selection.stroke = egui::Stroke::new(1.0, EMBER);

    // Overlay elevation (§3.4 level 3): one soft shadow, no stroke.
    let overlay_shadow = egui::epaint::Shadow {
        offset: [0, 15],
        blur: 50,
        spread: 0,
        color: Color32::from_black_alpha(128),
    };
    visuals.popup_shadow = overlay_shadow;
    visuals.window_shadow = overlay_shadow;
    visuals.window_stroke = egui::Stroke::NONE;
    visuals.window_corner_radius = egui::CornerRadius::same(R2);
    visuals.menu_corner_radius = egui::CornerRadius::same(R2);

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = BG_2;
    let quiet_outline = if high_contrast { OUTLINE } else { HAIRLINE };
    let quiet_text = if high_contrast { TEXT } else { TEXT_DIM };
    w.noninteractive.bg_stroke = egui::Stroke::new(1.0, quiet_outline);
    w.noninteractive.fg_stroke = egui::Stroke::new(1.0, quiet_text);
    w.inactive.bg_fill = BG_3;
    w.inactive.weak_bg_fill = BG_3;
    w.inactive.bg_stroke = egui::Stroke::new(1.0, quiet_outline);
    w.inactive.fg_stroke = egui::Stroke::new(1.0, quiet_text);
    w.hovered.bg_fill = BG_4;
    w.hovered.weak_bg_fill = BG_4;
    w.hovered.bg_stroke = egui::Stroke::new(1.0, OUTLINE);
    w.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT);
    // Pressed state stays tonal — ember is reserved for the one primary
    // action per screen, not a generic press flash.
    w.active.bg_fill = BG_4;
    w.active.weak_bg_fill = BG_4;
    w.active.bg_stroke = egui::Stroke::new(1.0, OUTLINE);
    w.active.fg_stroke = egui::Stroke::new(1.0, TEXT);
    w.open.bg_fill = BG_3;
    w.open.weak_bg_fill = BG_3;
    w.open.bg_stroke = egui::Stroke::new(1.0, quiet_outline);

    let r = egui::CornerRadius::same(R1);
    for s in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        s.corner_radius = r;
    }

    let text_styles: std::collections::BTreeMap<TextStyle, egui::FontId> = {
        use egui::{FontFamily, FontId};
        let medium = FontFamily::Name(crate::fonts::FAMILY_MEDIUM.into());
        let semibold = FontFamily::Name(crate::fonts::FAMILY_SEMIBOLD.into());
        let mono_medium = FontFamily::Name(crate::fonts::FAMILY_MONO_MEDIUM.into());
        [
            (display(), FontId::new(22.0, semibold.clone())),
            (title(), FontId::new(16.0, semibold.clone())),
            (body_strong(), FontId::new(13.0, medium.clone())),
            (overline(), FontId::new(10.5, medium.clone())),
            (mono_s(), FontId::new(11.0, FontFamily::Monospace)),
            (mono_chip(), FontId::new(10.5, mono_medium)),
            (TextStyle::Heading, FontId::new(16.0, semibold)),
            (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(13.0, medium)),
            (
                TextStyle::Small,
                FontId::new(11.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(12.5, FontFamily::Monospace),
            ),
        ]
        .into()
    };

    // egui 0.35: apply to every theme slot so light/dark both use the instrument look.
    ctx.all_styles_mut(|style| {
        style.visuals = visuals.clone();
        style.text_styles = text_styles.clone();
        style.animation_time = 0.16;
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.scroll = egui::style::ScrollStyle::floating();
        style.spacing.scroll.bar_width = 6.0;
    });
}
