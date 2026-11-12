//! Protection against losing unsaved typing when the app or the computer dies: the text of every
//! tab with unsaved changes is written to a recovery folder a few seconds after it changes, the
//! folder is emptied when the app exits normally, and whatever a dead instance left behind is
//! brought back as unsaved tabs at the next start.
//!
//! Each running instance keeps its entries in its own folder (`recovery\<id>\`) and holds a lock
//! file open there for as long as it runs. Another instance only takes over a folder whose lock it
//! can delete, which Windows refuses while the file is open, so a second window never steals the
//! first one's unsaved text.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

/// What is kept for one tab.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Entry {
    /// The file the tab was editing, or `None` for an untitled tab.
    pub path: Option<String>,
    pub text: String,
    /// Fingerprint of the file's content as the editor last saw it, so a restore can tell that the
    /// file was changed by something else meanwhile. `None` for untitled tabs.
    #[serde(default)]
    pub disk_hash: Option<u64>,
}

/// A fingerprint of a text, to notice that it changed.
pub fn text_hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

const LOCK: &str = ".lock";

pub struct Store {
    root: PathBuf,
    mine: PathBuf,
    /// Held open for the life of the instance: it marks this folder as in use.
    lock: Option<fs::File>,
}

impl Store {
    /// A store for one running instance, identified by `instance` (the process id).
    pub fn new(root: PathBuf, instance: u32) -> Self {
        let mine = root.join(instance.to_string());
        // Process ids are reused. Entries a dead process of the same number left here would be
        // overwritten by this instance's tabs (they count from 1 again) and deleted on a normal
        // exit, so they are moved aside first and restored with the other orphans.
        if mine.is_dir() && !read_entries(&mine).is_empty() {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let _ = fs::rename(&mine, root.join(format!("{instance}-left-{stamp}")));
        }
        let lock = Self::take_lock(&mine);
        Self { root, mine, lock }
    }

    fn take_lock(folder: &std::path::Path) -> Option<fs::File> {
        fs::create_dir_all(folder).ok()?;
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // Readers and writers may share it, but not delete or rename it: that is the lock.
            options.share_mode(0x1 | 0x2);
        }
        options.open(folder.join(LOCK)).ok()
    }

    /// Beside the settings, so the installer leaves it alone like them.
    pub fn default_location() -> Self {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        Self::new(
            base.join("Forge Room Early Preview").join("recovery"),
            std::process::id(),
        )
    }

    fn file(&self, id: u64) -> PathBuf {
        self.mine.join(format!("tab-{id}.json"))
    }

    /// Write (or replace) the entry for a tab. The file is replaced atomically, so a crash in the
    /// middle never leaves a half-written recovery file.
    pub fn save(&self, id: u64, entry: &Entry) -> io::Result<()> {
        fs::create_dir_all(&self.mine)?;
        let bytes = serde_json::to_vec(entry).map_err(io::Error::other)?;
        crate::document::write_atomic(&self.file(id), &bytes)
    }

    /// Forget one tab's entry (it was saved, or closed). A missing entry is fine.
    pub fn remove(&self, id: u64) {
        let _ = fs::remove_file(self.file(id));
    }

    /// Forget this instance's entries (a normal exit) and release its folder.
    pub fn clear(&mut self) {
        remove_entries(&self.mine);
        // Close the lock first: an open file cannot be deleted.
        self.lock = None;
        let _ = fs::remove_file(self.mine.join(LOCK));
        let _ = fs::remove_dir(&self.mine);
    }

    /// The entries left behind by instances that are no longer running, oldest tab first. Their
    /// folders are removed once read. A folder whose lock is still held belongs to a live instance
    /// and is left alone. Damaged or foreign files are skipped rather than failing the others.
    pub fn take_orphans(&self) -> Vec<Entry> {
        let Ok(folders) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for folder in folders.flatten() {
            let path = folder.path();
            // A link (a junction someone planted) is not an instance's folder: following it would
            // read and delete files wherever it points.
            if crate::file_ops::is_link(&path) || !path.is_dir() || path == self.mine {
                continue;
            }
            // The lock can only be deleted when no instance holds it open. A folder without a lock
            // file was left by an instance that died before taking it.
            let lock = path.join(LOCK);
            if lock.exists() && fs::remove_file(&lock).is_err() {
                continue;
            }
            found.extend(read_entries(&path));
            remove_entries(&path);
            let _ = fs::remove_dir(&path);
        }
        found.sort_by_key(|(id, _)| *id);
        found.into_iter().map(|(_, entry)| entry).collect()
    }

    /// This instance's own entries, for tests and diagnostics.
    #[cfg(test)]
    fn own(&self) -> Vec<Entry> {
        let mut entries = read_entries(&self.mine);
        entries.sort_by_key(|(id, _)| *id);
        entries.into_iter().map(|(_, entry)| entry).collect()
    }
}

fn is_entry_file(name: &str) -> bool {
    name.starts_with("tab-") && name.ends_with(".json")
}

fn remove_entries(folder: &std::path::Path) {
    if let Ok(files) = fs::read_dir(folder) {
        for file in files.flatten() {
            if is_entry_file(&file.file_name().to_string_lossy()) {
                let _ = fs::remove_file(file.path());
            }
        }
    }
}

fn read_entries(folder: &std::path::Path) -> Vec<(u64, Entry)> {
    let Ok(files) = fs::read_dir(folder) else {
        return Vec::new();
    };
    files
        .flatten()
        .filter_map(|file| {
            let name = file.file_name().to_string_lossy().into_owned();
            let id = name
                .strip_prefix("tab-")?
                .strip_suffix(".json")?
                .parse()
                .ok()?;
            let entry = serde_json::from_slice(&fs::read(file.path()).ok()?).ok()?;
            Some((id, entry))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(label: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "forge-recovery-{label}-{}-{stamp}",
            std::process::id()
        ))
    }

    fn entry(path: Option<&str>, text: &str) -> Entry {
        Entry {
            path: path.map(String::from),
            text: text.into(),
            disk_hash: None,
        }
    }

    #[test]
    fn entries_round_trip_in_tab_order_and_are_replaced_in_place() {
        let dir = root("roundtrip");
        let store = Store::new(dir.clone(), 1);
        assert!(store.own().is_empty());
        store
            .save(10, &entry(None, "untitled text\nline 2"))
            .unwrap();
        store
            .save(2, &entry(Some(r"C:\work\a.rs"), "fn a() {}"))
            .unwrap();
        store
            .save(2, &entry(Some(r"C:\work\a.rs"), "fn a() { 1 }"))
            .unwrap();
        assert_eq!(
            store.own(),
            vec![
                entry(Some(r"C:\work\a.rs"), "fn a() { 1 }"),
                entry(None, "untitled text\nline 2")
            ],
            "ordered by tab id (2 before 10), the second save replaced the first"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn removing_one_or_clearing_all_leaves_other_files_alone() {
        let dir = root("remove");
        let mut store = Store::new(dir.clone(), 1);
        store.save(1, &entry(None, "one")).unwrap();
        store.save(2, &entry(None, "two")).unwrap();
        store.remove(1);
        store.remove(99); // never saved: harmless
        assert_eq!(store.own(), vec![entry(None, "two")]);
        fs::write(dir.join("1").join("notes.txt"), "not ours").unwrap();
        store.clear();
        assert!(store.own().is_empty());
        assert!(dir.join("1").join("notes.txt").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_and_foreign_files_are_skipped_not_fatal() {
        let dir = root("damaged");
        let store = Store::new(dir.clone(), 1);
        store.save(1, &entry(None, "good")).unwrap();
        let mine = dir.join("1");
        fs::write(mine.join("tab-2.json"), "{ half a file").unwrap();
        fs::write(
            mine.join("tab-x.json"),
            "{\"path\":null,\"text\":\"bad id\"}",
        )
        .unwrap();
        fs::write(
            mine.join("other.json"),
            "{\"path\":null,\"text\":\"foreign\"}",
        )
        .unwrap();
        assert_eq!(store.own(), vec![entry(None, "good")]);
        let names: Vec<String> = fs::read_dir(&mine)
            .unwrap()
            .map(|file| file.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().all(|name| !name.ends_with(".tmp")),
            "{names:?}"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn large_and_unusual_text_survives() {
        let dir = root("large");
        let store = Store::new(dir.clone(), 1);
        let text = format!(
            "{}\r\nünïcode 日本 \u{1f600}\0 tab\t",
            "x".repeat(2_000_000)
        );
        store.save(1, &entry(None, &text)).unwrap();
        assert_eq!(store.own()[0].text, text);
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_second_instance_never_takes_a_running_instances_text() {
        let dir = root("instances");
        let first = Store::new(dir.clone(), 1);
        first
            .save(1, &entry(None, "first window, unsaved"))
            .unwrap();
        let second = Store::new(dir.clone(), 2);
        // The first instance holds its lock: nothing is taken.
        assert!(second.take_orphans().is_empty());
        assert_eq!(first.own(), vec![entry(None, "first window, unsaved")]);

        // The first instance dies (its lock is released) without clearing: the second takes over.
        drop(first);
        assert_eq!(
            second.take_orphans(),
            vec![entry(None, "first window, unsaved")]
        );
        assert!(
            !dir.join("1").exists(),
            "the orphan folder is gone once taken"
        );
        assert!(second.take_orphans().is_empty(), "taken only once");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_folder_without_a_lock_is_an_orphan_and_a_normal_exit_leaves_nothing() {
        let dir = root("orphans");
        let crashed = dir.join("77");
        fs::create_dir_all(&crashed).unwrap();
        fs::write(
            crashed.join("tab-1.json"),
            serde_json::to_vec(&entry(Some("x.rs"), "left behind")).unwrap(),
        )
        .unwrap();
        let mut store = Store::new(dir.clone(), 5);
        assert_eq!(
            store.take_orphans(),
            vec![entry(Some("x.rs"), "left behind")]
        );
        store.save(1, &entry(None, "mine")).unwrap();
        store.clear();
        assert!(
            !dir.join("5").exists(),
            "a normal exit removes this instance's folder"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_dead_process_with_the_same_id_does_not_lose_its_text() {
        let dir = root("pid-reuse");
        // A process numbered 9 crashed and left tab 1 behind; a new process gets the number 9 too.
        let old = dir.join("9");
        fs::create_dir_all(&old).unwrap();
        fs::write(
            old.join("tab-1.json"),
            serde_json::to_vec(&entry(Some("old.rs"), "old work")).unwrap(),
        )
        .unwrap();
        let mut store = Store::new(dir.clone(), 9);
        store.save(1, &entry(Some("new.rs"), "new work")).unwrap();
        // The new instance's tab 1 did not replace the old one, and a normal exit keeps the old text.
        assert_eq!(store.own(), vec![entry(Some("new.rs"), "new work")]);
        store.clear();
        let next = Store::new(dir.clone(), 10);
        assert_eq!(next.take_orphans(), vec![entry(Some("old.rs"), "old work")]);
        let _ = fs::remove_dir_all(dir);
    }

    /// A junction `link` -> `target` (no privilege needed on Windows); None when it cannot be made.
    #[cfg(windows)]
    fn junction(link: &std::path::Path, target: &std::path::Path) -> Option<()> {
        use std::os::windows::process::CommandExt;
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .creation_flags(0x0800_0000)
            .output()
            .ok()?;
        made.status.success().then_some(())
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_in_the_recovery_folder_is_not_followed_or_emptied() {
        let dir = root("junction");
        fs::create_dir_all(&dir).unwrap();
        let outside = dir.join("outside-folder");
        fs::create_dir_all(&outside).unwrap();
        fs::write(
            outside.join("tab-1.json"),
            serde_json::to_vec(&entry(None, "outside")).unwrap(),
        )
        .unwrap();
        fs::write(outside.join(LOCK), "").unwrap();
        let recovery = dir.join("recovery");
        fs::create_dir_all(&recovery).unwrap();
        if junction(&recovery.join("77"), &outside).is_none() {
            let _ = fs::remove_dir_all(dir);
            return; // junctions cannot be made here
        }
        let store = Store::new(recovery.clone(), 5);
        let taken = store.take_orphans();
        assert!(
            taken.is_empty(),
            "the outside folder's text was taken: {taken:?}"
        );
        assert!(
            outside.join("tab-1.json").exists(),
            "a file outside was deleted"
        );
        assert!(outside.join(LOCK).exists(), "a file outside was deleted");
        drop(store);
        let _ = fs::remove_dir_all(dir);
    }
}
