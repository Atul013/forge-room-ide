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

#[test]
fn killed_host_recovers_acknowledged_public_work_from_disk() {
    let path = std::env::temp_dir().join(format!(
        "forge-killed-journal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut server = Peer::start();
    let hosted = server.request(json!({"command":"host_journal","path":path}));
    let mut owner = Peer::start();
    let joined = owner.request(json!({"command":"join","invite":hosted["invite"],"name":"Ada"}));
    let identity = joined["identity"].as_str().unwrap().to_owned();
    let ack = owner.request(json!({"command":"post","text":"saved before process kill"}));
    assert!(ack["error"].is_null());
    server.child.kill().unwrap();
    server.child.wait().unwrap();
    let host = Host::recover_journal("127.0.0.1:0".parse().unwrap(), &path).unwrap();
    assert!(host
        .snapshot()
        .unwrap()
        .events
        .iter()
        .any(|event| event.text == "saved before process kill"));
    assert!(host
        .recovered_members()
        .unwrap()
        .iter()
        .any(|member| member.id == identity));
    assert!(
        host.resume().is_err(),
        "fresh journal required after recovery"
    );
    let next = path.with_extension("next");
    host.enable_journal(&next).unwrap();
    host.resume().unwrap();
    let mut returning = Peer::start();
    let joined = returning.request(json!({"command":"join","invite":host.reclaim_invite(&identity,Duration::from_secs(60)).unwrap().encode(),"name":"Ada"}));
    assert_eq!(joined["identity"], identity);
    assert!(
        returning.request(json!({"command":"post","text":"saved after recovery"}))["error"]
            .is_null()
    );
    drop(host);
    let host = Host::recover_journal("127.0.0.1:0".parse().unwrap(), &next).unwrap();
    assert!(host
        .snapshot()
        .unwrap()
        .events
        .iter()
        .any(|event| event.text == "saved after recovery"));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(next).unwrap();
}

#[test]
fn checkpoint_restart_requires_fresh_claims_and_preserves_owner_work() {
    use forge_room_core::{
        transport::{Action, RoomLifecycle},
        TaskPriority, TaskStatus,
    };
    let checkpoint = std::env::temp_dir().join(format!(
        "forge-room-recovery-{}-{}.json",
        std::process::id(),
        forge_room_core::now_epoch_seconds()
    ));
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "recovery trial").unwrap();
    let address = host.address();
    let (mut owner, mut other) = (Peer::start(), Peer::start());
    let old_invite = host.invite(Duration::from_secs(60)).unwrap();
    let joined = owner.request(json!({"command":"join","invite":old_invite.encode(),"name":"Ada"}));
    let owner_id = joined["identity"].as_str().unwrap().to_owned();
    let joined = other.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Grace"}));
    let other_id = joined["identity"].as_str().unwrap().to_owned();
    host.control(Action::CreateTask {
        title: "Recover owner work".into(),
        owner: owner_id.clone(),
        criteria: vec![],
        dependencies: vec![],
        priority: TaskPriority::High,
        due_hours: None,
    })
    .unwrap();
    assert!(owner.execute(Action::message("public checkpoint message"))["rejection"].is_null());
    assert!(owner.execute(Action::Message {
        text: "private must not persist".into(),
        recipient: Some(other_id.clone()),
        task: None,
        decision: false,
        parent: None
    })["rejection"]
        .is_null());
    assert!(owner.execute(Action::SharePatch {
        recipient: other_id,
        path: "src/main.rs".into(),
        before: "private original".into(),
        after: "private replacement".into()
    })["rejection"]
        .is_null());
    host.control(Action::Countdown {
        seconds: Some(180),
        running: true,
    })
    .unwrap();
    host.save_checkpoint(&checkpoint).unwrap();
    let saved = std::fs::read(&checkpoint).unwrap();
    assert!(!String::from_utf8_lossy(&saved).contains("private must not persist"));
    assert!(!String::from_utf8_lossy(&saved).contains("private original"));
    assert!(!String::from_utf8_lossy(&saved).contains("private replacement"));
    assert!(!String::from_utf8_lossy(&saved).contains(&old_invite.encode()));
    assert!(host.save_checkpoint(&checkpoint).is_err());
    assert_eq!(std::fs::read(&checkpoint).unwrap(), saved);
    drop(host);
    let host = Host::recover(address, &checkpoint).unwrap();
    assert_eq!(host.lifecycle().unwrap(), RoomLifecycle::Paused);
    assert!(host.invite(Duration::from_secs(60)).is_err());
    let snapshot = host.snapshot().unwrap();
    assert_eq!(snapshot.shared.tasks[0].owner_id, owner_id);
    assert!(!snapshot.shared.countdown.running);
    assert!(snapshot.members.iter().all(|member| !member.online));
    host.resume().unwrap();
    let (mut impostor, mut reclaimed) = (Peer::start(), Peer::start());
    let joined = impostor.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Ada"}));
    assert_ne!(joined["identity"], owner_id);
    assert!(impostor.execute(Action::Transition {
        task: "T1".into(),
        status: TaskStatus::InProgress,
        completed: vec![]
    })["rejection"]
        .is_string());
    let stale = host
        .reclaim_invite(&owner_id, Duration::from_secs(60))
        .unwrap();
    let claim = host
        .reclaim_invite(&owner_id, Duration::from_secs(60))
        .unwrap();
    assert!(
        reclaimed.request(json!({"command":"join","invite":stale.encode(),"name":"Ada"}))["error"]
            .is_string()
    );
    let joined =
        reclaimed.request(json!({"command":"join","invite":claim.encode(),"name":"Changed name"}));
    assert_eq!(joined["identity"], owner_id);
    assert!(reclaimed.execute(Action::Transition {
        task: "T1".into(),
        status: TaskStatus::InProgress,
        completed: vec![]
    })["rejection"]
        .is_null());
    assert!(host
        .reclaim_invite(&owner_id, Duration::from_secs(60))
        .is_err());
    assert!(owner.request(json!({"command":"poll"}))["error"].is_string());
    host.revoke(&claim).unwrap();
    assert!(reclaimed.request(json!({"command":"poll"}))["error"].is_string());
    drop(host);
    std::fs::remove_file(checkpoint).unwrap();
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

#[test]
fn independent_clients_opt_in_to_read_only_task_focus_and_clear_it_on_optout_or_leave() {
    use forge_room_core::{transport::Action, TaskPriority};
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "task focus trial").unwrap();
    let (mut owner, mut viewer) = (Peer::start(), Peer::start());
    let owner_join = owner.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Ada"}));
    let owner_id = owner_join["identity"].as_str().unwrap().to_owned();
    viewer.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Grace"}));
    assert!(owner.execute(Action::CreateTask {
        title: "Review API contract".into(),
        owner: owner_id.clone(),
        criteria: vec![],
        dependencies: vec![],
        priority: TaskPriority::Normal,
        due_hours: Some(1),
    })["rejection"]
        .is_null());
    assert!(owner.execute(Action::ShareFocus {
        task: Some("T1".into())
    })["rejection"]
        .is_null());
    let followed = viewer.request(json!({"command":"poll"}));
    assert_eq!(
        followed["snapshot"]["shared"]["task_focus"][0]["actor"],
        owner_id
    );
    assert_eq!(
        followed["snapshot"]["shared"]["task_focus"][0]["task"],
        "T1"
    );
    let encoded = serde_json::to_string(&followed["snapshot"]["shared"]["task_focus"]).unwrap();
    assert!(
        !encoded.contains("path") && !encoded.contains("buffer") && !encoded.contains("cursor")
    );
    assert!(owner.execute(Action::ShareFocus {
        task: Some("T404".into())
    })["rejection"]
        .is_string());
    assert!(owner.execute(Action::ShareFocus { task: None })["rejection"].is_null());
    assert!(
        viewer.request(json!({"command":"poll"}))["snapshot"]["shared"]["task_focus"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(owner.execute(Action::ShareFocus {
        task: Some("T1".into())
    })["rejection"]
        .is_null());
    assert!(owner.request(json!({"command":"leave"}))["left"]
        .as_bool()
        .unwrap());
    assert!(
        viewer.request(json!({"command":"poll"}))["snapshot"]["shared"]["task_focus"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn independent_clients_explicitly_share_only_saved_code_location_and_clear_it() {
    use forge_room_core::transport::Action;
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "code location trial").unwrap();
    let (mut owner, mut viewer, mut outsider) = (Peer::start(), Peer::start(), Peer::start());
    let joined = owner.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Ada"}));
    let owner_id = joined["identity"].as_str().unwrap().to_owned();
    let viewer_join = viewer.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Grace"}));
    let viewer_id = viewer_join["identity"].as_str().unwrap().to_owned();
    outsider.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Lin"}));

    assert!(owner.execute(Action::ShareLocation {
        path: Some("src/main.rs".into()),
        line: Some(42),
    })["rejection"]
        .is_null());
    let received = viewer.request(json!({"command":"poll"}));
    let location = &received["snapshot"]["shared"]["code_locations"][0];
    assert_eq!(location["actor"], owner_id);
    assert_eq!(location["path"], "src/main.rs");
    assert_eq!(location["line"], 42);
    let encoded = serde_json::to_string(&received["snapshot"]["shared"]["code_locations"]).unwrap();
    assert!(
        !encoded.contains("buffer")
            && !encoded.contains("selection")
            && !encoded.contains("content")
    );
    assert_eq!(
        outsider.request(json!({"command":"poll"}))["snapshot"]["shared"]["code_locations"][0]
            ["path"],
        "src/main.rs"
    );
    let sent = owner.execute(Action::ContextMessage {
        text: "Review this private line".into(),
        recipient: Some(viewer_id),
        task: None,
        decision: false,
        parent: None,
        path: "src/private.rs".into(),
        line: 23,
    });
    assert!(sent["rejection"].is_null());
    let owner_events = sent["snapshot"]["events"].clone();
    let viewer_events = viewer.request(json!({"command":"poll"}))["snapshot"]["events"].clone();
    let outsider_events = outsider.request(json!({"command":"poll"}))["snapshot"]["events"].clone();
    assert_eq!(
        owner_events[0]["code_reference"]["path"], "src/private.rs",
        "{owner_events}"
    );
    assert_eq!(viewer_events[0]["code_reference"]["line"], 23);
    assert!(outsider_events.as_array().unwrap().is_empty());
    assert!(owner_events[0]["text"]
        .as_str()
        .unwrap()
        .contains("private line"));
    let public = owner.execute(Action::ContextMessage {
        text: "Review the parser entry".into(),
        recipient: None,
        task: None,
        decision: false,
        parent: None,
        path: "src/parser.rs".into(),
        line: 8,
    });
    assert!(public["rejection"].is_null());
    let public_view = outsider.request(json!({"command":"poll"}));
    assert_eq!(
        public_view["snapshot"]["events"][0]["code_reference"]["path"],
        "src/parser.rs"
    );
    assert!(owner.execute(Action::ContextMessage {
        text: "unsafe reference".into(),
        recipient: None,
        task: None,
        decision: false,
        parent: None,
        path: "../outside.rs".into(),
        line: 1,
    })["rejection"]
        .is_string());

    for (path, line) in [
        ("../secret.rs", 1),
        (".env", 1),
        ("src\\main.rs", 1),
        ("src/main.rs", 0),
        ("src/main.rs", 10_000_001),
    ] {
        assert!(owner.execute(Action::ShareLocation {
            path: Some(path.into()),
            line: Some(line),
        })["rejection"]
            .is_string());
    }
    assert!(owner.execute(Action::ShareLocation {
        path: None,
        line: Some(3)
    })["rejection"]
        .is_string());
    assert!(owner.execute(Action::ShareLocation {
        path: None,
        line: None
    })["rejection"]
        .is_null());
    assert!(
        viewer.request(json!({"command":"poll"}))["snapshot"]["shared"]["code_locations"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    assert!(owner.execute(Action::ShareLocation {
        path: Some("src/main.rs".into()),
        line: Some(7),
    })["rejection"]
        .is_null());
    assert!(owner.request(json!({"command":"leave"}))["left"]
        .as_bool()
        .unwrap());
    assert!(
        viewer.request(json!({"command":"poll"}))["snapshot"]["shared"]["code_locations"]
            .as_array()
            .unwrap()
            .is_empty()
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
    assert_eq!(received["snapshot"]["patches"][0]["review"], "pending");
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
    use forge_room_core::transport::PatchReview;
    assert!(a.execute(Action::ReviewPatch {
        id,
        decision: PatchReview::Reviewed
    })["rejection"]
        .is_string());
    assert!(c.execute(Action::ReviewPatch {
        id,
        decision: PatchReview::Declined
    })["rejection"]
        .is_string());
    assert!(b.execute(Action::ReviewPatch {
        id,
        decision: PatchReview::Reviewed
    })["rejection"]
        .is_null());
    assert_eq!(
        a.request(json!({"command":"poll"}))["snapshot"]["patches"][0]["review"],
        "reviewed"
    );
    assert!(c.request(json!({"command":"poll"}))["snapshot"]["patches"]
        .as_array()
        .unwrap()
        .is_empty());
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
    let owner = joined_a["identity"].as_str().unwrap().to_owned();
    assert!(a.execute(forge_room_core::transport::Action::CreateTask {
        title: "Reconnect readiness".into(),
        owner,
        criteria: vec![],
        dependencies: vec![],
        priority: forge_room_core::TaskPriority::High,
        due_hours: Some(1)
    })["rejection"]
        .is_null());
    assert!(
        a.execute(forge_room_core::transport::Action::UpdateReadiness {
            task: "T1".into(),
            expected_revision: 0,
            profile: forge_room_core::transport::ReadinessProfile {
                estimate_minutes: Some(10),
                help_allowed: true,
                ..Default::default()
            }
        })["rejection"]
            .is_null()
    );
    dropping.store(true, Ordering::SeqCst);
    assert!(b.request(json!({"command":"poll"}))["error"].is_string());
    assert!(
        a.execute(forge_room_core::transport::Action::UpdateReadiness {
            task: "T1".into(),
            expected_revision: 1,
            profile: forge_room_core::transport::ReadinessProfile {
                estimate_minutes: Some(20),
                blocker: "Updated while peer offline".into(),
                help_allowed: true,
                ..Default::default()
            }
        })["rejection"]
            .is_null()
    );
    assert!(a.execute(forge_room_core::transport::Action::RequestHelp {
        task: "T1".into(),
        expected_revision: 2,
        reason: "Review after reconnect".into()
    })["rejection"]
        .is_null());
    a.request(json!({"command":"post","text":"committed while B was offline"}));
    dropping.store(false, Ordering::SeqCst);
    let recovered = b.request(json!({"command":"poll"}));
    assert_eq!(
        recovered["snapshot"]["events"][0]["text"],
        "committed while B was offline"
    );
    assert_eq!(
        recovered["snapshot"]["shared"]["readiness"]["profiles"][0]["revision"],
        2
    );
    assert_eq!(
        recovered["snapshot"]["shared"]["readiness"]["help"][0]["reason"],
        "Review after reconnect"
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

#[test]
fn independent_owners_coordinate_readiness_help_and_dependency_impacts() {
    use forge_room_core::{
        transport::{Action, Declaration, ReadinessProfile},
        TaskPriority,
    };
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "readiness workflow").unwrap();
    let (mut a, mut b) = (Peer::start(), Peer::start());
    let aj=a.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Ada"}));
    let bj=b.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Grace"}));
    let aid = aj["identity"].as_str().unwrap().to_string();
    let bid = bj["identity"].as_str().unwrap().to_string();
    let create = |title: &str, owner: String, dependencies: Vec<String>| Action::CreateTask {
        title: title.into(),
        owner,
        criteria: vec![],
        dependencies,
        priority: TaskPriority::High,
        due_hours: Some(1),
    };
    assert!(a.execute(create("API", aid.clone(), vec![]))["rejection"].is_null());
    assert!(b.execute(create("UI", bid.clone(), vec!["T1".into()]))["rejection"].is_null());
    let profile = ReadinessProfile {
        estimate_minutes: Some(45),
        blocker: "Need contract review".into(),
        files: vec!["src/api.rs".into()],
        checks: Declaration::ClaimedFail,
        help_allowed: true,
        ..Default::default()
    };
    let publish = Action::UpdateReadiness {
        task: "T1".into(),
        expected_revision: 0,
        profile: profile.clone(),
    };
    assert!(b.execute(publish.clone())["rejection"].is_string());
    assert!(a.execute(publish.clone())["rejection"].is_null());
    assert!(a.execute(publish)["rejection"].is_string());
    assert!(a.execute(Action::RequestHelp {
        task: "T1".into(),
        expected_revision: 1,
        reason: "Review the API contract".into()
    })["rejection"]
        .is_null());
    let shared = b.request(json!({"command":"poll"}));
    assert_eq!(
        shared["snapshot"]["shared"]["readiness"]["help"][0]["reason"],
        "Review the API contract"
    );
    assert_eq!(shared["snapshot"]["shared"]["tasks"][0]["ownerId"], aid);
    assert!(a.execute(Action::DeclareImpact {
        revision: "a".repeat(40),
        source: "T1".into(),
        target: "T2".into(),
        file: "src/api.rs".into(),
        summary: "Response schema changed; UI mapping needs review".into()
    })["rejection"]
        .is_null());
    assert!(a.execute(Action::AcknowledgeImpact {
        id: 1,
        accepted: true
    })["rejection"]
        .is_string());
    assert!(b.execute(Action::AcknowledgeImpact {
        id: 1,
        accepted: true
    })["rejection"]
        .is_null());
    let mut opted_out = profile;
    opted_out.help_allowed = false;
    assert!(a.execute(Action::UpdateReadiness {
        task: "T1".into(),
        expected_revision: 1,
        profile: opted_out
    })["rejection"]
        .is_null());
    let shared = b.request(json!({"command":"poll"}));
    let r = &shared["snapshot"]["shared"]["readiness"];
    assert_eq!(r["profiles"][0]["revision"], 2);
    assert_eq!(r["help"][0]["closed"], true);
    assert_eq!(r["impacts"][0]["acknowledged_by"], bid);
    assert_eq!(shared["snapshot"]["shared"]["tasks"][0]["status"], "open");
}

#[test]
fn independent_clients_observe_host_pause_resume_archive_and_public_export_privacy() {
    use forge_room_core::transport::{Action, RoomLifecycle};
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "lifecycle trial").unwrap();
    let (mut a, mut b, mut late) = (Peer::start(), Peer::start(), Peer::start());
    a.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Ada"}));
    let b_invite = host.invite(Duration::from_secs(60)).unwrap();
    let b_join = b.request(json!({"command":"join","invite":b_invite.encode(),"name":"Grace"}));
    let late_invite = host.invite(Duration::from_secs(60)).unwrap();
    let b_id = b_join["identity"].as_str().unwrap().to_owned();
    assert!(a.execute(Action::SharePatch {
        recipient: b_id.clone(),
        path: "src/lib.rs".into(),
        before: "old source".into(),
        after: "new source".into(),
    })["rejection"]
        .is_null());

    assert!(a.execute(Action::Message {
        text: "public recap".into(),
        recipient: None,
        task: None,
        decision: false,
        parent: None
    })["rejection"]
        .is_null());
    assert!(a.execute(Action::Message {
        text: "private note".into(),
        recipient: Some(b_id),
        task: None,
        decision: false,
        parent: None
    })["rejection"]
        .is_null());
    let archive = serde_json::to_value(host.snapshot().unwrap()).unwrap();
    let exported = archive["events"].as_array().unwrap();
    assert_eq!(
        exported.len(),
        1,
        "public host archive must omit direct messages"
    );
    assert_eq!(exported[0]["text"], "public recap");
    assert!(archive["patches"].as_array().unwrap().is_empty());
    assert_eq!(
        b.request(json!({"command":"poll"}))["snapshot"]["patches"][0]["after"],
        "new source"
    );

    host.pause().unwrap();
    assert_eq!(host.lifecycle().unwrap(), RoomLifecycle::Paused);
    assert!(host.invite(Duration::from_secs(60)).is_err());
    assert!(host
        .control(Action::Countdown {
            seconds: Some(30),
            running: true
        })
        .is_err());
    assert!(
        late.request(json!({"command":"join","invite":late_invite.encode(),"name":"Late"}))
            ["error"]
            .is_string()
    );
    let paused_a = a.request(json!({"command":"poll"}));
    let paused_b = b.request(json!({"command":"poll"}));
    assert_eq!(paused_a["snapshot"]["lifecycle"], "paused");
    assert_eq!(paused_b["snapshot"]["lifecycle"], "paused");
    assert!(b.execute(Action::Message {
        text: "blocked while paused".into(),
        recipient: None,
        task: None,
        decision: false,
        parent: None
    })["rejection"]
        .as_str()
        .unwrap()
        .contains("paused"));

    host.resume().unwrap();
    assert!(a.execute(Action::Message {
        text: "resumed".into(),
        recipient: None,
        task: None,
        decision: false,
        parent: None
    })["rejection"]
        .is_null());
    host.archive().unwrap();
    assert_eq!(host.lifecycle().unwrap(), RoomLifecycle::Archived);
    assert!(host.resume().is_err(), "archive is final");
    assert!(host
        .control(Action::Countdown {
            seconds: Some(30),
            running: true
        })
        .is_err());
    assert!(b.execute(Action::Message {
        text: "blocked after archive".into(),
        recipient: None,
        task: None,
        decision: false,
        parent: None
    })["rejection"]
        .as_str()
        .unwrap()
        .contains("archived"));
    assert_eq!(
        a.request(json!({"command":"poll"}))["snapshot"]["lifecycle"],
        "archived"
    );
    assert!(
        late.request(json!({"command":"join","invite":late_invite.encode(),"name":"Late"}))
            ["error"]
            .is_string()
    );
    let final_archive = serde_json::to_value(host.snapshot().unwrap()).unwrap();
    assert_eq!(final_archive["lifecycle"], "archived");
    assert!(final_archive["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|event| event["text"] != "private note"));
}

#[test]
fn independent_clients_share_validated_mentions_and_visibility_scoped_reactions() {
    use forge_room_core::transport::Action;
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "conversation trial").unwrap();
    let (mut a, mut b, mut outsider) = (Peer::start(), Peer::start(), Peer::start());
    let aj = a.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Ada"}));
    let bj = b.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Grace"}));
    outsider.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Lin"}));
    let aid = aj["identity"].as_str().unwrap().to_owned();
    let bid = bj["identity"].as_str().unwrap().to_owned();

    let mentioned = a.execute(Action::MentionMessage {
        text: "Review the updated contract".into(),
        mentions: vec![bid.clone()],
        task: None,
        decision: false,
        parent: None,
    });
    assert!(mentioned["rejection"].is_null());
    assert_eq!(mentioned["snapshot"]["events"][0]["mentions"][0], bid);
    assert!(
        b.request(json!({"command":"poll"}))["snapshot"]["events"][0]["mentions"][0].is_string()
    );
    assert!(
        outsider.request(json!({"command":"poll"}))["snapshot"]["events"][0]["mentions"][0]
            .is_string()
    );

    for mentions in [
        vec!["f".repeat(64)],
        vec![bid.clone(), bid.clone()],
        vec![aid.clone()],
    ] {
        assert!(a.execute(Action::MentionMessage {
            text: "invalid mention list".into(),
            mentions,
            task: None,
            decision: false,
            parent: None
        })["rejection"]
            .is_string());
    }
    assert!(a.execute(Action::MentionMessage {
        text: "too many mentions".into(),
        mentions: vec![bid.clone(); 9],
        task: None,
        decision: false,
        parent: None
    })["rejection"]
        .is_string());

    assert!(a.execute(Action::Message {
        text: "private detail".into(),
        recipient: Some(bid.clone()),
        task: None,
        decision: false,
        parent: None
    })["rejection"]
        .is_null());
    assert!(b.execute(Action::React {
        sequence: 2,
        emoji: "🤖".into(),
        active: true
    })["rejection"]
        .is_string());
    assert!(outsider.execute(Action::React {
        sequence: 2,
        emoji: "👀".into(),
        active: true
    })["rejection"]
        .as_str()
        .unwrap()
        .contains("not visible"));
    assert!(b.execute(Action::React {
        sequence: 2,
        emoji: "👀".into(),
        active: true
    })["rejection"]
        .is_null());
    let participant_view = a.request(json!({"command":"poll"}));
    assert_eq!(
        participant_view["snapshot"]["reactions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(participant_view["snapshot"]["reactions"][0]["sequence"], 2);
    let outsider_view = outsider.request(json!({"command":"poll"}));
    assert!(outsider_view["snapshot"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|event| event["text"] != "private detail"));
    assert!(outsider_view["snapshot"]["reactions"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(
        serde_json::to_value(host.snapshot().unwrap()).unwrap()["reactions"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    assert!(b.execute(Action::React {
        sequence: 1,
        emoji: "✅".into(),
        active: true
    })["rejection"]
        .is_null());
    let public_view = outsider.request(json!({"command":"poll"}));
    assert!(public_view["snapshot"]["reactions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reaction| reaction["sequence"] == 1 && reaction["actor"] == bid));
    assert!(b.execute(Action::React {
        sequence: 2,
        emoji: "👀".into(),
        active: false
    })["rejection"]
        .is_null());
    assert!(
        a.request(json!({"command":"poll"}))["snapshot"]["reactions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|reaction| reaction["sequence"] != 2)
    );
}

#[test]
fn independent_clients_ship_scope_checks_and_private_export_boundary() {
    use forge_room_core::{
        transport::{
            ship_blockers, ship_handoff, Action, ShipCheck, ShipPlan, ShipResult, Snapshot,
        },
        TaskPriority,
    };
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "ship workflow").unwrap();
    let (mut a, mut b) = (Peer::start(), Peer::start());
    let joined = a.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Owner"}));
    let owner = joined["identity"].as_str().unwrap().to_string();
    let joined = b.request(json!({"command":"join","invite":host.invite(Duration::from_secs(60)).unwrap().encode(),"name":"Reviewer"}));
    let reviewer = joined["identity"].as_str().unwrap().to_string();
    host.control(Action::CreateTask {
        title: "Release deliverable".into(),
        owner: owner.clone(),
        criteria: vec!["Reviewed".into()],
        dependencies: vec![],
        priority: TaskPriority::High,
        due_hours: None,
    })
    .unwrap();
    let task = host.snapshot().unwrap().shared.tasks[0].id.clone();
    let plan = ShipPlan {
        title: "Demo release".into(),
        required_tasks: vec![task],
        checks: vec![ShipCheck {
            title: "Smoke test".into(),
            owner: owner.clone(),
        }],
        preview_reference: "Local preview build 12".into(),
        submission_reference: String::new(),
    };
    let edit = Action::SetShipPlan {
        expected_revision: 0,
        plan: plan.clone(),
    };
    assert!(a.execute(edit.clone())["rejection"].is_string());
    host.control(edit).unwrap();
    let report = Action::ReportShipCheck {
        expected_revision: 1,
        check: 0,
        result: ShipResult::ClaimedPass,
        note: "Manual smoke test passed".into(),
    };
    assert!(b.execute(report.clone())["rejection"].is_string());
    assert!(a.execute(report.clone())["rejection"].is_null());
    assert!(a.execute(report)["rejection"].is_string());
    let other = b.request(json!({"command":"poll"}));
    assert_eq!(
        other["snapshot"]["shared"]["shipping"]["reports"][0]["result"],
        "ClaimedPass"
    );
    let private = a.execute(Action::Message {
        text: "PRIVATE_SHIP_SENTINEL".into(),
        recipient: Some(reviewer.clone()),
        task: None,
        decision: false,
        parent: None,
    });
    let snapshot: Snapshot = serde_json::from_value(private["snapshot"].clone()).unwrap();
    assert!(snapshot
        .events
        .iter()
        .any(|event| event.text == "PRIVATE_SHIP_SENTINEL"));
    let exported = ship_handoff(&snapshot).unwrap();
    assert!(!exported.contains("PRIVATE_SHIP_SENTINEL"));
    assert!(exported.contains("Manual smoke test passed"));
    assert!(ship_blockers(&snapshot)
        .iter()
        .any(|issue| issue.contains("Submission")));
    assert!(ship_blockers(&snapshot)
        .iter()
        .any(|issue| issue.contains("required task")));
    let patch = a.execute(Action::SharePatch {
        recipient: reviewer,
        path: "private.txt".into(),
        before: "PRIVATE_PATCH_BEFORE".into(),
        after: "PRIVATE_PATCH_AFTER".into(),
    });
    assert!(patch["rejection"].is_null());
    let snapshot: Snapshot = serde_json::from_value(patch["snapshot"].clone()).unwrap();
    assert!(!snapshot.patches.is_empty());
    let exported = ship_handoff(&snapshot).unwrap();
    assert!(!exported.contains("PRIVATE_PATCH"));
    assert!(!exported.contains("private.txt"));
    host.pause().unwrap();
    assert!(a.execute(Action::ReportShipCheck {
        expected_revision: 2,
        check: 0,
        result: ShipResult::ClaimedFail,
        note: "Changed".into()
    })["rejection"]
        .is_string());
    host.resume().unwrap();
    host.control(Action::SetShipPlan {
        expected_revision: 2,
        plan,
    })
    .unwrap();
    let refreshed = a.request(json!({"command":"poll"}));
    assert_eq!(refreshed["snapshot"]["shared"]["shipping"]["revision"], 3);
    assert_eq!(
        refreshed["snapshot"]["shared"]["shipping"]["reports"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}
