//! Runtime color themes. The six Forge Room Studio themes (carried over from the old Code OSS
//! extension) are plain VS Code color-theme JSON; this module reads the parts the app uses.

use eframe::egui::{self, Color32, FontFamily, FontId, TextStyle};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

pub const DEFAULT_THEME: &str = "forge-room";

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    pub bg_editor: Color32,
    pub bg_sidebar: Color32,
    pub bg_activity: Color32,
    pub title_bar: Color32,
    pub status_bar: Color32,
    pub status_text: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub button: Color32,
    pub button_text: Color32,
    pub badge: Color32,
    pub badge_text: Color32,
    pub input_bg: Color32,
    pub selection: Color32,
    pub line_highlight: Color32,
    pub line_number: Color32,
    pub list_active: Color32,
    pub comment: Color32,
    pub keyword: Color32,
    pub string: Color32,
    pub number: Color32,
    pub function: Color32,
    pub type_color: Color32,
    pub variable: Color32,
}

/// One TextMate scope rule from the theme's `tokenColors`.
#[derive(Clone, Debug)]
pub struct TokenRule {
    pub scopes: String,
    pub foreground: Option<Color32>,
    pub italic: bool,
    pub bold: bool,
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub key: String,
    pub name: String,
    pub mood: String,
    pub description: String,
    pub swatches: Vec<Color32>,
    pub palette: Palette,
    pub tokens: Vec<TokenRule>,
}

#[derive(Deserialize)]
struct CatalogFile {
    themes: Vec<CatalogEntry>,
}

#[derive(Deserialize)]
struct CatalogEntry {
    key: String,
    name: String,
    mood: String,
    description: String,
    colors: Vec<String>,
}

#[derive(Deserialize)]
struct ThemeFile {
    #[serde(rename = "type")]
    kind: String,
    colors: HashMap<String, String>,
    #[serde(rename = "tokenColors", default)]
    token_colors: Vec<TokenColor>,
}

#[derive(Deserialize)]
struct TokenColor {
    scope: ScopeField,
    settings: TokenSettings,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ScopeField {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize)]
struct TokenSettings {
    foreground: Option<String>,
    #[serde(rename = "fontStyle")]
    font_style: Option<String>,
}

const CATALOG: &str = include_str!("../themes/catalog.json");
const FILES: [(&str, &str); 7] = [
    ("forge-room", include_str!("../themes/forge-room.json")),
    ("neon-ronin", include_str!("../themes/neon-ronin.json")),
    (
        "starship-deck",
        include_str!("../themes/starship-deck.json"),
    ),
    ("moss-shrine", include_str!("../themes/moss-shrine.json")),
    ("ink-ember", include_str!("../themes/ink-ember.json")),
    ("paper-studio", include_str!("../themes/paper-studio.json")),
    ("signal-guard", include_str!("../themes/signal-guard.json")),
];

/// `#rgb`, `#rrggbb` or `#rrggbbaa`.
pub fn parse_hex(text: &str) -> Option<Color32> {
    let hex = text.trim().strip_prefix('#')?;
    let byte = |index: usize| u8::from_str_radix(hex.get(index..index + 2)?, 16).ok();
    match hex.len() {
        3 => {
            let digit = |index: usize| u8::from_str_radix(hex.get(index..index + 1)?, 16).ok();
            Some(Color32::from_rgb(
                digit(0)? * 17,
                digit(1)? * 17,
                digit(2)? * 17,
            ))
        }
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            byte(6)?,
        )),
        _ => None,
    }
}

pub fn mix(from: Color32, to: Color32, amount: f32) -> Color32 {
    let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount).round() as u8;
    Color32::from_rgb(
        lerp(from.r(), to.r()),
        lerp(from.g(), to.g()),
        lerp(from.b(), to.b()),
    )
}

fn build(entry: &CatalogEntry, file: &ThemeFile) -> Theme {
    let dark = file.kind != "light";
    let colors = &file.colors;
    let get = |key: &str| colors.get(key).and_then(|value| parse_hex(value));
    let bg_editor = get("editor.background").unwrap_or(Color32::from_rgb(21, 23, 22));
    let text = get("foreground")
        .or_else(|| get("editor.foreground"))
        .unwrap_or(Color32::from_rgb(230, 230, 225));
    let text_dim = get("descriptionForeground").unwrap_or_else(|| mix(text, bg_editor, 0.4));
    let bg_sidebar = get("sideBar.background").unwrap_or(bg_editor);
    let bg_activity = get("activityBar.background").unwrap_or(bg_sidebar);
    let accent = get("focusBorder")
        .or_else(|| get("activityBar.foreground"))
        .unwrap_or(Color32::from_rgb(199, 76, 69));

    // The first token rule whose scope list mentions `scope` supplies that token's color.
    let token_color = |scope: &str, fallback: Color32| {
        file.token_colors
            .iter()
            .find(|rule| match &rule.scope {
                ScopeField::One(one) => one.contains(scope),
                ScopeField::Many(many) => many.iter().any(|item| item.contains(scope)),
            })
            .and_then(|rule| rule.settings.foreground.as_deref())
            .and_then(parse_hex)
            .unwrap_or(fallback)
    };

    let palette = Palette {
        dark,
        bg_editor,
        bg_sidebar,
        bg_activity,
        title_bar: get("titleBar.activeBackground").unwrap_or(bg_activity),
        status_bar: get("statusBar.background").unwrap_or(bg_activity),
        status_text: get("statusBar.foreground").unwrap_or(text),
        border: get("panel.border")
            .or_else(|| get("tab.border"))
            .unwrap_or_else(|| mix(bg_editor, text, 0.12)),
        text,
        text_dim,
        accent,
        button: get("button.background").unwrap_or(accent),
        button_text: get("button.foreground").unwrap_or(Color32::WHITE),
        badge: get("activityBarBadge.background").unwrap_or(accent),
        badge_text: get("activityBarBadge.foreground").unwrap_or(Color32::WHITE),
        input_bg: get("input.background").unwrap_or_else(|| mix(bg_editor, text, 0.06)),
        selection: get("editor.selectionBackground").unwrap_or_else(|| {
            Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 90)
        }),
        line_highlight: get("editor.lineHighlightBackground")
            .unwrap_or_else(|| Color32::from_rgba_unmultiplied(255, 255, 255, 9)),
        line_number: get("editorLineNumber.foreground").unwrap_or(text_dim),
        list_active: get("list.activeSelectionBackground")
            .unwrap_or_else(|| mix(bg_sidebar, accent, 0.25)),
        comment: token_color("comment", text_dim),
        keyword: token_color("keyword", accent),
        string: token_color("string", text),
        number: token_color("constant.numeric", text),
        function: token_color("entity.name.function", text),
        type_color: token_color("entity.name.type", text),
        variable: token_color("variable", text),
    };

    let tokens = file
        .token_colors
        .iter()
        .map(|rule| {
            let scopes = match &rule.scope {
                ScopeField::One(one) => one.clone(),
                ScopeField::Many(many) => many.join(", "),
            };
            let style = rule.settings.font_style.as_deref().unwrap_or("");
            TokenRule {
                scopes,
                foreground: rule.settings.foreground.as_deref().and_then(parse_hex),
                italic: style.contains("italic"),
                bold: style.contains("bold"),
            }
        })
        .collect();

    Theme {
        key: entry.key.clone(),
        name: entry.name.clone(),
        mood: entry.mood.clone(),
        description: entry.description.clone(),
        swatches: entry.colors.iter().filter_map(|c| parse_hex(c)).collect(),
        palette,
        tokens,
    }
}

/// All bundled themes, in catalog order.
pub fn all() -> &'static [Arc<Theme>] {
    static THEMES: OnceLock<Vec<Arc<Theme>>> = OnceLock::new();
    THEMES.get_or_init(|| {
        let catalog: CatalogFile = serde_json::from_str(CATALOG).expect("bundled theme catalog");
        catalog
            .themes
            .iter()
            .filter_map(|entry| {
                let json = FILES.iter().find(|(key, _)| *key == entry.key)?.1;
                let file: ThemeFile = serde_json::from_str(json).ok()?;
                Some(Arc::new(build(entry, &file)))
            })
            .collect()
    })
}

pub fn find(key: &str) -> Option<Arc<Theme>> {
    all().iter().find(|theme| theme.key == key).cloned()
}

fn slot() -> &'static RwLock<Arc<Theme>> {
    static CURRENT: OnceLock<RwLock<Arc<Theme>>> = OnceLock::new();
    CURRENT.get_or_init(|| RwLock::new(find(DEFAULT_THEME).unwrap_or_else(|| all()[0].clone())))
}

pub fn current() -> Arc<Theme> {
    slot().read().expect("theme lock").clone()
}

/// Switch the active theme and restyle egui. Unknown keys are ignored.
pub fn set(context: &egui::Context, key: &str) {
    if let Some(theme) = find(key) {
        *slot().write().expect("theme lock") = theme;
        apply(context);
    }
}

pub fn apply(context: &egui::Context) {
    let p = current().palette;
    let mut visuals = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = p.bg_editor;
    visuals.window_fill = p.bg_sidebar;
    visuals.extreme_bg_color = p.input_bg;
    visuals.faint_bg_color = mix(p.bg_editor, p.text, 0.04);
    visuals.window_stroke = egui::Stroke::new(1.0, p.border);
    visuals.override_text_color = Some(p.text);
    visuals.selection.bg_fill = p.selection;
    visuals.selection.stroke = egui::Stroke::new(1.0, p.text);
    visuals.hyperlink_color = p.accent;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
    visuals.widgets.inactive.weak_bg_fill = mix(p.bg_sidebar, p.text, 0.08);
    visuals.widgets.inactive.bg_fill = mix(p.bg_sidebar, p.text, 0.08);
    visuals.widgets.hovered.weak_bg_fill = mix(p.bg_sidebar, p.text, 0.15);
    visuals.widgets.hovered.bg_fill = mix(p.bg_sidebar, p.text, 0.15);
    visuals.widgets.active.weak_bg_fill = p.button;
    visuals.widgets.active.bg_fill = p.button;
    // Installer-style controls: quiet fills, 1px hairlines, generous rounding.
    let rounding = egui::CornerRadius::same(6);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = rounding;
    }
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, mix(p.bg_sidebar, p.text, 0.10));
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, mix(p.bg_sidebar, p.text, 0.28));
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, p.accent);
    visuals.window_corner_radius = egui::CornerRadius::same(10);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);
    context.set_visuals(visuals);

    context.all_styles_mut(|style| {
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(14.5, FontFamily::Monospace),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(22.0, crate::fonts::semibold()),
        );
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
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

/// Secondary text on the status bar: the status text color eased toward the bar color.
pub fn status_dim() -> Color32 {
    let p = current().palette;
    mix(p.status_bar, p.status_text, 0.72)
}

/// A highlight/hover wash that works on dark and light themes.
pub fn overlay(alpha: u8) -> Color32 {
    if current().palette.dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, alpha)
    }
}

macro_rules! accessors {
    ($($name:ident),* $(,)?) => {
        $(pub fn $name() -> Color32 { current().palette.$name })*
    };
}

accessors!(
    bg_editor,
    bg_sidebar,
    bg_activity,
    title_bar,
    status_bar,
    status_text,
    border,
    text,
    text_dim,
    accent,
    button,
    button_text,
    badge,
    badge_text,
    list_active,
    line_highlight,
    line_number,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colors() {
        assert_eq!(parse_hex("#fff"), Some(Color32::from_rgb(255, 255, 255)));
        assert_eq!(parse_hex("#102030"), Some(Color32::from_rgb(16, 32, 48)));
        assert_eq!(
            parse_hex("#80603b88"),
            Some(Color32::from_rgba_unmultiplied(128, 96, 59, 136))
        );
        assert_eq!(parse_hex("nonsense"), None);
        assert_eq!(parse_hex("#12345"), None);
    }

    #[test]
    fn all_bundled_themes_load_with_syntax_rules() {
        let themes = all();
        assert_eq!(themes.len(), 7);
        let keys: Vec<&str> = themes.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "forge-room",
                "neon-ronin",
                "starship-deck",
                "moss-shrine",
                "ink-ember",
                "paper-studio",
                "signal-guard"
            ]
        );
        for theme in themes {
            assert!(theme.tokens.len() >= 7, "{} token rules", theme.key);
            assert!(theme.swatches.len() >= 3, "{} swatches", theme.key);
        }
        assert!(!find("paper-studio").unwrap().palette.dark);
        assert!(find("ink-ember").unwrap().palette.dark);
        assert!(find("missing").is_none());
    }

    #[test]
    fn editor_text_stays_readable_on_every_theme() {
        // Studio promised 4.5:1 for core text; check it survived the port.
        for theme in all() {
            let p = theme.palette;
            assert!(
                contrast(p.text, p.bg_editor) >= 4.5,
                "{} text/editor contrast",
                theme.key
            );
            assert!(
                contrast(p.keyword, p.bg_editor) >= 3.0,
                "{} keyword/editor contrast",
                theme.key
            );
        }
    }

    fn luminance(color: Color32) -> f32 {
        let channel = |value: u8| {
            let v = value as f32 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }
}
