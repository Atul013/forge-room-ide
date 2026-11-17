//! A bounded Debug Adapter Protocol client for an adapter listening on loopback.
//! The adapter is started separately; connecting never executes project code implicitly.
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

const MAX_MESSAGE: usize = 1024 * 1024;

#[derive(Debug)]
pub enum Event {
    Connected,
    Message(String),
    Stopped { reason: String, thread: i64 },
    Stack(Vec<String>),
    Ended(String),
}

pub struct Session {
    commands: Sender<Command>,
    events: Receiver<Event>,
}

enum Command {
    Request(&'static str, Value),
    Disconnect,
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
        })
    }

    pub fn request(&self, name: &'static str, args: Value) {
        let _ = self.commands.send(Command::Request(name, args));
    }

    pub fn disconnect(&self) {
        let _ = self.commands.send(Command::Disconnect);
    }

    pub fn drain(&self) -> Vec<Event> {
        self.events.try_iter().take(32).collect()
    }
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
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "adapter closed",
            ));
        }
        header_bytes += read;
        if header_bytes > 4096 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DAP headers too large",
            ));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
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
                    } else if !ok {
                        let detail = value
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("request rejected");
                        let _ = events.send(Event::Message(format!("{name}: {}", bounded(detail))));
                    }
                    if name == "stackTrace" && ok {
                        let frames = value
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
                                        let path = frame
                                            .pointer("/source/path")
                                            .and_then(Value::as_str)
                                            .unwrap_or("");
                                        let line =
                                            frame.get("line").and_then(Value::as_u64).unwrap_or(0);
                                        bounded(&format!("{name}  {path}:{line}"))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        let _ = events.send(Event::Stack(frames));
                    }
                } else if kind == Some("event") {
                    match value.get("event").and_then(Value::as_str).unwrap_or("") {
                        "initialized" => {
                            let _ = write_request(
                                &mut stream,
                                &mut sequence,
                                "configurationDone",
                                json!({}),
                            );
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
                                let _ = events.send(Event::Message(bounded(output)));
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
                Command::Disconnect => {
                    let _ = write_request(
                        &mut stream,
                        &mut sequence,
                        "disconnect",
                        json!({"terminateDebuggee":false}),
                    );
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
                    Event::Stopped { reason, thread } => {
                        assert_eq!(reason, "breakpoint");
                        assert_eq!(thread, 7);
                        stopped = true;
                    }
                    Event::Stack(frames) => {
                        assert_eq!(frames, ["main  sample.py:4"]);
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
}
