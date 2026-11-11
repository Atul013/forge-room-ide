//! Text search used by Find/Replace. All positions are character indexes (not bytes), which is
//! what the editor's cursor uses.

use std::ops::Range;

pub const MAX_MATCHES: usize = 20_000;

/// How a query is matched: exactly or ignoring case, anywhere or only as a whole word, as plain
/// text or as a regular expression.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Mode {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A query ready to be run over many texts. Regular expressions are compiled once, with a limit on
/// backtracking so a pattern like `(a+)+$` ends with "no results" instead of hanging the editor.
pub struct Matcher {
    kind: Kind,
}

enum Kind {
    Literal {
        needle: Vec<char>,
        case_sensitive: bool,
        whole_word: bool,
    },
    Pattern(fancy_regex::Regex),
}

impl Matcher {
    /// Compile `query`; the error is the regular expression's own message. An empty query is valid
    /// and matches nothing.
    pub fn new(query: &str, mode: Mode) -> Result<Matcher, String> {
        if !mode.regex || query.is_empty() {
            return Ok(Matcher {
                kind: Kind::Literal {
                    needle: query.chars().collect(),
                    case_sensitive: mode.case_sensitive,
                    whole_word: mode.whole_word,
                },
            });
        }
        let mut pattern = query.to_string();
        if mode.whole_word {
            pattern = format!(r"\b(?:{pattern})\b");
        }
        if !mode.case_sensitive {
            pattern = format!("(?i){pattern}");
        }
        fancy_regex::RegexBuilder::new(&pattern)
            .backtrack_limit(200_000)
            .build()
            .map(|regex| Matcher {
                kind: Kind::Pattern(regex),
            })
            .map_err(|error| error.to_string())
    }

    /// Every non-empty, non-overlapping match in `text`, as character ranges.
    pub fn find_all(&self, text: &str) -> Vec<Range<usize>> {
        match &self.kind {
            Kind::Literal {
                needle,
                case_sensitive,
                whole_word,
            } => literal(text, needle, *case_sensitive, *whole_word),
            Kind::Pattern(regex) => {
                let mut bytes: Vec<Range<usize>> = Vec::new();
                for found in regex.find_iter(text) {
                    // A match that exceeds the backtracking limit is an error: stop there.
                    let Ok(found) = found else { break };
                    if found.start() < found.end() {
                        bytes.push(found.start()..found.end());
                    }
                    if bytes.len() >= MAX_MATCHES {
                        break;
                    }
                }
                to_char_ranges(text, &bytes)
            }
        }
    }
}

/// Convert ascending byte ranges into character ranges in one pass over the text.
fn to_char_ranges(text: &str, bytes: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut chars = text.char_indices().map(|(byte, _)| byte).peekable();
    let (mut index, mut result) = (0usize, Vec::with_capacity(bytes.len()));
    let mut at = |target: usize, index: &mut usize| {
        while let Some(&byte) = chars.peek() {
            if byte >= target {
                break;
            }
            chars.next();
            *index += 1;
        }
        // Past the last character the index is the character count.
        if chars.peek().is_none() && target >= text.len() {
            *index = text.chars().count();
        }
        *index
    };
    for range in bytes {
        let start = at(range.start, &mut index);
        let end = at(range.end, &mut index);
        result.push(start..end);
    }
    result
}

fn literal(
    text: &str,
    needle: &[char],
    case_sensitive: bool,
    whole_word: bool,
) -> Vec<Range<usize>> {
    if needle.is_empty() {
        return Vec::new();
    }
    let hay: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index + needle.len() <= hay.len() && found.len() < MAX_MATCHES {
        let end = index + needle.len();
        let equal = hay[index..end]
            .iter()
            .zip(needle)
            .all(|(a, b)| same(*a, *b, case_sensitive));
        // A word edge only needs a boundary where the query itself starts or ends with a word
        // character, so `foo(` can still be found as a whole word.
        let before = index == 0 || !is_word(hay[index - 1]) || !is_word(needle[0]);
        let after = end == hay.len() || !is_word(hay[end]) || !is_word(needle[needle.len() - 1]);
        if equal && (!whole_word || (before && after)) {
            found.push(index..end);
            index = end;
        } else {
            index += 1;
        }
    }
    found
}

fn same(a: char, b: char, case_sensitive: bool) -> bool {
    if case_sensitive {
        a == b
    } else {
        a == b || a.to_lowercase().eq(b.to_lowercase())
    }
}

/// Every non-overlapping occurrence of `query`, as character ranges (plain text, tests only:
/// the app goes through [`Matcher`]).
#[cfg(test)]
pub fn find_all(text: &str, query: &str, case_sensitive: bool) -> Vec<Range<usize>> {
    literal(
        text,
        &query.chars().collect::<Vec<_>>(),
        case_sensitive,
        false,
    )
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

    fn matches(query: &str, text: &str, mode: Mode) -> Vec<(usize, usize)> {
        Matcher::new(query, mode)
            .unwrap()
            .find_all(text)
            .into_iter()
            .map(|range| (range.start, range.end))
            .collect()
    }

    #[test]
    fn whole_word_only_matches_between_non_word_characters() {
        let words = Mode {
            whole_word: true,
            ..Mode::default()
        };
        assert_eq!(
            matches("cat", "cat concat cat_x cat.", words),
            vec![(0, 3), (17, 20)]
        );
        assert_eq!(matches("cat", "cat concat", Mode::default()).len(), 2);
        // A query that ends in a symbol needs no boundary after it.
        assert_eq!(matches("foo(", "foo(x) xfoo(", words), vec![(0, 4)]);
        // Case still follows the option.
        let exact = Mode {
            whole_word: true,
            case_sensitive: true,
            regex: false,
        };
        assert_eq!(matches("Cat", "cat Cat", exact), vec![(4, 7)]);
    }

    #[test]
    fn regular_expressions_give_character_ranges_for_multibyte_text() {
        let regex = Mode {
            regex: true,
            ..Mode::default()
        };
        // "é" is two bytes: the ranges must still count characters.
        assert_eq!(matches(r"w.rld", "héllo wörld", regex), vec![(6, 11)]);
        assert_eq!(
            matches(r"\d+", "a1 b22 c333", regex),
            vec![(1, 2), (4, 6), (8, 11)]
        );
        assert_eq!(
            matches("É", "été", regex),
            vec![(0, 1), (2, 3)],
            "ignores case by default"
        );
        let exact = Mode {
            regex: true,
            case_sensitive: true,
            whole_word: false,
        };
        assert!(matches("É", "été", exact).is_empty());
        // Whole word applies to the whole pattern.
        let both = Mode {
            regex: true,
            whole_word: true,
            ..Mode::default()
        };
        assert_eq!(
            matches("a|ab", "ab a ab b", both),
            vec![(0, 2), (3, 4), (5, 7)]
        );
    }

    #[test]
    fn empty_matches_invalid_patterns_and_runaway_patterns_are_handled() {
        let regex = Mode {
            regex: true,
            ..Mode::default()
        };
        // A pattern that can match nothing never produces empty matches.
        assert_eq!(matches("x*", "axxb", regex), vec![(1, 3)]);
        assert!(matches("", "abc", regex).is_empty());
        assert!(Matcher::new("(", regex).is_err());
        assert!(Matcher::new("[a-", regex).is_err());
        // Catastrophic backtracking gives up instead of hanging.
        let text = format!("{}!", "a".repeat(40));
        let started = std::time::Instant::now();
        let _ = Matcher::new("(a+)+$", regex).unwrap().find_all(&text);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        // Plain text queries are never treated as patterns.
        assert_eq!(matches("a.c", "abc a.c", Mode::default()), vec![(4, 7)]);
        assert_eq!(matches("(", "f(x)", Mode::default()), vec![(1, 2)]);
    }

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
