use super::*;
use crate::{Participant, Role, TaskPriority};
fn task(id: &str, actor: &str, deps: Vec<String>) -> Task {
    Task {
        id: id.into(),
        title: id.into(),
        owner_id: actor.into(),
        owner_name: "owner".into(),
        status: TaskStatus::Open,
        priority: TaskPriority::High,
        dependencies: deps,
        acceptance_criteria: vec![],
        completed_criteria: vec![],
        worktree_branch: None,
        due_at_epoch_seconds: Some(5000),
        updated_at_epoch_seconds: 1,
    }
}
fn state() -> RoomState {
    let actor = "a".repeat(64);
    let mut s = RoomState::new(
        "fixture",
        Participant {
            id: actor.clone(),
            name: "owner".into(),
            role: Role::Human,
            owner_id: None,
        },
    );
    s.tasks = vec![
        task("T1", &actor, vec![]),
        task("T2", &"b".repeat(64), vec!["T1".into()]),
    ];
    s
}
fn profile() -> ReadinessProfile {
    ReadinessProfile {
        estimate_minutes: Some(30),
        files: vec!["src/api.rs".into()],
        help_allowed: true,
        ..Default::default()
    }
}
#[test]
fn only_owner_can_publish_revision_or_request_help_and_optout_closes_it() {
    let s = state();
    let mut authority = Authority::default();
    let a = "a".repeat(64);
    let b = "b".repeat(64);
    let update = Action::UpdateReadiness {
        task: "T1".into(),
        expected_revision: 0,
        profile: profile(),
    };
    assert!(authority.apply(&b, &update, &s).is_err());
    authority.apply(&a, &update, &s).unwrap();
    assert!(authority.apply(&a, &update, &s).is_err());
    let help = Action::RequestHelp {
        task: "T1".into(),
        expected_revision: 1,
        reason: "Need review".into(),
    };
    assert!(authority.apply(&b, &help, &s).is_err());
    authority.apply(&a, &help, &s).unwrap();
    assert!(authority.apply(&a, &help, &s).is_err());
    let mut p = profile();
    p.help_allowed = false;
    authority
        .apply(
            &a,
            &Action::UpdateReadiness {
                task: "T1".into(),
                expected_revision: 1,
                profile: p,
            },
            &s,
        )
        .unwrap();
    assert!(authority.view.help[0].closed);
    assert!(authority.apply(&a, &help, &s).is_err());
    assert_eq!(s.tasks[0].owner_id, a);
    assert_eq!(s.tasks[0].status, TaskStatus::Open);
    let mut p = profile();
    p.files = vec![".env".into()];
    assert!(p.validate().is_err());
    p.files = vec!["../escape".into()];
    assert!(p.validate().is_err());
}
#[test]
fn impact_requires_source_owner_link_and_target_owner_review_after_handoff() {
    let mut s = state();
    let mut authority = Authority::default();
    let a = "a".repeat(64);
    let b = "b".repeat(64);
    let c = "c".repeat(64);
    authority
        .apply(
            &a,
            &Action::UpdateReadiness {
                task: "T1".into(),
                expected_revision: 0,
                profile: profile(),
            },
            &s,
        )
        .unwrap();
    let impact = Action::DeclareImpact {
        revision: "a".repeat(40),
        source: "T1".into(),
        target: "T2".into(),
        file: "src/api.rs".into(),
        summary: "Response field renamed; update UI mapping".into(),
    };
    assert!(authority.apply(&b, &impact, &s).is_err());
    authority.apply(&a, &impact, &s).unwrap();
    assert!(authority.apply(&a, &impact, &s).is_err());
    let ack = Action::AcknowledgeImpact {
        id: 1,
        accepted: false,
    };
    assert!(authority.apply(&a, &ack, &s).is_err());
    authority.apply(&b, &ack, &s).unwrap();
    assert!(assess(
        &s.tasks,
        &authority.view,
        &super::super::Countdown::default(),
        1
    )[1]
    .reasons
    .iter()
    .any(|r| r.contains("disputed")));
    s.tasks[1].owner_id = c.clone();
    assert!(authority.apply(&b, &ack, &s).is_err());
    assert!(assess(
        &s.tasks,
        &authority.view,
        &super::super::Countdown::default(),
        1
    )[1]
    .reasons
    .iter()
    .any(|r| r.contains("current-owner review")));
    authority
        .apply(
            &c,
            &Action::AcknowledgeImpact {
                id: 1,
                accepted: true,
            },
            &s,
        )
        .unwrap();
    s.tasks[0].owner_id = c;
    assert!(
        !assess(
            &s.tasks,
            &authority.view,
            &super::super::Countdown::default(),
            1
        )[0]
        .help_allowed
    );
}
#[test]
fn critical_path_uses_longest_parallel_branch_and_unknowns_never_claim_readiness() {
    let mut s = state();
    let actor = "a".repeat(64);
    s.tasks = vec![
        task("T1", &actor, vec![]),
        task("T2", &actor, vec![]),
        task("T3", &actor, vec!["T1".into(), "T2".into()]),
    ];
    let mut authority = Authority::default();
    for id in ["T1", "T2", "T3"] {
        authority
            .apply(
                &actor,
                &Action::UpdateReadiness {
                    task: id.into(),
                    expected_revision: 0,
                    profile: profile(),
                },
                &s,
            )
            .unwrap();
    }
    let paused = super::super::Countdown {
        remaining_seconds: 1,
        running: false,
    };
    let a = assess(&s.tasks, &authority.view, &paused, 1000);
    assert_eq!(a[2].critical_minutes, Some(60));
    assert_eq!(a[2].budget_seconds, Some(4000));
    assert!(a[2].gaps.iter().any(|r| r.contains("Checks: unknown")));
    let running = super::super::Countdown {
        remaining_seconds: 60,
        running: true,
    };
    assert!(assess(&s.tasks, &authority.view, &running, 1000)[2]
        .reasons
        .iter()
        .any(|r| r.contains("exceeds")));
    s.tasks[0].status = TaskStatus::Done;
    assert_eq!(
        assess(&s.tasks, &authority.view, &paused, 1000)
            .last()
            .unwrap()
            .critical_minutes,
        Some(60)
    );
    s.tasks[1].dependencies = vec!["T3".into()];
    assert!(assess(&s.tasks, &authority.view, &paused, 1000)
        .iter()
        .all(|a| a.critical_minutes.is_none()));
}
#[test]
fn older_snapshots_default_to_unsupported_and_invalid_records_are_rejected() {
    let mut value = serde_json::to_value(super::super::SharedSnapshot::default()).unwrap();
    value.as_object_mut().unwrap().remove("readiness");
    let decoded: super::super::SharedSnapshot = serde_json::from_value(value).unwrap();
    assert!(!decoded.readiness.supported);
    let mut authority = Authority::default();
    let s = state();
    authority
        .apply(
            &"a".repeat(64),
            &Action::UpdateReadiness {
                task: "T1".into(),
                expected_revision: 0,
                profile: profile(),
            },
            &s,
        )
        .unwrap();
    authority.view.validate(&s.tasks).unwrap();
    authority
        .view
        .profiles
        .push(authority.view.profiles[0].clone());
    assert!(authority.view.validate(&s.tasks).is_err());
}

#[test]
fn overlapping_dependency_graph_is_memoized_and_recommendations_respect_optout() {
    let actor = "a".repeat(64);
    let mut s = state();
    s.tasks = (1i32..=100)
        .map(|i| {
            task(
                &format!("T{i}"),
                &actor,
                ((i - 2).max(1)..i).map(|d| format!("T{d}")).collect(),
            )
        })
        .collect();
    let mut authority = Authority::default();
    for i in 1..=100 {
        let mut p = profile();
        p.estimate_minutes = Some(1);
        authority
            .apply(
                &actor,
                &Action::UpdateReadiness {
                    task: format!("T{i}"),
                    expected_revision: 0,
                    profile: p,
                },
                &s,
            )
            .unwrap();
    }
    let a = assess(
        &s.tasks,
        &authority.view,
        &super::super::Countdown {
            remaining_seconds: 10,
            running: true,
        },
        1000,
    );
    assert_eq!(a.last().unwrap().critical_minutes, Some(100));
    assert!(a.last().unwrap().recommendation.is_some());
    let mut p = profile();
    p.help_allowed = false;
    authority
        .apply(
            &actor,
            &Action::UpdateReadiness {
                task: "T100".into(),
                expected_revision: 1,
                profile: p,
            },
            &s,
        )
        .unwrap();
    assert!(assess(
        &s.tasks,
        &authority.view,
        &super::super::Countdown {
            remaining_seconds: 10,
            running: true
        },
        1000
    )
    .last()
    .unwrap()
    .recommendation
    .is_none());
}
