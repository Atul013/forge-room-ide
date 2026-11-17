//! Shared Room policy. Saved local rooms are never imported into this authority.
use crate::{
    now_epoch_seconds, ActivityEvent, Evidence, EvidenceKind, HandoffRequest, Participant, Role,
    RoomState, Task, TaskPriority, TaskStatus,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    UpdateReadiness {
        task: String,
        expected_revision: u64,
        profile: super::ReadinessProfile,
    },
    RequestHelp {
        task: String,
        expected_revision: u64,
        reason: String,
    },
    CloseHelp {
        task: String,
    },
    DeclareImpact {
        revision: String,
        source: String,
        target: String,
        file: String,
        summary: String,
    },
    AcknowledgeImpact {
        id: u64,
        accepted: bool,
    },
    /// Publish one task as the member's current Room focus; this never shares file or editor data.
    ShareFocus {
        task: Option<String>,
    },
    /// Explicitly share a workspace-relative saved-file location without contents or selection.
    ShareLocation {
        path: Option<String>,
        line: Option<u32>,
    },
    CreateMilestone {
        title: String,
        due_hours: Option<u64>,
    },
    MilestoneDone {
        id: String,
        done: bool,
    },
    SharePatch {
        recipient: String,
        path: String,
        before: String,
        after: String,
    },
    RevokePatch {
        id: u64,
    },
    Message {
        text: String,
        recipient: Option<String>,
        task: Option<String>,
        decision: bool,
        parent: Option<u64>,
    },
    /// Public message which explicitly notifies selected joined members.
    MentionMessage {
        text: String,
        mentions: Vec<String>,
        task: Option<String>,
        decision: bool,
        parent: Option<u64>,
    },
    /// Add or remove the caller's reaction on a visible retained message.
    React {
        sequence: u64,
        emoji: String,
        active: bool,
    },
    CreateTask {
        title: String,
        owner: String,
        criteria: Vec<String>,
        dependencies: Vec<String>,
        priority: TaskPriority,
        due_hours: Option<u64>,
    },
    Transition {
        task: String,
        status: TaskStatus,
        completed: Vec<String>,
    },
    Priority {
        task: String,
        priority: TaskPriority,
    },
    Due {
        task: String,
        hours: Option<u64>,
    },
    RequestHandoff {
        task: String,
        target: String,
    },
    ApproveHandoff {
        task: String,
    },
    Evidence {
        task: String,
        kind: EvidenceKind,
        text: String,
    },
    Countdown {
        seconds: Option<u64>,
        running: bool,
    },
}
impl Action {
    pub fn message(text: &str) -> Self {
        Self::Message {
            text: text.into(),
            recipient: None,
            task: None,
            decision: false,
            parent: None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Countdown {
    pub remaining_seconds: u64,
    pub running: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub title: String,
    pub due_at_epoch_seconds: Option<u64>,
    pub done: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskFocus {
    pub actor: String,
    pub task: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeLocation {
    pub actor: String,
    pub path: String,
    pub line: u32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SharedSnapshot {
    #[serde(default)]
    pub readiness: super::ReadinessState,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    pub tasks: Vec<Task>,
    pub handoffs: Vec<HandoffRequest>,
    pub evidence: Vec<Evidence>,
    pub activity: Vec<ActivityEvent>,
    pub countdown: Countdown,
    #[serde(default)]
    pub task_focus: Vec<TaskFocus>,
    /// Explicitly shared workspace-relative saved-file locations; no source text is included.
    #[serde(default)]
    pub code_locations: Vec<CodeLocation>,
}
pub(super) struct SharedRoom {
    readiness: super::readiness::Authority,
    milestones: Vec<Milestone>,
    pub(super) state: RoomState,
    remaining: Duration,
    started: Option<Instant>,
    activity_sequence: u64,
    task_focus: std::collections::HashMap<String, String>,
    code_locations: std::collections::HashMap<String, (String, u32)>,
}
impl SharedRoom {
    pub(super) fn new() -> Result<Self, String> {
        Ok(Self {
            readiness: Default::default(),
            milestones: Vec::new(),
            state: RoomState::new(
                "network",
                Participant {
                    id: super::secret()?,
                    name: "Host authority".into(),
                    role: Role::Human,
                    owner_id: None,
                },
            ),
            remaining: Duration::ZERO,
            started: None,
            activity_sequence: 0,
            task_focus: Default::default(),
            code_locations: Default::default(),
        })
    }
    pub(super) fn sync_members(&mut self, members: Vec<Participant>) {
        self.state
            .participants
            .retain(|p| p.id == self.state.host_id);
        self.state.participants.extend(members);
        let joined: std::collections::HashSet<_> = self
            .state
            .participants
            .iter()
            .map(|person| person.id.as_str())
            .collect();
        self.task_focus
            .retain(|actor, _| joined.contains(actor.as_str()));
        self.code_locations
            .retain(|actor, _| joined.contains(actor.as_str()));
    }
    pub(super) fn remaining(&self) -> Duration {
        self.started
            .map(|start| self.remaining.saturating_sub(start.elapsed()))
            .unwrap_or(self.remaining)
    }
    pub(super) fn snapshot(&self) -> SharedSnapshot {
        SharedSnapshot {
            readiness: self.readiness.view.clone(),
            milestones: self.milestones.clone(),
            tasks: self.state.tasks.clone(),
            handoffs: self.state.pending_handoffs.clone(),
            evidence: self.state.evidence.clone(),
            activity: self
                .state
                .activity
                .iter()
                .rev()
                .take(128)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect(),
            countdown: Countdown {
                remaining_seconds: self.remaining().as_secs(),
                running: self.started.is_some() && !self.remaining().is_zero(),
            },
            task_focus: {
                let mut focus: Vec<_> = self
                    .task_focus
                    .iter()
                    .map(|(actor, task)| TaskFocus {
                        actor: actor.clone(),
                        task: task.clone(),
                    })
                    .collect();
                focus.sort_by(|left, right| left.actor.cmp(&right.actor));
                focus
            },
            code_locations: {
                let mut locations: Vec<_> = self
                    .code_locations
                    .iter()
                    .map(|(actor, (path, line))| CodeLocation {
                        actor: actor.clone(),
                        path: path.clone(),
                        line: *line,
                    })
                    .collect();
                locations.sort_by(|left, right| left.actor.cmp(&right.actor));
                locations
            },
        }
    }
    pub(super) fn audit(&mut self, actor: &str, detail: &str) {
        self.activity_sequence += 1;
        self.state.activity.push(ActivityEvent {
            id: format!("network-{}", self.activity_sequence),
            actor_id: actor.into(),
            detail: detail.into(),
            recorded_at_epoch_seconds: now_epoch_seconds(),
        });
        if self.state.activity.len() > 128 {
            self.state.activity.remove(0);
        }
    }
    pub(super) fn apply(&mut self, actor: &str, action: Action) -> Result<(), String> {
        if let Some(detail) = self.readiness.apply(actor, &action, &self.state)? {
            self.audit(actor, &detail);
            return Ok(());
        }
        let before = self.state.activity.len();
        match action {
            Action::UpdateReadiness { .. }
            | Action::RequestHelp { .. }
            | Action::CloseHelp { .. }
            | Action::DeclareImpact { .. }
            | Action::AcknowledgeImpact { .. } => {
                unreachable!("readiness authority handled this action")
            }
            Action::ShareFocus { task } => {
                if let Some(task_id) = task {
                    if task_id.len() > 16
                        || !self
                            .state
                            .tasks
                            .iter()
                            .any(|item| item.id == task_id && item.status != TaskStatus::Done)
                    {
                        return Err("Focus must reference an unfinished shared task".into());
                    }
                    self.task_focus.insert(actor.to_owned(), task_id.clone());
                    self.audit(actor, &format!("shared task focus {task_id}"));
                } else if self.task_focus.remove(actor).is_some() {
                    self.audit(actor, "stopped sharing task focus");
                }
                return Ok(());
            }
            Action::ShareLocation { path, line } => {
                match (path, line) {
                    (Some(path), Some(line)) => {
                        let path = validate_code_location(&path, line)?;
                        self.code_locations.insert(actor.to_owned(), (path, line));
                        self.audit(actor, "shared a saved-file code location");
                    }
                    (None, None) => {
                        if self.code_locations.remove(actor).is_some() {
                            self.audit(actor, "stopped sharing code location");
                        }
                    }
                    _ => return Err("A code location needs both a path and line".into()),
                }
                return Ok(());
            }
            Action::CreateMilestone { title, due_hours } => {
                if actor != self.state.host_id {
                    return Err("Only the host can create milestones".into());
                }
                if self.milestones.len() >= 20 {
                    return Err("Room milestone limit reached".into());
                }
                let title = super::bounded(&title, 200)?;
                let deadline = due(due_hours)?;
                let id = format!("M{}", self.milestones.len() + 1);
                self.milestones.push(Milestone {
                    id: id.clone(),
                    title,
                    due_at_epoch_seconds: deadline,
                    done: false,
                });
                self.audit(actor, &format!("created milestone {id}"));
                return Ok(());
            }
            Action::MilestoneDone { id, done } => {
                if actor != self.state.host_id {
                    return Err("Only the host can update milestones".into());
                }
                self.milestones
                    .iter_mut()
                    .find(|m| m.id == id)
                    .ok_or("Milestone not found")?
                    .done = done;
                self.audit(
                    actor,
                    &format!(
                        "marked milestone {id} {}",
                        if done { "complete" } else { "open" }
                    ),
                );
                return Ok(());
            }
            Action::Message { .. } | Action::MentionMessage { .. } | Action::React { .. } => {
                return Err("Use the message channel".into());
            }
            Action::SharePatch { .. } | Action::RevokePatch { .. } => {
                return Err("Use the patch channel".into())
            }
            Action::CreateTask {
                title,
                owner,
                criteria,
                dependencies,
                priority,
                due_hours,
            } => {
                if self.state.tasks.len() >= 100 {
                    return Err("Room task limit reached".into());
                }
                let title = super::bounded(&title, 200)?;
                if criteria.len() > 8 || dependencies.len() > 16 {
                    return Err("Too many criteria or dependencies".into());
                }
                let criteria = criteria
                    .iter()
                    .map(|c| super::bounded(c, 200))
                    .collect::<Result<Vec<_>, _>>()?;
                if criteria
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != criteria.len()
                {
                    return Err("Acceptance criteria must be unique".into());
                }
                if dependencies
                    .iter()
                    .any(|id| !self.state.tasks.iter().any(|t| &t.id == id))
                {
                    return Err("Dependency must be a shared Room task".into());
                }
                let owner_name = self
                    .state
                    .participants
                    .iter()
                    .find(|p| p.id == owner)
                    .ok_or("Owner is no longer joined")?
                    .name
                    .clone();
                let task = Task {
                    id: format!("T{}", self.state.tasks.len() + 1),
                    title,
                    owner_id: owner,
                    owner_name,
                    status: TaskStatus::Open,
                    priority,
                    dependencies,
                    acceptance_criteria: criteria,
                    completed_criteria: Vec::new(),
                    worktree_branch: None,
                    due_at_epoch_seconds: due(due_hours)?,
                    updated_at_epoch_seconds: now_epoch_seconds(),
                };
                self.state
                    .create_task(actor, task)
                    .map_err(|e| e.to_string())?;
            }
            Action::Transition {
                task,
                status,
                completed,
            } => {
                if completed.len() > 8 || completed.iter().any(|c| c.chars().count() > 200) {
                    return Err("Invalid completion checklist".into());
                }
                self.state
                    .transition_task(actor, &task, status, completed)
                    .map_err(|e| e.to_string())?;
            }
            Action::Priority { task, priority } => self
                .state
                .set_task_priority(actor, &task, priority)
                .map_err(|e| e.to_string())?,
            Action::Due { task, hours } => {
                let deadline = due(hours)?;
                self.state
                    .set_task_due(actor, &task, deadline)
                    .map_err(|e| e.to_string())?;
            }
            Action::RequestHandoff { task, target } => self
                .state
                .request_handoff(actor, &task, &target)
                .map_err(|e| e.to_string())?,
            Action::ApproveHandoff { task } => self
                .state
                .approve_handoff(actor, &task)
                .map_err(|e| e.to_string())?,
            Action::Evidence { task, kind, text } => {
                if self.state.evidence.len() >= 128 {
                    return Err("Room evidence limit reached".into());
                }
                let text = super::bounded(&text, 500)?;
                self.state
                    .record_evidence(actor, &task, kind, text)
                    .map_err(|e| e.to_string())?;
            }
            Action::Countdown { seconds, running } => {
                if actor != self.state.host_id {
                    return Err("Only the host can control the countdown".into());
                }
                let remaining = match seconds {
                    Some(seconds) if seconds <= 7 * 86400 => Duration::from_secs(seconds),
                    Some(_) => return Err("Countdown limit is seven days".into()),
                    None => self.remaining(),
                };
                self.remaining = remaining;
                self.started = running.then(Instant::now);
                self.audit(
                    actor,
                    if running {
                        "started or resumed the countdown"
                    } else {
                        "paused or reset the countdown"
                    },
                );
                return Ok(());
            }
        }
        // Replace domain-local activity IDs with a stable sequence for this shared session.
        for event in &mut self.state.activity[before..] {
            self.activity_sequence += 1;
            event.id = format!("network-{}", self.activity_sequence);
        }
        self.task_focus.retain(|_, task_id| {
            self.state
                .tasks
                .iter()
                .any(|task| task.id == *task_id && task.status != TaskStatus::Done)
        });
        if self.state.activity.len() > 128 {
            self.state.activity.drain(..self.state.activity.len() - 128);
        }
        Ok(())
    }
}
fn due(hours: Option<u64>) -> Result<Option<u64>, String> {
    if hours.is_some_and(|h| h > 30 * 24) {
        return Err("Due time limit is thirty days".into());
    }
    Ok(hours.map(|h| now_epoch_seconds().saturating_add(h * 3600)))
}
pub(super) fn validate(snapshot: &SharedSnapshot) -> Result<(), String> {
    snapshot.readiness.validate(&snapshot.tasks)?;
    if snapshot.milestones.len() > 20
        || snapshot
            .milestones
            .iter()
            .any(|m| m.id.len() > 16 || super::bounded(&m.title, 200).is_err())
    {
        return Err("Invalid milestone state".into());
    }
    if snapshot.tasks.len() > 100
        || snapshot.handoffs.len() > 100
        || snapshot.evidence.len() > 128
        || snapshot.activity.len() > 128
        || snapshot.countdown.remaining_seconds > 7 * 86400
        || snapshot.task_focus.len() > 32
        || snapshot.task_focus.iter().any(|focus| {
            focus.actor.len() != 64
                || focus.task.len() > 16
                || !snapshot.tasks.iter().any(|task| task.id == focus.task)
                || !snapshot
                    .tasks
                    .iter()
                    .any(|task| task.id == focus.task && task.status != TaskStatus::Done)
        })
        || snapshot.code_locations.len() > 32
        || snapshot.code_locations.iter().any(|location| {
            location.actor.len() != 64
                || validate_code_location(&location.path, location.line).is_err()
        })
        || snapshot.tasks.iter().any(|t| {
            t.id.len() > 16
                || t.owner_id.len() != 64
                || super::bounded(&t.title, 200).is_err()
                || t.owner_name.chars().count() > 64
                || t.dependencies.len() > 16
                || t.dependencies.iter().any(|d| d.len() > 16)
                || t.acceptance_criteria.len() > 8
                || t.acceptance_criteria
                    .iter()
                    .any(|c| super::bounded(c, 200).is_err())
                || t.completed_criteria.len() > 8
                || t.completed_criteria.iter().any(|c| c.chars().count() > 200)
                || t.worktree_branch.is_some()
        })
        || snapshot.evidence.iter().any(|e| {
            super::bounded(&e.summary, 500).is_err()
                || e.task_id.len() > 16
                || e.actor_id.len() != 64
        })
        || snapshot
            .activity
            .iter()
            .any(|e| e.detail.chars().count() > 500 || e.actor_id.len() != 64)
        || snapshot.handoffs.iter().any(|h| {
            h.task_id.len() > 16 || h.from_owner_id.len() != 64 || h.target_owner_id.len() != 64
        })
    {
        return Err("Invalid shared Room state".into());
    }
    Ok(())
}

fn validate_code_location(path: &str, line: u32) -> Result<String, String> {
    if path.is_empty()
        || path.len() > 512
        || path.contains(['\\', ':'])
        || path.chars().any(char::is_control)
        || path.starts_with('/')
        || path.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.starts_with('.')
                || [".git", ".ssh"].contains(&part.to_ascii_lowercase().as_str())
        })
        || line == 0
        || line > 10_000_000
    {
        return Err(
            "Code location must be a safe workspace-relative path and bounded positive line".into(),
        );
    }
    Ok(path.to_owned())
}
