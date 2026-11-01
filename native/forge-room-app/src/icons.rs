//! Small vector icons drawn with the painter, so the app needs no icon font or image assets.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, epaint::StrokeKind};

pub fn chevron(painter: &Painter, center: Pos2, open: bool, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let (a, b, c) = if open {
        (
            center + egui::vec2(-3.5, -1.8),
            center + egui::vec2(0.0, 1.8),
            center + egui::vec2(3.5, -1.8),
        )
    } else {
        (
            center + egui::vec2(-1.8, -3.5),
            center + egui::vec2(1.8, 0.0),
            center + egui::vec2(-1.8, 3.5),
        )
    };
    painter.line_segment([a, b], stroke);
    painter.line_segment([b, c], stroke);
}

pub fn folder(painter: &Painter, rect: Rect, color: Color32) {
    let tab = Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width() * 0.45, rect.height() * 0.28),
    );
    let body = Rect::from_min_max(
        egui::pos2(rect.min.x, rect.min.y + rect.height() * 0.2),
        rect.max,
    );
    painter.rect_stroke(tab, 1.0, Stroke::new(1.2, color), StrokeKind::Inside);
    painter.rect_stroke(body, 1.5, Stroke::new(1.2, color), StrokeKind::Inside);
}

pub fn file(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.2, color);
    let fold = rect.width() * 0.35;
    let points = [
        rect.left_top(),
        egui::pos2(rect.right() - fold, rect.top()),
        egui::pos2(rect.right(), rect.top() + fold),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    painter.add(egui::Shape::line(points.to_vec(), stroke));
    painter.line_segment(
        [
            egui::pos2(rect.right() - fold, rect.top()),
            egui::pos2(rect.right() - fold, rect.top() + fold),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(rect.right() - fold, rect.top() + fold),
            egui::pos2(rect.right(), rect.top() + fold),
        ],
        stroke,
    );
}

/// Two stacked pages: the "Explorer" glyph for the activity bar.
pub fn explorer(painter: &Painter, rect: Rect, color: Color32) {
    let back = Rect::from_min_size(
        rect.min + egui::vec2(rect.width() * 0.22, 0.0),
        egui::vec2(rect.width() * 0.62, rect.height() * 0.72),
    );
    let front = Rect::from_min_size(
        rect.min + egui::vec2(0.0, rect.height() * 0.24),
        egui::vec2(rect.width() * 0.62, rect.height() * 0.76),
    );
    painter.rect_stroke(back, 1.5, Stroke::new(1.5, color), StrokeKind::Inside);
    painter.rect_filled(front, 1.5, Color32::from_rgb(18, 19, 20));
    painter.rect_stroke(front, 1.5, Stroke::new(1.5, color), StrokeKind::Inside);
}

pub fn close_cross(painter: &Painter, center: Pos2, color: Color32) {
    let stroke = Stroke::new(1.3, color);
    let d = 3.2;
    painter.line_segment(
        [center + egui::vec2(-d, -d), center + egui::vec2(d, d)],
        stroke,
    );
    painter.line_segment(
        [center + egui::vec2(-d, d), center + egui::vec2(d, -d)],
        stroke,
    );
}
