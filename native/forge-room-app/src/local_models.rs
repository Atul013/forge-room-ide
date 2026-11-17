//! Small, bounded localhost model catalog requests for the two supported local providers.
use crate::agent::{Provider, valid_model};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

const MAX_RESPONSE: u64 = 512 * 1024;

pub fn discover(provider: Provider) -> Result<Vec<String>, String> {
    let (port, path) = match provider {
        Provider::Ollama => (11434, "/api/tags"),
        Provider::LmStudio => (1234, "/v1/models"),
        _ => return Err("Choose Ollama or LM Studio for local model discovery".into()),
    };
    fetch(provider, port, path)
}

fn fetch(provider: Provider, port: u16, path: &str) -> Result<Vec<String>, String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream =
        TcpStream::connect_timeout(&address, Duration::from_secs(2)).map_err(|_| {
            format!(
                "{} server is not available on localhost:{port}",
                provider.name()
            )
        })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: localhost:{port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    stream
        .take(MAX_RESPONSE + 1)
        .read_to_end(&mut response)
        .map_err(|e| e.to_string())?;
    if response.len() as u64 > MAX_RESPONSE {
        return Err("Local model list is too large".into());
    }
    let boundary = response
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .ok_or("Local server sent an invalid HTTP response")?;
    let header = std::str::from_utf8(&response[..boundary]).map_err(|e| e.to_string())?;
    if !header
        .lines()
        .next()
        .is_some_and(|line| line.contains(" 200 "))
    {
        return Err(format!("{} model list request failed", provider.name()));
    }
    let body = &response[boundary + 4..];
    let chunked = header.lines().any(|line| {
        line.to_ascii_lowercase().starts_with("transfer-encoding:")
            && line.to_ascii_lowercase().contains("chunked")
    });
    let decoded = if chunked {
        decode_chunks(body)?
    } else {
        body.to_vec()
    };
    parse_models(provider, &decoded)
}

fn decode_chunks(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut rest = body;
    loop {
        let line_end = rest
            .windows(2)
            .position(|part| part == b"\r\n")
            .ok_or("Invalid chunked local response")?;
        let size = std::str::from_utf8(&rest[..line_end]).map_err(|e| e.to_string())?;
        let size = usize::from_str_radix(size.split(';').next().unwrap_or(""), 16)
            .map_err(|_| "Invalid chunk size")?;
        rest = &rest[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if size > rest.len().saturating_sub(2)
            || out.len() + size > MAX_RESPONSE as usize
            || rest.get(size..size + 2) != Some(b"\r\n")
        {
            return Err("Invalid chunked local response".into());
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

fn parse_models(provider: Provider, body: &[u8]) -> Result<Vec<String>, String> {
    let value: Value =
        serde_json::from_slice(body).map_err(|_| "Local server sent invalid model JSON")?;
    let items = match provider {
        Provider::Ollama => value.get("models"),
        Provider::LmStudio => value.get("data"),
        _ => None,
    }
    .and_then(Value::as_array)
    .ok_or("Local server sent no model list")?;
    let mut names: Vec<String> = items
        .iter()
        .take(500)
        .filter_map(|item| {
            let name = match provider {
                Provider::Ollama => item.get("name").or_else(|| item.get("model")),
                Provider::LmStudio => item.get("id"),
                _ => None,
            }?
            .as_str()?;
            valid_model(name).then(|| name.to_owned())
        })
        .collect();
    names.sort();
    names.dedup();
    names.truncate(100);
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn discovers_local_models_from_both_provider_shapes() {
        for (provider, path, body, expected) in [
            (
                Provider::Ollama,
                "/api/tags",
                r#"{"models":[{"name":"qwen:7b"},{"name":"bad model"}]}"#,
                "qwen:7b",
            ),
            (
                Provider::LmStudio,
                "/v1/models",
                r#"{"data":[{"id":"local/model-1"}]}"#,
                "local/model-1",
            ),
        ] {
            let server = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = server.local_addr().unwrap().port();
            let thread = std::thread::spawn(move || {
                let (mut stream, _) = server.accept().unwrap();
                let mut request = [0; 256];
                let size = stream.read(&mut request).unwrap();
                assert!(
                    String::from_utf8_lossy(&request[..size]).starts_with(&format!("GET {path} "))
                );
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            });
            assert_eq!(fetch(provider, port, path).unwrap(), [expected]);
            thread.join().unwrap();
        }
    }
}
