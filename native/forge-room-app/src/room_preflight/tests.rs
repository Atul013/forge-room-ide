use super::*;
use forge_room_core::{
    Task, TaskPriority,
    transport::{ReadinessProfile, TaskReadiness},
};
struct Fixture {
    root: PathBuf,
    snapshot: Snapshot,
}
impl Fixture {
    fn new() -> Self {
        static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "forge-preflight-{}-{}",
            std::process::id(),
            ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='room-preflight-fixture'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"),"#[deprecated(note=\"LOCAL_PRIVATE_RECEIPT_MARKER\")] fn old() {} pub fn current(){old();}\n").unwrap();
        let actor = "a".repeat(64);
        let mut snapshot = Snapshot {
            you: actor.clone(),
            ..Default::default()
        };
        snapshot.shared.tasks.push(Task {
            id: "T1".into(),
            title: "Fixture".into(),
            owner_id: actor.clone(),
            owner_name: "Owner".into(),
            status: TaskStatus::Open,
            priority: TaskPriority::Normal,
            dependencies: vec![],
            acceptance_criteria: vec![],
            completed_criteria: vec![],
            worktree_branch: None,
            due_at_epoch_seconds: None,
            updated_at_epoch_seconds: 0,
        });
        snapshot.shared.readiness.profiles.push(TaskReadiness {
            task: "T1".into(),
            revision: 1,
            owner: actor,
            updated: 1,
            profile: ReadinessProfile {
                files: vec!["src/lib.rs".into()],
                ..Default::default()
            },
        });
        Self { root, snapshot }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
#[test]
fn real_preflight_receipt_is_status_only_and_revoked_by_revision_change() {
    let mut f = Fixture::new();
    let mut state = State::default();
    state.prepare(&f.root, &f.snapshot, "T1").unwrap();
    assert!(state.run.is_none());
    assert!(
        state
            .plan
            .as_ref()
            .unwrap()
            .display()
            .contains("--all-targets")
    );
    state
        .start(&f.root, &f.snapshot, "T1", &egui::Context::default())
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while state.run.is_some() {
        state.reconcile(Some(&f.root), &f.snapshot);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let receipt = state.receipt.as_ref().unwrap();
    assert_eq!(receipt.evidence.exit_code, Some(0));
    assert!(receipt.unchanged);
    assert!(
        receipt
            .evidence
            .stderr
            .contains("LOCAL_PRIVATE_RECEIPT_MARKER")
    );
    let shared = summary("T1", 1, receipt);
    assert!(!shared.contains("LOCAL_PRIVATE_RECEIPT_MARKER"));
    assert!(shared.contains("not host-verified"));
    assert!(shared.len() < 500);
    f.snapshot.shared.readiness.profiles[0].revision = 2;
    state.reconcile(Some(&f.root), &f.snapshot);
    assert!(state.receipt.is_none());
    assert!(state.scope.is_none());
}
#[test]
fn stale_files_and_foreign_owner_cannot_start_a_reviewed_check() {
    let mut f = Fixture::new();
    let mut state = State::default();
    state.prepare(&f.root, &f.snapshot, "T1").unwrap();
    std::fs::write(f.root.join("src/lib.rs"), "pub fn changed(){}\n").unwrap();
    assert!(
        state
            .start(&f.root, &f.snapshot, "T1", &egui::Context::default())
            .is_err()
    );
    assert!(state.run.is_none());
    f.snapshot.you = "b".repeat(64);
    assert!(state.prepare(&f.root, &f.snapshot, "T1").is_err());
    state.reconcile(Some(&f.root), &f.snapshot);
    assert!(state.plan.is_none());
    assert!(state.scope.is_none());
}
