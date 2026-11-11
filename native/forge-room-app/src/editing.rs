//! Typing helpers for the code editor: keeping the indentation when Enter is pressed, and opening
//! a block (`{`, `(`, `[`) onto its own indented line. The text decisions are plain functions;
//! `smart_enter` connects them to egui's text-edit state.

use eframe::egui::{self, text::CCursor, text::CCursorRange};

/// A byte offset clamped into `text` and moved back to a character boundary, so a stray offset can
/// never panic a slice.
pub(crate) fn boundary(text: &str, offset: usize) -> usize {
    text.floor_char_boundary(offset.min(text.len()))
}

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
    let (start, end) = (boundary(text, start), boundary(text, end));
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

/// The result of indenting or outdenting: replace `text[replace]` with `replacement`, then select
/// `selection` (byte offsets in the new text).
#[derive(Debug, Eq, PartialEq)]
pub struct Edit {
    pub replace: std::ops::Range<usize>,
    pub replacement: String,
    pub selection: (usize, usize),
}

/// Width of one indent level in columns.
fn unit_width(unit: &str) -> usize {
    if unit == "\t" { 4 } else { unit.len().max(1) }
}

/// Tab with no selection inserts spaces up to the next indent stop (or a tab character).
pub fn tab_at_caret(text: &str, caret: usize, unit: &str) -> Edit {
    let caret = boundary(text, caret);
    let line_start = text[..caret].rfind('\n').map_or(0, |index| index + 1);
    let inserted = if unit == "\t" {
        "\t".to_string()
    } else {
        let column = text[line_start..caret].chars().count();
        " ".repeat(unit_width(unit) - column % unit_width(unit))
    };
    let at = caret + inserted.len();
    Edit {
        replace: caret..caret,
        replacement: inserted,
        selection: (at, at),
    }
}

/// Indent (or, with `outdent`, unindent) every line touched by the selection `start..end`. A
/// selection that ends at the very start of a line does not include that line. Blank lines are
/// not indented. Outdenting removes one tab, or up to one level of spaces.
pub fn shift_lines(text: &str, start: usize, end: usize, unit: &str, outdent: bool) -> Edit {
    let (start, end) = (boundary(text, start), boundary(text, end));
    let (start, end) = (start.min(end), start.max(end));
    let first = text[..start].rfind('\n').map_or(0, |index| index + 1);
    // Where the last affected line ends (before its newline).
    let last_pos = if end > start && text[..end].ends_with('\n') {
        end - 1
    } else {
        end
    };
    let last = text[last_pos..]
        .find('\n')
        .map_or(text.len(), |index| last_pos + index);
    let width = unit_width(unit);
    let mut replacement = String::new();
    let (mut new_start, mut new_end) = (start as isize, end as isize);
    let mut position = first;
    for (index, line) in text[first..last].split('\n').enumerate() {
        if index > 0 {
            replacement.push('\n');
        }
        let delta: isize = if outdent {
            let removed = if line.starts_with('\t') {
                1
            } else {
                line.chars().take(width).take_while(|c| *c == ' ').count()
            };
            replacement.push_str(&line[removed..]);
            -(removed as isize)
        } else if line.trim().is_empty() {
            replacement.push_str(line);
            0
        } else {
            replacement.push_str(unit);
            replacement.push_str(line);
            unit.len() as isize
        };
        // Selection ends move with the change made at or before them on their own line.
        let line_end = position + line.len();
        if start >= position && start <= line_end {
            new_start += if outdent {
                delta.max(-((start - position) as isize))
            } else {
                delta
            };
        } else if start > line_end {
            new_start += delta;
        }
        if end >= position && end <= line_end {
            new_end += if outdent {
                delta.max(-((end - position) as isize))
            } else {
                delta
            };
        } else if end > line_end {
            new_end += delta;
        }
        position = line_end + 1;
    }
    Edit {
        replace: first..last,
        replacement,
        selection: (new_start.max(0) as usize, new_end.max(0) as usize),
    }
}

/// A key press the editing helpers may take over: only a plain key counts. Shift and Alt
/// variants (Shift+Enter, Alt+Enter) and anything while an input method is composing are left to
/// egui, which `consume_key` alone would not do because it ignores extra Shift and Alt.
fn plain_key_pending(ui: &egui::Ui, key: egui::Key) -> bool {
    ui.input(|input| {
        !input.modifiers.shift
            && !input.modifiers.alt
            && !input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Ime(_)))
            && input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Key { key: k, pressed: true, .. } if *k == key
                )
            })
    })
}

/// The caret and selection of the text edit `id` as byte offsets, and its stored state.
pub(crate) fn selection_bytes(ui: &egui::Ui, id: egui::Id, text: &str) -> Option<(usize, usize)> {
    let state = egui::TextEdit::load_state(ui.ctx(), id)?;
    let range = state.cursor.char_range()?;
    let to_byte = |chars: usize| {
        text.char_indices()
            .nth(chars)
            .map_or(text.len(), |(index, _)| index)
    };
    let (first, last) = (
        range.primary.index.0.min(range.secondary.index.0),
        range.primary.index.0.max(range.secondary.index.0),
    );
    Some((to_byte(first), to_byte(last)))
}

/// Handles Enter for the text edit `id`: replaces the selection with [`enter_text`] and moves the
/// caret, once for every Enter press that arrived in this frame. Returns false (leaving the key
/// to egui) when the editor is not focused, Shift or Alt is held, or an input method is composing.
pub fn smart_enter(ui: &egui::Ui, id: egui::Id, text: &mut String) -> bool {
    if !ui.memory(|memory| memory.has_focus(id)) || !plain_key_pending(ui, egui::Key::Enter) {
        return false;
    }
    let Some((mut start, mut end)) = selection_bytes(ui, id, text) else {
        return false;
    };
    let presses =
        ui.input_mut(|input| input.count_and_consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    if presses == 0 {
        return false;
    }
    let unit = indent_unit(text);
    for _ in 0..presses {
        let (inserted, cursor) = enter_text(text, start, end, &unit);
        text.replace_range(start..end, &inserted);
        start += cursor;
        end = start;
    }
    place_caret(ui, id, text, start, start);
    true
}

/// Moves the caret (or selection) of the text edit `id` to byte offsets in `text`.
pub(crate) fn place_caret(ui: &egui::Ui, id: egui::Id, text: &str, from: usize, to: usize) {
    let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) else {
        return;
    };
    let chars = |byte: usize| text[..boundary(text, byte)].chars().count();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(chars(from)),
        CCursor::new(chars(to)),
    )));
    state.store(ui.ctx(), id);
}

// ---- Bracket and quote pairs ---------------------------------------------------------------

/// The character that closes `opening`, for the pairs that are completed while typing.
fn pair_of(opening: char) -> Option<char> {
    match opening {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '`' => Some('`'),
        '\'' => Some('\''),
        _ => None,
    }
}

fn is_quote(c: char) -> bool {
    matches!(c, '"' | '`' | '\'')
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a pair may be opened right before `after`: at the end of the text or line, before
/// whitespace, or before a closing character. Typing `(` in front of a word does not add `)`.
fn can_open_before(after: Option<char>) -> bool {
    after.is_none_or(|c| c.is_whitespace() || matches!(c, ')' | ']' | '}' | ',' | ';' | '.' | ':'))
}

/// What typing `typed` does to the selection `start..end` (byte offsets): complete a pair,
/// surround the selection, or step over a closing character that is already there. `None` leaves
/// the character to the text edit.
pub fn typed_char(text: &str, start: usize, end: usize, typed: char) -> Option<Edit> {
    let (start, end) = (boundary(text, start), boundary(text, end));
    let (start, end) = (start.min(end), start.max(end));
    let after = text[end..].chars().next();
    let before = text[..start].chars().next_back();
    let closer = pair_of(typed);

    if start != end {
        // Typing an opening character over a selection wraps it, and keeps it selected.
        let closer = closer?;
        let selected = &text[start..end];
        if selected.contains('\n') && is_quote(typed) {
            return None;
        }
        let replacement = format!("{typed}{selected}{closer}");
        let inner = start + typed.len_utf8();
        return Some(Edit {
            replace: start..end,
            replacement,
            selection: (inner, inner + selected.len()),
        });
    }

    // Typing the closing character that is already next to the caret steps over it.
    if after == Some(typed) && (matches!(typed, ')' | ']' | '}') || is_quote(typed)) {
        let at = end + typed.len_utf8();
        return Some(Edit {
            replace: end..end,
            replacement: String::new(),
            selection: (at, at),
        });
    }

    let closer = closer?;
    if !can_open_before(after) {
        return None;
    }
    if is_quote(typed) {
        // No pair inside a word (an apostrophe in "don't") or after another quote, and none after
        // `&` or `<` where a single quote starts a Rust lifetime.
        let prefix = text[..start].rsplit('\n').next().unwrap_or("");
        if before.is_some_and(|c| is_word(c) || is_quote(c))
            || (typed == '\'' && prefix.ends_with(['&', '<']))
        {
            return None;
        }
    }
    let at = start + typed.len_utf8();
    Some(Edit {
        replace: start..start,
        replacement: format!("{typed}{closer}"),
        selection: (at, at),
    })
}

/// Backspace between an empty pair (`(|)`, `"|"`) removes both characters.
pub fn backspace_in_pair(text: &str, caret: usize) -> Option<Edit> {
    let caret = boundary(text, caret);
    let before = text[..caret].chars().next_back()?;
    let after = text[caret..].chars().next()?;
    if pair_of(before) != Some(after) {
        return None;
    }
    let from = caret - before.len_utf8();
    Some(Edit {
        replace: from..caret + after.len_utf8(),
        replacement: String::new(),
        selection: (from, from),
    })
}

/// Handles typed characters and Backspace for pairs in the text edit `id`. Typed characters are
/// taken from the start of this frame's events only as long as each is handled, so the order of
/// everything else is kept. Returns true when the text changed.
pub fn smart_pairs(ui: &egui::Ui, id: egui::Id, text: &mut String) -> bool {
    if !ui.memory(|memory| memory.has_focus(id)) {
        return false;
    }
    let composing = ui.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Ime(_)))
    });
    if composing || ui.input(|input| input.modifiers.command || input.modifiers.alt) {
        return false;
    }
    let Some((mut start, mut end)) = selection_bytes(ui, id, text) else {
        return false;
    };
    let mut changed = false;
    let mut handled = 0;
    let events = ui.input(|input| input.events.clone());
    for event in &events {
        let edit = match event {
            egui::Event::Text(typed) => {
                let mut chars = typed.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => typed_char(text, start, end, c),
                    _ => None,
                }
            }
            egui::Event::Key {
                key: egui::Key::Backspace,
                pressed: true,
                modifiers,
                ..
            } if *modifiers == egui::Modifiers::NONE && start == end => {
                backspace_in_pair(text, start)
            }
            _ => None,
        };
        let Some(edit) = edit else { break };
        text.replace_range(edit.replace.clone(), &edit.replacement);
        (start, end) = edit.selection;
        changed = true;
        handled += 1;
    }
    if handled == 0 {
        return false;
    }
    // Those events are taken: egui must not type them a second time.
    ui.input_mut(|input| {
        input.events.drain(..handled);
    });
    place_caret(ui, id, text, start, end);
    changed
}

/// Handles Tab (indent) and Shift+Tab (outdent) for the text edit `id`. With a selection spanning
/// lines every selected line moves; otherwise Tab inserts an indent stop at the caret and Shift+Tab
/// unindents the current line. Returns false when nothing was done.
pub fn smart_tab(ui: &egui::Ui, id: egui::Id, text: &mut String) -> bool {
    if !ui.memory(|memory| memory.has_focus(id)) || ui.input(|input| input.modifiers.alt) {
        return false;
    }
    let Some((start, end)) = selection_bytes(ui, id, text) else {
        return false;
    };
    let outdents =
        ui.input_mut(|input| input.count_and_consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
    let indents = if outdents == 0 {
        ui.input_mut(|input| input.count_and_consume_key(egui::Modifiers::NONE, egui::Key::Tab))
    } else {
        0
    };
    if outdents + indents == 0 {
        return false;
    }
    let unit = indent_unit(text);
    let (mut start, mut end) = (start, end);
    for _ in 0..outdents + indents {
        let multi_line = text[start..end].contains('\n');
        let edit = if outdents == 0 && !multi_line && start == end {
            tab_at_caret(text, start, &unit)
        } else {
            shift_lines(text, start, end, &unit, outdents > 0)
        };
        text.replace_range(edit.replace.clone(), &edit.replacement);
        (start, end) = edit.selection;
    }
    place_caret(ui, id, text, start, end);
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

    fn apply(text: &str, edit: &Edit) -> (String, (usize, usize)) {
        let mut result = text.to_string();
        result.replace_range(edit.replace.clone(), &edit.replacement);
        (result, edit.selection)
    }

    #[test]
    fn tab_at_the_caret_goes_to_the_next_indent_stop() {
        let at = |text: &str, caret: usize| apply(text, &tab_at_caret(text, caret, "    ")).0;
        assert_eq!(at("", 0), "    ");
        assert_eq!(at("ab", 2), "ab  ");
        assert_eq!(at("abcd", 4), "abcd    ");
        assert_eq!(at("x\nab", 4), "x\nab  ");
        assert_eq!(apply("ab", &tab_at_caret("ab", 1, "\t")).0, "a\tb");
        assert_eq!(tab_at_caret("ab", 2, "    ").selection, (4, 4));
    }

    #[test]
    fn tab_and_shift_tab_move_every_selected_line() {
        let text = "a\n\nb\nc";
        let (indented, selection) = apply(text, &shift_lines(text, 0, text.len(), "  ", false));
        assert_eq!(indented, "  a\n\n  b\n  c");
        assert_eq!(selection, (2, text.len() + 6));
        let (back, _) = apply(
            &indented,
            &shift_lines(&indented, 0, indented.len(), "  ", true),
        );
        assert_eq!(back, text);
    }

    #[test]
    fn a_selection_ending_at_a_line_start_leaves_that_line_alone() {
        let text = "one\ntwo\nthree";
        let (result, _) = apply(text, &shift_lines(text, 0, 8, "    ", false));
        assert_eq!(result, "    one\n    two\nthree");
    }

    #[test]
    fn outdent_removes_one_tab_or_at_most_one_level_of_spaces() {
        let go = |text: &str| apply(text, &shift_lines(text, 0, text.len(), "    ", true)).0;
        assert_eq!(go("\tx"), "x");
        assert_eq!(go("      x"), "  x");
        assert_eq!(go("  x"), "x");
        assert_eq!(go("x"), "x");
        assert_eq!(go(" \ty"), "\ty");
    }

    #[test]
    fn shifting_keeps_the_caret_with_its_text() {
        // Caret in the middle of the second line.
        let text = "ab\ncdef";
        let (result, (a, b)) = apply(text, &shift_lines(text, 5, 5, "    ", false));
        assert_eq!(result, "ab\n    cdef");
        assert_eq!((a, b), (9, 9));
        // Outdent never moves the caret before the start of its line.
        let text = "  xy";
        let (_, (a, _)) = apply(text, &shift_lines(text, 1, 1, "    ", true));
        assert_eq!(a, 0);
        // Multibyte text and out-of-range offsets do not panic.
        let _ = shift_lines("héllo\nwörld", 2, 9, "    ", false);
        let _ = shift_lines("ab", 40, 50, "    ", true);
    }

    fn typed(text: &str, start: usize, end: usize, c: char) -> Option<(String, (usize, usize))> {
        typed_char(text, start, end, c).map(|edit| apply(text, &edit))
    }

    #[test]
    fn opening_characters_complete_their_pair() {
        assert_eq!(typed("", 0, 0, '('), Some(("()".into(), (1, 1))));
        assert_eq!(typed("f ", 2, 2, '{'), Some(("f {}".into(), (3, 3))));
        assert_eq!(typed("x = ", 4, 4, '"'), Some(("x = \"\"".into(), (5, 5))));
        assert_eq!(typed("a[", 2, 2, '['), Some(("a[[]".into(), (3, 3))));
        // Before a closing character or a comma it still pairs.
        assert_eq!(typed("f()", 2, 2, '('), Some(("f(())".into(), (3, 3))));
        assert_eq!(typed("a,b", 1, 1, '('), Some(("a(),b".into(), (2, 2))));
    }

    #[test]
    fn no_pair_in_front_of_a_word_or_inside_one() {
        assert_eq!(typed("word", 0, 0, '('), None);
        assert_eq!(typed("don", 3, 3, '\''), None);
        // Right after a closing quote there is no new pair either.
        assert_eq!(typed("\"a\"", 3, 3, '"'), None);
        // A quote right after another quote does not nest a third.
        assert_eq!(typed("\"", 1, 1, '"'), None);
        // Rust lifetimes.
        assert_eq!(typed("fn f<", 5, 5, '\''), None);
        assert_eq!(typed("&", 1, 1, '\''), None);
        assert_eq!(typed("x = ", 4, 4, '\''), Some(("x = ''".into(), (5, 5))));
        // Other characters are not ours.
        assert_eq!(typed("", 0, 0, 'a'), None);
        assert_eq!(typed("", 0, 0, ')'), None);
    }

    #[test]
    fn typing_a_closer_steps_over_the_one_already_there() {
        assert_eq!(typed("f()", 2, 2, ')'), Some(("f()".into(), (3, 3))));
        assert_eq!(typed("[x]", 2, 2, ']'), Some(("[x]".into(), (3, 3))));
        assert_eq!(typed("\"ab\"", 3, 3, '"'), Some(("\"ab\"".into(), (4, 4))));
        assert_eq!(typed("f(x", 3, 3, ')'), None);
    }

    #[test]
    fn typing_an_opener_over_a_selection_wraps_it() {
        assert_eq!(typed("ab cd", 3, 5, '('), Some(("ab (cd)".into(), (4, 6))));
        assert_eq!(typed("hi", 0, 2, '"'), Some(("\"hi\"".into(), (1, 3))));
        // A backwards selection gives the same result, and a letter replaces as usual.
        assert_eq!(typed("ab", 2, 0, '{'), Some(("{ab}".into(), (1, 3))));
        assert_eq!(typed("ab", 0, 2, 'x'), None);
        // A quote around several lines is not assumed.
        assert_eq!(typed("a\nb", 0, 3, '"'), None);
        assert_eq!(typed("a\nb", 0, 3, '(').unwrap().0, "(a\nb)");
    }

    #[test]
    fn backspace_inside_an_empty_pair_removes_both() {
        let go = |text: &str, caret: usize| {
            backspace_in_pair(text, caret).map(|edit| apply(text, &edit))
        };
        assert_eq!(go("f()", 2), Some(("f".into(), (1, 1))));
        assert_eq!(go("\"\"", 1), Some(("".into(), (0, 0))));
        assert_eq!(go("{}", 1), Some(("".into(), (0, 0))));
        assert_eq!(go("(x)", 2), None);
        assert_eq!(go("(]", 1), None);
        assert_eq!(go("", 0), None);
        assert_eq!(go("é()", 3), Some(("é".into(), (2, 2))));
    }

    fn type_text(text: &str, from: usize, to: usize, events: Vec<egui::Event>) -> (String, bool) {
        let context = egui::Context::default();
        let id = egui::Id::unique("editor");
        let mut text = text.to_string();
        let mut output = context.run_ui(Default::default(), |ui| {
            let shown = egui::TextEdit::multiline(&mut text).id(id).show(ui);
            shown.response.request_focus();
            let mut state = shown.state;
            state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(from),
                CCursor::new(to),
            )));
            state.store(ui.ctx(), id);
        });
        output.textures_delta.clear();
        let raw = egui::RawInput {
            events,
            ..Default::default()
        };
        let mut changed = false;
        let mut output = context.run_ui(raw, |ui| {
            changed = smart_pairs(ui, id, &mut text);
            // egui must see whatever was not taken.
            egui::TextEdit::multiline(&mut text).id(id).show(ui);
        });
        output.textures_delta.clear();
        (text, changed)
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn typing_through_egui_completes_steps_over_and_deletes_pairs() {
        let text = |s: &str| egui::Event::Text(s.to_string());
        // `(` adds the pair, and egui does not type the character a second time.
        assert_eq!(type_text("f", 1, 1, vec![text("(")]), ("f()".into(), true));
        // `)` steps over.
        assert_eq!(
            type_text("f()", 2, 2, vec![text(")")]),
            ("f()".into(), true)
        );
        // Backspace in an empty pair removes both.
        assert_eq!(
            type_text("f()", 2, 2, vec![key(egui::Key::Backspace)]),
            ("f".into(), true)
        );
        // A plain letter is left to egui.
        assert_eq!(type_text("f", 1, 1, vec![text("x")]), ("fx".into(), false));
        // Order is kept: `x` first, then `(`: only the unhandled prefix stops the handler.
        let (typed, _) = type_text("", 0, 0, vec![text("x"), text("(")]);
        assert_eq!(typed, "x(");
        // An input method in progress is never interfered with.
        let ime = egui::Event::Ime(egui::ImeEvent::Commit("あ".into()));
        assert!(!type_text("f", 1, 1, vec![ime, text("(")]).1);
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

    /// Runs two frames: the first focuses an editor holding `text` with the selection `from..to`
    /// (char indices), the second delivers `keys` before the editor is drawn. Returns the text and
    /// whether the handler took the keys.
    fn press(
        text: &str,
        from: usize,
        to: usize,
        keys: Vec<(egui::Key, egui::Modifiers)>,
        handler: fn(&egui::Ui, egui::Id, &mut String) -> bool,
    ) -> (String, bool) {
        let context = egui::Context::default();
        let id = egui::Id::unique("editor");
        let mut text = text.to_string();
        let mut output = context.run_ui(Default::default(), |ui| {
            let shown = egui::TextEdit::multiline(&mut text).id(id).show(ui);
            shown.response.request_focus();
            let mut state = shown.state;
            state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(from),
                CCursor::new(to),
            )));
            state.store(ui.ctx(), id);
        });
        output.textures_delta.clear();
        let mut raw = egui::RawInput::default();
        for (key, modifiers) in keys {
            raw.events.push(egui::Event::ModifiersChanged(modifiers));
            raw.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            });
        }
        let mut handled = false;
        let mut output = context.run_ui(raw, |ui| {
            handled = handler(ui, id, &mut text);
        });
        output.textures_delta.clear();
        (text, handled)
    }

    #[test]
    fn two_enters_in_one_frame_make_two_lines() {
        let enter = (egui::Key::Enter, egui::Modifiers::NONE);
        let (text, handled) = press("  a", 3, 3, vec![enter, enter], smart_enter);
        assert!(handled);
        assert_eq!(text, "  a\n  \n  ");
    }

    #[test]
    fn shift_and_alt_enter_are_left_to_egui() {
        for modifiers in [egui::Modifiers::SHIFT, egui::Modifiers::ALT] {
            let (text, handled) = press(
                "  a",
                3,
                3,
                vec![(egui::Key::Enter, modifiers)],
                smart_enter,
            );
            assert!(!handled, "{modifiers:?}");
            assert_eq!(text, "  a");
        }
    }

    #[test]
    fn tab_and_shift_tab_work_through_egui_on_a_selection() {
        let (text, handled) = press(
            "a\nb",
            0,
            3,
            vec![(egui::Key::Tab, egui::Modifiers::NONE)],
            smart_tab,
        );
        assert!(handled);
        assert_eq!(text, "    a\n    b");
        let (back, handled) = press(
            &text,
            0,
            text.chars().count(),
            vec![(egui::Key::Tab, egui::Modifiers::SHIFT)],
            smart_tab,
        );
        assert!(handled);
        assert_eq!(back, "a\nb");
        // Tab at a bare caret inserts an indent stop.
        let (text, _) = press(
            "ab",
            2,
            2,
            vec![(egui::Key::Tab, egui::Modifiers::NONE)],
            smart_tab,
        );
        assert_eq!(text, "ab  ");
        // Ctrl+Tab belongs to tab switching.
        let (text, handled) = press(
            "ab",
            2,
            2,
            vec![(egui::Key::Tab, egui::Modifiers::CTRL)],
            smart_tab,
        );
        assert!(!handled);
        assert_eq!(text, "ab");
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
