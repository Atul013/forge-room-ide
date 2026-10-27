use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum DocumentError {
    NoPath,
    Io(io::Error),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPath => write!(formatter, "enter a file path first"),
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

#[derive(Default)]
pub struct DocumentSession {
    path: Option<PathBuf>,
    text: String,
    saved_text: String,
}

impl DocumentSession {
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved_text
    }

    pub fn open(&mut self, path: impl Into<PathBuf>) -> Result<(), DocumentError> {
        let path = path.into();
        let text = fs::read_to_string(&path)?;
        self.path = Some(path);
        self.saved_text.clone_from(&text);
        self.text = text;
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), DocumentError> {
        let path = self.path.as_ref().ok_or(DocumentError::NoPath)?;
        fs::write(path, self.text.as_bytes())?;
        self.saved_text.clone_from(&self.text);
        Ok(())
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
