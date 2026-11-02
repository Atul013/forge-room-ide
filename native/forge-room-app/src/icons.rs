//! The icon set. The SVGs in `assets/icons` (outline, 24x24 box, 1.75 px stroke) are compiled
//! into the app; their paths are flattened once and stroked with the painter, so the icons
//! scale and recolor like text and need no image or icon font.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke};
use kurbo::{BezPath, PathEl};
use std::collections::HashMap;
use std::sync::OnceLock;

/// One flattened subpath in the 24x24 icon box.
struct Line {
    points: Vec<Pos2>,
}

const SOURCES: [(&str, &str); 64] = [
    (
        "accessibility-off",
        include_str!("../assets/icons/accessibility-off.svg"),
    ),
    ("ai", include_str!("../assets/icons/ai.svg")),
    ("arrow-down", include_str!("../assets/icons/arrow-down.svg")),
    (
        "arrow-right",
        include_str!("../assets/icons/arrow-right.svg"),
    ),
    ("arrow-up", include_str!("../assets/icons/arrow-up.svg")),
    ("bell", include_str!("../assets/icons/bell.svg")),
    ("bot", include_str!("../assets/icons/bot.svg")),
    ("calendar", include_str!("../assets/icons/calendar.svg")),
    ("check", include_str!("../assets/icons/check.svg")),
    (
        "chevron-down",
        include_str!("../assets/icons/chevron-down.svg"),
    ),
    (
        "chevron-right",
        include_str!("../assets/icons/chevron-right.svg"),
    ),
    ("circle", include_str!("../assets/icons/circle.svg")),
    (
        "circle-check",
        include_str!("../assets/icons/circle-check.svg"),
    ),
    ("circle-dot", include_str!("../assets/icons/circle-dot.svg")),
    ("clock", include_str!("../assets/icons/clock.svg")),
    ("close", include_str!("../assets/icons/close.svg")),
    ("cloud", include_str!("../assets/icons/cloud.svg")),
    ("code", include_str!("../assets/icons/code.svg")),
    (
        "commit-dropdown",
        include_str!("../assets/icons/commit-dropdown.svg"),
    ),
    ("explorer", include_str!("../assets/icons/explorer.svg")),
    ("extensions", include_str!("../assets/icons/extensions.svg")),
    (
        "external-link",
        include_str!("../assets/icons/external-link.svg"),
    ),
    ("eye", include_str!("../assets/icons/eye.svg")),
    ("file", include_str!("../assets/icons/file.svg")),
    ("file-json", include_str!("../assets/icons/file-json.svg")),
    ("file-ts", include_str!("../assets/icons/file-ts.svg")),
    ("flag", include_str!("../assets/icons/flag.svg")),
    (
        "folder-open",
        include_str!("../assets/icons/folder-open.svg"),
    ),
    ("github", include_str!("../assets/icons/github.svg")),
    ("hourglass", include_str!("../assets/icons/hourglass.svg")),
    ("layers", include_str!("../assets/icons/layers.svg")),
    ("lock", include_str!("../assets/icons/lock.svg")),
    ("models", include_str!("../assets/icons/models.svg")),
    ("more", include_str!("../assets/icons/more.svg")),
    ("new-file", include_str!("../assets/icons/new-file.svg")),
    (
        "panel-bottom",
        include_str!("../assets/icons/panel-bottom.svg"),
    ),
    ("panel-left", include_str!("../assets/icons/panel-left.svg")),
    (
        "panel-right",
        include_str!("../assets/icons/panel-right.svg"),
    ),
    ("plus", include_str!("../assets/icons/plus.svg")),
    ("pulse", include_str!("../assets/icons/pulse.svg")),
    ("refresh", include_str!("../assets/icons/refresh.svg")),
    ("rocket", include_str!("../assets/icons/rocket.svg")),
    ("room", include_str!("../assets/icons/room.svg")),
    (
        "room-active",
        include_str!("../assets/icons/room-active.svg"),
    ),
    ("run-debug", include_str!("../assets/icons/run-debug.svg")),
    ("search", include_str!("../assets/icons/search.svg")),
    ("send", include_str!("../assets/icons/send.svg")),
    ("settings", include_str!("../assets/icons/settings.svg")),
    ("share", include_str!("../assets/icons/share.svg")),
    ("shield", include_str!("../assets/icons/shield.svg")),
    (
        "source-control",
        include_str!("../assets/icons/source-control.svg"),
    ),
    ("studio", include_str!("../assets/icons/studio.svg")),
    ("tag", include_str!("../assets/icons/tag.svg")),
    ("target", include_str!("../assets/icons/target.svg")),
    ("terminal", include_str!("../assets/icons/terminal.svg")),
    ("tests", include_str!("../assets/icons/tests.svg")),
    ("upload-box", include_str!("../assets/icons/upload-box.svg")),
    ("user", include_str!("../assets/icons/user.svg")),
    ("user-plus", include_str!("../assets/icons/user-plus.svg")),
    ("users", include_str!("../assets/icons/users.svg")),
    ("warning", include_str!("../assets/icons/warning.svg")),
    (
        "window-maximize",
        include_str!("../assets/icons/window-maximize.svg"),
    ),
    (
        "window-minimize",
        include_str!("../assets/icons/window-minimize.svg"),
    ),
    ("workflows", include_str!("../assets/icons/workflows.svg")),
];

/// Every `d="..."` attribute in an SVG, in order.
fn path_data(svg: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find(" d=\"") {
        rest = &rest[start + 4..];
        let Some(end) = rest.find('"') else {
            break;
        };
        found.push(&rest[..end]);
        rest = &rest[end..];
    }
    found
}

fn flatten(data: &str) -> Vec<Line> {
    let Ok(path) = BezPath::from_svg(data) else {
        return Vec::new();
    };
    let mut lines: Vec<Line> = Vec::new();
    let mut current: Vec<Pos2> = Vec::new();
    let flush = |current: &mut Vec<Pos2>, lines: &mut Vec<Line>| {
        if current.len() > 1 {
            lines.push(Line {
                points: std::mem::take(current),
            });
        } else {
            current.clear();
        }
    };
    kurbo::flatten(path, 0.04, |element| match element {
        PathEl::MoveTo(p) => {
            flush(&mut current, &mut lines);
            current.push(egui::pos2(p.x as f32, p.y as f32));
        }
        PathEl::LineTo(p) => current.push(egui::pos2(p.x as f32, p.y as f32)),
        PathEl::ClosePath => {
            if let Some(first) = current.first().copied() {
                current.push(first);
            }
            flush(&mut current, &mut lines);
        }
        PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
    });
    flush(&mut current, &mut lines);
    lines
}

fn icons() -> &'static HashMap<&'static str, Vec<Line>> {
    static ICONS: OnceLock<HashMap<&'static str, Vec<Line>>> = OnceLock::new();
    ICONS.get_or_init(|| {
        SOURCES
            .iter()
            .map(|(name, svg)| {
                let lines = path_data(svg).into_iter().flat_map(flatten).collect();
                (*name, lines)
            })
            .collect()
    })
}

/// Draw the named icon centred on `center`, `size` points wide (the 24 unit box).
pub fn draw(painter: &Painter, name: &str, center: Pos2, size: f32, color: Color32) {
    let Some(lines) = icons().get(name) else {
        return;
    };
    let scale = size / 24.0;
    let origin = center - egui::vec2(12.0, 12.0) * scale;
    let stroke = Stroke::new((1.75 * scale).max(1.0), color);
    for line in lines {
        let points: Vec<Pos2> = line
            .points
            .iter()
            .map(|p| origin + p.to_vec2() * scale)
            .collect();
        painter.add(egui::Shape::line(points, stroke));
    }
}

fn boxed(painter: &Painter, name: &str, rect: Rect, factor: f32, color: Color32) {
    draw(
        painter,
        name,
        rect.center(),
        rect.width().max(rect.height()) * factor,
        color,
    );
}

pub fn chevron(painter: &Painter, center: Pos2, open: bool, color: Color32) {
    let name = if open {
        "chevron-down"
    } else {
        "chevron-right"
    };
    draw(painter, name, center, 16.0, color);
}

pub fn folder(painter: &Painter, rect: Rect, color: Color32) {
    boxed(painter, "explorer", rect, 1.35, color);
}

pub fn file(painter: &Painter, rect: Rect, color: Color32) {
    boxed(painter, "file", rect, 1.35, color);
}

pub fn close_cross(painter: &Painter, center: Pos2, color: Color32) {
    draw(painter, "close", center, 16.0, color);
}

/// Git branch glyph.
pub fn branch(painter: &Painter, rect: Rect, color: Color32) {
    boxed(painter, "source-control", rect, 1.1, color);
}

/// Two people (head and shoulders) for the Room view.
pub fn room(painter: &Painter, rect: Rect, color: Color32) {
    boxed(painter, "room", rect, 1.1, color);
}

pub fn search(painter: &Painter, rect: Rect, color: Color32) {
    boxed(painter, "search", rect, 1.1, color);
}

/// Vertical arrow (push/pull counts).
pub fn arrow(painter: &Painter, center: Pos2, up: bool, color: Color32) {
    draw(
        painter,
        if up { "arrow-up" } else { "arrow-down" },
        center,
        16.0,
        color,
    );
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
    draw(painter, "refresh", center, 18.0, color);
}

/// Three dots for "More actions".
pub fn more(painter: &Painter, center: Pos2, color: Color32) {
    draw(painter, "more", center, 18.0, color);
}

/// Play triangle for Run/Debug.
/// Four-point sparkle for the AI entry.
pub fn sparkle(painter: &Painter, rect: Rect, color: Color32) {
    boxed(painter, "ai", rect, 1.1, color);
}

/// Terminal window with a prompt for the Terminal entry.
pub fn plus(painter: &Painter, center: Pos2, color: Color32) {
    draw(painter, "plus", center, 16.0, color);
}

/// A page with a plus, for "New File".
pub fn new_file(painter: &Painter, center: Pos2, color: Color32) {
    draw(painter, "new-file", center, 18.0, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_flattens_to_lines_inside_the_box() {
        for (name, _) in SOURCES {
            let lines = icons().get(name).expect(name);
            assert!(!lines.is_empty(), "{name} produced no lines");
            for line in lines {
                for p in &line.points {
                    assert!(
                        (-1.0..=25.0).contains(&p.x) && (-1.0..=25.0).contains(&p.y),
                        "{name} point {p:?} is outside the 24x24 box"
                    );
                }
            }
        }
    }
}
