use super::*;
struct Fixture {
    root: PathBuf,
    state: State,
    config: Config,
    workflow: Workflow,
}
impl Fixture {
    fn new() -> Self {
        static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "forge-controlled-{}-{}",
            std::process::id(),
            ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("source.rs"), "PRIVATE_SOURCE_123").unwrap();
        let config = Config {
            model: "test".into(),
            ..Config::default()
        };
        let mut state = State::default();
        let source = ai_workflow::source(&root, &root.join("source.rs")).unwrap();
        let prompt = state
            .request(&root, &config, &[source], "Fix this file")
            .unwrap();
        assert!(!prompt.contains("PRIVATE_SOURCE_123"));
        Self {
            root,
            state,
            config,
            workflow: Workflow::default(),
        }
    }
    fn action(&mut self, value: Value) -> Result<String, String> {
        let r = self.state.reserve(&self.root, &self.config, 1000)?;
        self.state.commit(r);
        self.state.receive(
            &self.root,
            &self.config,
            &value.to_string(),
            &mut self.workflow,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.state.reset();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
#[test]
fn read_denial_and_stale_approval_never_deliver_contents() {
    let mut f = Fixture::new();
    f.action(json!({"action":"read_file","path":"source.rs"}))
        .unwrap();
    assert!(
        !f.state
            .session
            .as_ref()
            .unwrap()
            .prompt()
            .unwrap()
            .contains("PRIVATE_SOURCE_123")
    );
    f.state.deny(&mut f.workflow).unwrap();
    assert!(
        !f.state
            .current_prompt(&f.root, &f.config)
            .unwrap()
            .contains("PRIVATE_SOURCE_123")
    );
    f.action(json!({"action":"read_file","path":"source.rs"}))
        .unwrap();
    std::fs::write(f.root.join("source.rs"), "changed").unwrap();
    assert!(f.state.approve_read(&f.root, &f.config).is_err());
    assert!(f.state.session.as_ref().unwrap().reads.is_empty());
}
#[test]
fn approved_read_exact_edit_apply_and_stale_replay() {
    let mut f = Fixture::new();
    f.action(json!({"action":"read_file","path":"source.rs"}))
        .unwrap();
    f.state.approve_read(&f.root, &f.config).unwrap();
    assert!(
        f.state
            .current_prompt(&f.root, &f.config)
            .unwrap()
            .contains("PRIVATE_SOURCE_123")
    );
    f.action(json!({"action":"propose_edits","summary":"fix","edits":[{"path":"source.rs","after":"fixed\n"}]})).unwrap();
    f.state.guard_apply(&f.root, &f.config).unwrap();
    let files = f.workflow.preview.as_ref().unwrap().files.clone();
    assert!(
        f.workflow
            .apply(&f.root, &[f.root.join("source.rs")], false)
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(f.root.join("source.rs")).unwrap(),
        "PRIVATE_SOURCE_123"
    );
    f.workflow.apply(&f.root, &[], false).unwrap();
    f.state.after_apply(&f.root, &files, false);
    assert_eq!(
        std::fs::read_to_string(f.root.join("source.rs")).unwrap(),
        "fixed\n"
    );
    assert!(f.state.guard_apply(&f.root, &f.config).is_err());
    assert!(
        f.state
            .receive(
                &f.root,
                &f.config,
                r#"{"action":"finish","summary":"replay"}"#,
                &mut f.workflow
            )
            .is_err()
    );
    f.workflow.apply(&f.root, &[], true).unwrap();
    assert_eq!(
        std::fs::read_to_string(f.root.join("source.rs")).unwrap(),
        "PRIVATE_SOURCE_123"
    );
}
#[test]
fn strict_actions_scope_and_opt_in_recipes() {
    for value in [
        json!({"action":"read_file","path":"../secret"}),
        json!({"action":"read_file","path":"source.rs","extra":true}),
        json!({"action":"shell","command":"whoami"}),
        json!({"action":"run_check","recipe":"cargo_test"}),
        json!({"action":"propose_edits","summary":"no read","edits":[{"path":"source.rs","after":"bad"}]}),
    ] {
        let mut f = Fixture::new();
        assert!(f.action(value).is_err());
        assert!(f.state.session.as_ref().unwrap().pending.is_none());
        assert_eq!(
            std::fs::read_to_string(f.root.join("source.rs")).unwrap(),
            "PRIVATE_SOURCE_123"
        );
    }
}
#[test]
fn reservations_configuration_expiry_and_failure_recovery() {
    let mut f = Fixture::new();
    let r = f.state.reserve(&f.root, &f.config, 100).unwrap();
    assert_eq!(f.state.session.as_ref().unwrap().turns, 0);
    f.state.commit(r);
    assert!(f.state.reserve(&f.root, &f.config, 100).is_err());
    f.state.record_error("network failed");
    assert!(f.state.reserve(&f.root, &f.config, 100).is_ok());
    assert!(f.state.reserve(&f.root, &f.config, 512 * 1024 + 1).is_err());
    let mut config = f.config.clone();
    config.model = "other".into();
    assert!(f.state.current_prompt(&f.root, &config).is_err());
    let s = f.state.session.as_mut().unwrap();
    s.policy.turns = 1;
    assert!(f.state.reserve(&f.root, &f.config, 100).is_err());
    let s = f.state.session.as_mut().unwrap();
    s.policy.turns = 8;
    s.policy.output_tokens = 4096;
    assert!(f.state.reserve(&f.root, &f.config, 100).is_err());
    let s = f.state.session.as_mut().unwrap();
    s.policy.output_tokens = 65536;
    s.policy.estimate_limit_usd = 0.01;
    s.policy.input_usd_per_million = 1000.0;
    assert!(f.state.reserve(&f.root, &f.config, 10000).is_err());
    f.state.session.as_mut().unwrap().started = Instant::now() - Duration::from_secs(1900);
    assert!(f.state.current_prompt(&f.root, &f.config).is_err());
}

#[test]
fn approved_real_cargo_validation_requires_evidence_delivery() {
    let mut f = Fixture::new();
    std::fs::create_dir_all(f.root.join("src")).unwrap();
    std::fs::write(
        f.root.join("Cargo.toml"),
        "[package]\nname='controlled-fixture'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(f.root.join("src/lib.rs"), "pub fn value()->u32 { 42 }\n").unwrap();
    f.state
        .session
        .as_mut()
        .unwrap()
        .policy
        .recipes
        .push(Recipe::CargoCheck);
    f.action(json!({"action":"run_check","recipe":"cargo_check"}))
        .unwrap();
    assert!(!f.root.join("target").exists());
    f.state
        .approve_check(&f.root, &f.config, &eframe::egui::Context::default())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if f.state.poll().is_some() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    let evidence = f.state.session.as_ref().unwrap().evidence.as_ref().unwrap();
    assert_eq!(evidence.exit_code, Some(0));
    assert!(evidence.capture_complete);
    assert!(f.state.current_prompt(&f.root, &f.config).is_err());
    f.state.deliver_evidence(&f.root, &f.config, false).unwrap();
    let prompt = f.state.current_prompt(&f.root, &f.config).unwrap();
    assert!(prompt.contains("output_withheld"));
    assert!(!prompt.contains("Finished `dev`"));
    f.action(json!({"action":"finish","summary":"checked"}))
        .unwrap();
    assert!(f.state.reserve(&f.root, &f.config, 100).is_err());
}
#[test]
fn native_http_action_is_data_until_read_approval() {
    use crate::{agent::Event, native_models};
    let action = json!({"action":"read_file","path":"source.rs"}).to_string();
    let response =
        json!({"choices":[{"message":{"content":action},"finish_reason":"stop"}]}).to_string();
    let (config, server) =
        native_models::tests::mock(native_models::Protocol::Chat, response, false, "200 OK");
    let mut f = Fixture::new();
    f.state.reset();
    f.config = config;
    let source = ai_workflow::source(&f.root, &f.root.join("source.rs")).unwrap();
    let prompt = f
        .state
        .request(&f.root, &f.config, &[source], "Fix file")
        .unwrap();
    let prepared = native_models::prepare(&f.config, prompt, &[]).unwrap();
    let reservation = f
        .state
        .reserve(
            &f.root,
            &f.config,
            serde_json::to_vec(&prepared.body).unwrap().len(),
        )
        .unwrap();
    let mut run = native_models::start(prepared, || {}).unwrap();
    f.state.commit(reservation);
    let mut output = String::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    'poll: loop {
        for event in run.poll() {
            match event {
                Event::Text(s) => output.push_str(&s),
                Event::Done { error, .. } => {
                    assert_eq!(error, None);
                    break 'poll;
                }
                _ => {}
            }
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let captured = server.join().unwrap();
    assert!(!captured.body.to_string().contains("PRIVATE_SOURCE_123"));
    f.state
        .receive(&f.root, &f.config, &output, &mut f.workflow)
        .unwrap();
    assert!(f.state.session.as_ref().unwrap().reads.is_empty());
    assert_eq!(
        std::fs::read_to_string(f.root.join("source.rs")).unwrap(),
        "PRIVATE_SOURCE_123"
    );
}
