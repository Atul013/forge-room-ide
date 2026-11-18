//! Shared, human-declared shipping scope. This grants no execution authority.
use super::{Action, Snapshot};
use crate::{now_epoch_seconds, RoomState, Task, TaskStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipCheck {
    pub title: String,
    pub owner: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipPlan {
    pub title: String,
    pub required_tasks: Vec<String>,
    pub checks: Vec<ShipCheck>,
    pub preview_reference: String,
    pub submission_reference: String,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipResult {
    #[default]
    Unknown,
    ClaimedPass,
    ClaimedFail,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShipReport {
    pub check: usize,
    pub actor: String,
    pub result: ShipResult,
    pub note: String,
    pub recorded_at: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ShipState {
    pub supported: bool,
    pub revision: u64,
    pub plan: ShipPlan,
    pub reports: Vec<ShipReport>,
}
fn text(value: &str, max: usize, required: bool) -> Result<(), String> {
    if value.len() > max
        || value.chars().any(char::is_control)
        || (required && value.trim().is_empty())
    {
        return Err("Invalid Ship text: check length and remove control characters".into());
    }
    Ok(())
}
impl ShipPlan {
    fn validate(&self, tasks: &[Task], configured: bool) -> Result<(), String> {
        text(&self.title, 160, configured)?;
        text(&self.preview_reference, 500, false)?;
        text(&self.submission_reference, 500, false)?;
        if self.required_tasks.len() > 100 || self.checks.len() > 24 {
            return Err("Ship plan limit exceeded".into());
        }
        let mut seen = std::collections::HashSet::new();
        for id in &self.required_tasks {
            if !seen.insert(id) || !tasks.iter().any(|task| &task.id == id) {
                return Err("Required task is missing or repeated".into());
            }
        }
        seen.clear();
        for check in &self.checks {
            text(&check.title, 160, true)?;
            if check.owner.len() != 64
                || !check.owner.bytes().all(|b| b.is_ascii_hexdigit())
                || !seen.insert(&check.title)
            {
                return Err("Invalid or duplicate Ship check".into());
            }
        }
        Ok(())
    }
}
impl ShipState {
    pub fn validate(&self, tasks: &[Task]) -> Result<(), String> {
        self.plan.validate(tasks, self.revision > 0)?;
        if self.reports.len() > self.plan.checks.len() {
            return Err("Too many Ship reports".into());
        }
        let mut seen = std::collections::HashSet::new();
        for report in &self.reports {
            let check = self
                .plan
                .checks
                .get(report.check)
                .ok_or("Unknown Ship check")?;
            if report.actor != check.owner || !seen.insert(report.check) {
                return Err("Invalid Ship report owner".into());
            }
            text(&report.note, 1000, report.result != ShipResult::Unknown)?;
        }
        Ok(())
    }
    pub(super) fn apply(
        &mut self,
        actor: &str,
        action: &Action,
        room: &RoomState,
    ) -> Result<bool, String> {
        match action {
            Action::SetShipPlan {
                expected_revision,
                plan,
            } => {
                if actor != room.host_id {
                    return Err("Only the host can define release scope".into());
                }
                self.check_revision(*expected_revision)?;
                plan.validate(&room.tasks, true)?;
                if plan
                    .checks
                    .iter()
                    .any(|check| !room.participants.iter().any(|p| p.id == check.owner))
                {
                    return Err("Assign checks to a currently joined member".into());
                }
                self.plan = plan.clone();
                self.reports.clear();
            }
            Action::ReportShipCheck {
                expected_revision,
                check,
                result,
                note,
            } => {
                self.check_revision(*expected_revision)?;
                let item = self.plan.checks.get(*check).ok_or("Ship check not found")?;
                if item.owner != actor {
                    return Err("Only the assigned owner can report this check".into());
                }
                text(note, 1000, *result != ShipResult::Unknown)?;
                self.reports.retain(|report| report.check != *check);
                self.reports.push(ShipReport {
                    check: *check,
                    actor: actor.into(),
                    result: *result,
                    note: note.clone(),
                    recorded_at: now_epoch_seconds(),
                });
            }
            _ => return Ok(false),
        }
        self.revision += 1;
        Ok(true)
    }
    fn check_revision(&self, expected: u64) -> Result<(), String> {
        if expected != self.revision || self.revision == u64::MAX {
            return Err("Ship plan changed; reload before publishing".into());
        }
        Ok(())
    }
}

/// Deterministic public review. No private messages or patches are inspected.
pub fn ship_blockers(snapshot: &Snapshot) -> Vec<String> {
    let state = &snapshot.shared.shipping;
    let mut issues = Vec::new();
    if state.revision == 0 {
        issues.push("Release scope has not been published".into());
    }
    if state.plan.required_tasks.is_empty() {
        issues.push("No required tasks selected".into());
    }
    if state.plan.checks.is_empty() {
        issues.push("No release checks assigned".into());
    }
    if state.plan.preview_reference.trim().is_empty() {
        issues.push("Preview reference missing".into());
    }
    if state.plan.submission_reference.trim().is_empty() {
        issues.push("Submission reference missing".into());
    }
    for task in &snapshot.shared.tasks {
        if task.priority == crate::TaskPriority::Critical && task.status != TaskStatus::Done {
            issues.push(format!("{}: critical task remains unfinished", task.id));
        }
    }
    for id in &state.plan.required_tasks {
        let Some(task) = snapshot.shared.tasks.iter().find(|task| &task.id == id) else {
            issues.push(format!("Required task {id} is missing"));
            continue;
        };
        if task.status != TaskStatus::Done {
            issues.push(format!("{id}: required task is {:?}", task.status));
            if task
                .due_at_epoch_seconds
                .is_some_and(|due| due <= snapshot.server_time_epoch_seconds)
            {
                issues.push(format!("{id}: required task is overdue"));
            }
        }
        for dependency in &task.dependencies {
            if !snapshot
                .shared
                .tasks
                .iter()
                .any(|t| &t.id == dependency && t.status == TaskStatus::Done)
            {
                issues.push(format!(
                    "{id}: dependency {dependency} is unfinished or missing"
                ));
            }
        }
        if let Some(profile) = snapshot.shared.readiness.profile(id) {
            if profile.owner != task.owner_id {
                issues.push(format!("{id}: readiness belongs to the previous owner"));
            }
            if !profile.profile.blocker.trim().is_empty() {
                issues.push(format!("{id}: {}", profile.profile.blocker));
            }
            if profile.profile.checks == super::Declaration::ClaimedFail {
                issues.push(format!("{id}: owner reports failed checks"));
            }
        }
    }
    for impact in &snapshot.shared.readiness.impacts {
        let acknowledged_by_current_owner = snapshot
            .shared
            .tasks
            .iter()
            .find(|task| task.id == impact.target)
            .is_some_and(|task| impact.acknowledged_by.as_ref() == Some(&task.owner_id));
        if state.plan.required_tasks.contains(&impact.target)
            && (impact.accepted != Some(true) || !acknowledged_by_current_owner)
        {
            issues.push(format!(
                "Impact {} on {} requires resolution",
                impact.id, impact.target
            ));
        }
    }
    for (index, check) in state.plan.checks.iter().enumerate() {
        if !snapshot
            .members
            .iter()
            .any(|member| member.id == check.owner && member.online)
        {
            issues.push(format!(
                "{}: assigned member is offline or no longer joined",
                check.title
            ));
        }
        if !state
            .reports
            .iter()
            .any(|report| report.check == index && report.result == ShipResult::ClaimedPass)
        {
            issues.push(format!("{}: no claimed-pass report", check.title));
        }
    }
    issues
}

/// Explicit public projection; never serialize the full recipient snapshot here.
pub fn ship_handoff(snapshot: &Snapshot) -> Result<String, String> {
    let tasks: Vec<_> = snapshot.shared.tasks.iter().filter(|task| snapshot.shared.shipping.plan.required_tasks.contains(&task.id)).map(|task| serde_json::json!({
        "id":task.id,"title":task.title,"owner":task.owner_id,"owner_name":task.owner_name,"status":task.status,"dependencies":task.dependencies
    })).collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "format":"forge-room-ship-handoff-v1", "evidence":"Manual declarations; no independent verification",
        "host_time":snapshot.server_time_epoch_seconds, "shipping":snapshot.shared.shipping,
        "required_tasks":tasks, "blockers":ship_blockers(snapshot)
    })).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Participant, Role, TaskPriority};
    fn room() -> super::super::workflow::SharedRoom {
        let mut room = super::super::workflow::SharedRoom::new().unwrap();
        room.state.participants.push(Participant {
            id: "a".repeat(64),
            name: "Owner".into(),
            role: Role::Human,
            owner_id: None,
        });
        room.apply(
            &room.state.host_id.clone(),
            Action::CreateTask {
                title: "Release".into(),
                owner: "a".repeat(64),
                criteria: vec!["tested".into()],
                dependencies: vec![],
                priority: TaskPriority::High,
                due_hours: None,
            },
        )
        .unwrap();
        room
    }
    fn plan(room: &super::super::workflow::SharedRoom) -> ShipPlan {
        ShipPlan {
            title: "Release one".into(),
            required_tasks: vec![room.state.tasks[0].id.clone()],
            checks: vec![ShipCheck {
                title: "Smoke test".into(),
                owner: "a".repeat(64),
            }],
            preview_reference: "preview build 7".into(),
            submission_reference: String::new(),
        }
    }
    #[test]
    fn host_scope_owner_reports_stale_guards_and_reset_are_atomic() {
        let mut room = room();
        let host = room.state.host_id.clone();
        let plan = plan(&room);
        let edit = Action::SetShipPlan {
            expected_revision: 0,
            plan: plan.clone(),
        };
        assert!(room.apply(&"a".repeat(64), edit.clone()).is_err());
        room.apply(&host, edit).unwrap();
        let report = Action::ReportShipCheck {
            expected_revision: 1,
            check: 0,
            result: ShipResult::ClaimedPass,
            note: "Manual smoke run passed".into(),
        };
        assert!(room.apply(&host, report.clone()).is_err());
        room.apply(&"a".repeat(64), report.clone()).unwrap();
        assert!(room.apply(&"a".repeat(64), report).is_err());
        let before = serde_json::to_string(&room.snapshot().shipping).unwrap();
        let mut invalid = plan.clone();
        invalid.checks.push(invalid.checks[0].clone());
        assert!(room
            .apply(
                &host,
                Action::SetShipPlan {
                    expected_revision: 2,
                    plan: invalid
                }
            )
            .is_err());
        assert_eq!(
            before,
            serde_json::to_string(&room.snapshot().shipping).unwrap()
        );
        room.apply(
            &host,
            Action::SetShipPlan {
                expected_revision: 2,
                plan,
            },
        )
        .unwrap();
        assert_eq!(room.snapshot().shipping.revision, 3);
        assert!(room.snapshot().shipping.reports.is_empty());
    }
    #[test]
    fn review_distinguishes_manual_passes_critical_work_and_stale_impact_owners() {
        let mut room = room();
        let host = room.state.host_id.clone();
        let mut plan = plan(&room);
        plan.submission_reference = "Submission receipt 42".into();
        room.apply(
            &host,
            Action::SetShipPlan {
                expected_revision: 0,
                plan,
            },
        )
        .unwrap();
        room.apply(
            &"a".repeat(64),
            Action::ReportShipCheck {
                expected_revision: 1,
                check: 0,
                result: ShipResult::ClaimedPass,
                note: "Manual smoke passed".into(),
            },
        )
        .unwrap();
        let mut snapshot = Snapshot {
            shared: room.snapshot(),
            members: vec![super::super::Member {
                id: "a".repeat(64),
                name: "Owner".into(),
                online: true,
            }],
            ..Default::default()
        };
        snapshot.shared.tasks[0].status = TaskStatus::Done;
        assert!(ship_blockers(&snapshot).is_empty());
        snapshot.members[0].online = false;
        assert!(ship_blockers(&snapshot)
            .iter()
            .any(|issue| issue.contains("offline")));
        snapshot.members[0].online = true;
        let mut critical = snapshot.shared.tasks[0].clone();
        critical.id = "T99".into();
        critical.priority = TaskPriority::Critical;
        critical.status = TaskStatus::Open;
        snapshot.shared.tasks.push(critical);
        assert!(ship_blockers(&snapshot)
            .iter()
            .any(|issue| issue.contains("critical")));
        snapshot
            .shared
            .readiness
            .impacts
            .push(super::super::Impact {
                revision: "a".repeat(40),
                id: 1,
                source: "T99".into(),
                target: snapshot.shared.tasks[0].id.clone(),
                file: "public.rs".into(),
                summary: "Review API".into(),
                actor: "a".repeat(64),
                opened: 1,
                acknowledged_by: Some("b".repeat(64)),
                accepted: Some(true),
            });
        assert!(ship_blockers(&snapshot)
            .iter()
            .any(|issue| issue.contains("Impact")));
    }

    #[test]
    fn invalid_references_limits_and_old_snapshot_compatibility() {
        let room = room();
        let mut plan = plan(&room);
        plan.required_tasks.push("missing".into());
        assert!(plan.validate(&room.state.tasks, true).is_err());
        plan.required_tasks.pop();
        plan.preview_reference = "x".repeat(501);
        assert!(plan.validate(&room.state.tasks, true).is_err());
        let mut value = serde_json::to_value(room.snapshot()).unwrap();
        value.as_object_mut().unwrap().remove("shipping");
        let older: super::super::SharedSnapshot = serde_json::from_value(value).unwrap();
        assert!(!older.shipping.supported);
        older.shipping.validate(&older.tasks).unwrap();
    }
}
