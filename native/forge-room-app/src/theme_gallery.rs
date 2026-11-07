//! The Color Theme gallery: preview cards painted in each theme's own colors. Picking a card
//! applies it live; Keep saves it, Cancel puts the previous theme back.

use crate::theme::{self, Theme};
use eframe::egui::{
    self, Align2, Color32, FontId, Rect, RichText, Sense, Stroke, epaint::StrokeKind,
};

pub struct Gallery {
    pub open: bool,
    previous: String,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            open: false,
            previous: theme::DEFAULT_THEME.to_string(),
        }
    }
}

pub enum Outcome {
    Keep,
    Cancel(String),
}

impl Gallery {
    pub fn open(&mut self) {
        self.previous = theme::current().key.clone();
        self.open = true;
    }
}

const CARD: egui::Vec2 = egui::vec2(262.0, 196.0);

pub fn show(context: &egui::Context, gallery: &mut Gallery) -> Option<Outcome> {
    if !gallery.open {
        return None;
    }
    let active = theme::current().key.clone();
    let mut picked: Option<String> = None;
    let mut keep = false;
    let mut cancel = false;

    let modal = egui::Modal::new(egui::Id::unique("theme-gallery")).show(context, |ui| {
        ui.set_width(CARD.x * 3.0 + 40.0);
        ui.label(RichText::new("Color Theme").size(22.0).strong());
        ui.label(
            RichText::new(
                "Pick a look. It applies instantly: keep it, or go back to what you had.",
            )
            .color(theme::text_dim()),
        );
        ui.add_space(14.0);
        egui::Grid::new("theme-cards")
            .num_columns(3)
            .spacing([20.0, 20.0])
            .show(ui, |ui| {
                for (index, theme) in theme::all().iter().enumerate() {
                    if card(ui, theme, theme.key == active).clicked() {
                        picked = Some(theme.key.clone());
                    }
                    if index % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let primary = egui::Button::new(
                RichText::new("Keep this theme")
                    .strong()
                    .color(theme::button_text()),
            )
            .fill(theme::button())
            .corner_radius(6.0)
            .min_size(egui::vec2(150.0, 32.0));
            if ui.add(primary).clicked() {
                keep = true;
            }
            if ui
                .add(egui::Button::new("Cancel").min_size(egui::vec2(90.0, 32.0)))
                .clicked()
            {
                cancel = true;
            }
        });
    });
    if modal.should_close() {
        cancel = true;
    }

    if let Some(key) = picked {
        theme::set(context, &key);
    }
    if keep {
        gallery.open = false;
        return Some(Outcome::Keep);
    }
    if cancel {
        gallery.open = false;
        return Some(Outcome::Cancel(gallery.previous.clone()));
    }
    None
}

/// One preview card, drawn entirely with the theme it represents.
fn card(ui: &mut egui::Ui, theme: &Theme, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(CARD, Sense::click());
    let p = theme.palette;
    let painter = ui.painter();
    let lift = if response.hovered() { 1.5 } else { 0.0 };
    let rect = rect.translate(egui::vec2(0.0, -lift));

    // Card body.
    let outer = if selected {
        Stroke::new(2.0, theme::accent())
    } else {
        Stroke::new(1.0, theme::border())
    };
    painter.rect_filled(rect, 10.0, theme::bg_sidebar());
    painter.rect_stroke(rect, 10.0, outer, StrokeKind::Inside);

    // Miniature window.
    let window = Rect::from_min_size(
        rect.min + egui::vec2(10.0, 10.0),
        egui::vec2(CARD.x - 20.0, 112.0),
    );
    painter.rect_filled(window, 6.0, p.bg_editor);
    let activity = Rect::from_min_size(window.min, egui::vec2(14.0, window.height()));
    let sidebar = Rect::from_min_size(
        window.min + egui::vec2(14.0, 0.0),
        egui::vec2(46.0, window.height()),
    );
    painter.rect_filled(activity, 6.0, p.bg_activity);
    painter.rect_filled(sidebar, 0.0, p.bg_sidebar);
    painter.rect_filled(
        Rect::from_min_size(
            egui::pos2(activity.left() + 4.0, activity.top() + 8.0),
            egui::vec2(6.0, 6.0),
        ),
        2.0,
        p.accent,
    );
    for row in 0..4 {
        painter.rect_filled(
            Rect::from_min_size(
                egui::pos2(
                    sidebar.left() + 6.0,
                    sidebar.top() + 10.0 + row as f32 * 12.0,
                ),
                egui::vec2(if row == 1 { 30.0 } else { 22.0 }, 4.0),
            ),
            2.0,
            if row == 1 { p.accent } else { p.text_dim },
        );
    }
    let code_left = sidebar.right() + 10.0;
    painter.rect_filled(
        Rect::from_min_size(
            egui::pos2(sidebar.right(), window.top() + 8.0),
            egui::vec2(window.right() - sidebar.right() - 4.0, 12.0),
        ),
        0.0,
        p.line_highlight,
    );
    // Code lines in the theme's own token colors.
    let lines: [(f32, Color32); 6] = [
        (22.0, p.keyword),
        (48.0, p.function),
        (34.0, p.string),
        (28.0, p.number),
        (58.0, p.type_color),
        (40.0, p.comment),
    ];
    for (row, (width, color)) in lines.iter().enumerate() {
        let y = window.top() + 12.0 + row as f32 * 13.0;
        painter.rect_filled(
            Rect::from_min_size(egui::pos2(code_left, y), egui::vec2(*width, 5.0)),
            2.0,
            *color,
        );
        painter.rect_filled(
            Rect::from_min_size(
                egui::pos2(code_left + width + 6.0, y),
                egui::vec2(26.0 + (row as f32 * 9.0) % 30.0, 5.0),
            ),
            2.0,
            p.variable,
        );
    }
    let status = Rect::from_min_max(
        egui::pos2(window.left(), window.bottom() - 9.0),
        window.right_bottom(),
    );
    painter.rect_filled(status, 0.0, p.status_bar);

    // Name, mood and swatches.
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.top() + 136.0),
        Align2::LEFT_TOP,
        &theme.name,
        FontId::proportional(14.5),
        theme::text(),
    );
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.top() + 156.0),
        Align2::LEFT_TOP,
        &theme.mood,
        FontId::proportional(12.0),
        theme::text_dim(),
    );
    for (index, swatch) in theme.swatches.iter().enumerate() {
        painter.circle_filled(
            egui::pos2(
                rect.right() - 16.0 - index as f32 * 17.0,
                rect.top() + 146.0,
            ),
            6.0,
            *swatch,
        );
    }
    if selected {
        painter.text(
            egui::pos2(rect.left() + 12.0, rect.bottom() - 14.0),
            Align2::LEFT_CENTER,
            "Active",
            FontId::proportional(11.0),
            theme::accent(),
        );
    }
    response.on_hover_text(&theme.description)
}
