//! Review and apply bounded LSP WorkspaceEdit text changes inside one trusted folder.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct FileChange {
    pub path: PathBuf,
    pub before: String,
    pub after: String,
    pub lines: Vec<String>,
}

pub struct Preview {
    pub files: Vec<FileChange>,
    pub edit_count: usize,
}

fn path_from_uri(uri: &str) -> Result<PathBuf, String> {
    let path = uri
        .strip_prefix("file://")
        .ok_or("Rename contains a non-file URI")?;
    let mut bytes = Vec::new();
    let mut input = path.bytes();
    while let Some(byte) = input.next() {
        if byte == b'%' {
            let a = (input.next().ok_or("Bad URI escape")? as char)
                .to_digit(16)
                .ok_or("Bad URI escape")?;
            let b = (input.next().ok_or("Bad URI escape")? as char)
                .to_digit(16)
                .ok_or("Bad URI escape")?;
            bytes.push((a * 16 + b) as u8);
        } else {
            bytes.push(byte);
        }
    }
    let decoded = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let decoded = decoded
        .strip_prefix('/')
        .unwrap_or(&decoded)
        .replace('/', "\\");
    Ok(PathBuf::from(decoded))
}

fn byte_at(text: &str, line: usize, character: usize) -> Option<usize> {
    if line > text.bytes().filter(|byte| *byte == b'\n').count() {
        return None;
    }
    let start = text
        .split_inclusive('\n')
        .take(line)
        .map(str::len)
        .sum::<usize>();
    let content = text.get(start..)?.split('\n').next()?;
    let mut units = 0;
    for (byte, ch) in content.char_indices() {
        if units == character {
            return Some(start + byte);
        }
        units += ch.len_utf16();
        if units > character {
            return None;
        }
    }
    (units == character).then_some(start + content.len())
}

pub fn plan(root: &Path, edit: &Value) -> Result<Preview, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut groups: Vec<(PathBuf, Vec<Value>)> = Vec::new();
    if let Some(changes) = edit.get("changes").and_then(Value::as_object) {
        for (uri, edits) in changes {
            let edits = edits.as_array().ok_or("Rename edits are malformed")?;
            groups.push((path_from_uri(uri)?, edits.clone()));
        }
    }
    if let Some(changes) = edit.get("documentChanges").and_then(Value::as_array) {
        for change in changes {
            let uri = change
                .pointer("/textDocument/uri")
                .and_then(Value::as_str)
                .ok_or("Rename includes unsupported file operations")?;
            let edits = change
                .get("edits")
                .and_then(Value::as_array)
                .ok_or("Rename edits are malformed")?;
            groups.push((path_from_uri(uri)?, edits.clone()));
        }
    }
    if groups.is_empty() || groups.len() > 20 {
        return Err("Rename must affect 1 to 20 files".into());
    }
    let mut files = Vec::new();
    let mut edit_count = 0;
    for (path, edits) in groups {
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("Rename target is a link".into());
        }
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if !path.starts_with(&root) {
            return Err("Rename target is outside the trusted folder or is a link".into());
        }
        if files.iter().any(|file: &FileChange| file.path == path) {
            return Err("Rename repeats a file".into());
        }
        if fs::metadata(&path).map_or(true, |m| m.len() > 1024 * 1024) {
            return Err("Rename target exceeds 1 MB".into());
        }
        let before = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if edits.is_empty() || edits.len() > 500 {
            return Err("Rename has too many or no edits".into());
        }
        edit_count += edits.len();
        if edit_count > 500 {
            return Err("Rename exceeds 500 edits".into());
        }
        let mut spans = Vec::new();
        let mut lines = Vec::new();
        for item in edits {
            let range = item.get("range").ok_or("Rename edit has no range")?;
            let start = range.get("start").ok_or("Rename edit has no start")?;
            let end = range.get("end").ok_or("Rename edit has no end")?;
            let pos = |value: &Value| -> Option<usize> {
                byte_at(
                    &before,
                    value.get("line")?.as_u64()? as usize,
                    value.get("character")?.as_u64()? as usize,
                )
            };
            let a = pos(start).ok_or("Rename range is outside the file or splits a character")?;
            let b = pos(end).ok_or("Rename range is outside the file or splits a character")?;
            if a > b {
                return Err("Rename range is reversed".into());
            }
            let replacement = item
                .get("newText")
                .and_then(Value::as_str)
                .ok_or("Rename replacement is missing")?;
            if replacement.len() > 10_000 {
                return Err("Rename replacement is too large".into());
            }
            lines.push(format!(
                "Line {}: {} → {}",
                start.get("line").and_then(Value::as_u64).unwrap_or(0) + 1,
                before[a..b].chars().take(80).collect::<String>(),
                replacement.chars().take(80).collect::<String>()
            ));
            spans.push((a, b, replacement.to_owned()));
        }
        spans.sort_by_key(|span| std::cmp::Reverse(span.0));
        let mut after = before.clone();
        let mut previous = before.len();
        for (a, b, replacement) in spans {
            if b > previous {
                return Err("Rename edits overlap".into());
            }
            after.replace_range(a..b, &replacement);
            previous = a;
        }
        if after.len() > 2 * 1024 * 1024 {
            return Err("Renamed file exceeds 2 MB".into());
        }
        files.push(FileChange {
            path,
            before,
            after,
            lines,
        });
    }
    Ok(Preview { files, edit_count })
}

pub fn apply(preview: &Preview) -> Result<(), String> {
    for file in &preview.files {
        if fs::read_to_string(&file.path).map_err(|e| e.to_string())? != file.before {
            return Err(format!(
                "{} changed since the rename preview",
                file.path.display()
            ));
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let mut staged = Vec::new();
    for (index, file) in preview.files.iter().enumerate() {
        let temp = file
            .path
            .with_extension(format!("forge-rename-{stamp}-{index}.tmp"));
        let stage = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .and_then(|mut output| std::io::Write::write_all(&mut output, file.after.as_bytes()));
        if let Err(error) = stage {
            for path in &staged {
                let _ = fs::remove_file(path);
            }
            return Err(format!("Could not stage rename: {error}"));
        }
        staged.push(temp);
    }
    let mut backups = Vec::new();
    for (index, file) in preview.files.iter().enumerate() {
        let backup = file
            .path
            .with_extension(format!("forge-rename-{stamp}-{index}.bak"));
        let result = (|| {
            if fs::read_to_string(&file.path)? != file.before {
                return Err(std::io::Error::other("file changed during rename"));
            }
            fs::rename(&file.path, &backup)?;
            if let Err(error) = fs::rename(&staged[index], &file.path) {
                let _ = fs::rename(&backup, &file.path);
                return Err(error);
            }
            Ok::<(), std::io::Error>(())
        })();
        if let Err(error) = result {
            let mut restore_failed = false;
            for (path, backup) in backups.iter().rev() {
                if fs::remove_file(path).is_err() || fs::rename(backup, path).is_err() {
                    restore_failed = true;
                }
            }
            for temp in &staged {
                let _ = fs::remove_file(temp);
            }
            return Err(if restore_failed {
                format!("Rename stopped; restore failed, inspect .bak files: {error}")
            } else {
                format!("Rename stopped; prior files were restored: {error}")
            });
        }
        backups.push((file.path.clone(), backup));
    }
    for (_, backup) in backups {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rename_preview_applies_and_rejects_stale_files() {
        let root = std::env::temp_dir().join(format!("forge-rename-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let file = root.join("main.rs");
        let original = "fn old() {}\nfn main() { old(); }\n";
        fs::write(&file, original).unwrap();
        let uri = format!(
            "file:///{}",
            file.to_string_lossy()
                .replace('\\', "/")
                .trim_start_matches('/')
        );
        let edit = json!({"changes": {uri: [
            {"range":{"start":{"line":0,"character":3},"end":{"line":0,"character":6}},"newText":"new"},
            {"range":{"start":{"line":1,"character":12},"end":{"line":1,"character":15}},"newText":"new"}
        ]}});
        let preview = plan(&root, &edit).unwrap();
        assert_eq!(preview.edit_count, 2);
        fs::write(&file, "external change").unwrap();
        assert!(apply(&preview).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "external change");
        fs::write(&file, original).unwrap();
        apply(&preview).unwrap();
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "fn new() {}\nfn main() { new(); }\n"
        );
        let _ = fs::remove_dir_all(root);
    }
}
