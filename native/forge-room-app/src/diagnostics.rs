//! Problems from `cargo check`: the compiler's own errors and warnings with the file, line and
//! column, so the Problems panel can list them and open each one. This is not a language server:
//! it runs when asked, and what it shows is as old as the last run.

use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A check that runs longer than this is stopped.
const LIMIT: Duration = Duration::from_secs(600);
const MAX_PROBLEMS: usize = 500;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Problem {
    pub error: bool,
    pub message: String,
    pub file: PathBuf,
    /// 1-based.
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub problems: Vec<Problem>,
    /// Why the check could not run or ended early (no cargo, a manifest that does not parse).
    pub failure: Option<String>,
    pub manifest: Option<PathBuf>,
    pub truncated: bool,
}

/// The `Cargo.toml` to check: one in `root`, else the workspace (or first crate) found one or two
/// folders down, which is how a repository with its Rust code in `native/` is laid out.
pub fn find_manifest(root: &Path) -> Option<PathBuf> {
    let own = root.join("Cargo.toml");
    if own.is_file() {
        return Some(own);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut folders: Vec<PathBuf> = vec![root.to_path_buf()];
    for _ in 0..2 {
        let mut next = Vec::new();
        for folder in &folders {
            let Ok(entries) = std::fs::read_dir(folder) else {
                continue;
            };
            let mut subfolders: Vec<PathBuf> = entries
                .flatten()
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                .map(|entry| entry.path())
                .filter(|path| {
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                    !matches!(
                        name.as_deref(),
                        Some("target" | "node_modules" | ".git" | ".venv")
                    )
                })
                .collect();
            subfolders.sort();
            for path in subfolders {
                if path.join("Cargo.toml").is_file() {
                    candidates.push(path.join("Cargo.toml"));
                }
                next.push(path);
            }
        }
        folders = next;
    }
    // A workspace root covers the crates under it, so it is the better thing to check.
    candidates
        .iter()
        .find(|manifest| {
            std::fs::read_to_string(manifest)
                .is_ok_and(|text| text.lines().any(|line| line.trim() == "[workspace]"))
        })
        .or_else(|| candidates.first())
        .cloned()
}

/// The problem in one line of `cargo check --message-format=json` output, when it is a compiler
/// error or warning with a place in a file. `base` is where relative file names start.
pub fn parse_line(line: &str, base: &Path) -> Option<Problem> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("reason")?.as_str()? != "compiler-message" {
        return None;
    }
    let message = value.get("message")?;
    let error = match message.get("level")?.as_str()? {
        "error" => true,
        "warning" => false,
        _ => return None,
    };
    let text = message.get("message")?.as_str()?;
    // "aborting due to ..." and "N warnings emitted" have no place of their own.
    let span = message
        .get("spans")?
        .as_array()?
        .iter()
        .find(|span| span.get("is_primary").and_then(Value::as_bool) == Some(true))?;
    let name = span.get("file_name")?.as_str()?;
    let file = resolve(base, name)?;
    Some(Problem {
        error,
        message: text.to_string(),
        file,
        line: span.get("line_start")?.as_u64()?.max(1) as usize,
        column: span.get("column_start")?.as_u64()?.max(1) as usize,
    })
}

/// A file name from the compiler: absolute as it is, or relative to `base` or one of its
/// parents (the workspace root may be above the folder that was checked).
fn resolve(base: &Path, name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    base.ancestors()
        .take(4)
        .map(|folder| folder.join(path))
        .find(|candidate| candidate.is_file())
}

/// Run `cargo check` for the project in `root` and collect what it reports.
pub fn check(root: &Path) -> Report {
    let mut report = Report::default();
    let Some(manifest) = find_manifest(root) else {
        report.failure = Some("No Cargo.toml in this folder or the ones just below it".into());
        return report;
    };
    let base = manifest.parent().unwrap_or(root).to_path_buf();
    report.manifest = Some(manifest.clone());
    let mut command = Command::new("cargo");
    command
        .args([
            "check",
            "--message-format=json",
            "--all-targets",
            "--manifest-path",
        ])
        .arg(&manifest)
        .current_dir(&base)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            report.failure = Some(if error.kind() == std::io::ErrorKind::NotFound {
                "cargo was not found. Install Rust, or start Forge from a terminal where cargo \
                 works (its .cargo bin folder must be on PATH)"
                    .to_string()
            } else {
                format!("Could not run cargo: {error}")
            });
            return report;
        }
    };
    // Read on a thread so a chatty build can never fill the pipe and stall, and so the time limit
    // can still end it.
    let mut pipe = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(pipe) = &mut pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let mut error_pipe = child.stderr.take();
    let error_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(pipe) = &mut error_pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() > LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                report.failure = Some("cargo check ran too long and was stopped".into());
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                report.failure = Some(format!("Could not wait for cargo: {error}"));
                break None;
            }
        }
    };
    let output = reader.join().unwrap_or_default();
    for line in String::from_utf8_lossy(&output).lines() {
        if let Some(problem) = parse_line(line, &base)
            && !report.problems.contains(&problem)
        {
            if report.problems.len() >= MAX_PROBLEMS {
                report.truncated = true;
                break;
            }
            report.problems.push(problem);
        }
    }
    // No problems and a failing cargo means it could not build at all (a broken manifest).
    if let Some(status) = status
        && !status.success()
        && report.problems.is_empty()
        && report.failure.is_none()
    {
        let errors = String::from_utf8_lossy(&error_reader.join().unwrap_or_default()).to_string();
        let reason = errors
            .lines()
            .find(|line| line.trim_start().starts_with("error"))
            .or_else(|| errors.lines().find(|line| !line.trim().is_empty()))
            .unwrap_or("");
        report.failure = Some(if reason.is_empty() {
            "cargo check failed before it could report problems".to_string()
        } else {
            format!("cargo check failed: {}", reason.trim())
        });
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("forge-diag-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn compiler_messages_become_problems_and_the_rest_is_ignored() {
        let dir = temp("parse");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src").join("main.rs"), "fn main() {}").unwrap();
        let error = r#"{"reason":"compiler-message","message":{"level":"error","message":"mismatched types","spans":[{"is_primary":false,"file_name":"src/main.rs","line_start":1,"column_start":1},{"is_primary":true,"file_name":"src/main.rs","line_start":7,"column_start":13}]}}"#;
        let problem = parse_line(error, &dir).unwrap();
        assert!(problem.error);
        assert_eq!((problem.line, problem.column), (7, 13));
        assert_eq!(problem.message, "mismatched types");
        assert!(problem.file.ends_with("main.rs"));
        let warning = error.replace(r#""error""#, r#""warning""#);
        assert!(!parse_line(&warning, &dir).unwrap().error);
        // A summary line has no span; other reasons and notes are not problems; junk is ignored.
        let summary = r#"{"reason":"compiler-message","message":{"level":"error","message":"aborting due to 1 previous error","spans":[]}}"#;
        assert_eq!(parse_line(summary, &dir), None);
        let note = error.replace(r#""error""#, r#""note""#);
        assert_eq!(parse_line(&note, &dir), None);
        assert_eq!(parse_line(r#"{"reason":"compiler-artifact"}"#, &dir), None);
        assert_eq!(parse_line("not json", &dir), None);
        // A file that is not there cannot be opened, so it is not listed.
        let missing = error.replace("src/main.rs", "src/gone.rs");
        assert_eq!(parse_line(&missing, &dir), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_manifest_is_found_in_the_folder_or_in_a_workspace_just_below() {
        let dir = temp("manifest");
        assert_eq!(find_manifest(&dir), None);
        fs::create_dir_all(dir.join("native").join("app")).unwrap();
        fs::create_dir_all(dir.join("target").join("x")).unwrap();
        fs::write(dir.join("target").join("x").join("Cargo.toml"), "").unwrap();
        fs::write(
            dir.join("native").join("app").join("Cargo.toml"),
            "[package]",
        )
        .unwrap();
        // A lone crate two levels down is used; a workspace root above it is better.
        assert_eq!(
            find_manifest(&dir),
            Some(dir.join("native").join("app").join("Cargo.toml"))
        );
        fs::write(
            dir.join("native").join("Cargo.toml"),
            "[workspace]\nmembers = [\"app\"]",
        )
        .unwrap();
        assert_eq!(
            find_manifest(&dir),
            Some(dir.join("native").join("Cargo.toml"))
        );
        fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(find_manifest(&dir), Some(dir.join("Cargo.toml")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_real_cargo_check_reports_an_error_with_its_place() {
        if Command::new("cargo").arg("--version").output().is_err() {
            return;
        }
        let dir = temp("real");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(
            dir.join("src").join("main.rs"),
            "fn main() {\n    let x: u32 = \"a\";\n}\n",
        )
        .unwrap();
        let report = check(&dir);
        assert!(report.failure.is_none(), "{:?}", report.failure);
        let first = report.problems.iter().find(|p| p.error).expect("an error");
        assert_eq!(first.line, 2);
        assert!(first.file.ends_with("main.rs"));
        fs::remove_dir_all(dir).unwrap();
    }
}
