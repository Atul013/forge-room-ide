//! Syntax highlighting with the active theme's own token colors (TextMate scope rules),
//! not a fixed built-in palette.

use crate::theme::{self, Theme};
use eframe::egui::{self, text::LayoutJob};
use std::sync::{Mutex, OnceLock};
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color, FontStyle, ScopeSelectors, StyleModifier, Theme as SyntectTheme, ThemeItem,
    ThemeSettings,
};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syntect_color(color: egui::Color32) -> Color {
    Color {
        r: color.r(),
        g: color.g(),
        b: color.b(),
        a: 255,
    }
}

fn to_syntect(theme: &Theme) -> SyntectTheme {
    let scopes = theme
        .tokens
        .iter()
        .filter_map(|rule| {
            let selectors = rule.scopes.parse::<ScopeSelectors>().ok()?;
            let mut font_style = FontStyle::empty();
            if rule.italic {
                font_style |= FontStyle::ITALIC;
            }
            if rule.bold {
                font_style |= FontStyle::BOLD;
            }
            Some(ThemeItem {
                scope: selectors,
                style: StyleModifier {
                    foreground: rule.foreground.map(syntect_color),
                    background: None,
                    font_style: Some(font_style),
                },
            })
        })
        .collect();
    SyntectTheme {
        name: Some(theme.name.clone()),
        author: None,
        settings: ThemeSettings {
            foreground: Some(syntect_color(theme.palette.text)),
            ..ThemeSettings::default()
        },
        scopes,
    }
}

type CacheKey = (String, String, u64);

fn cache() -> &'static Mutex<Option<(CacheKey, LayoutJob)>> {
    static CACHE: OnceLock<Mutex<Option<(CacheKey, LayoutJob)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Highlight `text` for a file extension using the active theme. The last result is cached, so
/// calling this every frame only costs a hash of the text while it is unchanged.
pub fn highlight(text: &str, extension: &str, font_id: &egui::FontId) -> LayoutJob {
    let theme = theme::current();
    let key = (theme.key.clone(), extension.to_string(), hash(text));
    if let Some((cached_key, job)) = cache().lock().expect("syntax cache").as_ref()
        && *cached_key == key
    {
        return job.clone();
    }

    let set = syntaxes();
    let syntax = set
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| set.find_syntax_plain_text());
    let syntect_theme = to_syntect(&theme);
    let mut lines = HighlightLines::new(syntax, &syntect_theme);
    let mut job = LayoutJob::default();
    for line in LinesWithEndings::from(text) {
        match lines.highlight_line(line, set) {
            Ok(ranges) => {
                for (style, piece) in ranges {
                    let color = egui::Color32::from_rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );
                    let mut format = egui::TextFormat::simple(font_id.clone(), color);
                    format.italics = style.font_style.contains(FontStyle::ITALIC);
                    job.append(piece, 0.0, format);
                }
            }
            Err(_) => job.append(
                line,
                0.0,
                egui::TextFormat::simple(font_id.clone(), theme.palette.text),
            ),
        }
    }
    *cache().lock().expect("syntax cache") = Some((key, job.clone()));
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_keywords_get_the_theme_keyword_color() {
        let theme = theme::current();
        let job = highlight("fn main() {}\n", "rs", &egui::FontId::monospace(14.0));
        // `fn` is the first section and must use a keyword rule color from the theme.
        let first = job.sections.first().unwrap();
        let keyword_rule = theme
            .tokens
            .iter()
            .find(|rule| rule.scopes.contains("keyword"))
            .and_then(|rule| rule.foreground)
            .unwrap();
        assert_eq!(first.format.color, keyword_rule);
        assert_eq!(job.text, "fn main() {}\n");
    }

    #[test]
    fn unknown_extensions_fall_back_to_plain_text() {
        let job = highlight("hello\n", "zzz", &egui::FontId::monospace(14.0));
        assert_eq!(job.text, "hello\n");
    }
}
