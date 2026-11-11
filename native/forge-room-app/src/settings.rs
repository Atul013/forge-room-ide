//! User settings, stored as JSON under `%LOCALAPPDATA%\Forge Room Early Preview`.
//! The installer and uninstaller leave this folder alone, so choices survive updates.

use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

#[derive(Default, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: Option<String>,
    /// Folders opened most recently, newest first.
    #[serde(default)]
    pub recent_folders: Vec<String>,
}

/// How many recent folders are kept.
pub const MAX_RECENT_FOLDERS: usize = 10;

/// Put `folder` first in `recent`, dropping an earlier entry for the same folder (compared
/// without regard to case or the direction of slashes, as Windows paths are) and keeping at most
/// [`MAX_RECENT_FOLDERS`].
pub fn remember_folder(recent: &mut Vec<String>, folder: &str) {
    let key = |path: &str| {
        path.replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    let wanted = key(folder);
    if wanted.is_empty() {
        return;
    }
    recent.retain(|known| key(known) != wanted);
    recent.insert(0, folder.to_string());
    recent.truncate(MAX_RECENT_FOLDERS);
}

/// Load the settings, change them and save them again, so one setting never overwrites another.
pub fn update(change: impl FnOnce(&mut Settings)) -> io::Result<()> {
    let mut settings = load();
    change(&mut settings);
    save(&settings)
}

fn path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Forge Room Early Preview").join("settings.json")
}

/// Missing or unreadable settings simply mean defaults.
pub fn load() -> Settings {
    fs::read_to_string(path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) -> io::Result<()> {
    let file = path();
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    // Write beside the target and rename, so a crash never leaves half a file.
    let temporary = file.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_string_pretty(settings).map_err(io::Error::other)?,
    )?;
    fs::rename(temporary, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_tolerates_unknown_or_missing_fields() {
        let parsed: Settings =
            serde_json::from_str(r#"{"theme":"moss-shrine","future":1}"#).unwrap();
        assert_eq!(parsed.theme.as_deref(), Some("moss-shrine"));
        let empty: Settings = serde_json::from_str("{}").unwrap();
        assert!(empty.theme.is_none());
        let text = serde_json::to_string(&Settings {
            theme: Some("ink-ember".into()),
            ..Default::default()
        })
        .unwrap();
        assert!(text.contains("ink-ember"));
        // Settings written before recent folders existed still load.
        assert!(empty.recent_folders.is_empty());
    }

    #[test]
    fn recent_folders_are_newest_first_without_duplicates_and_capped() {
        let mut recent = Vec::new();
        remember_folder(&mut recent, r"C:\a");
        remember_folder(&mut recent, r"C:\b");
        remember_folder(&mut recent, "c:/A/");
        assert_eq!(recent, vec!["c:/A/".to_string(), r"C:\b".to_string()]);
        remember_folder(&mut recent, "");
        assert_eq!(recent.len(), 2);
        for index in 0..30 {
            remember_folder(&mut recent, &format!(r"C:\folder{index}"));
        }
        assert_eq!(recent.len(), MAX_RECENT_FOLDERS);
        assert_eq!(recent[0], r"C:\folder29");
    }
}
