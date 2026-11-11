//! The window's last state: the folder and the files that were open, so starting the app again
//! (without being given something to open) puts you back where you were. Unsaved text is not part
//! of it: that is what crash recovery is for. Files and folders that are gone are dropped.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// More than this many files are not restored (a runaway session must not slow the start).
const MAX_FILES: usize = 40;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Session {
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
    /// Index into `files` of the tab that was in front.
    #[serde(default)]
    pub active: Option<usize>,
    /// How many terminals were open. Their programs cannot come back, but the same number of fresh
    /// shells does, in the same folder.
    #[serde(default)]
    pub terminals: usize,
    /// Which panels were open and how big they were.
    #[serde(default)]
    pub layout: Option<Layout>,
}

/// The shape of the window: panels shown, the bottom tab, and panel sizes in points.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Layout {
    pub sidebar: bool,
    pub right: bool,
    pub terminal: bool,
    pub preview: bool,
    /// 0 terminal, 1 output, 2 problems.
    pub bottom_tab: u8,
    pub sidebar_width: u16,
    pub right_width: u16,
    pub terminal_height: u16,
}

/// At most this many terminals are reopened.
pub const MAX_TERMINALS: usize = 6;

impl Session {
    /// The session without folders and files that no longer exist, and with `active` pointing at
    /// the same file (or at nothing when that file is gone).
    pub fn clean(self) -> Session {
        let folder = self.folder.filter(|folder| Path::new(folder).is_dir());
        let active_file = self.active.and_then(|index| self.files.get(index).cloned());
        let mut files: Vec<String> = Vec::new();
        for file in self.files {
            if files.len() < MAX_FILES && Path::new(&file).is_file() && !files.contains(&file) {
                files.push(file);
            }
        }
        let active = active_file.and_then(|file| files.iter().position(|known| *known == file));
        Session {
            folder,
            files,
            active,
            terminals: self.terminals.min(MAX_TERMINALS),
            layout: self.layout.map(|layout| Layout {
                // A size outside what the panels allow would be clamped anyway; keep it sane.
                sidebar_width: layout.sidebar_width.clamp(180, 560),
                right_width: layout.right_width.clamp(280, 640),
                terminal_height: layout.terminal_height.clamp(120, 720),
                ..layout
            }),
        }
    }
}

pub fn default_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Forge Room Early Preview").join("session.json")
}

/// A missing or unreadable file is an empty session.
pub fn load_from(path: &Path) -> Session {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_to(path: &Path, session: &Session) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Beside the target and renamed over it, so a crash never leaves half a file.
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_string_pretty(session).map_err(io::Error::other)?,
    )?;
    fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_round_trips_and_forgets_what_is_gone() {
        let dir = std::env::temp_dir().join(format!("forge-session-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("a.txt"), dir.join("b.txt"));
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        let (a, b) = (a.display().to_string(), b.display().to_string());
        let gone = dir.join("gone.txt").display().to_string();
        let session = Session {
            folder: Some(dir.display().to_string()),
            files: vec![gone.clone(), a.clone(), b.clone(), a.clone()],
            active: Some(2),
            terminals: 99,
            layout: Some(Layout {
                sidebar: false,
                right: true,
                terminal: true,
                preview: false,
                bottom_tab: 2,
                sidebar_width: 10,
                right_width: 364,
                terminal_height: 9000,
            }),
        };
        let file = dir.join("session.json");
        save_to(&file, &session).unwrap();
        let loaded = load_from(&file);
        assert_eq!(loaded, session);
        // The missing file and the repeat are dropped, and the active tab is still b.
        let cleaned = loaded.clean();
        assert_eq!(cleaned.files, vec![a.clone(), b.clone()]);
        assert_eq!(cleaned.active, Some(1));
        assert_eq!(cleaned.terminals, MAX_TERMINALS);
        let layout = cleaned.layout.expect("the layout is kept");
        assert_eq!((layout.sidebar_width, layout.terminal_height), (180, 720));
        assert!(!layout.sidebar && layout.right && layout.bottom_tab == 2);
        // An active file that is gone leaves nothing active; a gone folder is dropped.
        let cleaned = Session {
            folder: Some(dir.join("nope").display().to_string()),
            files: vec![gone, a],
            active: Some(0),
            terminals: 0,
            layout: None,
        }
        .clean();
        assert_eq!((cleaned.folder, cleaned.active), (None, None));
        // Missing, broken or odd files mean an empty session.
        assert_eq!(load_from(&dir.join("missing.json")), Session::default());
        fs::write(&file, "{ not json").unwrap();
        assert_eq!(load_from(&file), Session::default());
        fs::write(&file, "{}").unwrap();
        assert_eq!(load_from(&file), Session::default());
        fs::remove_dir_all(dir).unwrap();
    }
}
