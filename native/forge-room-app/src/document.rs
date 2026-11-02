use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug)]
pub enum DocumentError {
    NoPath,
    /// The file on disk no longer matches what was opened or last saved.
    ChangedOnDisk,
    TooLarge(u64),
    Io(io::Error),
}

/// Files above this size are refused until the editor can page large files.
pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPath => write!(formatter, "this file has no path yet"),
            Self::ChangedOnDisk => write!(formatter, "the file changed on disk"),
            Self::TooLarge(bytes) => write!(
                formatter,
                "file is too large for this preview ({:.1} MB, limit {} MB)",
                *bytes as f64 / 1_048_576.0,
                MAX_FILE_BYTES / 1_048_576
            ),
            Self::Io(error) if error.kind() == io::ErrorKind::InvalidData => {
                write!(formatter, "not a UTF-8 text file")
            }
            Self::Io(error) => write!(formatter, "file operation failed: {error}"),
        }
    }
}

impl std::error::Error for DocumentError {}

impl From<io::Error> for DocumentError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Cheap fingerprint of the file as last read or written by this session.
type DiskStamp = (SystemTime, u64);

fn stamp(path: &Path) -> Option<DiskStamp> {
    let metadata = fs::metadata(path).ok()?;
    Some((metadata.modified().ok()?, metadata.len()))
}

#[derive(Default)]
pub struct DocumentSession {
    path: Option<PathBuf>,
    text: String,
    saved_text: String,
    disk_stamp: Option<DiskStamp>,
}

impl DocumentSession {
    /// A document that is not backed by a file (for example a diff view); never dirty.
    pub fn with_text(text: String) -> Self {
        Self {
            path: None,
            saved_text: text.clone(),
            text,
            disk_stamp: None,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn file_name(&self) -> Option<String> {
        self.path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved_text
    }

    pub fn open(&mut self, path: impl Into<PathBuf>) -> Result<(), DocumentError> {
        let path = path.into();
        let size = fs::metadata(&path)?.len();
        if size > MAX_FILE_BYTES {
            return Err(DocumentError::TooLarge(size));
        }
        let text = fs::read_to_string(&path)?;
        self.disk_stamp = stamp(&path);
        self.path = Some(path);
        self.saved_text.clone_from(&text);
        self.text = text;
        Ok(())
    }

    /// True when the file on disk now holds different content than this session last saw.
    /// A touched-but-identical file is not a change. A deleted file is not reported: saving
    /// simply recreates it.
    pub fn changed_on_disk(&mut self) -> bool {
        let Some(path) = &self.path else {
            return false;
        };
        let Some(current) = stamp(path) else {
            return false;
        };
        if Some(current) == self.disk_stamp {
            return false;
        }
        match fs::read_to_string(path) {
            Ok(text) if text == self.saved_text => {
                self.disk_stamp = Some(current);
                false
            }
            _ => true,
        }
    }

    /// Save, refusing to overwrite content that changed on disk since it was opened or saved.
    pub fn save(&mut self) -> Result<(), DocumentError> {
        if self.changed_on_disk() {
            return Err(DocumentError::ChangedOnDisk);
        }
        self.overwrite()
    }

    /// Save without checking for outside changes (the user chose to overwrite them).
    pub fn overwrite(&mut self) -> Result<(), DocumentError> {
        let path = self.path.as_ref().ok_or(DocumentError::NoPath)?;
        fs::write(path, self.text.as_bytes())?;
        self.disk_stamp = stamp(path);
        self.saved_text.clone_from(&self.text);
        Ok(())
    }

    /// Discard the buffer and take the file's current content from disk.
    pub fn reload(&mut self) -> Result<(), DocumentError> {
        let path = self.path.clone().ok_or(DocumentError::NoPath)?;
        self.open(path)
    }

    pub fn save_as(&mut self, path: impl Into<PathBuf>) -> Result<(), DocumentError> {
        self.path = Some(path.into());
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "forge-room-document-{}-{stamp}.txt",
            std::process::id()
        ))
    }

    #[test]
    fn open_edit_save_roundtrip() {
        let path = test_path();
        fs::write(&path, "first\n").unwrap();

        let mut document = DocumentSession::default();
        document.open(&path).unwrap();
        assert_eq!(document.text, "first\n");
        assert!(!document.is_dirty());

        document.text_mut().push_str("second\n");
        assert!(document.is_dirty());
        document.save().unwrap();
        assert!(!document.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "first\nsecond\n");

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn save_refuses_to_overwrite_outside_changes() {
        let path = test_path();
        fs::write(
            &path, "mine
",
        )
        .unwrap();
        let mut document = DocumentSession::default();
        document.open(&path).unwrap();
        document.text_mut().push_str(
            "edit
",
        );

        fs::write(
            &path,
            "theirs, longer
",
        )
        .unwrap();
        assert!(matches!(document.save(), Err(DocumentError::ChangedOnDisk)));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "theirs, longer
"
        );

        document.overwrite().unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "mine
edit
"
        );
        document.save().unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn reload_takes_the_disk_content_and_clears_the_conflict() {
        let path = test_path();
        fs::write(
            &path, "one
",
        )
        .unwrap();
        let mut document = DocumentSession::default();
        document.open(&path).unwrap();
        document.text_mut().push_str(
            "unsaved
",
        );

        fs::write(
            &path, "two!!
",
        )
        .unwrap();
        assert!(document.changed_on_disk());
        document.reload().unwrap();
        assert_eq!(
            document.text(),
            "two!!
"
        );
        assert!(!document.is_dirty());
        assert!(!document.changed_on_disk());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn identical_rewrite_and_deletion_are_not_conflicts() {
        let path = test_path();
        fs::write(
            &path, "same
",
        )
        .unwrap();
        let mut document = DocumentSession::default();
        document.open(&path).unwrap();
        document.text_mut().push_str(
            "edit
",
        );

        fs::write(
            &path, "same
",
        )
        .unwrap();
        assert!(!document.changed_on_disk());
        fs::remove_file(&path).unwrap();
        assert!(!document.changed_on_disk());
        document.save().unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "same
edit
"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn save_as_sets_the_document_path() {
        let path = test_path();
        let mut document = DocumentSession::default();
        document.text_mut().push_str("preview");
        document.save_as(&path).unwrap();

        assert_eq!(document.path(), Some(path.as_path()));
        assert_eq!(fs::read_to_string(&path).unwrap(), "preview");
        fs::remove_file(path).unwrap();
    }
}
