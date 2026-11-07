//! The Forge Room mark and the wireframe room from the installer's artwork, drawn with the
//! painter so they scale cleanly and follow the active theme's accent.

use crate::{fonts, theme};
use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

/// The Forge Room logo centred on `center`.
pub fn logo(painter: &Painter, center: Pos2, radius: f32) {
    crate::mark::draw(painter, center, radius * 2.0);
}

/// Letter-spaced wordmark, like the installer's "F O R G E  R O O M".
pub fn wordmark(painter: &Painter, center: Pos2, size: f32) {
    painter.text(
        center,
        Align2::CENTER_CENTER,
        "F O R G E   R O O M",
        FontId::new(size, fonts::semibold()),
        theme::text(),
    );
}

/// The installer's architectural room with its red-lit doorway. The artwork's own coordinate
/// space is 224 wide; its bottom edge (y = 520) is placed at `bottom_left` and it grows upward.
pub fn room(painter: &Painter, bottom_left: Pos2, scale: f32, opacity: f32) {
    let at = |x: f32, y: f32| {
        Pos2::new(
            bottom_left.x + x * scale,
            bottom_left.y - (520.0 - y) * scale,
        )
    };
    let fade = |value: u8| {
        Color32::from_rgba_unmultiplied(value, value, value + 3, (255.0 * opacity) as u8)
    };
    let line = |points: &[(f32, f32)], color: Color32, width: f32| {
        painter.add(Shape::line(
            points.iter().map(|(x, y)| at(*x, *y)).collect(),
            Stroke::new(width, color),
        ));
    };
    line(
        &[(0.0, 310.0), (47.0, 279.0), (47.0, 405.0), (0.0, 436.0)],
        fade(0x56),
        1.0,
    );
    line(
        &[(0.0, 310.0), (94.0, 342.0), (224.0, 283.0)],
        fade(0x56),
        1.0,
    );
    line(
        &[(47.0, 279.0), (94.0, 294.0), (224.0, 238.0)],
        fade(0x42),
        1.0,
    );
    line(
        &[(94.0, 294.0), (94.0, 417.0), (224.0, 361.0)],
        fade(0x3C),
        1.0,
    );
    line(
        &[
            (117.0, 331.0),
            (194.0, 298.0),
            (194.0, 386.0),
            (117.0, 417.0),
            (117.0, 331.0),
        ],
        fade(0x4B),
        1.0,
    );
    line(
        &[
            (125.0, 336.0),
            (186.0, 310.0),
            (186.0, 382.0),
            (125.0, 407.0),
            (125.0, 336.0),
        ],
        fade(0x33),
        1.0,
    );
    // The lit doorway.
    let accent = theme::accent();
    let door: Vec<Pos2> = [
        (173.0, 325.0),
        (194.0, 317.0),
        (194.0, 386.0),
        (173.0, 394.0),
    ]
    .iter()
    .map(|(x, y)| at(*x, *y))
    .collect();
    painter.add(Shape::convex_polygon(
        door,
        Color32::from_rgba_unmultiplied(
            accent.r(),
            accent.g(),
            accent.b(),
            (150.0 * opacity) as u8,
        ),
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(
                accent.r(),
                accent.g(),
                accent.b(),
                (230.0 * opacity) as u8,
            ),
        ),
    ));
    line(
        &[(0.0, 437.0), (94.0, 417.0), (224.0, 447.0)],
        fade(0x33),
        1.0,
    );
    for y in [447.0, 460.0, 476.0, 493.0, 515.0] {
        line(
            &[(0.0, y), (94.0, 417.0), (224.0, y - 20.0)],
            fade(0x29),
            0.7,
        );
    }
    for x in [0.0, 30.0, 65.0, 110.0, 165.0, 224.0] {
        line(&[(94.0, 417.0), (x, 520.0)], fade(0x29), 0.7);
    }
}

/// The whole welcome artwork inside `rect`: the room low on the left, the mark centred.
pub fn welcome_backdrop(painter: &Painter, rect: Rect) {
    let scale = (rect.height() / 620.0).clamp(0.9, 1.8);
    room(
        painter,
        Pos2::new(rect.left() + 28.0, rect.bottom() - 16.0),
        scale,
        1.0,
    );
}
