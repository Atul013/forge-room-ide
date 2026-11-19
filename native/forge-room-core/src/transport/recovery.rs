//! Explicit immutable public checkpoints. No session token, invite, TLS key, DM or patch is saved.
use super::*;
mod inspection;
mod journal;
pub use inspection::{
    inspect_recovery, recovery_file_stamp, RecoveryComparison, RecoveryFileStamp, RecoveryKind,
    RecoveryPreview, RecoverySummary, RecoveryTaskChange,
};
pub(super) use journal::Journal;
use std::{fs::OpenOptions, path::Path};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    version: u32,
    host: String,
    snapshot: Snapshot,
}

impl Host {
    /// Saves a new checkpoint without replacing any existing file. Later changes require a new save.
    pub fn save_checkpoint(&self, path: &Path) -> Result<()> {
        let state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_storage()?;
        let checkpoint = Checkpoint::capture(&state);
        checkpoint.validate()?;
        let bytes = serde_json::to_vec_pretty(&checkpoint).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_FRAME {
            return Err("Checkpoint is too large".into());
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("Checkpoint was not saved: {e}"))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("Checkpoint incomplete; keep the previous checkpoint: {e}"))
    }

    /// Starts a fresh TLS host from a checkpoint, paused unless the source was archived.
    pub fn recover(bind: SocketAddr, path: &Path) -> Result<Self> {
        let preview = inspect_recovery(path)?;
        if preview.summary().kind != RecoveryKind::Checkpoint {
            return Err("Choose a public checkpoint, not a journal".into());
        }
        Self::recover_preview(bind, &preview)
    }

    fn recover_bytes(bind: SocketAddr, bytes: &[u8]) -> Result<Self> {
        let checkpoint: Checkpoint =
            serde_json::from_slice(bytes).map_err(|e| format!("Invalid checkpoint: {e}"))?;
        checkpoint.validate()?;
        let host = Self::start(bind, &checkpoint.snapshot.room)?;
        {
            let mut state = host.authority.lock().map_err(|_| "Room unavailable")?;
            state.shared =
                workflow::SharedRoom::restore(checkpoint.host, checkpoint.snapshot.shared)?;
            state.lifecycle = if checkpoint.snapshot.lifecycle == RoomLifecycle::Archived {
                RoomLifecycle::Archived
            } else {
                RoomLifecycle::Paused
            };
            state.sequence = checkpoint.snapshot.cursor;
            state.events = checkpoint.snapshot.events.into();
            state.recovered = checkpoint
                .snapshot
                .members
                .into_iter()
                .map(|member| (member.id.clone(), member))
                .collect();
            state.sync_members();
            let actor = state.shared.state.host_id.clone();
            state.shared.audit(
                &actor,
                "recovered a public checkpoint; new invites required",
            );
        }
        Ok(host)
    }

    /// Only the local host can bind a fresh single-use invite to a retained offline owner identity.
    pub fn reclaim_invite(&self, member: &str, lifetime: Duration) -> Result<Invite> {
        let invite = self.invite(lifetime)?;
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        let Some(member) = state.recovered.get(member).cloned() else {
            state.grants.remove(&invite.token);
            return Err("Only an offline retained member can be reclaimed".into());
        };
        // Invalidate any earlier outstanding claim for the same identity.
        for grant in state.grants.values_mut() {
            if grant
                .reclaim
                .as_ref()
                .is_some_and(|old| old.id == member.id)
            {
                grant.revoked = true;
            }
        }
        state
            .grants
            .get_mut(&invite.token)
            .ok_or("Invite unavailable")?
            .reclaim = Some(member);
        Ok(invite)
    }

    pub fn recovered_members(&self) -> Result<Vec<Member>> {
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_storage()?;
        let now = Instant::now();
        state.retire_sessions(|s| s.expires <= now);
        state.sync_members();
        let mut members: Vec<_> = state.recovered.values().cloned().collect();
        members.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(members)
    }
}

impl Checkpoint {
    fn capture(state: &Authority) -> Self {
        let mut snapshot = state.snapshot(0, None);
        snapshot.reactions.clear();
        snapshot.shared.task_focus.clear();
        snapshot.shared.code_locations.clear();
        snapshot.shared.countdown.running = false;
        for member in &mut snapshot.members {
            member.online = false;
        }
        Checkpoint {
            version: 1,
            host: state.shared.state.host_id.clone(),
            snapshot,
        }
    }

    fn validate(&self) -> Result<()> {
        let snapshot = &self.snapshot;
        let identity = |id: &str| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit());
        workflow::validate(&snapshot.shared)?;
        let mut ids = HashSet::new();
        if self.version != 1
            || !identity(&self.host)
            || bounded(&snapshot.room, 80).is_err()
            || !snapshot.you.is_empty()
            || !snapshot.patches.is_empty()
            || !snapshot.reactions.is_empty()
            || snapshot.members.len() > MAX_MEMBERS
            || snapshot.events.len() > MAX_EVENTS
            || !snapshot.shared.task_focus.is_empty()
            || !snapshot.shared.code_locations.is_empty()
            || snapshot.shared.countdown.running
            || snapshot.cursor > 1_000_000_000_000
            || snapshot.shared.shipping.revision > 1_000_000_000_000
            || snapshot
                .shared
                .readiness
                .profiles
                .iter()
                .any(|profile| profile.revision > 1_000_000_000_000)
            || snapshot
                .shared
                .readiness
                .impacts
                .iter()
                .any(|impact| impact.id > 1_000_000_000_000)
            || snapshot.shared.activity.iter().any(|event| {
                event
                    .id
                    .strip_prefix("network-")
                    .and_then(|id| id.parse::<u64>().ok())
                    .is_none_or(|id| id > 1_000_000_000_000)
            })
            || snapshot.members.iter().any(|m| {
                !identity(&m.id)
                    || m.id == self.host
                    || m.online
                    || bounded(&m.name, 64).is_err()
                    || !ids.insert(&m.id)
            })
            || snapshot.events.iter().any(|e| {
                e.recipient.is_some()
                    || !identity(&e.actor)
                    || bounded(&e.text, 2000).is_err()
                    || e.sequence > snapshot.cursor
                    || e.mentions.len() > MAX_MENTIONS
                    || e.mentions.iter().any(|id| !identity(id))
                    || e.mentions.iter().collect::<HashSet<_>>().len() != e.mentions.len()
                    || e.task.as_ref().is_some_and(|task| task.len() > 16)
                    || e.parent.is_some_and(|parent| parent >= e.sequence)
                    || e.code_reference.as_ref().is_some_and(|reference| {
                        workflow::validate_code_location(&reference.path, reference.line).is_err()
                    })
            })
        {
            return Err("Invalid public recovery checkpoint".into());
        }
        let mut events = HashSet::new();
        let mut tasks = HashSet::new();
        if snapshot.events.iter().any(|e| !events.insert(e.sequence))
            || snapshot
                .events
                .windows(2)
                .any(|pair| pair[0].sequence >= pair[1].sequence)
            || snapshot
                .shared
                .tasks
                .iter()
                .any(|t| t.id.is_empty() || !tasks.insert(&t.id))
            || snapshot
                .shared
                .tasks
                .iter()
                .any(|t| t.dependencies.iter().any(|id| !tasks.contains(id)))
        {
            return Err("Invalid recovery references".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_rejected_by_clients_are_rejected_before_recovery() {
        let path = std::env::temp_dir().join(format!(
            "forge-room-invalid-events-{}.json",
            secret().unwrap()
        ));
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "messages").unwrap();
        let (mut client, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Ada").unwrap();
        client.post("first").unwrap();
        client.post("second").unwrap();
        host.save_checkpoint(&path).unwrap();
        let original: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for case in 0..4 {
            let mut value = original.clone();
            let events = value["snapshot"]["events"].as_array_mut().unwrap();
            match case {
                0 => events.swap(0, 1),
                1 => {
                    events[0]["mentions"] =
                        serde_json::json!([client.identity.clone(), client.identity.clone()])
                }
                2 => events[0]["task"] = "x".repeat(17).into(),
                _ => events[0]["parent"] = events[0]["sequence"].clone(),
            }
            let bytes = serde_json::to_vec(&value).unwrap();
            std::fs::write(&path, &bytes).unwrap();
            assert!(
                Host::recover("127.0.0.1:0".parse().unwrap(), &path).is_err(),
                "case {case}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn archived_checkpoint_stays_archived_and_invalid_data_is_preserved() {
        let path = std::env::temp_dir().join(format!(
            "forge-room-checkpoint-unit-{}.json",
            secret().unwrap()
        ));
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "archived").unwrap();
        host.archive().unwrap();
        host.save_checkpoint(&path).unwrap();
        let restored = Host::recover("127.0.0.1:0".parse().unwrap(), &path).unwrap();
        assert_eq!(restored.lifecycle().unwrap(), RoomLifecycle::Archived);
        assert!(restored.resume().is_err());
        drop(restored);
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        value["version"] = 900.into();
        let invalid = serde_json::to_vec(&value).unwrap();
        std::fs::write(&path, &invalid).unwrap();
        assert!(Host::recover("127.0.0.1:0".parse().unwrap(), &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), invalid);
        value["version"] = 1.into();
        value["snapshot"]["you"] = "injected credentials".into();
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(Host::recover("127.0.0.1:0".parse().unwrap(), &path).is_err());
        std::fs::write(&path, b"{truncated").unwrap();
        assert!(Host::recover("127.0.0.1:0".parse().unwrap(), &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{truncated");
        std::fs::remove_file(path).unwrap();
    }
}
