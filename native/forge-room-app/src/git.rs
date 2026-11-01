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
    pub upstream: Option<String>,
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
    // Credentials come from a helper (e.g. Git Credential Manager); never block on a console prompt.
    command
        .current_dir(root)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
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
        } else if let Some(rest) = record.strip_prefix("# branch.upstream ") {
            status.upstream = Some(rest.to_string());
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

// ---- Sync -----------------------------------------------------------------------------------

/// Download remote changes without touching the working tree.
pub fn fetch(root: &Path) -> Result<String, String> {
    run(root, &["fetch", "--all", "--prune"]).map(|_| "Fetched".into())
}

/// Fast-forward or merge the upstream into the current branch.
pub fn pull(root: &Path) -> Result<String, String> {
    run(root, &["pull", "--no-edit"]).map(|out| first_line(&out, "Pulled"))
}

/// Push the current branch; publishes it (sets the upstream) when it has none yet.
pub fn push(root: &Path, has_upstream: bool) -> Result<String, String> {
    if has_upstream {
        run(root, &["push"]).map(|_| "Pushed".into())
    } else {
        run(root, &["push", "--set-upstream", "origin", "HEAD"]).map(|_| "Published branch".into())
    }
}

fn first_line(text: &str, fallback: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

// ---- Branches -------------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Branch {
    pub name: String,
    pub current: bool,
    pub remote: bool,
    pub upstream: Option<String>,
}

pub fn branches(root: &Path) -> Result<Vec<Branch>, String> {
    let output = run(
        root,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(HEAD)\x1f%(refname)\x1f%(refname:short)\x1f%(upstream:short)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\x1f').collect();
            if parts.len() < 4 || parts[1].ends_with("/HEAD") {
                return None;
            }
            Some(Branch {
                name: parts[2].to_string(),
                current: parts[0] == "*",
                remote: parts[1].starts_with("refs/remotes/"),
                upstream: Some(parts[3].to_string()).filter(|value| !value.is_empty()),
            })
        })
        .collect())
}

/// Switch branch. A remote branch (`origin/feature`) becomes a local tracking branch.
pub fn checkout(root: &Path, name: &str, remote: bool) -> Result<(), String> {
    if remote {
        run(root, &["checkout", "--track", name]).map(|_| ())
    } else {
        run(root, &["checkout", name]).map(|_| ())
    }
}

pub fn create_branch(root: &Path, name: &str) -> Result<(), String> {
    run(root, &["checkout", "-b", name]).map(|_| ())
}

// ---- Discard, stash, undo ---------------------------------------------------------------------

/// Throw away working-tree changes to one file. Untracked files are deleted.
pub fn discard(root: &Path, change: &Change) -> Result<(), String> {
    if change.is_untracked() {
        run(root, &["clean", "-f", "--", &change.path]).map(|_| ())
    } else {
        run(
            root,
            &["restore", "--worktree", "--source=HEAD", "--", &change.path],
        )
        .or_else(|_| run(root, &["checkout", "--", &change.path]))
        .map(|_| ())
    }
}

pub fn stash_push(root: &Path, message: &str) -> Result<String, String> {
    let mut args = vec!["stash", "push", "--include-untracked"];
    if !message.trim().is_empty() {
        args.push("-m");
        args.push(message);
    }
    run(root, &args).map(|out| first_line(&out, "Stashed"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stash {
    pub index: usize,
    pub message: String,
}

pub fn stash_list(root: &Path) -> Vec<Stash> {
    run(root, &["stash", "list", "--format=%gs"])
        .map(|out| {
            out.lines()
                .enumerate()
                .map(|(index, message)| Stash {
                    index,
                    message: message.to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn stash_pop(root: &Path, index: usize) -> Result<String, String> {
    let reference = format!("stash@{{{index}}}");
    run(root, &["stash", "pop", &reference]).map(|out| first_line(&out, "Applied stash"))
}

/// Undo the last commit but keep its changes staged.
pub fn undo_last_commit(root: &Path) -> Result<(), String> {
    run(root, &["reset", "--soft", "HEAD~1"]).map(|_| ())
}

/// Replace the last commit with the staged changes (and a new message when given).
pub fn amend(root: &Path, message: &str) -> Result<String, String> {
    if message.trim().is_empty() {
        run(root, &["commit", "--amend", "--no-edit"])?;
    } else {
        run(root, &["commit", "--amend", "-m", message])?;
    }
    Ok(run(root, &["rev-parse", "--short", "HEAD"])?
        .trim()
        .to_string())
}

// ---- History, graph and blame ------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitInfo {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub time: i64,
    /// Branch, tag and HEAD labels pointing at this commit, as git prints them.
    pub refs: Vec<String>,
    pub subject: String,
}

impl CommitInfo {
    pub fn short(&self) -> &str {
        &self.hash[..self.hash.len().min(7)]
    }
}

const LOG_FORMAT: &str = "--format=%H\x1f%P\x1f%an\x1f%ae\x1f%at\x1f%D\x1f%s\x1e";

fn parse_log(output: &str) -> Vec<CommitInfo> {
    output
        .split('\x1e')
        .filter_map(|record| {
            let parts: Vec<&str> = record.trim_start_matches('\n').split('\x1f').collect();
            if parts.len() < 7 || parts[0].is_empty() {
                return None;
            }
            Some(CommitInfo {
                hash: parts[0].to_string(),
                parents: parts[1].split_whitespace().map(String::from).collect(),
                author: parts[2].to_string(),
                email: parts[3].to_string(),
                time: parts[4].parse().unwrap_or(0),
                refs: parts[5]
                    .split(", ")
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .collect(),
                subject: parts[6].to_string(),
            })
        })
        .collect()
}

/// Recent history of every branch (newest first, topological) for the commit graph.
pub fn log_all(root: &Path, limit: usize) -> Result<Vec<CommitInfo>, String> {
    let limit = format!("--max-count={limit}");
    run(
        root,
        &[
            "log",
            "--all",
            "--topo-order",
            "--decorate=short",
            &limit,
            LOG_FORMAT,
        ],
    )
    .map(|out| parse_log(&out))
}

/// Commits that touched one file, following renames.
pub fn file_log(root: &Path, path: &str, limit: usize) -> Result<Vec<CommitInfo>, String> {
    let limit = format!("--max-count={limit}");
    run(
        root,
        &[
            "log",
            "--follow",
            "--decorate=short",
            &limit,
            LOG_FORMAT,
            "--",
            path,
        ],
    )
    .map(|out| parse_log(&out))
}

/// Full details of one commit: message, files changed and the patch.
pub fn show(root: &Path, hash: &str) -> Result<String, String> {
    run(root, &["show", "--no-color", "--stat", "--patch", hash])
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlameLine {
    pub hash: String,
    pub author: String,
    pub time: i64,
    pub summary: String,
}

impl BlameLine {
    pub fn is_uncommitted(&self) -> bool {
        self.hash.chars().all(|c| c == '0')
    }
}

/// Who last changed each line of a file (index 0 is line 1). Empty for untracked files.
pub fn blame(root: &Path, path: &str) -> Vec<BlameLine> {
    run(root, &["blame", "--porcelain", "--", path])
        .map(|out| parse_blame(&out))
        .unwrap_or_default()
}

pub fn parse_blame(output: &str) -> Vec<BlameLine> {
    use std::collections::HashMap;
    let mut known: HashMap<String, BlameLine> = HashMap::new();
    let mut lines = Vec::new();
    let mut current: Option<String> = None;
    for line in output.lines() {
        if line.starts_with('\t') {
            if let Some(hash) = &current
                && let Some(info) = known.get(hash)
            {
                lines.push(info.clone());
            }
            current = None;
        } else if current.is_none() {
            let hash = line.split_whitespace().next().unwrap_or("").to_string();
            if hash.len() >= 40 {
                known.entry(hash.clone()).or_insert_with(|| BlameLine {
                    hash: hash.clone(),
                    author: String::new(),
                    time: 0,
                    summary: String::new(),
                });
                current = Some(hash);
            }
        } else if let Some(hash) = &current
            && let Some(info) = known.get_mut(hash)
        {
            if let Some(value) = line.strip_prefix("author ") {
                info.author = value.to_string();
            } else if let Some(value) = line.strip_prefix("author-time ") {
                info.time = value.parse().unwrap_or(0);
            } else if let Some(value) = line.strip_prefix("summary ") {
                info.summary = value.to_string();
            }
        }
    }
    lines
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

#[cfg(test)]
mod history_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn available() -> bool {
        run(Path::new("."), &["--version"]).is_ok()
    }

    fn unique(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("forge-room-{name}-{}-{stamp}", std::process::id()))
    }

    /// Run git with a throwaway identity so tests never depend on the machine's config.
    fn git(dir: &Path, args: &[&str]) -> String {
        let mut full = vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ];
        full.extend_from_slice(args);
        run(dir, &full).unwrap_or_else(|error| panic!("git {args:?} failed: {error}"))
    }

    fn repo() -> PathBuf {
        let dir = unique("hist");
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q"]);
        fs::write(dir.join("a.txt"), "one\ntwo\nthree\n").unwrap();
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-q", "-m", "first commit"]);
        dir
    }

    #[test]
    fn parses_blame_porcelain_with_repeated_commits() {
        let hash_a = "a".repeat(40);
        let hash_b = "b".repeat(40);
        let sample = format!(
            "{hash_a} 1 1 2\nauthor Ada\nauthor-time 1700000000\nsummary first\nfilename f\n\tline1\n\
             {hash_a} 2 2\n\tline2\n\
             {hash_b} 3 3 1\nauthor Grace\nauthor-time 1700001000\nsummary second\nfilename f\n\tline3\n"
        );
        let lines = parse_blame(&sample);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].author, "Ada");
        assert_eq!(lines[1].author, "Ada");
        assert_eq!(lines[2].author, "Grace");
        assert_eq!(lines[2].summary, "second");
        assert!(!lines[0].is_uncommitted());
    }

    #[test]
    fn parses_log_records_with_refs_and_parents() {
        let output = "aaaa\x1fbbbb cccc\x1fAda\x1fa@x\x1f100\x1fHEAD -> main, origin/main, tag: v1\x1fMerge it\x1e\nbbbb\x1f\x1fGrace\x1fg@x\x1f50\x1f\x1froot\x1e";
        let commits = parse_log(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].parents, vec!["bbbb", "cccc"]);
        assert_eq!(
            commits[0].refs,
            vec!["HEAD -> main", "origin/main", "tag: v1"]
        );
        assert_eq!(commits[1].parents.len(), 0);
        assert_eq!(commits[1].subject, "root");
    }

    #[test]
    fn branches_discard_stash_undo_and_blame_in_a_real_repository() {
        if !available() {
            return;
        }
        let dir = repo();

        // Blame and file history.
        let blame_lines = blame(&dir, "a.txt");
        assert_eq!(blame_lines.len(), 3);
        assert_eq!(blame_lines[0].author, "Test");
        assert_eq!(file_log(&dir, "a.txt", 10).unwrap().len(), 1);
        assert!(show(&dir, "HEAD").unwrap().contains("first commit"));

        // Branches.
        create_branch(&dir, "feature").unwrap();
        let listed = branches(&dir).unwrap();
        assert!(listed.iter().any(|b| b.name == "feature" && b.current));
        checkout(&dir, "main", false).unwrap();
        git(&dir, &["branch", "-d", "feature"]);
        assert!(!branches(&dir).unwrap().iter().any(|b| b.name == "feature"));

        // Discard a tracked change and an untracked file.
        fs::write(dir.join("a.txt"), "changed\n").unwrap();
        fs::write(dir.join("new.txt"), "x\n").unwrap();
        let status_now = status(&dir).unwrap();
        for change in &status_now.changes {
            discard(&dir, change).unwrap();
        }
        assert_eq!(
            fs::read_to_string(dir.join("a.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "one\ntwo\nthree\n"
        );
        assert!(!dir.join("new.txt").exists());

        // Stash round trip.
        fs::write(dir.join("a.txt"), "stashed\n").unwrap();
        stash_push(&dir, "wip").unwrap();
        assert_eq!(stash_list(&dir).len(), 1);
        assert!(status(&dir).unwrap().changes.is_empty());
        stash_pop(&dir, 0).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("a.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "stashed\n"
        );

        // Commit, amend, undo.
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-q", "-m", "second"]);
        assert_eq!(log_all(&dir, 10).unwrap().len(), 2);
        undo_last_commit(&dir).unwrap();
        assert_eq!(log_all(&dir, 10).unwrap().len(), 1);
        assert!(status(&dir).unwrap().changes[0].is_staged());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn publish_push_fetch_and_pull_against_a_local_remote() {
        if !available() {
            return;
        }
        let remote = unique("remote");
        fs::create_dir_all(&remote).unwrap();
        git(&remote, &["init", "-q", "--bare"]);

        let first = repo();
        git(
            &first,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        assert_eq!(status(&first).unwrap().upstream, None);
        push(&first, false).unwrap(); // publishes the branch
        let status_after = status(&first).unwrap();
        assert!(status_after.upstream.is_some());
        assert_eq!((status_after.ahead, status_after.behind), (0, 0));

        // A second clone pushes a commit; the first sees it as "behind", then pulls it.
        let second = unique("clone");
        run(
            Path::new("."),
            &[
                "clone",
                "-q",
                remote.to_str().unwrap(),
                second.to_str().unwrap(),
            ],
        )
        .unwrap();
        fs::write(second.join("b.txt"), "from clone\n").unwrap();
        git(&second, &["add", "-A"]);
        git(&second, &["commit", "-q", "-m", "from the clone"]);
        push(&second, true).unwrap();

        fetch(&first).unwrap();
        assert_eq!(status(&first).unwrap().behind, 1);
        pull(&first).unwrap();
        assert_eq!(status(&first).unwrap().behind, 0);
        assert!(first.join("b.txt").exists());

        // A local commit makes the first repo "ahead".
        fs::write(first.join("c.txt"), "local\n").unwrap();
        git(&first, &["add", "-A"]);
        git(&first, &["commit", "-q", "-m", "local work"]);
        assert_eq!(status(&first).unwrap().ahead, 1);
        push(&first, true).unwrap();
        assert_eq!(status(&first).unwrap().ahead, 0);

        let graph = log_all(&first, 10).unwrap();
        assert_eq!(graph.len(), 3);
        assert!(graph[0].refs.iter().any(|name| name.contains("main")));
        for path in [remote, first, second] {
            let _ = fs::remove_dir_all(path);
        }
    }
}
