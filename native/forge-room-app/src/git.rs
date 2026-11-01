//! Version control through the `git` command line, like VS Code does. No libgit2 and no extra
//! runtime: it needs `git` on PATH and reports a clear message when it is missing.

use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    /// Path relative to the repository root, with forward slashes.
    pub path: String,
    /// Index (staged) state letter, `.` when unchanged.
    pub index: char,
    /// Working-tree state letter, `.` when unchanged, `?` when untracked.
    pub worktree: char,
}

impl Change {
    pub fn is_staged(&self) -> bool {
        self.index != '.' && self.index != '?'
    }

    pub fn is_unstaged(&self) -> bool {
        self.worktree != '.'
    }

    pub fn is_untracked(&self) -> bool {
        self.worktree == '?'
    }

    pub fn is_conflict(&self) -> bool {
        self.index == 'U'
            || self.worktree == 'U'
            || (self.index == 'A' && self.worktree == 'A')
            || (self.index == 'D' && self.worktree == 'D')
    }

    /// Single letter shown in the list for the staged or unstaged side.
    pub fn letter(&self, staged: bool) -> char {
        if self.is_conflict() {
            return '!';
        }
        let letter = if staged { self.index } else { self.worktree };
        if letter == '?' { 'U' } else { letter }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Status {
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub changes: Vec<Change>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HunkKind {
    Added,
    Modified,
    Deleted,
}

/// A changed region of the working file compared to HEAD, in 1-based new-file line numbers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hunk {
    pub kind: HunkKind,
    pub start: usize,
    pub len: usize,
}

impl Hunk {
    pub fn covers(&self, line: usize) -> bool {
        match self.kind {
            // A deletion is drawn on the line that now follows the removed text.
            HunkKind::Deleted => line == self.start.max(1),
            _ => line >= self.start && line < self.start + self.len,
        }
    }
}

pub fn run(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.current_dir(root).args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flash from a GUI app
    }
    let output = command.output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "Git is not installed or not on PATH".to_string()
        } else {
            format!("could not run git: {error}")
        }
    })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if message.is_empty() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            message
        })
    }
}

pub fn is_repo(root: &Path) -> bool {
    run(root, &["rev-parse", "--is-inside-work-tree"])
        .map(|out| out.trim() == "true")
        .unwrap_or(false)
}

pub fn status(root: &Path) -> Result<Status, String> {
    let output = run(
        root,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=all",
        ],
    )?;
    Ok(parse_status(&output))
}

pub fn parse_status(output: &str) -> Status {
    let mut status = Status::default();
    let mut records = output.split('\0').filter(|record| !record.is_empty());
    while let Some(record) = records.next() {
        if let Some(rest) = record.strip_prefix("# branch.head ") {
            if rest != "(detached)" {
                status.branch = Some(rest.to_string());
            }
        } else if let Some(rest) = record.strip_prefix("# branch.ab ") {
            let mut parts = rest.split(' ');
            status.ahead = parts
                .next()
                .and_then(|value| value.trim_start_matches('+').parse().ok())
                .unwrap_or(0);
            status.behind = parts
                .next()
                .and_then(|value| value.trim_start_matches('-').parse().ok())
                .unwrap_or(0);
        } else if let Some(path) = record.strip_prefix("? ") {
            status.changes.push(Change {
                path: path.to_string(),
                index: '.',
                worktree: '?',
            });
        } else if record.starts_with("1 ") || record.starts_with("2 ") || record.starts_with("u ") {
            let kind = record.as_bytes()[0];
            let fields = match kind {
                b'1' => 9,
                b'2' => 10,
                _ => 11,
            };
            let parts: Vec<&str> = record.splitn(fields, ' ').collect();
            if parts.len() == fields {
                let mut state = parts[1].chars();
                status.changes.push(Change {
                    path: parts[fields - 1].to_string(),
                    index: state.next().unwrap_or('.'),
                    worktree: state.next().unwrap_or('.'),
                });
            }
            if kind == b'2' {
                records.next(); // the original path of a rename/copy
            }
        }
    }
    status
        .changes
        .sort_by_key(|change| change.path.to_lowercase());
    status
}

pub fn stage(root: &Path, paths: &[String]) -> Result<(), String> {
    let mut args = vec!["add", "-A", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(root, &args).map(|_| ())
}

pub fn unstage(root: &Path, paths: &[String]) -> Result<(), String> {
    let mut args = vec!["reset", "-q", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(root, &args).map(|_| ())
}

pub fn commit(root: &Path, message: &str) -> Result<String, String> {
    run(root, &["commit", "-m", message])?;
    Ok(run(root, &["rev-parse", "--short", "HEAD"])?
        .trim()
        .to_string())
}

/// Unified diff of one file, staged (`--cached`) or working tree.
pub fn diff(root: &Path, path: &str, staged: bool) -> Result<String, String> {
    let mut args = vec!["diff", "--no-color"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(path);
    run(root, &args)
}

/// Changed regions of a tracked file compared to HEAD (staged and unstaged together).
pub fn file_hunks(root: &Path, path: &str) -> Vec<Hunk> {
    run(root, &["diff", "HEAD", "--no-color", "-U0", "--", path])
        .map(|output| parse_hunks(&output))
        .unwrap_or_default()
}

pub fn parse_hunks(diff: &str) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    for line in diff.lines() {
        let Some(rest) = line.strip_prefix("@@ -") else {
            continue;
        };
        let Some((old, rest)) = rest.split_once(" +") else {
            continue;
        };
        let new = rest.split(" @@").next().unwrap_or("");
        let (_, old_len) = range(old);
        let (start, len) = range(new);
        let kind = if len == 0 {
            HunkKind::Deleted
        } else if old_len == 0 {
            HunkKind::Added
        } else {
            HunkKind::Modified
        };
        hunks.push(Hunk { kind, start, len });
    }
    hunks
}

/// `a,b` or `a` (length 1) from a hunk header.
fn range(text: &str) -> (usize, usize) {
    match text.split_once(',') {
        Some((start, len)) => (start.parse().unwrap_or(0), len.parse().unwrap_or(0)),
        None => (text.parse().unwrap_or(0), 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn git_available() -> bool {
        run(Path::new("."), &["--version"]).is_ok()
    }

    fn temp_repo() -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("forge-room-vcs-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        run(&dir, &["init", "-q"]).unwrap();
        dir
    }

    fn commit_as_test(dir: &Path, message: &str) {
        run(
            dir,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "-m",
                message,
            ],
        )
        .unwrap();
    }

    #[test]
    fn parses_porcelain_v2_status() {
        let sample = "# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 .M N... 100644 100644 100644 aaa bbb src/main.rs\0\
1 A. N... 000000 100644 100644 000 ccc new.txt\0\
2 R. N... 100644 100644 100644 aaa bbb R100 renamed.txt\0old.txt\0\
? notes.md\0";
        let status = parse_status(sample);
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!((status.ahead, status.behind), (2, 1));
        let paths: Vec<&str> = status.changes.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, ["new.txt", "notes.md", "renamed.txt", "src/main.rs"]);
        let main = status
            .changes
            .iter()
            .find(|c| c.path == "src/main.rs")
            .unwrap();
        assert!(!main.is_staged() && main.is_unstaged());
        let new = status.changes.iter().find(|c| c.path == "new.txt").unwrap();
        assert!(new.is_staged() && !new.is_unstaged());
        let notes = status
            .changes
            .iter()
            .find(|c| c.path == "notes.md")
            .unwrap();
        assert!(notes.is_untracked());
        assert_eq!(notes.letter(false), 'U');
    }

    #[test]
    fn parses_hunk_headers() {
        let diff = "@@ -1,0 +2,3 @@\n+a\n+b\n+c\n@@ -10,2 +13,2 @@\n-x\n@@ -20,3 +22,0 @@\n";
        let hunks = parse_hunks(diff);
        assert_eq!(hunks.len(), 3);
        assert_eq!(
            hunks[0],
            Hunk {
                kind: HunkKind::Added,
                start: 2,
                len: 3
            }
        );
        assert_eq!(hunks[1].kind, HunkKind::Modified);
        assert_eq!(hunks[2].kind, HunkKind::Deleted);
        assert!(hunks[0].covers(4) && !hunks[0].covers(5));
        assert!(hunks[2].covers(22));
    }

    #[test]
    fn stage_commit_and_diff_in_a_real_repository() {
        if !git_available() {
            return;
        }
        let dir = temp_repo();
        fs::write(dir.join("a.txt"), "one\ntwo\n").unwrap();
        assert!(is_repo(&dir));

        let before = status(&dir).unwrap();
        assert_eq!(before.changes.len(), 1);
        assert!(before.changes[0].is_untracked());

        stage(&dir, &["a.txt".to_string()]).unwrap();
        let staged = status(&dir).unwrap();
        assert!(staged.changes[0].is_staged());
        unstage(&dir, &["a.txt".to_string()]).unwrap();
        assert!(status(&dir).unwrap().changes[0].is_untracked());

        stage(&dir, &["a.txt".to_string()]).unwrap();
        commit_as_test(&dir, "first");
        assert!(status(&dir).unwrap().changes.is_empty());

        fs::write(dir.join("a.txt"), "one\nTWO\nthree\n").unwrap();
        let changed = status(&dir).unwrap();
        assert!(changed.changes[0].is_unstaged() && !changed.changes[0].is_staged());
        assert!(diff(&dir, "a.txt", false).unwrap().contains("+TWO"));
        let hunks = file_hunks(&dir, "a.txt");
        assert!(hunks.iter().any(|hunk| hunk.covers(2)));
        fs::remove_dir_all(dir).unwrap();
    }
}
