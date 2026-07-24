//! Brand typography: Inter (UI) + JetBrains Mono (all numerals/data).
//!
//! Ships static weight instances (egui has no variable-axis support), so the
//! app has true Regular / Medium / SemiBold weights. Medium and SemiBold are
//! registered as named `FontFamily`s consumed by the `theme` type scale.
//! Phosphor (Regular) supplies the single UI icon voice (§3.6).
//!
//! IMPORTANT: the bundled Inter/JetBrains Mono instances are *stripped of
//! their Private Use Area cmap entries*. Upstream Inter maps ~745 PUA
//! codepoints to stylistic-alternate glyphs (U+E24A → "ṁ", U+E270 → "ÿ", …);
//! because Inter sits ahead of Phosphor in the fallback chain, those
//! mappings shadow the icon font and the UI renders stray letters instead of
//! icons. If you ever replace the font files, re-strip the PUA range
//! (U+E000–U+F8FF) — the regression test below fails otherwise.
use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Inter Medium — `body-strong`, buttons, nav labels, `overline` eyebrows.
pub const FAMILY_MEDIUM: &str = "inter-medium";
/// Inter SemiBold — `display` and `title` styles.
pub const FAMILY_SEMIBOLD: &str = "inter-semibold";
/// JetBrains Mono Medium — `mono-chip` scientific-state tokens only.
pub const FAMILY_MONO_MEDIUM: &str = "jbmono-medium";

/// Build the full font table: brand fonts first in the default families,
/// Phosphor icons as a Proportional fallback, and the named weight families.
pub fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    for (key, bytes) in [
        ("inter", &include_bytes!("../assets/Inter-Regular.ttf")[..]),
        (
            "inter-medium",
            &include_bytes!("../assets/Inter-Medium.ttf")[..],
        ),
        (
            "inter-semibold",
            &include_bytes!("../assets/Inter-SemiBold.ttf")[..],
        ),
        (
            "jbmono",
            &include_bytes!("../assets/JetBrainsMono-Regular.ttf")[..],
        ),
        (
            "jbmono-medium",
            &include_bytes!("../assets/JetBrainsMono-Medium.ttf")[..],
        ),
    ] {
        fonts
            .font_data
            .insert(key.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    // Our fonts first in each default family so they take priority over the
    // bundled fallbacks (which stay available for glyph coverage).
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "jbmono".to_owned());
    // Phosphor Regular right behind Inter so icon glyphs resolve everywhere
    // proportional text is drawn (inserts itself at index 1).
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    // Icons must also resolve in monospace contexts (status glyphs, chips).
    // Appended last: phosphor's a–z/space ligature access codepoints stay
    // shadowed by the real text fonts ahead of it.
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("phosphor".to_owned());

    // Named weight families keep the default fallback chain behind them so
    // symbols/emoji/icons still resolve when a glyph is missing from the
    // weight instance.
    let proportional_fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let monospace_fallback = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    let with_fallback = |first: &str, fallback: &[String]| {
        let mut family = vec![first.to_owned()];
        family.extend(fallback.iter().cloned());
        family
    };
    fonts.families.insert(
        FontFamily::Name(FAMILY_MEDIUM.into()),
        with_fallback("inter-medium", &proportional_fallback),
    );
    fonts.families.insert(
        FontFamily::Name(FAMILY_SEMIBOLD.into()),
        with_fallback("inter-semibold", &proportional_fallback),
    );
    fonts.families.insert(
        FontFamily::Name(FAMILY_MONO_MEDIUM.into()),
        with_fallback("jbmono-medium", &monospace_fallback),
    );
    fonts
}

pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tier 1 mechanics guard: every named family the theme resolves must be
    /// registered here and its first entry must point at loaded font data —
    /// a mismatch would silently fall back to Regular and erase the weights.
    #[test]
    fn named_weight_families_are_registered_and_backed_by_font_data() {
        let fonts = definitions();
        for family in [FAMILY_MEDIUM, FAMILY_SEMIBOLD, FAMILY_MONO_MEDIUM] {
            let chain = fonts
                .families
                .get(&FontFamily::Name(family.into()))
                .unwrap_or_else(|| panic!("family {family} is not registered"));
            let first = chain.first().expect("family chain is empty");
            assert!(
                fonts.font_data.contains_key(first),
                "family {family} resolves to missing font data {first}"
            );
            assert!(
                chain.len() > 1,
                "family {family} has no fallback chain for glyph coverage"
            );
        }
        // Icon font present and reachable from proportional text.
        assert!(fonts.font_data.contains_key("phosphor"));
        let proportional = fonts.families.get(&FontFamily::Proportional).unwrap();
        assert_eq!(proportional[0], "inter");
        assert!(proportional.contains(&"phosphor".to_owned()));
        assert_eq!(
            fonts.families.get(&FontFamily::Monospace).unwrap()[0],
            "jbmono"
        );
    }

    /// Icon regression guard: the phosphor icon font must be present, and no
    /// font ordered *before* it in the proportional fallback chain may claim
    /// Private Use Area codepoints. Upstream Inter ships ~745 PUA mappings
    /// for its stylistic alternates (U+E24A → "ṁ", U+E270 → "ÿ", …); if those
    /// mappings are present they shadow the phosphor icons and the nav renders
    /// letters instead of glyphs. The bundled Inter instances are therefore
    /// stripped of their PUA cmap entries — this test fails if anyone swaps in
    /// unstripped fonts or reorders the chain.
    #[test]
    fn phosphor_codepoints_resolve_to_phosphor_not_a_text_font() {
        let fonts = definitions();
        let proportional = fonts.families.get(&FontFamily::Proportional).unwrap();
        let phosphor_index = proportional
            .iter()
            .position(|name| name == "phosphor")
            .expect("phosphor font missing from the proportional fallback chain");

        // Every face ahead of phosphor must leave the PUA to the icon font.
        for name in &proportional[..phosphor_index] {
            let data = fonts
                .font_data
                .get(name)
                .unwrap_or_else(|| panic!("font data missing for {name}"));
            let face =
                fontdue::Font::from_bytes(data.font.as_ref(), fontdue::FontSettings::default())
                    .unwrap_or_else(|error| panic!("could not parse {name}: {error}"));
            let claimed: Vec<u32> = (0xE000..=0xF8FF)
                .filter_map(char::from_u32)
                .filter(|c| face.lookup_glyph_index(*c) != 0)
                .map(|c| c as u32)
                .collect();
            assert!(
                claimed.is_empty(),
                "{name} claims {} PUA codepoints (e.g. U+{:04X}) and would shadow \
                 the phosphor icons behind it in the fallback chain",
                claimed.len(),
                claimed[0],
            );
        }

        // The icon glyphs the shell uses must exist in the phosphor face.
        let phosphor = fonts.font_data.get("phosphor").unwrap();
        let face =
            fontdue::Font::from_bytes(phosphor.font.as_ref(), fontdue::FontSettings::default())
                .expect("phosphor font data must parse");
        for icon in [
            egui_phosphor::regular::FOLDER,
            egui_phosphor::regular::WIND,
            egui_phosphor::regular::CHART_BAR,
            egui_phosphor::regular::BOOK_OPEN,
            egui_phosphor::regular::CUBE,
            egui_phosphor::regular::GEAR,
            egui_phosphor::regular::PLAY,
        ] {
            let c = icon.chars().next().unwrap();
            assert_ne!(
                face.lookup_glyph_index(c),
                0,
                "phosphor face lacks U+{:04X}",
                c as u32
            );
        }
    }
}
