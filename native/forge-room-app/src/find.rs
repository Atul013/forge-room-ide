//! Text search used by Find/Replace. All positions are character indexes (not bytes), which is
//! what the editor's cursor uses.

use std::ops::Range;

pub const MAX_MATCHES: usize = 20_000;

fn same(a: char, b: char, case_sensitive: bool) -> bool {
    if case_sensitive {
        a == b
    } else {
        a == b || a.to_lowercase().eq(b.to_lowercase())
    }
}

/// Every non-overlapping occurrence of `query`, as character ranges.
pub fn find_all(text: &str, query: &str, case_sensitive: bool) -> Vec<Range<usize>> {
    let needle: Vec<char> = query.chars().collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let hay: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index + needle.len() <= hay.len() && found.len() < MAX_MATCHES {
        if hay[index..index + needle.len()]
            .iter()
            .zip(&needle)
            .all(|(a, b)| same(*a, *b, case_sensitive))
        {
            found.push(index..index + needle.len());
            index += needle.len();
        } else {
            index += 1;
        }
    }
    found
}

fn byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(byte, _)| byte)
        .unwrap_or(text.len())
}

pub fn replace_range(text: &str, range: Range<usize>, replacement: &str) -> String {
    let start = byte_index(text, range.start);
    let end = byte_index(text, range.end);
    let mut result = String::with_capacity(text.len() + replacement.len());
    result.push_str(&text[..start]);
    result.push_str(replacement);
    result.push_str(&text[end..]);
    result
}

/// Replace every match in one pass.
pub fn replace_all(text: &str, matches: &[Range<usize>], replacement: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;
    for range in matches {
        result.extend(&chars[cursor..range.start]);
        result.push_str(replacement);
        cursor = range.end;
    }
    result.extend(&chars[cursor..]);
    result
}

/// Character index of the start of 1-based `line`, clamped to the last line.
pub fn line_start(text: &str, line: usize) -> usize {
    let mut current = 1;
    if line <= 1 {
        return 0;
    }
    for (index, ch) in text.chars().enumerate() {
        if ch == '\n' {
            current += 1;
            if current == line {
                return index + 1;
            }
        }
    }
    // Past the end: the start of the last line.
    text.rfind('\n')
        .map(|byte| text[..byte + 1].chars().count())
        .unwrap_or(0)
}

/// Index of the first match at or after `from`, wrapping around; `None` when there are none.
pub fn next_match(matches: &[Range<usize>], from: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(
        matches
            .iter()
            .position(|range| range.start >= from)
            .unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_matches_case_sensitively_or_not() {
        assert_eq!(find_all("Foo foo FOO", "foo", true), vec![4..7]);
        assert_eq!(
            find_all("Foo foo FOO", "foo", false),
            vec![0..3, 4..7, 8..11]
        );
        assert!(find_all("abc", "", false).is_empty());
        assert!(find_all("abc", "abcd", false).is_empty());
    }

    #[test]
    fn matches_do_not_overlap_and_count_characters() {
        assert_eq!(find_all("aaaa", "aa", true), vec![0..2, 2..4]);
        // Positions are characters, not bytes.
        assert_eq!(find_all("héllo héllo", "héllo", true), vec![0..5, 6..11]);
    }

    #[test]
    fn replaces_one_or_all() {
        assert_eq!(replace_range("héllo world", 6..11, "there"), "héllo there");
        let text = "a-b-c";
        let matches = find_all(text, "-", true);
        assert_eq!(replace_all(text, &matches, "+"), "a+b+c");
        assert_eq!(replace_all("nothing", &[], "x"), "nothing");
        assert_eq!(replace_all("aXbX", &find_all("aXbX", "x", false), ""), "ab");
    }

    #[test]
    fn locates_line_starts() {
        let text = "one\ntwo\nthree";
        assert_eq!(line_start(text, 1), 0);
        assert_eq!(line_start(text, 2), 4);
        assert_eq!(line_start(text, 3), 8);
        assert_eq!(line_start(text, 99), 8);
        assert_eq!(line_start("single", 5), 0);
    }

    #[test]
    fn next_match_wraps_around() {
        let matches = vec![2..4, 10..12];
        assert_eq!(next_match(&matches, 0), Some(0));
        assert_eq!(next_match(&matches, 5), Some(1));
        assert_eq!(next_match(&matches, 13), Some(0));
        assert_eq!(next_match(&[], 0), None);
    }
}
