//! Text transformations run from the command palette on the editor's selection (or on the whole
//! file when nothing is selected, where that makes sense): change case, sort and de-duplicate
//! lines, trim trailing whitespace and join lines, plus Select Next Occurrence (Ctrl+D). Each
//! returns an [`Edit`] like the other editing helpers, so the selection follows the new text.

use crate::editing::{self, Edit};
use eframe::egui;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Uppercase,
    Lowercase,
    SortAscending,
    SortDescending,
    RemoveDuplicateLines,
    TrimTrailingWhitespace,
    JoinLines,
    SelectNextOccurrence,
}

/// The first and last byte of the lines the selection touches (the end is before that line's
/// newline); the whole text when `whole_if_empty` and nothing is selected.
fn lines_of(text: &str, start: usize, end: usize, whole_if_empty: bool) -> (usize, usize) {
    let (start, end) = (editing::boundary(text, start), editing::boundary(text, end));
    let (start, end) = (start.min(end), start.max(end));
    if start == end && whole_if_empty {
        return (0, text.len());
    }
    let first = text[..start].rfind('\n').map_or(0, |index| index + 1);
    let anchor = if end > start && text[..end].ends_with('\n') {
        end - 1
    } else {
        end
    };
    let last = text[anchor..]
        .find('\n')
        .map_or(text.len(), |index| anchor + index);
    (first, without_carriage_return(text, first, last))
}

/// `last` moved back before a carriage return that belongs to the line break after it (`\r\n`).
fn without_carriage_return(text: &str, first: usize, last: usize) -> usize {
    let last = last.max(first);
    if last > first && text.as_bytes()[last - 1] == b'\r' && text[last..].starts_with('\n') {
        last - 1
    } else {
        last
    }
}

/// The lines of `text` as (content, line break) pairs: the break is `"\r\n"`, `"\n"` or empty for a
/// last line without one. Sorting and de-duplicating move the contents and leave the breaks where
/// they are, so a final newline and the file's line-ending style survive.
fn lines_with_breaks(text: &str) -> Vec<(&str, &str)> {
    text.split_inclusive('\n')
        .map(|piece| {
            if let Some(body) = piece.strip_suffix("\r\n") {
                (body, "\r\n")
            } else if let Some(body) = piece.strip_suffix('\n') {
                (body, "\n")
            } else {
                (piece, "")
            }
        })
        .collect()
}

/// Replace `first..last` and select the replacement.
fn replace_selecting(first: usize, last: usize, replacement: String) -> Edit {
    let end = first + replacement.len();
    Edit {
        replace: first..last,
        replacement,
        selection: (first, end),
    }
}

/// Order lines for sorting: ignoring case first, so `apple` sorts next to `Apple`, and by the exact
/// text second so the order is the same every time.
fn sort_key(line: &str) -> (String, &str) {
    (line.to_lowercase(), line)
}

pub fn apply(kind: Kind, text: &str, start: usize, end: usize) -> Option<Edit> {
    let (start, end) = (editing::boundary(text, start), editing::boundary(text, end));
    let (start, end) = (start.min(end), start.max(end));
    match kind {
        Kind::Uppercase | Kind::Lowercase => {
            if start == end {
                return None;
            }
            let selected = &text[start..end];
            let changed = if kind == Kind::Uppercase {
                selected.to_uppercase()
            } else {
                selected.to_lowercase()
            };
            (changed != selected).then(|| replace_selecting(start, end, changed))
        }
        Kind::SortAscending | Kind::SortDescending | Kind::RemoveDuplicateLines => {
            let (first, last) = lines_of(text, start, end, true);
            let original = &text[first..last];
            let lines = lines_with_breaks(original);
            let bodies: Vec<&str> = lines.iter().map(|(body, _)| *body).collect();
            let mut changed = bodies.clone();
            match kind {
                Kind::SortAscending => changed.sort_by_key(|line| sort_key(line)),
                Kind::SortDescending => {
                    changed.sort_by_key(|line| std::cmp::Reverse(sort_key(line)));
                }
                _ => {
                    let mut seen = std::collections::HashSet::new();
                    changed.retain(|line| seen.insert(*line));
                }
            }
            if changed == bodies {
                return None;
            }
            // The line breaks stay in their places; when lines were dropped the last line keeps
            // the break the file had at its end.
            let last_break = lines.last().map_or("", |(_, line_break)| *line_break);
            let mut result = String::with_capacity(original.len());
            for (index, body) in changed.iter().enumerate() {
                result.push_str(body);
                if index + 1 == changed.len() {
                    result.push_str(last_break);
                } else {
                    result.push_str(lines.get(index).map_or("\n", |(_, line_break)| *line_break));
                }
            }
            Some(replace_selecting(first, first + original.len(), result))
        }
        Kind::TrimTrailingWhitespace => {
            let (first, last) = lines_of(text, start, end, true);
            let trimmed: Vec<&str> = text[first..last]
                .split('\n')
                .map(|line| {
                    // A carriage return before the break is part of the line ending, not blank.
                    let body = line.strip_suffix('\r');
                    let kept = body.unwrap_or(line).trim_end_matches([' ', '\t']);
                    if body.is_some() {
                        // `kept` ends before the `\r`; the `\r` is added back below.
                        &line[..kept.len()]
                    } else {
                        kept
                    }
                })
                .collect();
            let mut result = String::new();
            for (index, (original, line)) in text[first..last].split('\n').zip(&trimmed).enumerate()
            {
                if index > 0 {
                    result.push('\n');
                }
                result.push_str(line);
                if original.ends_with('\r') {
                    result.push('\r');
                }
            }
            (result != text[first..last]).then(|| replace_selecting(first, last, result))
        }
        Kind::JoinLines => {
            // With a selection, join the lines it touches; with a caret, the line and the next.
            let (first, mut last) = lines_of(text, start, end, false);
            if start == end && last < text.len() {
                // `last` stops before a carriage return, so skip it with the break to the next line.
                let after = text[last..]
                    .find('\n')
                    .map_or(text.len(), |index| last + index + 1);
                let next_end = text[after..]
                    .find('\n')
                    .map_or(text.len(), |index| after + index);
                last = without_carriage_return(text, first, next_end);
            }
            let block = &text[first..last];
            if !block.contains('\n') {
                return None;
            }
            let mut joined = String::new();
            for (index, line) in block.split('\n').enumerate() {
                let line = line.strip_suffix('\r').unwrap_or(line);
                if index == 0 {
                    joined.push_str(line.trim_end());
                } else if !line.trim().is_empty() {
                    if !joined.is_empty() {
                        joined.push(' ');
                    }
                    joined.push_str(line.trim());
                }
            }
            Some(replace_selecting(first, last, joined))
        }
        Kind::SelectNextOccurrence => {
            // With a caret, first select the word under it; with a selection, find the next one.
            if start == end {
                let is_word = |c: char| c.is_alphanumeric() || c == '_';
                let from = text[..start]
                    .char_indices()
                    .rev()
                    .take_while(|(_, c)| is_word(*c))
                    .last()
                    .map_or(start, |(index, _)| index);
                let to = start
                    + text[start..]
                        .char_indices()
                        .take_while(|(_, c)| is_word(*c))
                        .last()
                        .map_or(0, |(index, c)| index + c.len_utf8());
                return (from < to).then_some(Edit {
                    replace: from..from,
                    replacement: String::new(),
                    selection: (from, to),
                });
            }
            let needle = &text[start..end];
            // The next match after the selection, wrapping round to the start of the file.
            let found = text[end..]
                .find(needle)
                .map(|index| end + index)
                .or_else(|| text[..start].find(needle))?;
            Some(Edit {
                replace: found..found,
                replacement: String::new(),
                selection: (found, found + needle.len()),
            })
        }
    }
}

/// Run a transformation on the selection of the text edit `id`. Returns true when something
/// changed (a different selection counts).
pub fn run(ui: &egui::Ui, id: egui::Id, text: &mut String, kind: Kind) -> bool {
    let Some((start, end)) = editing::selection_bytes(ui, id, text) else {
        return false;
    };
    let Some(edit) = apply(kind, text, start, end) else {
        return false;
    };
    text.replace_range(edit.replace.clone(), &edit.replacement);
    editing::place_caret(ui, id, text, edit.selection.0, edit.selection.1);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go(kind: Kind, text: &str, start: usize, end: usize) -> Option<(String, (usize, usize))> {
        apply(kind, text, start, end).map(|edit| {
            let mut result = text.to_string();
            result.replace_range(edit.replace.clone(), &edit.replacement);
            (result, edit.selection)
        })
    }

    #[test]
    fn case_changes_apply_to_the_selection_only() {
        assert_eq!(
            go(Kind::Uppercase, "abc def", 4, 7),
            Some(("abc DEF".into(), (4, 7)))
        );
        assert_eq!(
            go(Kind::Lowercase, "ABC DEF", 0, 3),
            Some(("abc DEF".into(), (0, 3)))
        );
        // Nothing selected, or already in that case: nothing to do.
        assert_eq!(go(Kind::Uppercase, "abc", 1, 1), None);
        assert_eq!(go(Kind::Uppercase, "ABC", 0, 3), None);
        // A character that grows when upper-cased keeps the selection on the new text.
        assert_eq!(
            go(Kind::Uppercase, "stra\u{df}e", 0, 7),
            Some(("STRASSE".into(), (0, 7)))
        );
    }

    #[test]
    fn lines_sort_ignoring_case_and_the_whole_file_is_used_without_a_selection() {
        let text = "pear\nApple\nbanana\napple";
        assert_eq!(
            go(Kind::SortAscending, text, 0, 0),
            Some(("Apple\napple\nbanana\npear".into(), (0, 23)))
        );
        assert_eq!(
            go(Kind::SortDescending, text, 0, 0).unwrap().0,
            "pear\nbanana\napple\nApple"
        );
        // Only the selected lines are sorted.
        assert_eq!(
            go(Kind::SortAscending, "z\nb\na\nq", 2, 5).unwrap().0,
            "z\na\nb\nq"
        );
        // Already sorted: no edit.
        assert_eq!(go(Kind::SortAscending, "a\nb", 0, 0), None);
    }

    #[test]
    fn sorting_keeps_the_final_newline_and_the_line_ending_style() {
        // A whole file that ends with a newline: no phantom blank line comes to the top.
        assert_eq!(go(Kind::SortAscending, "b\na\n", 0, 0).unwrap().0, "a\nb\n");
        assert_eq!(
            go(Kind::SortDescending, "a\nb\n", 0, 0).unwrap().0,
            "b\na\n"
        );
        // CRLF: the breaks stay where they are, in the same style.
        assert_eq!(go(Kind::SortAscending, "b\r\na", 0, 0).unwrap().0, "a\r\nb");
        assert_eq!(
            go(Kind::SortAscending, "c\r\nb\r\na\r\n", 0, 0).unwrap().0,
            "a\r\nb\r\nc\r\n"
        );
        // Blank lines are lines too and are not dropped.
        assert_eq!(
            go(Kind::SortAscending, "b\n\na\n", 0, 0).unwrap().0,
            "\na\nb\n"
        );
        // De-duplicating keeps the final newline, whether or not the last line was a repeat.
        assert_eq!(
            go(Kind::RemoveDuplicateLines, "a\n\n", 0, 0),
            None,
            "one blank line is no repeat"
        );
        assert_eq!(
            go(Kind::RemoveDuplicateLines, "a\n\n\n", 0, 0).unwrap().0,
            "a\n\n"
        );
        assert_eq!(
            go(Kind::RemoveDuplicateLines, "a\nb\na\n", 0, 0).unwrap().0,
            "a\nb\n"
        );
        assert_eq!(
            go(Kind::RemoveDuplicateLines, "a\nb\na", 0, 0).unwrap().0,
            "a\nb"
        );
        assert_eq!(
            go(Kind::RemoveDuplicateLines, "a\r\nb\r\na", 0, 0)
                .unwrap()
                .0,
            "a\r\nb"
        );
        // A partial selection leaves the text around it alone.
        assert_eq!(
            go(Kind::SortAscending, "z\nb\na\nq\n", 2, 5).unwrap().0,
            "z\na\nb\nq\n"
        );
    }

    #[test]
    fn joining_keeps_the_carriage_return_of_the_break_after_the_block() {
        assert_eq!(
            go(Kind::JoinLines, "a\r\nb\r\nc", 0, 0).unwrap().0,
            "a b\r\nc"
        );
        assert_eq!(
            go(Kind::JoinLines, "a\r\nb\r\nc", 0, 6).unwrap().0,
            "a b\r\nc"
        );
        assert_eq!(
            go(Kind::JoinLines, "x\r\na\r\nb\r\nc\r\n", 3, 3).unwrap().0,
            "x\r\na b\r\nc\r\n"
        );
    }

    #[test]
    fn duplicate_lines_are_removed_keeping_the_first() {
        assert_eq!(
            go(Kind::RemoveDuplicateLines, "a\nb\na\nc\nb", 0, 0)
                .unwrap()
                .0,
            "a\nb\nc"
        );
        assert_eq!(go(Kind::RemoveDuplicateLines, "a\nb", 0, 0), None);
        // A blank line counts as a line: only the repeats go.
        assert_eq!(
            go(Kind::RemoveDuplicateLines, "a\n\nb\n\nc", 0, 0)
                .unwrap()
                .0,
            "a\n\nb\nc"
        );
    }

    #[test]
    fn trailing_whitespace_goes_but_line_endings_stay() {
        assert_eq!(
            go(Kind::TrimTrailingWhitespace, "a  \nb\t\n c ", 0, 0)
                .unwrap()
                .0,
            "a\nb\n c"
        );
        // A carriage return is a line ending, not whitespace to trim.
        assert_eq!(
            go(Kind::TrimTrailingWhitespace, "a  \r\nb\r\n", 0, 0)
                .unwrap()
                .0,
            "a\r\nb\r\n"
        );
        assert_eq!(go(Kind::TrimTrailingWhitespace, "clean\nfile", 0, 0), None);
        // Only the selected lines are touched.
        assert_eq!(
            go(Kind::TrimTrailingWhitespace, "a \nb \nc ", 3, 4)
                .unwrap()
                .0,
            "a \nb\nc "
        );
    }

    #[test]
    fn lines_join_with_one_space() {
        // A selection across lines joins them all.
        assert_eq!(
            go(Kind::JoinLines, "  one\n   two  \n\nthree\nfour", 0, 20)
                .unwrap()
                .0,
            "  one two three\nfour"
        );
        // A caret joins its line with the next.
        assert_eq!(go(Kind::JoinLines, "a\nb\nc", 0, 0).unwrap().0, "a b\nc");
        assert_eq!(
            go(Kind::JoinLines, "a\nb\nc", 4, 4),
            None,
            "no line after the last"
        );
        assert_eq!(go(Kind::JoinLines, "single", 0, 6), None);
        assert_eq!(go(Kind::JoinLines, "a\r\nb", 0, 0).unwrap().0, "a b");
    }

    #[test]
    fn select_next_occurrence_selects_the_word_then_the_next_match_and_wraps() {
        let text = "foo bar foo baz foo";
        // From a caret inside the first foo: the word itself.
        assert_eq!(
            go(Kind::SelectNextOccurrence, text, 1, 1).unwrap().1,
            (0, 3)
        );
        // Then each next one, and back to the first after the last.
        assert_eq!(
            go(Kind::SelectNextOccurrence, text, 0, 3).unwrap().1,
            (8, 11)
        );
        assert_eq!(
            go(Kind::SelectNextOccurrence, text, 8, 11).unwrap().1,
            (16, 19)
        );
        assert_eq!(
            go(Kind::SelectNextOccurrence, text, 16, 19).unwrap().1,
            (0, 3)
        );
        // A selection that occurs only once has no other place to go; a caret on a space selects
        // nothing.
        assert_eq!(go(Kind::SelectNextOccurrence, "only", 0, 4), None);
        assert_eq!(go(Kind::SelectNextOccurrence, "a  b", 2, 2), None);
        // Multibyte words are selected whole.
        assert_eq!(
            go(Kind::SelectNextOccurrence, "h\u{e9}llo w", 2, 2)
                .unwrap()
                .1,
            (0, 6)
        );
    }

    #[test]
    fn stray_offsets_and_empty_text_do_not_panic() {
        for kind in [
            Kind::Uppercase,
            Kind::Lowercase,
            Kind::SortAscending,
            Kind::SortDescending,
            Kind::RemoveDuplicateLines,
            Kind::TrimTrailingWhitespace,
            Kind::JoinLines,
            Kind::SelectNextOccurrence,
        ] {
            let _ = apply(kind, "", 0, 0);
            let _ = apply(kind, "h\u{e9}llo\nw\u{f6}rld", 2, 99);
            let _ = apply(kind, "x", 40, 50);
        }
    }
}
