//! Bounded immutable inspection. Inspecting never opens a listener or mints recovery authority.
use super::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
const MAX_JOURNAL: u64 = 64 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryKind {
    Checkpoint,
    Journal,
}
impl RecoveryKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Checkpoint => "Public checkpoint",
            Self::Journal => "Public journal",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoverySummary {
    pub kind: RecoveryKind,
    pub room: String,
    pub room_id: String,
    pub captured: u64,
    pub lifecycle: RoomLifecycle,
    pub cursor: u64,
    pub members: usize,
    pub tasks: usize,
    pub done: usize,
    pub messages: usize,
    pub evidence: usize,
    pub milestones: usize,
    pub readiness: usize,
    pub pending_impacts: usize,
    pub ship_revision: u64,
    pub countdown_seconds: u64,
    pub records: usize,
    pub complete_bytes: u64,
    pub tail_bytes: u64,
    pub file_bytes: u64,
    pub digest: String,
    pub public_state_digest: String,
}
/// Not serializable and not externally mutable. The raw source is never retained or exported.
#[derive(Clone)]
pub struct RecoveryPreview {
    source: PathBuf,
    summary: RecoverySummary,
    checkpoint: Checkpoint,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryFileStamp {
    source: PathBuf,
    digest: String,
    bytes: u64,
}
impl RecoveryFileStamp {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}
pub fn recovery_file_stamp(path: &Path) -> Result<RecoveryFileStamp> {
    let (source, bytes) = read(path)?;
    Ok(RecoveryFileStamp {
        source,
        digest: crate::workflows::digest(&bytes),
        bytes: bytes.len() as u64,
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryTaskChange {
    pub id: String,
    pub title: String,
    pub change: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryComparison {
    pub same_room: bool,
    pub same_public_state: bool,
    pub before_cursor: u64,
    pub after_cursor: u64,
    pub before_lifecycle: RoomLifecycle,
    pub after_lifecycle: RoomLifecycle,
    pub coordination: Vec<String>,
    pub tasks: Vec<RecoveryTaskChange>,
}
fn regular(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let linked = metadata.file_type().is_symlink();
    if linked || !metadata.is_file() {
        return Err("Choose a regular non-linked recovery file".into());
    }
    Ok(())
}
fn read(path: &Path) -> Result<(PathBuf, Vec<u8>)> {
    regular(path)?;
    let source = path.canonicalize().map_err(|e| e.to_string())?;
    regular(&source)?;
    let file = fs::File::open(&source).map_err(|e| e.to_string())?;
    let before = file.metadata().map_err(|e| e.to_string())?;
    if before.len() > MAX_JOURNAL {
        return Err("Recovery source exceeds 64 MiB".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_JOURNAL + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let after = fs::metadata(&source).map_err(|e| e.to_string())?;
    regular(&source)?;
    if bytes.len() as u64 != before.len()
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(
            "Recovery source changed during inspection; inspect a stable file again".into(),
        );
    }
    Ok((source, bytes))
}
fn checkpoint(bytes: &[u8]) -> Result<Checkpoint> {
    if bytes.len() > MAX_FRAME {
        return Err("Checkpoint exceeds 8 MiB".into());
    }
    let checkpoint: Checkpoint = serde_json::from_slice(bytes).map_err(|e| {
        format!(
            "Invalid public recovery checkpoint JSON/schema at line {}, column {} ({:?})",
            e.line(),
            e.column(),
            e.classify()
        )
    })?;
    checkpoint
        .validate()
        .map_err(|_| "Invalid public recovery checkpoint state or references".to_string())?;
    Ok(checkpoint)
}
pub fn inspect_recovery(path: &Path) -> Result<RecoveryPreview> {
    let (source, bytes) = read(path)?;
    let (kind, saved, records, complete) = if bytes.starts_with(b"FRJ1\n") {
        let mut offset = 5usize;
        let mut records = 0usize;
        let mut previous: Option<Checkpoint> = None;
        while bytes.len().saturating_sub(offset) >= 4 {
            let length = u32::from_be_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .map_err(|_| "Invalid journal header")?,
            ) as usize;
            if length > MAX_FRAME {
                return Err("Invalid journal record length".into());
            }
            let end = offset + 4 + length;
            if bytes.len() < end + 4 {
                break;
            }
            if &bytes[end..end + 4] != b"DONE" {
                return Err("Corrupt complete journal record".into());
            }
            let next = checkpoint(&bytes[offset + 4..end])?;
            if let Some(before) = &previous {
                if before.host != next.host
                    || before.snapshot.room != next.snapshot.room
                    || before.snapshot.shared.room_id != next.snapshot.shared.room_id
                    || before.snapshot.cursor > next.snapshot.cursor
                    || (before.snapshot.lifecycle == RoomLifecycle::Archived
                        && next.snapshot.lifecycle != RoomLifecycle::Archived)
                {
                    return Err("Journal Room identity, public message cursor or terminal archive continuity changed; original source preserved".into());
                }
            }
            previous = Some(next);
            records += 1;
            offset = end + 4;
        }
        (
            RecoveryKind::Journal,
            previous.ok_or("Journal has no complete saved record")?,
            records,
            offset,
        )
    } else {
        (
            RecoveryKind::Checkpoint,
            checkpoint(&bytes)?,
            1,
            bytes.len(),
        )
    };
    let snapshot = &saved.snapshot;
    let public_bytes = serde_json::to_vec(&(
        &saved.host,
        &snapshot.room,
        &snapshot.shared,
        &snapshot.members,
        &snapshot.events,
        snapshot.cursor,
        snapshot.lifecycle,
    ))
    .map_err(|e| e.to_string())?;
    let summary = RecoverySummary {
        kind,
        room: snapshot.room.clone(),
        room_id: snapshot.shared.room_id.clone(),
        captured: snapshot.server_time_epoch_seconds,
        lifecycle: snapshot.lifecycle,
        cursor: snapshot.cursor,
        members: snapshot.members.len(),
        tasks: snapshot.shared.tasks.len(),
        done: snapshot
            .shared
            .tasks
            .iter()
            .filter(|task| task.status == crate::TaskStatus::Done)
            .count(),
        messages: snapshot.events.len(),
        evidence: snapshot.shared.evidence.len(),
        milestones: snapshot.shared.milestones.len(),
        readiness: snapshot.shared.readiness.profiles.len(),
        pending_impacts: snapshot
            .shared
            .readiness
            .impacts
            .iter()
            .filter(|impact| impact.acknowledged_by.is_none())
            .count(),
        ship_revision: snapshot.shared.shipping.revision,
        countdown_seconds: snapshot.shared.countdown.remaining_seconds,
        records,
        complete_bytes: complete as u64,
        tail_bytes: bytes.len().saturating_sub(complete) as u64,
        file_bytes: bytes.len() as u64,
        digest: crate::workflows::digest(&bytes),
        public_state_digest: crate::workflows::digest(&public_bytes),
    };
    Ok(RecoveryPreview {
        source,
        summary,
        checkpoint: saved,
    })
}
impl RecoveryPreview {
    pub fn file_stamp(&self) -> RecoveryFileStamp {
        RecoveryFileStamp {
            source: self.source.clone(),
            digest: self.summary.digest.clone(),
            bytes: self.summary.file_bytes,
        }
    }
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn summary(&self) -> &RecoverySummary {
        &self.summary
    }
    pub fn tasks(&self) -> &[crate::Task] {
        &self.checkpoint.snapshot.shared.tasks
    }
    pub fn members(&self) -> &[Member] {
        &self.checkpoint.snapshot.members
    }
    pub fn recheck(&self) -> Result<()> {
        let (source, bytes) = read(&self.source)?;
        if source != self.source || crate::workflows::digest(&bytes) != self.summary.digest {
            return Err("Recovery source changed since review; inspect it again".into());
        }
        Ok(())
    }
    /// Export validated typed fields only; unknown input JSON is deliberately not copied.
    pub fn export_checkpoint(&self, path: &Path) -> Result<()> {
        self.recheck()?;
        self.checkpoint.validate()?;
        let bytes = serde_json::to_vec_pretty(&self.checkpoint).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_FRAME {
            return Err("Sanitized checkpoint exceeds 8 MiB".into());
        }
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())
    }
    pub fn compare(&self, after: &Self) -> Result<RecoveryComparison> {
        let same_room = self.checkpoint.host == after.checkpoint.host
            && self.summary.room_id == after.summary.room_id
            && self.summary.room == after.summary.room;
        let mut tasks = Vec::new();
        let mut coordination = Vec::new();
        if same_room {
            let a = &self.checkpoint.snapshot;
            let b = &after.checkpoint.snapshot;
            for (label, before, after) in [
                (
                    "Lifecycle",
                    crate::workflows::seal(&a.lifecycle)?,
                    crate::workflows::seal(&b.lifecycle)?,
                ),
                (
                    "Countdown snapshot",
                    crate::workflows::seal(&a.shared.countdown)?,
                    crate::workflows::seal(&b.shared.countdown)?,
                ),
                (
                    "Readiness / help / impacts",
                    crate::workflows::seal(&a.shared.readiness)?,
                    crate::workflows::seal(&b.shared.readiness)?,
                ),
                (
                    "Ship declarations",
                    crate::workflows::seal(&a.shared.shipping)?,
                    crate::workflows::seal(&b.shared.shipping)?,
                ),
                (
                    "Milestone snapshot",
                    crate::workflows::seal(&a.shared.milestones)?,
                    crate::workflows::seal(&b.shared.milestones)?,
                ),
                (
                    "Saved member snapshot",
                    crate::workflows::seal(&a.members)?,
                    crate::workflows::seal(&b.members)?,
                ),
                (
                    "Retained public messages",
                    crate::workflows::seal(&a.events)?,
                    crate::workflows::seal(&b.events)?,
                ),
                (
                    "Handoff snapshot",
                    crate::workflows::seal(&a.shared.handoffs)?,
                    crate::workflows::seal(&b.shared.handoffs)?,
                ),
                (
                    "Evidence declarations",
                    crate::workflows::seal(&a.shared.evidence)?,
                    crate::workflows::seal(&b.shared.evidence)?,
                ),
                (
                    "Retained public activity",
                    crate::workflows::seal(&a.shared.activity)?,
                    crate::workflows::seal(&b.shared.activity)?,
                ),
            ] {
                if before != after {
                    coordination.push(label.into());
                }
            }
            let before = self
                .tasks()
                .iter()
                .map(|task| (task.id.clone(), task))
                .collect::<BTreeMap<_, _>>();
            let after = after
                .tasks()
                .iter()
                .map(|task| (task.id.clone(), task))
                .collect::<BTreeMap<_, _>>();
            for id in before
                .keys()
                .chain(after.keys())
                .collect::<std::collections::BTreeSet<_>>()
            {
                let change = match (before.get(id), after.get(id)) {
                    (None, Some(_)) => Some("added"),
                    (Some(_), None) => Some("absent in selected source"),
                    (Some(a), Some(b))
                        if crate::workflows::seal(a)? != crate::workflows::seal(b)? =>
                    {
                        Some("changed")
                    }
                    _ => None,
                };
                if let Some(change) = change {
                    let task = after
                        .get(id)
                        .or_else(|| before.get(id))
                        .ok_or("Missing comparison task")?;
                    tasks.push(RecoveryTaskChange {
                        id: id.clone(),
                        title: task.title.clone(),
                        change: change.into(),
                    });
                }
            }
        }
        Ok(RecoveryComparison {
            same_room,
            same_public_state: self.summary.public_state_digest
                == after.summary.public_state_digest,
            before_cursor: self.summary.cursor,
            after_cursor: after.summary.cursor,
            before_lifecycle: self.summary.lifecycle,
            after_lifecycle: after.summary.lifecycle,
            coordination,
            tasks,
        })
    }
}
impl Host {
    pub fn recover_preview(bind: SocketAddr, preview: &RecoveryPreview) -> Result<Self> {
        preview.recheck()?;
        let bytes = serde_json::to_vec(&preview.checkpoint).map_err(|e| e.to_string())?;
        let host = Self::recover_bytes(bind, &bytes)?;
        if preview.summary.kind == RecoveryKind::Journal {
            host.authority
                .lock()
                .map_err(|_| "Room unavailable")?
                .journal_required = true;
        }
        Ok(host)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_schema_diagnostics_do_not_echo_untrusted_json_values_into_recovery_metadata() {
        let fixture = Fixture::new();
        let path = fixture.path("invalid.json");
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "diagnostic privacy").unwrap();
        host.save_checkpoint(&path).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["snapshot"]["lifecycle"] =
            serde_json::json!("UNTRUSTED_PRIVATE_VALUE_MUST_NOT_BE_LOGGED");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let error = inspect_recovery(&path).err().unwrap();
        assert!(error.contains("JSON/schema"));
        assert!(!error.contains("UNTRUSTED_PRIVATE_VALUE"));
        let stamp = recovery_file_stamp(&path).unwrap();
        assert_eq!(stamp.bytes(), fs::metadata(&path).unwrap().len());
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("forge-recovery-inspection-{}", secret().unwrap()));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            assert!(self
                .0
                .starts_with(std::env::temp_dir().canonicalize().unwrap()));
            assert!(self
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("forge-recovery-inspection-"));
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn inspected_checkpoint_is_stale_guarded_sanitized_and_restores_without_old_grants() {
        let fixture = Fixture::new();
        let path = fixture.path("room.json");
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "inspected").unwrap();
        let old = host.invite(Duration::from_secs(60)).unwrap();
        let (mut member, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Ada").unwrap();
        member.post("public work").unwrap();
        host.save_checkpoint(&path).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["snapshot"]["ignored_private_history"] =
            serde_json::json!("PRIVATE_UNKNOWN_MUST_NOT_EXPORT");
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let preview = inspect_recovery(&path).unwrap();
        assert_eq!(preview.summary().kind, RecoveryKind::Checkpoint);
        assert_eq!(preview.summary().members, 1);
        assert_eq!(preview.summary().messages, 1);
        assert_eq!(preview.members()[0].name, "Ada");
        let export = fixture.path("sanitized.json");
        preview.export_checkpoint(&export).unwrap();
        assert!(preview.export_checkpoint(&export).is_err());
        assert!(!String::from_utf8(fs::read(&export).unwrap())
            .unwrap()
            .contains("PRIVATE_UNKNOWN"));
        let recovered = Host::recover_preview("127.0.0.1:0".parse().unwrap(), &preview).unwrap();
        assert_eq!(recovered.lifecycle().unwrap(), RoomLifecycle::Paused);
        assert_eq!(
            recovered.recovered_members().unwrap()[0].id,
            member.identity
        );
        let mut old = old;
        old.address = recovered.address();
        assert!(Client::join(old, "not an owner").is_err());
        value["snapshot"]["server_time_epoch_seconds"] = serde_json::json!(1);
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(preview.recheck().is_err());
        assert!(Host::recover_preview("127.0.0.1:0".parse().unwrap(), &preview).is_err());
        assert!(preview
            .export_checkpoint(&fixture.path("stale.json"))
            .is_err());
    }
    #[test]
    fn journal_tail_report_exports_last_complete_state_and_requires_new_durability_before_resume() {
        let fixture = Fixture::new();
        let path = fixture.path("room.frjournal");
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "journal inspect").unwrap();
        host.enable_journal(&path).unwrap();
        let (mut member, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Ada").unwrap();
        member.post("acknowledged").unwrap();
        drop(member);
        drop(host);
        let complete = fs::metadata(&path).unwrap().len();
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&[0, 0, 0, 30, b'{', b't']).unwrap();
        drop(file);
        let preview = inspect_recovery(&path).unwrap();
        assert_eq!(preview.summary().kind, RecoveryKind::Journal);
        assert_eq!(preview.summary().tail_bytes, 6);
        assert_eq!(preview.summary().complete_bytes, complete);
        assert!(preview.summary().records >= 2);
        let export = fixture.path("last-complete.json");
        preview.export_checkpoint(&export).unwrap();
        assert_eq!(inspect_recovery(&export).unwrap().summary().messages, 1);
        assert_eq!(fs::metadata(&path).unwrap().len(), complete + 6);
        let recovered = Host::recover_preview("127.0.0.1:0".parse().unwrap(), &preview).unwrap();
        assert!(recovered.resume().is_err());
        recovered
            .enable_journal(&fixture.path("fresh.frjournal"))
            .unwrap();
        recovered.resume().unwrap();
        assert_eq!(recovered.lifecycle().unwrap(), RoomLifecycle::Active);
    }
    #[test]
    fn individually_valid_foreign_records_and_regressing_message_cursors_are_rejected() {
        let fixture = Fixture::new();
        let path = fixture.path("room.frjournal");
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "first").unwrap();
        host.enable_journal(&path).unwrap();
        let initial = fs::read(&path).unwrap();
        let (mut client, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Ada").unwrap();
        client.post("later").unwrap();
        let later = fs::read(&path).unwrap();
        drop(client);
        drop(host);
        let mut regressing = later.clone();
        regressing.extend_from_slice(&initial[5..]);
        fs::write(&path, regressing).unwrap();
        assert!(inspect_recovery(&path)
            .err()
            .unwrap()
            .contains("continuity"));
        assert!(Host::recover_journal("127.0.0.1:0".parse().unwrap(), &path).is_err());
        let other = Host::start("127.0.0.1:0".parse().unwrap(), "second").unwrap();
        let second = fixture.path("second.frjournal");
        other.enable_journal(&second).unwrap();
        let mut mixed = later;
        mixed.extend_from_slice(&fs::read(second).unwrap()[5..]);
        fs::write(&path, mixed).unwrap();
        assert!(inspect_recovery(&path)
            .err()
            .unwrap()
            .contains("continuity"));
    }
    #[test]
    fn comparison_excludes_capture_clock_but_exposes_task_changes_and_foreign_room_identity() {
        let fixture = Fixture::new();
        let path = fixture.path("first.json");
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "comparison").unwrap();
        host.save_checkpoint(&path).unwrap();
        let before = inspect_recovery(&path).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["snapshot"]["server_time_epoch_seconds"] = serde_json::json!(123);
        let clock = fixture.path("clock.json");
        fs::write(&clock, serde_json::to_vec(&value).unwrap()).unwrap();
        let after = inspect_recovery(&clock).unwrap();
        let comparison = before.compare(&after).unwrap();
        assert!(comparison.same_room);
        assert!(comparison.same_public_state);
        assert!(comparison.tasks.is_empty());
        let (member, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Owner").unwrap();
        host.control(Action::CreateTask {
            title: "Public task".into(),
            owner: member.identity.clone(),
            criteria: vec![],
            dependencies: vec![],
            priority: crate::TaskPriority::Normal,
            due_hours: None,
        })
        .unwrap();
        let changed = fixture.path("changed.json");
        host.save_checkpoint(&changed).unwrap();
        let after = inspect_recovery(&changed).unwrap();
        let comparison = before.compare(&after).unwrap();
        assert!(comparison.same_room);
        assert_eq!(comparison.tasks.len(), 1);
        assert_eq!(comparison.tasks[0].change, "added");
        let other = Host::start("127.0.0.1:0".parse().unwrap(), "comparison").unwrap();
        let foreign = fixture.path("foreign.json");
        other.save_checkpoint(&foreign).unwrap();
        let comparison = before
            .compare(&inspect_recovery(&foreign).unwrap())
            .unwrap();
        assert!(!comparison.same_room);
        assert!(comparison.tasks.is_empty());
    }
    #[test]
    fn complete_corruption_and_kind_mismatch_preserve_originals() {
        let fixture = Fixture::new();
        let path = fixture.path("room.frjournal");
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "bad record").unwrap();
        host.enable_journal(&path).unwrap();
        let original = fs::read(&path).unwrap();
        assert!(Host::recover("127.0.0.1:0".parse().unwrap(), &path).is_err());
        let mut broken = original.clone();
        broken.extend_from_slice(&2u32.to_be_bytes());
        broken.extend_from_slice(b"{}DONE");
        fs::write(&path, &broken).unwrap();
        assert!(inspect_recovery(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), broken);
        let checkpoint = fixture.path("checkpoint.json");
        host.save_checkpoint(&checkpoint).unwrap();
        assert!(Host::recover_journal("127.0.0.1:0".parse().unwrap(), &checkpoint).is_err());
        assert_eq!(fs::read(&path).unwrap(), broken);
    }
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    #[test]
    fn terminal_archive_cannot_be_undone_by_appending_an_earlier_valid_active_frame() {
        let path =
            std::env::temp_dir().join(format!("forge-archive-continuity-{}", secret().unwrap()));
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "terminal history").unwrap();
        host.enable_journal(&path).unwrap();
        let active = fs::read(&path).unwrap();
        host.archive().unwrap();
        let mut bytes = fs::read(&path).unwrap();
        bytes.extend_from_slice(&active[5..]);
        drop(host);
        fs::write(&path, &bytes).unwrap();
        let error = inspect_recovery(&path).err().unwrap();
        assert!(error.contains("archive continuity"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_file(path).unwrap();
    }
}
