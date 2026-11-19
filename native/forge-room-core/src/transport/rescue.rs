//! Pure deadline triage over public coordination data. No execution authority.
use super::{assess, Countdown, ReadinessState, ShipPlan, ShipState, Snapshot};
use crate::{Task, TaskPriority, TaskStatus};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub const MAX_RESCUE_REPORT: usize = 512 * 1024;
const EVIDENCE: &str = "Manual owner declarations and estimates; desktop, real-provider and physical two-client acceptance require separate evidence";

/// Deliberately excludes clocks, messages, patches, events and private local histories.
/// Clock observations belong to each frozen report, not to the scope mutation guard.
pub fn rescue_anchor(
    room_id: &str,
    tasks: &[Task],
    readiness: &ReadinessState,
    ship: &ShipState,
) -> Result<String, String> {
    crate::workflows::seal(&(room_id, tasks, readiness, ship))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RescueRow {
    pub task: String,
    pub title: String,
    pub owner: String,
    pub owner_name: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub selected: bool,
    pub prerequisite: bool,
    pub dependencies: Vec<String>,
    pub remaining_minutes: Option<u32>,
    pub critical_minutes: Option<u64>,
    pub budget_seconds: Option<u64>,
    pub warnings: Vec<String>,
    pub unknowns: Vec<String>,
    pub assistance: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RescueReport {
    pub version: u32,
    pub evidence: String,
    pub room: String,
    pub anchor: String,
    pub observed_at: u64,
    pub countdown_seconds: u64,
    pub countdown_running: bool,
    pub ship_revision: u64,
    pub selected: Vec<String>,
    pub required: Vec<String>,
    pub deferred: Vec<String>,
    pub rows: Vec<RescueRow>,
    pub owners: Vec<RescueOwner>,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RescueOwner {
    pub owner: String,
    pub tasks: Vec<String>,
    pub known_minutes: u64,
    pub unknown_estimates: u32,
}
fn owner_work(rows: &[RescueRow], required: &[String]) -> Vec<RescueOwner> {
    let mut owners = BTreeMap::new();
    for row in rows
        .iter()
        .filter(|r| required.contains(&r.task) && r.status != TaskStatus::Done)
    {
        let owner = owners
            .entry(row.owner.clone())
            .or_insert_with(|| RescueOwner {
                owner: row.owner.clone(),
                tasks: vec![],
                known_minutes: 0,
                unknown_estimates: 0,
            });
        owner.tasks.push(row.task.clone());
        if let Some(minutes) = row.remaining_minutes {
            owner.known_minutes += u64::from(minutes);
        } else {
            owner.unknown_estimates += 1;
        }
    }
    for owner in owners.values_mut() {
        owner.tasks.sort();
    }
    owners.into_values().collect()
}

/// Resolve transitive dependencies in a stable order; missing/cyclic input fails closed.
pub fn rescue_scope(tasks: &[Task], selected: &[String]) -> Result<Vec<String>, String> {
    scope(
        &tasks
            .iter()
            .map(|t| (t.id.as_str(), t.dependencies.as_slice()))
            .collect::<Vec<_>>(),
        selected,
    )
}
fn scope(graph: &[(&str, &[String])], selected: &[String]) -> Result<Vec<String>, String> {
    if graph.len() > 100 || selected.is_empty() || selected.len() > 100 {
        return Err("Choose 1–100 release targets from at most 100 tasks".into());
    }
    let unique: HashSet<_> = graph.iter().map(|t| t.0).collect();
    if unique.len() != graph.len()
        || selected.iter().collect::<HashSet<_>>().len() != selected.len()
    {
        return Err("Duplicate task or release target".into());
    }
    fn visit(
        id: &str,
        graph: &[(&str, &[String])],
        active: &mut HashSet<String>,
        done: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if done.contains(id) {
            return Ok(());
        }
        if !active.insert(id.into()) {
            return Err(format!("Dependency cycle at {id}"));
        }
        let (_, dependencies) = graph
            .iter()
            .find(|t| t.0 == id)
            .ok_or_else(|| format!("Missing dependency or target {id}"))?;
        // Finished prerequisites still belong to the release's explainable dependency closure.
        for dependency in *dependencies {
            visit(dependency, graph, active, done)?;
        }
        active.remove(id);
        done.insert(id.into());
        Ok(())
    }
    let mut done = BTreeSet::new();
    for id in selected {
        visit(id, graph, &mut HashSet::new(), &mut done)?;
    }
    Ok(done.into_iter().collect())
}

pub fn rescue_report(
    snapshot: &Snapshot,
    selected: &[String],
    clock: &Countdown,
    now: u64,
) -> Result<RescueReport, String> {
    let tasks = &snapshot.shared.tasks;
    let required = rescue_scope(tasks, selected)?;
    let readiness = &snapshot.shared.readiness;
    let assessments = assess(tasks, readiness, clock, now);
    let mut rows = Vec::new();
    for task in tasks {
        let a = assessments.iter().find(|a| a.task == task.id);
        let current = readiness
            .profile(&task.id)
            .filter(|p| p.owner == task.owner_id);
        let request = readiness
            .help
            .iter()
            .any(|h| h.task == task.id && h.actor == task.owner_id && !h.closed);
        let assistance = if task.status == TaskStatus::Done {
            "Completed; assistance is not requested"
        } else if current.is_none_or(|p| !p.profile.help_allowed) {
            "Owner opted out or has not published current consent"
        } else if request {
            "Current owner requests assistance; agree a narrow handoff separately"
        } else {
            "Owner permits recommendations; no open assistance request"
        };
        rows.push(RescueRow {
            task: task.id.clone(),
            title: task.title.clone(),
            owner: task.owner_id.clone(),
            owner_name: task.owner_name.clone(),
            status: task.status,
            priority: task.priority,
            selected: selected.contains(&task.id),
            prerequisite: required.contains(&task.id) && !selected.contains(&task.id),
            dependencies: task.dependencies.clone(),
            remaining_minutes: if task.status == TaskStatus::Done {
                Some(0)
            } else {
                current.and_then(|p| p.profile.estimate_minutes)
            },
            critical_minutes: a.and_then(|a| a.critical_minutes),
            budget_seconds: a.and_then(|a| a.budget_seconds),
            warnings: a.map_or_else(Vec::new, |a| a.reasons.clone()),
            unknowns: a.map_or_else(Vec::new, |a| a.gaps.clone()),
            assistance: assistance.into(),
        });
    }
    // Required critical work first, then required warnings, then optional work. Stable ties.
    rows.sort_by_key(|r| {
        (
            !required.contains(&r.task),
            match r.priority {
                TaskPriority::Critical => 0,
                TaskPriority::High => 1,
                TaskPriority::Normal => 2,
            },
            r.warnings.is_empty(),
            r.task.clone(),
        )
    });
    let mut scenario = snapshot.clone();
    scenario.shared.shipping.plan.required_tasks = required.clone();
    let mut published = snapshot.shared.shipping.plan.required_tasks.clone();
    published.sort();
    if published != required {
        // Preview the same report reset the authoritative scope action will perform.
        scenario.shared.shipping.reports.clear();
    }
    let mut blockers = super::ship_blockers(&scenario);
    if !readiness.supported {
        blockers.push("Host does not advertise readiness coordination".into());
    }
    let deferred: Vec<_> = tasks
        .iter()
        .filter(|t| !required.contains(&t.id))
        .map(|t| t.id.clone())
        .collect();
    for row in rows
        .iter()
        .filter(|r| required.contains(&r.task) && r.status != TaskStatus::Done)
    {
        if !row.warnings.is_empty() || !row.unknowns.is_empty() {
            blockers.push(format!(
                "{}: {} warnings, {} unknowns (owner declarations)",
                row.task,
                row.warnings.len(),
                row.unknowns.len()
            ));
        }
    }
    let mut selected = selected.to_vec();
    selected.sort();
    let owners = owner_work(&rows, &required);
    for owner in &owners {
        if clock.running && owner.known_minutes.saturating_mul(60) > clock.remaining_seconds {
            blockers.push(format!(
                "Owner {}: known required workload {} min exceeds the countdown (manual estimate)",
                owner.owner, owner.known_minutes
            ));
        }
        if owner.unknown_estimates > 0 {
            blockers.push(format!(
                "Owner {}: {} required task estimate(s) unknown",
                owner.owner, owner.unknown_estimates
            ));
        }
    }
    let report = RescueReport {
        version: 1,
        evidence: EVIDENCE.into(),
        room: snapshot.shared.room_id.clone(),
        anchor: rescue_anchor(
            &snapshot.shared.room_id,
            tasks,
            readiness,
            &snapshot.shared.shipping,
        )?,
        observed_at: now,
        countdown_seconds: clock.remaining_seconds,
        countdown_running: clock.running,
        ship_revision: snapshot.shared.shipping.revision,
        selected,
        required,
        deferred,
        rows,
        owners,
        blockers,
    };
    report.validate()?;
    Ok(report)
}

impl RescueReport {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.evidence != EVIDENCE
            || self.room.len() > 256
            || self.anchor.len() != 64
            || !self.anchor.bytes().all(|b| b.is_ascii_hexdigit())
            || self.rows.len() > 100
            || self.blockers.len() > 512
        {
            return Err("Unsupported rescue report or limits".into());
        }
        let ids: HashSet<_> = self.rows.iter().map(|r| &r.task).collect();
        if ids.len() != self.rows.len()
            || self.selected.is_empty()
            || self.selected.len() > 100
            || self.required.len() > 100
            || self.deferred.len() > 100
        {
            return Err("Invalid rescue task inventory".into());
        }
        for list in [&self.selected, &self.required, &self.deferred] {
            if list.iter().collect::<HashSet<_>>().len() != list.len()
                || list.iter().any(|id| !ids.contains(id))
            {
                return Err("Unknown or duplicate rescue task reference".into());
            }
        }
        if self.selected.iter().any(|id| !self.required.contains(id))
            || self.rows.iter().any(|r| {
                self.required.contains(&r.task) == self.deferred.contains(&r.task)
                    || r.selected != self.selected.contains(&r.task)
                    || r.prerequisite != (self.required.contains(&r.task) && !r.selected)
                    || r.dependencies.len() > 100
                    || r.dependencies.iter().collect::<HashSet<_>>().len() != r.dependencies.len()
                    || r.dependencies.iter().any(|d| !ids.contains(d))
                    || (self.required.contains(&r.task)
                        && r.dependencies.iter().any(|d| !self.required.contains(d)))
                    || r.warnings.len() > 32
                    || r.unknowns.len() > 32
                    || r.task.is_empty()
                    || r.task.len() > 32
                    || !r
                        .task
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                    || r.owner.len() != 64
                    || !r.owner.bytes().all(|b| b.is_ascii_hexdigit())
                    || r.remaining_minutes
                        .is_some_and(|m| m > 10080 || (m == 0 && r.status != TaskStatus::Done))
            })
        {
            return Err("Inconsistent rescue scope".into());
        }
        let graph: Vec<_> = self
            .rows
            .iter()
            .map(|r| (r.task.as_str(), r.dependencies.as_slice()))
            .collect();
        scope(
            &graph,
            &self.rows.iter().map(|r| r.task.clone()).collect::<Vec<_>>(),
        )?;
        if scope(&graph, &self.selected)? != self.required {
            return Err(
                "Required work must be exactly the dependency closure of selected targets".into(),
            );
        }
        if self.owners != owner_work(&self.rows, &self.required) {
            return Err("Owner workloads do not match required work".into());
        }
        // Bound all imported text and reject controls; multiline JSON remains presentation only.
        let value = serde_json::to_value(self).map_err(|e| e.to_string())?;
        fn text_bounds(v: &serde_json::Value) -> bool {
            match v {
                serde_json::Value::String(s) => s.len() <= 2048 && !s.chars().any(char::is_control),
                serde_json::Value::Array(a) => a.iter().all(text_bounds),
                serde_json::Value::Object(o) => o.values().all(text_bounds),
                _ => true,
            }
        }
        if !text_bounds(&value)
            || serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_RESCUE_REPORT
        {
            return Err("Rescue report text/byte limit exceeded".into());
        }
        Ok(())
    }
    pub fn json(&self) -> Result<String, String> {
        self.validate()?;
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        if text.len() > MAX_RESCUE_REPORT {
            return Err("Rescue export exceeds byte limit".into());
        }
        Ok(text)
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_RESCUE_REPORT {
            return Err("Rescue import exceeds byte limit".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn compare(&self, newer: &Self) -> Result<Vec<String>, String> {
        self.validate()?;
        newer.validate()?;
        if self.room != newer.room {
            return Err("Reports describe different Rooms".into());
        }
        let mut changes = Vec::new();
        if self.anchor != newer.anchor {
            changes.push("Public coordination source changed".into());
        }
        if self.required != newer.required {
            changes.push("Required dependency scope changed".into());
        }
        if self.countdown_running != newer.countdown_running
            || self.countdown_seconds != newer.countdown_seconds
        {
            changes.push("Countdown observation changed".into());
        }
        for row in &newer.rows {
            match self.rows.iter().find(|old| old.task == row.task) {
                None => changes.push(format!("{}: added", row.task)),
                Some(old) if old != row => changes.push(format!(
                    "{}: task, estimate, risk, scope or consent changed",
                    row.task
                )),
                _ => {}
            }
        }
        for row in &self.rows {
            if !newer.rows.iter().any(|r| r.task == row.task) {
                changes.push(format!("{}: removed", row.task));
            }
        }
        if self.blockers != newer.blockers {
            changes.push("Release blockers changed".into());
        }
        Ok(changes)
    }
}

/// Host policy reuses ShipPlan authorization/validation; only required_tasks may change.
pub(super) fn scope_plan(
    room_id: &str,
    tasks: &[Task],
    readiness: &ReadinessState,
    ship: &ShipState,
    anchor: &str,
    selected: &[String],
) -> Result<ShipPlan, String> {
    if ship.revision == 0 || rescue_anchor(room_id, tasks, readiness, ship)? != anchor {
        return Err("Rescue source changed or release scope is unpublished; review again".into());
    }
    let required = rescue_scope(tasks, selected)?;
    let mut old = ship.plan.required_tasks.clone();
    old.sort();
    if required == old {
        return Err("Rescue scope is unchanged".into());
    }
    let mut plan = ship.plan.clone();
    plan.required_tasks = required;
    Ok(plan)
}

#[cfg(test)]
mod tests;
