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
    /// Primary project root. Git, tasks, AI and Room remain scoped here.
    pub root: PathBuf,
    roots: Vec<PathBuf>,
    expanded: HashSet<PathBuf>,
    cache: HashMap<PathBuf, Vec<Entry>>,
}

impl Workspace {
    pub fn new(root: PathBuf) -> Self {
        let mut expanded = HashSet::new();
        expanded.insert(root.clone());
        Self {
            roots: vec![root.clone()],
            root,
            expanded,
            cache: HashMap::new(),
        }
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Add a browse/search root, ignoring duplicate paths (including Windows case/slash aliases).
    pub fn add_root(&mut self, root: PathBuf) -> bool {
        if !root.is_dir()
            || self.roots.iter().any(|known| {
                same_path(known, &root) || root.starts_with(known) || known.starts_with(&root)
            })
        {
            return false;
        }
        self.expanded.insert(root.clone());
        self.roots.push(root);
        self.cache.clear();
        true
    }

    /// Remove a secondary root. The primary project root cannot be removed independently.
    pub fn remove_root(&mut self, root: &Path) -> bool {
        let Some(index) = self.roots.iter().position(|known| same_path(known, root)) else {
            return false;
        };
        if index == 0 {
            return false;
        }
        let removed = self.roots.remove(index);
        self.expanded.retain(|path| !path.starts_with(&removed));
        self.cache.retain(|path, _| !path.starts_with(&removed));
        true
    }

    pub fn label(&self, root: &Path) -> String {
        let name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        if self
            .roots
            .iter()
            .filter(|candidate| candidate.file_name() == root.file_name())
            .count()
            < 2
        {
            return name;
        }
        format!(
            "{} — {}",
            name,
            root.parent()
                .map(|parent| parent.display().to_string())
                .unwrap_or_default()
        )
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

fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.to_string_lossy().replace('/', "\\");
    let right = right.to_string_lossy().replace('/', "\\");
    left.trim_end_matches('\\')
        .eq_ignore_ascii_case(right.trim_end_matches('\\'))
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

    #[test]
    fn secondary_roots_are_deduplicated_removable_and_primary_is_retained() {
        let primary = temp_dir();
        let extra = temp_dir();
        let mut workspace = Workspace::new(primary.clone());
        assert!(workspace.add_root(extra.clone()));
        assert!(!workspace.add_root(extra.clone()));
        assert_eq!(workspace.roots(), &[primary.clone(), extra.clone()]);
        assert!(!workspace.remove_root(&primary));
        assert!(workspace.remove_root(&extra));
        assert_eq!(workspace.roots(), &[primary.clone()]);
        assert!(!workspace.add_root(primary.clone()));
        assert!(!workspace.add_root(primary.join("missing")));
        fs::create_dir(primary.join("nested")).unwrap();
        assert!(!workspace.add_root(primary.join("nested")));
        fs::remove_dir_all(primary).unwrap();
        fs::remove_dir_all(extra).unwrap();
    }

    #[test]
    fn duplicate_folder_names_get_disambiguated_explorer_labels() {
        let base = temp_dir();
        let left = base.join("left").join("src");
        let right = base.join("right").join("src");
        fs::create_dir_all(&left).unwrap();
        fs::create_dir_all(&right).unwrap();
        let mut workspace = Workspace::new(left.clone());
        assert!(workspace.add_root(right.clone()));
        assert_ne!(workspace.label(&left), workspace.label(&right));
        fs::remove_dir_all(base).unwrap();
    }
}

/// Current Git branch (or short commit when detached), read from `.git/HEAD` without running git.
pub fn git_branch(root: &Path) -> Option<String> {
    let head = fs::read_to_string(root.join(".git").join("HEAD")).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref: refs/heads/") {
        Some(branch) => Some(branch.to_string()),
        None if head.len() >= 7 && head.chars().all(|c| c.is_ascii_hexdigit()) => {
            Some(head[..7].to_string())
        }
        None => None,
    }
}

#[cfg(test)]
mod git_tests {
    use super::*;

    fn repo_with_head(contents: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("forge-room-git-{}-{stamp}", std::process::id()));
        fs::create_dir_all(dir.join(".git")).unwrap();
        fs::write(dir.join(".git").join("HEAD"), contents).unwrap();
        dir
    }

    #[test]
    fn reads_branch_name_and_detached_head() {
        let dir = repo_with_head("ref: refs/heads/native-rust-ide\n");
        assert_eq!(git_branch(&dir).as_deref(), Some("native-rust-ide"));
        fs::write(
            dir.join(".git").join("HEAD"),
            "a6bcad7e4f0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d\n",
        )
        .unwrap();
        assert_eq!(git_branch(&dir).as_deref(), Some("a6bcad7"));
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(git_branch(&dir), None);
    }
}
