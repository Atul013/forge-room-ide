//! Workspace file listing and find-in-files, with the exclusions people expect.

use crate::{find, git};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Folders never searched or listed.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".impeccable",
    "artifacts",
    ".vs",
    "dist",
];
const MAX_FILE_BYTES: u64 = 1024 * 1024;
pub const MAX_HITS: usize = 2_000;
const MAX_LISTED_FILES: usize = 20_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hit {
    pub path: PathBuf,
    /// 1-based line.
    pub line: usize,
    /// 1-based column in characters.
    pub column: usize,
    /// Characters in the first match.
    pub length: usize,
    pub preview: String,
    /// The matches inside `preview`, as character ranges, for highlighting.
    pub marks: Vec<std::ops::Range<usize>>,
}

#[derive(Clone, Debug, Default)]
pub struct Results {
    pub hits: Vec<Hit>,
    pub files_scanned: usize,
    pub truncated: bool,
    pub cancelled: bool,
    /// Why the query could not run (an invalid regular expression).
    pub error: Option<String>,
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>, cancel: &AtomicBool) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if cancel.load(Ordering::Relaxed) || out.len() >= MAX_LISTED_FILES {
            return;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => {
                if !SKIP_DIRS.contains(&name.as_str()) {
                    walk(&path, out, cancel);
                }
            }
            Ok(kind) if kind.is_file() => out.push(path),
            _ => {}
        }
    }
}

/// The files Git considers part of the project: tracked ones and untracked ones that are not
/// ignored. This follows `.gitignore` exactly (a `.venv`, `build` or `out` folder stays out, a
/// tracked `dist/` stays in), which a fixed skip list cannot. `None` outside a repository or
/// when Git is unavailable.
fn git_files(root: &Path, cancel: &AtomicBool) -> Option<Vec<PathBuf>> {
    // Only a folder that is itself the top of a repository is listed through Git; a subfolder (or a
    // folder merely sitting inside some outer repository) is walked instead.
    let top = git::run(root, &["rev-parse", "--show-toplevel"]).ok()?;
    let (top, here) = (
        fs::canonicalize(top.trim()).ok()?,
        fs::canonicalize(root).ok()?,
    );
    if top != here {
        return None;
    }
    let output = git::run(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )
    .ok()?;
    let mut files = Vec::new();
    for path in output
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| root.join(path))
    {
        // A link is not followed: its target may be outside the folder (or on a network share that
        // answering `is_file` would contact).
        if is_link(&path) {
            continue;
        }
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            // A submodule or a nested repository: Git lists only the folder, so walk it. A tracked
            // file that was deleted is neither and is dropped.
            walk(&path, &mut files, cancel);
        }
    }
    files.sort();
    files.dedup();
    files.truncate(MAX_LISTED_FILES);
    Some(files)
}

/// Whether `path` itself is a symbolic link or a junction.
fn is_link(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

/// Every project file: from Git's own view when the folder is a repository, otherwise by walking
/// the folder and skipping the usual build and VCS directories.
fn collect(root: &Path, cancel: &AtomicBool) -> Vec<PathBuf> {
    git_files(root, cancel).unwrap_or_else(|| {
        let mut files = Vec::new();
        walk(root, &mut files, cancel);
        files
    })
}

/// Files under `root`, for Quick Open.
pub fn list_files(root: &Path) -> Vec<PathBuf> {
    collect(root, &AtomicBool::new(false))
}

/// Search every text file under `root` for `query`. Checks `cancel` so a newer query can
/// stop an older one.
pub fn search(root: &Path, query: &str, mode: find::Mode, cancel: &AtomicBool) -> Results {
    let mut results = Results::default();
    if query.is_empty() {
        return results;
    }
    let matcher = match find::Matcher::new(query, mode) {
        Ok(matcher) => matcher,
        Err(error) => {
            results.error = Some(error);
            return results;
        }
    };
    for path in collect(root, cancel) {
        if cancel.load(Ordering::Relaxed) {
            results.cancelled = true;
            return results;
        }
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > MAX_FILE_BYTES {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue; // binary or not UTF-8
        };
        results.files_scanned += 1;
        for (index, line) in text.lines().enumerate() {
            let matches = matcher.find_all(line);
            if let Some(first) = matches.first() {
                // The preview drops leading whitespace and is cut short: the marks follow it.
                let lead = line.chars().take_while(|c| c.is_whitespace()).count();
                let preview: String = line.trim().chars().take(160).collect();
                let shown = preview.chars().count();
                let marks = matches
                    .iter()
                    .filter_map(|range| {
                        // A match that begins in the dropped whitespace keeps the part that is shown.
                        let start = range.start.saturating_sub(lead);
                        let end = range.end.saturating_sub(lead).min(shown);
                        (start < end).then_some(start..end)
                    })
                    .collect();
                results.hits.push(Hit {
                    path: path.clone(),
                    line: index + 1,
                    column: first.start + 1,
                    length: first.end - first.start,
                    preview,
                    marks,
                });
                if results.hits.len() >= MAX_HITS {
                    results.truncated = true;
                    return results;
                }
            }
        }
    }
    results
}

/// What a replace across files did.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Replaced {
    pub files: usize,
    pub replacements: usize,
    /// Files left alone, with the reason: open with unsaved changes, changed meanwhile, unwritable.
    pub skipped: Vec<(PathBuf, String)>,
    pub error: Option<String>,
    pub cancelled: bool,
}

/// Replace every match of `query` in every text file under `root` with `replacement` (literal text:
/// no capture groups). Each file keeps its encoding and line endings and is written atomically; a
/// file in `protected` (open with unsaved changes) or one that changed on disk while this ran is
/// left alone and listed. Lines are matched one at a time, as Search in Files shows them.
pub fn replace_in_files(
    root: &Path,
    query: &str,
    mode: find::Mode,
    replacement: &str,
    protected: &[PathBuf],
    cancel: &AtomicBool,
) -> Replaced {
    let mut report = Replaced::default();
    let matcher = match find::Matcher::new(query, mode) {
        Ok(matcher) if !query.is_empty() => matcher,
        Ok(_) => return report,
        Err(error) => {
            report.error = Some(error);
            return report;
        }
    };
    let files = collect(root, cancel);
    if files.len() >= MAX_LISTED_FILES {
        // The list stops there, so a replace would quietly cover only part of the project.
        report.error = Some(format!(
            "This folder has {MAX_LISTED_FILES} files or more, more than Replace All covers; \
             nothing was changed"
        ));
        return report;
    }
    let root_canonical = crate::tabs::normalize(root);
    for path in files {
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            return report;
        }
        // A file reached through a link in one of its folders may live outside this one.
        if !crate::tabs::normalize(&path).starts_with(&root_canonical) {
            continue;
        }
        if fs::metadata(&path)
            .map(|m| m.len() > MAX_FILE_BYTES)
            .unwrap_or(true)
        {
            continue;
        }
        // A link is never followed: its target may be outside the folder, or be visited twice.
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            continue;
        }
        let mut session = crate::document::DocumentSession::default();
        if session.open(&path).is_err() {
            continue; // binary, not a supported encoding, or unreadable
        }
        // Search in Files only reads UTF-8, so a UTF-16 file was never shown as a result.
        if matches!(
            session.encoding(),
            crate::document::Encoding::Utf16Le | crate::document::Encoding::Utf16Be
        ) {
            continue;
        }
        let mut count = 0;
        let mut gave_up = false;
        let replaced: Vec<String> = session
            .text()
            .split('\n')
            .map(|line| {
                let (matches, stopped) = matcher.find_all_checked(line);
                gave_up |= stopped;
                if matches.is_empty() {
                    return line.to_string();
                }
                count += matches.len();
                find::replace_all(line, &matches, replacement)
            })
            .collect();
        if count == 0 {
            continue;
        }
        let canonical = crate::tabs::normalize(&path);
        if protected
            .iter()
            .any(|p| *p == canonical || *p == path || crate::tabs::normalize(p) == canonical)
        {
            report
                .skipped
                .push((path, "it is open with unsaved changes".into()));
            continue;
        }
        if gave_up {
            report
                .skipped
                .push((path, "the pattern was too complex to run on it".into()));
            continue;
        }
        *session.text_mut() = replaced.join("\n");
        match session.save() {
            Ok(()) => {
                report.files += 1;
                report.replacements += count;
            }
            Err(error) => report.skipped.push((path, error.to_string())),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact() -> find::Mode {
        find::Mode {
            case_sensitive: true,
            ..find::Mode::default()
        }
    }

    fn tree() -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("forge-room-search-{}-{stamp}", std::process::id()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(
            root.join("src/a.rs"),
            "fn main() {}\n// TODO: later\nlet todo = 1;\n",
        )
        .unwrap();
        fs::write(root.join("notes.txt"), "nothing here\n").unwrap();
        fs::write(root.join("target/skip.rs"), "TODO in build output\n").unwrap();
        fs::write(root.join(".git/config"), "TODO in git\n").unwrap();
        fs::write(
            root.join("blob.bin"),
            [0xFF, 0xFE, 0x00, 0x54, 0x4F, 0x44, 0x4F],
        )
        .unwrap();
        root
    }

    #[test]
    fn replacing_in_files_keeps_encoding_and_line_endings_and_skips_protected_files() {
        let root = tree();
        let crlf = root.join("crlf.txt");
        fs::write(&crlf, "one TODO\r\ntwo\r\n").unwrap();
        let utf16 = root.join("wide.txt");
        let wide_bytes = |text: &str| {
            let mut bytes = vec![0xFF, 0xFE];
            for unit in text.encode_utf16() {
                bytes.extend(unit.to_le_bytes());
            }
            bytes
        };
        fs::write(&utf16, wide_bytes("a TODO b\n")).unwrap();
        let busy = root.join("busy.txt");
        fs::write(&busy, "TODO busy\n").unwrap();
        let protected = vec![crate::tabs::normalize(&busy)];
        let never = AtomicBool::new(false);
        let report = replace_in_files(&root, "TODO", exact(), "DONE", &protected, &never);
        assert_eq!(fs::read(&crlf).unwrap(), b"one DONE\r\ntwo\r\n");
        // UTF-16 files are not searched, so they are not replaced either.
        assert_eq!(fs::read(&utf16).unwrap(), wide_bytes("a TODO b\n"));
        assert_eq!(fs::read_to_string(&busy).unwrap(), "TODO busy\n");
        let source = fs::read_to_string(root.join("src/a.rs")).unwrap();
        assert_eq!(
            source.replace("\r\n", "\n"),
            "fn main() {}\n// DONE: later\nlet todo = 1;\n"
        );
        // Build output is never touched.
        let skipped = fs::read_to_string(root.join("target/skip.rs")).unwrap();
        assert!(skipped.contains("TODO"));
        assert_eq!((report.files, report.replacements), (2, 2));
        assert_eq!(report.skipped.len(), 1);
        let bad = find::Mode {
            regex: true,
            ..exact()
        };
        assert!(
            replace_in_files(&root, "(", bad, "x", &[], &never)
                .error
                .is_some()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lists_files_without_build_or_vcs_folders() {
        let root = tree();
        let names: Vec<String> = list_files(&root)
            .iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert!(names.contains(&"src/a.rs".to_string()));
        assert!(
            !names
                .iter()
                .any(|name| name.starts_with("target") || name.starts_with(".git"))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_repository_is_listed_the_way_gitignore_says() {
        if git::run(Path::new("."), &["--version"]).is_err() {
            return;
        }
        let root = tree();
        // `tree()` has a fake .git folder; make this a real repository instead.
        fs::remove_dir_all(root.join(".git")).unwrap();
        fs::remove_dir_all(root.join("target")).unwrap();
        git::run(&root, &["init", "-q"]).unwrap();
        fs::write(root.join(".gitignore"), ".venv/\nbuild/\n*.log\n").unwrap();
        for dir in [".venv", "build", "dist"] {
            fs::create_dir_all(root.join(dir)).unwrap();
            fs::write(root.join(dir).join("x.txt"), "x\n").unwrap();
        }
        fs::write(root.join("debug.log"), "log\n").unwrap();
        fs::write(root.join("gone.txt"), "gone\n").unwrap();
        // An untracked nested repository: Git lists only its folder.
        fs::create_dir_all(root.join("nested")).unwrap();
        git::run(&root.join("nested"), &["init", "-q"]).unwrap();
        fs::write(root.join("nested/n.txt"), "nested\n").unwrap();
        git::run(&root, &["add", "dist/x.txt", "gone.txt", "notes.txt"]).unwrap();
        fs::remove_file(root.join("gone.txt")).unwrap();

        let names: Vec<String> = list_files(&root)
            .iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        for wanted in [
            "src/a.rs",
            "notes.txt",
            ".gitignore",
            "dist/x.txt",
            "nested/n.txt",
        ] {
            assert!(
                names.contains(&wanted.to_string()),
                "{wanted} missing in {names:?}"
            );
        }
        for unwanted in [".venv/x.txt", "build/x.txt", "debug.log", "gone.txt"] {
            assert!(!names.contains(&unwanted.to_string()), "{unwanted} listed");
        }
        assert!(
            !names.iter().any(|n| n.starts_with("nested/.git")),
            "{names:?}"
        );
        // Search sees exactly the same files.
        let found = search(&root, "x", exact(), &AtomicBool::new(false));
        assert!(
            found
                .hits
                .iter()
                .all(|h| !h.path.starts_with(root.join("build")))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_lines_with_positions_and_skips_junk() {
        let root = tree();
        let cancel = AtomicBool::new(false);
        let insensitive = search(&root, "todo", find::Mode::default(), &cancel);
        let lines: Vec<(usize, usize)> = insensitive
            .hits
            .iter()
            .map(|h| (h.line, h.column))
            .collect();
        assert_eq!(lines, vec![(2, 4), (3, 5)]);
        assert!(insensitive.hits.iter().all(|h| h.path.ends_with("a.rs")));
        assert_eq!(insensitive.hits[0].preview, "// TODO: later");

        let sensitive = search(&root, "todo", exact(), &cancel);
        assert_eq!(sensitive.hits.len(), 1);
        assert!(search(&root, "", exact(), &cancel).hits.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn regex_and_whole_word_searches_mark_their_matches() {
        let root = tree();
        let cancel = AtomicBool::new(false);
        let regex = find::Mode {
            regex: true,
            ..find::Mode::default()
        };
        let found = search(&root, r"t.do\b", regex, &cancel);
        assert_eq!(found.hits.len(), 2);
        // `marks` are inside the trimmed preview, and `length` is the real match length.
        let first = &found.hits[0];
        assert_eq!(first.preview, "// TODO: later");
        assert_eq!(first.marks, vec![3..7]);
        assert_eq!(first.length, 4);
        // A match that starts in the leading whitespace still marks the visible part.
        fs::write(root.join("lead.txt"), "    foo bar\n").unwrap();
        let leading = search(&root, r"\s+foo", regex, &cancel);
        assert_eq!(leading.hits.len(), 1);
        assert_eq!(leading.hits[0].preview, "foo bar");
        assert_eq!(leading.hits[0].marks, vec![0..3]);
        // A bad pattern is reported, not searched.
        let bad = search(&root, "(", regex, &cancel);
        assert!(bad.error.is_some() && bad.hits.is_empty());
        // Whole word skips `todo` inside longer words.
        let words = find::Mode {
            whole_word: true,
            ..find::Mode::default()
        };
        assert_eq!(search(&root, "tod", words, &cancel).hits.len(), 0);
        assert_eq!(search(&root, "todo", words, &cancel).hits.len(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_cancelled_search_stops() {
        let root = tree();
        let cancel = AtomicBool::new(true);
        let results = search(&root, "todo", find::Mode::default(), &cancel);
        assert!(results.hits.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
