//! Workspace file listing and find-in-files, with the exclusions people expect.

use crate::find;
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
    pub preview: String,
}

#[derive(Clone, Debug, Default)]
pub struct Results {
    pub hits: Vec<Hit>,
    pub files_scanned: usize,
    pub truncated: bool,
    pub cancelled: bool,
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

/// Files under `root` (excluding build and VCS folders), for Quick Open.
pub fn list_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(root, &mut files, &AtomicBool::new(false));
    files
}

/// Search every text file under `root` for `query`. Checks `cancel` so a newer query can
/// stop an older one.
pub fn search(root: &Path, query: &str, case_sensitive: bool, cancel: &AtomicBool) -> Results {
    let mut results = Results::default();
    if query.is_empty() {
        return results;
    }
    let mut files = Vec::new();
    walk(root, &mut files, cancel);
    for path in files {
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
            let matches = find::find_all(line, query, case_sensitive);
            if let Some(first) = matches.first() {
                results.hits.push(Hit {
                    path: path.clone(),
                    line: index + 1,
                    column: first.start + 1,
                    preview: line.trim().chars().take(160).collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn finds_lines_with_positions_and_skips_junk() {
        let root = tree();
        let cancel = AtomicBool::new(false);
        let insensitive = search(&root, "todo", false, &cancel);
        let lines: Vec<(usize, usize)> = insensitive
            .hits
            .iter()
            .map(|h| (h.line, h.column))
            .collect();
        assert_eq!(lines, vec![(2, 4), (3, 5)]);
        assert!(insensitive.hits.iter().all(|h| h.path.ends_with("a.rs")));
        assert_eq!(insensitive.hits[0].preview, "// TODO: later");

        let sensitive = search(&root, "todo", true, &cancel);
        assert_eq!(sensitive.hits.len(), 1);
        assert!(search(&root, "", true, &cancel).hits.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_cancelled_search_stops() {
        let root = tree();
        let cancel = AtomicBool::new(true);
        let results = search(&root, "todo", false, &cancel);
        assert!(results.hits.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
