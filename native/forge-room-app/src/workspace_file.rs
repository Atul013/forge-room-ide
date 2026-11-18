use crate::workspace::{MAX_ADDITIONAL_ROOTS, Workspace};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 65536;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceFile {
    version: u32,
    folders: Vec<PathBuf>,
}

pub fn load(path: &Path) -> Result<Vec<PathBuf>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Workspace file exceeds 64 KiB".into());
    }
    let document: WorkspaceFile = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if document.version != 1
        || document.folders.is_empty()
        || document.folders.len() > MAX_ADDITIONAL_ROOTS + 1
    {
        return Err("Unsupported workspace version or root count".into());
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let roots: Result<Vec<_>, _> = document
        .folders
        .iter()
        .map(|root| {
            let root = if root.is_absolute() {
                root.clone()
            } else {
                base.join(root)
            };
            root.canonicalize()
                .map_err(|e| format!("{}: {e}", root.display()))
        })
        .collect();
    validate(roots?)
}

fn validate(roots: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    if roots.is_empty() || roots.len() > MAX_ADDITIONAL_ROOTS + 1 || !roots[0].is_dir() {
        return Err("Invalid workspace roots".into());
    }
    let mut workspace = Workspace::new(roots[0].clone());
    for root in roots.iter().skip(1) {
        if !workspace.add_root(root.clone()) {
            return Err(format!(
                "Duplicate, nested or unavailable root: {}",
                root.display()
            ));
        }
    }
    Ok(roots)
}

pub fn save(path: &Path, roots: &[PathBuf]) -> Result<(), String> {
    let roots = roots
        .iter()
        .map(|p| p.canonicalize().map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let roots = validate(roots)?;
    let base = path
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let document = WorkspaceFile {
        version: 1,
        folders: roots
            .into_iter()
            .map(|root| {
                root.strip_prefix(&base)
                    .map(Path::to_path_buf)
                    .unwrap_or(root)
            })
            .collect(),
    };
    let bytes = serde_json::to_vec_pretty(&document).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Workspace file exceeds 64 KiB".into());
    }
    // Create-only prevents overwriting source files or a concurrently changed workspace.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("Choose a new workspace filename: {e}"))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_roundtrip_and_invalid_roots_preserve_existing_file() {
        let base = std::env::temp_dir().join(format!(
            "forge-workspace-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(base.join("one")).unwrap();
        fs::create_dir_all(base.join("two")).unwrap();
        let path = base.join("test.forge-workspace");
        let roots = vec![base.join("one"), base.join("two")];
        save(&path, &roots).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(&base.display().to_string()));
        assert_eq!(
            load(&path).unwrap(),
            roots
                .iter()
                .map(|p| p.canonicalize().unwrap())
                .collect::<Vec<_>>()
        );
        assert!(save(&path, &roots).is_err());
        assert_eq!(bytes, fs::read(&path).unwrap());
        fs::write(&path, br#"{"version":1,"folders":["one","one"]}"#).unwrap();
        assert!(load(&path).is_err());
        fs::write(&path, br#"{"version":2,"folders":["one"]}"#).unwrap();
        assert!(load(&path).is_err());
        fs::write(&path, vec![b' '; 65537]).unwrap();
        assert!(load(&path).is_err());
        fs::remove_dir_all(base).unwrap();
    }
}
