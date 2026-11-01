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
    painter.rect_filled(front, 1.5, crate::theme::bg_activity());
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

/// Git branch glyph (two nodes joined to a third) for the status bar.
pub fn branch(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.2, color);
    let x = rect.left() + 3.0;
    let top = egui::pos2(x, rect.top() + 2.5);
    let bottom = egui::pos2(x, rect.bottom() - 2.5);
    let side = egui::pos2(rect.right() - 3.0, rect.top() + rect.height() * 0.38);
    painter.line_segment([top, bottom], stroke);
    painter.line_segment(
        [side, egui::pos2(x, rect.bottom() - rect.height() * 0.45)],
        stroke,
    );
    for center in [top, bottom, side] {
        painter.circle_stroke(center, 2.0, stroke);
    }
}

/// Two people (head and shoulders) for the Room view.
pub fn room(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let front = egui::pos2(rect.center().x - 3.0, rect.top() + rect.height() * 0.34);
    painter.circle_stroke(front, 4.2, stroke);
    painter.circle_stroke(
        egui::pos2(rect.center().x + 6.5, rect.top() + rect.height() * 0.28),
        3.2,
        Stroke::new(1.3, color),
    );
    let base = rect.bottom() - 1.5;
    let mut shoulders = Vec::new();
    for step in 0..=8 {
        let t = step as f32 / 8.0 * std::f32::consts::PI;
        shoulders.push(egui::pos2(
            rect.center().x - 3.0 - t.cos() * 8.0,
            base - t.sin() * 7.0,
        ));
    }
    painter.add(egui::Shape::line(shoulders, stroke));
}

/// Magnifying glass for the Search view.
pub fn search(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6, color);
    let center = egui::pos2(rect.center().x - 2.0, rect.center().y - 2.0);
    painter.circle_stroke(center, 6.5, stroke);
    painter.line_segment(
        [
            center + egui::vec2(4.8, 4.8),
            center + egui::vec2(10.0, 10.0),
        ],
        Stroke::new(2.0, color),
    );
}

/// Small vertical arrow (push/pull counts), drawn because the font lacks the glyph.
pub fn arrow(painter: &Painter, center: Pos2, up: bool, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let direction = if up { -1.0 } else { 1.0 };
    let tip = center + egui::vec2(0.0, direction * 5.5);
    let tail = center - egui::vec2(0.0, direction * 5.0);
    painter.line_segment([tail, tip], stroke);
    painter.line_segment([tip, tip + egui::vec2(-3.2, -direction * 3.4)], stroke);
    painter.line_segment([tip, tip + egui::vec2(3.2, -direction * 3.4)], stroke);
}
