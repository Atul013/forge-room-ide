use forge_room_core::transport::Host;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

struct Peer {
    child: Child,
    input: ChildStdin,
    output: Receiver<Value>,
}
impl Peer {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_forge-room-peer"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, output) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else {
                    break;
                };
                let Ok(value) = serde_json::from_str(&line) else {
                    break;
                };
                if sender.send(value).is_err() {
                    break;
                }
            }
        });
        Self {
            input: child.stdin.take().unwrap(),
            output,
            child,
        }
    }
    fn request(&mut self, value: Value) -> Value {
        writeln!(self.input, "{value}").unwrap();
        self.input.flush().unwrap();
        self.output
            .recv_timeout(Duration::from_secs(10))
            .expect("independent client responded within deadline")
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn independent_processes_join_deny_replay_revoke_and_leave() {
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "process trial").unwrap();
    let invite_a = host.invite(Duration::from_secs(60)).unwrap();
    let invite_b = host.invite(Duration::from_secs(60)).unwrap();
    let (mut a, mut b) = (Peer::start(), Peer::start());
    assert_ne!(a.child.id(), b.child.id());
    let joined_a = a.request(json!({"command":"join","invite":invite_a.encode(),"name":"Ada"}));
    assert!(joined_a["error"].is_null(), "join failed: {joined_a}");
    let id_a = joined_a["identity"]
        .as_str()
        .expect("first independent client joined")
        .to_string();
    let denied =
        b.request(json!({"command":"join","invite":invite_a.encode(),"name":"Impersonator"}));
    assert!(denied["error"].is_string());
    let joined_b = b.request(json!({"command":"join","invite":invite_b.encode(),"name":"Grace"}));
    assert_ne!(joined_b["identity"], id_a);
    assert_eq!(joined_b["snapshot"]["members"].as_array().unwrap().len(), 2);
    let posted = a.request(json!({"command":"post","text":"message from independent client A"}));
    assert_eq!(posted["snapshot"]["events"][0]["actor"], id_a);
    // Every poll is a fresh TCP/TLS connection with the existing session and replay cursor.
    let replay = b.request(json!({"command":"poll"}));
    assert_eq!(replay["snapshot"]["events"][0]["sequence"], 1);
    assert!(b.request(json!({"command":"poll"}))["snapshot"]["events"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(
        b.request(json!({"command":"post","text":"reply from independent client B"}))["error"]
            .is_null()
    );
    let received = a.request(json!({"command":"poll"}));
    assert_eq!(received["snapshot"]["events"][0]["sequence"], 2);
    assert_eq!(received["snapshot"]["members"].as_array().unwrap().len(), 2);
    host.revoke(&invite_b).unwrap();
    assert!(b.request(json!({"command":"poll"}))["error"].is_string());
    assert!(b.request(json!({"command":"post","text":"must be denied"}))["error"].is_string());
    assert_eq!(host.snapshot().unwrap().events.len(), 2);
    assert_eq!(a.request(json!({"command":"leave"}))["left"], true);
    assert!(host.snapshot().unwrap().members.is_empty());
}

#[test]
fn independent_client_recovers_after_a_real_connection_drop() {
    use std::{
        net::{Shutdown, TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread,
    };
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "reconnect trial").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let dropping = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    struct Proxy {
        stop: Arc<AtomicBool>,
        worker: Option<thread::JoinHandle<()>>,
    }
    impl Drop for Proxy {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = self.worker.take().unwrap().join();
        }
    }
    let (drop_connections, stopping, target) = (dropping.clone(), stop.clone(), host.address());
    let worker = thread::spawn(move || {
        while !stopping.load(Ordering::SeqCst) {
            let Ok((mut incoming, _)) = listener.accept() else {
                thread::sleep(Duration::from_millis(5));
                continue;
            };
            if drop_connections.load(Ordering::SeqCst) {
                continue;
            }
            incoming.set_nonblocking(false).unwrap();
            incoming
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            incoming
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut outgoing = TcpStream::connect_timeout(&target, Duration::from_secs(2)).unwrap();
            outgoing
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            outgoing
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let (mut reader, mut writer) =
                (incoming.try_clone().unwrap(), outgoing.try_clone().unwrap());
            let forward = thread::spawn(move || {
                let _ = std::io::copy(&mut reader, &mut writer);
                let _ = writer.shutdown(Shutdown::Write);
            });
            let _ = std::io::copy(&mut outgoing, &mut incoming);
            let _ = incoming.shutdown(Shutdown::Both);
            let _ = outgoing.shutdown(Shutdown::Both);
            let _ = forward.join();
        }
    });
    let _proxy = Proxy {
        stop,
        worker: Some(worker),
    };
    let invite_a = host.invite(Duration::from_secs(60)).unwrap();
    let invite_b = host.invite(Duration::from_secs(60)).unwrap();
    // Route B through a fault-injection proxy without changing the pinned host certificate.
    let mut routed: Value = serde_json::from_str(&invite_b.encode()).unwrap();
    routed["address"] = json!(endpoint.to_string());
    let (mut a, mut b) = (Peer::start(), Peer::start());
    let joined_a = a.request(json!({"command":"join","invite":invite_a.encode(),"name":"Ada"}));
    assert!(joined_a["identity"].is_string());
    let joined_b = b.request(json!({"command":"join","invite":routed.to_string(),"name":"Grace"}));
    let identity = joined_b["identity"]
        .as_str()
        .expect("B joined through proxy")
        .to_string();
    dropping.store(true, Ordering::SeqCst);
    assert!(b.request(json!({"command":"poll"}))["error"].is_string());
    a.request(json!({"command":"post","text":"committed while B was offline"}));
    dropping.store(false, Ordering::SeqCst);
    let recovered = b.request(json!({"command":"poll"}));
    assert_eq!(
        recovered["snapshot"]["events"][0]["text"],
        "committed while B was offline"
    );
    let roster = recovered["snapshot"]["members"].as_array().unwrap();
    assert_eq!(roster.len(), 2);
    assert_eq!(roster.iter().filter(|m| m["id"] == identity).count(), 1);
    assert!(b.request(json!({"command":"poll"}))["snapshot"]["events"]
        .as_array()
        .unwrap()
        .is_empty());
    host.revoke(&invite_b).unwrap();
    assert!(b.request(json!({"command":"poll"}))["error"].is_string());
    assert_eq!(host.snapshot().unwrap().events.len(), 1);
}
