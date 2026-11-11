//! Typing helpers for the code editor: keeping the indentation when Enter is pressed, and opening
//! a block (`{`, `(`, `[`) onto its own indented line. The text decisions are plain functions;
//! `smart_enter` connects them to egui's text-edit state.

use eframe::egui::{self, text::CCursor, text::CCursorRange};

/// One level of indentation, in the style the file already uses: a tab when most indented lines
/// start with one, otherwise the smallest space indentation seen (2 or 4), defaulting to 4 spaces.
pub fn indent_unit(text: &str) -> String {
    let (mut tabs, mut spaced) = (0usize, 0usize);
    let mut smallest: Option<usize> = None;
    for line in text.lines().take(2_000) {
        if line.starts_with('\t') {
            tabs += 1;
        } else if line.starts_with(' ') {
            let width = line.len() - line.trim_start_matches(' ').len();
            if !line.trim().is_empty() {
                spaced += 1;
                smallest = Some(smallest.map_or(width, |current| current.min(width)));
            }
        }
    }
    if tabs > spaced {
        "\t".to_string()
    } else {
        " ".repeat(smallest.filter(|width| matches!(width, 2 | 4)).unwrap_or(4))
    }
}

fn closing(opening: char) -> Option<char> {
    match opening {
        '{' => Some('}'),
        '(' => Some(')'),
        '[' => Some(']'),
        _ => None,
    }
}

/// What pressing Enter should insert for the selection `start..end` (byte offsets), and where the
/// cursor goes inside the inserted text (in bytes).
///
/// The new line starts with the current line's indentation. After an opening bracket it is one
/// level deeper, and when the matching closing bracket follows the cursor it is moved down onto
/// its own line, with the cursor left on the indented line between them.
pub fn enter_text(text: &str, start: usize, end: usize, unit: &str) -> (String, usize) {
    let (start, end) = (start.min(text.len()), end.min(text.len()));
    let line_start = text[..start].rfind('\n').map_or(0, |index| index + 1);
    let indent: String = text[line_start..start]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let before = text[..start]
        .trim_end_matches([' ', '\t'])
        .chars()
        .next_back();
    let after = text[end..].chars().next();
    let deeper = before.and_then(closing).is_some();
    let level = if deeper {
        format!("{indent}{unit}")
    } else {
        indent.clone()
    };
    let mut inserted = format!("\n{level}");
    let cursor = inserted.len();
    if deeper && before.and_then(closing) == after {
        inserted.push('\n');
        inserted.push_str(&indent);
    }
    (inserted, cursor)
}

/// Handles an Enter key press for the text edit `id`: replaces the selection with
/// [`enter_text`] and moves the cursor. Returns false (and leaves the key for egui) when the editor
/// is not focused, a key modifier is held, or the cursor state is unknown.
pub fn smart_enter(ui: &egui::Ui, id: egui::Id, text: &mut String) -> bool {
    if !ui.memory(|memory| memory.has_focus(id)) {
        return false;
    }
    // A composing input method uses Enter to confirm; leave it alone.
    if ui.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Ime(_)))
    }) {
        return false;
    }
    let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) else {
        return false;
    };
    let Some(range) = state.cursor.char_range() else {
        return false;
    };
    if !ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
        return false;
    }
    let (first, last) = (
        range.primary.index.0.min(range.secondary.index.0),
        range.primary.index.0.max(range.secondary.index.0),
    );
    let byte = |chars: usize| {
        text.char_indices()
            .nth(chars)
            .map_or(text.len(), |(index, _)| index)
    };
    let (start, end) = (byte(first), byte(last));
    let (inserted, cursor) = enter_text(text, start, end, &indent_unit(text));
    let caret = first + inserted[..cursor].chars().count();
    text.replace_range(start..end, &inserted);
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(caret))));
    state.store(ui.ctx(), id);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enter(text: &str, at: usize) -> (String, usize) {
        let (inserted, cursor) = enter_text(text, at, at, "    ");
        let mut result = text.to_string();
        result.insert_str(at, &inserted);
        (result, at + cursor)
    }

    #[test]
    fn enter_keeps_the_current_indentation() {
        assert_eq!(
            enter("    let x = 1;", 14),
            ("    let x = 1;\n    ".into(), 19)
        );
        assert_eq!(enter("\tx", 2), ("\tx\n\t".into(), 4));
        assert_eq!(enter("plain", 5), ("plain\n".into(), 6));
        assert_eq!(enter("", 0), ("\n".into(), 1));
    }

    #[test]
    fn enter_in_the_middle_of_a_line_splits_it() {
        let (text, cursor) = enter("  ab", 3);
        assert_eq!(text, "  a\n  b");
        assert_eq!(cursor, 6);
    }

    #[test]
    fn enter_after_an_opening_bracket_indents_one_level() {
        assert_eq!(enter("fn f() {", 8), ("fn f() {\n    ".into(), 13));
        assert_eq!(enter("  call(", 7), ("  call(\n      ".into(), 14));
        // Trailing spaces before the cursor do not hide the bracket.
        assert_eq!(enter("if x { ", 7).0, "if x { \n    ");
    }

    #[test]
    fn enter_between_matching_brackets_opens_a_block() {
        let (text, cursor) = enter("  fn f() {}", 10);
        assert_eq!(text, "  fn f() {\n      \n  }");
        assert_eq!(&text[..cursor], "  fn f() {\n      ");
        let (text, _) = enter("[]", 1);
        assert_eq!(text, "[\n    \n]");
        // A different closing bracket is not a pair.
        assert_eq!(enter("(]", 1).0, "(\n    ]");
    }

    #[test]
    fn enter_replaces_a_selection_and_handles_multibyte_text() {
        let (inserted, _) = enter_text("héllo wörld", 6, 12, "    ");
        assert_eq!(inserted, "\n");
        assert_eq!(enter("é{", 3).0, "é{\n    ");
        // Offsets past the end are clamped instead of panicking.
        assert_eq!(enter_text("ab", 9, 9, "    ").0, "\n");
    }

    #[test]
    fn the_indent_unit_follows_the_file() {
        assert_eq!(indent_unit(""), "    ");
        assert_eq!(indent_unit("fn a() {\n  x\n}\n"), "  ");
        assert_eq!(indent_unit("fn a() {\n    x\n        y\n}\n"), "    ");
        assert_eq!(indent_unit("a\n\tb\n\tc\n  d\n"), "\t");
        // An odd indentation width is not taken as the file's style.
        assert_eq!(indent_unit("a\n   b\n"), "    ");
    }

    #[test]
    fn pressing_enter_in_a_focused_editor_inserts_the_indented_line() {
        let context = egui::Context::default();
        let id = egui::Id::unique("editor");
        let mut text = String::from("    if x {}");
        // Frame 1: the editor takes focus and the caret sits between the braces.
        let mut output = context.run_ui(Default::default(), |ui| {
            let output = egui::TextEdit::multiline(&mut text).id(id).show(ui);
            output.response.request_focus();
            let mut state = output.state;
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(10))));
            state.store(ui.ctx(), id);
        });
        output.textures_delta.clear();
        // Frame 2: Enter arrives and is handled before the text edit sees it.
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut handled = false;
        let mut output = context.run_ui(raw, |ui| {
            handled = smart_enter(ui, id, &mut text);
            egui::TextEdit::multiline(&mut text).id(id).show(ui);
        });
        output.textures_delta.clear();
        assert!(handled);
        assert_eq!(text, "    if x {\n        \n    }");
        let state = egui::TextEdit::load_state(&context, id).unwrap();
        // The caret is on the indented middle line: after "    if x {", a newline and 8 spaces.
        assert_eq!(
            state.cursor.char_range().unwrap().primary.index.0,
            10 + 1 + 8
        );
    }
}
