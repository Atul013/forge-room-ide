use super::*;
use crate::transport::{
    Action, Client, Host, ReadinessProfile, ShipCheck, ShipResult, TaskReadiness,
};
use std::time::Duration;

fn task(id: &str, dependencies: &[&str]) -> Task {
    Task {
        id: id.into(),
        title: format!("Work {id}"),
        owner_id: "a".repeat(64),
        owner_name: "Ada".into(),
        status: TaskStatus::Open,
        priority: TaskPriority::Normal,
        dependencies: dependencies.iter().map(|s| (*s).into()).collect(),
        acceptance_criteria: vec![],
        completed_criteria: vec![],
        worktree_branch: Some("PRIVATE_LOCAL_BRANCH".into()),
        due_at_epoch_seconds: None,
        updated_at_epoch_seconds: 1,
    }
}
pub(super) fn fixture() -> Snapshot {
    let mut s = Snapshot {
        room: "fixture-room".into(),
        server_time_epoch_seconds: 1000,
        ..Default::default()
    };
    s.shared.room_id = "fixture-room-id".into();
    s.shared.tasks = vec![task("T1", &[]), task("T2", &["T1"]), task("T3", &[])];
    s.shared.tasks[2].priority = TaskPriority::Critical;
    s.shared.shipping.supported = true;
    s.shared.shipping.revision = 1;
    s.shared.shipping.plan.title = "Release".into();
    s.shared.shipping.plan.required_tasks = vec!["T1".into()];
    s.shared.readiness.supported = true;
    s.shared.readiness.profiles = s
        .shared
        .tasks
        .iter()
        .map(|t| TaskReadiness {
            task: t.id.clone(),
            revision: 1,
            owner: t.owner_id.clone(),
            updated: 1,
            profile: ReadinessProfile {
                estimate_minutes: Some(20),
                help_allowed: true,
                ..Default::default()
            },
        })
        .collect();
    s
}
fn report(s: &Snapshot) -> RescueReport {
    rescue_report(
        s,
        &["T2".into()],
        &Countdown {
            remaining_seconds: 1800,
            running: true,
        },
        1000,
    )
    .unwrap()
}

#[test]
fn closure_handles_shared_completed_prerequisites_and_rejects_missing_cycles_and_duplicates() {
    let mut s = fixture();
    s.shared.tasks[0].status = TaskStatus::Done;
    assert_eq!(
        rescue_scope(&s.shared.tasks, &["T2".into(), "T1".into()]).unwrap(),
        vec!["T1", "T2"]
    );
    assert!(rescue_scope(&s.shared.tasks, &[]).is_err());
    assert!(rescue_scope(&s.shared.tasks, &["T2".into(), "T2".into()]).is_err());
    s.shared.tasks[0].dependencies.push("T2".into());
    assert!(rescue_scope(&s.shared.tasks, &["T2".into()]).is_err());
    s.shared.tasks[0].dependencies = vec!["missing".into()];
    assert!(rescue_scope(&s.shared.tasks, &["T2".into()]).is_err());
}
#[test]
fn estimates_unknowns_paused_time_and_optout_do_not_become_completion_or_dispatch() {
    let mut s = fixture();
    let r = report(&s);
    let t2 = r.rows.iter().find(|r| r.task == "T2").unwrap();
    assert_eq!(t2.critical_minutes, Some(40));
    assert_eq!(t2.budget_seconds, Some(1800));
    assert!(t2.warnings.iter().any(|s| s.contains("exceeds")));
    assert!(r
        .blockers
        .iter()
        .any(|s| s.contains("T3") && s.contains("critical")));
    assert!(t2.assistance.contains("no open"));
    s.shared.readiness.profiles[1].owner = "b".repeat(64);
    let r = rescue_report(
        &s,
        &["T2".into()],
        &Countdown {
            remaining_seconds: 0,
            running: false,
        },
        1000,
    )
    .unwrap();
    let t2 = r.rows.iter().find(|r| r.task == "T2").unwrap();
    assert_eq!(t2.critical_minutes, None);
    assert_eq!(t2.budget_seconds, None);
    assert!(t2.assistance.contains("not published"));
    assert_eq!(s.shared.tasks[1].status, TaskStatus::Open);
}
#[test]
fn consent_and_open_help_must_belong_to_current_owner() {
    let mut s = fixture();
    s.shared.readiness.help.push(super::super::HelpRequest {
        task: "T2".into(),
        actor: "b".repeat(64),
        reason: "old owner".into(),
        opened: 1,
        closed: false,
    });
    assert!(report(&s)
        .rows
        .iter()
        .find(|r| r.task == "T2")
        .unwrap()
        .assistance
        .contains("no open"));
    s.shared.readiness.help[0].actor = "a".repeat(64);
    assert!(report(&s)
        .rows
        .iter()
        .find(|r| r.task == "T2")
        .unwrap()
        .assistance
        .contains("requests assistance"));
    s.shared.readiness.profiles[1].profile.help_allowed = false;
    assert!(report(&s)
        .rows
        .iter()
        .find(|r| r.task == "T2")
        .unwrap()
        .assistance
        .contains("opted out"));
}
#[test]
fn public_projection_roundtrips_and_imports_reject_inconsistent_unknown_and_oversized_fields() {
    let s = fixture();
    let r = report(&s);
    let json = r.json().unwrap();
    assert!(!json.contains("PRIVATE_LOCAL_BRANCH"));
    assert!(!json.contains("acceptanceCriteria"));
    assert_eq!(RescueReport::parse(json.as_bytes()).unwrap(), r);
    let mut value = serde_json::to_value(&r).unwrap();
    value["private_token"] = "secret".into();
    assert!(RescueReport::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut bad = r.clone();
    bad.required.retain(|s| s != "T1");
    assert!(bad.json().is_err());
    let mut bad = r.clone();
    bad.rows[0].title = "x".repeat(2049);
    assert!(bad.json().is_err());
    let mut bad = r.clone();
    bad.rows
        .iter_mut()
        .find(|row| row.task == "T3")
        .unwrap()
        .dependencies
        .push("T3".into());
    assert!(bad.json().is_err());
    let mut bad = r.clone();
    bad.rows[0].title = "control\ntext".into();
    assert!(bad.json().is_err());
    assert!(RescueReport::parse(&vec![b' '; MAX_RESCUE_REPORT + 1]).is_err());
}
#[test]
fn comparison_detects_time_scope_risk_and_owner_changes_and_refuses_other_rooms() {
    let mut s = fixture();
    let old = report(&s);
    assert!(old.compare(&old).unwrap().is_empty());
    s.shared.readiness.profiles[1].profile.estimate_minutes = Some(50);
    let newer = report(&s);
    assert!(old
        .compare(&newer)
        .unwrap()
        .iter()
        .any(|s| s.starts_with("T2:")));
    s.shared.room_id = "other".into();
    assert!(old.compare(&report(&s)).is_err());
}
#[test]
fn anchor_tracks_readiness_tasks_and_ship_but_not_clock_or_private_content() {
    let mut s = fixture();
    let initial = report(&s).anchor;
    s.server_time_epoch_seconds += 50;
    s.shared.countdown.running = true;
    assert_eq!(initial, report(&s).anchor);
    s.shared.readiness.profiles[1].profile.help_allowed = false;
    assert_ne!(initial, report(&s).anchor);
}

#[test]
fn workload_counts_shared_prerequisites_once_and_keeps_unknown_owner_estimates_explicit() {
    let mut s = fixture();
    let r = report(&s);
    assert_eq!(r.owners.len(), 1);
    assert_eq!(r.owners[0].known_minutes, 40);
    assert_eq!(r.owners[0].tasks, vec!["T1", "T2"]);
    assert!(r
        .blockers
        .iter()
        .any(|b| b.contains("workload") && b.contains("exceeds")));
    s.shared.readiness.profiles[0].profile.estimate_minutes = None;
    let r = report(&s);
    assert_eq!(r.owners[0].known_minutes, 20);
    assert_eq!(r.owners[0].unknown_estimates, 1);
    let mut bad = r;
    bad.owners[0].known_minutes = 0;
    assert!(bad.validate().is_err());
}

#[test]
fn report_scope_previews_check_reset_and_same_named_rooms_cannot_share_anchors_or_comparisons() {
    let mut s = fixture();
    s.shared.shipping.plan.checks.push(ShipCheck {
        title: "Smoke".into(),
        owner: "a".repeat(64),
    });
    s.shared.shipping.reports.push(super::super::ShipReport {
        check: 0,
        actor: "a".repeat(64),
        result: ShipResult::ClaimedPass,
        note: "old scope pass".into(),
        recorded_at: 1,
    });
    let r = report(&s);
    assert!(r
        .blockers
        .iter()
        .any(|b| b.contains("Smoke: no claimed-pass")));
    let anchor = r.anchor.clone();
    s.shared.room_id = "distinct-room-same-name".into();
    let other = report(&s);
    assert_ne!(anchor, other.anchor);
    assert!(r.compare(&other).is_err());
    assert!(scope_plan(
        &s.shared.room_id,
        &s.shared.tasks,
        &s.shared.readiness,
        &s.shared.shipping,
        &anchor,
        &["T2".into()]
    )
    .is_err());
    let mut bad = other;
    bad.rows
        .iter_mut()
        .find(|r| r.task == "T1")
        .unwrap()
        .dependencies
        .push("T2".into());
    assert!(bad.validate().is_err());
}

#[test]
fn tls_host_only_scope_is_dependency_closed_preserves_plan_resets_reports_and_rejects_stale_review()
{
    let host = Host::start("127.0.0.1:0".parse().unwrap(), "rescue").unwrap();
    let (mut owner, _) =
        Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Ada").unwrap();
    for dependencies in [vec![], vec!["T1".into()], vec![]] {
        host.control(Action::CreateTask {
            title: "Work".into(),
            owner: owner.identity.clone(),
            criteria: vec!["verified".into()],
            dependencies,
            priority: TaskPriority::Normal,
            due_hours: None,
        })
        .unwrap();
    }
    let plan = ShipPlan {
        title: "Release".into(),
        required_tasks: vec!["T1".into(), "T2".into(), "T3".into()],
        checks: vec![ShipCheck {
            title: "Smoke".into(),
            owner: owner.identity.clone(),
        }],
        preview_reference: "preview".into(),
        submission_reference: "receipt".into(),
    };
    host.control(Action::SetShipPlan {
        expected_revision: 0,
        plan: plan.clone(),
    })
    .unwrap();
    assert!(owner
        .execute(Action::ReportShipCheck {
            expected_revision: 1,
            check: 0,
            result: ShipResult::ClaimedPass,
            note: "manual".into()
        })
        .unwrap()
        .rejection
        .is_none());
    let before = host.snapshot().unwrap();
    let anchor = rescue_anchor(
        &before.shared.room_id,
        &before.shared.tasks,
        &before.shared.readiness,
        &before.shared.shipping,
    )
    .unwrap();
    let action = Action::SetRescueScope {
        expected_anchor: anchor,
        selected: vec!["T2".into()],
    };
    assert!(owner.execute(action.clone()).unwrap().rejection.is_some());
    host.control(Action::Countdown {
        seconds: Some(1800),
        running: true,
    })
    .unwrap();
    host.control(action.clone()).unwrap();
    let after = host.snapshot().unwrap();
    assert_eq!(after.shared.shipping.plan.required_tasks, vec!["T1", "T2"]);
    let mut expected = plan;
    expected.required_tasks = vec!["T1".into(), "T2".into()];
    assert_eq!(after.shared.shipping.plan, expected);
    assert!(after.shared.shipping.reports.is_empty());
    assert_eq!(after.shared.tasks, before.shared.tasks);
    assert!(host.control(action).is_err());
    let fresh = Action::SetRescueScope {
        expected_anchor: rescue_anchor(
            &after.shared.room_id,
            &after.shared.tasks,
            &after.shared.readiness,
            &after.shared.shipping,
        )
        .unwrap(),
        selected: vec!["T3".into()],
    };
    assert!(owner
        .execute(Action::UpdateReadiness {
            task: "T1".into(),
            expected_revision: 0,
            profile: ReadinessProfile::default()
        })
        .unwrap()
        .rejection
        .is_none());
    assert!(host.control(fresh).is_err());
    assert_eq!(host.snapshot().unwrap().shared.shipping.plan, expected);
}
