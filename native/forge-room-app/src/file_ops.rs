//! File operations from the Explorer that need rules of their own: names that are valid on every
//! platform we may be used on (and on Windows in particular), and renaming without overwriting
//! anything.

use std::fs;
use std::path::{Path, PathBuf};

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// A single file or folder name that is valid as it stands, or why it is not. The name is
/// trimmed; path separators, `.` and `..`, characters Windows forbids, control characters, names
/// ending in a dot and the reserved device names (`CON`, `NUL`, `COM1`...) are refused.
pub fn valid_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a name".into());
    }
    if name == "." || name == ".." {
        return Err("\".\" and \"..\" are not names".into());
    }
    if let Some(bad) = name.chars().find(|c| {
        matches!(c, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control()
    }) {
        return Err(format!("A name cannot contain {bad:?}"));
    }
    if name.ends_with('.') {
        return Err("A name cannot end with a dot".into());
    }
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    if RESERVED
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
    {
        return Err(format!("\"{stem}\" is a reserved name on Windows"));
    }
    Ok(name)
}

/// Rename `from` (a file or folder) to `new_name` in the same folder and return the new path.
/// Nothing is overwritten: a different existing entry with that name is an error. Changing only
/// the case of a name is allowed.
pub fn rename(from: &Path, new_name: &str) -> Result<PathBuf, String> {
    let name = valid_name(new_name)?;
    if !from.exists() {
        return Err("That file no longer exists".into());
    }
    let target = from.with_file_name(name);
    if target == from {
        return Ok(target);
    }
    if target.exists() && !is_same_entry(from, &target) {
        return Err(format!("\"{name}\" already exists here"));
    }
    fs::rename(from, &target).map_err(|error| format!("Could not rename: {error}"))?;
    Ok(target)
}

/// Whether two paths are the same file or folder (a rename that only changes case on a
/// case-insensitive file system names the same entry).
fn is_same_entry(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(label: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "forge-fileops-{label}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn good_names_pass_and_are_trimmed() {
        assert_eq!(valid_name("notes.txt"), Ok("notes.txt"));
        assert_eq!(valid_name("  main.rs "), Ok("main.rs"));
        assert_eq!(
            valid_name("ends "),
            Ok("ends"),
            "surrounding spaces are trimmed away"
        );
        assert_eq!(valid_name(".gitignore"), Ok(".gitignore"));
        assert_eq!(valid_name("a b (1).md"), Ok("a b (1).md"));
        assert_eq!(valid_name("héllo.txt"), Ok("héllo.txt"));
        assert_eq!(valid_name("console.log"), Ok("console.log"));
    }

    #[test]
    fn bad_names_are_refused_with_a_reason() {
        for bad in [
            "",
            "   ",
            ".",
            "..",
            "a/b",
            "a\\b",
            "a:b",
            "a?b",
            "a*b",
            "a<b",
            "a>b",
            "a|b",
            "a\"b",
            "a\tb",
            "ends.",
            "con",
            "NUL",
            "Com1",
            "lpt9.txt",
            "aux.tar.gz",
        ] {
            assert!(valid_name(bad).is_err(), "{bad:?} must be refused");
        }
        assert!(valid_name("a:b").unwrap_err().contains("':'"));
    }

    #[test]
    fn renaming_a_file_or_a_folder_returns_the_new_path() {
        let dir = folder("rename");
        let file = dir.join("old.txt");
        fs::write(&file, "x").unwrap();
        let renamed = rename(&file, "new.txt").unwrap();
        assert_eq!(renamed, dir.join("new.txt"));
        assert!(!file.exists() && renamed.exists());

        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("inner.txt"), "y").unwrap();
        let moved = rename(&sub, "renamed").unwrap();
        assert!(moved.join("inner.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn nothing_is_overwritten_and_bad_requests_are_errors() {
        let dir = folder("clash");
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        assert!(rename(&a, "b.txt").unwrap_err().contains("already exists"));
        assert_eq!(fs::read_to_string(&b).unwrap(), "b");
        assert!(rename(&a, "x/y").is_err());
        assert!(rename(&dir.join("missing.txt"), "z.txt").is_err());
        // The same name is a harmless no-op.
        assert_eq!(rename(&a, "a.txt").unwrap(), a);
        assert!(a.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn a_case_only_rename_works_on_a_case_insensitive_file_system() {
        let dir = folder("case");
        let file = dir.join("readme.md");
        fs::write(&file, "x").unwrap();
        let renamed = rename(&file, "README.md").unwrap();
        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["README.md".to_string()]);
        assert!(renamed.exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
