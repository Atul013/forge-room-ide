use super::*;
use forge_room_core::{
    Task, TaskPriority, TaskStatus,
    transport::{ReadinessProfile, TaskReadiness},
};
fn task(id: &str, owner: &str, deps: Vec<String>) -> Task {
    Task {
        id: id.into(),
        title: id.into(),
        owner_id: owner.into(),
        owner_name: "Owner".into(),
        status: TaskStatus::Open,
        priority: TaskPriority::Normal,
        dependencies: deps,
        acceptance_criteria: vec![],
        completed_criteria: vec![],
        worktree_branch: None,
        due_at_epoch_seconds: None,
        updated_at_epoch_seconds: 0,
    }
}
#[test]
fn real_saved_diff_scan_routes_only_declared_dependencies_and_never_returns_diff_body() {
    let root = std::env::temp_dir().join(format!("forge-room-impact-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let git = |args: &[&str]| crate::git::run(&root, args).unwrap();
    git(&["init", "-q"]);
    git(&["config", "user.email", "fixture@example.test"]);
    git(&["config", "user.name", "Fixture"]);
    git(&["config", "core.autocrlf", "false"]);
    let file = root.join("api[1].rs");
    std::fs::write(&file, "baseline\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    let base = git(&["rev-parse", "HEAD"]);
    std::fs::write(&file, "PRIVATE_DIFF_CONTENT_NOT_FOR_NETWORK\n").unwrap();
    std::fs::write(root.join("untracked.txt"), "UNTRACKED_PRIVATE").unwrap();
    let before = git(&["status", "--porcelain"]);
    let owner = "a".repeat(64);
    let mut shared = SharedSnapshot {
        tasks: vec![
            task("T1", &owner, vec![]),
            task("T2", &"b".repeat(64), vec!["T1".into()]),
            task("T3", &"b".repeat(64), vec![]),
        ],
        ..Default::default()
    };
    shared.readiness.profiles.push(TaskReadiness {
        task: "T1".into(),
        revision: 1,
        owner: owner.clone(),
        updated: 1,
        profile: ReadinessProfile {
            files: vec!["api[1].rs".into()],
            ..Default::default()
        },
    });
    let result = scan(
        &root,
        &shared,
        &owner,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].target, "T2");
    assert_eq!(result.items[0].base, base.trim());
    assert!(!result.items[0].summary.contains("PRIVATE_DIFF_CONTENT"));
    assert!(!result.items[0].summary.contains("UNTRACKED_PRIVATE"));
    assert_eq!(git(&["status", "--porcelain"]), before);
    assert!(
        scan(
            &root,
            &shared,
            &owner,
            &std::sync::atomic::AtomicBool::new(true)
        )
        .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn late_scan_result_cannot_cross_root_or_identity() {
    let root = std::env::temp_dir().canonicalize().unwrap();
    let (tx, rx) = mpsc::channel();
    tx.send(Ok(ResultSet::default())).unwrap();
    let mut state = State {
        job: Some(Job {
            root,
            actor: "old".into(),
            room: "room".into(),
            rx,
            cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }),
        ..Default::default()
    };
    let snapshot = Snapshot {
        you: "new".into(),
        room: "room".into(),
        ..Default::default()
    };
    state.poll(Some(&std::env::temp_dir()), &snapshot);
    assert!(state.result.is_none());
    assert!(state.notice.contains("Discarded"));
}
