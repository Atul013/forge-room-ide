//! Private recovery index. External sources are references; only explicitly managed files can move.
use crate::local_records;
use forge_room_core::{
    transport::{
        RecoveryFileStamp, RecoveryKind, RecoveryPreview, RecoverySummary, recovery_file_stamp,
    },
    workflows,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
const RECORD: u64 = 1024 * 1024;
const VAULT: u64 = 512 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub path: String,
    pub label: String,
    pub project: Option<String>,
    pub summary: Option<RecoverySummary>,
    pub checked: u64,
    pub issue: Option<String>,
    pub pinned: bool,
    pub reserved: Option<RecoveryKind>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub at: u64,
    pub kind: String,
    pub detail: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub format: u32,
    pub revision: u64,
    pub entries: BTreeMap<String, Entry>,
    pub history: Vec<Event>,
}
impl Default for Index {
    fn default() -> Self {
        Self {
            format: 1,
            revision: 0,
            entries: BTreeMap::new(),
            history: vec![],
        }
    }
}
fn key(path: &Path) -> String {
    workflows::digest(
        path.to_string_lossy()
            .replace('/', "\\")
            .to_lowercase()
            .as_bytes(),
    )
}
fn hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
impl Index {
    pub fn seal(&self) -> Result<String, String> {
        workflows::seal(self)
    }
    fn validate(&self) -> Result<(), String> {
        if self.format != 1 || self.entries.len() > 128 || self.history.len() > 64 {
            return Err("Recovery index version/retention limit is unsupported".into());
        }
        for (id, entry) in &self.entries {
            if !hex(id)
                || entry.id != *id
                || entry.id != key(Path::new(&entry.path))
                || !Path::new(&entry.path).is_absolute()
            {
                return Err("Invalid recovery source identity".into());
            }
            workflows::text(&entry.path, 4096, true)?;
            workflows::text(&entry.label, 640, true)?;
            if let Some(project) = &entry.project {
                workflows::text(project, 4096, true)?;
                if !Path::new(project).is_absolute() {
                    return Err("Recorded project must be absolute".into());
                }
            }
            if let Some(issue) = &entry.issue {
                workflows::text(issue, 2000, false)?;
            }
            if let Some(summary) = &entry.summary
                && (!hex(&summary.digest)
                    || !hex(&summary.public_state_digest)
                    || summary.room.len() > 640
                    || summary.file_bytes > 64 * 1024 * 1024
                    || summary.complete_bytes > summary.file_bytes
                    || summary.file_bytes - summary.complete_bytes != summary.tail_bytes
                    || summary.records == 0)
            {
                return Err("Invalid cached recovery inspection".into());
            }
        }
        for event in &self.history {
            workflows::text(&event.kind, 80, true)?;
            workflows::text(&event.detail, 1000, false)?;
        }
        Ok(())
    }
}
pub struct Lease {
    _file: fs::File,
    pub path: PathBuf,
}
pub struct Reservation {
    pub path: PathBuf,
    pub kind: RecoveryKind,
    pub lease: Lease,
}
pub struct Store {
    pub root: PathBuf,
    pub index: Index,
}
impl Store {
    pub fn owned_directory(root: Option<&Path>, path: &Path) -> Result<bool, String> {
        let root = root.map(Path::to_owned).unwrap_or_else(Self::default_root);
        let Some(parent) = path.parent() else {
            return Err("Choose an absolute destination with a parent directory".into());
        };
        let parent = parent.canonicalize().map_err(|e| e.to_string())?;
        let Ok(root) = root.canonicalize() else {
            return Ok(false);
        };
        Ok(parent.starts_with(root))
    }
    fn default_root() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Forge Room Early Preview/room-recovery")
    }
    pub fn open() -> Result<Self, String> {
        Self::open_at(&Self::default_root())
    }
    pub fn open_at(root: &Path) -> Result<Self, String> {
        let mut store = Self::directory_at(root)?;
        store.index = store.latest()?;
        Ok(store)
    }
    fn directory_at(root: &Path) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        local_records::directory(root)?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        for name in ["states", "vault", "quarantine", "leases"] {
            let dir = root.join(name);
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            local_records::directory(&dir)?;
        }
        let store = Self {
            root,
            index: Index::default(),
        };
        Ok(store)
    }
    pub fn lease_for(root: Option<&Path>, path: &Path) -> Result<Option<Lease>, String> {
        let root = root.map(Path::to_owned).unwrap_or_else(Self::default_root);
        let Some(parent) = path.parent() else {
            return Ok(None);
        };
        let parent = parent.canonicalize().map_err(|e| e.to_string())?;
        let Ok(root) = root.canonicalize() else {
            return Ok(None);
        };
        if parent != root.join("vault") && parent != root.join("quarantine") {
            return Ok(None);
        }
        Self::directory_at(&root)?.lease(path)
    }
    fn directories(&self) -> Result<(), String> {
        local_records::directory(&self.root)?;
        for name in ["states", "vault", "quarantine", "leases"] {
            local_records::directory(&self.root.join(name))?;
        }
        Ok(())
    }
    fn lock(&self) -> Result<fs::File, String> {
        self.directories()?;
        let path = self.root.join(".writer");
        if path.exists() {
            local_records::file(&path)?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.try_lock()
            .map_err(|e| format!("Recovery index writer is busy: {e}"))?;
        Ok(file)
    }
    fn states(&self) -> Result<Vec<(u64, PathBuf)>, String> {
        self.directories()?;
        let mut paths = vec![];
        for entry in fs::read_dir(self.root.join("states")).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            local_records::file(&path)?;
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or("Invalid recovery state name")?;
            if path.extension().is_none_or(|ext| ext != "state")
                || name.len() != 16
                || !name.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err("Unexpected recovery index record; original bytes preserved".into());
            }
            paths.push((name.parse().map_err(|_| "Invalid recovery sequence")?, path));
            if paths.len() > 128 {
                return Err("Too many recovery index snapshots".into());
            }
        }
        paths.sort_by_key(|(number, _)| *number);
        Ok(paths)
    }
    fn latest(&self) -> Result<Index, String> {
        let paths = self.states()?;
        let empty = paths.is_empty();
        for (_, path) in paths.into_iter().rev() {
            match local_records::read::<Index>(&path, RECORD) {
                Ok(index) => {
                    index.validate()?;
                    return Ok(index);
                }
                Err(error) if error == local_records::INCOMPLETE => (),
                Err(error) => return Err(error),
            }
        }
        if empty {
            Ok(Index::default())
        } else {
            Err("No complete recovery index; files preserved. Selected-file inspection still works.".into())
        }
    }
    pub fn reload(&mut self) -> Result<(), String> {
        self.index = self.latest()?;
        Ok(())
    }
    fn current(&self) -> Result<(), String> {
        if self.latest()?.seal()? != self.index.seal()? {
            return Err(
                "Recovery catalog changed in another window; reload and review again".into(),
            );
        }
        Ok(())
    }
    fn commit(&mut self, mut next: Index, kind: &str, detail: String) -> Result<(), String> {
        let mut states = self.states()?;
        let sequence = states.last().map_or(1, |(n, _)| n + 1);
        if sequence >= 10_000_000_000_000_000 {
            return Err("Recovery record sequence exhausted".into());
        }
        next.revision = self
            .index
            .revision
            .checked_add(1)
            .ok_or("Recovery index revision exhausted")?;
        next.history.push(Event {
            at: forge_room_core::now_epoch_seconds(),
            kind: kind.into(),
            detail,
        });
        if next.history.len() > 64 {
            next.history.remove(0);
        }
        next.validate()?;
        let path = self
            .root
            .join("states")
            .join(format!("{sequence:016}.state"));
        local_records::write_new(&path, RECORD, &next)?;
        self.index = next;
        states.push((sequence, path));
        for (_, path) in states.iter().take(states.len().saturating_sub(16)) {
            local_records::file(path)?;
            let _ = fs::remove_file(path);
        }
        Ok(())
    }
    fn normalized(&self, path: &Path) -> Result<PathBuf, String> {
        let parent = path
            .parent()
            .ok_or("Missing recovery parent")?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        local_records::directory(&parent)?;
        Ok(parent.join(path.file_name().ok_or("Missing recovery filename")?))
    }
    pub fn managed(&self, path: &Path) -> Result<bool, String> {
        let path = self.normalized(path)?;
        Ok(self.managed_candidate(&path))
    }
    /// UI classification is lexical; worker mutations revalidate the real path and held lease.
    pub fn managed_candidate(&self, path: &Path) -> bool {
        let Some(parent) = path.parent() else {
            return false;
        };
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        (parent == self.root.join("vault") || parent == self.root.join("quarantine"))
            && name.len() == 32
            && name.bytes().all(|byte| byte.is_ascii_hexdigit())
            && path
                .extension()
                .is_some_and(|ext| ext == "json" || ext == "frjournal")
    }
    pub fn lease(&self, path: &Path) -> Result<Option<Lease>, String> {
        self.directories()?;
        let path = self.normalized(path)?;
        if !self.managed(&path)? {
            return Ok(None);
        }
        let lease = self
            .root
            .join("leases")
            .join(format!("{}.lease", key(&path)));
        if lease.exists() {
            local_records::file(&lease)?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lease)
            .map_err(|e| e.to_string())?;
        file.try_lock().map_err(
            |_| "This managed recovery source is in use by another operation or hosted Room",
        )?;
        Ok(Some(Lease { _file: file, path }))
    }
    pub fn usage(&self) -> Result<(u64, u64, usize), String> {
        self.directories()?;
        let mut bytes = 0u64;
        let mut reserved = 0u64;
        let mut files = 0usize;
        for dir in ["vault", "quarantine"] {
            for entry in fs::read_dir(self.root.join(dir)).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                local_records::file(&path)?;
                if !self.managed(&path)? {
                    return Err("Unexpected managed recovery file; preserved for inspection".into());
                }
                let length = fs::metadata(&path).map_err(|e| e.to_string())?.len();
                let charge = if path.extension().is_some_and(|ext| ext == "frjournal") {
                    64 * 1024 * 1024
                } else {
                    8 * 1024 * 1024
                };
                if length > charge {
                    return Err(
                        "Managed source exceeds its type capacity; original file preserved".into(),
                    );
                }
                bytes += length;
                reserved += charge;
                files += 1;
                if files > 128 {
                    return Err("Managed vault exceeds 128 files".into());
                }
            }
        }
        for entry in self.index.entries.values() {
            if let Some(kind) = entry.reserved
                && !Path::new(&entry.path).exists()
            {
                reserved += match kind {
                    RecoveryKind::Journal => 64 * 1024 * 1024,
                    RecoveryKind::Checkpoint => 8 * 1024 * 1024,
                };
            }
        }
        Ok((bytes, reserved, files))
    }
    pub fn reserve(
        &mut self,
        kind: RecoveryKind,
        label: &str,
        project: Option<String>,
    ) -> Result<Reservation, String> {
        workflows::text(label, 640, true)?;
        let _lock = self.lock()?;
        self.current()?;
        let charge = match kind {
            RecoveryKind::Journal => 64 * 1024 * 1024,
            RecoveryKind::Checkpoint => 8 * 1024 * 1024,
        };
        if self.usage()?.1 + charge > VAULT {
            return Err("Managed recovery vault reached its 512 MiB reserved capacity; review inactive sources".into());
        }
        let ext = match kind {
            RecoveryKind::Journal => "frjournal",
            RecoveryKind::Checkpoint => "json",
        };
        let path = self
            .root
            .join("vault")
            .join(format!("{}.{ext}", workflows::new_id()?));
        if path.exists() {
            return Err("Generated recovery path already exists".into());
        }
        let lease = self
            .lease(&path)?
            .ok_or("Managed recovery lease unavailable")?;
        let id = key(&path);
        let mut next = self.index.clone();
        next.entries.insert(
            id.clone(),
            Entry {
                id,
                path: path.to_str().ok_or("Recovery path must be UTF-8")?.into(),
                label: label.into(),
                project,
                summary: None,
                checked: forge_room_core::now_epoch_seconds(),
                issue: Some("Reserved; creation not yet inspected. No automatic resume.".into()),
                pinned: false,
                reserved: Some(kind),
            },
        );
        self.commit(
            next,
            "reserved",
            format!("Reserved new managed {}", kind.label()),
        )?;
        Ok(Reservation { path, kind, lease })
    }
    pub fn register(
        &mut self,
        preview: &RecoveryPreview,
        project: Option<String>,
    ) -> Result<(), String> {
        let _lock = self.lock()?;
        self.current()?;
        let id = key(preview.source());
        let mut next = self.index.clone();
        let old = next.entries.get(&id);
        let pinned = old.is_some_and(|entry| entry.pinned);
        let project = project.or_else(|| old.and_then(|entry| entry.project.clone()));
        next.entries.insert(
            id.clone(),
            Entry {
                id,
                path: preview
                    .source()
                    .to_str()
                    .ok_or("Recovery source must be UTF-8")?
                    .into(),
                label: preview.summary().room.clone(),
                project,
                summary: Some(preview.summary().clone()),
                checked: forge_room_core::now_epoch_seconds(),
                issue: None,
                pinned,
                reserved: None,
            },
        );
        self.commit(
            next,
            "inspected",
            "Recorded a public recovery inspection; source bytes preserved".into(),
        )
    }
    pub fn issue(&mut self, id: &str, error: String) -> Result<(), String> {
        let _lock = self.lock()?;
        self.current()?;
        let mut next = self.index.clone();
        let entry = next
            .entries
            .get_mut(id)
            .ok_or("Recovery entry no longer exists")?;
        entry.issue = Some(bounded_issue(error));
        entry.checked = forge_room_core::now_epoch_seconds();
        self.commit(
            next,
            "inspection_issue",
            "Source unavailable or invalid; original files preserved".into(),
        )
    }
    pub fn pin(&mut self, id: &str, pinned: bool, expected: &str) -> Result<(), String> {
        let _lock = self.lock()?;
        self.current()?;
        if self.index.seal()? != expected {
            return Err("Pin review is stale".into());
        }
        let mut next = self.index.clone();
        next.entries
            .get_mut(id)
            .ok_or("Recovery entry missing")?
            .pinned = pinned;
        self.commit(next, "pin", "Changed a local recovery bookmark".into())
    }
    pub fn forget(&mut self, id: &str, expected: &str) -> Result<(), String> {
        let _lock = self.lock()?;
        self.current()?;
        if self.index.seal()? != expected {
            return Err("Forget review is stale".into());
        }
        let mut next = self.index.clone();
        if next.entries.remove(id).is_none() {
            return Err("Recovery entry missing".into());
        }
        self.commit(
            next,
            "forgotten",
            "Forgot metadata only; original files were not removed".into(),
        )
    }
    pub fn quarantine(
        &mut self,
        id: &str,
        expected: &str,
        digest: &str,
    ) -> Result<PathBuf, String> {
        let _lock = self.lock()?;
        self.current()?;
        if self.index.seal()? != expected {
            return Err("Quarantine review is stale".into());
        }
        let entry = self.index.entries.get(id).ok_or("Recovery entry missing")?;
        let path = PathBuf::from(&entry.path);
        if !self.managed(&path)? || path.parent() != Some(self.root.join("vault").as_path()) {
            return Err("Only an owned managed vault file can be quarantined; external sources remain untouched".into());
        }
        let _lease = self.lease(&path)?.ok_or("Managed lease unavailable")?;
        let stamp = recovery_file_stamp(&path)?;
        if stamp.digest() != digest {
            return Err("Source changed since quarantine inspection".into());
        }
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or("Missing recovery extension")?;
        let target = self
            .root
            .join("quarantine")
            .join(format!("{}.{ext}", workflows::new_id()?));
        if target.exists() {
            return Err("Quarantine target exists".into());
        }
        // Commit the new reference first. A crash leaves both the original bytes and an inspectable missing reference.
        let mut next = self.index.clone();
        let mut moved = next.entries.remove(id).ok_or("Recovery entry missing")?;
        moved.id = key(&target);
        moved.path = target
            .to_str()
            .ok_or("Quarantine path must be UTF-8")?
            .into();
        next.entries.insert(moved.id.clone(), moved);
        self.commit(
            next,
            "quarantined",
            "Recorded reviewed quarantine target; file movement requires completion/inspection"
                .into(),
        )?;
        fs::rename(&path,&target).map_err(|e|format!("Index retained the quarantine target, but the original file is still preserved at {}: {e}",path.display()))?;
        self.commit(
            self.index.clone(),
            "quarantine_completed",
            "Owned source moved into private quarantine".into(),
        )
        .map_err(|e| {
            format!(
                "File moved to {}, but completion audit was not saved: {e}",
                target.display()
            )
        })?;
        Ok(target)
    }
    pub fn discover(&self) -> Result<Vec<PathBuf>, String> {
        let mut paths = vec![];
        let mut checked = 0usize;
        for directory in ["vault", "quarantine"] {
            for entry in fs::read_dir(self.root.join(directory)).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                local_records::file(&path)?;
                if !self.managed(&path)? {
                    return Err("Unexpected managed source; files preserved".into());
                }
                checked += 1;
                if checked > 128 {
                    return Err("Managed file discovery reached 128 files".into());
                }
                if !self.index.entries.contains_key(&key(&path)) {
                    paths.push(path);
                }
                if paths.len() > 128 {
                    return Err("Too many unindexed managed sources".into());
                }
            }
        }
        paths.sort();
        Ok(paths)
    }
    pub fn id(path: &Path) -> String {
        key(path)
    }
    pub fn track_problem(
        &mut self,
        stamp: &RecoveryFileStamp,
        error: String,
        project: Option<String>,
    ) -> Result<(), String> {
        let _lock = self.lock()?;
        self.current()?;
        let id = key(stamp.source());
        let mut next = self.index.clone();
        let old = next.entries.get(&id);
        let label = old
            .map(|entry| entry.label.clone())
            .unwrap_or_else(|| "Unverified recovery source".into());
        let pinned = old.is_some_and(|entry| entry.pinned);
        let summary = old.and_then(|entry| entry.summary.clone());
        let project = project.or_else(|| old.and_then(|entry| entry.project.clone()));
        next.entries.insert(
            id.clone(),
            Entry {
                id,
                path: stamp
                    .source()
                    .to_str()
                    .ok_or("Recovery source must be UTF-8")?
                    .into(),
                label,
                project,
                summary,
                checked: forge_room_core::now_epoch_seconds(),
                issue: Some(bounded_issue(error)),
                pinned,
                reserved: None,
            },
        );
        self.commit(
            next,
            "inspection_issue",
            "Recorded a failed source inspection; bytes preserved".into(),
        )
    }
    pub fn purge_quarantine(
        &mut self,
        id: &str,
        expected: &str,
        digest: &str,
    ) -> Result<(), String> {
        let _lock = self.lock()?;
        self.current()?;
        if self.index.seal()? != expected {
            return Err("Removal review is stale".into());
        }
        let path = PathBuf::from(
            &self
                .index
                .entries
                .get(id)
                .ok_or("Recovery entry missing")?
                .path,
        );
        if !self.managed(&path)? || path.parent() != Some(self.root.join("quarantine").as_path()) {
            return Err("Removal is limited to owned private quarantine files".into());
        }
        let _lease = self.lease(&path)?.ok_or("Managed lease unavailable")?;
        if recovery_file_stamp(&path)?.digest() != digest {
            return Err("Quarantined source changed since review".into());
        }
        let mut next = self.index.clone();
        next.entries.remove(id);
        self.commit(
            next,
            "removed",
            "Approved removal of exact owned quarantine bytes; completion must be recorded".into(),
        )?;
        fs::remove_file(&path).map_err(|e| {
            format!(
                "Metadata removed but bytes remain preserved at {}: {e}",
                path.display()
            )
        })?;
        self.commit(
            self.index.clone(),
            "removal_completed",
            "Reviewed inactive quarantine file removed".into(),
        )
        .map_err(|e| format!("File was removed, but completion audit was not saved: {e}"))
    }
}
fn bounded_issue(mut value: String) -> String {
    let mut end = value.len().min(1800);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_room_core::transport::{Host, inspect_recovery};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "forge-recovery-index-{}",
                workflows::new_id().unwrap()
            ));
            fs::create_dir(&root).unwrap();
            Self(root.canonicalize().unwrap())
        }
        fn store(&self) -> Store {
            Store::open_at(&self.0.join("catalog")).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            assert!(
                self.0
                    .starts_with(std::env::temp_dir().canonicalize().unwrap())
            );
            assert!(
                self.0
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("forge-recovery-index-")
            );
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn external_references_survive_forget_and_managed_lease_blocks_move_until_writer_is_done() {
        let fixture = Fixture::new();
        let mut store = fixture.store();
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "catalog trial").unwrap();
        let external = fixture.0.join("external.json");
        host.save_checkpoint(&external).unwrap();
        let preview = inspect_recovery(&external).unwrap();
        store.register(&preview, None).unwrap();
        let id = Store::id(preview.source());
        let seal = store.index.seal().unwrap();
        store.pin(&id, true, &seal).unwrap();
        assert!(store.forget(&id, &seal).is_err());
        assert!(
            store
                .quarantine(&id, &store.index.seal().unwrap(), &preview.summary().digest)
                .is_err()
        );
        store.forget(&id, &store.index.seal().unwrap()).unwrap();
        assert!(external.exists());
        let reservation = store
            .reserve(RecoveryKind::Checkpoint, "managed trial", None)
            .unwrap();
        assert!(store.managed(&reservation.path).unwrap());
        host.save_checkpoint(&reservation.path).unwrap();
        let preview = inspect_recovery(&reservation.path).unwrap();
        store.register(&preview, None).unwrap();
        let id = Store::id(&reservation.path);
        let seal = store.index.seal().unwrap();
        assert!(
            store
                .quarantine(&id, &seal, &preview.summary().digest)
                .is_err()
        );
        let other = fixture.store();
        assert!(other.lease(&reservation.path).is_err());
        let original = fs::read(&reservation.path).unwrap();
        drop(reservation);
        let target = store
            .quarantine(&id, &seal, &preview.summary().digest)
            .unwrap();
        assert_eq!(fs::read(&target).unwrap(), original);
        let id = Store::id(&target);
        let seal = store.index.seal().unwrap();
        assert!(store.purge_quarantine(&id, &seal, &"0".repeat(64)).is_err());
        store
            .purge_quarantine(&id, &seal, &preview.summary().digest)
            .unwrap();
        assert!(!target.exists());
        assert!(external.exists());
    }
    #[test]
    fn concurrent_catalogs_stale_writes_and_torn_or_complete_corruption_fail_without_losing_files()
    {
        let fixture = Fixture::new();
        let mut a = fixture.store();
        let mut b = fixture.store();
        let lock = a.lock().unwrap();
        assert!(
            b.reserve(RecoveryKind::Checkpoint, "blocked", None)
                .is_err()
        );
        drop(lock);
        let reservation = a.reserve(RecoveryKind::Checkpoint, "first", None).unwrap();
        assert!(b.reserve(RecoveryKind::Checkpoint, "stale", None).is_err());
        b.reload().unwrap();
        assert_eq!(b.index.revision, a.index.revision);
        drop(reservation);
        let number = a.states().unwrap().last().unwrap().0 + 1;
        let path = a.root.join("states").join(format!("{number:016}.state"));
        fs::write(&path, b"FRC1\ninterrupted").unwrap();
        assert_eq!(fixture.store().index.revision, a.index.revision);
        fs::write(&path, b"FRC1\n{}\nDONE\n").unwrap();
        assert!(Store::open_at(&a.root).is_err());
        assert!(path.exists());
    }
    #[test]
    fn pending_managed_journals_reserve_worst_case_capacity_even_before_file_creation() {
        let fixture = Fixture::new();
        let mut store = fixture.store();
        let mut first = None;
        for index in 0..8 {
            let reserved = store
                .reserve(RecoveryKind::Journal, &format!("journal {index}"), None)
                .unwrap();
            if first.is_none() {
                first = Some(Store::id(&reserved.path));
            }
            drop(reserved);
        }
        assert_eq!(store.usage().unwrap().1, VAULT);
        assert!(
            store
                .reserve(RecoveryKind::Journal, "too much", None)
                .is_err()
        );
        let seal = store.index.seal().unwrap();
        store.forget(&first.unwrap(), &seal).unwrap();
        let reserved = store
            .reserve(RecoveryKind::Checkpoint, "fits", None)
            .unwrap();
        assert_eq!(reserved.kind, RecoveryKind::Checkpoint);
        assert_eq!(
            store.usage().unwrap().1,
            7 * 64 * 1024 * 1024 + 8 * 1024 * 1024
        );
    }
    #[test]
    fn damaged_owned_sources_can_be_preserved_in_quarantine_and_unindexed_files_are_discoverable() {
        let fixture = Fixture::new();
        let mut store = fixture.store();
        let reservation = store
            .reserve(RecoveryKind::Checkpoint, "unfinished", None)
            .unwrap();
        let path = reservation.path.clone();
        fs::write(&path, b"corrupt original bytes").unwrap();
        drop(reservation);
        let stamp = recovery_file_stamp(&path).unwrap();
        assert_eq!(stamp.bytes(), 22);
        store
            .track_problem(&stamp, "Unsupported recovery schema".into(), None)
            .unwrap();
        let seal = store.index.seal().unwrap();
        let target = store
            .quarantine(&Store::id(&path), &seal, stamp.digest())
            .unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"corrupt original bytes");
        let seal = store.index.seal().unwrap();
        store.forget(&Store::id(&target), &seal).unwrap();
        assert!(target.exists());
        assert_eq!(store.discover().unwrap(), vec![target]);
    }
}
