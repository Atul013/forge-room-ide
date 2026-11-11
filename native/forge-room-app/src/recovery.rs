//! Protection against losing unsaved typing when the app or the computer dies: the text of every
//! tab with unsaved changes is written to a recovery folder a few seconds after it changes, the
//! folder is emptied when the app exits normally, and whatever is found there at the next start is
//! brought back as unsaved tabs.

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
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Beside the settings, so the installer leaves it alone like them.
    pub fn default_location() -> Self {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        Self::new(base.join("Forge Room Early Preview").join("recovery"))
    }

    fn file(&self, id: u64) -> PathBuf {
        self.dir.join(format!("tab-{id}.json"))
    }

    /// Write (or replace) the entry for a tab. The file is replaced atomically, so a crash in the
    /// middle never leaves a half-written recovery file.
    pub fn save(&self, id: u64, entry: &Entry) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let bytes = serde_json::to_vec(entry).map_err(io::Error::other)?;
        crate::document::write_atomic(&self.file(id), &bytes)
    }

    /// Forget one tab's entry (it was saved, or closed). A missing entry is fine.
    pub fn remove(&self, id: u64) {
        let _ = fs::remove_file(self.file(id));
    }

    /// Forget everything (a normal exit, or after the entries were restored).
    pub fn clear(&self) {
        if let Ok(entries) = fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("tab-") && name.ends_with(".json") {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    /// The entries left behind, oldest tab first. Files that cannot be read or are damaged (a
    /// crash while writing, a hand edit) are skipped rather than failing the others.
    pub fn load_all(&self) -> Vec<Entry> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut found: Vec<(u64, Entry)> = entries
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
            .collect();
        found.sort_by_key(|(id, _)| *id);
        found.into_iter().map(|(_, entry)| entry).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(label: &str) -> (Store, PathBuf) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "forge-recovery-{label}-{}-{stamp}",
            std::process::id()
        ));
        (Store::new(dir.clone()), dir)
    }

    fn entry(path: Option<&str>, text: &str) -> Entry {
        Entry {
            path: path.map(String::from),
            text: text.into(),
        }
    }

    #[test]
    fn entries_round_trip_in_tab_order_and_are_replaced_in_place() {
        let (store, dir) = store("roundtrip");
        assert!(store.load_all().is_empty(), "no folder yet is not an error");
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
            store.load_all(),
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
        let (store, dir) = store("remove");
        store.save(1, &entry(None, "one")).unwrap();
        store.save(2, &entry(None, "two")).unwrap();
        store.remove(1);
        store.remove(99); // never saved: harmless
        assert_eq!(store.load_all(), vec![entry(None, "two")]);
        fs::write(dir.join("notes.txt"), "not ours").unwrap();
        store.clear();
        assert!(store.load_all().is_empty());
        assert!(
            dir.join("notes.txt").exists(),
            "only recovery files are removed"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_and_foreign_files_are_skipped_not_fatal() {
        let (store, dir) = store("damaged");
        store.save(1, &entry(None, "good")).unwrap();
        fs::write(dir.join("tab-2.json"), "{ half a file").unwrap();
        fs::write(
            dir.join("tab-x.json"),
            "{\"path\":null,\"text\":\"bad id\"}",
        )
        .unwrap();
        fs::write(
            dir.join("other.json"),
            "{\"path\":null,\"text\":\"foreign\"}",
        )
        .unwrap();
        assert_eq!(store.load_all(), vec![entry(None, "good")]);
        // No temporary file is left behind by saving.
        let names: Vec<String> = fs::read_dir(&dir)
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
        let (store, dir) = store("large");
        let text = format!(
            "{}\r\nünïcode 日本 \u{1f600}\0 tab\t",
            "x".repeat(2_000_000)
        );
        store.save(1, &entry(None, &text)).unwrap();
        assert_eq!(store.load_all()[0].text, text);
        let _ = fs::remove_dir_all(dir);
    }
}
