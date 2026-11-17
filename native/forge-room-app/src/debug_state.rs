//! Per-workspace debugger breakpoints, stored outside the project.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

const MAX_FILE: u64 = 128 * 1024;
const MAX_SOURCES: usize = 128;
const MAX_LINES: usize = 100;

#[derive(Default, Deserialize, Serialize)]
struct Saved {
    version: u32,
    breakpoints: std::collections::BTreeMap<String, Vec<usize>>,
}

fn path(root: &Path) -> PathBuf {
    crate::room_state::store_for(&crate::tabs::normalize(root))
        .path()
        .with_extension("debug.json")
}

pub fn load(root: &Path) -> HashMap<PathBuf, Vec<usize>> {
    let file = path(root);
    if fs::metadata(&file).map_or(true, |meta| meta.len() > MAX_FILE) {
        return HashMap::new();
    }
    let Ok(bytes) = fs::read(file) else {
        return HashMap::new();
    };
    let Ok(saved) = serde_json::from_slice::<Saved>(&bytes) else {
        return HashMap::new();
    };
    if saved.version != 1 {
        return HashMap::new();
    }
    let Ok(root) = root.canonicalize() else {
        return HashMap::new();
    };
    saved
        .breakpoints
        .into_iter()
        .take(MAX_SOURCES)
        .filter_map(|(relative, mut lines)| {
            let relative = Path::new(&relative);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return None;
            }
            let file = root.join(relative).canonicalize().ok()?;
            if !file.starts_with(&root) || !file.is_file() {
                return None;
            }
            lines.retain(|line| *line > 0 && *line <= 1_000_000);
            lines.sort_unstable();
            lines.dedup();
            lines.truncate(MAX_LINES);
            (!lines.is_empty()).then_some((file, lines))
        })
        .collect()
}

pub fn save(root: &Path, breakpoints: &HashMap<PathBuf, Vec<usize>>) -> io::Result<()> {
    let root = root.canonicalize()?;
    let mut saved = Saved {
        version: 1,
        ..Saved::default()
    };
    for (file, lines) in breakpoints {
        if saved.breakpoints.len() >= MAX_SOURCES {
            break;
        }
        let Ok(file) = file.canonicalize() else {
            continue;
        };
        let Ok(relative) = file.strip_prefix(&root) else {
            continue;
        };
        if !file.is_file() {
            continue;
        }
        let mut lines: Vec<_> = lines
            .iter()
            .copied()
            .filter(|line| *line > 0 && *line <= 1_000_000)
            .take(MAX_LINES)
            .collect();
        lines.sort_unstable();
        lines.dedup();
        if !lines.is_empty() {
            saved
                .breakpoints
                .insert(relative.to_string_lossy().into_owned(), lines);
        }
    }
    let bytes = serde_json::to_vec(&saved).map_err(io::Error::other)?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Too many breakpoints",
        ));
    }
    let file = path(&root);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::document::write_atomic(&file, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_breakpoints_round_trip_without_external_paths() {
        let root = std::env::temp_dir().join(format!("forge-debug-state-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let inside = root.join("main.py");
        let outside = root
            .parent()
            .unwrap()
            .join(format!("forge-debug-outside-{}.py", std::process::id()));
        fs::write(&inside, "print(1)").unwrap();
        fs::write(&outside, "print(2)").unwrap();
        let values = HashMap::from([(inside.clone(), vec![5, 2, 5]), (outside.clone(), vec![1])]);
        save(&root, &values).unwrap();
        let loaded = load(&root);
        assert_eq!(loaded.len(), 1);
        assert_eq!(
            loaded.get(&inside.canonicalize().unwrap()),
            Some(&vec![2, 5])
        );
        let _ = fs::remove_file(path(&root));
        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir_all(&root);
    }
}
