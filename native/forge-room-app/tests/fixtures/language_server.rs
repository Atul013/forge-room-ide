//! Standalone std-only protocol peer. Compiled by lifecycle tests with the current Rust toolchain.
use std::io::{BufRead, Read, Write};
fn send(value: &str) {
    let mut out = std::io::stdout().lock();
    write!(out, "Content-Length: {}\r\n\r\n{value}", value.len()).unwrap();
    out.flush().unwrap();
}
fn number(body: &str, field: &str) -> i64 {
    body.split(&format!("\"{field}\":"))
        .nth(1)
        .unwrap()
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap()
        .parse()
        .unwrap()
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut log = std::fs::File::create(&args[1]).unwrap();
    writeln!(log, "pid:{}", std::process::id()).unwrap();
    log.flush().unwrap();
    let mode = args.get(2).map(String::as_str).unwrap_or("incremental");
    let mut reader = std::io::BufReader::new(std::io::stdin().lock());
    loop {
        let mut size = 0;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                return;
            }
            if line == "\r\n" {
                break;
            }
            if let Some(n) = line.strip_prefix("Content-Length:") {
                size = n.trim().parse().unwrap();
            }
        }
        let mut bytes = vec![0; size];
        reader.read_exact(&mut bytes).unwrap();
        let body = String::from_utf8(bytes).unwrap();
        writeln!(log, "{body}").unwrap();
        log.flush().unwrap();
        if body.contains("\"method\":\"shutdown\"") {
            let id = number(&body, "id");
            send(&format!(r#"{{"jsonrpc":"2.0","id":{id},"result":null}}"#));
        } else if body.contains("\"method\":\"exit\"") {
            return;
        } else if body.contains("\"method\":\"initialize\"") {
            if mode == "stall" {
                std::thread::sleep(std::time::Duration::from_secs(120));
                return;
            }
            let sync = if mode == "full" { "1" } else { "2" };
            let encoding = if mode == "bad-encoding" {
                "utf-8"
            } else {
                "utf-16"
            };
            send(&format!(
                r#"{{"jsonrpc":"2.0","id":1,"result":{{"capabilities":{{"positionEncoding":"{encoding}","textDocumentSync":{{"openClose":true,"change":{sync},"save":{{"includeText":true}}}}}}}}}}"#
            ));
        } else if body.contains("\"method\":\"textDocument/didOpen\"")
            || body.contains("\"method\":\"textDocument/didChange\"")
        {
            let uri = body
                .split("\"uri\":")
                .nth(1)
                .unwrap()
                .split('"')
                .nth(1)
                .unwrap();
            let version = number(&body, "version");
            send(&format!(
                r#"{{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{{"uri":"{uri}","version":{version},"diagnostics":[{{"range":{{"start":{{"line":0,"character":0}},"end":{{"line":0,"character":1}}}},"severity":1,"message":"version {version}"}}]}}}}"#
            ));
            // Delayed stale diagnostics must never replace the current version.
            send(&format!(
                r#"{{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{{"uri":"{uri}","version":{},"diagnostics":[{{"range":{{"start":{{"line":0,"character":0}}}},"message":"STALE"}}]}}}}"#,
                version - 1
            ));
            send(
                r#"{"jsonrpc":"2.0","id":900,"method":"workspace/applyEdit","params":{"edit":{}}}"#,
            );
        } else if body.contains("\"method\":\"textDocument/hover\"") {
            if mode == "delay-hover" {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            let id = number(&body, "id");
            send(&format!(
                r#"{{"jsonrpc":"2.0","id":{id},"result":{{"contents":"live hover"}}}}"#
            ));
        } else if body.contains("\"method\":\"textDocument/references\"") {
            let id = number(&body, "id");
            send(&format!(r#"{{"jsonrpc":"2.0","id":{id},"result":[]}}"#));
        }
    }
}
