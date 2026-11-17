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
    fn execute(&mut self, action: forge_room_core::transport::Action) -> Value {
        self.request(json!({"command":"execute","action":action}))
    }
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

#[test]
fn independent_clients_share_tasks_handoffs_clock_and_private_conversations() {
    use forge_room_core::{transport::Action, EvidenceKind, TaskPriority, TaskStatus};
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "shared workflow").unwrap();
    let (mut a, mut b, mut outsider) = (Peer::start(), Peer::start(), Peer::start());
    let a_join=a.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Ada"}));
    let b_invite = host.invite(Duration::from_secs(60)).unwrap();
    let b_join = b.request(json!({"command":"join","invite":b_invite.encode(),"name":"Grace"}));
    let a_id = a_join["identity"].as_str().unwrap().to_string();
    let b_id = b_join["identity"].as_str().unwrap().to_string();
    outsider.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Third client"}));
    let create = |title: &str, dependencies: Vec<String>| Action::CreateTask {
        title: title.into(),
        owner: a_id.clone(),
        criteria: vec!["checks passed".into()],
        dependencies,
        priority: TaskPriority::High,
        due_hours: Some(1),
    };
    assert!(a.execute(Action::CreateTask {
        title: "duplicate criteria".into(),
        owner: a_id.clone(),
        criteria: vec!["same".into(), "same".into()],
        dependencies: vec![],
        priority: TaskPriority::Normal,
        due_hours: None
    })["rejection"]
        .is_string());
    assert!(a.execute(create("Protocol", vec![]))["rejection"].is_null());
    assert!(a.execute(create("UI", vec!["T1".into()]))["rejection"].is_null());
    let shared = b.request(json!({"command":"poll"}));
    assert_eq!(
        shared["snapshot"]["shared"]["tasks"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(outsider.execute(Action::Transition {
        task: "T1".into(),
        status: TaskStatus::Done,
        completed: vec!["checks passed".into()]
    })["rejection"]
        .is_string());
    // A domain rejection is acknowledged and does not poison the client's next action.
    assert!(
        outsider.request(json!({"command":"post","text":"still connected after denial"}))["error"]
            .is_null()
    );
    assert!(a.execute(Action::Transition {
        task: "T2".into(),
        status: TaskStatus::InProgress,
        completed: vec![]
    })["rejection"]
        .is_string());
    assert!(a.execute(Action::Transition {
        task: "T1".into(),
        status: TaskStatus::Done,
        completed: vec![]
    })["rejection"]
        .is_string());
    assert!(a.execute(Action::Evidence {
        task: "T1".into(),
        kind: EvidenceKind::Check,
        text: "focused protocol trial passed".into()
    })["rejection"]
        .is_null());
    assert!(a.execute(Action::Transition {
        task: "T1".into(),
        status: TaskStatus::Done,
        completed: vec!["checks passed".into()]
    })["rejection"]
        .is_null());
    assert!(a.execute(Action::RequestHandoff {
        task: "T2".into(),
        target: b_id.clone()
    })["rejection"]
        .is_null());
    assert!(b.execute(Action::ApproveHandoff { task: "T2".into() })["rejection"].is_string());
    host.control(Action::ApproveHandoff { task: "T2".into() })
        .unwrap();
    assert!(a.execute(Action::Priority {
        task: "T2".into(),
        priority: TaskPriority::Critical
    })["rejection"]
        .is_string());
    let started = b.execute(Action::Transition {
        task: "T2".into(),
        status: TaskStatus::InProgress,
        completed: vec![],
    });
    assert!(started["rejection"].is_null());
    assert_eq!(started["snapshot"]["shared"]["tasks"][1]["ownerId"], b_id);
    assert!(outsider.execute(Action::Countdown {
        seconds: Some(10),
        running: true
    })["rejection"]
        .is_string());
    host.control(Action::Countdown {
        seconds: Some(90),
        running: true,
    })
    .unwrap();
    let clock = a.request(json!({"command":"poll"}));
    assert_eq!(clock["snapshot"]["shared"]["countdown"]["running"], true);
    assert!(
        clock["snapshot"]["shared"]["countdown"]["remaining_seconds"]
            .as_u64()
            .unwrap()
            <= 90
    );
    host.control(Action::Countdown {
        seconds: None,
        running: false,
    })
    .unwrap();
    let paused = b.request(json!({"command":"poll"}));
    assert_eq!(paused["snapshot"]["shared"]["countdown"]["running"], false);
    assert!(outsider.execute(Action::CreateMilestone {
        title: "unauthorized".into(),
        due_hours: None
    })["rejection"]
        .is_string());
    host.control(Action::CreateMilestone {
        title: "Working demo".into(),
        due_hours: Some(1),
    })
    .unwrap();
    let milestone = b.request(json!({"command":"poll"}));
    assert_eq!(
        milestone["snapshot"]["shared"]["milestones"][0]["title"],
        "Working demo"
    );
    assert!(a.execute(Action::MilestoneDone {
        id: "M1".into(),
        done: true
    })["rejection"]
        .is_string());
    host.control(Action::MilestoneDone {
        id: "M1".into(),
        done: true,
    })
    .unwrap();
    assert_eq!(
        a.request(json!({"command":"poll"}))["snapshot"]["shared"]["milestones"][0]["done"],
        true
    );
    let private = "private-content-must-never-reach-third-client";
    let sent = a.execute(Action::Message {
        text: private.into(),
        recipient: Some(b_id.clone()),
        task: Some("T1".into()),
        decision: true,
        parent: None,
    });
    let parent = sent["snapshot"]["events"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["sequence"]
        .as_u64()
        .unwrap();
    let received = b.request(json!({"command":"poll"}));
    assert!(received.to_string().contains(private));
    assert!(b.execute(Action::Message {
        text: "private reply".into(),
        recipient: Some(a_id.clone()),
        task: None,
        decision: false,
        parent: Some(parent)
    })["rejection"]
        .is_null());
    let hidden = outsider.request(json!({"command":"poll"}));
    assert!(!hidden.to_string().contains(private));
    assert!(!hidden.to_string().contains("private reply"));
    assert!(!serde_json::to_string(&host.snapshot().unwrap())
        .unwrap()
        .contains(private));
    let denied = outsider.execute(Action::Message {
        text: "attempt to quote private thread".into(),
        recipient: None,
        task: None,
        decision: false,
        parent: Some(parent),
    });
    assert!(denied["rejection"].is_string());
    let mut late = Peer::start();
    let late_join=late.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Late guest"}));
    assert!(!late_join.to_string().contains(private));
    assert_eq!(
        late_join["snapshot"]["shared"]["tasks"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    host.revoke(&b_invite).unwrap();
    assert!(b.execute(Action::Due {
        task: "T2".into(),
        hours: Some(1)
    })["error"]
        .is_string());
    assert!(a.execute(Action::Message {
        text: "revoked recipient".into(),
        recipient: Some(b_id),
        task: None,
        decision: false,
        parent: None
    })["rejection"]
        .is_string());
    assert!(
        a.request(json!({"command":"post","text":"public after a rejected private send"}))["error"]
            .is_null()
    );
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn saved_patches_are_recipient_scoped_filtered_and_revocable() {
    use forge_room_core::transport::Action;
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "patch trial").unwrap();
    let (mut a, mut b, mut c) = (Peer::start(), Peer::start(), Peer::start());
    let invite_a = host.invite(Duration::from_secs(60)).unwrap();
    a.request(json!({"command":"join","invite":invite_a.encode(),"name":"Sender"}));
    let joined=b.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Recipient"}));
    let recipient = joined["identity"].as_str().unwrap().to_string();
    c.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Unrelated"}));
    let share = |path: &str, after: &str| Action::SharePatch {
        recipient: recipient.clone(),
        path: path.into(),
        before: "old content\n".into(),
        after: after.into(),
    };
    assert!(a.execute(share(".env", "secret value"))["rejection"].is_string());
    assert!(a.execute(share("../outside.txt", "must be denied"))["rejection"].is_string());
    assert!(a.execute(share(
        "code.txt",
        "-----BEGIN PRIVATE KEY-----\nprivate\n-----END PRIVATE KEY-----"
    ))["rejection"]
        .is_string());
    assert!(a.execute(share("code.txt", "api_key = \"abcdef123456789\""))["rejection"].is_string());
    let sent = a.execute(share(
        "private-patch-path.txt",
        "recipient-only saved replacement\n",
    ));
    assert!(sent["rejection"].is_null());
    let id = sent["snapshot"]["patches"][0]["id"].as_u64().unwrap();
    let received = b.request(json!({"command":"poll"}));
    assert_eq!(
        received["snapshot"]["patches"][0]["after"],
        "recipient-only saved replacement\n"
    );
    let unrelated = c.request(json!({"command":"poll"}));
    assert!(unrelated["snapshot"]["patches"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!unrelated.to_string().contains("private-patch-path.txt"));
    assert!(!unrelated
        .to_string()
        .contains("recipient-only saved replacement"));
    assert!(host.snapshot().unwrap().patches.is_empty());
    assert!(c.execute(Action::RevokePatch { id })["rejection"].is_string());
    assert!(a.execute(Action::RevokePatch { id })["rejection"].is_null());
    assert!(b.request(json!({"command":"poll"}))["snapshot"]["patches"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(
        a.execute(share("private-patch-path.txt", "second authorized patch\n"))["rejection"]
            .is_null()
    );
    host.revoke(&invite_a).unwrap();
    assert!(b.request(json!({"command":"poll"}))["snapshot"]["patches"]
        .as_array()
        .unwrap()
        .is_empty());
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
