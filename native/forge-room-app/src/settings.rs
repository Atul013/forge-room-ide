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
    /// Size of the editor text relative to the default (1.0). Missing or odd values mean 1.0.
    #[serde(default)]
    pub editor_scale: Option<f32>,
    /// Save file tabs automatically once their text stops changing.
    #[serde(default)]
    pub auto_save: Option<bool>,
    /// Folders the user trusts: Git and project tools only run in these (or below them).
    #[serde(default)]
    pub trusted_folders: Vec<String>,
    /// The agent panel's choices: provider ("claude" or "codex"), model, effort and mode ("read" or
    /// "edit"; full access is never remembered).
    #[serde(default)]
    pub ai_provider: Option<String>,
    #[serde(default)]
    pub ai_model: Option<String>,
    #[serde(default)]
    pub ai_effort: Option<String>,
    #[serde(default)]
    pub ai_mode: Option<String>,
}

fn trust_key(path: &str) -> String {
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

/// Whether `folder` is one of the trusted folders or inside one (compared the way Windows paths
/// are: without regard to case or the direction of slashes).
pub fn is_trusted(trusted: &[String], folder: &str) -> bool {
    let folder = trust_key(folder);
    trusted.iter().any(|known| {
        let known = trust_key(known);
        !known.is_empty() && (folder == known || folder.starts_with(&format!("{known}\\")))
    })
}

/// Add `folder` to the trusted ones (once).
pub fn trust(trusted: &mut Vec<String>, folder: &str) {
    if !is_trusted(trusted, folder)
        || !trusted
            .iter()
            .any(|known| trust_key(known) == trust_key(folder))
    {
        trusted.retain(|known| trust_key(known) != trust_key(folder));
        trusted.push(folder.to_string());
    }
}

/// Smallest and largest editor text scale.
pub const MIN_SCALE: f32 = 0.6;
pub const MAX_SCALE: f32 = 2.5;

/// A scale from a settings file: anything that is not a sane number falls back to 1.0.
pub fn clean_scale(value: Option<f32>) -> f32 {
    match value {
        Some(scale) if scale.is_finite() => scale.clamp(MIN_SCALE, MAX_SCALE),
        _ => 1.0,
    }
}

/// The scale one step (`delta`, in tenths of the default size) from `current`, rounded to a tenth
/// and kept inside the allowed range.
pub fn step_scale(current: f32, delta: i32) -> f32 {
    let tenths = (current * 10.0).round() as i32 + delta;
    (tenths as f32 / 10.0).clamp(MIN_SCALE, MAX_SCALE)
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
    fn the_editor_scale_steps_by_tenths_inside_its_limits() {
        assert_eq!(step_scale(1.0, 1), 1.1);
        assert_eq!(step_scale(1.0, -1), 0.9);
        assert_eq!(step_scale(1.0, 3), 1.3);
        // Rounding: repeated float steps never drift.
        let mut scale = 1.0;
        for _ in 0..7 {
            scale = step_scale(scale, 1);
        }
        assert_eq!(scale, 1.7);
        // The ends hold.
        assert_eq!(step_scale(MAX_SCALE, 1), MAX_SCALE);
        assert_eq!(step_scale(MIN_SCALE, -1), MIN_SCALE);
        // A settings file may hold anything.
        assert_eq!(clean_scale(None), 1.0);
        assert_eq!(clean_scale(Some(f32::NAN)), 1.0);
        assert_eq!(clean_scale(Some(f32::INFINITY)), 1.0);
        assert_eq!(clean_scale(Some(99.0)), MAX_SCALE);
        assert_eq!(clean_scale(Some(-3.0)), MIN_SCALE);
        assert_eq!(clean_scale(Some(1.4)), 1.4);
        let parsed: Settings = serde_json::from_str(r#"{"editor_scale":1.5}"#).unwrap();
        assert_eq!(parsed.editor_scale, Some(1.5));
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

    #[test]
    fn a_trusted_folder_covers_what_is_inside_it_and_nothing_next_to_it() {
        let mut trusted = Vec::new();
        assert!(!is_trusted(&trusted, r"C:\work\app"));
        trust(&mut trusted, r"C:\Work\App");
        trust(&mut trusted, r"c:/work/app/");
        assert_eq!(trusted.len(), 1);
        assert!(is_trusted(&trusted, r"C:\work\app"));
        assert!(is_trusted(&trusted, r"c:\WORK\app\sub\deeper"));
        assert!(!is_trusted(&trusted, r"C:\work\application"));
        assert!(!is_trusted(&trusted, r"C:\work"));
        assert!(!is_trusted(&[String::new()], r"C:\work"));
    }
}
