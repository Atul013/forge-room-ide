//! Per-project room state: where it is stored, how identities and task IDs are made, and how
//! times are shown. The rules themselves (ownership, dependencies, handoffs) live in
//! `forge-room-core`; nothing here re-implements them.

use forge_room_core::{Participant, Role, RoomState, StateStore};
use std::path::{Path, PathBuf};

/// Local-only by default: rooms live beside the app's settings, not inside the project, so
/// opening a folder never writes into it.
pub fn store_for(workspace: &Path) -> StateStore {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    StateStore::new(
        base.join("Forge Room Early Preview")
            .join("rooms")
            .join(format!("{:016x}.json", fnv1a(&workspace.to_string_lossy()))),
    )
}

/// Where the lobby settings for `workspace` are saved, beside its room file.
pub fn lobby_path_for(workspace: &Path) -> PathBuf {
    store_for(workspace).path().with_extension("lobby.json")
}

fn fnv1a(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text.to_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// `Ada Lovelace` becomes `ada-lovelace`; never empty.
pub fn slug(name: &str) -> String {
    let mut result = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
        } else if !result.ends_with('-') && !result.is_empty() {
            result.push('-');
        }
    }
    let trimmed = result.trim_matches('-');
    if trimmed.is_empty() {
        "member".to_string()
    } else {
        trimmed.to_string()
    }
}

/// A participant ID not already used in the room.
pub fn unique_participant_id(state: &RoomState, name: &str) -> String {
    let base = slug(name);
    let mut candidate = base.clone();
    let mut counter = 2;
    while state.participants.iter().any(|item| item.id == candidate) {
        candidate = format!("{base}-{counter}");
        counter += 1;
    }
    candidate
}

/// `T1`, `T2`, ... one past the highest existing number.
pub fn next_task_id(state: &RoomState) -> String {
    let highest = state
        .tasks
        .iter()
        .filter_map(|task| task.id.strip_prefix('T')?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    format!("T{}", highest + 1)
}

pub fn new_room(workspace_name: &str, user_name: &str) -> RoomState {
    RoomState::new(
        slug(workspace_name),
        Participant {
            id: slug(user_name),
            name: user_name.to_string(),
            role: Role::Human,
            owner_id: None,
        },
    )
}

pub fn now() -> u64 {
    forge_room_core::now_epoch_seconds()
}

pub fn local_user_name() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "You".to_string())
}

/// "just now", "5m ago", "3h ago", "2d ago".
pub fn ago(now: u64, then: u64) -> String {
    let seconds = now.saturating_sub(then);
    match seconds {
        0..=44 => "just now".to_string(),
        45..=3_599 => format!("{}m ago", (seconds + 30) / 60),
        3_600..=86_399 => format!("{}h ago", seconds / 3_600),
        _ => format!("{}d ago", seconds / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_room_core::{Task, TaskStatus};

    fn task(id: &str) -> Task {
        Task {
            id: id.into(),
            title: "t".into(),
            owner_id: "ada".into(),
            owner_name: "Ada".into(),
            status: TaskStatus::Open,
            dependencies: Vec::new(),
            acceptance_criteria: Vec::new(),
            completed_criteria: Vec::new(),
            worktree_branch: None,
            updated_at_epoch_seconds: 0,
        }
    }

    #[test]
    fn slugs_are_stable_and_never_empty() {
        assert_eq!(slug("Ada Lovelace"), "ada-lovelace");
        assert_eq!(slug("  --Forge  Room!! "), "forge-room");
        assert_eq!(slug("???"), "member");
    }

    #[test]
    fn participant_ids_stay_unique() {
        let mut room = new_room("demo", "Ada");
        assert_eq!(unique_participant_id(&room, "Grace"), "grace");
        assert_eq!(unique_participant_id(&room, "Ada"), "ada-2");
        room.participants.push(Participant {
            id: "ada-2".into(),
            name: "Ada".into(),
            role: Role::Human,
            owner_id: None,
        });
        assert_eq!(unique_participant_id(&room, "Ada"), "ada-3");
    }

    #[test]
    fn task_ids_continue_from_the_highest_number() {
        let mut room = new_room("demo", "Ada");
        assert_eq!(next_task_id(&room), "T1");
        room.tasks.push(task("T1"));
        room.tasks.push(task("T7"));
        room.tasks.push(task("custom"));
        assert_eq!(next_task_id(&room), "T8");
    }

    #[test]
    fn room_files_are_per_workspace_and_case_insensitive() {
        let a = store_for(Path::new(r"C:\Work\App"));
        let b = store_for(Path::new(r"c:\work\app"));
        let c = store_for(Path::new(r"C:\Work\Other"));
        assert_eq!(a.path(), b.path());
        assert_ne!(a.path(), c.path());
    }

    #[test]
    fn relative_times_read_naturally() {
        assert_eq!(ago(1000, 990), "just now");
        assert_eq!(ago(1000, 700), "5m ago");
        assert_eq!(ago(10_000, 0), "2h ago");
        assert_eq!(ago(300_000, 0), "3d ago");
        assert_eq!(ago(5, 100), "just now");
    }
}
