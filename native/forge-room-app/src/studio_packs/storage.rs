use crate::local_records;
use forge_room_core::{
    packs::{self, Activation, Catalog, Pack},
    workflows,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
const LIMIT: u64 = 512 * 1024 * 1024;
const LEDGER_LIMIT: u64 = 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Installed {
    pub id: String,
    pub version: packs::Version,
    pub digest: String,
    pub installed_at: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub at: u64,
    pub kind: String,
    pub detail: String,
    pub previous: Activation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    pub format: u32,
    pub revision: u64,
    pub versions: BTreeMap<String, Installed>,
    pub activation: Activation,
    pub history: Vec<Event>,
}
impl Default for Ledger {
    fn default() -> Self {
        Self {
            format: 1,
            revision: 0,
            versions: BTreeMap::new(),
            activation: Activation::default(),
            history: vec![],
        }
    }
}
impl Ledger {
    fn validate(&self) -> Result<(), String> {
        if self.format != 1
            || self.versions.len() > 64
            || self.history.len() > 64
            || self.activation.enabled.len() > 32
            || serde_json::to_vec(self).map_err(|e| e.to_string())?.len() as u64 > LEDGER_LIMIT
        {
            return Err("Pack ledger exceeds supported schema/limits".into());
        }
        for (key, version) in &self.versions {
            if !packs::pack_id(&version.id)
                || key != &format!("{}@{}", version.id, version.version)
                || !digest(&version.digest)
            {
                return Err("Invalid installed pack identity".into());
            }
        }
        for event in &self.history {
            workflows::text(&event.kind, 80, true)?;
            workflows::text(&event.detail, 1000, false)?;
            if event.previous.format != 1 || event.previous.enabled.len() > 32 {
                return Err("Invalid prior activation snapshot".into());
            }
        }
        Ok(())
    }
    pub fn seal(&self) -> Result<String, String> {
        workflows::seal(self)
    }
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
pub struct Store {
    pub root: PathBuf,
    pub ledger: Ledger,
}
#[derive(Clone)]
pub struct ActivationReview {
    pub expected_ledger: String,
    pub review: packs::Review,
}
impl Store {
    pub fn open() -> Result<Self, String> {
        let root = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Forge Room Early Preview/packs");
        Self::open_at(&root)
    }
    pub fn open_at(root: &Path) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        local_records::directory(root)?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        for directory in ["data", "states", "quarantine"] {
            let path = root.join(directory);
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            local_records::directory(&path)?;
        }
        let mut store = Self {
            root,
            ledger: Ledger::default(),
        };
        store.ledger = store.latest()?;
        Ok(store)
    }
    fn validate_paths(&self) -> Result<(), String> {
        local_records::directory(&self.root)?;
        for name in ["data", "states", "quarantine"] {
            local_records::directory(&self.root.join(name))?;
        }
        Ok(())
    }
    fn lock(&self) -> Result<fs::File, String> {
        self.validate_paths()?;
        let path = self.root.join(".writer-lock");
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
            .map_err(|e| format!("Pack transaction writer is unavailable: {e}"))?;
        Ok(file)
    }
    fn states(&self) -> Result<Vec<(u64, PathBuf)>, String> {
        self.validate_paths()?;
        let mut states = Vec::new();
        for entry in fs::read_dir(self.root.join("states")).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = name
                .strip_suffix(".state")
                .ok_or("Unexpected file in pack state storage")?;
            if number.len() != 16 || !number.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("Invalid pack state sequence".into());
            }
            local_records::file(&entry.path())?;
            states.push((
                number.parse().map_err(|_| "Invalid state sequence")?,
                entry.path(),
            ));
            if states.len() > 128 {
                return Err("Too many pack state snapshots".into());
            }
        }
        states.sort_by_key(|(number, _)| *number);
        Ok(states)
    }
    fn latest(&self) -> Result<Ledger, String> {
        let states = self.states()?;
        let empty = states.is_empty();
        for (_, path) in states.into_iter().rev() {
            match local_records::read::<Ledger>(&path, LEDGER_LIMIT) {
                Ok(ledger) => {
                    ledger.validate()?;
                    return Ok(ledger);
                }
                Err(error) if error == local_records::INCOMPLETE => continue,
                Err(error) => return Err(error),
            }
        }
        if empty {
            Ok(Ledger::default())
        } else {
            Err("No complete pack ledger exists; storage preserved for repair".into())
        }
    }
    pub fn reload(&mut self) -> Result<(), String> {
        self.ledger = self.latest()?;
        Ok(())
    }
    fn current(&self) -> Result<(), String> {
        if self.latest()? != self.ledger {
            return Err(
                "Pack registry changed in another window; reload before applying a review".into(),
            );
        }
        Ok(())
    }
    pub fn usage(&self) -> Result<u64, String> {
        self.validate_paths()?;
        let mut bytes = 0u64;
        let mut count = 0usize;
        for name in ["data", "states", "quarantine"] {
            for entry in fs::read_dir(self.root.join(name)).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                local_records::file(&entry.path())?;
                bytes = bytes.saturating_add(entry.metadata().map_err(|e| e.to_string())?.len());
                count += 1;
                if count > 1024 {
                    return Err(
                        "Private pack storage has too many files; inspect and prune inactive data"
                            .into(),
                    );
                }
            }
        }
        Ok(bytes)
    }
    fn capacity(&self, extra: u64) -> Result<(), String> {
        if self.usage()?.saturating_add(extra) > LIMIT {
            return Err(
                "Private pack storage reached 512 MiB; prune inactive versions/quarantined files"
                    .into(),
            );
        }
        Ok(())
    }
    fn path(&self, digest_value: &str) -> Result<PathBuf, String> {
        if !digest(digest_value) {
            return Err("Invalid private pack digest".into());
        }
        self.validate_paths()?;
        Ok(self.root.join("data").join(format!("{digest_value}.pack")))
    }
    pub fn pack(&self, key: &str) -> Result<Pack, String> {
        let installed = self
            .ledger
            .versions
            .get(key)
            .ok_or("Pack version is not installed")?;
        let pack: Pack = local_records::read(
            &self.path(&installed.digest)?,
            packs::MAX_PACK as u64 + 2048,
        )?;
        pack.validate()?;
        if pack.key() != key || pack.digest() != Ok(installed.digest.clone()) {
            return Err("Installed pack content/identity changed".into());
        }
        Ok(pack)
    }
    pub fn issues(&self) -> BTreeMap<String, String> {
        self.ledger
            .versions
            .keys()
            .filter_map(|key| self.pack(key).err().map(|error| (key.clone(), error)))
            .collect()
    }
    pub fn catalog(&self) -> Result<Catalog, String> {
        let catalog = self
            .ledger
            .versions
            .keys()
            .filter_map(|key| self.pack(key).ok().map(|pack| (key.clone(), pack)))
            .collect::<Catalog>();
        self.ledger.activation.validate(&catalog)?;
        Ok(catalog)
    }
    fn commit(&mut self, mut next: Ledger, kind: &str, detail: String) -> Result<(), String> {
        self.current()?;
        next.revision = self
            .ledger
            .revision
            .checked_add(1)
            .ok_or("Pack ledger revision exhausted")?;
        next.history.push(Event {
            at: forge_room_core::now_epoch_seconds(),
            kind: kind.into(),
            detail,
            previous: self.ledger.activation.clone(),
        });
        if next.history.len() > 64 {
            next.history.remove(0);
        }
        next.validate()?;
        let mut states = self.states()?;
        let next_file = states
            .last()
            .map_or(1, |(number, _)| number.saturating_add(1));
        let path = self
            .root
            .join("states")
            .join(format!("{next_file:016}.state"));
        self.capacity(serde_json::to_vec(&next).map_err(|e| e.to_string())?.len() as u64 + 1024)?;
        local_records::write_new(&path, LEDGER_LIMIT, &next)?;
        self.ledger = next;
        states.push((next_file, path));
        let excess = states.len().saturating_sub(16);
        for (_, path) in states.into_iter().take(excess) {
            local_records::file(&path)?;
            let _ = fs::remove_file(path);
        }
        Ok(())
    }
    pub fn install(&mut self, pack: &Pack, expected: &str) -> Result<(), String> {
        pack.validate()?;
        if pack.digest()? != expected {
            return Err("Pack differs from the inspected import".into());
        }
        let _lock = self.lock()?;
        self.current()?;
        let key = pack.key();
        if let Some(existing) = self.ledger.versions.get(&key) {
            if existing.digest == expected {
                self.pack(&key).map_err(|_|"Installed bytes are unavailable; use explicit Repair from this exact reviewed source")?;
                return Ok(());
            }
            return Err(
                "This version is immutable; use a new version for different content".into(),
            );
        }
        if self.ledger.versions.len() >= 64 {
            return Err("64 installed versions retained; remove inactive versions first".into());
        }
        let path = self.path(expected)?;
        if path.exists() {
            match local_records::read::<Pack>(&path, packs::MAX_PACK as u64 + 2048) {
                Ok(old) if old == *pack => (),
                Err(error) if error == local_records::INCOMPLETE => {
                    let quarantine = self.root.join("quarantine").join(format!(
                        "{}-{}.incomplete",
                        expected,
                        workflows::new_id()?
                    ));
                    local_records::file(&path)?;
                    fs::rename(&path, quarantine).map_err(|e| e.to_string())?;
                    self.capacity(packs::MAX_PACK as u64 + 2048)?;
                    local_records::write_new(&path, packs::MAX_PACK as u64 + 2048, pack)?;
                }
                _ => {
                    return Err(
                        "Existing immutable pack file changed; preserved for inspection".into(),
                    );
                }
            }
        } else {
            self.capacity(
                serde_json::to_vec(pack).map_err(|e| e.to_string())?.len() as u64 + 2048,
            )?;
            local_records::write_new(&path, packs::MAX_PACK as u64 + 2048, pack)?;
        }
        let mut next = self.ledger.clone();
        next.versions.insert(
            key.clone(),
            Installed {
                id: pack.id.clone(),
                version: pack.version,
                digest: expected.into(),
                installed_at: forge_room_core::now_epoch_seconds(),
            },
        );
        self.commit(
            next,
            "installed",
            format!("Installed {key}; resources remain disabled"),
        )
    }
    pub fn review(
        &self,
        selection: BTreeMap<String, packs::Selection>,
    ) -> Result<ActivationReview, String> {
        self.current()?;
        let catalog = self.catalog()?;
        Ok(ActivationReview {
            expected_ledger: self.ledger.seal()?,
            review: self.ledger.activation.review(selection, &catalog)?,
        })
    }
    pub fn activate(&mut self, review: &ActivationReview) -> Result<(), String> {
        let _lock = self.lock()?;
        self.current()?;
        if self.ledger.seal()? != review.expected_ledger {
            return Err("Pack activation is stale; inspect a fresh review".into());
        }
        let catalog = self.catalog()?;
        let activation = self.ledger.activation.accept(&review.review, &catalog)?;
        let mut next = self.ledger.clone();
        next.activation = activation;
        self.commit(
            next,
            "activation",
            "Applied exactly reviewed resource selection".into(),
        )
    }
    pub fn rollback_review(&self, event: usize) -> Result<ActivationReview, String> {
        let previous = self
            .ledger
            .history
            .get(event)
            .ok_or("No retained activation snapshot")?
            .previous
            .clone();
        self.review(previous.enabled)
    }
    pub fn repair_disable(&mut self, expected: &str) -> Result<(), String> {
        let _lock = self.lock()?;
        self.current()?;
        if self.ledger.seal()? != expected {
            return Err("Repair review is stale".into());
        }
        let mut next = self.ledger.clone();
        next.activation = Activation {
            format: 1,
            revision: next
                .activation
                .revision
                .checked_add(1)
                .ok_or("Activation revision exhausted")?,
            enabled: BTreeMap::new(),
        };
        self.commit(
            next,
            "repair_disabled",
            "Explicitly disabled all contributions; version bytes retained for repair".into(),
        )
    }
    pub fn repair_data(&mut self, pack: &Pack, expected: &str) -> Result<(), String> {
        pack.validate()?;
        if pack.digest()? != expected {
            return Err("Repair differs from inspected source".into());
        }
        let _lock = self.lock()?;
        self.current()?;
        let key = pack.key();
        let installed = self
            .ledger
            .versions
            .get(&key)
            .ok_or("Repair requires an existing version record")?;
        if installed.digest != expected {
            return Err("Repair source must match the original complete manifest digest".into());
        }
        if self.pack(&key).is_ok() {
            return Ok(());
        }
        let path = self.path(expected)?;
        if path.exists() {
            local_records::file(&path)?;
            let quarantine = self.root.join("quarantine").join(format!(
                "{}-{}.repair",
                expected,
                workflows::new_id()?
            ));
            fs::rename(&path, quarantine).map_err(|e| e.to_string())?;
        }
        self.capacity(packs::MAX_PACK as u64 + 2048)?;
        local_records::write_new(&path, packs::MAX_PACK as u64 + 2048, pack)?;
        self.commit(
            self.ledger.clone(),
            "repaired",
            format!("Restored original reviewed bytes for {key}; prior bytes quarantined"),
        )
    }
    pub fn quarantine(&mut self, key: &str) -> Result<Option<String>, String> {
        let _lock = self.lock()?;
        self.current()?;
        let installed = self
            .ledger
            .versions
            .get(key)
            .cloned()
            .ok_or("No installed version record")?;
        if self
            .ledger
            .activation
            .enabled
            .get(&installed.id)
            .is_some_and(|selected| selected.version == installed.version)
        {
            return Err("Explicitly disable active resources before quarantining a version".into());
        }
        let path = self.path(&installed.digest)?;
        if path.exists() {
            local_records::file(&path)?;
        }
        let mut next = self.ledger.clone();
        next.versions.remove(key);
        self.commit(
            next,
            "quarantined",
            format!("Removed inactive {key} reference; bytes retained in quarantine"),
        )?;
        if !path.exists() {
            return Ok(None);
        }
        let target = self.root.join("quarantine").join(format!(
            "{}-{}.removed",
            installed.digest,
            workflows::new_id()?
        ));
        Ok(fs::rename(path, target)
            .err()
            .map(|error| error.to_string()))
    }
    pub fn purge(&mut self, key: &str) -> Result<Option<String>, String> {
        let _lock = self.lock()?;
        self.current()?;
        let installed = self
            .ledger
            .versions
            .get(key)
            .cloned()
            .ok_or("No such installed version")?;
        if self
            .ledger
            .activation
            .enabled
            .get(&installed.id)
            .is_some_and(|selected| selected.version == installed.version)
        {
            return Err("Disable or switch the active version before removing it".into());
        }
        self.pack(key)?;
        let path = self.path(&installed.digest)?;
        let mut next = self.ledger.clone();
        next.versions.remove(key);
        self.commit(
            next,
            "removed",
            format!("Removed inactive {key}; historical rollback may require reimport"),
        )?;
        let cleanup = (|| {
            let old: Pack = local_records::read(&path, packs::MAX_PACK as u64 + 2048)?;
            if old.digest()? != installed.digest {
                return Err("Inactive pack changed before cleanup; preserved".into());
            }
            local_records::file(&path)?;
            fs::remove_file(path).map_err(|e| e.to_string())
        })();
        Ok(cleanup.err())
    }
    pub fn export(&self, key: &str, path: &Path) -> Result<(), String> {
        let pack = self.pack(key)?;
        let bytes = serde_json::to_vec_pretty(&pack).map_err(|e| e.to_string())?;
        if bytes.len() > packs::MAX_PACK {
            return Err("Export exceeds the pack limit".into());
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "forge-packs-store-{}",
                workflows::new_id().unwrap()
            ));
            fs::create_dir(&root).unwrap();
            Self(root.canonicalize().unwrap())
        }
        fn open(&self) -> Store {
            Store::open_at(&self.0).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let temp = std::env::temp_dir().canonicalize().unwrap();
            assert!(
                self.0.starts_with(&temp)
                    && self
                        .0
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("forge-packs-store-")
            );
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn pack(version: &str) -> Pack {
        Pack {
            format: 1,
            api: 1,
            id: "local.trial".into(),
            version: version.parse().unwrap(),
            name: "Trial".into(),
            publisher: "Local claim".into(),
            summary: String::new(),
            license: "MIT".into(),
            license_text: String::new(),
            source: String::new(),
            readme: String::new(),
            dependencies: vec![],
            resources: vec![
                packs::Resource::new(
                    "guide".into(),
                    "Guide".into(),
                    packs::Kind::Readme,
                    "Local documentation".into(),
                )
                .unwrap(),
            ],
        }
    }
    #[test]
    fn immutable_install_review_activation_restart_and_rollback_do_not_overwrite_versions() {
        let fixture = Fixture::new();
        let mut store = fixture.open();
        let first = pack("1.0.0");
        store.install(&first, &first.digest().unwrap()).unwrap();
        assert!(store.ledger.activation.enabled.is_empty());
        let selected = [(
            first.id.clone(),
            packs::Selection {
                version: first.version,
                resources: ["guide".into()].into_iter().collect(),
            },
        )]
        .into_iter()
        .collect();
        let review = store.review(selected).unwrap();
        store.activate(&review).unwrap();
        assert!(store.activate(&review).is_err());
        let restarted = fixture.open();
        assert_eq!(restarted.ledger.activation, store.ledger.activation);
        assert!(store.purge(&first.key()).is_err());
        let off = store.review(BTreeMap::new()).unwrap();
        store.activate(&off).unwrap();
        let warning = store.purge(&first.key()).unwrap();
        assert!(warning.is_none());
        assert!(store.ledger.versions.is_empty());
        let mut changed = first.clone();
        changed.publisher = "Different claim".into();
        let second = pack("2.0.0");
        store.install(&second, &second.digest().unwrap()).unwrap();
        store.install(&first, &first.digest().unwrap()).unwrap();
        assert!(store.install(&changed, &changed.digest().unwrap()).is_err());
    }
    #[test]
    fn concurrent_windows_stale_review_and_torn_or_corrupt_ledger_are_distinguished() {
        let fixture = Fixture::new();
        let mut a = fixture.open();
        let mut b = fixture.open();
        let first = pack("1.0.0");
        let guard = a.lock().unwrap();
        assert!(b.install(&first, &first.digest().unwrap()).is_err());
        drop(guard);
        a.install(&first, &first.digest().unwrap()).unwrap();
        assert!(b.install(&first, &first.digest().unwrap()).is_err());
        b.reload().unwrap();
        let review = b.review(BTreeMap::new()).unwrap();
        let second = pack("2.0.0");
        a.install(&second, &second.digest().unwrap()).unwrap();
        assert!(b.activate(&review).is_err());
        let next = a.states().unwrap().last().unwrap().0 + 1;
        let path = a.root.join("states").join(format!("{next:016}.state"));
        fs::write(&path, b"FRC1\n{torn").unwrap();
        assert_eq!(fixture.open().ledger.revision, a.ledger.revision);
        fs::write(path, b"FRC1\n{}\nDONE\n").unwrap();
        assert!(Store::open_at(&fixture.0).is_err());
    }
    #[test]
    fn changed_version_bytes_require_explicit_exact_repair_or_preserving_quarantine() {
        let fixture = Fixture::new();
        let mut store = fixture.open();
        let first = pack("1.0.0");
        let hash = first.digest().unwrap();
        store.install(&first, &hash).unwrap();
        let review = store
            .review(
                [(
                    first.id.clone(),
                    packs::Selection {
                        version: first.version,
                        resources: ["guide".into()].into_iter().collect(),
                    },
                )]
                .into_iter()
                .collect(),
            )
            .unwrap();
        store.activate(&review).unwrap();
        let path = store.path(&hash).unwrap();
        fs::write(&path, b"corrupt bytes").unwrap();
        assert!(store.catalog().is_err());
        assert!(store.issues().contains_key(&first.key()));
        assert!(store.install(&first, &hash).is_err());
        let mut wrong = first.clone();
        wrong.publisher = "Changed repair".into();
        assert!(store.repair_data(&wrong, &wrong.digest().unwrap()).is_err());
        store.repair_data(&first, &hash).unwrap();
        assert_eq!(store.pack(&first.key()).unwrap(), first);
        assert_eq!(
            fs::read_dir(store.root.join("quarantine")).unwrap().count(),
            1
        );
        fs::write(&path, b"different corrupt bytes").unwrap();
        let expected = store.ledger.seal().unwrap();
        store.repair_disable(&expected).unwrap();
        assert!(store.ledger.activation.enabled.is_empty());
        assert!(store.catalog().is_ok());
        store.quarantine(&first.key()).unwrap();
        assert!(!path.exists());
        assert!(store.ledger.versions.is_empty());
        assert_eq!(
            fs::read_dir(store.root.join("quarantine")).unwrap().count(),
            2
        );
    }
}
