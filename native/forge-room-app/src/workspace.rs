use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Names never shown in the explorer (VS Code hides `.git` by default too).
const HIDDEN: &[&str] = &[".git"];
/// Session restore keeps at most this many secondary roots.
pub const MAX_ADDITIONAL_ROOTS: usize = 15;

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
        let root = crate::tabs::normalize(&root);
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
        let root = crate::tabs::normalize(&root);
        if self.roots.len() > MAX_ADDITIONAL_ROOTS
            || !root.is_dir()
            || self.roots.iter().any(|known| roots_overlap(known, &root))
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

    /// Validate an explicit project choice and return its persisted root order.
    pub fn activation_roots(&self, root: &Path) -> Option<Vec<PathBuf>> {
        let selected = self.roots.iter().find(|known| same_path(known, root))?;
        if !selected.is_dir() {
            return None;
        }
        let mut roots = vec![selected.clone()];
        roots.extend(
            self.roots
                .iter()
                .filter(|known| *known != selected)
                .cloned(),
        );
        Some(roots)
    }

    pub fn containing_root(&self, path: &Path) -> Option<&Path> {
        self.roots
            .iter()
            .find(|root| path.starts_with(root))
            .map(PathBuf::as_path)
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

fn roots_overlap(left: &Path, right: &Path) -> bool {
    let key = |path: &Path| {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        #[cfg(windows)]
        let canonical = PathBuf::from(canonical.to_string_lossy().to_lowercase());
        canonical
    };
    let left = key(left);
    let right = key(right);
    left.starts_with(&right) || right.starts_with(&left)
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = crate::tabs::normalize(left)
        .to_string_lossy()
        .replace('/', "\\");
    let right = crate::tabs::normalize(right)
        .to_string_lossy()
        .replace('/', "\\");
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
        crate::tabs::normalize(&dir)
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
        assert_eq!(workspace.roots().len(), 1);
        assert_eq!(workspace.roots()[0], primary);
        assert!(!workspace.add_root(primary.clone()));
        assert!(!workspace.add_root(primary.join("missing")));
        fs::create_dir(primary.join("nested")).unwrap();
        assert!(!workspace.add_root(primary.join("nested")));
        fs::remove_dir_all(primary).unwrap();
        fs::remove_dir_all(extra).unwrap();
    }

    #[test]
    fn secondary_roots_match_the_session_restore_limit() {
        let parent = temp_dir();
        let primary = parent.join("primary");
        fs::create_dir(&primary).unwrap();
        let mut workspace = Workspace::new(primary);

        for index in 0..MAX_ADDITIONAL_ROOTS {
            let root = parent.join(format!("extra-{index}"));
            fs::create_dir(&root).unwrap();
            assert!(workspace.add_root(root));
        }

        let overflow = parent.join("overflow");
        fs::create_dir(&overflow).unwrap();
        assert!(!workspace.add_root(overflow));
        assert_eq!(workspace.roots().len(), MAX_ADDITIONAL_ROOTS + 1);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn resolved_aliases_cannot_duplicate_or_nest_roots() {
        let base = temp_dir();
        let project = base.join("project");
        fs::create_dir_all(project.join("child")).unwrap();
        let mut workspace = Workspace::new(project.clone());
        assert!(!workspace.add_root(project.join("child").join("..")));
        assert!(!workspace.add_root(project.join("child")));
        assert!(!workspace.add_root(base.clone()));
        #[cfg(windows)]
        assert!(!workspace.add_root(PathBuf::from(
            project.join("child").to_string_lossy().to_uppercase()
        )));
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn secondary_alias_uses_tab_identity_for_activation_and_relative_paths() {
        let base = temp_dir();
        let primary = base.join("primary");
        let secondary = base.join("secondary");
        fs::create_dir(&primary).unwrap();
        fs::create_dir(&secondary).unwrap();
        fs::create_dir(base.join("alias-parent")).unwrap();
        let mut workspace = Workspace::new(primary);
        assert!(workspace.add_root(base.join("alias-parent").join("..").join("secondary")));
        let normalized = crate::tabs::normalize(&secondary);
        assert_eq!(workspace.roots()[1], normalized);
        assert!(workspace.activation_roots(&normalized).is_some());
        assert_eq!(
            workspace.containing_root(&normalized.join("file.rs")),
            Some(normalized.as_path())
        );
        fs::remove_dir_all(base).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_short_paths_keep_canonical_root_identity_and_activation() {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetShortPathNameW(long: *const u16, short: *mut u16, size: u32) -> u32;
        }
        let base = temp_dir();
        let primary = base.join("long-primary-project");
        let secondary = base.join("long-secondary-project");
        fs::create_dir(&primary).unwrap();
        fs::create_dir(&secondary).unwrap();
        let short = |path: &Path| {
            let long: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let mut buffer = vec![0u16; 32768];
            let length = unsafe {
                GetShortPathNameW(long.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32)
            };
            assert!(length > 0 && (length as usize) < buffer.len());
            PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize]))
        };
        let mut workspace = Workspace::new(short(&primary));
        assert!(workspace.add_root(short(&secondary)));
        assert_eq!(workspace.root, crate::tabs::normalize(&primary));
        assert_eq!(
            workspace.activation_roots(&short(&secondary)),
            Some(vec![
                crate::tabs::normalize(&secondary),
                crate::tabs::normalize(&primary)
            ])
        );
        assert_eq!(
            workspace.containing_root(&crate::tabs::normalize(&secondary).join("file.rs")),
            Some(crate::tabs::normalize(&secondary).as_path())
        );
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn activating_a_root_retains_all_roots_and_rejects_unavailable_choices() {
        let base = temp_dir();
        let first = base.join("one");
        let second = base.join("two");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        let mut workspace = Workspace::new(first.clone());
        assert!(workspace.add_root(second.clone()));
        assert_eq!(
            workspace.activation_roots(&second),
            Some(vec![second.clone(), first.clone()])
        );
        assert_eq!(workspace.roots(), &[first, second.clone()]);
        assert!(workspace.activation_roots(&base).is_none());
        fs::remove_dir(&second).unwrap();
        assert!(workspace.activation_roots(&second).is_none());
        fs::remove_dir_all(base).unwrap();
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
