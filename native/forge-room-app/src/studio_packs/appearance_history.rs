//! Private bounded create-only appearance history. It contains no project paths or data grants.
use crate::local_records;
use forge_room_core::packs::Profile;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Saved {
    pub at: u64,
    pub profile: Profile,
}
fn paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    local_records::directory(root)?;
    let dir = root.join("appearance-history");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    local_records::directory(&dir)?;
    let mut paths = vec![];
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        local_records::file(&path)?;
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or("Invalid appearance record")?;
        if path.extension().is_none_or(|ext| ext != "profile")
            || name.len() != 20
            || !name.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("Unexpected appearance history file; original bytes preserved".into());
        }
        paths.push(path);
        if paths.len() > 32 {
            return Err("Too many appearance history records".into());
        }
    }
    paths.sort();
    Ok(paths)
}
pub fn load(root: &Path) -> Result<Vec<Saved>, String> {
    let mut saved = vec![];
    for path in paths(root)? {
        match local_records::read::<Saved>(&path, 64 * 1024) {
            Ok(record) => {
                record.profile.validate()?;
                saved.push(record);
            }
            Err(error) if error == local_records::INCOMPLETE => (),
            Err(error) => return Err(error),
        }
    }
    Ok(saved)
}
pub fn save(root: &Path, profile: &Profile) -> Result<(), String> {
    profile.validate()?;
    let lockpath = root.join(".appearance-lock");
    if lockpath.exists() {
        local_records::file(&lockpath)?;
    }
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lockpath)
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|e| format!("Appearance history writer unavailable: {e}"))?;
    let paths = paths(root)?;
    let sequence = paths
        .last()
        .and_then(|path| path.file_stem()?.to_str()?.parse::<u64>().ok())
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("Appearance history sequence exhausted")?;
    local_records::write_new(
        &root
            .join("appearance-history")
            .join(format!("{sequence:020}.profile")),
        64 * 1024,
        &Saved {
            at: forge_room_core::now_epoch_seconds(),
            profile: profile.clone(),
        },
    )?;
    for path in paths.iter().take(paths.len().saturating_sub(7)) {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_appearance_keeps_are_bounded_and_reopen_only_as_data() {
        let root = std::env::temp_dir().join(format!(
            "forge-appearance-history-{}",
            forge_room_core::workflows::new_id().unwrap()
        ));
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        for index in 0..10 {
            let profile = Profile {
                name: format!("Choice {index}"),
                ..Profile::default()
            };
            save(&root, &profile).unwrap();
        }
        let saved = load(&root).unwrap();
        assert_eq!(saved.len(), 8);
        assert_eq!(saved.last().unwrap().profile.name, "Choice 9");
        fs::write(
            root.join("appearance-history/00000000000000000011.profile"),
            b"FRC1\ninterrupted",
        )
        .unwrap();
        assert_eq!(load(&root).unwrap().len(), 8);
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("forge-appearance-history-")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
