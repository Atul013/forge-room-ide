use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Instant,
};
pub(crate) struct Captured {
    pub body: Value,
    pub headers: String,
}
pub(crate) fn mock(
    protocol: Protocol,
    response: String,
    streaming: bool,
    status: &str,
) -> (Config, thread::JoinHandle<Captured>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let status = status.to_owned();
    let thread = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        let mut stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < until, "native request never arrived");
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => panic!("{e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut header = Vec::new();
        let mut byte = [0; 1];
        while !header.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
            assert!(header.len() < 16384);
        }
        let headers = String::from_utf8(header).unwrap();
        let size = headers
            .lines()
            .find_map(|l| {
                l.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|v| v.trim().parse::<usize>().ok())
            })
            .unwrap_or(0);
        let mut bytes = vec![0; size];
        stream.read_exact(&mut bytes).unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        let kind = if streaming {
            "text/event-stream"
        } else {
            "application/json"
        };
        write!(stream,"HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",response.len()).unwrap();
        for fragment in response.as_bytes().chunks(7) {
            if stream.write_all(fragment).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        Captured { body, headers }
    });
    (
        Config {
            base_url: format!("http://127.0.0.1:{port}/v1"),
            model: "test-model".into(),
            protocol,
            ..Config::default()
        },
        thread,
    )
}
pub(crate) fn finish(run: &mut Run) -> (String, Option<String>) {
    let until = Instant::now() + Duration::from_secs(10);
    let mut text = String::new();
    loop {
        for event in run.poll() {
            match event {
                Event::Text(t) => text.push_str(&t),
                Event::Done { error, .. } => return (text, error),
                _ => {}
            }
        }
        assert!(Instant::now() < until, "native worker failed to finish");
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn chat_stream_handles_split_utf8_and_bounded_request_without_tools() {
    let response = format!(
        "data: {}\r\n\r\ndata: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"delta":{"content":"hello 😀"},"finish_reason":null}]}),
        json!({"choices":[{"delta":{},"finish_reason":"stop"}]})
    );
    let (config, server) = mock(Protocol::Chat, response, true, "200 OK");
    let prepared = prepare(&config, "question".into(), &[]).unwrap();
    let mut run = start(prepared, || {}).unwrap();
    let (text, error) = finish(&mut run);
    assert_eq!(error, None);
    assert_eq!(text, "hello 😀");
    let request = server.join().unwrap();
    assert!(
        request
            .headers
            .starts_with("POST /v1/chat/completions HTTP/1.1")
    );
    assert_eq!(request.body["max_tokens"], 4096);
    assert!(request.body.get("tools").is_none());
    assert_eq!(request.body["messages"][0]["content"], "question");
}
#[test]
fn messages_stream_and_json_fallback_use_their_own_contract() {
    let response = format!(
        "event: content_block_delta\ndata: {}\n\nevent: message_delta\ndata: {}\n\nevent: message_stop\ndata: {}\n\n",
        json!({"type":"content_block_delta","delta":{"type":"text_delta","text":"message answer"}}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn"}}),
        json!({"type":"message_stop"})
    );
    let (config, server) = mock(Protocol::Messages, response, true, "200 OK");
    let mut run = start(prepare(&config, "question".into(), &[]).unwrap(), || {}).unwrap();
    assert_eq!(finish(&mut run), ("message answer".into(), None));
    let request = server.join().unwrap();
    assert!(request.headers.starts_with("POST /v1/messages "));
    assert!(
        request
            .headers
            .to_lowercase()
            .contains("anthropic-version: 2023-06-01")
    );
    let response =
        json!({"choices":[{"message":{"content":"json answer","tool_calls":null,"function_call":null,"refusal":null},"finish_reason":"stop"}]})
            .to_string();
    let (mut config, server) = mock(Protocol::Chat, response, false, "200 OK");
    config.modern_limit = true;
    let mut run = start(prepare(&config, "q".into(), &[]).unwrap(), || {}).unwrap();
    assert_eq!(finish(&mut run), ("json answer".into(), None));
    let request = server.join().unwrap();
    assert_eq!(request.body["max_completion_tokens"], 4096);
    assert!(request.body.get("max_tokens").is_none());
}
#[test]
fn refused_tools_truncation_incomplete_stream_and_http_errors_never_complete() {
    for (response, streaming, status, needle) in [
        (
            format!(
                "data: {}\n\ndata: [DONE]\n\n",
                json!({"choices":[{"delta":{"tool_calls":[{"function":{"name":"run_command","arguments":"{}"}}]},"finish_reason":"tool_calls"}]})
            ),
            true,
            "200 OK",
            "tools",
        ),
        (
            format!(
                "data: {}\n\n",
                json!({"choices":[{"delta":{"content":"partial"},"finish_reason":"length"}]})
            ),
            true,
            "200 OK",
            "token limit",
        ),
        (
            format!(
                "data: {}\n\n",
                json!({"choices":[{"delta":{"content":"partial"}}]})
            ),
            true,
            "200 OK",
            "complete answer",
        ),
        (
            "{\"error\":\"DO NOT SHOW PRIVATE RESPONSE\"}".into(),
            false,
            "401 Unauthorized",
            "HTTP 401",
        ),
    ] {
        let (config, server) = mock(Protocol::Chat, response, streaming, status);
        let mut run = start(prepare(&config, "q".into(), &[]).unwrap(), || {}).unwrap();
        let (_, error) = finish(&mut run);
        let error = error.unwrap();
        assert!(error.contains(needle), "{error}");
        assert!(!error.contains("PRIVATE RESPONSE"));
        server.join().unwrap();
    }
}
#[test]
fn stop_closes_a_request_waiting_for_headers() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = std::sync::mpsc::channel();
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut buffer = [0; 2048];
        let _ = stream.read(&mut buffer).unwrap();
        tx.send(()).unwrap();
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) => panic!("request connection did not close: {e}"),
            }
        }
    });
    let config = Config {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        model: "test".into(),
        ..Config::default()
    };
    let mut run = start(prepare(&config, "q".into(), &[]).unwrap(), || {}).unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let before = Instant::now();
    run.cancel();
    let (_, error) = finish(&mut run);
    assert_eq!(error.as_deref(), Some("Stopped"));
    assert!(before.elapsed() < Duration::from_secs(2));
    peer.join().unwrap();
}
#[test]
fn endpoint_auth_limits_history_and_stream_frames_are_validated() {
    let mut config = Config {
        model: "test".into(),
        ..Config::default()
    };
    for endpoint in [
        "http://192.168.1.2/v1",
        "http://example.com/v1",
        "https://key@example.com/v1",
        "https://example.com/v1?key=private",
        "file:///tmp/api",
    ] {
        config.base_url = endpoint.into();
        assert!(config.validate().is_err());
    }
    config.base_url = "http://localhost:11434/v1".into();
    assert_eq!(config.url("models").unwrap().host_str(), Some("127.0.0.1"));
    config.key_env = "sk-not-a-variable".into();
    assert!(config.validate().is_err());
    config.key_env = "FORGE_NATIVE_TEST_MISSING_KEY_179491".into();
    assert!(start(prepare(&config, "q".into(), &[]).unwrap(), || {}).is_err());
    config.key_env.clear();
    config.max_tokens = 1;
    assert!(prepare(&config, "q".into(), &[]).is_err());
    config.max_tokens = 4096;
    assert!(prepare(&config, "x".repeat(MAX_INPUT), &[]).is_err());
    let old = vec![json!({"role":"user","content":"old native turn"})];
    let p = prepare(&config, "q".into(), &old).unwrap();
    assert_eq!(p.body["messages"].as_array().unwrap().len(), 1);
    config.history = true;
    assert_eq!(
        prepare(&config, "q".into(), &old).unwrap().body["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let mut decoder = Decoder::default();
    assert!(decoder.feed(&vec![b'x'; 256 * 1024 + 1]).is_err());
    let old: crate::settings::Settings = serde_json::from_str("{}").unwrap();
    assert!(old.native_model.is_none());
    let loaded: Config = serde_json::from_str("{}").unwrap();
    assert_eq!(loaded.max_tokens, 4096);
}
#[test]
fn model_catalog_uses_get_and_returns_bounded_ids() {
    let (config, server) = mock(
        Protocol::Chat,
        json!({"data":[{"id":"local/model"},{"id":"local/model"},{"id":"other"}]}).to_string(),
        false,
        "200 OK",
    );
    assert_eq!(discover(config).unwrap(), vec!["local/model", "other"]);
    let request = server.join().unwrap();
    assert!(request.headers.starts_with("GET /v1/models "));
    assert!(request.body.is_null());
}
#[test]
fn redirects_do_not_forward_prompt_to_another_host_and_native_refusals_fail() {
    let other = TcpListener::bind("127.0.0.1:0").unwrap();
    other.set_nonblocking(true).unwrap();
    let status = format!(
        "302 Found\r\nLocation: http://127.0.0.1:{}/stolen",
        other.local_addr().unwrap().port()
    );
    let (config, server) = mock(Protocol::Chat, "private error body".into(), false, &status);
    let mut run = start(
        prepare(&config, "PRIVATE PROMPT".into(), &[]).unwrap(),
        || {},
    )
    .unwrap();
    assert!(finish(&mut run).1.unwrap().contains("HTTP 302"));
    server.join().unwrap();
    assert!(matches!(other.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    let value = json!({"choices":[{"message":{"content":"partial","refusal":"refused"},"finish_reason":"stop"}]});
    assert!(decode_full(Protocol::Chat, &value).is_err());
    let value =
        json!({"content":[{"type":"text","text":"anthropic JSON"}],"stop_reason":"end_turn"});
    assert_eq!(
        decode_full(Protocol::Messages, &value).unwrap(),
        "anthropic JSON"
    );
}
#[test]
fn native_history_is_scoped_to_account_protocol_flow_and_workspace() {
    let root = std::env::temp_dir().join(format!("forge-native-history-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let mut state = panel::State::default();
    state.config.model = "test".into();
    state.config.history = true;
    state
        .prepare(
            &root,
            crate::ai_workflow::Flow::Ask,
            "first".into(),
            "first".into(),
        )
        .unwrap();
    state.review = None;
    state.inflight = Some("first".into());
    state.complete(true, "answer");
    assert_eq!(state.history.len(), 2);
    state
        .prepare(
            &root,
            crate::ai_workflow::Flow::Ask,
            "next".into(),
            "next".into(),
        )
        .unwrap();
    assert_eq!(
        state.review.take().unwrap().prepared.body["messages"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    state.config.key_env = "OTHER_ACCOUNT_KEY".into();
    state
        .prepare(
            &root,
            crate::ai_workflow::Flow::Ask,
            "next".into(),
            "next".into(),
        )
        .unwrap();
    assert_eq!(
        state.review.take().unwrap().prepared.body["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    state.inflight = Some("next".into());
    state.complete(true, "other account");
    state
        .prepare(
            &root,
            crate::ai_workflow::Flow::Plan,
            "plan".into(),
            "plan".into(),
        )
        .unwrap();
    assert_eq!(
        state.review.take().unwrap().prepared.body["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    state.reset();
    assert!(state.history.is_empty());
    assert!(state.review.is_none());
}
