use super::*;
use forge_room_core::workflows::{CheckRecipe, Provenance, RunStatus, StepKind, StepStatus};
use std::{
    fs,
    time::{Duration, Instant},
};
struct Fixture {
    base: PathBuf,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base =
            std::env::temp_dir().join(format!("forge-workflow-{}", workflows::new_id().unwrap()));
        fs::create_dir_all(base.join("project")).unwrap();
        let base = base.canonicalize().unwrap();
        Self {
            root: base.join("project"),
            base,
        }
    }
    fn contexts(&self) -> BTreeMap<String, resolver::Context> {
        [(
            "primary".into(),
            resolver::Context {
                alias: "primary".into(),
                label: "Project".into(),
                root: self.root.clone(),
                source: self.root.clone(),
                worktree: None,
            },
        )]
        .into_iter()
        .collect()
    }
    fn review(&self, workflow: Workflow) -> resolver::Review {
        resolver::review(
            &self.root,
            workflow.expand(&BTreeMap::new(), &[]).unwrap(),
            &self.contexts(),
            None,
            &[],
            false,
        )
        .unwrap()
    }
    fn engine(&self) -> executor::Engine {
        executor::Engine::from_store(
            storage::Store::open_at(&self.base.join("history"), &self.root).unwrap(),
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let temp = std::env::temp_dir().canonicalize().unwrap();
        assert!(
            self.base.starts_with(&temp)
                && self
                    .base
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("forge-workflow-")
        );
        let _ = fs::remove_dir_all(&self.base);
    }
}
fn manual() -> Workflow {
    let mut workflow = workflows::starter_workflows()
        .into_iter()
        .find(|workflow| workflow.id == "manual-release")
        .unwrap();
    workflow.steps.truncate(1);
    workflow
}
fn approve(engine: &mut executor::Engine, id: &str) {
    let seal = engine
        .records
        .iter()
        .find(|record| record.saved.run.id == id)
        .unwrap()
        .saved
        .run
        .plan
        .seal
        .clone();
    engine
        .change(id, |run| {
            run.approve(&seal, forge_room_core::now_epoch_seconds())
        })
        .unwrap();
}
fn tick(
    fixture: &Fixture,
    engine: &mut executor::Engine,
    config: Option<&crate::native_models::Config>,
) {
    engine.tick(
        &egui::Context::default(),
        Some(&fixture.root),
        true,
        &[],
        &fixture.contexts(),
        config,
    );
    assert!(engine.fault.is_none(), "{:?}", engine.fault);
}
fn finish(
    fixture: &Fixture,
    engine: &mut executor::Engine,
    config: Option<&crate::native_models::Config>,
) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while engine
        .records
        .iter()
        .any(|record| !record.saved.run.terminal())
    {
        tick(fixture, engine, config);
        assert!(
            Instant::now() < deadline,
            "workflow did not finish: {:?}",
            engine
                .records
                .iter()
                .map(|record| record.saved.run.status_summary())
                .collect::<Vec<_>>()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn project_library_preserves_external_edits_and_backups_and_imports_are_closed() {
    let fixture = Fixture::new();
    let backup = fixture.base.join("backups");
    fs::create_dir(&backup).unwrap();
    let library = library::Library::default();
    let digest = library::save(&fixture.root, &library, None, &backup).unwrap();
    assert!(library::save(&fixture.root, &library, None, &backup).is_err());
    let mut next = library.clone();
    next.workflows[0].title = "Changed quality plan".into();
    let second = library::save(&fixture.root, &next, Some(&digest), &backup).unwrap();
    assert_ne!(digest, second);
    assert_eq!(fs::read_dir(&backup).unwrap().count(), 1);
    fs::write(library::location(&fixture.root), b"external edit").unwrap();
    assert!(library::save(&fixture.root, &next, Some(&second), &backup).is_err());
    assert_eq!(
        fs::read(library::location(&fixture.root)).unwrap(),
        b"external edit"
    );
    let imported=library::vscode(br#"{// JSONC
 "tasks":[{"label":"check","type":"process","command":"cargo","args":["check"]},{"label":"shell","type":"shell","command":"cargo test && echo secret","dependsOn":"check"},{"label":"env","type":"process","command":"cargo","args":["test"],"options":{"env":{"FLAG":"1"}}}]}"#).unwrap();
    assert!(matches!(
        imported.workflows[0].steps[0].action,
        StepKind::Check {
            recipe: CheckRecipe::CargoCheck
        }
    ));
    assert!(matches!(
        imported.workflows[0].steps[1].action,
        StepKind::Manual { .. }
    ));
    assert!(matches!(
        imported.workflows[0].steps[2].action,
        StepKind::Manual { .. }
    ));
    assert_eq!(imported.workflows[0].steps[1].needs, vec!["task-1"]);
    assert!(library::vscode(br#"{"tasks":[{"label":"x","dependsOn":"missing"}]}"#).is_err());
    let mut appended = library.clone();
    appended.append(library.clone()).unwrap();
    assert_ne!(
        appended.workflows[0].id,
        appended.workflows[library.workflows.len()].id
    );
    appended.validate().unwrap();
}
#[test]
fn storage_locks_torn_tail_fallback_corruption_and_restart_require_review() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    assert!(storage::Store::open_at(&fixture.base.join("history"), &fixture.root).is_err());
    let id = engine.add(fixture.review(manual())).unwrap();
    approve(&mut engine, &id);
    let folder = engine.store.root.join(&id);
    let latest = fs::read_dir(&folder)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "run"))
        .map(|entry| entry.path())
        .max()
        .unwrap();
    fs::write(&latest, b"FRC1\n{torn").unwrap();
    let loaded = engine.store.load(&id).unwrap();
    assert_eq!(loaded.saved.run.status, RunStatus::Draft);
    drop(engine);
    let recovered = fixture.engine();
    assert_eq!(
        recovered.records[0].saved.run.status,
        RunStatus::Interrupted
    );
    assert!(recovered.records[0].saved.run.approved_seal.is_none());
    let folder = recovered.store.root.join(&id);
    let latest = fs::read_dir(&folder)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "run"))
        .map(|entry| entry.path())
        .max()
        .unwrap();
    fs::write(&latest, b"FRC1\n{}\nDONE\n").unwrap();
    assert!(recovered.store.load(&id).is_err());
}
#[test]
fn exact_review_refuses_changed_inputs_and_saved_arbitrary_commands() {
    if crate::controlled_build::validation::find("python").is_none() {
        return;
    }
    let fixture = Fixture::new();
    fs::write(fixture.root.join("test.py"), b"print('ok')").unwrap();
    let mut workflow = manual();
    workflow.steps[0].action = StepKind::Check {
        recipe: CheckRecipe::PythonCompile,
    };
    workflow.steps[0].inputs = vec!["test.py".into()];
    let review = fixture.review(workflow);
    review
        .verify_step(&review.expanded.workflow.steps[0].id)
        .unwrap();
    fs::write(fixture.root.join("test.py"), b"print('changed')").unwrap();
    assert!(
        review
            .verify_step(&review.expanded.workflow.steps[0].id)
            .is_err()
    );
    let mut modified = review.clone();
    let id = modified.expanded.workflow.steps[0].id.clone();
    modified.steps.get_mut(&id).unwrap().inputs.clear();
    modified
        .steps
        .get_mut(&id)
        .unwrap()
        .command
        .as_mut()
        .unwrap()
        .args = vec!["-c".into(), "print('not an allowed recipe')".into()];
    modified.seal = workflows::seal(&(
        &modified.source,
        &modified.expanded,
        &modified.steps,
        &modified.source_file,
        modified.seal_tracked,
    ))
    .unwrap();
    assert!(
        modified
            .verify_step(&id)
            .unwrap_err()
            .contains("resolution changed")
    );
}
#[test]
fn real_python_recipe_artifacts_private_logs_and_status_projection() {
    if crate::controlled_build::validation::find("python").is_none() {
        return;
    }
    let fixture = Fixture::new();
    fs::write(fixture.root.join("test_flow.py"),"import unittest\nfrom pathlib import Path\nclass Trial(unittest.TestCase):\n def test_receipt(self):\n  print('PRIVATE_LOG_SENTINEL')\n  Path('receipt.txt').write_text('artifact result')\n  self.assertEqual(2+2,4)\n").unwrap();
    let mut workflow = manual();
    workflow.steps[0].action = StepKind::Check {
        recipe: CheckRecipe::PythonUnittest,
    };
    workflow.steps[0].inputs = vec!["test_flow.py".into()];
    workflow.steps[0].artifacts = vec!["receipt.txt".into()];
    let mut engine = fixture.engine();
    let id = engine.add(fixture.review(workflow)).unwrap();
    approve(&mut engine, &id);
    finish(&fixture, &mut engine, None);
    let record = &engine.records[0];
    assert_eq!(record.saved.run.status, RunStatus::Succeeded);
    let detail = engine.store.detail(&id, &record.saved.details[0]).unwrap();
    assert!(
        detail
            .native
            .as_ref()
            .unwrap()
            .stdout
            .contains("PRIVATE_LOG_SENTINEL")
    );
    assert_eq!(
        engine
            .store
            .verify_artifacts(&id, &detail)
            .get("receipt.txt"),
        Some(&true)
    );
    assert!(!reports::brief(record).contains("PRIVATE_LOG_SENTINEL"));
    assert!(
        !reports::receipt(record, &record.saved.run.steps[0].id)
            .unwrap()
            .contains("PRIVATE_LOG_SENTINEL")
    );
    assert_eq!(
        record.saved.run.steps[0].attempts[0]
            .outcome
            .as_ref()
            .unwrap()
            .provenance,
        Provenance::NativeProcess
    );
    let target = engine
        .store
        .root
        .join(&id)
        .join(detail.artifacts[0].stored.as_ref().unwrap());
    fs::write(target, b"changed").unwrap();
    assert_eq!(
        engine
            .store
            .verify_artifacts(&id, &detail)
            .get("receipt.txt"),
        Some(&false)
    );
    engine.delete(&id).unwrap();
    assert!(engine.records.is_empty());
}
#[test]
fn scheduler_serializes_overlapping_contexts_and_storage_failure_prevents_spawn() {
    if crate::controlled_build::validation::find("python").is_none() {
        return;
    }
    let fixture = Fixture::new();
    fs::write(
        fixture.root.join("test_wait.py"),
        "import unittest,time\nclass Trial(unittest.TestCase):\n def test_wait(self): time.sleep(.2)\n",
    )
    .unwrap();
    let mut workflow = manual();
    workflow.steps[0].action = StepKind::Check {
        recipe: CheckRecipe::PythonUnittest,
    };
    workflow.steps[0].inputs = vec!["test_wait.py".into()];
    let review = fixture.review(workflow);
    let mut engine = fixture.engine();
    let first = engine.add(review.clone()).unwrap();
    let second = engine.add(review).unwrap();
    approve(&mut engine, &first);
    approve(&mut engine, &second);
    tick(&fixture, &mut engine, None);
    assert_eq!(engine.active_count(), 1);
    assert_eq!(engine.records[1].saved.run.attempts_used, 0);
    finish(&fixture, &mut engine, None);
    let third = engine.add(engine.records[0].review.clone()).unwrap();
    approve(&mut engine, &third);
    let folder = engine.store.root.join(&third);
    fs::create_dir(folder.join("0000000000000003.run")).unwrap();
    engine.tick(
        &egui::Context::default(),
        Some(&fixture.root),
        true,
        &[],
        &fixture.contexts(),
        None,
    );
    assert!(engine.fault.is_some());
    assert_eq!(engine.active_count(), 0);
}
#[test]
fn native_model_step_needs_attempt_and_exact_send_approval_and_reserves_budget() {
    let fixture = Fixture::new();
    let mut workflow = manual();
    workflow.steps[0].action = StepKind::AiReview {
        prompt: "Summarize acceptance limitations".into(),
        include_statuses: true,
        include_inputs: false,
        max_tokens: 128,
    };
    workflow.policy.model_calls = 1;
    workflow.policy.model_output_tokens = 128;
    let mut engine = fixture.engine();
    let id = engine.add(fixture.review(workflow)).unwrap();
    approve(&mut engine, &id);
    let response = serde_json::json!({"choices":[{"message":{"content":"MODEL_SUGGESTION_SENTINEL"},"finish_reason":"stop"}]}).to_string();
    let (config, server) = crate::native_models::tests::mock(
        crate::native_models::Protocol::Chat,
        response,
        false,
        "200 OK",
    );
    tick(&fixture, &mut engine, Some(&config));
    assert_eq!(engine.active_count(), 0);
    let step = engine.records[0].saved.run.steps[0].id.clone();
    engine
        .change(&id, |run| {
            run.approve_attempt(&step, forge_room_core::now_epoch_seconds())
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while engine.model_review().is_none() {
        tick(&fixture, &mut engine, Some(&config));
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(engine.records[0].saved.run.model_calls_used, 1);
    assert_eq!(engine.records[0].saved.run.model_tokens_reserved, 128);
    engine
        .send_model(&id, &step, &egui::Context::default())
        .unwrap();
    finish(&fixture, &mut engine, Some(&config));
    let captured = server.join().unwrap();
    assert_eq!(captured.body["max_tokens"], 128);
    assert!(captured.body.get("tools").is_none());
    let record = &engine.records[0];
    assert_eq!(record.saved.run.steps[0].status, StepStatus::Succeeded);
    assert_eq!(
        record.saved.run.steps[0].attempts[0]
            .outcome
            .as_ref()
            .unwrap()
            .provenance,
        Provenance::NativeModel
    );
    let detail = engine.store.detail(&id, &record.saved.details[0]).unwrap();
    assert!(
        detail
            .model_text
            .unwrap()
            .contains("MODEL_SUGGESTION_SENTINEL")
    );
    assert!(!reports::brief(record).contains("MODEL_SUGGESTION_SENTINEL"));
}
#[test]
fn egui_editor_and_history_build_without_native_surface_acceptance() {
    let fixture = Fixture::new();
    let engine = fixture.engine();
    let mut state = State {
        open: true,
        root: Some(fixture.root.clone()),
        engine: Some(engine),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280., 900.),
            )),
            ..Default::default()
        },
        |ui| state.show(ui.ctx(), true, &[], &[]),
    );
    output.textures_delta.clear();
    assert!(!output.shapes.is_empty());
    let id = state
        .engine
        .as_mut()
        .unwrap()
        .add(fixture.review(manual()))
        .unwrap();
    state.selected_run = id;
    state.page = 1;
    let mut output = ctx.run_ui(Default::default(), |ui| {
        state.show(ui.ctx(), true, &[], &[])
    });
    output.textures_delta.clear();
    assert!(!output.shapes.is_empty());
}
#[test]
fn private_editor_drafts_and_leased_attempts_survive_restart_without_launch_permission() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine();
    let mut library = library::Library::default();
    library.workflows[0].id.clear();
    let draft = storage::Draft {
        library,
        baseline: Some("old-file-digest".into()),
        json: Some("{ unfinished JSON draft".into()),
    };
    engine.store.save_draft(&draft).unwrap();
    let restored = engine.store.load_draft().unwrap().unwrap();
    assert_eq!(restored.json, draft.json);
    assert!(restored.library.validate().is_err());
    let id = engine.add(fixture.review(manual())).unwrap();
    approve(&mut engine, &id);
    let step = engine.records[0].saved.run.steps[0].id.clone();
    engine
        .change(&id, |run| {
            let now = forge_room_core::now_epoch_seconds();
            run.approve_attempt(&step, now)?;
            run.claim(&step, now)?;
            Ok(())
        })
        .unwrap();
    let token = engine.records[0].saved.run.steps[0].attempts[0]
        .token
        .clone();
    drop(engine);
    let engine = fixture.engine();
    assert_eq!(engine.active_count(), 0);
    let run = &engine.records[0].saved.run;
    assert_eq!(run.status, RunStatus::Interrupted);
    assert_eq!(run.steps[0].status, StepStatus::Interrupted);
    assert_eq!(run.steps[0].attempts[0].token, token);
    assert_eq!(
        run.steps[0].attempts[0]
            .outcome
            .as_ref()
            .unwrap()
            .provenance,
        Provenance::Interrupted
    );
    assert!(run.approved_seal.is_none());
}
#[test]
fn trust_revocation_cancels_owned_python_and_never_starts_queued_peer() {
    if crate::controlled_build::validation::find("python").is_none() {
        return;
    }
    let fixture = Fixture::new();
    fs::write(fixture.root.join("test_wait.py"),"import unittest,time\nfrom pathlib import Path\nclass Trial(unittest.TestCase):\n def test_wait(self):\n  Path('started.txt').write_text('owned')\n  time.sleep(10)\n  Path('should-not-exist.txt').write_text('late')\n").unwrap();
    let mut workflow = manual();
    workflow.steps[0].action = StepKind::Check {
        recipe: CheckRecipe::PythonUnittest,
    };
    workflow.steps[0].inputs = vec!["test_wait.py".into()];
    let review = fixture.review(workflow);
    let mut engine = fixture.engine();
    let first = engine.add(review.clone()).unwrap();
    let second = engine.add(review).unwrap();
    approve(&mut engine, &first);
    approve(&mut engine, &second);
    let until = Instant::now() + Duration::from_secs(5);
    while !fixture.root.join("started.txt").exists() {
        tick(&fixture, &mut engine, None);
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(20));
    }
    let until = Instant::now() + Duration::from_secs(5);
    while engine.active_count() > 0 {
        engine.tick(
            &egui::Context::default(),
            Some(&fixture.root),
            false,
            &[],
            &fixture.contexts(),
            None,
        );
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!fixture.root.join("should-not-exist.txt").exists());
    assert_eq!(engine.records[1].saved.run.attempts_used, 0);
    assert_eq!(engine.records[0].saved.run.status, RunStatus::Canceled);
    assert_eq!(engine.records[1].saved.run.status, RunStatus::Canceled);
}
#[test]
fn native_node_and_cargo_recipes_execute_real_disposable_projects() {
    let fixture = Fixture::new();
    let mut workflow = manual();
    workflow.steps[0].action = StepKind::Checkpoint {
        label: "start".into(),
    };
    let mut executed = 0;
    if crate::controlled_build::validation::find("node").is_some() {
        fs::write(
            fixture.root.join("sample.test.js"),
            "const test=require('node:test'); const assert=require('node:assert/strict'); test('native workflow',()=>assert.equal(2+2,4));\n",
        )
        .unwrap();
        let mut step = workflow.steps[0].clone();
        step.id = "node".into();
        step.action = StepKind::Check {
            recipe: CheckRecipe::NodeTest,
        };
        step.inputs = vec!["sample.test.js".into()];
        step.needs = vec![workflow.steps[0].id.clone()];
        workflow.steps.push(step);
        executed += 1;
    }
    if crate::controlled_build::validation::find("cargo").is_some() {
        fs::create_dir(fixture.root.join("src")).unwrap();
        fs::write(
            fixture.root.join("Cargo.toml"),
            "[package]\nname='workflow-smoke'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            fixture.root.join("src/lib.rs"),
            "#[test] fn native_trial(){assert_eq!(2+2,4);}\n",
        )
        .unwrap();
        let mut step = workflow.steps[0].clone();
        step.id = "cargo".into();
        step.action = StepKind::Check {
            recipe: CheckRecipe::CargoTest,
        };
        step.inputs = vec!["src/lib.rs".into()];
        step.needs = vec![workflow.steps.last().unwrap().id.clone()];
        workflow.steps.push(step);
        executed += 1;
    }
    let mut engine = fixture.engine();
    let id = engine.add(fixture.review(workflow)).unwrap();
    approve(&mut engine, &id);
    finish(&fixture, &mut engine, None);
    assert_eq!(engine.records[0].saved.run.status, RunStatus::Succeeded);
    assert_eq!(
        engine.records[0]
            .saved
            .run
            .steps
            .iter()
            .filter(|step| step
                .attempts
                .last()
                .and_then(|attempt| attempt.outcome.as_ref())
                .is_some_and(|outcome| outcome.provenance == Provenance::NativeProcess
                    && outcome.exit_code == Some(0)))
            .count(),
        executed
    );
}
