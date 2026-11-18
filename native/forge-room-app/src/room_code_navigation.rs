//! Resolves shared Room code references against the receiver's current workspace.
use forge_room_core::transport::CodeReference;
use std::path::{Path, PathBuf};

pub(crate) fn resolve(root: &Path, reference: &CodeReference) -> Result<(PathBuf, usize), String> {
    if reference.line == 0 || reference.line > 10_000_000 {
        return Err("The shared location has an invalid line number".into());
    }
    let path = reference.path.as_str();
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path.chars().any(char::is_control)
    {
        return Err("The shared path is not a workspace-relative path".into());
    }
    let mut relative = PathBuf::new();
    for component in path.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err("The shared path contains an unsafe component".into());
        }
        relative.push(component);
    }
    let canonical_root = root.canonicalize().map_err(|error| error.to_string())?;
    let absolute = canonical_root.join(relative);
    let checked = crate::ai_workflow::checked(&canonical_root, &absolute)?;
    Ok((checked, reference.line as usize))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "forge-room-navigation-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn reference(path: &str, line: u32) -> CodeReference {
        CodeReference {
            path: path.into(),
            line,
        }
    }

    #[test]
    fn resolves_existing_local_workspace_file_and_line() {
        let temp = Temp::new();
        fs::create_dir_all(temp.0.join("src")).unwrap();
        fs::write(temp.0.join("src/main.rs"), "fn main() {}\n").unwrap();
        let (path, line) = resolve(&temp.0, &reference("src/main.rs", 1)).unwrap();
        assert_eq!(path, temp.0.join("src/main.rs").canonicalize().unwrap());
        assert_eq!(line, 1);
    }

    #[test]
    fn rejects_traversal_absolute_streams_credentials_and_missing_files() {
        let temp = Temp::new();
        for path in [
            "../secret",
            "/etc/passwd",
            "C:/secret",
            "src\\main.rs",
            "file:secret",
            ".env",
            "id_ed25519",
            "missing.rs",
        ] {
            assert!(
                resolve(&temp.0, &reference(path, 1)).is_err(),
                "accepted {path}"
            );
        }
    }

    #[test]
    fn rejects_invalid_lines_and_linked_files() {
        let temp = Temp::new();
        fs::write(temp.0.join("source.rs"), "fn main() {}\n").unwrap();
        assert!(resolve(&temp.0, &reference("source.rs", 0)).is_err());
        assert!(resolve(&temp.0, &reference("source.rs", u32::MAX)).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(temp.0.join("source.rs"), temp.0.join("linked.rs")).unwrap();
            assert!(resolve(&temp.0, &reference("linked.rs", 1)).is_err());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_file;
            if symlink_file(temp.0.join("source.rs"), temp.0.join("linked.rs")).is_ok() {
                assert!(resolve(&temp.0, &reference("linked.rs", 1)).is_err());
            }
        }
    }
}
