//! Native language protocol helpers and an on-demand navigation fallback.
//! `live` owns persistent trusted-workspace sessions. Language servers have ambient OS rights;
//! refusing server edit requests does not sandbox the server process.
use crate::diagnostics::{Problem, Report};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_secs(45);
const MAX_MESSAGE: usize = 4 * 1024 * 1024;

pub mod live;

fn language(path: &Path) -> Option<(&'static str, &'static str, &'static [&'static str])> {
    match path.extension()?.to_str()? {
        "rs" => Some(("rust", "rust-analyzer", &[])),
        "py" => Some(("python", "pylsp", &[])),
        "ts" | "tsx" | "js" | "jsx" => {
            Some(("typescript", "typescript-language-server", &["--stdio"]))
        }
        _ => None,
    }
}

fn uri(path: &Path) -> String {
    let path_text = path.to_string_lossy();
    let path_text = path_text.strip_prefix("\\\\?\\").unwrap_or(&path_text);
    let mut normalized = path_text.replace('\\', "/");
    if normalized.as_bytes().get(1) == Some(&b':') {
        let drive = normalized[0..1].to_ascii_lowercase();
        normalized.replace_range(0..1, &drive);
    }
    let normalized = if normalized.starts_with('/') {
        normalized
    } else {
        format!("/{normalized}")
    };
    let mut out = String::from("file://");
    for byte in normalized.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn write_message(writer: &mut impl Write, message: &Value) -> std::io::Result<()> {
    let payload = serde_json::to_vec(message)?;
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
    writer.write_all(&payload)?;
    writer.flush()
}

fn read_message(reader: &mut impl BufRead) -> std::io::Result<Value> {
    let mut length = None;
    let mut headers = 0usize;
    loop {
        let mut bytes = Vec::new();
        if reader
            .take((8193usize.saturating_sub(headers)) as u64)
            .read_until(b'\n', &mut bytes)?
            == 0
        {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        headers += bytes.len();
        if headers > 8192 || !bytes.ends_with(b"\n") {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        let line = std::str::from_utf8(&bytes).map_err(std::io::Error::other)?;
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("Content-Length")
        {
            if length.is_some() {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .filter(|size| *size <= MAX_MESSAGE)
                    .ok_or(std::io::ErrorKind::InvalidData)?,
            );
        }
    }
    let size = length.ok_or(std::io::ErrorKind::InvalidData)?;
    let mut payload = vec![0; size];
    reader.read_exact(&mut payload)?;
    serde_json::from_slice(&payload).map_err(std::io::Error::other)
}

fn parse_diagnostics(value: &Value, file: &Path) -> Vec<Problem> {
    value
        .get("params")
        .and_then(|p| p.get("diagnostics"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(500)
        .filter_map(|item| {
            let start = item.get("range")?.get("start")?;
            let message = item.get("message")?.as_str()?.to_owned();
            Some(Problem {
                error: item.get("severity").and_then(Value::as_u64).unwrap_or(1) == 1,
                message,
                file: file.to_path_buf(),
                line: start.get("line")?.as_u64()? as usize + 1,
                column: start.get("character")?.as_u64()? as usize + 1,
            })
        })
        .collect()
}

#[cfg(test)]
pub fn check(root: &Path, file: &Path, text: &str) -> Report {
    let mut report = Report {
        source: "language server",
        ..Report::default()
    };
    let Some((language_id, program, args)) = language(file) else {
        report.failure = Some("No configured language server for this file type".into());
        return report;
    };
    let file = match file.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            report.failure = Some(e.to_string());
            return report;
        }
    };
    let root = match root.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            report.failure = Some(e.to_string());
            return report;
        }
    };
    if !file.starts_with(&root) {
        report.failure = Some("File is outside the trusted folder".into());
        return report;
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            report.failure = Some(format!("{program} is unavailable: {e}"));
            return report;
        }
    };
    let mut stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes)
            .chars()
            .take(1000)
            .collect::<String>()
    });
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        while let Ok(message) = read_message(&mut reader) {
            if sender.send(message).is_err() {
                break;
            }
        }
    });
    let result = (|| -> Result<Vec<Problem>, String> {
        write_message(&mut stdin, &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "processId":std::process::id(),"rootUri":uri(&root),"capabilities":{},"workspaceFolders":null
        }})).map_err(|e| e.to_string())?;
        let deadline = Instant::now() + LIMIT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!("{program} did not initialize within 45 seconds"));
            }
            let message = receiver
                .recv_timeout(remaining)
                .map_err(|_| format!("{program} stopped or timed out during initialization"))?;
            if message.get("id") == Some(&json!(1)) {
                if message.get("error").is_some() {
                    return Err(format!("{program} rejected initialization"));
                }
                break;
            }
            answer_server_request(&mut stdin, &message);
        }
        write_message(
            &mut stdin,
            &json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
        )
        .map_err(|e| e.to_string())?;
        let file_uri = uri(&file);
        write_message(
            &mut stdin,
            &json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
                "textDocument":{"uri":file_uri,"languageId":language_id,"version":1,"text":text}
            }}),
        )
        .map_err(|e| e.to_string())?;
        let mut empty_since: Option<Instant> = None;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return if empty_since.is_some() {
                    Ok(Vec::new())
                } else {
                    Err(format!("{program} sent no diagnostics within 45 seconds"))
                };
            }
            let wait = empty_since.map_or(remaining, |since| {
                remaining.min(Duration::from_secs(10).saturating_sub(since.elapsed()))
            });
            if wait.is_zero() {
                return Ok(Vec::new());
            }
            let message = match receiver.recv_timeout(wait) {
                Ok(message) => message,
                Err(_) if empty_since.is_some() => return Ok(Vec::new()),
                Err(_) => return Err(format!("{program} stopped or timed out while checking")),
            };
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
                && message
                    .get("params")
                    .and_then(|p| p.get("uri"))
                    .and_then(Value::as_str)
                    == Some(file_uri.as_str())
            {
                let problems = parse_diagnostics(&message, &file);
                if !problems.is_empty() {
                    return Ok(problems);
                }
                empty_since.get_or_insert_with(Instant::now);
            }
            answer_server_request(&mut stdin, &message);
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    let server_log = stderr_reader.join().unwrap_or_default();
    match result {
        Ok(problems) => report.problems = problems,
        Err(error) => {
            report.failure = Some(if server_log.is_empty() {
                error
            } else {
                format!("{error}: {server_log}")
            })
        }
    }
    report
}

fn answer_server_request(writer: &mut impl Write, message: &Value) {
    let (Some(id), Some(method)) = (
        message.get("id"),
        message.get("method").and_then(Value::as_str),
    ) else {
        return;
    };
    let result = match method {
        "workspace/configuration" => {
            let count = message
                .get("params")
                .and_then(|p| p.get("items"))
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            json!(vec![Value::Null; count])
        }
        "workspace/workspaceFolders" => json!([]),
        _ => Value::Null,
    };
    let _ = write_message(writer, &json!({"jsonrpc":"2.0","id":id,"result":result}));
}

#[derive(Clone, Copy)]
pub enum QueryKind {
    Hover,
    Definition,
    Completion,
    References,
    Rename,
}

pub struct QueryAnswer {
    pub kind: QueryKind,
    pub message: String,
    pub location: Option<(std::path::PathBuf, usize, usize)>,
    pub completions: Vec<(String, String)>,
    pub locations: Vec<(std::path::PathBuf, usize, usize)>,
    pub rename_edit: Option<Value>,
}

fn query_result(kind: QueryKind, value: &Value, root: &Path) -> QueryAnswer {
    match kind {
        QueryKind::Hover => {
            let contents = &value["result"]["contents"];
            let message = if let Some(text) = contents.as_str() {
                text.to_owned()
            } else if let Some(text) = contents.get("value").and_then(Value::as_str) {
                text.to_owned()
            } else {
                "No hover information at cursor".into()
            };
            QueryAnswer {
                kind,
                message: message.chars().take(4000).collect(),
                location: None,
                completions: Vec::new(),
                locations: Vec::new(),
                rename_edit: None,
            }
        }
        QueryKind::Definition => {
            let item = value
                .get("result")
                .and_then(|v| v.as_array())
                .and_then(|items| items.first())
                .or_else(|| value.get("result"));
            let location = item.and_then(|v| {
                let target = v.get("targetUri").or_else(|| v.get("uri"))?.as_str()?;
                let path = path_from_uri(target)?;
                if !path.canonicalize().ok()?.starts_with(root) {
                    return None;
                }
                let start = v
                    .get("targetSelectionRange")
                    .or_else(|| v.get("range"))?
                    .get("start")?;
                Some((
                    path,
                    start.get("line")?.as_u64()? as usize + 1,
                    start.get("character")?.as_u64()? as usize + 1,
                ))
            });
            QueryAnswer {
                kind,
                message: if location.is_some() {
                    "Definition found".into()
                } else {
                    "No in-workspace definition at cursor".into()
                },
                location,
                completions: Vec::new(),
                locations: Vec::new(),
                rename_edit: None,
            }
        }
        QueryKind::Completion => {
            let result = &value["result"];
            let items = result
                .as_array()
                .or_else(|| result.get("items").and_then(Value::as_array));
            let completions = items
                .into_iter()
                .flatten()
                .take(100)
                .filter(|item| item.get("insertTextFormat").and_then(Value::as_u64) != Some(2))
                .filter_map(|item| {
                    let label = item.get("label")?.as_str()?.to_owned();
                    let insert = item
                        .get("insertText")
                        .and_then(Value::as_str)
                        .unwrap_or(&label)
                        .to_owned();
                    if label.len() > 120 || insert.len() > 1000 || insert.contains('$') {
                        return None;
                    }
                    Some((label, insert))
                })
                .collect::<Vec<_>>();
            QueryAnswer {
                kind,
                message: format!("{} completion(s)", completions.len()),
                location: None,
                completions,
                locations: Vec::new(),
                rename_edit: None,
            }
        }
        QueryKind::References => {
            let locations = value
                .get("result")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .take(500)
                .filter_map(|item| {
                    let path = path_from_uri(item.get("uri")?.as_str()?)?;
                    if !path.canonicalize().ok()?.starts_with(root) {
                        return None;
                    }
                    let start = item.get("range")?.get("start")?;
                    Some((
                        path,
                        start.get("line")?.as_u64()? as usize + 1,
                        start.get("character")?.as_u64()? as usize + 1,
                    ))
                })
                .collect::<Vec<_>>();
            QueryAnswer {
                kind,
                message: format!("{} in-workspace reference(s)", locations.len()),
                location: None,
                completions: Vec::new(),
                locations,
                rename_edit: None,
            }
        }
        QueryKind::Rename => QueryAnswer {
            kind,
            message: if value.get("result").is_some_and(Value::is_object) {
                "Review the proposed rename".into()
            } else {
                "Language server proposed no rename".into()
            },
            location: None,
            completions: Vec::new(),
            locations: Vec::new(),
            rename_edit: value
                .get("result")
                .filter(|result| result.is_object())
                .cloned(),
        },
    }
}

fn path_from_uri(uri: &str) -> Option<std::path::PathBuf> {
    let path = uri.strip_prefix("file://")?;
    let mut bytes = Vec::new();
    let mut input = path.as_bytes().iter().copied();
    while let Some(byte) = input.next() {
        if byte == b'%' {
            let a = (input.next()? as char).to_digit(16)?;
            let b = (input.next()? as char).to_digit(16)?;
            bytes.push((a * 16 + b) as u8);
        } else {
            bytes.push(byte);
        }
    }
    let decoded = String::from_utf8(bytes).ok()?;
    #[cfg(windows)]
    let decoded = decoded
        .strip_prefix('/')
        .unwrap_or(&decoded)
        .replace('/', "\\");
    Some(std::path::PathBuf::from(decoded))
}

pub fn query(
    root: &Path,
    file: &Path,
    text: &str,
    line: usize,
    column: usize,
    kind: QueryKind,
) -> QueryAnswer {
    query_with_name(root, file, text, line, column, kind, None)
}

pub fn query_with_name(
    root: &Path,
    file: &Path,
    text: &str,
    line: usize,
    column: usize,
    kind: QueryKind,
    new_name: Option<&str>,
) -> QueryAnswer {
    let failed = |message: String| QueryAnswer {
        kind,
        message,
        location: None,
        completions: Vec::new(),
        locations: Vec::new(),
        rename_edit: None,
    };
    let Some((language_id, program, args)) = language(file) else {
        return failed("No configured language server for this file type".into());
    };
    let (Ok(root), Ok(file)) = (root.canonicalize(), file.canonicalize()) else {
        return failed("Folder or file is unavailable".into());
    };
    if !file.starts_with(&root) {
        return failed("File is outside the trusted folder".into());
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return failed(format!("{program} is unavailable: {error}")),
    };
    let mut stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        while let Ok(message) = read_message(&mut reader) {
            if sender.send(message).is_err() {
                break;
            }
        }
    });
    let result = (|| -> Result<QueryAnswer, String> {
        write_message(&mut stdin, &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "processId":std::process::id(),"rootUri":uri(&root),"capabilities":{},"workspaceFolders":null
        }})).map_err(|e| e.to_string())?;
        let deadline = Instant::now() + LIMIT;
        loop {
            let message = receiver
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| format!("{program} did not initialize"))?;
            if message.get("id") == Some(&json!(1)) {
                if message.get("error").is_some() {
                    return Err(format!("{program} rejected initialization"));
                }
                break;
            }
            answer_server_request(&mut stdin, &message);
        }
        write_message(
            &mut stdin,
            &json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
        )
        .map_err(|e| e.to_string())?;
        write_message(
            &mut stdin,
            &json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
                "textDocument":{"uri":uri(&file),"languageId":language_id,"version":1,"text":text}
            }}),
        )
        .map_err(|e| e.to_string())?;
        let ready_deadline = Instant::now() + Duration::from_secs(10);
        let file_uri = uri(&file);
        loop {
            let remaining = ready_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let Ok(message) = receiver.recv_timeout(remaining) else {
                break;
            };
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
                && message
                    .get("params")
                    .and_then(|p| p.get("uri"))
                    .and_then(Value::as_str)
                    == Some(file_uri.as_str())
                && !parse_diagnostics(&message, &file).is_empty()
            {
                break;
            }
            answer_server_request(&mut stdin, &message);
        }
        let offset = text
            .lines()
            .nth(line.saturating_sub(1))
            .unwrap_or("")
            .chars()
            .take(column.saturating_sub(1))
            .map(char::len_utf16)
            .sum::<usize>();
        let method = match kind {
            QueryKind::Hover => "textDocument/hover",
            QueryKind::Definition => "textDocument/definition",
            QueryKind::Completion => "textDocument/completion",
            QueryKind::References => "textDocument/references",
            QueryKind::Rename => "textDocument/rename",
        };
        let mut params = json!({
            "textDocument":{"uri":uri(&file)},"position":{"line":line.saturating_sub(1),"character":offset}
        });
        match kind {
            QueryKind::References => params["context"] = json!({"includeDeclaration":true}),
            QueryKind::Completion => params["context"] = json!({"triggerKind":1}),
            QueryKind::Rename => params["newName"] = json!(new_name.unwrap_or("")),
            _ => {}
        }
        write_message(
            &mut stdin,
            &json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}),
        )
        .map_err(|e| e.to_string())?;
        loop {
            let message = receiver
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| format!("{program} did not answer {method}"))?;
            if message.get("id") == Some(&json!(2)) {
                if let Some(error) = message.get("error") {
                    return Err(format!("{program}: {error}"));
                }
                return Ok(query_result(kind, &message, &root));
            }
            answer_server_request(&mut stdin, &message);
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    result.unwrap_or_else(failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_rejects_duplicate_or_oversized_headers_and_payloads() {
        for bytes in [
            b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".to_vec(),
            b"Content-Length: 4194305\r\n\r\n".to_vec(),
            vec![b'x'; 8193],
            b"Content-Length: 2\r\n\r\n{".to_vec(),
        ] {
            assert!(read_message(&mut std::io::Cursor::new(bytes)).is_err());
        }
        assert_eq!(
            read_message(&mut std::io::Cursor::new(b"content-length: 2\r\n\r\n{}")).unwrap(),
            json!({})
        );
    }

    #[test]
    #[ignore = "requires an installed rust-analyzer server"]
    fn rust_analyzer_protocol_smoke() {
        let root = std::env::temp_dir().join(format!("forge-lsp-{}", std::process::id()));
        let _ = std::fs::create_dir_all(root.join("src"));
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"lsp-probe\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
        )
        .unwrap();
        let file = root.join("src/main.rs");
        let source = "fn helper() {}\nfn main() { helper(); let x: i32 = true; }\n";
        std::fs::write(&file, source).unwrap();
        let report = check(&root, &file, source);
        assert!(report.failure.is_none(), "{:?}", report.failure);
        assert!(report.problems.iter().any(|problem| problem.error));
        let definition = query(&root, &file, source, 2, 14, QueryKind::Definition);
        assert_eq!(
            definition.location.as_ref().map(|(_, line, _)| *line),
            Some(1),
            "{}",
            definition.message
        );
        let completion = query(&root, &file, source, 2, 15, QueryKind::Completion);
        assert!(
            completion
                .completions
                .iter()
                .any(|(label, _)| label.contains("helper")),
            "{}",
            completion.message
        );
        let references = query(&root, &file, source, 2, 14, QueryKind::References);
        assert!(references.locations.len() >= 2, "{}", references.message);
        let rename = query_with_name(
            &root,
            &file,
            source,
            2,
            14,
            QueryKind::Rename,
            Some("renamed"),
        );
        let edit = rename.rename_edit.expect(&rename.message);
        let preview = crate::lsp_rename::plan(&root, &edit).unwrap();
        assert!(preview.edit_count >= 2);
        assert!(preview.files[0].after.contains("fn renamed()"));
        let _ = std::fs::remove_dir_all(root);
    }
}
