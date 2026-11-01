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

/// A flat 26px icon button: no frame until hovered, dim until hovered.
pub fn icon_button(
    ui: &mut egui::Ui,
    tooltip: &str,
    draw: impl FnOnce(&Painter, Pos2, Color32),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    let hovered = response.hovered();
    if hovered {
        ui.painter()
            .rect_filled(rect, 6.0, crate::theme::overlay(18));
    }
    let color = if hovered {
        crate::theme::text()
    } else {
        crate::theme::text_dim()
    };
    draw(ui.painter(), rect.center(), color);
    response.on_hover_text(tooltip)
}

/// Circular arrow for "Refresh".
pub fn refresh(painter: &Painter, center: Pos2, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let mut points = Vec::new();
    for step in 0..=14 {
        let angle = -0.6 + step as f32 / 14.0 * 5.0;
        points.push(center + egui::vec2(angle.cos() * 6.0, angle.sin() * 6.0));
    }
    painter.add(egui::Shape::line(points, stroke));
    let tip = center + egui::vec2((-0.6_f32).cos() * 6.0, (-0.6_f32).sin() * 6.0);
    painter.line_segment([tip, tip + egui::vec2(-1.0, -4.5)], stroke);
    painter.line_segment([tip, tip + egui::vec2(-4.8, -0.5)], stroke);
}

/// Three dots for "More actions".
pub fn more(painter: &Painter, center: Pos2, color: Color32) {
    for offset in [-6.0, 0.0, 6.0] {
        painter.circle_filled(center + egui::vec2(offset, 0.0), 1.6, color);
    }
}

/// Play triangle for Run/Debug.
pub fn play(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let points = vec![
        c + egui::vec2(-5.0, -8.0),
        c + egui::vec2(7.0, 0.0),
        c + egui::vec2(-5.0, 8.0),
        c + egui::vec2(-5.0, -8.0),
    ];
    painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
}

/// Puzzle-like block for Extensions.
pub fn extensions(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let body = Rect::from_center_size(rect.center() + egui::vec2(0.0, 1.5), egui::vec2(15.0, 15.0));
    painter.rect_stroke(body, 3.0, stroke, StrokeKind::Inside);
    painter.circle_stroke(body.center_top() + egui::vec2(0.0, -1.5), 2.8, stroke);
    painter.circle_stroke(body.right_center() + egui::vec2(1.5, 0.0), 2.8, stroke);
}

/// Four-point sparkle for the AI entry.
pub fn sparkle(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let (r, k) = (9.0, 2.2);
    let points = vec![
        c + egui::vec2(0.0, -r),
        c + egui::vec2(k, -k),
        c + egui::vec2(r, 0.0),
        c + egui::vec2(k, k),
        c + egui::vec2(0.0, r),
        c + egui::vec2(-k, k),
        c + egui::vec2(-r, 0.0),
        c + egui::vec2(-k, -k),
        c + egui::vec2(0.0, -r),
    ];
    painter.add(egui::Shape::line(points, Stroke::new(1.4, color)));
}

/// Terminal window with a prompt for the Terminal entry.
pub fn terminal(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let frame = Rect::from_center_size(rect.center(), egui::vec2(18.0, 15.0));
    painter.rect_stroke(frame, 2.5, stroke, StrokeKind::Inside);
    let c = frame.center();
    painter.line_segment(
        [c + egui::vec2(-4.5, -2.5), c + egui::vec2(-1.5, 0.0)],
        stroke,
    );
    painter.line_segment(
        [c + egui::vec2(-1.5, 0.0), c + egui::vec2(-4.5, 2.5)],
        stroke,
    );
    painter.line_segment([c + egui::vec2(1.0, 3.0), c + egui::vec2(4.5, 3.0)], stroke);
}

/// A plus sign.
pub fn plus(painter: &Painter, center: Pos2, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    painter.line_segment(
        [
            center + egui::vec2(-5.0, 0.0),
            center + egui::vec2(5.0, 0.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(0.0, -5.0),
            center + egui::vec2(0.0, 5.0),
        ],
        stroke,
    );
}

/// A page with a plus, for "New File".
pub fn new_file(painter: &Painter, center: Pos2, color: Color32) {
    file(
        painter,
        Rect::from_center_size(center + egui::vec2(-1.0, 0.0), egui::vec2(12.0, 15.0)),
        color,
    );
    plus(painter, center + egui::vec2(5.0, 5.0), color);
}
