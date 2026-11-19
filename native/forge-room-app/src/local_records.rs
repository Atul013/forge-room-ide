//! Shared bounded create-only envelopes. FRC1 is also the existing workflow history byte format.
use forge_room_core::workflows;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
pub const INCOMPLETE: &str = "Incomplete local record";
const MAX: u64 = 8 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    digest: String,
    payload: serde_json::Value,
}
pub fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}
pub fn file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if linked(&metadata) || !metadata.is_file() {
        return Err("Linked/non-regular local record is unsupported".into());
    }
    Ok(())
}
pub fn directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if linked(&metadata) || !metadata.is_dir() {
        return Err("Linked/non-directory local record storage is unsupported".into());
    }
    Ok(())
}
pub fn write_new(path: &Path, max: u64, value: &impl Serialize) -> Result<(), String> {
    if max > MAX {
        return Err("Unsupported record bound".into());
    }
    let payload = serde_json::to_value(value).map_err(|e| e.to_string())?;
    let envelope = Envelope {
        version: 1,
        digest: workflows::seal(&payload)?,
        payload,
    };
    let bytes = serde_json::to_vec(&envelope).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("Local record exceeds its byte limit".into());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(b"FRC1\n")
        .and_then(|_| file.write_all(&bytes))
        .and_then(|_| file.write_all(b"\nDONE\n"))
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("Local record was not durably saved: {e}"))
}
pub fn read<T: DeserializeOwned>(path: &Path, max: u64) -> Result<T, String> {
    if max > MAX {
        return Err("Unsupported record bound".into());
    }
    file(path)?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(max + 12)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max + 11 {
        return Err("Oversized local record".into());
    }
    if !bytes.starts_with(b"FRC1\n") || !bytes.ends_with(b"\nDONE\n") {
        return Err(INCOMPLETE.into());
    }
    let envelope: Envelope = serde_json::from_slice(&bytes[5..bytes.len() - 6])
        .map_err(|e| format!("Corrupt complete local record: {e}"))?;
    if envelope.version != 1 || workflows::seal(&envelope.payload)? != envelope.digest {
        return Err("Local record checksum/version mismatch".into());
    }
    serde_json::from_value(envelope.payload).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_workflow_frame_format_roundtrips_and_partial_or_complete_corruption_is_distinct() {
        let root =
            std::env::temp_dir().join(format!("forge-records-{}", workflows::new_id().unwrap()));
        fs::create_dir(&root).unwrap();
        let path = root.join("sample.record");
        let value = serde_json::json!({"a":1});
        write_new(&path, 1024, &value).unwrap();
        let expected=b"FRC1\n{\"version\":1,\"digest\":\"015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862\",\"payload\":{\"a\":1}}\nDONE\n";
        assert_eq!(fs::read(&path).unwrap(), expected);
        assert_eq!(read::<serde_json::Value>(&path, 1024).unwrap(), value);
        assert!(write_new(&path, 1024, &value).is_err());
        fs::write(&path, b"FRC1\n{torn").unwrap();
        assert_eq!(
            read::<serde_json::Value>(&path, 1024).unwrap_err(),
            INCOMPLETE
        );
        fs::write(&path, b"FRC1\n{}\nDONE\n").unwrap();
        assert_ne!(
            read::<serde_json::Value>(&path, 1024).unwrap_err(),
            INCOMPLETE
        );
        fs::remove_file(&path).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
