use crate::document::{DocumentError, DocumentSession};
use std::path::{Path, PathBuf};

pub struct Tab {
    pub id: u64,
    pub doc: DocumentSession,
    untitled: Option<u32>,
    /// True until the editor has placed the cursor at the start of the document.
    pub fresh: bool,
    /// Read-only generated view (such as a diff) instead of a file.
    pub readonly: bool,
    label: Option<String>,
    /// Syntax name used for highlighting when there is no file extension.
    pub syntax: Option<String>,
}

impl Tab {
    pub fn title(&self) -> String {
        if let Some(label) = &self.label {
            label.clone()
        } else if let Some(name) = self.doc.file_name() {
            name
        } else {
            format!("Untitled-{}", self.untitled.unwrap_or(1))
        }
    }
}

/// Ordered open documents plus the active one.
#[derive(Default)]
pub struct Tabs {
    pub tabs: Vec<Tab>,
    pub active: usize,
    next_id: u64,
    next_untitled: u32,
}

/// Canonical form used to recognise the same file opened twice. Strips the
/// `\\?\` prefix Windows adds so paths stay readable.
pub fn normalize(path: &Path) -> PathBuf {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => canonical,
    }
}

impl Tabs {
    fn take_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn active_tab(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active)
    }

    pub fn active_path(&self) -> Option<&Path> {
        self.tabs.get(self.active).and_then(|tab| tab.doc.path())
    }

    /// Point the tabs of a renamed file (or of files inside a renamed folder) at their new place.
    pub fn path_moved(&mut self, from: &Path, to: &Path) {
        let (from, to) = (normalize(from), normalize(to));
        for tab in &mut self.tabs {
            tab.doc.moved(&from, &to);
        }
    }

    pub fn any_dirty(&self) -> bool {
        self.tabs.iter().any(|tab| tab.doc.is_dirty())
    }

    pub fn new_untitled(&mut self) {
        self.next_untitled += 1;
        let tab = Tab {
            id: self.take_id(),
            doc: DocumentSession::default(),
            untitled: Some(self.next_untitled),
            fresh: true,
            readonly: false,
            label: None,
            syntax: None,
        };
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    /// Open `path`, or just activate its tab when it is already open.
    pub fn open_file(&mut self, path: &Path) -> Result<(), DocumentError> {
        let path = normalize(path);
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.doc.path() == Some(path.as_path()))
        {
            self.active = index;
            return Ok(());
        }
        let mut doc = DocumentSession::default();
        doc.open(path)?;
        let tab = Tab {
            id: self.take_id(),
            doc,
            untitled: None,
            fresh: true,
            readonly: false,
            label: None,
            syntax: None,
        };
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        Ok(())
    }

    /// Show generated read-only text in a tab, reusing the tab with the same label.
    pub fn open_virtual(&mut self, label: String, text: String, syntax: &str) {
        let doc = DocumentSession::with_text(text);
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.label.as_deref() == Some(label.as_str()))
        {
            self.tabs[index].doc = doc;
            self.tabs[index].fresh = true;
            self.active = index;
            return;
        }
        let tab = Tab {
            id: self.take_id(),
            doc,
            untitled: None,
            fresh: true,
            readonly: true,
            label: Some(label),
            syntax: Some(syntax.to_string()),
        };
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    pub fn close(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active = 0;
        } else if self.active > index || self.active >= self.tabs.len() {
            self.active = self.active.saturating_sub(1).min(self.tabs.len() - 1);
        }
    }

    pub fn next(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "forge-room-tabs-{}-{stamp}-{name}",
            std::process::id()
        ));
        fs::write(&path, "hello").unwrap();
        path
    }

    #[test]
    fn opening_the_same_file_twice_reuses_the_tab() {
        let path = temp_file("a.txt");
        let mut tabs = Tabs::default();
        tabs.open_file(&path).unwrap();
        tabs.open_file(&path).unwrap();
        assert_eq!(tabs.tabs.len(), 1);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn closing_keeps_a_valid_active_tab() {
        let first = temp_file("1.txt");
        let second = temp_file("2.txt");
        let mut tabs = Tabs::default();
        tabs.open_file(&first).unwrap();
        tabs.open_file(&second).unwrap();
        assert_eq!(tabs.active, 1);

        tabs.close(0);
        assert_eq!(tabs.tabs.len(), 1);
        assert_eq!(tabs.active, 0);
        tabs.close(0);
        assert!(tabs.tabs.is_empty());
        assert_eq!(tabs.active, 0);
        fs::remove_file(first).unwrap();
        fs::remove_file(second).unwrap();
    }

    #[test]
    fn untitled_tabs_are_numbered_and_dirty_tracking_works() {
        let mut tabs = Tabs::default();
        tabs.new_untitled();
        tabs.new_untitled();
        assert_eq!(tabs.tabs[0].title(), "Untitled-1");
        assert_eq!(tabs.tabs[1].title(), "Untitled-2");
        assert!(!tabs.any_dirty());
        tabs.active_tab().unwrap().doc.text_mut().push('x');
        assert!(tabs.any_dirty());
    }
}
