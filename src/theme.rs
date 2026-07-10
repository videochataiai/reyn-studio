//! "Precision Instrument" design tokens — warm-dark ember palette (DESIGN.md).
#![allow(dead_code)] // full palette kept; some tokens used by later views
use egui::Color32;

pub const BG: Color32 = Color32::from_rgb(0x1c, 0x11, 0x0b); // canvas
pub const SURFACE_LOWEST: Color32 = Color32::from_rgb(0x16, 0x0c, 0x06); // nav/rails
pub const SURFACE_LOW: Color32 = Color32::from_rgb(0x25, 0x19, 0x12);
pub const SURFACE: Color32 = Color32::from_rgb(0x29, 0x1d, 0x16); // cards
pub const SURFACE_HIGH: Color32 = Color32::from_rgb(0x34, 0x27, 0x20); // inputs/elevated
pub const SURFACE_HIGHEST: Color32 = Color32::from_rgb(0x40, 0x32, 0x2a);

pub const EMBER: Color32 = Color32::from_rgb(0xff, 0x7a, 0x1a); // primary action
pub const BRAND: Color32 = Color32::from_rgb(0xff, 0xb6, 0x8e); // brand text / highlights
pub const GOLD: Color32 = Color32::from_rgb(0xf7, 0xbe, 0x1d); // secondary data
pub const TERTIARY: Color32 = Color32::from_rgb(0x8a, 0xce, 0xff); // data blue

pub const TEXT: Color32 = Color32::from_rgb(0xf5, 0xde, 0xd3);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0xe0, 0xc0, 0xb1);
pub const TEXT_MUTE: Color32 = Color32::from_rgb(0xa7, 0x8b, 0x7d);

pub const OUTLINE: Color32 = Color32::from_rgb(0xa7, 0x8b, 0x7d); // strong border
pub const OUTLINE_VARIANT: Color32 = Color32::from_rgb(0x58, 0x42, 0x36); // hairline / tech grid

pub const SUCCESS: Color32 = Color32::from_rgb(0x34, 0xd3, 0x99);
pub const ON_EMBER: Color32 = Color32::from_rgb(0x2a, 0x14, 0x00);

/// Apply the instrument theme to an egui context (colors, spacing, rounding).
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = SURFACE_LOWEST; // text edit backgrounds
    visuals.faint_bg_color = SURFACE_LOW;
    visuals.hyperlink_color = TERTIARY;
    visuals.selection.bg_fill = EMBER.linear_multiply(0.35);
    visuals.selection.stroke = egui::Stroke::new(1.0, EMBER);

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = SURFACE;
    w.noninteractive.bg_stroke = egui::Stroke::new(1.0, OUTLINE_VARIANT);
    w.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_DIM);
    w.inactive.bg_fill = SURFACE_HIGH;
    w.inactive.weak_bg_fill = SURFACE_HIGH;
    w.inactive.bg_stroke = egui::Stroke::new(1.0, OUTLINE_VARIANT);
    w.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_DIM);
    w.hovered.bg_fill = SURFACE_HIGHEST;
    w.hovered.weak_bg_fill = SURFACE_HIGHEST;
    w.hovered.bg_stroke = egui::Stroke::new(1.0, OUTLINE);
    w.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT);
    w.active.bg_fill = EMBER;
    w.active.weak_bg_fill = SURFACE_HIGHEST;
    w.active.bg_stroke = egui::Stroke::new(1.0, EMBER);
    w.active.fg_stroke = egui::Stroke::new(1.0, TEXT);

    let r = egui::CornerRadius::same(3);
    for s in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        s.corner_radius = r;
    }

    // egui 0.35: apply to every theme slot so light/dark both use the instrument look.
    ctx.all_styles_mut(|style| {
        style.visuals = visuals.clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
    });
}
