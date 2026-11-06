//! Syntax highlighting with the active theme's own token colors (TextMate scope rules),
//! not a fixed built-in palette.

use crate::theme::{self, Theme};
use eframe::egui::{self, text::LayoutJob};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use syntect::highlighting::{
    Color, FontStyle, HighlightIterator, HighlightState, Highlighter, ScopeSelectors,
    StyleModifier, Theme as SyntectTheme, ThemeItem, ThemeSettings,
};
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};
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

/// Above this size syntax highlighting is switched off (see docs/native/EDITOR_PERFORMANCE.md).
pub const HIGHLIGHT_LIMIT_BYTES: usize = 2 * 1024 * 1024;

/// A single line longer than this is drawn plain: one parser call cannot be interrupted, so a
/// pathological line (minified code, a data blob) would otherwise stall the frame.
pub const MAX_HIGHLIGHT_LINE_BYTES: usize = 4000;

/// Highlighting is spread over frames: each call spends at most this long, and lines it has not
/// reached yet are drawn plain until a later frame catches up.
const FRAME_BUDGET: Duration = Duration::from_millis(12);

type LineState = (ParseState, HighlightState);

struct StyledLine {
    text: String,
    /// Parser and highlighter state at the start of this line, so highlighting can resume here.
    before: LineState,
    /// (color, italic, byte length) pieces that cover `text`.
    spans: Vec<(egui::Color32, bool, usize)>,
}

struct Cache {
    theme_key: String,
    extension: String,
    syntect_theme: Arc<SyntectTheme>,
    /// Highlighted leading lines of the text; shorter than the text while catching up.
    lines: Vec<StyledLine>,
    /// State after the last cached line.
    after: Option<LineState>,
    complete: bool,
    text_hash: u64,
}

impl Cache {
    fn new(theme: &Theme, extension: &str) -> Self {
        Self {
            theme_key: theme.key.clone(),
            extension: extension.to_string(),
            syntect_theme: Arc::new(to_syntect(theme)),
            lines: Vec::new(),
            after: None,
            complete: false,
            text_hash: 0,
        }
    }
}

static PENDING: AtomicBool = AtomicBool::new(false);

/// True while the last `highlight` call left lines unhighlighted; the caller should repaint soon.
pub fn pending() -> bool {
    PENDING.load(Ordering::Relaxed)
}

fn cache() -> &'static Mutex<Option<Cache>> {
    static CACHE: OnceLock<Mutex<Option<Cache>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Highlight `text` for a file extension using the active theme. Only lines from the first edit
/// onward are re-parsed, and only until the parser state matches the old one again, so typing in
/// a large file stays cheap. Work per call is time-budgeted; see [`pending`].
pub fn highlight(text: &str, extension: &str, font_id: &egui::FontId) -> LayoutJob {
    let theme = theme::current();
    // A line too long to parse would leave the parser state wrong for every line after it (an
    // opener or closer inside it is never seen), so such a file is shown as plain text.
    if text.len() > HIGHLIGHT_LIMIT_BYTES
        || text
            .split_inclusive('\n')
            .any(|line| line.len() > MAX_HIGHLIGHT_LINE_BYTES)
    {
        // Cache nothing for these files: plain text keeps every keystroke bounded by layout alone.
        PENDING.store(false, Ordering::Relaxed);
        let mut job = LayoutJob::default();
        job.append(
            text,
            0.0,
            egui::TextFormat::simple(font_id.clone(), theme.palette.text),
        );
        return job;
    }
    let mut guard = cache().lock().expect("syntax cache");
    let mut state = match guard.take() {
        Some(state) if state.theme_key == theme.key && state.extension == extension => state,
        _ => Cache::new(&theme, extension),
    };
    let job = update(&mut state, text, font_id, theme.palette.text, FRAME_BUDGET);
    PENDING.store(!state.complete, Ordering::Relaxed);
    *guard = Some(state);
    job
}

fn update(
    cache: &mut Cache,
    text: &str,
    font_id: &egui::FontId,
    plain_color: egui::Color32,
    budget: Duration,
) -> LayoutJob {
    let text_hash = hash(text);
    if !(cache.complete && cache.text_hash == text_hash) {
        refresh(cache, text, budget);
        cache.text_hash = text_hash;
    }

    let mut job = LayoutJob::default();
    let mut offset = 0;
    for line in &cache.lines {
        let mut start = 0;
        for &(color, italic, length) in &line.spans {
            let mut format = egui::TextFormat::simple(font_id.clone(), color);
            format.italics = italic;
            job.append(&line.text[start..start + length], 0.0, format);
            start += length;
        }
        offset += line.text.len();
    }
    if offset < text.len() {
        job.append(
            &text[offset..],
            0.0,
            egui::TextFormat::simple(font_id.clone(), plain_color),
        );
    }
    job
}

fn refresh(cache: &mut Cache, text: &str, budget: Duration) {
    let set = syntaxes();
    let syntax = set
        .find_syntax_by_extension(&cache.extension)
        .unwrap_or_else(|| set.find_syntax_plain_text());
    let theme = cache.syntect_theme.clone();
    let highlighter = Highlighter::new(&theme);
    let plain = theme.settings.foreground.map_or(egui::Color32::GRAY, |c| {
        egui::Color32::from_rgb(c.r, c.g, c.b)
    });

    let new_lines: Vec<&str> = LinesWithEndings::from(text).collect();
    let count = new_lines.len();
    let old_count = cache.lines.len();
    let prefix = (0..old_count.min(count))
        .take_while(|&index| cache.lines[index].text == new_lines[index])
        .count();
    // Lines after the edit can be kept as soon as the parser state lines up with the old one.
    // That is only valid when the old cache covered the whole old text.
    let mut suffix = 0;
    if cache.complete {
        let limit = (old_count - prefix).min(count - prefix);
        while suffix < limit
            && cache.lines[old_count - 1 - suffix].text == new_lines[count - 1 - suffix]
        {
            suffix += 1;
        }
    }

    let old_after = cache.after.take();
    let (mut parse, mut highlight) = if prefix < old_count {
        cache.lines[prefix].before.clone()
    } else {
        old_after.clone().unwrap_or_else(|| {
            (
                ParseState::new(syntax),
                HighlightState::new(&highlighter, ScopeStack::new()),
            )
        })
    };
    let tail = cache.lines.split_off(old_count - suffix);
    cache.lines.truncate(prefix);

    let tail_start = count - suffix;
    let started = Instant::now();
    let mut index = prefix;
    let mut spliced = false;
    while index < count {
        if index >= tail_start {
            let reuse = index - tail_start;
            if tail[reuse].before.0 == parse && tail[reuse].before.1 == highlight {
                cache.lines.extend(tail.into_iter().skip(reuse));
                spliced = true;
                break;
            }
        }
        if started.elapsed() > budget {
            break;
        }
        let line = new_lines[index];
        let before = (parse.clone(), highlight.clone());
        // `highlight` shows files with overlong lines as plain text; this only guards `update`.
        let spans = if line.len() > MAX_HIGHLIGHT_LINE_BYTES {
            vec![(plain, false, line.len())]
        } else {
            match parse.parse_line(line, set) {
                Ok(ops) => HighlightIterator::new(&mut highlight, &ops, line, &highlighter)
                    .map(|(style, piece)| {
                        let color = egui::Color32::from_rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        );
                        (
                            color,
                            style.font_style.contains(FontStyle::ITALIC),
                            piece.len(),
                        )
                    })
                    .collect(),
                Err(_) => vec![(plain, false, line.len())],
            }
        };
        cache.lines.push(StyledLine {
            text: line.to_string(),
            before,
            spans,
        });
        index += 1;
    }
    cache.complete = spliced || index == count;
    cache.after = if spliced {
        old_after
    } else {
        Some((parse, highlight))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_limit_counts_the_newline_like_the_parser() {
        // The parser sees each line with its newline: up to 4,000 bytes stay parsed.
        for body in [3997, 3998, 3999] {
            for end in ["\n", "\r\n"] {
                let first = format!("/*{}{end}", "a".repeat(body));
                let text = format!("{first}let x = 1;{end}*/{end}");
                let job = highlight(&text, "rs", &egui::FontId::monospace(14.0));
                assert_eq!(
                    job.sections.len() == 1,
                    first.len() > MAX_HIGHLIGHT_LINE_BYTES,
                    "first line {} bytes",
                    first.len()
                );
            }
        }
    }

    #[test]
    fn a_file_with_an_overlong_line_is_plain_so_no_state_is_guessed() {
        let text = format!(
            "/*{}\nlet x = 1;\n*/\n",
            "a".repeat(MAX_HIGHLIGHT_LINE_BYTES + 1)
        );
        let job = highlight(&text, "rs", &egui::FontId::monospace(14.0));
        assert_eq!(job.sections.len(), 1);
        assert_eq!(job.text, text);
    }

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

    #[test]
    fn huge_files_are_shown_as_plain_text() {
        let text = "let x = 1;
"
        .repeat(HIGHLIGHT_LIMIT_BYTES / 10);
        let job = highlight(&text, "rs", &egui::FontId::monospace(14.0));
        assert_eq!(job.sections.len(), 1);
        assert!(!pending());
    }

    #[test]
    fn text_without_a_final_newline_is_kept_in_full() {
        for (text, extension) in [
            (
                "hello
my-unsaved-edit",
                "txt",
            ),
            (
                "hello
my-unsaved-edit",
                "txt",
            ),
            (
                "fn main() {}
let last = 1",
                "rs",
            ),
            ("no newline at all", "txt"),
        ] {
            let mut cache = Cache::new(&theme::current(), extension);
            let job = run(&mut cache, text);
            assert_eq!(job.text, text, "{text:?}");
            let shown: String = pieces(&job)
                .into_iter()
                .map(|(piece, _, _)| piece)
                .collect();
            assert_eq!(
                shown, text,
                "sections must cover the whole text for {text:?}"
            );
        }
    }

    #[test]
    fn plain_text_is_drawn_in_the_theme_text_color() {
        let text_color = theme::current().palette.text;
        for text in [
            "hello
my-unsaved-edit",
            "hello
world
",
            "single",
        ] {
            let mut cache = Cache::new(&theme::current(), "txt");
            for (piece, color, _) in pieces(&run(&mut cache, text)) {
                assert_eq!(color, text_color, "{piece:?} in {text:?}");
            }
        }
    }

    #[test]
    fn typing_at_the_end_of_a_file_matches_a_fresh_highlight() {
        for extension in ["txt", "rs", "json"] {
            let mut incremental = Cache::new(&theme::current(), extension);
            let mut text = String::from(
                "hello
",
            );
            run(&mut incremental, &text);
            for ch in "my-unsaved-edit".chars() {
                text.push(ch);
                let got = run(&mut incremental, &text);
                let want = run(&mut Cache::new(&theme::current(), extension), &text);
                assert_eq!(
                    pieces(&got),
                    pieces(&want),
                    "after typing {ch:?} in .{extension}"
                );
            }
        }
    }

    #[test]
    fn a_pathological_line_does_not_stall_the_frame() {
        let line = "let x = 1; ".repeat(50_000);
        let mut cache = Cache::new(&theme::current(), "rs");
        let started = std::time::Instant::now();
        let job = run(&mut cache, &line);
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "took {:?}",
            started.elapsed()
        );
        assert_eq!(job.text, line);
        assert!(cache.complete);

        // Lines around it still highlight, and the file is whole.
        let text = format!(
            "fn a() {{}}
{line}
fn b() {{}}
"
        );
        let job = run(&mut cache, &text);
        assert_eq!(job.text, text);
    }

    fn fresh_cache() -> Cache {
        Cache::new(&theme::current(), "rs")
    }

    fn pieces(job: &LayoutJob) -> Vec<(String, egui::Color32, bool)> {
        job.sections
            .iter()
            .map(|section| {
                (
                    job.text[section.byte_range.start.0..section.byte_range.end.0].to_string(),
                    section.format.color,
                    section.format.italics,
                )
            })
            .collect()
    }

    fn run(cache: &mut Cache, text: &str) -> LayoutJob {
        let font = egui::FontId::monospace(14.0);
        update(
            cache,
            text,
            &font,
            egui::Color32::WHITE,
            Duration::from_secs(60),
        )
    }

    #[test]
    fn incremental_edits_match_a_fresh_highlight() {
        let base: String = (0..300)
            .map(|index| {
                format!(
                    "fn f{index}() {{ let x = {index}; }} // c
"
                )
            })
            .collect();
        let edits = [
            base.replacen("let x = 5;", "let x = 55555;", 1),
            // Opening a block comment changes the state of every later line.
            base.replacen("fn f10()", "/* fn f10()", 1),
            base.replacen("fn f10()", "/* fn f10()", 1)
                .replacen("fn f200()", "*/ fn f200()", 1),
            format!(
                "// new first line
{base}"
            ),
            base.lines()
                .skip(40)
                .map(|line| {
                    format!(
                        "{line}
"
                    )
                })
                .collect(),
            String::new(),
        ];
        let mut incremental = fresh_cache();
        run(&mut incremental, &base);
        for edited in &edits {
            let got = run(&mut incremental, edited);
            let want = run(&mut fresh_cache(), edited);
            assert_eq!(got.text, *edited);
            assert_eq!(pieces(&got), pieces(&want));
            assert!(incremental.complete);
        }
    }

    #[test]
    fn a_tiny_budget_catches_up_over_several_calls() {
        let text: String = (0..2000)
            .map(|index| {
                format!(
                    "let value_{index} = {index};
"
                )
            })
            .collect();
        let font = egui::FontId::monospace(14.0);
        let mut cache = fresh_cache();
        let mut calls = 0;
        loop {
            let job = update(
                &mut cache,
                &text,
                &font,
                egui::Color32::WHITE,
                Duration::from_micros(200),
            );
            assert_eq!(job.text, text, "unhighlighted lines are still shown");
            calls += 1;
            if cache.complete {
                break;
            }
            assert!(calls < 10_000);
        }
        assert!(
            calls > 1,
            "a 200 microsecond budget cannot finish in one call"
        );
        assert_eq!(
            pieces(&run(&mut cache, &text)),
            pieces(&run(&mut fresh_cache(), &text))
        );
    }

    /// Large-file measurement for roadmap item E03. Run with:
    /// `cargo test --release large_file_cost -- --ignored --nocapture`
    #[test]
    #[ignore = "benchmark, prints timings"]
    fn large_file_cost() {
        use std::time::Instant;
        let line = "    let value = compute(index, &items[index]).unwrap_or_default(); // note
";
        let font_id = egui::FontId::monospace(14.0);
        let context = egui::Context::default();
        let forever = Duration::from_secs(600);
        for kib in [100usize, 500, 1024, 5 * 1024, 10 * 1024] {
            let text = line.repeat(kib * 1024 / line.len());
            let mut cache = fresh_cache();
            let cold = Instant::now();
            let job = update(&mut cache, &text, &font_id, egui::Color32::WHITE, forever);
            let cold = cold.elapsed();
            // One keystroke in the middle of the file: re-parse a line, rebuild the job.
            let middle = text.len() / 2;
            let mut edited = text.clone();
            edited.insert(middle - middle % line.len(), 'x');
            let edit = Instant::now();
            let edited_job = update(&mut cache, &edited, &font_id, egui::Color32::WHITE, forever);
            let edit = edit.elapsed();
            let layout = Instant::now();
            let mut rows = 0;
            let _ = context.run_ui(Default::default(), |ui| {
                let galley = ui.fonts_mut(|fonts| fonts.layout_job(edited_job.clone()));
                rows = galley.rows.len();
            });
            let layout = layout.elapsed();
            println!(
                "{kib:>6} KiB {rows:>7} rows  open-highlight {cold:>9.1?}  keystroke-highlight {edit:>9.1?}  layout {layout:>9.1?}  ({} sections)",
                job.sections.len()
            );
        }
    }
}
