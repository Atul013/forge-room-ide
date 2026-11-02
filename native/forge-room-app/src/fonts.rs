//! Use the fonts Windows already ships (Segoe UI, Consolas) so the app looks native and matches
//! the installer. Missing files simply fall back to egui's built-in fonts.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

pub const SEMIBOLD: &str = "semibold";

pub fn semibold() -> FontFamily {
    FontFamily::Name(SEMIBOLD.into())
}

fn load(fonts: &mut FontDefinitions, key: &str, candidates: &[&str]) -> bool {
    let Some(windows) = std::env::var_os("WINDIR") else {
        return false;
    };
    for file in candidates {
        let path = std::path::Path::new(&windows).join("Fonts").join(file);
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(key.to_string(), Arc::new(FontData::from_owned(bytes)));
            return true;
        }
    }
    false
}

pub fn install(context: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    if load(&mut fonts, "segoe-ui", &["segoeui.ttf"]) {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe-ui".into());
    }
    if load(&mut fonts, "consolas", &["consola.ttf"]) {
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "consolas".into());
    }
    let mut semibold_chain: Vec<String> = Vec::new();
    if load(
        &mut fonts,
        "segoe-semibold",
        &["seguisb.ttf", "segoeuib.ttf"],
    ) {
        semibold_chain.push("segoe-semibold".into());
    }
    // The semibold family must always exist, with the normal fonts behind it.
    semibold_chain.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts.families.insert(semibold(), semibold_chain);
    context.set_fonts(fonts);
}
