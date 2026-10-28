use crate::RoomState;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    InvalidState(serde_json::Error),
    PrimaryAndBackupInvalid { primary: String, backup: String },
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "state I/O failed: {error}"),
            Self::InvalidState(error) => write!(formatter, "state is invalid JSON: {error}"),
            Self::PrimaryAndBackupInvalid { primary, backup } => write!(
                formatter,
                "primary state is invalid ({primary}); backup recovery also failed ({backup})"
            ),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<io::Error> for StoreError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidState(value)
    }
}

#[derive(Debug)]
pub struct LoadOutcome {
    pub state: RoomState,
    pub recovered_from_backup: bool,
}

#[derive(Clone, Debug)]
pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn backup_path(&self) -> PathBuf {
        self.path.with_extension("bak")
    }

    pub fn load(&self) -> Result<LoadOutcome, StoreError> {
        match read_state(&self.path) {
            Ok(state) => Ok(LoadOutcome {
                state,
                recovered_from_backup: false,
            }),
            Err(primary_error) => {
                let backup = self.backup_path();
                if !backup.exists() {
                    return Err(primary_error);
                }
                match read_state(&backup) {
                    Ok(state) => Ok(LoadOutcome {
                        state,
                        recovered_from_backup: true,
                    }),
                    Err(backup_error) => Err(StoreError::PrimaryAndBackupInvalid {
                        primary: primary_error.to_string(),
                        backup: backup_error.to_string(),
                    }),
                }
            }
        }
    }

    pub fn save(&self, state: &RoomState) -> Result<(), StoreError> {
        if let Some(parent) = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let temporary = self.path.with_extension("tmp");
        let backup = self.backup_path();
        if temporary.exists() {
            fs::remove_file(&temporary)?;
        }
        let bytes = serde_json::to_vec_pretty(state)?;
        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;

        let had_primary = self.path.exists();
        if had_primary {
            if backup.exists() {
                fs::remove_file(&backup)?;
            }
            fs::rename(&self.path, &backup)?;
        }
        if let Err(error) = fs::rename(&temporary, &self.path) {
            if had_primary && backup.exists() {
                let _ = fs::rename(&backup, &self.path);
            }
            return Err(StoreError::Io(error));
        }
        Ok(())
    }
}

fn read_state(path: &Path) -> Result<RoomState, StoreError> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Participant, Role, RoomState};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("forge-room-{name}-{}-{stamp}", std::process::id()))
            .join("state.json")
    }

    fn state(room_id: &str) -> RoomState {
        RoomState::new(
            room_id,
            Participant {
                id: "host".into(),
                name: "Host".into(),
                role: Role::Human,
                owner_id: None,
            },
        )
    }

    #[test]
    fn saves_and_loads_state() {
        let path = test_path("roundtrip");
        let store = StateStore::new(&path);
        store.save(&state("room-one")).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.state.room_id, "room-one");
        assert!(!loaded.recovered_from_backup);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn recovers_previous_snapshot_when_primary_is_corrupt() {
        let path = test_path("recovery");
        let store = StateStore::new(&path);
        store.save(&state("first")).unwrap();
        store.save(&state("second")).unwrap();
        fs::write(&path, b"not-json").unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.recovered_from_backup);
        assert_eq!(loaded.state.room_id, "first");
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
