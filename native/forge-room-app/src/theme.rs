use eframe::egui::{self, Color32, FontFamily, FontId, TextStyle};

pub const ACCENT: Color32 = Color32::from_rgb(199, 76, 69);
pub const BG_EDITOR: Color32 = Color32::from_rgb(15, 16, 17);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(22, 23, 25);
pub const BG_ACTIVITY: Color32 = Color32::from_rgb(18, 19, 20);
pub const BG_BAR: Color32 = Color32::from_rgb(27, 28, 30);
pub const BG_TAB_IDLE: Color32 = Color32::from_rgb(22, 23, 25);
pub const BORDER: Color32 = Color32::from_rgb(41, 42, 45);
pub const TEXT: Color32 = Color32::from_rgb(232, 232, 230);
pub const TEXT_DIM: Color32 = Color32::from_rgb(150, 152, 156);

pub fn configure(context: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_EDITOR;
    visuals.window_fill = BG_BAR;
    visuals.extreme_bg_color = BG_EDITOR;
    visuals.window_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(199, 76, 69, 110);
    visuals.selection.stroke = egui::Stroke::new(1.0, TEXT);
    visuals.hyperlink_color = Color32::from_rgb(218, 126, 126);
    visuals.widgets.active.bg_fill = ACCENT;
    context.set_visuals(visuals);

    context.all_styles_mut(|style| {
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        );
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(13.5, FontFamily::Proportional),
        );
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    });
}
