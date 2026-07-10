//! Brand typography: Inter (UI) + JetBrains Mono (all numerals/data).
use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter".to_owned(),
        Arc::new(FontData::from_static(include_bytes!("../assets/Inter.ttf"))),
    );
    fonts.font_data.insert(
        "jetbrains".to_owned(),
        Arc::new(FontData::from_static(include_bytes!("../assets/JetBrainsMono.ttf"))),
    );
    // Put our fonts first in each family so they take priority over the fallbacks.
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "inter".to_owned());
    fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "jetbrains".to_owned());
    ctx.set_fonts(fonts);
}
