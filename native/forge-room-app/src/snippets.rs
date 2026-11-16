//! Bounded, declarative VS Code snippet import. No extension code is executed.
use std::{fs, path::PathBuf};

#[derive(Clone)]
pub struct Snippet {
    pub name: String,
    pub prefix: String,
    pub body: String,
}

fn directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Forge Room Early Preview")
        .join("snippets")
}

fn parse(text: &str) -> Result<Vec<Snippet>, String> {
    if text.len() > 128 * 1024 {
        return Err("Snippet file exceeds 128 KB".into());
    }
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let object = value
        .as_object()
        .ok_or("Snippet file must be a JSON object")?;
    let mut result = Vec::new();
    for (name, entry) in object.iter().take(100) {
        let Some(body) = entry.get("body") else {
            continue;
        };
        let body = if let Some(s) = body.as_str() {
            s.to_owned()
        } else if let Some(lines) = body.as_array() {
            lines
                .iter()
                .map(|v| v.as_str().unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            continue;
        };
        if body.len() > 8192 || body.contains('$') || name.len() > 100 {
            continue;
        }
        let prefix = entry.get("prefix").and_then(|v| v.as_str()).unwrap_or(name);
        if prefix.len() > 100 {
            continue;
        }
        result.push(Snippet {
            name: name.clone(),
            prefix: prefix.into(),
            body,
        });
    }
    if result.is_empty() {
        return Err("No plain-text snippets found (placeholders are unsupported)".into());
    }
    Ok(result)
}

pub fn import(text: &str) -> Result<usize, String> {
    let snippets = parse(text)?;
    let dir = directory();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let hash = text.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    fs::write(dir.join(format!("{hash:016x}.json")), text).map_err(|e| e.to_string())?;
    Ok(snippets.len())
}

pub fn installed() -> Vec<Snippet> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(directory()) {
        for entry in entries.flatten().take(40) {
            if entry.path().extension().is_none_or(|ext| ext != "json")
                || entry.metadata().map_or(true, |m| m.len() > 128 * 1024)
            {
                continue;
            }
            if let Ok(text) = fs::read_to_string(entry.path())
                && let Ok(items) = parse(&text)
            {
                out.extend(items);
            }
        }
    }
    out
}
