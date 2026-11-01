use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Names never shown in the explorer (VS Code hides `.git` by default too).
const HIDDEN: &[&str] = &[".git"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

/// An opened folder with lazily-read, cached directory listings.
pub struct Workspace {
    pub root: PathBuf,
    expanded: HashSet<PathBuf>,
    cache: HashMap<PathBuf, Vec<Entry>>,
}

impl Workspace {
    pub fn new(root: PathBuf) -> Self {
        let mut expanded = HashSet::new();
        expanded.insert(root.clone());
        Self {
            root,
            expanded,
            cache: HashMap::new(),
        }
    }

    pub fn name(&self) -> String {
        self.root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.root.display().to_string())
    }

    pub fn is_expanded(&self, dir: &Path) -> bool {
        self.expanded.contains(dir)
    }

    pub fn toggle(&mut self, dir: &Path) {
        if !self.expanded.remove(dir) {
            self.expanded.insert(dir.to_path_buf());
        }
    }

    pub fn entries(&mut self, dir: &Path) -> Vec<Entry> {
        self.cache
            .entry(dir.to_path_buf())
            .or_insert_with(|| read_dir_sorted(dir))
            .clone()
    }

    /// Forget cached listings so the next frame re-reads the disk.
    pub fn refresh(&mut self) {
        self.cache.clear();
    }
}

/// Directories first, then files, each alphabetically (case-insensitive).
pub fn read_dir_sorted(dir: &Path) -> Vec<Entry> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read
        .filter_map(Result::ok)
        .filter_map(|item| {
            let name = item.file_name().to_string_lossy().into_owned();
            if HIDDEN.contains(&name.as_str()) {
                return None;
            }
            let is_dir = item.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            Some(Entry {
                path: item.path(),
                name,
                is_dir,
            })
        })
        .collect();
    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "forge-room-workspace-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lists_directories_first_sorted_and_hides_git() {
        let dir = temp_dir();
        fs::create_dir(dir.join("src")).unwrap();
        fs::create_dir(dir.join(".git")).unwrap();
        fs::create_dir(dir.join("Assets")).unwrap();
        fs::write(dir.join("b.txt"), "").unwrap();
        fs::write(dir.join("A.txt"), "").unwrap();

        let names: Vec<String> = read_dir_sorted(&dir).into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["Assets", "src", "A.txt", "b.txt"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn toggle_and_cache_refresh() {
        let dir = temp_dir();
        let mut workspace = Workspace::new(dir.clone());
        assert!(workspace.is_expanded(&dir));
        workspace.toggle(&dir);
        assert!(!workspace.is_expanded(&dir));

        assert!(workspace.entries(&dir).is_empty());
        fs::write(dir.join("new.txt"), "").unwrap();
        assert!(workspace.entries(&dir).is_empty(), "listing is cached");
        workspace.refresh();
        assert_eq!(workspace.entries(&dir).len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
