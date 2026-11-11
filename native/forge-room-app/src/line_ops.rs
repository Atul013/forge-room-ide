//! Whole-line editing commands: toggle comment (Ctrl+/), move lines (Alt+Up/Down), duplicate lines
//! (Shift+Alt+Up/Down) and delete lines (Ctrl+Shift+K). The text rules are plain functions that
//! return an [`Edit`]; `handle` connects the key chords to egui's text-edit state.

use crate::editing::{self, Edit};
use eframe::egui;

/// The first and last byte of the lines a selection touches (the end is before that line's
/// newline). A selection that ends at the very start of a line does not include that line.
fn line_span(text: &str, start: usize, end: usize) -> (usize, usize) {
    let (start, end) = (editing::boundary(text, start), editing::boundary(text, end));
    let (start, end) = (start.min(end), start.max(end));
    let first = text[..start].rfind('\n').map_or(0, |index| index + 1);
    let anchor = if end > start && text[..end].ends_with('\n') {
        end - 1
    } else {
        end
    };
    let last = text[anchor..]
        .find('\n')
        .map_or(text.len(), |index| anchor + index);
    (first, last.max(first))
}

/// The line comment marker for a file extension; `None` for formats without one.
pub fn comment_prefix(extension: &str) -> Option<&'static str> {
    Some(match extension.to_ascii_lowercase().as_str() {
        "rs" | "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "c" | "h" | "cpp" | "hpp" | "cc"
        | "cs" | "java" | "go" | "kt" | "swift" | "dart" | "scss" | "php" | "jsonc" => "//",
        "py" | "sh" | "bash" | "zsh" | "yml" | "yaml" | "toml" | "rb" | "ps1" | "r" | "pl"
        | "conf" | "ini" | "env" | "gitignore" | "dockerfile" | "tf" => "#",
        "sql" | "lua" | "hs" => "--",
        "bat" | "cmd" => "REM",
        _ => return None,
    })
}

/// The length of the comment marker at the start of `rest`, when there is one. A marker made of
/// letters (`REM`) must be a whole word and may be written in any case; symbol markers match as
/// they are.
fn marker_len(rest: &str, prefix: &str) -> Option<usize> {
    let head = rest.get(..prefix.len())?;
    if prefix.chars().all(char::is_alphabetic) {
        let whole_word = rest[prefix.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        (head.eq_ignore_ascii_case(prefix) && whole_word).then_some(prefix.len())
    } else {
        (head == prefix).then_some(prefix.len())
    }
}

/// Comment every selected line with `prefix`, or remove the comments when all of them already
/// have one. The marker goes at the shallowest indentation so the block stays aligned; blank lines
/// are left alone. The selection follows its text.
pub fn toggle_comment(text: &str, start: usize, end: usize, prefix: &str) -> Edit {
    let (first, last) = line_span(text, start, end);
    let lines: Vec<&str> = text[first..last].split('\n').collect();
    let indent_of = |line: &str| line.len() - line.trim_start_matches([' ', '\t']).len();
    let content: Vec<&&str> = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let unchanged = Edit {
        replace: first..last,
        replacement: text[first..last].to_string(),
        selection: (start.min(end), start.max(end)),
    };
    if content.is_empty() {
        return unchanged;
    }
    let remove = content
        .iter()
        .all(|line| marker_len(&line[indent_of(line)..], prefix).is_some());
    let column = content
        .iter()
        .map(|line| indent_of(line))
        .min()
        .unwrap_or(0);

    let mut replacement = String::new();
    // (old line start, old line length, byte column of the change, signed size of the change)
    let mut changes: Vec<(usize, usize, usize, isize)> = Vec::new();
    let mut position = first;
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            replacement.push('\n');
        }
        let (at, delta) = if line.trim().is_empty() {
            replacement.push_str(line);
            (0, 0)
        } else if remove {
            let at = indent_of(line);
            let rest = &line[at..];
            let marker = marker_len(rest, prefix).unwrap_or(0)
                + usize::from(rest[marker_len(rest, prefix).unwrap_or(0)..].starts_with(' '));
            replacement.push_str(&line[..at]);
            replacement.push_str(&rest[marker..]);
            (at, -(marker as isize))
        } else {
            let (before, after) = line.split_at(column);
            replacement.push_str(before);
            replacement.push_str(prefix);
            replacement.push(' ');
            replacement.push_str(after);
            (column, prefix.len() as isize + 1)
        };
        changes.push((position, line.len(), at, delta));
        position += line.len() + 1;
    }
    let map = |offset: usize| -> usize {
        let mut result = offset as isize;
        for &(line_start, line_len, at, delta) in &changes {
            if offset > line_start + line_len {
                result += delta;
            } else if offset >= line_start {
                let column = offset - line_start;
                if delta >= 0 {
                    if column >= at {
                        result += delta;
                    }
                } else if column >= at + (-delta) as usize {
                    result += delta;
                } else if column > at {
                    result -= (column - at) as isize;
                }
                break;
            } else {
                break;
            }
        }
        result.max(0) as usize
    };
    let (a, b) = (start.min(end), start.max(end));
    Edit {
        replace: first..last,
        replacement,
        selection: (map(a), map(b)),
    }
}

/// Move the selected lines one line up or down. `None` at the first or last line.
pub fn move_lines(text: &str, start: usize, end: usize, up: bool) -> Option<Edit> {
    let (first, last) = line_span(text, start, end);
    let (a, b) = (start.min(end), start.max(end));
    let block = &text[first..last];
    if up {
        if first == 0 {
            return None;
        }
        let previous_start = text[..first - 1].rfind('\n').map_or(0, |index| index + 1);
        let previous = &text[previous_start..first - 1];
        let shift = previous.len() + 1;
        Some(Edit {
            replace: previous_start..last,
            replacement: format!("{block}\n{previous}"),
            selection: (a - shift, b - shift),
        })
    } else {
        if last >= text.len() {
            return None;
        }
        let next_start = last + 1;
        let next_end = text[next_start..]
            .find('\n')
            .map_or(text.len(), |index| next_start + index);
        let next = &text[next_start..next_end];
        let shift = next.len() + 1;
        Some(Edit {
            replace: first..next_end,
            replacement: format!("{next}\n{block}"),
            selection: (a + shift, b + shift),
        })
    }
}

/// Copy the selected lines above or below themselves. The selection ends up on the upper copy
/// when copying up (it keeps its line numbers) and on the lower copy when copying down, like VS
/// Code.
pub fn duplicate_lines(text: &str, start: usize, end: usize, down: bool) -> Edit {
    let (first, last) = line_span(text, start, end);
    let (a, b) = (start.min(end), start.max(end));
    let block = &text[first..last];
    let shift = if down { block.len() + 1 } else { 0 };
    Edit {
        replace: first..last,
        replacement: format!("{block}\n{block}"),
        selection: (a + shift, b + shift),
    }
}

/// Delete the selected lines including their line break.
pub fn delete_lines(text: &str, start: usize, end: usize) -> Edit {
    let (first, last) = line_span(text, start, end);
    // The last line has no break after it: take the one before instead of leaving a blank line.
    let (from, to) = if last < text.len() {
        (first, last + 1)
    } else if first > 0 {
        (first - 1, last)
    } else {
        (first, last)
    };
    Edit {
        replace: from..to,
        replacement: String::new(),
        selection: (from.min(first), from.min(first)),
    }
}

/// Runs the line command bound to the key chord pressed in this frame, if any, for the text edit
/// `id`. Returns true when the text or caret changed.
pub fn handle(ui: &egui::Ui, id: egui::Id, text: &mut String, extension: &str) -> bool {
    use egui::{Key, Modifiers};
    // Mixed line endings (a stray CR in the buffer) would make line moves unsafe: leave the keys
    // to egui there.
    if !ui.memory(|memory| memory.has_focus(id)) || text.contains('\r') {
        return false;
    }
    enum Command {
        Comment,
        Move(bool),
        Duplicate(bool),
        Delete,
    }
    let chords = [
        (
            Modifiers::ALT | Modifiers::SHIFT,
            Key::ArrowUp,
            Command::Duplicate(false),
        ),
        (
            Modifiers::ALT | Modifiers::SHIFT,
            Key::ArrowDown,
            Command::Duplicate(true),
        ),
        (
            Modifiers::COMMAND | Modifiers::SHIFT,
            Key::K,
            Command::Delete,
        ),
        (Modifiers::ALT, Key::ArrowUp, Command::Move(true)),
        (Modifiers::ALT, Key::ArrowDown, Command::Move(false)),
        (Modifiers::COMMAND, Key::Slash, Command::Comment),
    ];
    // The exact chord: a pattern's extra Shift or Alt must not be ignored.
    let Some((modifiers, key, command)) = chords.into_iter().find(|(modifiers, key, _)| {
        ui.input(|input| input.key_pressed(*key) && input.modifiers.matches_exact(*modifiers))
    }) else {
        return false;
    };
    let Some((start, end)) = editing::selection_bytes(ui, id, text) else {
        return false;
    };
    let edit = match command {
        Command::Comment => {
            comment_prefix(extension).map(|prefix| toggle_comment(text, start, end, prefix))
        }
        Command::Move(up) => move_lines(text, start, end, up),
        Command::Duplicate(down) => Some(duplicate_lines(text, start, end, down)),
        Command::Delete => Some(delete_lines(text, start, end)),
    };
    // The key is only taken when the command did something.
    let Some(edit) = edit else { return false };
    ui.input_mut(|input| input.consume_key(modifiers, key));
    text.replace_range(edit.replace.clone(), &edit.replacement);
    editing::place_caret(ui, id, text, edit.selection.0, edit.selection.1);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::text::{CCursor, CCursorRange};

    fn apply(text: &str, edit: &Edit) -> (String, (usize, usize)) {
        let mut result = text.to_string();
        result.replace_range(edit.replace.clone(), &edit.replacement);
        (result, edit.selection)
    }

    #[test]
    fn comments_are_added_at_the_shallowest_indent_and_removed_again() {
        let text = "fn f() {\n    a;\n\n        b;\n}";
        let (commented, _) = apply(text, &toggle_comment(text, 0, text.len(), "//"));
        assert_eq!(commented, "// fn f() {\n//     a;\n\n//         b;\n// }");
        let (back, _) = apply(
            &commented,
            &toggle_comment(&commented, 0, commented.len(), "//"),
        );
        assert_eq!(back, text);
        // Only the indented lines: the marker lines up with the shallowest of them.
        let (inner, _) = apply(text, &toggle_comment(text, 10, 28, "//"));
        assert_eq!(inner, "fn f() {\n    // a;\n\n    //     b;\n}");
    }

    #[test]
    fn a_mixed_block_gets_commented_not_uncommented() {
        let text = "// a\nb";
        let (result, _) = apply(text, &toggle_comment(text, 0, text.len(), "//"));
        assert_eq!(result, "// // a\n// b");
        // Markers without the following space are removed too.
        let text = "#x\n# y";
        let (result, _) = apply(text, &toggle_comment(text, 0, text.len(), "#"));
        assert_eq!(result, "x\ny");
        // Blank-only selections do nothing.
        let text = "a\n\n";
        let (result, _) = apply(text, &toggle_comment(text, 2, 3, "//"));
        assert_eq!(result, text);
    }

    #[test]
    fn toggling_keeps_the_caret_with_its_text() {
        let text = "ab\ncd";
        let (result, (a, b)) = apply(text, &toggle_comment(text, 4, 4, "//"));
        assert_eq!(result, "ab\n// cd");
        assert_eq!((a, b), (7, 7));
        let (back, (a, _)) = apply(&result, &toggle_comment(&result, 4, 4, "//"));
        assert_eq!(back, "ab\ncd");
        assert_eq!(a, 3, "a caret inside the marker lands at the line start");
        // A caret before the insertion point stays put.
        let text = "  x";
        let (_, (a, _)) = apply(text, &toggle_comment(text, 1, 1, "//"));
        assert_eq!(a, 1);
    }

    #[test]
    fn rem_is_a_whole_word_in_any_case() {
        let text = "REMOVE x\nREM y";
        let (result, _) = apply(text, &toggle_comment(text, 0, text.len(), "REM"));
        assert_eq!(result, "REM REMOVE x\nREM REM y", "REMOVE is not a comment");
        let text = "rem a\nRem b\nREM";
        let (result, _) = apply(text, &toggle_comment(text, 0, text.len(), "REM"));
        assert_eq!(result, "a\nb\n");
        // A symbol marker still matches as written.
        assert_eq!(marker_len("// x", "//"), Some(2));
        assert_eq!(marker_len("/ x", "//"), None);
    }

    #[test]
    fn the_comment_marker_depends_on_the_language() {
        assert_eq!(comment_prefix("rs"), Some("//"));
        assert_eq!(comment_prefix("PY"), Some("#"));
        assert_eq!(comment_prefix("sql"), Some("--"));
        assert_eq!(comment_prefix("bat"), Some("REM"));
        assert_eq!(comment_prefix("json"), None);
        assert_eq!(comment_prefix("md"), None);
    }

    #[test]
    fn lines_move_up_and_down_with_their_selection() {
        let text = "one\ntwo\nthree";
        // Move "two" (caret in it) up and down.
        let (up, (a, _)) = apply(text, &move_lines(text, 5, 5, true).unwrap());
        assert_eq!(up, "two\none\nthree");
        assert_eq!(a, 1);
        let (down, (a, _)) = apply(text, &move_lines(text, 5, 5, false).unwrap());
        assert_eq!(down, "one\nthree\ntwo");
        assert_eq!(a, 11);
        // The edges do nothing.
        assert!(move_lines(text, 0, 0, true).is_none());
        assert!(move_lines(text, 12, 12, false).is_none());
        // A two-line block moves as one.
        let (block, _) = apply(text, &move_lines(text, 0, 6, false).unwrap());
        assert_eq!(block, "three\none\ntwo");
    }

    #[test]
    fn duplicating_and_deleting_lines() {
        let text = "a\nb\nc";
        let (down, (s, e)) = apply(text, &duplicate_lines(text, 2, 2, true));
        assert_eq!(down, "a\nb\nb\nc");
        assert_eq!((s, e), (4, 4));
        let (up, (s, _)) = apply(text, &duplicate_lines(text, 2, 2, false));
        assert_eq!(up, "a\nb\nb\nc");
        assert_eq!(s, 2);
        // Without a final newline the last line duplicates cleanly.
        assert_eq!(apply("x", &duplicate_lines("x", 1, 1, true)).0, "x\nx");

        assert_eq!(apply(text, &delete_lines(text, 2, 2)).0, "a\nc");
        assert_eq!(apply(text, &delete_lines(text, 0, 0)).0, "b\nc");
        // The last line takes the break before it, so no blank line is left.
        assert_eq!(apply(text, &delete_lines(text, 4, 4)).0, "a\nb");
        assert_eq!(apply("only", &delete_lines("only", 2, 2)).0, "");
        assert_eq!(apply("", &delete_lines("", 0, 0)).0, "");
        // Selections reaching into several lines remove all of them.
        assert_eq!(apply(text, &delete_lines(text, 0, 3)).0, "c");
    }

    #[test]
    fn multibyte_text_and_stray_offsets_do_not_panic() {
        let text = "héllo\nwörld";
        let _ = toggle_comment(text, 2, 9, "//");
        let _ = move_lines(text, 2, 2, false);
        let _ = duplicate_lines(text, 99, 99, true);
        let _ = delete_lines(text, 2, 40);
    }

    fn press(
        text: &str,
        at: usize,
        key: egui::Key,
        modifiers: egui::Modifiers,
        extension: &str,
    ) -> (String, bool) {
        let context = egui::Context::default();
        let id = egui::Id::unique("editor");
        let mut text = text.to_string();
        let mut output = context.run_ui(Default::default(), |ui| {
            let shown = egui::TextEdit::multiline(&mut text).id(id).show(ui);
            shown.response.request_focus();
            let mut state = shown.state;
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
            state.store(ui.ctx(), id);
        });
        output.textures_delta.clear();
        let raw = egui::RawInput {
            events: vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                },
            ],
            ..Default::default()
        };
        let mut handled = false;
        let mut output = context.run_ui(raw, |ui| {
            handled = handle(ui, id, &mut text, extension);
        });
        output.textures_delta.clear();
        (text, handled)
    }

    #[test]
    fn the_chords_reach_the_right_command_through_egui() {
        use egui::{Key, Modifiers};
        let text = "a\nb\nc";
        assert_eq!(
            press(text, 2, Key::ArrowUp, Modifiers::ALT, "rs").0,
            "b\na\nc"
        );
        assert_eq!(
            press(text, 2, Key::ArrowDown, Modifiers::ALT, "rs").0,
            "a\nc\nb"
        );
        // Shift+Alt is a duplicate, not a move.
        let shift_alt = Modifiers::ALT | Modifiers::SHIFT;
        assert_eq!(
            press(text, 2, Key::ArrowDown, shift_alt, "rs").0,
            "a\nb\nb\nc"
        );
        assert_eq!(
            press(text, 2, Key::K, Modifiers::COMMAND | Modifiers::SHIFT, "rs").0,
            "a\nc"
        );
        assert_eq!(
            press(text, 2, Key::Slash, Modifiers::COMMAND, "rs").0,
            "a\n// b\nc"
        );
        // A file type without a comment marker is left alone, and unbound keys are ignored.
        let (same, handled) = press(text, 2, Key::Slash, Modifiers::COMMAND, "json");
        assert!(!handled);
        assert_eq!(same, text);
        assert!(!press(text, 2, Key::ArrowUp, Modifiers::NONE, "rs").1);
        // Extra modifiers do not turn one chord into another.
        assert!(
            !press(
                text,
                2,
                Key::Slash,
                Modifiers::COMMAND | Modifiers::SHIFT,
                "rs"
            )
            .1
        );
        assert!(
            !press(
                text,
                2,
                Key::K,
                Modifiers::COMMAND | Modifiers::SHIFT | Modifiers::ALT,
                "rs"
            )
            .1
        );
        // A buffer with a stray CR is not touched.
        assert!(!press("a\r\nb", 3, Key::ArrowUp, Modifiers::ALT, "rs").1);
    }
}
