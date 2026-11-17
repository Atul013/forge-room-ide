//! A bounded Debug Adapter Protocol client for an adapter listening on loopback.
//! The adapter is started separately; connecting never executes project code implicitly.
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

const MAX_MESSAGE: usize = 1024 * 1024;

#[derive(Debug)]
pub enum Event {
    Connected,
    Initialized,
    BreakpointsSet { verified: bool },
    Message(String),
    Output(String),
    Stopped { reason: String, thread: i64 },
    Stack(Vec<Frame>),
    Variables(Vec<Variable>),
    Ended(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub id: i64,
    pub name: String,
    pub path: Option<PathBuf>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub kind: String,
}

pub struct Session {
    commands: Sender<Command>,
    events: Receiver<Event>,
    adapter: Option<Child>,
}

enum Command {
    Request(&'static str, Value),
    Disconnect(bool),
}

impl Session {
    pub fn connect(port: u16) -> Result<Self, String> {
        if port == 0 {
            return Err("Enter a loopback adapter port between 1 and 65535".into());
        }
        let address = SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
            .map_err(|e| format!("Cannot connect to local debug adapter: {e}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .map_err(|e| e.to_string())?;
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        std::thread::spawn(move || run(stream, command_rx, event_tx));
        Ok(Self {
            commands: command_tx,
            events: event_rx,
            adapter: None,
        })
    }

    /// Start debugpy's adapter for the chosen project interpreter and connect only to loopback.
    pub fn start_python(root: &Path) -> Result<Self, String> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let interpreter = python_interpreter(&root);
        let reserved = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = reserved.local_addr().map_err(|e| e.to_string())?.port();
        drop(reserved);
        let mut command = ProcessCommand::new(&interpreter);
        command
            .args(["-m", "debugpy.adapter", "--host", "127.0.0.1", "--port"])
            .arg(port.to_string())
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("Cannot start {}: {e}", interpreter.display()))?;
        for _ in 0..60 {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(format!(
                    "Python debug adapter exited ({status}); install debugpy in the project interpreter"
                ));
            }
            if let Ok(mut session) = Self::connect(port) {
                session.adapter = Some(child);
                return Ok(session);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
        let _ = child.wait();
        Err("Python debug adapter did not open its local port".into())
    }

    pub fn request(&self, name: &'static str, args: Value) {
        let _ = self.commands.send(Command::Request(name, args));
    }

    pub fn disconnect(&self) {
        self.finish(false);
    }

    pub fn stop(&self) {
        self.finish(true);
    }

    fn finish(&self, terminate: bool) {
        let _ = self.commands.send(Command::Disconnect(terminate));
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) {
            match self.events.recv_timeout(remaining) {
                Ok(Event::Ended(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }
    }

    pub fn drain(&self) -> Vec<Event> {
        self.events.try_iter().take(32).collect()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(mut child) = self.adapter.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn python_interpreter(root: &Path) -> PathBuf {
    for folder in [".venv", "venv"] {
        let path = root.join(folder).join(if cfg!(windows) {
            "Scripts/python.exe"
        } else {
            "bin/python"
        });
        if path.is_file() {
            return path;
        }
    }
    PathBuf::from(if cfg!(windows) { "py" } else { "python3" })
}

fn write_message(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value)?;
    if body.len() > MAX_MESSAGE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DAP message too large",
        ));
    }
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Value> {
    let mut length = None;
    let mut header_bytes = 0;
    loop {
        let mut line = Vec::new();
        loop {
            if header_bytes == 4096 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "DAP headers too large",
                ));
            }
            let mut byte = [0];
            reader.read_exact(&mut byte)?;
            header_bytes += 1;
            line.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
        let line = std::str::from_utf8(&line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if let Some(value) = line
            .split_once(':')
            .filter(|(key, _)| key.eq_ignore_ascii_case("Content-Length"))
        {
            length = value.1.trim().parse::<usize>().ok();
        }
    }
    let length = length
        .filter(|length| *length > 0 && *length <= MAX_MESSAGE)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid DAP length"))?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn run(mut stream: TcpStream, commands: Receiver<Command>, events: Sender<Event>) {
    let reader_stream = match stream.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            let _ = events.send(Event::Ended(error.to_string()));
            return;
        }
    };
    let mut reader = BufReader::new(reader_stream);
    let mut sequence = 1;
    let initialize = json!({"clientID":"forge-room","clientName":"Forge Room","adapterID":"generic",
        "linesStartAt1":true,"columnsStartAt1":true,"pathFormat":"path",
        "supportsVariableType":false,"supportsRunInTerminalRequest":false});
    if write_request(&mut stream, &mut sequence, "initialize", initialize).is_err() {
        let _ = events.send(Event::Ended("Failed to initialize adapter".into()));
        return;
    }
    let mut initialized = false;
    loop {
        match read_message(&mut reader) {
            Ok(value) => {
                let kind = value.get("type").and_then(Value::as_str);
                if kind == Some("response") {
                    let name = value.get("command").and_then(Value::as_str).unwrap_or("");
                    let ok = value.get("success").and_then(Value::as_bool) == Some(true);
                    if name == "initialize" && ok {
                        initialized = true;
                        let _ = events.send(Event::Connected);
                    } else if name == "setBreakpoints" {
                        let verified = ok
                            && value
                                .pointer("/body/breakpoints")
                                .and_then(Value::as_array)
                                .is_some_and(|items| {
                                    items.iter().all(|item| {
                                        item.get("verified").and_then(Value::as_bool) == Some(true)
                                    })
                                });
                        let _ = events.send(Event::BreakpointsSet { verified });
                    } else if !ok {
                        let detail = value
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("request rejected");
                        let _ = events.send(Event::Message(format!("{name}: {}", bounded(detail))));
                    }
                    if name == "stackTrace" && ok {
                        let frames: Vec<Frame> = value
                            .pointer("/body/stackFrames")
                            .and_then(Value::as_array)
                            .map(|items| {
                                items
                                    .iter()
                                    .take(40)
                                    .map(|frame| {
                                        let name = frame
                                            .get("name")
                                            .and_then(Value::as_str)
                                            .unwrap_or("frame");
                                        Frame {
                                            id: frame
                                                .get("id")
                                                .and_then(Value::as_i64)
                                                .unwrap_or(0),
                                            name: bounded(name),
                                            path: frame
                                                .pointer("/source/path")
                                                .and_then(Value::as_str)
                                                .map(PathBuf::from),
                                            line: frame
                                                .get("line")
                                                .and_then(Value::as_u64)
                                                .unwrap_or(0)
                                                as usize,
                                        }
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        if let Some(frame) = frames.first().filter(|frame| frame.id > 0) {
                            let _ = write_request(
                                &mut stream,
                                &mut sequence,
                                "scopes",
                                json!({"frameId":frame.id}),
                            );
                        }
                        let _ = events.send(Event::Stack(frames));
                    } else if name == "scopes" && ok {
                        if let Some(scopes) =
                            value.pointer("/body/scopes").and_then(Value::as_array)
                        {
                            let chosen = scopes
                                .iter()
                                .find(|scope| {
                                    scope.get("name").and_then(Value::as_str) == Some("Locals")
                                })
                                .or_else(|| scopes.first());
                            if let Some(reference) = chosen
                                .and_then(|scope| scope.get("variablesReference"))
                                .and_then(Value::as_i64)
                                .filter(|n| *n > 0)
                            {
                                let _ = write_request(
                                    &mut stream,
                                    &mut sequence,
                                    "variables",
                                    json!({"variablesReference":reference,"start":0,"count":100}),
                                );
                            }
                        }
                    } else if name == "variables" && ok {
                        let variables = value
                            .pointer("/body/variables")
                            .and_then(Value::as_array)
                            .map(|items| {
                                items
                                    .iter()
                                    .take(100)
                                    .map(|item| Variable {
                                        name: item
                                            .get("name")
                                            .and_then(Value::as_str)
                                            .unwrap_or("")
                                            .chars()
                                            .take(120)
                                            .collect(),
                                        value: item
                                            .get("value")
                                            .and_then(Value::as_str)
                                            .unwrap_or("")
                                            .chars()
                                            .take(400)
                                            .collect(),
                                        kind: item
                                            .get("type")
                                            .and_then(Value::as_str)
                                            .unwrap_or("")
                                            .chars()
                                            .take(80)
                                            .collect(),
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        let _ = events.send(Event::Variables(variables));
                    }
                } else if kind == Some("event") {
                    match value.get("event").and_then(Value::as_str).unwrap_or("") {
                        "initialized" => {
                            let _ = events.send(Event::Initialized);
                        }
                        "stopped" => {
                            let reason = value
                                .pointer("/body/reason")
                                .and_then(Value::as_str)
                                .unwrap_or("paused");
                            let thread = value
                                .pointer("/body/threadId")
                                .and_then(Value::as_i64)
                                .unwrap_or(1);
                            let _ = events.send(Event::Stopped {
                                reason: bounded(reason),
                                thread,
                            });
                            let _ = write_request(
                                &mut stream,
                                &mut sequence,
                                "stackTrace",
                                json!({"threadId":thread,"startFrame":0,"levels":40}),
                            );
                        }
                        "output" => {
                            if let Some(output) =
                                value.pointer("/body/output").and_then(Value::as_str)
                            {
                                let _ = events.send(Event::Output(bounded(output)));
                            }
                        }
                        "terminated" | "exited" => {
                            let _ = events.send(Event::Ended("Debug target ended".into()));
                            return;
                        }
                        _ => {}
                    }
                } else if kind == Some("request") {
                    // Never grant reverse requests that could start processes or open arbitrary files.
                    let reply = json!({"seq":sequence,"type":"response","request_seq":value.get("seq"),
                        "command":value.get("command"),"success":false,"message":"Unsupported reverse request"});
                    sequence += 1;
                    let _ = write_message(&mut stream, &reply);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(error) => {
                let _ = events.send(Event::Ended(format!("Adapter disconnected: {error}")));
                return;
            }
        }
        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Request(name, args) if initialized => {
                    if let Err(error) = write_request(&mut stream, &mut sequence, name, args) {
                        let _ = events.send(Event::Ended(error.to_string()));
                        return;
                    }
                }
                Command::Disconnect(terminate) => {
                    let _ = write_request(
                        &mut stream,
                        &mut sequence,
                        "disconnect",
                        json!({"terminateDebuggee":terminate}),
                    );
                    for _ in 0..2 {
                        if let Ok(reply) = read_message(&mut reader)
                            && reply.get("type").and_then(Value::as_str) == Some("response")
                            && reply.get("command").and_then(Value::as_str) == Some("disconnect")
                        {
                            break;
                        }
                    }
                    let _ = events.send(Event::Ended("Disconnected from adapter".into()));
                    return;
                }
                _ => {
                    let _ = events.send(Event::Message("Adapter is still initializing".into()));
                }
            }
        }
    }
}

fn write_request(
    stream: &mut TcpStream,
    sequence: &mut i64,
    command: &str,
    arguments: Value,
) -> io::Result<()> {
    let value = json!({"seq":*sequence,"type":"request","command":command,"arguments":arguments});
    *sequence += 1;
    write_message(stream, &value)
}

fn bounded(text: &str) -> String {
    text.chars().take(1000).collect()
}

/// Build the small debugpy launch subset from a saved file in the trusted folder.
pub fn python_launch(root: &Path, file: &Path) -> Result<Value, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let file = file.canonicalize().map_err(|e| e.to_string())?;
    if !file.starts_with(&root) || !file.is_file() {
        return Err("Select a saved file inside the trusted folder".into());
    }
    if file.extension().and_then(|e| e.to_str()) != Some("py") {
        return Err("Select a Python .py file to launch".into());
    }
    Ok(
        json!({"program":crate::tabs::normalize(&file),"cwd":crate::tabs::normalize(&root),"console":"internalConsole","justMyCode":true}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    #[test]
    fn framed_message_round_trip_and_limit() {
        let mut bytes = Vec::new();
        write_message(&mut bytes, &json!({"type":"event","event":"stopped"})).unwrap();
        assert_eq!(
            read_message(&mut BufReader::new(bytes.as_slice())).unwrap()["event"],
            "stopped"
        );
        let mut oversized = BufReader::new(b"Content-Length: 999999999\r\n\r\n".as_slice());
        assert_eq!(
            read_message(&mut oversized).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn local_adapter_initializes_and_reports_stopped_stack() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let adapter = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let init = read_message(&mut reader).unwrap();
            assert_eq!(init["command"], "initialize");
            write_message(
                &mut stream,
                &json!({"seq":1,"type":"response","request_seq":1,
                "command":"initialize","success":true,"body":{}}),
            )
            .unwrap();
            let attach = read_message(&mut reader).unwrap();
            assert_eq!(attach["command"], "attach");
            assert_eq!(attach["arguments"]["justMyCode"], true);
            write_message(
                &mut stream,
                &json!({"seq":2,"type":"event","event":"initialized"}),
            )
            .unwrap();
            let configured = read_message(&mut reader).unwrap();
            assert_eq!(configured["command"], "configurationDone");
            write_message(
                &mut stream,
                &json!({"seq":2,"type":"event","event":"stopped",
                "body":{"reason":"breakpoint","threadId":7}}),
            )
            .unwrap();
            let stack = read_message(&mut reader).unwrap();
            assert_eq!(stack["command"], "stackTrace");
            assert_eq!(stack["arguments"]["threadId"], 7);
            write_message(
                &mut stream,
                &json!({"seq":3,"type":"response","request_seq":2,
                "command":"stackTrace","success":true,"body":{"stackFrames":[
                    {"name":"main","source":{"path":"sample.py"},"line":4}]}}),
            )
            .unwrap();
        });
        let session = Session::connect(port).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut connected = false;
        let mut stopped = false;
        let mut stack = false;
        while std::time::Instant::now() < deadline && !(connected && stopped && stack) {
            for event in session.drain() {
                match event {
                    Event::Connected => {
                        connected = true;
                        session.request("attach", json!({"justMyCode":true}));
                    }
                    Event::Initialized => session.request("configurationDone", json!({})),
                    Event::Stopped { reason, thread } => {
                        assert_eq!(reason, "breakpoint");
                        assert_eq!(thread, 7);
                        stopped = true;
                    }
                    Event::Stack(frames) => {
                        assert_eq!(frames[0].name, "main");
                        assert_eq!(frames[0].path.as_deref(), Some(Path::new("sample.py")));
                        assert_eq!(frames[0].line, 4);
                        stack = true;
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        adapter.join().unwrap();
        assert!(connected && stopped && stack);
    }

    #[test]
    fn python_launch_stays_inside_workspace() {
        let dir = std::env::temp_dir().join(format!("forge-dap-{}", std::process::id()));
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("main.py");
        let outside = dir.join("outside.py");
        std::fs::write(&file, "print(1)").unwrap();
        std::fs::write(&outside, "print(2)").unwrap();
        assert_eq!(
            python_launch(&root, &file).unwrap()["console"],
            "internalConsole"
        );
        assert!(python_launch(&root, &outside).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn project_interpreter_precedes_system_python() {
        let root = std::env::temp_dir().join(format!("forge-dap-venv-{}", std::process::id()));
        let interpreter = root.join(".venv").join(if cfg!(windows) {
            "Scripts/python.exe"
        } else {
            "bin/python"
        });
        std::fs::create_dir_all(interpreter.parent().unwrap()).unwrap();
        std::fs::write(&interpreter, b"").unwrap();
        assert_eq!(python_interpreter(&root), interpreter);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_debugpy_launch_when_adapter_port_is_supplied() {
        let live_port = std::env::var("FORGE_DAP_LIVE_PORT").ok();
        let self_start = std::env::var("FORGE_DAP_SELF_START").ok().as_deref() == Some("1");
        if live_port.is_none() && !self_start {
            return;
        }
        let root = std::env::temp_dir().join(format!("forge-dap-live-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("sample.py");
        std::fs::write(&file, "first = 1\nsecond = 2\nprint('FORGE_DAP_LIVE_OK')\n").unwrap();
        let args = python_launch(&root, &file).unwrap();
        let session = if self_start {
            Session::start_python(&root).unwrap()
        } else {
            Session::connect(live_port.unwrap().parse::<u16>().unwrap()).unwrap()
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        let mut launched = false;
        let mut configured = false;
        let mut ended = false;
        let mut output = false;
        let mut stops = 0;
        let mut stepped = false;
        let mut breakpoint_verified = false;
        let mut locals_seen = false;
        let mut first_thread = None;
        let mut continue_thread = None;
        let mut stack_lines = Vec::new();
        let mut errors = Vec::new();
        while std::time::Instant::now() < deadline && !ended {
            for event in session.drain() {
                match event {
                    Event::Connected => {
                        session.request("launch", args.clone());
                        launched = true;
                    }
                    Event::Initialized => {
                        session.request(
                            "setBreakpoints",
                            json!({"source":{"path":crate::tabs::normalize(&file)},"breakpoints":[{"line":2}]}),
                        );
                    }
                    Event::BreakpointsSet { verified } => {
                        breakpoint_verified = verified;
                        session.request("configurationDone", json!({}));
                        configured = true;
                    }
                    Event::Stopped { thread, .. } => {
                        stops += 1;
                        if stops == 1 {
                            first_thread = Some(thread);
                        } else {
                            continue_thread = Some(thread);
                        }
                    }
                    Event::Stack(frames) => {
                        if frames.iter().any(|frame| frame.line == 3)
                            && let Some(thread) = continue_thread.take()
                        {
                            session.request("continue", json!({"threadId":thread}));
                        }
                        stack_lines.extend(frames);
                    }
                    Event::Output(message) => output |= message.contains("FORGE_DAP_LIVE_OK"),
                    Event::Variables(items) => {
                        if stops == 1 && !stepped {
                            locals_seen = items
                                .iter()
                                .any(|item| item.name == "first" && item.value == "1");
                            if let Some(thread) = first_thread.take() {
                                session.request("next", json!({"threadId":thread}));
                                stepped = true;
                            }
                        }
                    }
                    Event::Message(message) => {
                        if message.starts_with("launch:")
                            || message.starts_with("configurationDone:")
                        {
                            errors.push(message);
                        }
                    }
                    Event::Ended(_) => ended = true,
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = std::fs::remove_dir_all(&root);
        assert!(errors.is_empty(), "adapter errors: {errors:?}");
        assert!(
            launched
                && configured
                && ended
                && output
                && stepped
                && locals_seen
                && stops >= 2
                && breakpoint_verified
                && stack_lines.iter().any(|frame| frame.line == 2)
                && stack_lines.iter().any(|frame| frame.line == 3),
            "launch={launched} configured={configured} ended={ended} output={output} stops={stops} stepped={stepped} locals={locals_seen} breakpoint_verified={breakpoint_verified} stack={stack_lines:?}"
        );
    }
}
