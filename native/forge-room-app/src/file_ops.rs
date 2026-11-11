//! File operations from the Explorer that need rules of their own: names that are valid on every
//! platform we may be used on (and on Windows in particular), and renaming without overwriting
//! anything.

use std::fs;
use std::path::{Path, PathBuf};

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// `path` relative to `root` for copying (`src\\main.rs` on Windows); the full path when it is not
/// inside `root`. Both are compared in their normal form, so a `\\?\` prefix does not matter.
pub fn relative_display(root: &Path, path: &Path) -> String {
    let (root, full) = (crate::tabs::normalize(root), crate::tabs::normalize(path));
    full.strip_prefix(&root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map_or_else(
            || path.display().to_string(),
            |relative| relative.display().to_string(),
        )
}

/// Show a file in Windows File Explorer with it selected (a folder is opened). The path is passed as
/// one argument, never through a shell.
pub fn reveal(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("That file no longer exists".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Explorer wants `/select,"C:\path with spaces"`: the quotes belong to the path alone.
        let argument = if path.is_dir() {
            format!("\"{}\"", path.display())
        } else {
            format!("/select,\"{}\"", path.display())
        };
        std::process::Command::new("explorer.exe")
            .raw_arg(argument)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Could not open File Explorer: {error}"))
    }
    #[cfg(not(windows))]
    {
        Err("Showing a file in the file manager is only available on Windows".into())
    }
}

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
    // COM1-3 and LPT1-3 also exist as superscript digits, which Windows reserves as well.
    let upper = stem.to_uppercase();
    if let Some(digit) = upper
        .strip_prefix("COM")
        .or_else(|| upper.strip_prefix("LPT"))
        && matches!(digit, "\u{b9}" | "\u{b2}" | "\u{b3}")
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
    // `symlink_metadata`, so even a dangling symbolic link counts as something that is there.
    if fs::symlink_metadata(&target).is_ok() && !is_same_entry(from, &target) {
        return Err(format!("\"{name}\" already exists here"));
    }
    fs::rename(from, &target).map_err(|error| format!("Could not rename: {error}"))?;
    Ok(target)
}

/// Move a file or folder to the Recycle Bin, so deleting it can be undone from Windows. Nothing is
/// ever deleted for good: when the Recycle Bin cannot be used the call fails and the file stays. The
/// path travels in an environment variable, never inside the command text, so no file name can
/// change what the command does.
pub fn recycle(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("That file no longer exists".into());
    }
    #[cfg(windows)]
    {
        use std::io::Read;
        use std::os::windows::fs::MetadataExt;
        use std::os::windows::process::CommandExt;
        // A folder that is a link (a junction or symbolic link) is not recycled from here: the
        // call could treat the folder it points to as the thing to delete.
        let attributes = fs::symlink_metadata(path)
            .map_err(|error| format!("Could not read it: {error}"))?
            .file_attributes();
        const DIRECTORY: u32 = 0x10;
        const REPARSE_POINT: u32 = 0x400;
        if attributes & DIRECTORY != 0 && attributes & REPARSE_POINT != 0 {
            return Err("That folder is a link: remove the link itself in File Explorer".into());
        }
        let method = if path.is_dir() {
            "DeleteDirectory"
        } else {
            "DeleteFile"
        };
        // On a drive without a Recycle Bin (a network share, a USB stick) Windows deletes for good
        // when asked to recycle, so only fixed local drives are accepted.
        let script = format!(
            "$root = [System.IO.Path]::GetPathRoot($env:FORGE_RECYCLE); \
             $drive = $null; \
             if ($root -match '^[A-Za-z]:') {{ $drive = New-Object System.IO.DriveInfo($root) }}; \
             if ($null -eq $drive -or $drive.DriveType -ne 'Fixed') {{ [Console]::Error.WriteLine('The Recycle Bin is only \
             used for files on a fixed local drive; nothing was deleted'); exit 3 }}; \
             Add-Type -AssemblyName Microsoft.VisualBasic; \
             [Microsoft.VisualBasic.FileIO.FileSystem]::{method}($env:FORGE_RECYCLE, \
             'OnlyErrorDialogs', 'SendToRecycleBin')"
        );
        let mut child = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .env("FORGE_RECYCLE", path)
            .creation_flags(0x0800_0000)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| format!("Could not run PowerShell: {error}"))?;
        // stderr is drained on its own thread so a long message can never block the helper.
        let mut pipe = child.stderr.take();
        let reader = std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(pipe) = &mut pipe {
                let _ = pipe.read_to_string(&mut text);
            }
            text
        });
        // A dialog nobody can see (the window is hidden) must not keep this waiting for ever.
        let started = std::time::Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() > std::time::Duration::from_secs(60) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("The Recycle Bin did not answer in time; it was cancelled".into());
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(40)),
                Err(error) => return Err(format!("Could not wait for PowerShell: {error}")),
            }
        };
        let text = reader.join().unwrap_or_default();
        if status.success() && !path.exists() {
            Ok(())
        } else if text.trim().is_empty() {
            Err("Could not move it to the Recycle Bin".into())
        } else {
            Err(text
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("")
                .to_string())
        }
    }
    #[cfg(not(windows))]
    {
        Err("Moving to the Recycle Bin is only available on Windows".into())
    }
}

/// Whether `target` is `from` under a name that differs only in case: the same folder and the
/// same letters. Links are deliberately not followed (a link renamed onto the file it points at is
/// a clash, not a case change).
fn is_same_entry(from: &Path, target: &Path) -> bool {
    let (Some(a), Some(b)) = (from.file_name(), target.file_name()) else {
        return false;
    };
    if a.to_string_lossy().to_lowercase() != b.to_string_lossy().to_lowercase() {
        return false;
    }
    match (
        from.parent().map(fs::canonicalize),
        target.parent().map(fs::canonicalize),
    ) {
        (Some(Ok(a)), Some(Ok(b))) => a == b,
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

    /// Whether the Recycle Bin holds an item whose name contains `needle`; with `purge` the item is
    /// removed from the bin again, so the test leaves nothing behind.
    #[cfg(windows)]
    fn in_recycle_bin(needle: &str, purge: bool) -> bool {
        let script = "$bin = (New-Object -ComObject Shell.Application).NameSpace(0xA); \
             $found = $false; \
             foreach ($item in $bin.Items()) { if ($item.Name -like ('*' + $env:NEEDLE + '*')) { \
             $found = $true; if ($env:PURGE -eq '1') { \
             Remove-Item -LiteralPath $item.Path -Recurse -Force -ErrorAction SilentlyContinue } } }; \
             if ($found) { exit 0 } else { exit 1 }";
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env("NEEDLE", needle)
            .env("PURGE", if purge { "1" } else { "0" })
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    #[test]
    fn files_and_folders_go_to_the_recycle_bin_and_odd_names_are_safe() {
        let dir = folder("recycle");
        let stamp = format!(
            "zz{}x{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        // A name with characters that would break a command line if it were pasted into one.
        let file = dir.join(format!("it's $(odd) `name` & {stamp}.txt"));
        fs::write(&file, "x").unwrap();
        recycle(&file).unwrap();
        assert!(!file.exists());
        assert!(
            in_recycle_bin(&stamp, false),
            "the file really is in the Recycle Bin"
        );
        in_recycle_bin(&stamp, true);

        let sub = dir.join(format!("sub folder {stamp}b"));
        fs::create_dir_all(sub.join("inner")).unwrap();
        fs::write(sub.join("inner").join("a.txt"), "y").unwrap();
        recycle(&sub).unwrap();
        assert!(!sub.exists());
        assert!(in_recycle_bin(&format!("{stamp}b"), false));
        in_recycle_bin(&format!("{stamp}b"), true);

        // Something that is not there is an error, and nothing else is touched.
        fs::write(dir.join("keep.txt"), "k").unwrap();
        assert!(recycle(&dir.join("missing.txt")).is_err());
        assert!(dir.join("keep.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn relative_paths_are_inside_the_folder_or_the_full_path() {
        let dir = folder("relative");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src").join("a.rs"), "x").unwrap();
        let expected = Path::new("src").join("a.rs").display().to_string();
        assert_eq!(
            relative_display(&dir, &dir.join("src").join("a.rs")),
            expected
        );
        // Outside the folder, or the folder itself: the path as it is.
        let outside = std::env::temp_dir().join("elsewhere.txt");
        assert_eq!(
            relative_display(&dir, &outside),
            outside.display().to_string()
        );
        assert_eq!(relative_display(&dir, &dir), dir.display().to_string());
        assert!(reveal(&dir.join("missing")).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_superscript_device_names_are_reserved_too() {
        for bad in ["COM\u{b9}", "com\u{b2}.txt", "LPT\u{b3}"] {
            assert!(valid_name(bad).is_err(), "{bad:?}");
        }
        assert!(valid_name("COM\u{b4}").is_ok());
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
