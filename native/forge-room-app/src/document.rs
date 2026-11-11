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
                write!(formatter, "not a UTF-8 or UTF-16 text file")
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

/// How a file's bytes map to text. The buffer is always plain `\n` text; the encoding and line
/// ending the file came with are remembered and restored on save, so editing a CRLF or UTF-16 file
/// does not silently convert it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Encoding {
    #[default]
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

impl Encoding {
    pub fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf8Bom => "UTF-8 with BOM",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
    /// CR and LF used inconsistently (or a stray CR): the text is kept exactly as it is on disk.
    Mixed,
}

impl LineEnding {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::CrLf => "CRLF",
            Self::Mixed => "Mixed",
        }
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

/// Text, encoding and dominant line ending of a file's bytes. Fails with `InvalidData` for bytes
/// that are not valid UTF-8 or well-formed UTF-16 (other legacy encodings are not guessed).
fn decode(bytes: &[u8]) -> io::Result<(String, Encoding, LineEnding)> {
    let (raw, encoding) = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        (
            String::from_utf8(rest.to_vec()).map_err(|_| invalid("not UTF-8"))?,
            Encoding::Utf8Bom,
        )
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        (utf16(rest, u16::from_le_bytes)?, Encoding::Utf16Le)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        (utf16(rest, u16::from_be_bytes)?, Encoding::Utf16Be)
    } else {
        (
            String::from_utf8(bytes.to_vec()).map_err(|_| invalid("not UTF-8"))?,
            Encoding::Utf8,
        )
    };
    let (crlf, lf, cr) = (
        raw.matches("\r\n").count(),
        raw.matches('\n').count(),
        raw.matches('\r').count(),
    );
    // Only files that are purely one style are normalised. Anything else (mixed breaks, a stray
    // CR, `\r\r\n`) is kept byte for byte, so saving can never alter a line the user did not edit.
    Ok(if cr == 0 {
        (raw, encoding, LineEnding::Lf)
    } else if cr == crlf && lf == crlf {
        (raw.replace("\r\n", "\n"), encoding, LineEnding::CrLf)
    } else {
        (raw, encoding, LineEnding::Mixed)
    })
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> io::Result<String> {
    if !bytes.len().is_multiple_of(2) {
        return Err(invalid("truncated UTF-16"));
    }
    let (pairs, _) = bytes.as_chunks::<2>();
    let units: Vec<u16> = pairs.iter().map(|c| unit(*c)).collect();
    String::from_utf16(&units).map_err(|_| invalid("invalid UTF-16"))
}

/// The bytes to write for `text` in the file's own encoding and line ending.
fn encode(text: &str, encoding: Encoding, ending: LineEnding) -> Vec<u8> {
    let text = match ending {
        LineEnding::Lf | LineEnding::Mixed => text.to_string(),
        LineEnding::CrLf => text.replace("\r\n", "\n").replace('\n', "\r\n"),
    };
    match encoding {
        Encoding::Utf8 => text.into_bytes(),
        Encoding::Utf8Bom => {
            let mut out = vec![0xEF, 0xBB, 0xBF];
            out.extend_from_slice(text.as_bytes());
            out
        }
        Encoding::Utf16Le => {
            let mut out = vec![0xFF, 0xFE];
            out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
            out
        }
        Encoding::Utf16Be => {
            let mut out = vec![0xFE, 0xFF];
            out.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
            out
        }
    }
}

/// Replace `path` with `bytes` without ever leaving a half-written file. The data goes to a
/// sibling temporary file, is flushed to disk, and is then renamed over the original, so a crash or
/// power loss mid-save keeps either the old content or the new content. Anything that stops the
/// atomic route (a symlink, a read-only file, a folder where the temporary file cannot be made, a
/// file another program holds open) falls back to writing the file directly, as before; that also
/// reports the right error when it fails too. The replacement keeps the hidden and system marks
/// but not alternate data streams or custom permissions, and hard links are not followed.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let existing = fs::symlink_metadata(path).ok();
    if existing
        .as_ref()
        .is_some_and(|m| m.file_type().is_symlink() || m.permissions().readonly())
    {
        return fs::write(path, bytes);
    }
    // A short name that does not depend on the file's own (possibly very long) name, unique per
    // call and per running app.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temporary = path.with_file_name(format!(
        ".forge-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let remove_temporary = || {
        if let Ok(metadata) = fs::metadata(&temporary) {
            let mut permissions = metadata.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = fs::set_permissions(&temporary, permissions);
        }
        let _ = fs::remove_file(&temporary);
    };
    let written = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(windows)]
        if let Some(existing) = &existing {
            use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
            const HIDDEN_AND_SYSTEM: u32 = 0x2 | 0x4;
            options.attributes(existing.file_attributes() & HIDDEN_AND_SYSTEM);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    if written.is_err() {
        remove_temporary();
        return fs::write(path, bytes);
    }
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            remove_temporary();
            fs::write(path, bytes)
        }
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
    encoding: Encoding,
    line_ending: LineEnding,
}

impl DocumentSession {
    /// A document that is not backed by a file (for example a diff view); never dirty.
    pub fn with_text(text: String) -> Self {
        Self {
            path: None,
            saved_text: text.clone(),
            text,
            disk_stamp: None,
            encoding: Encoding::Utf8,
            line_ending: LineEnding::Lf,
        }
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// The file's current text as this session would read it, or why it cannot be read.
    fn read_disk(path: &Path) -> io::Result<(String, Encoding)> {
        decode(&fs::read(path)?).map(|(text, encoding, _)| (text, encoding))
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Follow a rename: when this document's file is `from` (or inside the folder `from`), it now
    /// lives under `to`. The text and the unsaved changes are untouched.
    pub fn moved(&mut self, from: &Path, to: &Path) {
        let Some(current) = &self.path else { return };
        if let Ok(rest) = current.strip_prefix(from) {
            // The file itself leaves an empty remainder: joining it would add a trailing separator.
            let new = if rest.as_os_str().is_empty() {
                to.to_path_buf()
            } else {
                to.join(rest)
            };
            // A rename keeps the modification time and size, so the fingerprint stays valid and an
            // outside change made just before it is still noticed.
            self.path = Some(new);
        }
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
        // Fingerprint first: a change landing between the stamp and the read is then noticed
        // later, instead of being hidden by a stamp that already includes it.
        let fingerprint = stamp(&path);
        let (text, encoding, line_ending) = decode(&fs::read(&path)?)?;
        self.encoding = encoding;
        self.line_ending = line_ending;
        self.disk_stamp = fingerprint;
        self.path = Some(path);
        self.saved_text.clone_from(&text);
        self.text = text;
        Ok(())
    }

    /// Quick check used for polling: true when the file's size or time changed and its content
    /// differs from what this session last saw. A touched-but-identical file is not a change, and a
    /// deleted file is not reported (saving simply recreates it). Because it trusts an unchanged
    /// size and time, it is only an optimisation: saving uses [`Self::content_differs`].
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
        match Self::read_disk(path) {
            Ok((text, encoding)) if text == self.saved_text && encoding == self.encoding => {
                self.disk_stamp = Some(current);
                false
            }
            _ => true,
        }
    }

    /// Reads the file and compares it with what this session last saw. Unlike the stamp check this
    /// cannot be fooled by a tool that rewrites a file and restores its time and size.
    fn content_differs(&self) -> Result<bool, DocumentError> {
        let Some(path) = &self.path else {
            return Ok(false);
        };
        match Self::read_disk(path) {
            Ok((text, encoding)) => Ok(text != self.saved_text || encoding != self.encoding),
            // Gone: saving recreates it. Not UTF-8 is a change. Any other failure (for example a
            // file another program holds open without read sharing) means the current bytes
            // cannot be inspected, which is no evidence that they are unchanged.
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => Ok(true),
            Err(error) => Err(DocumentError::Io(error)),
        }
    }

    /// Save, refusing to overwrite content that changed on disk since it was opened or saved.
    pub fn save(&mut self) -> Result<(), DocumentError> {
        if self.content_differs()? {
            return Err(DocumentError::ChangedOnDisk);
        }
        self.overwrite()
    }

    /// Save without checking for outside changes (the user chose to overwrite them).
    pub fn overwrite(&mut self) -> Result<(), DocumentError> {
        let path = self.path.as_ref().ok_or(DocumentError::NoPath)?;
        write_atomic(path, &encode(&self.text, self.encoding, self.line_ending))?;
        self.disk_stamp = stamp(path);
        self.saved_text.clone_from(&self.text);
        Ok(())
    }

    /// Discard the buffer and take the file's current content from disk.
    pub fn reload(&mut self) -> Result<(), DocumentError> {
        let path = self.path.clone().ok_or(DocumentError::NoPath)?;
        self.open(path)
    }

    /// Write to a new path and follow it. The caller has already confirmed replacing an existing
    /// file (the save dialog asks), so there is no outside-change check, and the old path is kept
    /// if the write fails.
    pub fn save_as(&mut self, path: impl Into<PathBuf>) -> Result<(), DocumentError> {
        let path = path.into();
        write_atomic(&path, &encode(&self.text, self.encoding, self.line_ending))?;
        self.disk_stamp = stamp(&path);
        self.path = Some(path);
        self.saved_text.clone_from(&self.text);
        Ok(())
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
    fn crlf_files_stay_crlf_and_the_buffer_is_plain_lf() {
        let path = test_path();
        fs::write(&path, "one\r\ntwo\r\n").unwrap();
        let mut document = DocumentSession::default();
        document.open(&path).unwrap();
        assert_eq!(document.text(), "one\ntwo\n");
        assert_eq!(document.line_ending(), LineEnding::CrLf);
        assert!(!document.is_dirty());

        // Typing Enter in the editor inserts a bare `\n`; the file must not end up mixed.
        document.text_mut().push_str("three\n");
        document.save().unwrap();
        assert_eq!(
            fs::read(&path).unwrap(),
            b"one\r\ntwo\r\nthree\r\n".to_vec()
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn only_purely_lf_or_purely_crlf_files_are_normalised() {
        assert_eq!(decode(b"a\nb\n").unwrap().2, LineEnding::Lf);
        assert_eq!(decode(b"a\r\nb\r\n").unwrap().2, LineEnding::CrLf);
        assert_eq!(decode(b"no newline").unwrap().2, LineEnding::Lf);
        assert_eq!(decode(b"").unwrap().0, "");
        // Mixed, lone CR and `\r\r\n` are kept exactly as written.
        for raw in [
            &b"a\nb\r\nc\n"[..],
            b"a\r\nb\nc\r\n",
            b"x\r\r\n",
            b"old\rmac\r",
        ] {
            let (text, _, ending) = decode(raw).unwrap();
            assert_eq!(ending, LineEnding::Mixed, "{raw:?}");
            assert_eq!(text.as_bytes(), raw);
            assert_eq!(encode(&text, Encoding::Utf8, ending), raw, "{raw:?}");
        }
    }

    #[test]
    fn mixed_files_save_back_byte_for_byte() {
        let path = test_path();
        let raw = b"a\nb\r\nc\r\r\nd".to_vec();
        fs::write(&path, &raw).unwrap();
        let mut document = DocumentSession::default();
        document.open(&path).unwrap();
        assert_eq!(document.line_ending(), LineEnding::Mixed);
        document.text_mut().push('!');
        document.save().unwrap();
        let mut expected = raw;
        expected.push(b'!');
        assert_eq!(fs::read(&path).unwrap(), expected);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_rewrite_in_another_encoding_counts_as_an_outside_change() {
        let path = test_path();
        fs::write(&path, "same\n").unwrap();
        let mut document = DocumentSession::default();
        document.open(&path).unwrap();
        let mut utf16 = vec![0xFF, 0xFE];
        utf16.extend("same\n".encode_utf16().flat_map(u16::to_le_bytes));
        fs::write(&path, utf16).unwrap();
        assert!(document.changed_on_disk());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn bom_and_utf16_files_round_trip_in_their_own_encoding() {
        for (bytes, encoding) in [
            (b"\xEF\xBB\xBFhello\n".to_vec(), Encoding::Utf8Bom),
            (
                [
                    &[0xFF, 0xFE][..],
                    &"hi\r\n\u{e9}\u{1f600}"
                        .encode_utf16()
                        .flat_map(u16::to_le_bytes)
                        .collect::<Vec<_>>(),
                ]
                .concat(),
                Encoding::Utf16Le,
            ),
            (
                [
                    &[0xFE, 0xFF][..],
                    &"hi\n"
                        .encode_utf16()
                        .flat_map(u16::to_be_bytes)
                        .collect::<Vec<_>>(),
                ]
                .concat(),
                Encoding::Utf16Be,
            ),
        ] {
            let path = test_path();
            fs::write(&path, &bytes).unwrap();
            let mut document = DocumentSession::default();
            document.open(&path).unwrap();
            assert_eq!(document.encoding(), encoding);
            assert!(
                !document.text().starts_with('\u{feff}'),
                "the BOM is not text"
            );
            // Saving unchanged text reproduces the original bytes exactly.
            document.text_mut().push('x');
            document.text_mut().pop();
            document.overwrite().unwrap();
            assert_eq!(fs::read(&path).unwrap(), bytes, "{encoding:?}");
            fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn invalid_bytes_and_truncated_utf16_are_refused() {
        assert!(decode(&[0xC3, 0x28]).is_err());
        assert!(decode(&[0xFF, 0xFE, 0x41]).is_err());
        // An unpaired surrogate is not valid UTF-16.
        assert!(decode(&[0xFF, 0xFE, 0x00, 0xD8]).is_err());
    }

    #[test]
    fn outside_edits_are_noticed_through_the_encoding() {
        let path = test_path();
        fs::write(&path, "a\r\nb\r\n").unwrap();
        let mut document = DocumentSession::default();
        document.open(&path).unwrap();
        // The same text with different line endings is not an outside change.
        fs::write(&path, "a\nb\n").unwrap();
        assert!(!document.changed_on_disk());
        // A different size, so the check does not depend on the clock's resolution.
        fs::write(&path, "a\nBB\n").unwrap();
        assert!(document.changed_on_disk());
        fs::remove_file(path).unwrap();
    }

    /// A path inside a fresh private folder, so leftovers can be checked without other tests'
    /// temporary files getting in the way.
    fn private_path() -> PathBuf {
        let dir = test_path().with_extension("dir");
        fs::create_dir_all(&dir).unwrap();
        dir.join("file.txt")
    }

    fn leftovers(path: &Path) -> Vec<String> {
        fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|file| file.starts_with(".forge-") && file.ends_with(".tmp"))
            .collect()
    }

    #[test]
    fn saving_replaces_the_file_and_leaves_no_temporary_behind() {
        let path = private_path();
        fs::write(&path, "old\n").unwrap();
        write_atomic(&path, b"new content\n").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new content\n");
        assert!(leftovers(&path).is_empty());
        // A new file is created the same way.
        fs::remove_file(&path).unwrap();
        write_atomic(&path, b"fresh").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"fresh");
        assert!(leftovers(&path).is_empty());
        fs::remove_file(path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn the_atomic_route_really_replaces_the_file() {
        let path = test_path();
        fs::write(
            &path, "old
",
        )
        .unwrap();
        let before = fs::metadata(&path).unwrap().created().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        write_atomic(
            &path, b"new
",
        )
        .unwrap();
        // A rename brings in the temporary file with its own creation time; a direct write
        // would keep the original.
        assert_ne!(fs::metadata(&path).unwrap().created().unwrap(), before);
        fs::remove_file(path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn a_hidden_file_stays_hidden_after_a_save() {
        use std::os::windows::fs::MetadataExt;
        let path = test_path();
        fs::write(&path, "x").unwrap();
        let attribute = std::process::Command::new("attrib")
            .arg("+H")
            .arg(&path)
            .status()
            .unwrap();
        assert!(attribute.success());
        write_atomic(&path, b"y").unwrap();
        assert_ne!(fs::metadata(&path).unwrap().file_attributes() & 0x2, 0);
        let _ = std::process::Command::new("attrib")
            .arg("-H")
            .arg(&path)
            .status();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_read_only_file_is_not_replaced_and_keeps_its_content() {
        let path = private_path();
        fs::write(&path, "keep me\n").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();

        assert!(write_atomic(&path, b"overwritten").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"keep me\n");
        assert!(leftovers(&path).is_empty());

        let mut permissions = fs::metadata(&path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&path, permissions).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn a_file_another_program_holds_open_is_written_in_place() {
        use std::os::windows::fs::OpenOptionsExt;
        let path = private_path();
        fs::write(&path, "old\n").unwrap();
        // Readers and writers allowed, but not rename or delete: the atomic rename must fail.
        let _holder = fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .unwrap();
        write_atomic(&path, b"updated\n").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"updated\n");
        assert!(leftovers(&path).is_empty());
        drop(_holder);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_renamed_file_keeps_its_buffer_and_saves_to_the_new_path() {
        let dir = test_path().with_extension("moved");
        fs::create_dir_all(dir.join("sub")).unwrap();
        let old = dir.join("sub").join("a.txt");
        fs::write(&old, "one\n").unwrap();
        let mut document = DocumentSession::default();
        document.open(&old).unwrap();
        document.text_mut().push_str("two\n");

        // Renaming the folder moves the file with it.
        fs::rename(dir.join("sub"), dir.join("renamed")).unwrap();
        document.moved(&dir.join("sub"), &dir.join("renamed"));
        assert_eq!(
            document.path(),
            Some(dir.join("renamed").join("a.txt").as_path())
        );
        assert!(document.is_dirty(), "unsaved changes survive the rename");
        document.save().unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("renamed").join("a.txt")).unwrap(),
            "one\ntwo\n"
        );
        // The file itself (not inside a folder) moves to exactly the new path, with no separator
        // appended.
        let exact_old = dir.join("renamed").join("a.txt");
        let exact_new = dir.join("renamed").join("b.txt");
        fs::rename(&exact_old, &exact_new).unwrap();
        document.moved(&exact_old, &exact_new);
        assert_eq!(document.path(), Some(exact_new.as_path()));
        assert!(
            !document.changed_on_disk(),
            "the moved file is the same content"
        );
        document.text_mut().push_str("three\n");
        document.save().unwrap();
        // A path elsewhere is not touched.
        let before = document.path().map(Path::to_path_buf);
        document.moved(&dir.join("other"), &dir.join("x"));
        assert_eq!(document.path().map(Path::to_path_buf), before);
        fs::remove_dir_all(dir).unwrap();
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

    #[cfg(windows)]
    #[test]
    fn save_refuses_when_the_file_cannot_be_read_back() {
        use std::os::windows::fs::OpenOptionsExt;
        let path = std::env::temp_dir().join(format!("forge-share-{}.txt", std::process::id()));
        fs::write(
            &path, "old
",
        )
        .unwrap();
        let mut doc = DocumentSession::default();
        doc.open(&path).unwrap();
        doc.text_mut().clear();
        doc.text_mut().push_str("mine\n");
        // Another program holds the file for writing without allowing reads.
        let mut holder = fs::OpenOptions::new()
            .write(true)
            .share_mode(2)
            .open(&path)
            .unwrap();
        io::Write::write_all(
            &mut holder,
            b"new
",
        )
        .unwrap();
        assert!(
            doc.save().is_err(),
            "an unreadable file must not be overwritten"
        );
        drop(holder);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "new
"
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn save_notices_outside_changes_that_keep_the_same_size_and_time() {
        let path = test_path();
        fs::write(&path, "old\n").unwrap();
        let original_time = fs::metadata(&path).unwrap().modified().unwrap();
        let mut document = DocumentSession::default();
        document.open(&path).unwrap();
        document.text_mut().clear();
        document.text_mut().push_str("mine\n");

        // Another program rewrites the bytes and puts the old modification time back.
        fs::write(&path, "new\n").unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(original_time))
            .unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), 4);

        assert!(matches!(document.save(), Err(DocumentError::ChangedOnDisk)));
        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn save_as_onto_an_existing_file_replaces_it_and_follows_it() {
        let first = test_path();
        let second = test_path().with_extension("other.txt");
        fs::write(
            &first,
            "original
",
        )
        .unwrap();
        fs::write(
            &second,
            "something else entirely
",
        )
        .unwrap();

        let mut document = DocumentSession::default();
        document.open(&first).unwrap();
        document.text_mut().push_str(
            "edit
",
        );
        // The native dialog already asked the user to confirm replacing `second`.
        document.save_as(&second).unwrap();

        assert_eq!(document.path(), Some(second.as_path()));
        assert_eq!(
            fs::read_to_string(&second).unwrap(),
            "original
edit
"
        );
        assert_eq!(
            fs::read_to_string(&first).unwrap(),
            "original
"
        );
        assert!(!document.is_dirty());
        // The tab now tracks the new file, so an outside change to it is noticed.
        fs::write(
            &second,
            "changed outside, longer
",
        )
        .unwrap();
        assert!(document.changed_on_disk());
        fs::remove_file(first).unwrap();
        fs::remove_file(second).unwrap();
    }

    #[test]
    fn a_failed_save_as_keeps_the_old_path() {
        let first = test_path();
        fs::write(
            &first, "keep
",
        )
        .unwrap();
        let mut document = DocumentSession::default();
        document.open(&first).unwrap();
        let missing_dir = std::env::temp_dir()
            .join("forge-room-no-such-dir")
            .join("x.txt");
        assert!(document.save_as(&missing_dir).is_err());
        assert_eq!(document.path(), Some(first.as_path()));
        fs::remove_file(first).unwrap();
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
