use super::*;
use std::collections::BTreeMap;
fn one() -> Workflow {
    let mut workflow = starter_workflows().remove(0);
    workflow.steps.truncate(2);
    workflow
}
#[test]
fn schemas_validate_starters_tokens_graphs_and_bounds() {
    for workflow in starter_workflows() {
        workflow.validate().unwrap();
        workflow.expand(&BTreeMap::new(), &[]).unwrap();
    }
    let mut workflow = one();
    workflow.steps[0].needs = vec![workflow.steps[1].id.clone()];
    assert!(workflow.validate().is_err());
    workflow = one();
    workflow.steps[0].cwd = "../outside".into();
    assert!(workflow.validate().is_err());
    workflow = one();
    workflow.parameters.push(Parameter {
        name: "folder".into(),
        label: "Project folder".into(),
        kind: ParameterKind::RelativePath,
        default: Some(".".into()),
        required: true,
        choices: vec![],
    });
    workflow.steps[0].cwd = "${folder}".into();
    assert!(workflow
        .expand(
            &[("folder".into(), "../bad".into())].into_iter().collect(),
            &[]
        )
        .is_err());
    assert_eq!(
        workflow
            .expand(
                &[("folder".into(), "native/app".into())]
                    .into_iter()
                    .collect(),
                &[]
            )
            .unwrap()
            .workflow
            .steps[0]
            .cwd,
        "native/app"
    );
}
#[test]
fn approvals_leases_dependencies_and_duplicate_receipts_are_enforced() {
    let workflow = one();
    let plan = workflow
        .expand(&BTreeMap::new(), &["check".into()])
        .unwrap();
    let mut run = Run::new(plan, "project".into(), 10).unwrap();
    assert!(run.offers(10).unwrap().is_empty());
    let seal = run.plan.seal.clone();
    run.approve(&seal, 10).unwrap();
    assert_eq!(run.offers(10).unwrap()[0].step, "format");
    let token = run.claim("format", 11).unwrap();
    assert!(run.claim("check", 11).is_err());
    let outcome = Outcome {
        passed: true,
        provenance: Provenance::NativeProcess,
        note: "exit zero".into(),
        exit_code: Some(0),
        timed_out: false,
        canceled: false,
        capture_complete: true,
        truncated: false,
        elapsed_ms: 3,
        output_bytes: 12,
        artifact_bytes: 0,
    };
    run.complete("format", &token, outcome.clone(), 12).unwrap();
    assert!(run.complete("format", &token, outcome.clone(), 12).is_err());
    let token = run.claim("check", 13).unwrap();
    run.complete("check", &token, outcome, 14).unwrap();
    assert_eq!(run.status, RunStatus::Succeeded);
    run.validate().unwrap();
}
#[test]
fn failures_manual_reports_recovery_and_budget_stops_stay_distinct() {
    let workflow = starter_workflows()
        .into_iter()
        .find(|workflow| workflow.id == "manual-release")
        .unwrap();
    let plan = workflow.expand(&BTreeMap::new(), &[]).unwrap();
    let mut run = Run::new(plan, "project".into(), 10).unwrap();
    let seal = run.plan.seal.clone();
    run.approve(&seal, 10).unwrap();
    assert!(run.offers(10).unwrap()[0].requires_approval);
    run.decide("scope", true, "Scope reviewed by human".into(), 11)
        .unwrap();
    assert_eq!(
        run.steps[0].attempts[0]
            .outcome
            .as_ref()
            .unwrap()
            .provenance,
        Provenance::ManualReport
    );
    run.approve_attempt("smoke", 12).unwrap();
    run.claim("smoke", 12).unwrap();
    run.recover(13).unwrap();
    assert_eq!(run.status, RunStatus::Interrupted);
    assert!(run.offers(14).unwrap().is_empty());
    assert_eq!(run.steps[1].status, StepStatus::Interrupted);
    run.validate().unwrap();
    let mut workflow = one();
    workflow.policy.max_attempts = 1;
    let mut run = Run::new(
        workflow.expand(&BTreeMap::new(), &[]).unwrap(),
        "project".into(),
        1,
    )
    .unwrap();
    let seal = run.plan.seal.clone();
    run.approve(&seal, 1).unwrap();
    run.claim("format", 1).unwrap();
    run.offers(2).unwrap();
    assert_eq!(run.status, RunStatus::Running);
    assert!(run.claim("check", 2).is_err());
}
#[test]
fn selected_targets_include_prerequisites_and_unknown_targets_fail() {
    let workflow = starter_workflows().remove(0);
    let plan = workflow
        .expand(&BTreeMap::new(), &["check".into()])
        .unwrap();
    assert_eq!(plan.included.len(), 2);
    assert!(workflow
        .expand(&BTreeMap::new(), &["missing".into()])
        .is_err());
    assert!(relative(".env").is_err());
    assert!(relative("secrets/key.pem").is_err());
    assert!(relative("src/test.rs").is_ok());
}
#[test]
fn plan_seals_detect_modified_saved_commands() {
    let mut plan = one().expand(&BTreeMap::new(), &[]).unwrap();
    plan.workflow.steps[0].cwd = "other".into();
    assert!(Run::new(plan, "project".into(), 1).is_err());
    assert_eq!(
        digest(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
fn checkpoint(id: &str, needs: &[&str], when: When) -> Step {
    Step {
        id: id.into(),
        title: id.into(),
        action: StepKind::Checkpoint { label: id.into() },
        needs: needs.iter().map(|id| (*id).into()).collect(),
        when,
        context: "primary".into(),
        cwd: ".".into(),
        inputs: vec![],
        artifacts: vec![],
        resources: vec![],
        timeout_seconds: 20,
        retries: 1,
        optional: false,
        review_each_attempt: false,
        room_task: None,
        room_id: None,
    }
}
fn graph(steps: Vec<Step>) -> Run {
    let workflow = Workflow {
        version: 1,
        id: "graph".into(),
        title: "Graph trial".into(),
        description: String::new(),
        parameters: vec![],
        steps,
        policy: Policy {
            parallelism: 4,
            continue_independent: true,
            allow_retry: true,
            ..Default::default()
        },
    };
    let mut run = Run::new(
        workflow.expand(&BTreeMap::new(), &[]).unwrap(),
        "project".into(),
        1,
    )
    .unwrap();
    let seal = run.plan.seal.clone();
    run.approve(&seal, 1).unwrap();
    run
}
fn done(run: &mut Run, id: &str, passed: bool, now: u64) {
    let token = run.claim(id, now).unwrap();
    run.complete(
        id,
        &token,
        Outcome::declared(passed, Provenance::RunCheckpoint, "local checkpoint".into()).unwrap(),
        now + 1,
    )
    .unwrap();
}
#[test]
fn conditional_graphs_resolve_success_failure_always_and_independent_work() {
    let mut run = graph(vec![
        checkpoint("base", &[], When::Success),
        checkpoint("success", &["base"], When::Success),
        checkpoint("failure", &["base"], When::Failure),
        checkpoint("cleanup", &["base"], When::Always),
        checkpoint("independent", &[], When::Success),
    ]);
    done(&mut run, "base", false, 2);
    let offers = run.offers(3).unwrap();
    assert_eq!(run.steps[1].status, StepStatus::Skipped);
    assert!(offers.iter().any(|offer| offer.step == "failure"));
    done(&mut run, "failure", true, 4);
    done(&mut run, "cleanup", true, 6);
    done(&mut run, "independent", true, 8);
    assert_eq!(run.status, RunStatus::Failed);
    run.validate().unwrap();
}
#[test]
fn pause_retry_resource_locks_and_cancel_preserve_attempt_identity() {
    let mut a = checkpoint("a", &[], When::Success);
    a.resources = vec!["database".into()];
    let mut b = checkpoint("b", &[], When::Success);
    b.resources = a.resources.clone();
    let mut run = graph(vec![a, b, checkpoint("child", &["a"], When::Success)]);
    let token = run.claim("a", 2).unwrap();
    assert!(run.claim("b", 2).is_err());
    run.pause(3).unwrap();
    assert!(run.claim("b", 3).is_err());
    run.complete(
        "a",
        &token,
        Outcome::declared(false, Provenance::RunCheckpoint, "failed".into()).unwrap(),
        4,
    )
    .unwrap();
    assert_eq!(run.status, RunStatus::Paused);
    run.resume(5).unwrap();
    done(&mut run, "b", true, 6);
    run.offers(8).unwrap();
    assert_eq!(run.status, RunStatus::Failed);
    run.retry_from("a", 9).unwrap();
    assert_eq!(run.status, RunStatus::Draft);
    assert_eq!(run.steps[2].status, StepStatus::Pending);
    let seal = run.plan.seal.clone();
    run.approve(&seal, 10).unwrap();
    assert!(run.claim("a", 11).is_err());
    run.approve_attempt("a", 11).unwrap();
    let retry = run.claim("a", 12).unwrap();
    assert_ne!(token, retry);
    run.stop("user canceled", 13);
    let mut canceled =
        Outcome::declared(false, Provenance::RunCheckpoint, "canceled".into()).unwrap();
    canceled.canceled = true;
    run.complete("a", &retry, canceled, 14).unwrap();
    assert_eq!(run.status, RunStatus::Canceled);
    run.validate().unwrap();
}
#[test]
fn invalid_closures_statuses_provenance_and_counters_are_rejected() {
    let mut run = graph(vec![
        checkpoint("a", &[], When::Success),
        checkpoint("b", &["a"], When::Success),
    ]);
    let mut tampered = run.plan.clone();
    tampered.included.remove("a");
    tampered.seal = seal(&(
        &tampered.workflow,
        &tampered.parameters,
        &tampered.targets,
        &tampered.included,
    ))
    .unwrap();
    assert!(Run::new(tampered, "project".into(), 1).is_err());
    run.steps[0].status = StepStatus::Succeeded;
    assert!(run.validate().is_err());
    run.steps[0].status = StepStatus::Pending;
    done(&mut run, "a", true, 2);
    let mut bad = run.clone();
    bad.steps[0].attempts[0]
        .outcome
        .as_mut()
        .unwrap()
        .provenance = Provenance::NativeModel;
    assert!(bad.validate().is_err());
    let mut bad = run.clone();
    bad.output_used = 1;
    assert!(bad.validate().is_err());
    run.validate().unwrap();
}
#[test]
fn model_reservations_are_bounded_and_manual_decisions_are_atomic() {
    let mut step = checkpoint("ai", &[], When::Success);
    step.action = StepKind::AiReview {
        prompt: "review".into(),
        include_statuses: false,
        include_inputs: false,
        max_tokens: 128,
    };
    let mut run = graph(vec![step]);
    run.approve_attempt("ai", 2).unwrap();
    assert!(run.claim("ai", 2).is_err());
    assert_eq!(run.model_calls_used, 0);
    let mut manual = checkpoint("manual", &["before"], When::Success);
    manual.action = StepKind::Manual {
        instructions: "review".into(),
    };
    let mut run = graph(vec![checkpoint("before", &[], When::Success), manual]);
    let before = run.clone();
    assert!(run.decide("manual", true, "early".into(), 2).is_err());
    assert_eq!(run, before);
    run.pause(2).unwrap();
    let before = run.clone();
    assert!(run.decide("manual", true, "paused".into(), 3).is_err());
    assert_eq!(run, before);
}
#[test]
fn relative_paths_and_typed_parameter_values_do_not_expand_environment_or_shell() {
    for path in [
        "/absolute",
        "C:/drive",
        "a\\b",
        "a/../b",
        "a//b",
        ".git/config",
        "dir/.env.local",
        "dir/private.KEY",
        "a\nfile",
    ] {
        assert!(relative(path).is_err(), "{path}");
    }
    let parameter = Parameter {
        name: "choice".into(),
        label: "Choice".into(),
        kind: ParameterKind::Choice,
        default: None,
        required: true,
        choices: vec!["one".into(), "two".into()],
    };
    assert!(parameter.validate_value("three").is_err());
    assert!(parameter.validate_value("one").is_ok());
    let values = [("literal".into(), "$(command); & other".into())]
        .into_iter()
        .collect();
    assert_eq!(
        substitute("${literal}", &values).unwrap(),
        "$(command); & other"
    );
}
#[test]
fn different_hosts_with_same_display_name_have_distinct_public_receipt_scopes() {
    let first = crate::transport::Host::start("127.0.0.1:0".parse().unwrap(), "same name").unwrap();
    let second =
        crate::transport::Host::start("127.0.0.1:0".parse().unwrap(), "same name").unwrap();
    let a = first.snapshot().unwrap();
    let b = second.snapshot().unwrap();
    assert_eq!(a.room, b.room);
    assert_eq!(a.shared.room_id.len(), 64);
    assert_ne!(a.shared.room_id, b.shared.room_id);
    let mut old = serde_json::to_value(&a.shared).unwrap();
    old.as_object_mut().unwrap().remove("room_id");
    let old: crate::transport::SharedSnapshot = serde_json::from_value(old).unwrap();
    assert!(old.room_id.is_empty());
    let mut step = serde_json::to_value(&one().steps[0]).unwrap();
    step.as_object_mut().unwrap().remove("room_id");
    let old: Step = serde_json::from_value(step).unwrap();
    assert!(old.room_id.is_none());
}
