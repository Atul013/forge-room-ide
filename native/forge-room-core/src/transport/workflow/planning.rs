use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedTask {
    pub key: String,
    pub existing: Option<String>,
    pub title: String,
    pub owner: String,
    pub criteria: Vec<String>,
    pub dependencies: Vec<String>,
    pub priority: TaskPriority,
    pub due_hours: Option<u64>,
    #[serde(default)]
    pub clear_due: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskPlan {
    pub version: u32,
    pub tasks: Vec<PlannedTask>,
}

/// Pure public preview; no actions, writes, membership or completion changes occur.
pub fn preview_plan(
    snapshot: &super::super::Snapshot,
    plan: &TaskPlan,
) -> Result<Vec<Task>, String> {
    if plan.version != 1 || plan.tasks.is_empty() || plan.tasks.len() > 100 {
        return Err("Plan needs version 1 and 1–100 task entries".into());
    }
    let old = &snapshot.shared.tasks;
    let mut tasks = old.clone();
    let mut keys = HashMap::new();
    let mut existing = HashSet::new();
    let mut next = old
        .iter()
        .filter_map(|task| task.id.strip_prefix('T')?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    for entry in &plan.tasks {
        if entry.key.is_empty()
            || entry.key.len() > 32
            || !entry
                .key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || keys.contains_key(&entry.key)
        {
            return Err("Plan keys must be unique short identifiers".into());
        }
        if old.iter().any(|task| task.id == entry.key)
            && entry.existing.as_deref() != Some(entry.key.as_str())
        {
            return Err("Plan key conflicts with an existing task ID".into());
        }
        let id = if let Some(id) = &entry.existing {
            if !old.iter().any(|task| &task.id == id) || !existing.insert(id) {
                return Err("Existing tasks must exist and appear once".into());
            }
            id.clone()
        } else {
            next = next.checked_add(1).ok_or("Task sequence exhausted")?;
            let id = format!("T{next}");
            if id.len() > 16 {
                return Err("Task sequence exhausted".into());
            }
            id
        };
        keys.insert(entry.key.clone(), id);
    }
    for entry in &plan.tasks {
        let title = super::super::bounded(&entry.title, 200)?;
        if entry.criteria.len() > 8
            || entry.dependencies.len() > 16
            || entry.due_hours.is_some_and(|hours| hours > 720)
            || (entry.clear_due && entry.due_hours.is_some())
        {
            return Err("Invalid plan checklist, dependency or deadline limits".into());
        }
        let criteria = entry
            .criteria
            .iter()
            .map(|text| super::super::bounded(text, 200))
            .collect::<Result<Vec<_>, _>>()?;
        if criteria.iter().collect::<HashSet<_>>().len() != criteria.len() {
            return Err("Criteria must be unique".into());
        }
        let id = keys.get(&entry.key).ok_or("Missing plan key")?.clone();
        let previous = old.iter().find(|task| task.id == id);
        if previous.is_some_and(|task| task.owner_id != entry.owner) {
            return Err(
                "Use the owner-approved handoff workflow to change an existing owner".into(),
            );
        }
        let owner_name = match previous {
            Some(task) => task.owner_name.clone(),
            None => snapshot
                .members
                .iter()
                .find(|member| member.id == entry.owner)
                .ok_or("Choose a joined owner for every new task")?
                .name
                .clone(),
        };
        let dependencies = entry
            .dependencies
            .iter()
            .map(|key| {
                keys.get(key)
                    .cloned()
                    .or_else(|| {
                        old.iter()
                            .find(|task| &task.id == key)
                            .map(|task| task.id.clone())
                    })
                    .ok_or_else(|| format!("Unknown dependency {key}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if dependencies.contains(&id)
            || dependencies.iter().collect::<HashSet<_>>().len() != dependencies.len()
        {
            return Err("Dependencies must be unique and cannot include the task itself".into());
        }
        let mut task = Task {
            id,
            title,
            owner_id: entry.owner.clone(),
            owner_name,
            status: previous.map_or(TaskStatus::Open, |task| task.status),
            priority: entry.priority,
            dependencies,
            acceptance_criteria: criteria,
            completed_criteria: previous
                .map_or_else(Vec::new, |task| task.completed_criteria.clone()),
            worktree_branch: None,
            due_at_epoch_seconds: if entry.clear_due {
                None
            } else if let Some(hours) = entry.due_hours {
                Some(now_epoch_seconds().saturating_add(hours * 3600))
            } else {
                previous.and_then(|task| task.due_at_epoch_seconds)
            },
            updated_at_epoch_seconds: previous.map_or(0, |task| task.updated_at_epoch_seconds),
        };
        if task
            .completed_criteria
            .iter()
            .any(|criterion| !task.acceptance_criteria.contains(criterion))
        {
            return Err(
                "Keep completed criteria or explicitly reopen/review the task first".into(),
            );
        }
        if let Some(previous) = previous {
            if &task != previous {
                if previous.status == TaskStatus::Done {
                    return Err("Reopen a completed task before changing its plan".into());
                }
                if snapshot
                    .shared
                    .handoffs
                    .iter()
                    .any(|handoff| handoff.task_id == task.id)
                {
                    return Err("Resolve the pending handoff before replanning this task".into());
                }
                task.updated_at_epoch_seconds = now_epoch_seconds();
            }
            let task_id = task.id.clone();
            *tasks
                .iter_mut()
                .find(|old| old.id == task_id)
                .ok_or("Task disappeared")? = task;
        } else {
            task.updated_at_epoch_seconds = now_epoch_seconds();
            tasks.push(task);
        }
    }
    if tasks.len() > 100 {
        return Err("Room task limit is 100".into());
    }
    let mut resolved = HashSet::new();
    while resolved.len() < tasks.len() {
        let before = resolved.len();
        for task in &tasks {
            if task.dependencies.iter().all(|id| resolved.contains(id)) {
                resolved.insert(task.id.clone());
            }
        }
        if before == resolved.len() {
            return Err("Task dependencies contain a cycle or missing task".into());
        }
    }
    for task in &tasks {
        if task.status != TaskStatus::Open
            && task.dependencies.iter().any(|id| {
                tasks
                    .iter()
                    .any(|dependency| &dependency.id == id && dependency.status != TaskStatus::Done)
            })
        {
            return Err("A started/completed task cannot gain unfinished dependencies".into());
        }
    }
    Ok(tasks)
}

impl SharedRoom {
    pub(super) fn apply_plan(
        &mut self,
        actor: &str,
        expected: Vec<Task>,
        plan: TaskPlan,
    ) -> Result<(), String> {
        if actor != self.state.host_id {
            return Err("Only the host can apply a shared task plan".into());
        }
        if expected != self.state.tasks {
            return Err("Task plan is stale; refresh and review it again".into());
        }
        let snapshot = super::super::Snapshot {
            shared: self.snapshot(),
            members: self
                .state
                .participants
                .iter()
                .map(|person| super::super::Member {
                    id: person.id.clone(),
                    name: person.name.clone(),
                    online: true,
                })
                .collect(),
            ..Default::default()
        };
        let tasks = preview_plan(&snapshot, &plan)?;
        let changed: HashSet<_> = tasks
            .iter()
            .filter(|task| !self.state.tasks.contains(task))
            .map(|task| task.id.clone())
            .collect();
        if changed.is_empty() {
            return Err("The reviewed plan makes no changes".into());
        }
        let revision = if self.shipping.revision == 0 {
            0
        } else {
            self.shipping
                .revision
                .checked_add(1)
                .ok_or("Ship revision exhausted")?
        };
        let mut profiles = self.readiness.view.profiles.clone();
        for task in tasks.iter().filter(|task| {
            changed.contains(&task.id) && self.state.tasks.iter().any(|old| old.id == task.id)
        }) {
            let next = profiles
                .iter()
                .find(|profile| profile.task == task.id)
                .map_or(0, |profile| profile.revision)
                .checked_add(1)
                .ok_or("Readiness revision exhausted")?;
            profiles.retain(|profile| profile.task != task.id);
            profiles.push(super::super::TaskReadiness {
                task: task.id.clone(),
                revision: next,
                owner: task.owner_id.clone(),
                updated: now_epoch_seconds(),
                profile: Default::default(),
            });
        }
        self.state.tasks = tasks;
        self.readiness.view.profiles = profiles;
        self.readiness
            .view
            .help
            .retain(|help| !changed.contains(&help.task));
        for impact in &mut self.readiness.view.impacts {
            if changed.contains(&impact.source) || changed.contains(&impact.target) {
                impact.acknowledged_by = None;
                impact.accepted = None;
            }
        }
        self.shipping.revision = revision;
        self.shipping.reports.clear();
        self.audit(actor, &format!("applied reviewed task plan: {} changed/new tasks; readiness and release reports require review", changed.len()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{Client, Host, ShipCheck, ShipPlan, ShipResult};
    use std::time::Duration;
    fn entry(key: &str, owner: &str, dependencies: &[&str]) -> PlannedTask {
        PlannedTask {
            key: key.into(),
            existing: None,
            title: key.into(),
            owner: owner.into(),
            criteria: vec!["manual check".into()],
            dependencies: dependencies.iter().map(|id| (*id).into()).collect(),
            priority: TaskPriority::Normal,
            due_hours: None,
            clear_due: false,
        }
    }
    #[test]
    fn atomic_forward_plan_stale_cycles_and_foreign_owner_denials() {
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "planning").unwrap();
        let (mut owner, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Ada").unwrap();
        let plan = TaskPlan {
            version: 1,
            tasks: vec![
                entry("ui", &owner.identity, &["api"]),
                entry("api", &owner.identity, &[]),
            ],
        };
        let action = Action::ApplyTaskPlan {
            expected_tasks: vec![],
            plan: plan.clone(),
        };
        assert!(owner.execute(action.clone()).unwrap().rejection.is_some());
        host.control(action.clone()).unwrap();
        let snapshot = host.snapshot().unwrap();
        assert_eq!(snapshot.shared.tasks.len(), 2);
        assert_eq!(snapshot.shared.tasks[0].dependencies, vec!["T2"]);
        assert!(host.control(action).is_err());
        let mut cycle = TaskPlan {
            version: 1,
            tasks: vec![
                entry("one", &owner.identity, &["two"]),
                entry("two", &owner.identity, &["one"]),
            ],
        };
        assert!(host
            .control(Action::ApplyTaskPlan {
                expected_tasks: snapshot.shared.tasks.clone(),
                plan: cycle.clone()
            })
            .is_err());
        assert_eq!(host.snapshot().unwrap().shared.tasks, snapshot.shared.tasks);
        cycle.tasks[1].dependencies.clear();
        cycle.tasks[1].owner = "f".repeat(64);
        assert!(host
            .control(Action::ApplyTaskPlan {
                expected_tasks: snapshot.shared.tasks.clone(),
                plan: cycle
            })
            .is_err());
        assert_eq!(host.snapshot().unwrap().shared.tasks, snapshot.shared.tasks);
        super::super::validate(&host.snapshot().unwrap().shared).unwrap();
    }
    #[test]
    fn replan_invalidates_readiness_and_ship_reports_without_transferring_or_completing_work() {
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "edit").unwrap();
        let (mut owner, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Ada").unwrap();
        host.control(Action::ApplyTaskPlan {
            expected_tasks: vec![],
            plan: TaskPlan {
                version: 1,
                tasks: vec![entry("work", &owner.identity, &[])],
            },
        })
        .unwrap();
        assert!(owner
            .execute(Action::UpdateReadiness {
                task: "T1".into(),
                expected_revision: 0,
                profile: Default::default()
            })
            .unwrap()
            .rejection
            .is_none());
        host.control(Action::SetShipPlan {
            expected_revision: 0,
            plan: ShipPlan {
                title: "release".into(),
                required_tasks: vec!["T1".into()],
                checks: vec![ShipCheck {
                    title: "manual check".into(),
                    owner: owner.identity.clone(),
                }],
                ..Default::default()
            },
        })
        .unwrap();
        owner
            .execute(Action::ReportShipCheck {
                expected_revision: 1,
                check: 0,
                result: ShipResult::ClaimedPass,
                note: "manual".into(),
            })
            .unwrap();
        let before = host.snapshot().unwrap();
        let mut edit = entry("T1", &owner.identity, &[]);
        edit.existing = Some("T1".into());
        edit.title = "Replanned work".into();
        host.control(Action::ApplyTaskPlan {
            expected_tasks: before.shared.tasks.clone(),
            plan: TaskPlan {
                version: 1,
                tasks: vec![edit.clone()],
            },
        })
        .unwrap();
        let after = host.snapshot().unwrap();
        assert_eq!(
            after.shared.readiness.profiles[0].profile,
            crate::transport::ReadinessProfile::default()
        );
        assert_eq!(after.shared.readiness.profiles[0].revision, 2);
        assert!(owner
            .execute(Action::UpdateReadiness {
                task: "T1".into(),
                expected_revision: 1,
                profile: Default::default()
            })
            .unwrap()
            .rejection
            .is_some());
        assert!(after.shared.shipping.reports.is_empty());
        assert_eq!(
            after.shared.shipping.revision,
            before.shared.shipping.revision + 1
        );
        assert_eq!(after.shared.tasks[0].owner_id, owner.identity);
        assert_eq!(after.shared.tasks[0].status, TaskStatus::Open);
        edit.owner = "f".repeat(64);
        assert!(preview_plan(
            &after,
            &TaskPlan {
                version: 1,
                tasks: vec![edit.clone()]
            }
        )
        .is_err());
        edit.owner = owner.identity.clone();
        owner
            .execute(Action::Transition {
                task: "T1".into(),
                status: TaskStatus::Done,
                completed: vec!["manual check".into()],
            })
            .unwrap();
        edit.title = "cannot rewrite done".into();
        let done = host.snapshot().unwrap();
        assert!(host
            .control(Action::ApplyTaskPlan {
                expected_tasks: done.shared.tasks,
                plan: TaskPlan {
                    version: 1,
                    tasks: vec![edit]
                }
            })
            .is_err());
    }
}
