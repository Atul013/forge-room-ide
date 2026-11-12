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

/// How long a Git process may run: reads are quick, so a read that is still going after half a
/// minute is stuck (a stalled share, a hung helper); changes and network work get far longer.
fn time_limit(args: &[&str]) -> std::time::Duration {
    const READS: &[&str] = &[
        "status",
        "diff",
        "log",
        "blame",
        "show",
        "branch",
        "stash",
        "rev-parse",
        "ls-files",
        "for-each-ref",
        "tag",
        "check-ref-format",
        "rev-list",
        "cat-file",
    ];
    let seconds = match args.first() {
        Some(name) if READS.contains(name) => 30,
        _ => 15 * 60,
    };
    std::time::Duration::from_secs(seconds)
}

/// `command.output()` with a deadline: a process that outlives it is killed and reported.
fn output_within(
    command: &mut Command,
    limit: std::time::Duration,
) -> std::io::Result<std::process::Output> {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(pipe) = &mut out_pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(pipe) = &mut err_pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "git took too long and was stopped",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    };
    Ok(std::process::Output {
        status,
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    })
}

pub fn run(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    // Credentials come from a helper (e.g. Git Credential Manager); never block on a console prompt.
    // `core.fsmonitor` is switched off because a repository's own config could name a program there
    // that Git would then run on every `status`.
    command
        .current_dir(root)
        .args(["-c", "core.fsmonitor=false"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        // Reads such as `git status` otherwise refresh the index and take its lock, which could
        // make a stage or commit running at the same moment fail with "index.lock exists".
        .env("GIT_OPTIONAL_LOCKS", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flash from a GUI app
    }
    let output = output_within(&mut command, time_limit(args)).map_err(|error| {
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

/// The top folder of the repository that `root` is inside, when that is a different folder (the
/// user opened a subfolder of a bigger repository). Git reports paths from the top folder, so every
/// Git command has to run from there.
pub fn toplevel(root: &Path) -> Option<std::path::PathBuf> {
    let out = run(root, &["rev-parse", "--show-toplevel"]).ok()?;
    let top = crate::tabs::normalize(Path::new(out.trim()));
    (top != crate::tabs::normalize(root)).then_some(top)
}

pub fn is_repo(root: &Path) -> bool {
    run(root, &["rev-parse", "--is-inside-work-tree"])
        .map(|out| out.trim() == "true")
        .unwrap_or(false)
}

/// Everything the Source Control view shows about one repository, read in one go. Gathering it takes
/// several Git processes, so the app runs [`snapshot`] on a worker thread.
pub struct Snapshot {
    pub root: std::path::PathBuf,
    pub is_repo: bool,
    /// A merge, cherry-pick or revert waiting to be finished or aborted.
    pub operation: Option<Operation>,
    pub status: Result<Status, String>,
    pub branch: Option<String>,
    pub commits: Vec<CommitInfo>,
    pub stashes: usize,
}

pub fn snapshot(root: &Path) -> Snapshot {
    let mut snapshot = Snapshot {
        root: root.to_path_buf(),
        is_repo: is_repo(root),
        operation: None,
        status: Err("not a repository".into()),
        branch: None,
        commits: Vec::new(),
        stashes: 0,
    };
    if !snapshot.is_repo {
        return snapshot;
    }
    snapshot.operation = in_progress(root);
    snapshot.status = status(root);
    snapshot.branch = match &snapshot.status {
        Ok(status) => status
            .branch
            .clone()
            .or_else(|| crate::workspace::git_branch(root)),
        Err(_) => crate::workspace::git_branch(root),
    };
    snapshot.commits = log_all(root, 80).unwrap_or_default();
    snapshot.stashes = stash_list(root).len();
    snapshot
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
    // Finishing a merge, cherry-pick or revert: with no message typed, use the one Git prepared
    // (the picked commit's own message, the revert text, the merge message).
    if message.trim().is_empty() && in_progress(root).is_some() {
        run(root, &["commit", "--no-edit"])?;
    } else {
        run(root, &["commit", "-m", message])?;
    }
    Ok(run(root, &["rev-parse", "--short", "HEAD"])?
        .trim()
        .to_string())
}

/// Unified diff of one file, staged (`--cached`) or working tree.
pub fn diff(root: &Path, path: &str, staged: bool) -> Result<String, String> {
    let mut args = vec!["diff", "--no-color", "--no-ext-diff", "--no-textconv"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(path);
    run(root, &args)
}

/// Changed regions of a tracked file compared to HEAD (staged and unstaged together).
/// Like [`run`], with `input` written to the command's standard input.
fn run_with_input(root: &Path, args: &[&str], input: &[u8]) -> Result<String, String> {
    use std::io::Write;
    use std::process::Stdio;
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args(["-c", "core.fsmonitor=false"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not run git: {error}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        // A write error shows up as git's own complaint about the patch below.
        let _ = stdin.write_all(input);
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not run git: {error}"))?;
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

/// The raw bytes Git prints for `args`. A patch must reach `git apply` unchanged, which text
/// conversion would not guarantee for a file that is not UTF-8.
fn run_bytes(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args(["-c", "core.fsmonitor=false"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = output_within(&mut command, time_limit(args))
        .map_err(|error| format!("could not run git: {error}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// The change of `path` that is not staged yet (working tree against the index) which covers
/// `line` (1-based, in the file as it is now), as a patch that applies on its own. A deleted
/// block is covered by the line just above it, the line the gutter marks it on.
///
/// It refuses what could do more than the clicked bar shows: a file that is new or deleted as a
/// whole (reverting would delete it for good), and a change that overlaps something already staged
/// (the gutter draws staged and unstaged changes together, so the bar might mean another change).
pub fn unstaged_hunk_patch(root: &Path, path: &str, line: usize) -> Result<Vec<u8>, String> {
    let diff = run_bytes(
        root,
        &[
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "-U0",
            "--",
            path,
        ],
    )?;
    let mut header: Vec<u8> = Vec::new();
    let mut chosen: Option<(Vec<u8>, Hunk)> = None;
    let mut current: Option<(Vec<u8>, Option<Hunk>)> = None;
    let finish = |current: &mut Option<(Vec<u8>, Option<Hunk>)>,
                  chosen: &mut Option<(Vec<u8>, Hunk)>| {
        if let Some((text, Some(hunk))) = current.take()
            && chosen.is_none()
            && hunk.covers(line)
        {
            *chosen = Some((text, hunk));
        }
    };
    for piece in diff.split_inclusive(|byte| *byte == b'\n') {
        if piece.starts_with(b"@@ ") {
            finish(&mut current, &mut chosen);
            // The header line is plain ASCII, so lossy text is exact here.
            let hunk = parse_hunks(&String::from_utf8_lossy(piece))
                .into_iter()
                .next();
            current = Some((piece.to_vec(), hunk));
        } else if let Some((body, _)) = &mut current {
            body.extend_from_slice(piece);
        } else {
            header.extend_from_slice(piece);
        }
    }
    finish(&mut current, &mut chosen);
    let (hunk_text, hunk) =
        chosen.ok_or_else(|| "There is no unstaged change at this line".to_string())?;
    let header_text = String::from_utf8_lossy(&header);
    if header_text.contains("\nnew file mode") || header_text.contains("--- /dev/null") {
        return Err("This file is new: stage or discard it as a whole in Source Control".into());
    }
    if header_text.contains("\ndeleted file mode") || header_text.contains("+++ /dev/null") {
        return Err(
            "This file is deleted: restore or stage it as a whole in Source Control".into(),
        );
    }
    // The change must be exactly one of the changes the gutter draws (from the diff against HEAD).
    let same = file_hunks(root, path).iter().any(|shown| {
        shown.covers(line)
            && shown.start == hunk.start
            && shown.len == hunk.len
            && shown.kind == hunk.kind
    });
    if !same {
        return Err(
            "This change overlaps something already staged: use Source Control for it".into(),
        );
    }
    let mut patch = header;
    patch.extend_from_slice(&hunk_text);
    Ok(patch)
}

/// Stage the single change at `line` of `path`, leaving the rest of the file unstaged.
pub fn stage_hunk(root: &Path, path: &str, line: usize) -> Result<(), String> {
    let patch = unstaged_hunk_patch(root, path, line)?;
    run_with_input(root, &["apply", "--cached", "--unidiff-zero", "-"], &patch).map(|_| ())
}

/// Throw away the single unstaged change at `line` of `path` in the working tree.
pub fn revert_hunk(root: &Path, path: &str, line: usize) -> Result<(), String> {
    let patch = unstaged_hunk_patch(root, path, line)?;
    run_with_input(root, &["apply", "--reverse", "--unidiff-zero", "-"], &patch).map(|_| ())
}

pub fn file_hunks(root: &Path, path: &str) -> Vec<Hunk> {
    run(
        root,
        &[
            "diff",
            "HEAD",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "-U0",
            "--",
            path,
        ],
    )
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
    check_branch_name(root, name)?;
    run(root, &["checkout", "-b", name]).map(|_| ())
}

/// Every tag, newest first.
pub fn tags(root: &Path) -> Result<Vec<String>, String> {
    run(root, &["tag", "--list", "--sort=-creatordate"]).map(|out| {
        out.lines()
            .map(str::to_string)
            .filter(|t| !t.is_empty())
            .collect()
    })
}

fn check_tag_name(root: &Path, name: &str) -> Result<(), String> {
    if name.is_empty() || name.starts_with('-') {
        return Err(format!("\"{name}\" is not a valid tag name"));
    }
    run(root, &["check-ref-format", &format!("refs/tags/{name}")])
        .map(|_| ())
        .map_err(|_| format!("\"{name}\" is not a valid tag name"))
}

/// A lightweight tag on the current commit.
pub fn create_tag(root: &Path, name: &str) -> Result<(), String> {
    check_tag_name(root, name)?;
    run(root, &["tag", name]).map(|_| ())
}

/// Delete a local tag. The message carries the commit it pointed at, so it can be made again.
pub fn delete_tag(root: &Path, name: &str) -> Result<String, String> {
    check_tag_name(root, name)?;
    run(root, &["tag", "-d", name]).map(|out| first_line(&out, "Deleted tag"))
}

/// Refuses names Git itself would reject, and anything that could be read as an option
/// (`--orphan`, `-D`), before the name reaches a command line.
pub fn check_branch_name(root: &Path, name: &str) -> Result<(), String> {
    if name.is_empty() || name.starts_with('-') {
        return Err(format!("\"{name}\" is not a valid branch name"));
    }
    run(root, &["check-ref-format", "--branch", name])
        .map(|_| ())
        .map_err(|_| format!("\"{name}\" is not a valid branch name"))
}

/// What a merge did.
#[derive(Debug, Eq, PartialEq)]
pub enum MergeOutcome {
    /// Merged (or fast-forwarded); the text is Git's summary line.
    Merged(String),
    UpToDate,
    /// The merge stopped on conflicts. The repository is mid-merge: resolve each file in Source
    /// Control and commit, or abort with [`merge_abort`].
    Conflicts,
}

/// True while a merge is waiting to be completed or aborted.
pub fn is_merging(root: &Path) -> bool {
    run(root, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_ok()
}

/// True when tracked files have uncommitted changes. Merging (and aborting a merge) can mix those
/// changes up with the merge, so a merge is only started from a clean tree.
fn has_tracked_changes(root: &Path) -> Result<bool, String> {
    run(root, &["status", "--porcelain", "--untracked-files=no"]).map(|out| !out.trim().is_empty())
}

/// Merge `branch` into the current branch. Conflicts are an outcome, not an error.
pub fn merge(root: &Path, branch: &str) -> Result<MergeOutcome, String> {
    check_branch_name(root, branch)?;
    if let Some(operation) = in_progress(root) {
        return Err(format!(
            "A {} is in progress. Commit or abort it first",
            operation.label().to_lowercase()
        ));
    }
    if has_tracked_changes(root)? {
        return Err("Commit or stash your changes before merging".into());
    }
    let before = run(root, &["rev-parse", "HEAD"])?;
    match run(root, &["merge", "--no-edit", branch]) {
        Ok(output) => {
            // Compare commits instead of reading Git's (localised) wording.
            if run(root, &["rev-parse", "HEAD"])? == before {
                Ok(MergeOutcome::UpToDate)
            } else {
                Ok(MergeOutcome::Merged(first_line(&output, "Merged")))
            }
        }
        // MERGE_HEAD alone is not a conflict: a failing hook or signing step also leaves it. Only
        // unmerged paths are.
        Err(_) if is_merging(root) && has_unmerged_paths(root) => Ok(MergeOutcome::Conflicts),
        Err(error) => Err(error),
    }
}

fn has_unmerged_paths(root: &Path) -> bool {
    run(root, &["ls-files", "-u"]).is_ok_and(|out| !out.trim().is_empty())
}

/// A multi-step Git operation that stopped for the user: resolve the conflicts and commit, or abort.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Merge,
    CherryPick,
    Revert,
    /// A rebase started elsewhere (for example in the terminal) that stopped.
    Rebase,
}

impl Operation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Merge => "Merge",
            Self::CherryPick => "Cherry-pick",
            Self::Revert => "Revert",
            Self::Rebase => "Rebase",
        }
    }

    fn abort_args(self) -> [&'static str; 2] {
        match self {
            Self::Merge => ["merge", "--abort"],
            Self::CherryPick => ["cherry-pick", "--abort"],
            Self::Revert => ["revert", "--abort"],
            Self::Rebase => ["rebase", "--abort"],
        }
    }
}

/// The operation waiting to be finished or aborted, if any.
pub fn in_progress(root: &Path) -> Option<Operation> {
    // A rebase keeps its state in a folder, the others in a head file; the rebase is looked for
    // first because it runs cherry-picks internally and leaves their head files around.
    let in_git_dir = |name: &str| {
        run(root, &["rev-parse", "--git-path", name])
            .map(|path| root.join(path.trim()).exists())
            .unwrap_or(false)
    };
    if in_git_dir("rebase-merge") || in_git_dir("rebase-apply") {
        return Some(Operation::Rebase);
    }
    [
        ("MERGE_HEAD", Operation::Merge),
        ("CHERRY_PICK_HEAD", Operation::CherryPick),
        ("REVERT_HEAD", Operation::Revert),
    ]
    .into_iter()
    .find(|(head, _)| run(root, &["rev-parse", "-q", "--verify", head]).is_ok())
    .map(|(_, operation)| operation)
}

/// Give up the merge, cherry-pick or revert in progress and restore the state before it started.
pub fn abort_operation(root: &Path) -> Result<Operation, String> {
    let Some(operation) = in_progress(root) else {
        return Err("There is nothing in progress to abort".into());
    };
    run(root, &operation.abort_args()).map(|_| operation)
}

/// What cherry-picking or reverting a commit did.
#[derive(Debug, Eq, PartialEq)]
pub enum PickOutcome {
    /// A new commit was made; the text is Git's summary line.
    Done(String),
    /// It stopped on conflicts; resolve them and commit, or abort.
    Conflicts,
}

/// A commit hash as written by Git: 7 to 40 hexadecimal digits. Anything else (a branch name, an
/// option, a range) is refused before it reaches a command line.
fn check_hash(root: &Path, hash: &str) -> Result<(), String> {
    if !(7..=40).contains(&hash.len()) || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("\"{hash}\" is not a commit hash"));
    }
    let parents = run(root, &["rev-list", "--parents", "-n", "1", hash])
        .map_err(|_| format!("There is no commit {hash}"))?;
    // "<hash> <parent> <parent>": a merge commit needs a mainline chosen, which is not offered.
    if parents.split_whitespace().count() > 2 {
        return Err("That is a merge commit; choose its changes from the branches instead".into());
    }
    Ok(())
}

fn pick(root: &Path, command: &str, hash: &str) -> Result<PickOutcome, String> {
    check_hash(root, hash)?;
    if let Some(operation) = in_progress(root) {
        return Err(format!(
            "A {} is in progress. Commit or abort it first",
            operation.label().to_lowercase()
        ));
    }
    if has_tracked_changes(root)? {
        return Err("Commit or stash your changes first".into());
    }
    let mut args = vec![command];
    if command == "revert" {
        args.push("--no-edit");
    }
    args.push(hash);
    match run(root, &args) {
        Ok(output) => Ok(PickOutcome::Done(first_line(&output, "Done"))),
        Err(_) if in_progress(root).is_some() && has_unmerged_paths(root) => {
            Ok(PickOutcome::Conflicts)
        }
        Err(error) => {
            // Git can stop without a conflict: the change is already in this branch (an empty
            // pick), or a hook or signing step failed after a clean apply. Leaving that half-done
            // would show a banner about conflicts that do not exist, so it is cancelled.
            if in_progress(root).is_some() {
                let _ = abort_operation(root);
                return Err(if error.to_lowercase().contains("empty") {
                    "Those changes are already in this branch".to_string()
                } else {
                    format!("{error} (the operation was cancelled)")
                });
            }
            Err(error)
        }
    }
}

/// Apply one commit's changes on top of the current branch.
pub fn cherry_pick(root: &Path, hash: &str) -> Result<PickOutcome, String> {
    pick(root, "cherry-pick", hash)
}

/// Make a new commit that undoes one commit.
pub fn revert(root: &Path, hash: &str) -> Result<PickOutcome, String> {
    pick(root, "revert", hash)
}

/// Delete a local branch. Without `force` Git refuses unmerged branches; the current branch can
/// never be deleted.
pub fn delete_branch(root: &Path, name: &str, force: bool) -> Result<(), String> {
    check_branch_name(root, name)?;
    run(root, &["branch", if force { "-D" } else { "-d" }, name]).map(|_| ())
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

/// True when `text` still contains merge conflict markers (a `<<<<<<<` start and a `>>>>>>>` end).
pub fn has_conflict_markers(text: &str) -> bool {
    text.lines().any(|line| line.starts_with("<<<<<<< "))
        && text.lines().any(|line| line.starts_with(">>>>>>> "))
}

/// Which side of a merge conflict to keep.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    /// The branch being merged into (the current branch).
    Current,
    /// The branch being merged in.
    Incoming,
}

/// Resolve a conflicted file by taking one whole side, then stage it. The path is matched
/// literally (`a[1].txt` must not also match `a1.txt`). When the chosen side deleted the file
/// there is no version of it to check out, so the resolution is the deletion.
pub fn resolve_conflict(root: &Path, path: &str, side: Side) -> Result<(), String> {
    let (flag, stage) = match side {
        Side::Current => ("--ours", "2"),
        Side::Incoming => ("--theirs", "3"),
    };
    let stages = run(root, &["--literal-pathspecs", "ls-files", "-u", "--", path])?;
    let has_side = stages
        .lines()
        .any(|line| line.split(['\t', ' ']).nth(2) == Some(stage));
    if !stages.trim().is_empty() && !has_side {
        return run(root, &["--literal-pathspecs", "rm", "-f", "--", path]).map(|_| ());
    }
    run(root, &["--literal-pathspecs", "checkout", flag, "--", path])?;
    run(root, &["--literal-pathspecs", "add", "--", path]).map(|_| ())
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

/// Apply a stash and keep it in the list (unlike a pop, a conflict cannot lose it).
pub fn stash_apply(root: &Path, index: usize) -> Result<String, String> {
    let reference = format!("stash@{{{index}}}");
    run(root, &["stash", "apply", &reference]).map(|out| first_line(&out, "Applied stash"))
}

/// Drop a stash. The message names the commit that held it, which `git stash store` can bring back.
pub fn stash_drop(root: &Path, index: usize) -> Result<String, String> {
    let reference = format!("stash@{{{index}}}");
    run(root, &["stash", "drop", &reference]).map(|out| first_line(&out, "Dropped stash"))
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
    run(
        root,
        &[
            "show",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--stat",
            "--patch",
            hash,
        ],
    )
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
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_REPO: AtomicU64 = AtomicU64::new(0);

    fn git_available() -> bool {
        run(Path::new("."), &["--version"]).is_ok()
    }

    fn temp_repo() -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_REPO.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "forge-room-vcs-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        run(&dir, &["init", "-q"]).unwrap();
        // Byte-exact fixtures must not inherit a Windows-wide autocrlf setting.
        run(&dir, &["config", "core.autocrlf", "false"]).unwrap();
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
    fn resolving_matches_the_path_literally_and_handles_a_deleted_side() {
        if !git_available() {
            return;
        }
        let dir = temp_repo();
        let git = |args: &[&str]| {
            let mut full = vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
            ];
            full.extend_from_slice(args);
            run(&dir, &full)
        };
        for name in ["a[1].txt", "a1.txt", "gone.txt"] {
            fs::write(
                dir.join(name),
                "base
",
            )
            .unwrap();
        }
        git(&["add", "."]).unwrap();
        git(&["commit", "-q", "-m", "base"]).unwrap();
        let main = git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        git(&["checkout", "-q", "-b", "feature"]).unwrap();
        for name in ["a[1].txt", "a1.txt"] {
            fs::write(
                dir.join(name),
                "from feature
",
            )
            .unwrap();
        }
        git(&["rm", "-q", "gone.txt"]).unwrap();
        git(&["commit", "-q", "-am", "feature"]).unwrap();
        git(&["checkout", "-q", &main]).unwrap();
        for name in ["a[1].txt", "a1.txt", "gone.txt"] {
            fs::write(
                dir.join(name),
                "from main
",
            )
            .unwrap();
        }
        git(&["commit", "-q", "-am", "main"]).unwrap();
        assert!(git(&["merge", "feature"]).is_err());

        resolve_conflict(&dir, "a[1].txt", Side::Incoming).unwrap();
        let unmerged = git(&["ls-files", "-u"]).unwrap();
        assert!(!unmerged.contains("a[1].txt"));
        assert!(
            unmerged.contains("a1.txt"),
            "the other file stays conflicted"
        );
        assert!(
            fs::read_to_string(dir.join("a1.txt"))
                .unwrap()
                .contains("<<<<<<<")
        );

        // The incoming branch deleted gone.txt: accepting it means deleting the file.
        resolve_conflict(&dir, "gone.txt", Side::Incoming).unwrap();
        assert!(!dir.join("gone.txt").exists());
        assert!(!git(&["ls-files", "-u"]).unwrap().contains("gone.txt"));
    }

    #[test]
    fn tags_can_be_made_listed_and_deleted_and_bad_names_are_refused() {
        if !git_available() {
            return;
        }
        let (dir, _main) = merge_fixture();
        create_tag(&dir, "v1").unwrap();
        create_tag(&dir, "v2").unwrap();
        assert!(
            create_tag(&dir, "v1").is_err(),
            "an existing tag is not overwritten"
        );
        for bad in ["", "-d", "a b", "x..y", "--force"] {
            assert!(create_tag(&dir, bad).is_err(), "{bad:?}");
            assert!(delete_tag(&dir, bad).is_err(), "{bad:?}");
        }
        let mut found = tags(&dir).unwrap();
        found.sort();
        assert_eq!(found, vec!["v1", "v2"]);
        assert!(delete_tag(&dir, "v1").unwrap().contains("v1"));
        assert_eq!(tags(&dir).unwrap(), vec!["v2"]);
        assert!(delete_tag(&dir, "v1").is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_subfolder_of_a_repository_reports_the_top_folder() {
        if !git_available() {
            return;
        }
        let (dir, _main) = merge_fixture();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        assert_eq!(
            toplevel(&dir),
            None,
            "the top folder itself is not a subfolder"
        );
        assert_eq!(toplevel(&sub), Some(crate::tabs::normalize(&dir)));
        // Commands run from the top folder see paths the way status reports them.
        fs::write(sub.join("n.txt"), "x").unwrap();
        let top = toplevel(&sub).unwrap();
        stage(&top, &["sub/n.txt".to_string()]).unwrap();
        assert!(
            status(&top)
                .unwrap()
                .changes
                .iter()
                .any(|c| c.path == "sub/n.txt" && c.is_staged())
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_stash_can_be_applied_and_kept_or_dropped() {
        if !git_available() {
            return;
        }
        let (dir, _main) = merge_fixture();
        fs::write(dir.join("a.txt"), "changed\n").unwrap();
        stash_push(&dir, "work").unwrap();
        assert_eq!(stash_list(&dir).len(), 1);
        stash_apply(&dir, 0).unwrap();
        assert_eq!(fs::read_to_string(dir.join("a.txt")).unwrap(), "changed\n");
        // Applied, not popped: still in the list.
        assert_eq!(stash_list(&dir).len(), 1);
        stash_drop(&dir, 0).unwrap();
        assert!(stash_list(&dir).is_empty());
        assert!(stash_drop(&dir, 0).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    /// A repo with one commit on the default branch; returns the directory, a git runner and the
    /// name of the default branch.
    fn merge_fixture() -> (PathBuf, String) {
        let dir = temp_repo();
        let g = |args: &[&str]| {
            let mut full = vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
            ];
            full.extend_from_slice(args);
            run(&dir, &full).unwrap_or_else(|e| panic!("git {args:?}: {e}"))
        };
        // The functions under test run plain `git` with no -c options, and a CI runner has no
        // global identity: the repository gets its own, so merges and picks can make commits.
        g(&["config", "user.name", "Test"]);
        g(&["config", "user.email", "test@example.com"]);
        g(&["config", "commit.gpgsign", "false"]);
        fs::write(dir.join("a.txt"), "base\n").unwrap();
        g(&["add", "."]);
        g(&["commit", "-q", "-m", "base"]);
        let main = g(&["rev-parse", "--abbrev-ref", "HEAD"]).trim().to_string();
        (dir, main)
    }

    fn g(dir: &Path, args: &[&str]) -> String {
        let mut full = vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
        ];
        full.extend_from_slice(args);
        run(dir, &full).unwrap_or_else(|e| panic!("git {args:?}: {e}"))
    }

    #[test]
    fn single_changes_can_be_staged_or_reverted_leaving_the_rest() {
        if !git_available() {
            return;
        }
        let (dir, _main) = merge_fixture();
        let lines: Vec<String> = (1..=12).map(|n| format!("line {n}")).collect();
        fs::write(dir.join("f.txt"), lines.join("\n") + "\n").unwrap();
        g(&dir, &["add", "."]);
        g(&dir, &["commit", "-q", "-m", "twelve lines"]);
        // Three separate changes: modify line 2, delete line 6, add a line after line 10.
        let mut edited = lines.clone();
        edited[1] = "line 2 changed".into();
        edited.remove(5);
        edited.insert(9, "inserted".into());
        fs::write(dir.join("f.txt"), edited.join("\n") + "\n").unwrap();

        // Stage only the first change: the index holds it, the other two are still unstaged.
        stage_hunk(&dir, "f.txt", 2).unwrap();
        let staged = g(&dir, &["diff", "--cached", "-U0", "--", "f.txt"]);
        assert!(staged.contains("+line 2 changed") && !staged.contains("inserted"));
        assert!(!staged.contains("-line 6"));
        let unstaged = g(&dir, &["diff", "-U0", "--", "f.txt"]);
        assert!(!unstaged.contains("line 2 changed") && unstaged.contains("+inserted"));
        // Nothing is left to stage there now.
        assert!(
            stage_hunk(&dir, "f.txt", 2)
                .unwrap_err()
                .contains("no unstaged change")
        );

        // Revert the inserted line: it leaves the file, the others stay.
        revert_hunk(&dir, "f.txt", 10).unwrap();
        let text = fs::read_to_string(dir.join("f.txt"))
            .unwrap()
            .replace("\r\n", "\n");
        assert!(!text.contains("inserted"));
        assert!(text.contains("line 2 changed"));
        assert!(!text.contains("line 6"), "the deletion is still there");

        // A deleted block is covered by the line just above it, as the gutter marks it.
        stage_hunk(&dir, "f.txt", 5).unwrap();
        assert!(g(&dir, &["diff", "--cached", "-U0", "--", "f.txt"]).contains("-line 6"));
        assert!(unstaged_hunk_patch(&dir, "f.txt", 1).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_new_file_or_an_overlapping_staged_change_is_never_staged_or_reverted_by_a_bar() {
        if !git_available() {
            return;
        }
        let (dir, _main) = merge_fixture();
        // An intent-to-add file: reverting its only change would delete the file for good.
        fs::write(dir.join("new.txt"), "one\ntwo\n").unwrap();
        g(&dir, &["add", "-N", "new.txt"]);
        assert!(revert_hunk(&dir, "new.txt", 1).unwrap_err().contains("new"));
        assert!(stage_hunk(&dir, "new.txt", 1).is_err());
        assert_eq!(
            fs::read_to_string(dir.join("new.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "one\ntwo\n"
        );

        // A staged insertion with an unstaged deletion next to it: the bar on the insertion must not
        // act on the deletion.
        fs::write(dir.join("o.txt"), "a\nb\nc\n").unwrap();
        g(&dir, &["add", "o.txt"]);
        g(&dir, &["commit", "-q", "-m", "o"]);
        fs::write(dir.join("o.txt"), "a\nX\nb\nc\n").unwrap();
        g(&dir, &["add", "o.txt"]);
        fs::write(dir.join("o.txt"), "a\nX\n").unwrap();
        let before = fs::read_to_string(dir.join("o.txt")).unwrap();
        assert!(
            revert_hunk(&dir, "o.txt", 2)
                .unwrap_err()
                .contains("overlaps")
        );
        assert!(stage_hunk(&dir, "o.txt", 2).is_err());
        assert_eq!(fs::read_to_string(dir.join("o.txt")).unwrap(), before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_reverted_change_in_a_file_that_is_not_utf8_comes_back_byte_for_byte() {
        if !git_available() {
            return;
        }
        let (dir, _main) = merge_fixture();
        // Windows-1252 text: 0xE9 is an e-acute and is not valid UTF-8 on its own.
        let original: Vec<u8> = b"keep\ncaf\xe9\nend\n".to_vec();
        fs::write(dir.join("latin.txt"), &original).unwrap();
        g(&dir, &["add", "."]);
        g(&dir, &["commit", "-q", "-m", "latin"]);
        fs::write(dir.join("latin.txt"), b"keep\nend\n").unwrap();
        revert_hunk(&dir, "latin.txt", 1).unwrap();
        assert_eq!(fs::read(dir.join("latin.txt")).unwrap(), original);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_snapshot_gathers_everything_in_one_call() {
        if !git_available() {
            return;
        }
        let (dir, main) = merge_fixture();
        fs::write(dir.join("b.txt"), "new\n").unwrap();
        g(&dir, &["stash", "push", "--include-untracked", "-q"]);
        fs::write(dir.join("a.txt"), "edited\n").unwrap();
        let shot = snapshot(&dir);
        assert!(shot.is_repo && shot.operation.is_none());
        assert_eq!(shot.branch.as_deref(), Some(main.as_str()));
        // `log_all` walks every ref, so the stash commits are in the list as well.
        assert!(shot.commits.iter().any(|commit| commit.subject == "base"));
        assert_eq!(shot.stashes, 1);
        assert!(
            shot.status
                .as_ref()
                .unwrap()
                .changes
                .iter()
                .any(|c| c.path == "a.txt")
        );
        assert_eq!(shot.root, dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn merging_clean_up_to_date_and_conflicting_branches() {
        if !git_available() {
            return;
        }
        let (dir, main) = merge_fixture();
        g(&dir, &["checkout", "-q", "-b", "feature"]);
        fs::write(dir.join("b.txt"), "feature\n").unwrap();
        g(&dir, &["add", "."]);
        g(&dir, &["commit", "-q", "-m", "feature"]);
        g(&dir, &["checkout", "-q", &main]);

        assert!(matches!(
            merge(&dir, "feature"),
            Ok(MergeOutcome::Merged(_))
        ));
        assert!(dir.join("b.txt").exists());
        assert_eq!(merge(&dir, "feature"), Ok(MergeOutcome::UpToDate));

        // A conflicting change on both sides stops mid-merge, and aborting restores the tree.
        g(&dir, &["checkout", "-q", "-b", "other"]);
        fs::write(dir.join("a.txt"), "other\n").unwrap();
        g(&dir, &["commit", "-q", "-am", "other"]);
        g(&dir, &["checkout", "-q", &main]);
        fs::write(dir.join("a.txt"), "mine\n").unwrap();
        g(&dir, &["commit", "-q", "-am", "mine"]);
        assert_eq!(merge(&dir, "other"), Ok(MergeOutcome::Conflicts));
        assert!(is_merging(&dir));
        assert!(
            merge(&dir, "feature").is_err(),
            "no second merge while one is open"
        );
        assert_eq!(abort_operation(&dir), Ok(Operation::Merge));
        assert!(!is_merging(&dir));
        assert_eq!(
            fs::read_to_string(dir.join("a.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "mine\n"
        );
        assert!(abort_operation(&dir).is_err(), "nothing left to abort");

        // Uncommitted changes to tracked files: the merge is refused before it starts.
        fs::write(dir.join("a.txt"), "dirty\n").unwrap();
        assert!(
            merge(&dir, "feature")
                .unwrap_err()
                .contains("Commit or stash")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cherry_picks_and_reverts_with_conflicts_aborts_and_refused_hashes() {
        if !git_available() {
            return;
        }
        let (dir, main) = merge_fixture();
        g(&dir, &["checkout", "-q", "-b", "side"]);
        fs::write(dir.join("b.txt"), "side\n").unwrap();
        g(&dir, &["add", "."]);
        g(&dir, &["commit", "-q", "-m", "add b"]);
        let add_b = g(&dir, &["rev-parse", "HEAD"]).trim().to_string();
        fs::write(dir.join("a.txt"), "side change\n").unwrap();
        g(&dir, &["commit", "-q", "-am", "change a on side"]);
        let change_a = g(&dir, &["rev-parse", "HEAD"]).trim().to_string();
        g(&dir, &["checkout", "-q", &main]);

        // A clean cherry-pick makes a new commit with the same change.
        assert!(matches!(
            cherry_pick(&dir, &add_b),
            Ok(PickOutcome::Done(_))
        ));
        assert!(dir.join("b.txt").exists());
        assert_eq!(in_progress(&dir), None);
        // ... and a revert undoes it again.
        assert!(matches!(
            revert(&dir, g(&dir, &["rev-parse", "HEAD"]).trim()),
            Ok(PickOutcome::Done(_))
        ));
        assert!(!dir.join("b.txt").exists());

        // A conflicting cherry-pick stops mid-way; aborting restores the tree.
        fs::write(dir.join("a.txt"), "main change\n").unwrap();
        g(&dir, &["commit", "-q", "-am", "change a on main"]);
        assert_eq!(cherry_pick(&dir, &change_a), Ok(PickOutcome::Conflicts));
        assert_eq!(in_progress(&dir), Some(Operation::CherryPick));
        assert!(
            merge(&dir, "side").is_err(),
            "no merge while a cherry-pick is open"
        );
        assert!(revert(&dir, &add_b).is_err());
        assert_eq!(abort_operation(&dir), Ok(Operation::CherryPick));
        assert_eq!(in_progress(&dir), None);
        assert_eq!(
            fs::read_to_string(dir.join("a.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "main change\n"
        );

        // Cherry-picking what is already there is refused cleanly, with nothing left half-done.
        assert!(matches!(
            cherry_pick(&dir, &add_b),
            Ok(PickOutcome::Done(_))
        ));
        assert!(
            cherry_pick(&dir, &add_b)
                .unwrap_err()
                .contains("already in this branch")
        );
        assert_eq!(in_progress(&dir), None);

        // A conflict can be finished by committing with the message Git prepared.
        g(&dir, &["checkout", "-q", "-B", "work", &main]);
        assert_eq!(cherry_pick(&dir, &change_a), Ok(PickOutcome::Conflicts));
        fs::write(dir.join("a.txt"), "resolved\n").unwrap();
        g(&dir, &["add", "a.txt"]);
        g(&dir, &["config", "user.name", "Test"]);
        g(&dir, &["config", "user.email", "test@example.com"]);
        g(&dir, &["config", "commit.gpgsign", "false"]);
        assert!(commit(&dir, "").is_ok(), "the prepared message is used");
        assert_eq!(in_progress(&dir), None);
        let subject = g(&dir, &["log", "-1", "--format=%s"]);
        assert_eq!(subject.trim(), "change a on side");
        g(&dir, &["checkout", "-q", &main]);

        // Refused: not a hash, unknown hash, uncommitted changes.
        for bad in [
            "",
            "abc",
            "--abort",
            "HEAD",
            "main",
            "zzzzzzzz",
            "0000000000",
        ] {
            assert!(cherry_pick(&dir, bad).is_err(), "{bad:?}");
        }
        fs::write(dir.join("a.txt"), "dirty\n").unwrap();
        assert!(
            cherry_pick(&dir, &add_b)
                .unwrap_err()
                .contains("Commit or stash")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_conflicting_revert_stops_and_aborts_like_a_pick() {
        if !git_available() {
            return;
        }
        let (dir, _main) = merge_fixture();
        fs::write(dir.join("a.txt"), "second\n").unwrap();
        g(&dir, &["commit", "-q", "-am", "second"]);
        let second = g(&dir, &["rev-parse", "HEAD"]).trim().to_string();
        fs::write(dir.join("a.txt"), "third\n").unwrap();
        g(&dir, &["commit", "-q", "-am", "third"]);
        // Undoing "second" now collides with "third".
        assert_eq!(revert(&dir, &second), Ok(PickOutcome::Conflicts));
        assert_eq!(in_progress(&dir), Some(Operation::Revert));
        assert_eq!(abort_operation(&dir), Ok(Operation::Revert));
        assert_eq!(in_progress(&dir), None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_running_rebase_is_detected_refused_around_and_abortable() {
        if !git_available() {
            return;
        }
        let (dir, main) = merge_fixture();
        g(&dir, &["checkout", "-q", "-b", "topic"]);
        fs::write(dir.join("a.txt"), "topic\n").unwrap();
        g(&dir, &["commit", "-q", "-am", "topic"]);
        g(&dir, &["checkout", "-q", &main]);
        fs::write(dir.join("a.txt"), "main\n").unwrap();
        g(&dir, &["commit", "-q", "-am", "main"]);
        g(&dir, &["checkout", "-q", "topic"]);
        // The rebase stops on the conflict (a non-zero exit, so run it without the unwrap helper).
        let _ = run(
            &dir,
            &[
                "-c",
                "user.name=T",
                "-c",
                "user.email=t@e.com",
                "rebase",
                &main,
            ],
        );
        assert_eq!(in_progress(&dir), Some(Operation::Rebase));
        assert!(merge(&dir, &main).unwrap_err().contains("rebase"));
        assert_eq!(abort_operation(&dir), Ok(Operation::Rebase));
        assert_eq!(in_progress(&dir), None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_merge_commit_cannot_be_picked_or_reverted() {
        if !git_available() {
            return;
        }
        let (dir, main) = merge_fixture();
        g(&dir, &["checkout", "-q", "-b", "side"]);
        fs::write(dir.join("b.txt"), "b\n").unwrap();
        g(&dir, &["add", "."]);
        g(&dir, &["commit", "-q", "-m", "side"]);
        g(&dir, &["checkout", "-q", &main]);
        fs::write(dir.join("c.txt"), "c\n").unwrap();
        g(&dir, &["add", "."]);
        g(&dir, &["commit", "-q", "-m", "main"]);
        g(
            &dir,
            &["merge", "--no-ff", "-q", "-m", "merge side", "side"],
        );
        let merge_commit = g(&dir, &["rev-parse", "HEAD"]).trim().to_string();
        assert!(
            revert(&dir, &merge_commit)
                .unwrap_err()
                .contains("merge commit")
        );
        assert!(
            cherry_pick(&dir, &merge_commit)
                .unwrap_err()
                .contains("merge commit")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn deleting_branches_and_refusing_bad_names() {
        if !git_available() {
            return;
        }
        let (dir, main) = merge_fixture();
        create_branch(&dir, "topic").unwrap();
        g(&dir, &["checkout", "-q", &main]);
        assert!(
            delete_branch(&dir, &main, true).is_err(),
            "never the current branch"
        );
        delete_branch(&dir, "topic", false).unwrap();
        assert!(branches(&dir).unwrap().iter().all(|b| b.name != "topic"));

        for bad in ["", "-D", "--orphan", "a b", "x..y", "feature/", "ends.lock"] {
            assert!(create_branch(&dir, bad).is_err(), "{bad:?} must be refused");
            assert!(merge(&dir, bad).is_err());
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolving_a_conflict_keeps_the_chosen_side_and_stages_it() {
        if !git_available() {
            return;
        }
        let dir = temp_repo();
        let git = |args: &[&str]| {
            let mut full = vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
            ];
            full.extend_from_slice(args);
            run(&dir, &full)
        };
        fs::write(dir.join("a.txt"), "base\n").unwrap();
        git(&["add", "a.txt"]).unwrap();
        git(&["commit", "-q", "-m", "base"]).unwrap();
        let main = git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        git(&["checkout", "-q", "-b", "feature"]).unwrap();
        fs::write(dir.join("a.txt"), "from feature\n").unwrap();
        git(&["commit", "-q", "-am", "feature"]).unwrap();
        git(&["checkout", "-q", &main]).unwrap();
        fs::write(dir.join("a.txt"), "from main\n").unwrap();
        git(&["commit", "-q", "-am", "main"]).unwrap();
        assert!(
            git(&["merge", "feature"]).is_err(),
            "the merge should conflict"
        );

        let before = status(&dir).unwrap();
        assert!(before.changes.iter().any(|change| change.is_conflict()));

        resolve_conflict(&dir, "a.txt", Side::Incoming).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("a.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "from feature\n"
        );
        let after = status(&dir).unwrap();
        assert!(after.changes.iter().all(|change| !change.is_conflict()));
        let _ = fs::remove_dir_all(dir);

        // And the other side, in a second repo.
        let dir = temp_repo();
        let git = |args: &[&str]| {
            let mut full = vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
            ];
            full.extend_from_slice(args);
            run(&dir, &full)
        };
        fs::write(dir.join("a.txt"), "base\n").unwrap();
        git(&["add", "a.txt"]).unwrap();
        git(&["commit", "-q", "-m", "base"]).unwrap();
        let main = git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        git(&["checkout", "-q", "-b", "feature"]).unwrap();
        fs::write(dir.join("a.txt"), "from feature\n").unwrap();
        git(&["commit", "-q", "-am", "feature"]).unwrap();
        git(&["checkout", "-q", &main]).unwrap();
        fs::write(dir.join("a.txt"), "from main\n").unwrap();
        git(&["commit", "-q", "-am", "main"]).unwrap();
        assert!(git(&["merge", "feature"]).is_err());
        resolve_conflict(&dir, "a.txt", Side::Current).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("a.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "from main\n"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conflict_markers_are_detected_only_when_complete() {
        let conflicted = "a\n<<<<<<< HEAD\nmine\n=======\ntheirs\n>>>>>>> feature\nz\n";
        assert!(has_conflict_markers(conflicted));
        assert!(!has_conflict_markers("a\nmine\nz\n"));
        // A lone separator or an indented marker in prose or code is not a conflict.
        assert!(!has_conflict_markers("=======\n"));
        assert!(!has_conflict_markers(
            "    <<<<<<< in a string\n    >>>>>>> too\n"
        ));
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
