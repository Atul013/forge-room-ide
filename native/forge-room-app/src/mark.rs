//! The Forge Room logo. Unlike the outline icons it is a filled, multi-color mark, so each
//! SVG path is parsed once (straight segments only), triangulated and painted as a mesh.

use eframe::egui::{Color32, Mesh, Painter, Pos2, Shape, pos2};
use std::sync::OnceLock;

const LOGO: &str = include_str!("../assets/brand/forge-room-logo.svg");
const VIEW: f32 = 256.0;

struct Piece {
    color: Color32,
    triangles: Vec<[Pos2; 3]>,
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    Some(&tag[start..start + tag[start..].find('"')?])
}

fn color(hex: &str) -> Color32 {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Flattens the `M/m/L/l/H/h/V/v/Z` path data used by the logo into one polygon per subpath.
fn polygons(data: &str) -> Vec<Vec<Pos2>> {
    let mut tokens = Vec::new();
    let mut number = String::new();
    for ch in data.chars() {
        if ch.is_ascii_digit() || ch == '.' || (ch == '-' && number.is_empty()) {
            number.push(ch);
            continue;
        }
        if !number.is_empty() {
            tokens.push(std::mem::take(&mut number));
        }
        if ch == '-' {
            number.push(ch);
        } else if ch.is_ascii_alphabetic() {
            tokens.push(ch.to_string());
        }
    }
    if !number.is_empty() {
        tokens.push(number);
    }

    let mut shapes: Vec<Vec<Pos2>> = Vec::new();
    let mut current: Vec<Pos2> = Vec::new();
    let mut at = pos2(0.0, 0.0);
    let mut command = 'M';
    let mut i = 0;
    let value = |i: usize| {
        tokens
            .get(i)
            .and_then(|t| t.parse::<f32>().ok())
            .unwrap_or(0.0)
    };
    while i < tokens.len() {
        if tokens[i].chars().all(|c| c.is_ascii_alphabetic()) {
            command = tokens[i].chars().next().unwrap_or('Z');
            i += 1;
            if command == 'Z' || command == 'z' {
                if current.len() >= 3 {
                    shapes.push(std::mem::take(&mut current));
                }
                current.clear();
                continue;
            }
        }
        let relative = command.is_ascii_lowercase();
        match command.to_ascii_uppercase() {
            'M' | 'L' => {
                let (x, y) = (value(i), value(i + 1));
                at = if relative {
                    at + (x, y).into()
                } else {
                    pos2(x, y)
                };
                i += 2;
                if command.eq_ignore_ascii_case(&'M') {
                    current.clear();
                    command = if relative { 'l' } else { 'L' };
                }
            }
            'H' => {
                at.x = if relative { at.x + value(i) } else { value(i) };
                i += 1;
            }
            'V' => {
                at.y = if relative { at.y + value(i) } else { value(i) };
                i += 1;
            }
            _ => i += 1,
        }
        current.push(at);
    }
    shapes
}

fn cross(a: Pos2, b: Pos2, c: Pos2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Ear clipping for a simple polygon of either winding.
fn triangulate(points: &[Pos2]) -> Vec<[Pos2; 3]> {
    let area: f32 = (0..points.len())
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let mut rest: Vec<Pos2> = points.to_vec();
    if area < 0.0 {
        rest.reverse();
    }
    let mut out = Vec::new();
    let mut guard = rest.len() * rest.len();
    while rest.len() > 3 && guard > 0 {
        guard -= 1;
        let n = rest.len();
        let ear = (0..n).find(|&i| {
            let (a, b, c) = (rest[(i + n - 1) % n], rest[i], rest[(i + 1) % n]);
            cross(a, b, c) > 0.0
                && !rest.iter().any(|&p| {
                    p != a
                        && p != b
                        && p != c
                        && cross(a, b, p) >= 0.0
                        && cross(b, c, p) >= 0.0
                        && cross(c, a, p) >= 0.0
                })
        });
        let Some(i) = ear else { break };
        let n = rest.len();
        out.push([rest[(i + n - 1) % n], rest[i], rest[(i + 1) % n]]);
        rest.remove(i);
    }
    if rest.len() == 3 {
        out.push([rest[0], rest[1], rest[2]]);
    }
    out
}

fn pieces() -> &'static Vec<Piece> {
    static PIECES: OnceLock<Vec<Piece>> = OnceLock::new();
    PIECES.get_or_init(|| {
        LOGO.split("<path")
            .skip(1)
            .filter_map(|tag| {
                let fill = color(attribute(tag, "fill")?);
                let triangles = polygons(attribute(tag, "d")?)
                    .iter()
                    .flat_map(|shape| triangulate(shape))
                    .collect();
                Some(Piece {
                    color: fill,
                    triangles,
                })
            })
            .collect()
    })
}

/// The off-white frame of the logo would vanish on a light theme, so there it takes the ink color.
fn adapt(color: Color32, dark: bool, ink: Color32) -> Color32 {
    if !dark && color == Color32::from_rgb(0xE8, 0xE6, 0xDD) {
        ink
    } else {
        color
    }
}

/// Draws the logo centred on `center`, `size` points tall and wide.
pub fn draw(painter: &Painter, center: Pos2, size: f32) {
    let dark = crate::theme::current().palette.dark;
    let ink = crate::theme::text();
    let scale = size / VIEW;
    let origin = center - eframe::egui::vec2(size, size) / 2.0;
    for piece in pieces() {
        let mut mesh = Mesh::default();
        for triangle in &piece.triangles {
            let base = mesh.vertices.len() as u32;
            for p in triangle {
                mesh.colored_vertex(origin + p.to_vec2() * scale, adapt(piece.color, dark, ink));
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        painter.add(Shape::mesh(mesh));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logo_parses_into_three_filled_pieces_inside_the_box() {
        let pieces = pieces();
        assert_eq!(pieces.len(), 3);
        for piece in pieces {
            assert!(!piece.triangles.is_empty());
            for p in piece.triangles.iter().flatten() {
                assert!((-1.0..=VIEW + 1.0).contains(&p.x) && (-1.0..=VIEW + 1.0).contains(&p.y));
            }
        }
    }

    #[test]
    fn the_light_frame_turns_to_ink_only_on_light_themes() {
        let frame = Color32::from_rgb(0xE8, 0xE6, 0xDD);
        let ink = Color32::from_rgb(20, 20, 20);
        assert_eq!(adapt(frame, true, ink), frame);
        assert_eq!(adapt(frame, false, ink), ink);
        let beam = Color32::from_rgb(0xD9, 0x83, 0x7F);
        assert_eq!(adapt(beam, false, ink), beam);
    }

    #[test]
    fn triangulating_keeps_the_area_of_a_concave_shape() {
        // An L shape with area 3.
        let l = [
            pos2(0.0, 0.0),
            pos2(2.0, 0.0),
            pos2(2.0, 1.0),
            pos2(1.0, 1.0),
            pos2(1.0, 2.0),
            pos2(0.0, 2.0),
        ];
        let area: f32 = triangulate(&l)
            .iter()
            .map(|t| cross(t[0], t[1], t[2]).abs() / 2.0)
            .sum();
        assert!((area - 3.0).abs() < 1e-4);
    }
}
