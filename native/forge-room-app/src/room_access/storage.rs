//! Private metadata only. Credentials, full certificates, sessions and message bodies have no fields.
use crate::local_records;
use forge_room_core::workflows;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};
pub const MAX_PROFILE: usize = 128 * 1024;
const MAX_CATALOG: u64 = 512 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub room: String,
    pub room_id: String,
    pub address: SocketAddr,
    pub fingerprint: String,
    pub label: String,
}
impl Profile {
    pub fn id(&self) -> Result<String, String> {
        workflows::seal(&(
            self.room_id.to_ascii_lowercase(),
            self.address,
            self.fingerprint.to_ascii_lowercase(),
        ))
    }
    pub fn validate(&self) -> Result<(), String> {
        for (text, max, required) in [(&self.room, 80, true), (&self.label, 160, false)] {
            if text.len() > max
                || required && text.trim().is_empty()
                || text.chars().any(char::is_control)
            {
                return Err("Invalid bookmark text".into());
            }
        }
        if (!self.room_id.is_empty()
            && (self.room_id.len() != 64 || !self.room_id.bytes().all(|b| b.is_ascii_hexdigit())))
            || self.fingerprint.len() != 64
            || !self.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
            || self.address.port() == 0
            || self.address.ip().is_unspecified()
            || self.address.ip().is_multicast()
        {
            return Err("Invalid public Room identity/address/fingerprint".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profiles {
    pub version: u32,
    pub entries: Vec<Profile>,
}
impl Profiles {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.entries.len() > 64 {
            return Err("Bookmark profile needs version 1 and at most 64 entries".into());
        }
        let mut ids = std::collections::HashSet::new();
        for e in &self.entries {
            e.validate()?;
            if !ids.insert(e.id()?) {
                return Err("Duplicate bookmark profile".into());
            }
        }
        Ok(())
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_PROFILE {
            return Err("Bookmark import exceeds 128 KiB".into());
        }
        let mut value: Self = serde_json::from_slice(bytes).map_err(|e| {
            format!(
                "Invalid bookmark schema at line {}, column {}",
                e.line(),
                e.column()
            )
        })?;
        for entry in &mut value.entries {
            entry.room_id.make_ascii_lowercase();
            entry.fingerprint.make_ascii_lowercase();
        }
        value.validate()?;
        Ok(value)
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_PROFILE {
            return Err("Bookmark export exceeds 128 KiB".into());
        }
        Ok(bytes)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub profile: Profile,
    pub project: Option<String>,
    pub pinned: bool,
    pub joined_at: Option<u64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub version: u32,
    pub revision: u64,
    pub entries: Vec<Entry>,
}
impl Index {
    fn empty() -> Self {
        Self {
            version: 1,
            ..Default::default()
        }
    }
    fn validate(&self) -> Result<(), String> {
        Profiles {
            version: self.version,
            entries: self.entries.iter().map(|e| e.profile.clone()).collect(),
        }
        .validate()?;
        if self.entries.iter().any(|e| {
            e.project
                .as_ref()
                .is_some_and(|s| s.len() > 4096 || s.chars().any(char::is_control))
        }) {
            return Err("Invalid private bookmark metadata".into());
        }
        Ok(())
    }
    pub fn seal(&self) -> Result<String, String> {
        workflows::seal(self)
    }
    pub fn public(&self) -> Profiles {
        Profiles {
            version: 1,
            entries: self.entries.iter().map(|e| e.profile.clone()).collect(),
        }
    }
}
pub enum Edit {
    Visited(Profile, Option<String>, u64),
    Import(Profiles),
    Label(String, String),
    Pin(String, bool),
    Forget(String),
}
pub struct Store {
    pub root: PathBuf,
    pub index: Index,
    pub warning: String,
}
impl Store {
    pub fn default_root() -> Result<PathBuf, String> {
        Ok(PathBuf::from(
            std::env::var_os("LOCALAPPDATA").ok_or("Local application data unavailable")?,
        )
        .join("Forge Room Early Preview")
        .join("room-connections"))
    }
    pub fn open(root: &Path) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        local_records::directory(root)?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let mut store = Self {
            root,
            index: Index::empty(),
            warning: String::new(),
        };
        store.reload()?;
        Ok(store)
    }
    fn records(&self) -> Result<Vec<(u64, PathBuf)>, String> {
        local_records::directory(&self.root)?;
        let mut records = vec![];
        for (i, e) in fs::read_dir(&self.root)
            .map_err(|e| e.to_string())?
            .enumerate()
        {
            if i >= 256 {
                return Err("Connection catalog scan exceeds 256 entries".into());
            }
            let path = e.map_err(|e| e.to_string())?.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if let Some(stem) = name.strip_suffix(".frconnections")
                && stem.len() == 20
                && stem.bytes().all(|b| b.is_ascii_digit())
            {
                records.push((stem.parse().map_err(|_| "Catalog sequence overflow")?, path));
            }
        }
        records.sort_by_key(|(n, _)| *n);
        Ok(records)
    }
    fn reload(&mut self) -> Result<(), String> {
        self.warning.clear();
        let records = self.records()?;
        for (_, path) in records.iter().rev() {
            match local_records::read::<Index>(path,MAX_CATALOG){Ok(index)=>{index.validate()?;self.index=index;return Ok(());},Err(e) if e==local_records::INCOMPLETE=>{self.warning="Incomplete catalog tail preserved; displaying last complete version".into();},Err(_)=>return Err("Complete connection catalog corruption; files preserved, direct invite joining remains available".into())}
        }
        self.index = Index::empty();
        Ok(())
    }
    pub fn edit(&mut self, expected: Option<&str>, edit: Edit) -> Result<(), String> {
        local_records::directory(&self.root)?;
        let lock_path = self.root.join(".writer");
        if lock_path.exists() {
            local_records::file(&lock_path)?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|e| e.to_string())?;
        file.try_lock()
            .map_err(|_| "Connection catalog writer is busy")?;
        self.reload()?;
        if expected.is_some_and(|seal| self.index.seal().ok().as_deref() != Some(seal)) {
            return Err("Connection catalog changed; refresh and review again".into());
        }
        let mut next = self.index.clone();
        match edit {
            Edit::Visited(profile, project, at) => {
                profile.validate()?;
                let id = profile.id()?;
                if let Some(entry) = next
                    .entries
                    .iter_mut()
                    .find(|e| e.profile.id().ok().as_deref() == Some(&id))
                {
                    entry.joined_at = Some(entry.joined_at.map_or(at, |old| old.max(at)));
                    if project.is_some() {
                        entry.project = project;
                    }
                } else {
                    if next.entries.len() >= 64 {
                        return Err("Bookmark catalog is full (64); joining succeeded but metadata was not saved".into());
                    }
                    next.entries.push(Entry {
                        profile,
                        project,
                        pinned: false,
                        joined_at: Some(at),
                    });
                }
            }
            Edit::Import(profiles) => {
                profiles.validate()?;
                for profile in profiles.entries {
                    let id = profile.id()?;
                    if !next
                        .entries
                        .iter()
                        .any(|e| e.profile.id().ok().as_deref() == Some(&id))
                    {
                        next.entries.push(Entry {
                            profile,
                            project: None,
                            pinned: false,
                            joined_at: None,
                        });
                    }
                }
            }
            Edit::Label(id, label) => {
                let entry = next
                    .entries
                    .iter_mut()
                    .find(|e| e.profile.id().ok().as_deref() == Some(&id))
                    .ok_or("Bookmark no longer exists")?;
                entry.profile.label = label;
            }
            Edit::Pin(id, pinned) => {
                let entry = next
                    .entries
                    .iter_mut()
                    .find(|e| e.profile.id().ok().as_deref() == Some(&id))
                    .ok_or("Bookmark no longer exists")?;
                entry.pinned = pinned;
            }
            Edit::Forget(id) => {
                if !next
                    .entries
                    .iter()
                    .any(|e| e.profile.id().ok().as_deref() == Some(&id))
                {
                    return Err("Bookmark no longer exists".into());
                }
                next.entries
                    .retain(|e| e.profile.id().ok().as_deref() != Some(&id));
            }
        }
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or("Catalog revision exhausted")?;
        next.validate()?;
        let records = self.records()?;
        let sequence = records.last().map_or(Ok(1), |(n, _)| {
            n.checked_add(1).ok_or("Catalog sequence exhausted")
        })?;
        local_records::write_new(
            &self.root.join(format!("{sequence:020}.frconnections")),
            MAX_CATALOG,
            &next,
        )?;
        self.index = next;
        self.warning.clear();
        // Only old, known regular catalog records are pruned, after the new synced record exists.
        let records = self.records()?;
        for (_, path) in records.iter().take(records.len().saturating_sub(8)) {
            if local_records::file(path).is_ok()
                && let Err(e) = fs::remove_file(path)
            {
                self.warning =
                    format!("Bookmark saved; old catalog retention could not finish: {e}");
            }
        }
        drop(file);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> Profile {
        Profile {
            room: "Room".into(),
            room_id: "a".repeat(64),
            address: "127.0.0.1:1000".parse().unwrap(),
            fingerprint: "b".repeat(64),
            label: "Saved".into(),
        }
    }
    #[test]
    fn private_versions_stale_writers_tail_corruption_and_imported_metadata_are_distinct() {
        let root = std::env::temp_dir().join(format!(
            "forge-access-store-{}",
            workflows::new_id().unwrap()
        ));
        let mut a = Store::open(&root).unwrap();
        let seal = a.index.seal().unwrap();
        let mut b = Store::open(&root).unwrap();
        a.edit(
            Some(&seal),
            Edit::Import(Profiles {
                version: 1,
                entries: vec![profile()],
            }),
        )
        .unwrap();
        assert!(a.index.entries[0].joined_at.is_none());
        assert!(
            b.edit(Some(&seal), Edit::Pin(profile().id().unwrap(), true))
                .is_err()
        );
        a.edit(
            None,
            Edit::Visited(profile(), Some("C:/private-project".into()), 42),
        )
        .unwrap();
        assert_eq!(a.index.entries[0].joined_at, Some(42));
        a.edit(None, Edit::Visited(profile(), None, 21)).unwrap();
        assert_eq!(a.index.entries[0].joined_at, Some(42));
        let export = a.index.public().encode().unwrap();
        assert!(
            !String::from_utf8(export.clone())
                .unwrap()
                .contains("private-project")
        );
        assert_eq!(Profiles::parse(&export).unwrap().entries.len(), 1);
        fs::write(
            root.join("00000000000000000004.frconnections"),
            b"FRC1\ntorn",
        )
        .unwrap();
        let mut loaded = Store::open(&root).unwrap();
        assert!(!loaded.warning.is_empty());
        loaded
            .edit(None, Edit::Pin(profile().id().unwrap(), true))
            .unwrap();
        assert!(loaded.index.entries[0].pinned);
        fs::write(
            root.join("00000000000000000006.frconnections"),
            b"FRC1\n{}\nDONE\n",
        )
        .unwrap();
        assert!(Store::open(&root).is_err());
        for e in fs::read_dir(&root).unwrap() {
            fs::remove_file(e.unwrap().path()).unwrap();
        }
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn profile_schema_rejects_credentials_and_unknowns_without_echoing_them() {
        assert!(
            Profiles::parse(br#"{"version":1,"entries":[],"invite":"private-sentinel"}"#)
                .unwrap_err()
                .contains("schema")
        );
        let mut p = profile();
        p.address = "0.0.0.0:1".parse().unwrap();
        assert!(p.validate().is_err());
        assert!(Profiles::parse(&vec![b' '; MAX_PROFILE + 1]).is_err());
        let mut legacy = profile();
        legacy.room_id.clear();
        legacy.fingerprint.make_ascii_uppercase();
        let parsed = Profiles::parse(
            &Profiles {
                version: 1,
                entries: vec![legacy],
            }
            .encode()
            .unwrap(),
        )
        .unwrap();
        assert!(parsed.entries[0].room_id.is_empty());
        assert_eq!(parsed.entries[0].fingerprint, "b".repeat(64));
    }
}
