use super::*;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "forge-context-{}-{}",
            std::process::id(),
            ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn poll(state: &mut State, root: &Path, accept: bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while state.job.is_some() {
        state.poll(root, accept);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
#[test]
fn snapshot_inclusion_is_explicit_root_scoped_and_refresh_revokes_selection() {
    let root = Fixture::new();
    let other = Fixture::new();
    let mut state = State::default();
    state
        .capture(
            &root.0,
            Kind::Selection,
            "file.rs selected unsaved text".into(),
            "PRIVATE_DRAFT".into(),
            false,
        )
        .unwrap();
    assert_eq!(state.augment(&root.0, "task").unwrap(), "task");
    state.include(0, true).unwrap();
    let prompt = state.augment(&root.0, "task").unwrap();
    assert!(prompt.contains("PRIVATE_DRAFT"));
    assert!(prompt.contains("captured_epoch_seconds"));
    assert!(state.augment(&other.0, "task").is_err());
    assert!(
        state
            .capture(
                &other.0,
                Kind::Terminal,
                "wrong".into(),
                "other".into(),
                true
            )
            .is_err()
    );
    state
        .capture(
            &root.0,
            Kind::Selection,
            "file.rs selected unsaved text".into(),
            "NEW_DRAFT".into(),
            false,
        )
        .unwrap();
    assert_eq!(state.items.len(), 1);
    assert_eq!(state.augment(&root.0, "task").unwrap(), "task");
    state.clear();
    assert!(state.items.is_empty());
    assert!(state.root.is_none());
}
#[test]
fn truncation_is_utf8_safe_tail_preserving_and_total_bounded() {
    let root = Fixture::new();
    let mut state = State::default();
    let text = format!("{}TAIL", "😀".repeat(MAX_ITEM));
    for index in 0..4 {
        state
            .capture(
                &root.0,
                Kind::Terminal,
                format!("terminal{index}"),
                text.clone(),
                true,
            )
            .unwrap();
    }
    assert!(
        state.items.iter().all(|item| item.truncated
            && item.text.len() <= MAX_ITEM
            && item.text.ends_with("TAIL"))
    );
    assert!(
        state
            .capture(&root.0, Kind::Terminal, "overflow".into(), text, true)
            .is_err()
    );
    for item in &mut state.items {
        item.included = true;
    }
    let prompt = state.augment(&root.0, "task").unwrap();
    assert!(prompt.contains("truncated\":true"));
}
#[test]
fn pending_capture_is_discarded_after_scope_or_trust_changes() {
    let root = Fixture::new();
    let other = Fixture::new();
    let mut state = State::default();
    state
        .start(&root.0, |_| {
            Ok(ResultData::Snapshot(
                Kind::Terminal,
                "old".into(),
                "PRIVATE".into(),
                true,
            ))
        })
        .unwrap();
    poll(&mut state, &other.0, true);
    assert!(state.items.is_empty());
    state
        .start(&root.0, |_| {
            Ok(ResultData::Snapshot(
                Kind::Terminal,
                "old".into(),
                "PRIVATE".into(),
                true,
            ))
        })
        .unwrap();
    poll(&mut state, &root.0, false);
    assert!(state.items.is_empty());
}
#[test]
fn project_skills_are_discovered_without_executing_or_implicitly_including() {
    let root = Fixture::new();
    let dir = root.0.join(".agents/skills/checks");
    std::fs::create_dir_all(&dir).unwrap();
    let text = "---\nname: project-checks\ndescription: Inspect and validate\nlicense: MIT\n---\nUse scripts/run.ps1; network needs: none.\n";
    std::fs::write(dir.join("SKILL.md"), text).unwrap();
    std::fs::write(dir.join("run.ps1"), "throw 'MUST NOT EXECUTE'").unwrap();
    let mut state = State::default();
    state.discover(&root.0).unwrap();
    poll(&mut state, &root.0, true);
    assert_eq!(state.catalog.len(), 1);
    assert_eq!(state.catalog[0].name, "project-checks");
    assert_eq!(state.augment(&root.0, "task").unwrap(), "task");
    let source = crate::ai_workflow::source(&root.0, &state.catalog[0].path).unwrap();
    state
        .capture(&root.0, Kind::Skills, source.relative, source.before, false)
        .unwrap();
    assert_eq!(state.augment(&root.0, "task").unwrap(), "task");
    state.include(0, true).unwrap();
    let prompt = state.augment(&root.0, "task").unwrap();
    assert!(prompt.contains("license: MIT"));
    assert!(!prompt.contains("MUST NOT EXECUTE"));
    assert!(prompt.contains("cannot override app permissions"));
}
#[test]
fn git_context_is_literal_file_only_and_contains_both_layers() {
    let root = Fixture::new();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&root.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "fixture@example.test"]);
    git(&["config", "user.name", "Fixture"]);
    git(&["config", "core.autocrlf", "false"]);
    let file = root.0.join("file[1].txt");
    let other = root.0.join("file1.txt");
    std::fs::write(&file, "base\n").unwrap();
    std::fs::write(&other, "other base\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    std::fs::write(&file, "STAGED_CHANGE\n").unwrap();
    git(&["add", "--", "file[1].txt"]);
    std::fs::write(&file, "WORKING_CHANGE\n").unwrap();
    std::fs::write(&other, "OTHER_FILE_PRIVATE\n").unwrap();
    let mut state = State::default();
    state.start_diff(&root.0, &file).unwrap();
    poll(&mut state, &root.0, true);
    assert_eq!(state.items.len(), 1, "{}", state.notice);
    let text = &state.items[0].text;
    assert!(text.contains("STAGED_CHANGE"));
    assert!(text.contains("WORKING_CHANGE"));
    assert!(!text.contains("OTHER_FILE_PRIVATE"));
    std::fs::write(root.0.join(".env"), "KEY=secret").unwrap();
    assert!(state.start_diff(&root.0, &root.0.join(".env")).is_err());
}

#[test]
fn unicode_unsaved_selection_is_exact_and_does_not_read_outside_or_credentials() {
    let root = Fixture::new();
    let other = Fixture::new();
    let file = root.0.join("source.rs");
    std::fs::write(&file, "saved text").unwrap();
    let (title, text) = selection(&root.0, &file, "a\u{1f600}\u{4e2d}z", 1..3).unwrap();
    assert_eq!(text, "\u{1f600}\u{4e2d}");
    assert!(title.contains("may be unsaved"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "saved text");
    assert!(selection(&root.0, &file, "short", 0..20).is_err());
    let outside = other.0.join("outside.rs");
    std::fs::write(&outside, "outside").unwrap();
    assert!(selection(&root.0, &outside, "outside", 0..1).is_err());
    let secret = root.0.join(".env");
    std::fs::write(&secret, "KEY=value").unwrap();
    assert!(selection(&root.0, &secret, "KEY=value", 0..3).is_err());
}
