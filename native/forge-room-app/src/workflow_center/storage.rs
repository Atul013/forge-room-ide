//! Private create-only run snapshots and receipts. No record is a launch permission.
use super::resolver::{self, Review};
use crate::controlled_build::validation::Evidence;
use forge_room_core::workflows::{self, Run};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_RECORD: u64 = 8 * 1024 * 1024;
const MAX_STORAGE: u64 = 512 * 1024 * 1024;
const MAX_RUNS: usize = 32;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub requested: String,
    pub digest: String,
    pub bytes: u64,
    pub stored: Option<String>,
    pub error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Detail {
    pub version: u32,
    pub run: String,
    pub step: String,
    pub attempt: u32,
    pub command: Option<String>,
    pub native: Option<Evidence>,
    pub model_endpoint: Option<String>,
    pub model: Option<String>,
    pub prompt_digest: Option<String>,
    pub model_text: Option<String>,
    pub note: String,
    pub inputs_unchanged: bool,
    pub artifacts: Vec<Artifact>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetailRef {
    pub step: String,
    pub attempt: u32,
    pub file: String,
    pub digest: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SavedRun {
    pub version: u32,
    pub run: Run,
    pub review_digest: String,
    pub details: Vec<DetailRef>,
    pub saved_at: u64,
}
#[derive(Clone)]
pub struct Record {
    pub saved: SavedRun,
    pub review: Review,
}
pub struct Store {
    pub root: PathBuf,
    pub source: PathBuf,
    _lock: fs::File,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Draft {
    pub library: super::library::Library,
    pub baseline: Option<String>,
    pub json: Option<String>,
}
fn safe_file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if resolver::linked(&metadata) || !metadata.is_file() {
        return Err("Linked/non-regular workflow storage is unsupported".into());
    }
    Ok(())
}
fn safe_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if resolver::linked(&metadata) || !metadata.is_dir() {
        return Err("Linked/non-directory workflow storage is unsupported".into());
    }
    Ok(())
}
pub fn write_new(path: &Path, value: &impl Serialize) -> Result<(), String> {
    crate::local_records::write_new(path, MAX_RECORD, value)
}
fn read_envelope<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    crate::local_records::read(path, MAX_RECORD).map_err(|error| {
        if error == crate::local_records::INCOMPLETE {
            "Incomplete workflow record".into()
        } else {
            error
        }
    })
}
impl SavedRun {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.details.len() > 512 || self.review_digest.len() != 64 {
            return Err("Invalid saved workflow ledger".into());
        }
        self.run.validate()?;
        let mut seen = std::collections::BTreeSet::new();
        for detail in &self.details {
            if !self.run.steps.iter().any(|step| {
                step.id == detail.step
                    && step
                        .attempts
                        .iter()
                        .any(|attempt| attempt.number == detail.attempt)
            }) || detail.digest.len() != 64
                || !detail.file.ends_with(".detail")
                || Path::new(&detail.file).components().count() != 1
                || detail.file.contains(['\\', '/', '\0', ':'])
                || !seen.insert((&detail.step, detail.attempt))
            {
                return Err("Invalid attempt detail reference".into());
            }
        }
        Ok(())
    }
}
impl Record {
    pub fn new(run: Run, review: Review) -> Result<Self, String> {
        let review_digest = workflows::seal(&review)?;
        let saved = SavedRun {
            version: 1,
            run,
            review_digest,
            details: vec![],
            saved_at: forge_room_core::now_epoch_seconds(),
        };
        let result = Self { saved, review };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.saved.validate()?;
        self.review.validate()?;
        if self.saved.review_digest != workflows::seal(&self.review)?
            || self.saved.run.plan != self.review.expanded
            || self.saved.run.workspace != self.review.source.to_string_lossy()
        {
            return Err("Run/review context binding changed".into());
        }
        Ok(())
    }
}
impl Store {
    pub fn request_receipt(
        &self,
        id: &str,
        step: &str,
        attempt: u32,
        request: &serde_json::Value,
    ) -> Result<(), String> {
        if !workflows::identifier(step) {
            return Err("Invalid request step".into());
        }
        let folder = self.folder(id, false)?;
        let path = folder.join(format!("{step}-{attempt:04}.request"));
        if path.exists() {
            let old: serde_json::Value = read_envelope(&path)?;
            if old != *request {
                return Err("Exact reviewed model request changed".into());
            }
            return Ok(());
        }
        self.capacity(8192)?;
        write_new(&path, request)
    }
    pub fn export_artifact(
        &self,
        id: &str,
        artifact: &Artifact,
        target: &Path,
    ) -> Result<(), String> {
        let folder = self.folder(id, false)?;
        let name = artifact
            .stored
            .as_ref()
            .ok_or("Artifact was not captured")?;
        if Path::new(name).components().count() != 1 || name.contains(['/', '\\', ':']) {
            return Err("Invalid artifact path".into());
        }
        let path = folder.join(name);
        let seal = resolver::file_seal(&path, 8 * 1024 * 1024)?;
        if seal.digest != artifact.digest || seal.bytes != artifact.bytes {
            return Err("Stored artifact changed".into());
        }
        let mut bytes = Vec::new();
        fs::File::open(&path)
            .map_err(|e| e.to_string())?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != seal.bytes || workflows::digest(&bytes) != seal.digest {
            return Err("Artifact changed during export".into());
        }
        super::library::create_file(target, &bytes)
    }
    fn drafts(&self) -> Result<Vec<(u64, PathBuf)>, String> {
        let mut drafts = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(number) = name
                .strip_prefix("draft-")
                .and_then(|name| name.strip_suffix(".record"))
            {
                let number = number
                    .parse::<u64>()
                    .map_err(|_| "Invalid private draft sequence")?;
                safe_file(&entry.path())?;
                drafts.push((number, entry.path()));
                if drafts.len() > 64 {
                    return Err("Too many private draft snapshots".into());
                }
            }
        }
        drafts.sort_by_key(|(number, _)| *number);
        Ok(drafts)
    }
    pub fn save_draft(&self, draft: &Draft) -> Result<(), String> {
        let bytes = serde_json::to_vec(draft).map_err(|e| e.to_string())?;
        if bytes.len() > 3 * 1024 * 1024 {
            return Err("Private workflow draft exceeds 3 MiB".into());
        }
        self.capacity(bytes.len() as u64 + 1024)?;
        let mut drafts = self.drafts()?;
        let next = drafts
            .last()
            .map_or(1, |(number, _)| number.saturating_add(1));
        let file = self.root.join(format!("draft-{next:016}.record"));
        write_new(&file, draft)?;
        drafts.push((next, file));
        let excess = drafts.len().saturating_sub(3);
        for (_, path) in drafts.into_iter().take(excess) {
            safe_file(&path)?;
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn load_draft(&self) -> Result<Option<Draft>, String> {
        for (_, path) in self.drafts()?.into_iter().rev() {
            match read_envelope::<Draft>(&path) {
                Ok(draft) => {
                    if serde_json::to_vec(&draft).map_err(|e| e.to_string())?.len()
                        > 3 * 1024 * 1024
                    {
                        return Err("Oversized private draft".into());
                    }
                    return Ok(Some(draft));
                }
                Err(error) if error == "Incomplete workflow record" => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }
    pub fn open(source: &Path) -> Result<Self, String> {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Forge Room Early Preview")
            .join("workflow-center");
        Self::open_at(&base, source)
    }
    pub fn open_at(base: &Path, source: &Path) -> Result<Self, String> {
        let source = resolver::canonical(source)?;
        fs::create_dir_all(base).map_err(|e| e.to_string())?;
        safe_directory(base)?;
        let key = workflows::digest(source.to_string_lossy().replace('\\', "/").as_bytes());
        let root = base.join(key);
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        safe_directory(&root)?;
        if !resolver::canonical(&root)?.starts_with(resolver::canonical(base)?) {
            return Err("Workflow storage escaped its owned base".into());
        }
        let lock_path = root.join(".writer-lock");
        if lock_path.exists() {
            safe_file(&lock_path)?;
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|e| format!("Another window owns this project's workflow writer: {e}"))?;
        Ok(Self {
            root,
            source,
            _lock: lock,
        })
    }
    fn folder(&self, id: &str, create: bool) -> Result<PathBuf, String> {
        if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid workflow run identity".into());
        }
        safe_directory(&self.root)?;
        let folder = self.root.join(id);
        if create && !folder.exists() {
            if self.ids()?.len() >= MAX_RUNS {
                return Err(
                    "Workflow history limit reached; explicitly delete an inactive history entry"
                        .into(),
                );
            }
            fs::create_dir(&folder).map_err(|e| e.to_string())?;
        }
        safe_directory(&folder)?;
        if !resolver::canonical(&folder)?.starts_with(resolver::canonical(&self.root)?) {
            return Err("Run folder escaped owned storage".into());
        }
        Ok(folder)
    }
    pub fn ids(&self) -> Result<Vec<String>, String> {
        safe_directory(&self.root)?;
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.len() == 32 && name.bytes().all(|b| b.is_ascii_hexdigit()) {
                safe_directory(&entry.path())?;
                ids.push(name);
                if ids.len() > MAX_RUNS {
                    return Err("Workflow history exceeds its entry limit".into());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }
    pub fn usage(&self) -> Result<u64, String> {
        let mut bytes = 0u64;
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name() == ".writer-lock" {
                continue;
            }
            let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            if metadata.is_file() {
                safe_file(&entry.path())?;
                bytes = bytes.saturating_add(metadata.len());
            }
        }
        for id in self.ids()? {
            let folder = self.folder(&id, false)?;
            for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                safe_file(&entry.path())?;
                bytes = bytes.saturating_add(entry.metadata().map_err(|e| e.to_string())?.len());
            }
        }
        Ok(bytes)
    }
    fn capacity(&self, bytes: u64) -> Result<(), String> {
        if self.usage()?.saturating_add(bytes) > MAX_STORAGE {
            return Err("Private workflow history reached 512 MiB; prune inactive history before continuing".into());
        }
        Ok(())
    }
    pub fn save(&self, record: &mut Record) -> Result<(), String> {
        record.saved.saved_at = forge_room_core::now_epoch_seconds();
        record.validate()?;
        if record.review.source != self.source {
            return Err("Run belongs to another workspace".into());
        }
        let folder = self.folder(&record.saved.run.id, true)?;
        let review_path = folder.join("review.record");
        if !review_path.exists() {
            self.capacity(
                serde_json::to_vec(&record.review)
                    .map_err(|e| e.to_string())?
                    .len() as u64
                    + 1024,
            )?;
            write_new(&review_path, &record.review)?;
        } else {
            let old: Review = read_envelope(&review_path)?;
            if old != record.review {
                return Err("Saved run review cannot be replaced".into());
            }
        }
        let mut snapshots = self.snapshots(&folder)?;
        let next = snapshots
            .last()
            .map_or(1, |(number, _)| number.saturating_add(1));
        let file = folder.join(format!("{next:016}.run"));
        self.capacity(
            serde_json::to_vec(&record.saved)
                .map_err(|e| e.to_string())?
                .len() as u64
                + 1024,
        )?;
        write_new(&file, &record.saved)?;
        snapshots.push((next, file));
        // Only redundant internal snapshots are compacted; complete attempt receipts/artifacts remain.
        let excess = snapshots.len().saturating_sub(3);
        for (_, old) in snapshots.into_iter().take(excess) {
            safe_file(&old)?;
            if old.parent() != Some(folder.as_path()) {
                return Err("Snapshot compaction escaped its run folder".into());
            }
            let _ = fs::remove_file(old);
        }
        Ok(())
    }
    fn snapshots(&self, folder: &Path) -> Result<Vec<(u64, PathBuf)>, String> {
        let mut files = Vec::new();
        for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(number) = name.strip_suffix(".run") {
                if number.len() != 16 || !number.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("Invalid run snapshot filename".into());
                }
                safe_file(&entry.path())?;
                files.push((
                    number.parse().map_err(|_| "Invalid snapshot sequence")?,
                    entry.path(),
                ));
                if files.len() > 2048 {
                    return Err("Too many run snapshots".into());
                }
            }
        }
        files.sort_by_key(|(sequence, _)| *sequence);
        Ok(files)
    }
    pub fn load(&self, id: &str) -> Result<Record, String> {
        let folder = self.folder(id, false)?;
        let review: Review = read_envelope(&folder.join("review.record"))?;
        let snapshots = self.snapshots(&folder)?;
        for (_, path) in snapshots.into_iter().rev() {
            match read_envelope::<SavedRun>(&path) {
                Ok(saved) => {
                    let record = Record { saved, review };
                    record.validate()?;
                    if record.saved.run.id != id || record.review.source != self.source {
                        return Err("Saved run identity/root mismatch".into());
                    }
                    return Ok(record);
                }
                Err(error) if error == "Incomplete workflow record" => continue,
                Err(error) => return Err(error),
            }
        }
        Err("No complete workflow snapshot exists".into())
    }
    pub fn save_detail(&self, record: &mut Record, detail: &Detail) -> Result<(), String> {
        if detail.version != 1
            || detail.run != record.saved.run.id
            || !workflows::identifier(&detail.step)
            || detail.artifacts.len() > 16
        {
            return Err("Invalid attempt detail".into());
        }
        let folder = self.folder(&detail.run, false)?;
        let file = format!("{}-{:04}.detail", detail.step, detail.attempt);
        let digest = workflows::seal(detail)?;
        self.capacity(serde_json::to_vec(detail).map_err(|e| e.to_string())?.len() as u64 + 1024)?;
        write_new(&folder.join(&file), detail)?;
        record.saved.details.push(DetailRef {
            step: detail.step.clone(),
            attempt: detail.attempt,
            file,
            digest,
        });
        Ok(())
    }
    pub fn detail(&self, id: &str, reference: &DetailRef) -> Result<Detail, String> {
        let folder = self.folder(id, false)?;
        if Path::new(&reference.file).components().count() != 1
            || reference.file.contains(['/', '\\', ':'])
        {
            return Err("Invalid detail path".into());
        }
        let detail: Detail = read_envelope(&folder.join(&reference.file))?;
        if detail.run != id
            || detail.step != reference.step
            || detail.attempt != reference.attempt
            || workflows::seal(&detail)? != reference.digest
        {
            return Err("Attempt detail changed".into());
        }
        Ok(detail)
    }
    pub fn capture_artifact(
        &self,
        id: &str,
        step: &str,
        attempt: u32,
        cwd: &Path,
        relative: &str,
        remaining: u64,
    ) -> Artifact {
        let result = (|| {
            if !workflows::identifier(step) {
                return Err("Invalid step identity".into());
            }
            let path = resolver::path(cwd, relative, true)?;
            let seal = resolver::file_seal(&path, remaining.min(8 * 1024 * 1024))?;
            let folder = self.folder(id, false)?;
            let name = format!(
                "artifact-{}-{:04}-{}.bin",
                step,
                attempt,
                &seal.digest[..16]
            );
            let target = folder.join(&name);
            if !target.exists() {
                self.capacity(seal.bytes + 1024)?;
                let mut input = fs::File::open(&path).map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                std::io::Read::by_ref(&mut input)
                    .take(seal.bytes + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                if bytes.len() as u64 != seal.bytes || workflows::digest(&bytes) != seal.digest {
                    return Err("Artifact changed while being captured".into());
                }
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&target)
                    .map_err(|e| e.to_string())?;
                file.write_all(&bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|e| e.to_string())?;
            } else {
                let previous = resolver::file_seal(&target, 8 * 1024 * 1024)?;
                if previous.digest != seal.digest {
                    return Err("Stored artifact changed".into());
                }
            }
            Ok::<_, String>((seal, name))
        })();
        match result {
            Ok((seal, name)) => Artifact {
                requested: relative.into(),
                digest: seal.digest,
                bytes: seal.bytes,
                stored: Some(name),
                error: None,
            },
            Err(error) => Artifact {
                requested: relative.into(),
                digest: String::new(),
                bytes: 0,
                stored: None,
                error: Some(error),
            },
        }
    }
    pub fn verify_artifacts(&self, id: &str, detail: &Detail) -> BTreeMap<String, bool> {
        let folder = self.folder(id, false).ok();
        detail
            .artifacts
            .iter()
            .map(|artifact| {
                let valid =
                    folder
                        .as_ref()
                        .zip(artifact.stored.as_ref())
                        .is_some_and(|(folder, name)| {
                            Path::new(name).components().count() == 1
                                && !name.contains(['/', '\\', ':'])
                                && resolver::file_seal(&folder.join(name), 8 * 1024 * 1024)
                                    .is_ok_and(|seal| {
                                        seal.bytes == artifact.bytes
                                            && seal.digest == artifact.digest
                                    })
                        });
                (artifact.requested.clone(), valid)
            })
            .collect()
    }
    pub fn delete(&self, id: &str) -> Result<(), String> {
        let record = self.load(id)?;
        if !record.saved.run.terminal()
            && !matches!(
                record.saved.run.status,
                workflows::RunStatus::Draft | workflows::RunStatus::Interrupted
            )
        {
            return Err("Stop/recover a run before deleting its history".into());
        }
        let folder = self.folder(id, false)?;
        let root = resolver::canonical(&self.root)?;
        let resolved = resolver::canonical(&folder)?;
        if !resolved.starts_with(&root) || resolved == root {
            return Err("Refused unsafe history deletion".into());
        }
        for entry in fs::read_dir(&folder).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            safe_file(&path)?;
            if path.parent() != Some(folder.as_path()) {
                return Err("History path escaped run directory".into());
            }
        }
        for entry in fs::read_dir(&folder).map_err(|e| e.to_string())? {
            fs::remove_file(entry.map_err(|e| e.to_string())?.path()).map_err(|e| e.to_string())?;
        }
        fs::remove_dir(folder).map_err(|e| e.to_string())
    }
}
