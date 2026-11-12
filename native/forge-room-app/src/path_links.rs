//! File references in terminal output (`src\main.rs:12:5`, `C:\work\lib.py:40`): finding the one
//! under a cell, resolving it against the project folder, and turning `line:column` into an
//! editor offset. Ctrl+click in the terminal uses these to jump to the place a compiler, test
//! runner or stack trace points at.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathRef {
    pub path: String,
    /// 1-based line, when the reference has one.
    pub line: Option<usize>,
    /// 1-based column, when the reference has one.
    pub column: Option<usize>,
}

fn is_delimiter(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '<' | '>' | ',' | ';' | '|' | '{' | '}'
        )
}

/// The file reference at character column `column` of `line`, if there is one. A reference is a
/// path whose last component has a short alphanumeric extension, optionally followed by
/// `:line` or `:line:column`. URLs and plain numbers are not references.
pub fn ref_at(line: &str, column: usize) -> Option<PathRef> {
    // A Windows path may hold spaces (`C:\Users\Jane Doe\my proj\run.ps1:3`): look for one around
    // the cell first, since the word-by-word reading below would cut it at the first space.
    if let Some(found) = spaced_windows_path(line, column) {
        return Some(found);
    }
    let chars: Vec<char> = line.chars().collect();
    if column >= chars.len() || is_delimiter(chars[column]) {
        return None;
    }
    let mut start = column;
    while start > 0 && !is_delimiter(chars[start - 1]) {
        start -= 1;
    }
    let mut end = column + 1;
    while end < chars.len() && !is_delimiter(chars[end]) {
        end += 1;
    }
    let word: String = chars[start..end].iter().collect();
    parse(word.trim_end_matches(['.', ':', '!', '?']))
}

/// A drive-letter path with spaces in it that covers character `column` of `line`, with an optional
/// `:line` and `:column`. The file name must end in an extension that is followed by a delimiter, so
/// `Jane.Doe` in a folder name does not end the path.
fn spaced_windows_path(line: &str, column: usize) -> Option<PathRef> {
    use std::sync::OnceLock;
    static PATTERN: OnceLock<fancy_regex::Regex> = OnceLock::new();
    let pattern = PATTERN.get_or_init(|| {
        fancy_regex::Regex::new(
            r#"[A-Za-z]:[\\/][^:"<>|*?\r\n]*?\.[A-Za-z0-9]{1,8}(?::(\d+))?(?::(\d+))?(?=[\s"'\)\],;]|$)"#,
        )
        .expect("the pattern is valid")
    });
    // Character index of each byte offset, to compare with the clicked cell.
    let at = |byte: usize| line[..byte].chars().count();
    for found in pattern.captures_iter(line).flatten() {
        let whole = found.get(0)?;
        if !whole.as_str().contains(' ') {
            continue; // no space: the ordinary reading handles it
        }
        if at(whole.start()) <= column && column < at(whole.end()) {
            let number = |group: usize| {
                found
                    .get(group)
                    .and_then(|m| m.as_str().parse::<usize>().ok())
            };
            let path_end = found
                .get(1)
                .map_or(whole.end(), |m| m.start().saturating_sub(1));
            return Some(PathRef {
                path: line[whole.start()..path_end].to_string(),
                line: number(1),
                column: number(2),
            });
        }
    }
    None
}

fn parse(word: &str) -> Option<PathRef> {
    if word.contains("://") || word.is_empty() {
        return None;
    }
    // Peel up to two trailing `:number` parts, from the right, so a drive letter's colon stays.
    let mut rest = word;
    let mut numbers: Vec<usize> = Vec::new();
    while numbers.len() < 2 {
        let Some((head, tail)) = rest.rsplit_once(':') else {
            break;
        };
        match tail.parse::<usize>() {
            Ok(number) if !tail.is_empty() => {
                numbers.push(number);
                rest = head;
            }
            _ => break,
        }
    }
    numbers.reverse();
    if !looks_like_a_file(rest) {
        return None;
    }
    Some(PathRef {
        path: rest.to_string(),
        line: numbers.first().copied(),
        column: numbers.get(1).copied(),
    })
}

/// A path with a real file name: a last component like `main.rs` (an extension of one to ten
/// letters or digits that is not all digits, so `1.5` and `v2.0` are not files).
fn looks_like_a_file(path: &str) -> bool {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && (1..=10).contains(&extension.len())
        && extension.chars().all(|c| c.is_ascii_alphanumeric())
        && !extension.chars().all(|c| c.is_ascii_digit())
        && !path
            .chars()
            .any(|c| matches!(c, '*' | '?' | '"' | '<' | '>' | '|'))
}

/// Whether `path` is a network or device path (`\\server\share`, `//host/x`, `\\?\C:`, `\\.\COM1`).
/// Checking such a path makes Windows contact the named machine, which can hand an attacker's
/// server a login, so terminal output must never cause it.
fn is_network_or_device_path(path: &str) -> bool {
    // Windows reads either kind of slash (and a mix of them) as the start of a network or device
    // path: `\\host`, `//host`, `\/host`, `/\host`, `\\?\`, `\\.\`.
    let mut start = path.chars();
    let is_slash = |c: char| c == '\\' || c == '/';
    matches!((start.next(), start.next()), (Some(a), Some(b)) if is_slash(a) && is_slash(b))
}

/// The file a reference points at: an absolute local path as it is, or a relative one inside
/// `root`. `None` when there is no such file, when the path is a network or device path, or when a
/// relative path climbs out of the project with `..`.
pub fn resolve(reference: &PathRef, root: &Path) -> Option<PathBuf> {
    if is_network_or_device_path(&reference.path) {
        return None;
    }
    // A file name Windows would treat as a device (CON, NUL...) is not a file to open.
    let name = Path::new(&reference.path).file_name()?.to_str()?;
    crate::file_ops::valid_name(name).ok()?;
    let given = Path::new(&reference.path);
    let candidate = if given.is_absolute() {
        given.to_path_buf()
    } else {
        if given
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return None;
        }
        // `./src/x.rs` and `.\src\x.rs` are the same as `src/x.rs`.
        let joined = root.join(given);
        // A link inside the project may still point out of it: the real location must be inside.
        let (real, base) = (joined.canonicalize().ok()?, root.canonicalize().ok()?);
        if !real.starts_with(&base) {
            return None;
        }
        joined
    };
    candidate.is_file().then_some(candidate)
}

/// Character index in `text` of 1-based `line` and `column`, clamped to the end of that line (and
/// to the last line), so a stale or wrong position never lands somewhere unrelated.
pub fn locate(text: &str, line: usize, column: usize) -> usize {
    let mut start = 0;
    let mut current = 1;
    let line = line.max(1);
    let total = text.chars().count();
    let mut index = 0;
    for ch in text.chars() {
        if current == line {
            break;
        }
        index += 1;
        if ch == '\n' {
            current += 1;
            start = index;
        }
    }
    if current < line {
        // Past the last line: the start of the last line.
        return start.min(total);
    }
    let length = text
        .chars()
        .skip(start)
        .take_while(|c| *c != '\n' && *c != '\r')
        .count();
    start + column.saturating_sub(1).min(length)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(line: &str, column: usize) -> Option<(String, Option<usize>, Option<usize>)> {
        ref_at(line, column).map(|r| (r.path, r.line, r.column))
    }

    #[test]
    fn compiler_and_runner_output_forms_are_found() {
        let rust = " --> src/main.rs:12:5";
        assert_eq!(
            found(rust, 8),
            Some(("src/main.rs".into(), Some(12), Some(5)))
        );
        // Any column inside the reference works, including the numbers at the end.
        assert_eq!(found(rust, rust.len() - 1), found(rust, 8));
        assert_eq!(
            found(r"error in C:\work\lib.py:40: boom", 14),
            Some((r"C:\work\lib.py".into(), Some(40), None))
        );
        assert_eq!(
            found("at ./tests/a.test.ts:9:1", 6),
            Some(("./tests/a.test.ts".into(), Some(9), Some(1)))
        );
        assert_eq!(
            found("see notes.md", 6),
            Some(("notes.md".into(), None, None))
        );
        assert_eq!(
            found("File \"app/x.py\", line 3", 8),
            Some(("app/x.py".into(), None, None))
        );
    }

    #[test]
    fn urls_numbers_and_plain_words_are_not_references() {
        for text in [
            "https://example.com/a.html:80",
            "1.5",
            "v2.0",
            "just words here",
            "12:30:45",
            "(none)",
            "a.",
            ".gitignore:3",
            "x.toolongextension",
        ] {
            let column = text.find(|c: char| !c.is_whitespace()).unwrap_or(0);
            assert_eq!(found(text, column), None, "{text:?}");
        }
        // Delimiters and out-of-range columns give nothing.
        assert_eq!(found("a b.rs", 1), None);
        assert_eq!(found("a.rs", 40), None);
        assert_eq!(found("", 0), None);
    }

    #[test]
    fn trailing_punctuation_is_not_part_of_the_reference() {
        assert_eq!(
            found("failed at lib.rs:7.", 12),
            Some(("lib.rs".into(), Some(7), None))
        );
        assert_eq!(
            found("see lib.rs:7:2:", 5),
            Some(("lib.rs".into(), Some(7), Some(2)))
        );
        // Multibyte text before it counts characters, not bytes.
        assert_eq!(
            found("é é src/é.rs:3", 5),
            Some(("src/é.rs".into(), Some(3), None))
        );
    }

    #[test]
    fn references_resolve_inside_the_project_or_as_absolute_paths() {
        let root = std::env::temp_dir().join(format!("forge-links-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("a.rs"), "x").unwrap();
        let reference = |path: &str| PathRef {
            path: path.into(),
            line: None,
            column: None,
        };
        assert_eq!(
            resolve(&reference("src/a.rs"), &root),
            Some(root.join("src/a.rs"))
        );
        assert!(resolve(&reference("./src/a.rs"), &root).is_some());
        assert!(resolve(&reference("src/missing.rs"), &root).is_none());
        let absolute = root.join("src").join("a.rs");
        assert_eq!(
            resolve(
                &reference(&absolute.to_string_lossy()),
                Path::new("elsewhere")
            ),
            Some(absolute)
        );
        // A folder is not a file to open.
        assert!(resolve(&reference("src"), &root).is_none());
        // Network and device paths are never checked: that would contact the named machine.
        for bad in [
            r"\\evil\share\a.txt",
            "//evil/share/a.txt",
            r"\\?\C:\a.txt",
            r"\\.\COM1",
        ] {
            assert!(resolve(&reference(bad), &root).is_none(), "{bad}");
        }
        // Leaving the project with `..` is refused, wherever the file is.
        assert!(resolve(&reference("../a.rs"), &root.join("src")).is_none());
        assert!(resolve(&reference("src/../src/a.rs"), &root).is_none());
        // Device names are not files.
        assert!(resolve(&reference("src/NUL.txt"), &root).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_line_and_column_become_a_clamped_editor_offset() {
        let text = "one\ntwo words\nthree";
        assert_eq!(locate(text, 1, 1), 0);
        assert_eq!(locate(text, 2, 1), 4);
        assert_eq!(locate(text, 2, 5), 8);
        // Past the end of a line stays on that line.
        assert_eq!(locate(text, 1, 99), 3);
        // Past the last line lands at the start of the last line; line 0 means line 1.
        assert_eq!(locate(text, 99, 1), 14);
        assert_eq!(locate(text, 0, 0), 0);
        assert_eq!(locate("", 5, 5), 0);
        // CRLF: the carriage return is not part of the line.
        assert_eq!(locate("ab\r\ncd", 1, 99), 2);
        assert_eq!(locate("ab\r\ncd", 2, 2), 5);
    }

    #[test]
    fn a_network_path_is_refused_whatever_mix_of_slashes_it_starts_with() {
        let root = std::env::temp_dir();
        for path in [
            r"\\host\share\a.rs",
            "//host/share/a.rs",
            r"\/host/share/a.rs",
            r"/\host\share\a.rs",
            r"\\?\C:\a.rs",
            r"\\.\pipe\x",
        ] {
            let reference = PathRef {
                path: path.into(),
                line: None,
                column: None,
            };
            assert_eq!(resolve(&reference, &root), None, "{path}");
        }
    }

    #[test]
    fn a_windows_path_with_spaces_is_one_reference() {
        let text = r"At C:\Users\Jane Doe\my proj\run.ps1:3 char:1";
        let found = ref_at(text, 20).expect("a reference");
        assert_eq!(found.path, r"C:\Users\Jane Doe\my proj\run.ps1");
        assert_eq!((found.line, found.column), (Some(3), None));
        let text = r"error: C:\Users\Jane Doe\src\main.rs:3:1 expected";
        let found = ref_at(text, 30).expect("a reference");
        assert_eq!(found.path, r"C:\Users\Jane Doe\src\main.rs");
        assert_eq!((found.line, found.column), (Some(3), Some(1)));
        // A path without spaces still reads as before, and a dot in a folder name is not the end.
        let found = ref_at(r"C:\Users\Jane.Doe\a b\x.rs:7", 3).expect("a reference");
        assert_eq!(found.path, r"C:\Users\Jane.Doe\a b\x.rs");
        assert_eq!(found.line, Some(7));
        assert_eq!(ref_at(r"nothing here", 3), None);
    }
}
