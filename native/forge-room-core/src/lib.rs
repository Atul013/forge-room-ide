mod persistence;

pub use persistence::{LoadOutcome, StateStore, StoreError};

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

pub const NATIVE_PREVIEW_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Human,
    Agent,
}

impl FromStr for Role {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "human" => Ok(Self::Human),
            "agent" => Ok(Self::Agent),
            _ => Err(DomainError::InvalidRole),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Open,
    InProgress,
    Done,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(formatter, "open"),
            Self::InProgress => write!(formatter, "in-progress"),
            Self::Done => write!(formatter, "done"),
        }
    }
}

impl FromStr for TaskStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "open" => Ok(Self::Open),
            "in-progress" | "inProgress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            _ => Err(DomainError::InvalidStatus),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub owner_id: String,
    pub owner_name: String,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance_criteria: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub completed_criteria: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_branch: Option<String>,
    pub updated_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceKind {
    Check,
    Review,
    Handoff,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub task_id: String,
    pub actor_id: String,
    pub kind: EvidenceKind,
    pub summary: String,
    pub recorded_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffRequest {
    pub task_id: String,
    pub from_owner_id: String,
    pub target_owner_id: String,
    pub requested_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharingPolicy {
    pub share_file_paths: bool,
    pub share_saved_diffs: bool,
    pub share_unsaved_buffers: bool,
    pub share_terminal_output: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: String,
    pub actor_id: String,
    pub detail: String,
    pub recorded_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomState {
    pub schema_version: u32,
    pub room_id: String,
    pub host_id: String,
    pub participants: Vec<Participant>,
    pub tasks: Vec<Task>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_handoffs: Vec<HandoffRequest>,
    pub evidence: Vec<Evidence>,
    pub activity: Vec<ActivityEvent>,
    pub sharing: SharingPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainError {
    ActorNotConnected,
    AgentOwnerMissing,
    AcceptanceCriteriaTooLarge,
    CompletedTaskHandoff,
    DependencyBlocked(Vec<String>),
    DuplicateParticipant,
    DuplicateTask,
    EvidenceTooLong,
    HandoffRequestNotFound,
    HandoffStale,
    InvalidCompletedCriteria,
    InvalidRole,
    InvalidStatus,
    InvalidTitle,
    NewTaskMustStartOpen,
    OnlyHostCanAddParticipants,
    OnlyHostCanApproveHandoff,
    OnlyHostCanReopen,
    OnlyOwnerOrHostCanUpdate,
    OnlyOwnerCanRequestHandoff,
    ReopenBeforeEditingCompletion,
    ReopenWouldBlock(Vec<String>),
    SelfHandoff,
    TaskNotFound,
    TargetNotConnected,
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActorNotConnected => {
                write!(formatter, "actor is not a connected room participant")
            }
            Self::AgentOwnerMissing => {
                write!(formatter, "an agent must name a connected human owner")
            }
            Self::AcceptanceCriteriaTooLarge => write!(
                formatter,
                "a task supports at most eight acceptance criteria"
            ),
            Self::CompletedTaskHandoff => {
                write!(formatter, "completed tasks must be reopened before handoff")
            }
            Self::DependencyBlocked(ids) => write!(
                formatter,
                "unfinished dependencies block this transition: {}",
                ids.join(", ")
            ),
            Self::DuplicateParticipant => write!(formatter, "participant ID already exists"),
            Self::DuplicateTask => write!(formatter, "task ID already exists"),
            Self::EvidenceTooLong => write!(
                formatter,
                "evidence summary must contain 1 to 500 characters"
            ),
            Self::HandoffRequestNotFound => {
                write!(formatter, "no pending handoff exists for this task")
            }
            Self::HandoffStale => write!(
                formatter,
                "task ownership changed after this handoff was requested"
            ),
            Self::InvalidCompletedCriteria => write!(
                formatter,
                "completion confirmations must uniquely match every acceptance criterion"
            ),
            Self::InvalidRole => write!(formatter, "role must be human or agent"),
            Self::InvalidStatus => write!(formatter, "status must be open, in-progress, or done"),
            Self::InvalidTitle => write!(formatter, "task title must contain 1 to 200 characters"),
            Self::NewTaskMustStartOpen => write!(
                formatter,
                "new tasks must start open without a worktree branch"
            ),
            Self::OnlyHostCanAddParticipants => {
                write!(formatter, "only the room host can add a participant")
            }
            Self::OnlyHostCanApproveHandoff => {
                write!(formatter, "only the room host can approve a handoff")
            }
            Self::OnlyHostCanReopen => {
                write!(formatter, "only the room host can reopen a completed task")
            }
            Self::OnlyOwnerOrHostCanUpdate => write!(
                formatter,
                "only the current owner or host can update this task"
            ),
            Self::OnlyOwnerCanRequestHandoff => {
                write!(
                    formatter,
                    "only the assigned task owner can request a handoff"
                )
            }
            Self::ReopenBeforeEditingCompletion => write!(
                formatter,
                "reopen a completed task before changing completion details"
            ),
            Self::ReopenWouldBlock(ids) => write!(
                formatter,
                "active dependent tasks prevent reopening: {}",
                ids.join(", ")
            ),
            Self::SelfHandoff => write!(
                formatter,
                "handoff target must differ from the current owner"
            ),
            Self::TaskNotFound => write!(formatter, "task does not exist"),
            Self::TargetNotConnected => {
                write!(formatter, "task owner must be a connected participant")
            }
        }
    }
}

impl std::error::Error for DomainError {}

impl RoomState {
    pub fn new(room_id: impl Into<String>, host: Participant) -> Self {
        let host_id = host.id.clone();
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            room_id: room_id.into(),
            host_id,
            participants: vec![host],
            tasks: Vec::new(),
            pending_handoffs: Vec::new(),
            evidence: Vec::new(),
            activity: Vec::new(),
            sharing: SharingPolicy::default(),
        }
    }

    pub fn add_participant(
        &mut self,
        actor_id: &str,
        participant: Participant,
    ) -> Result<(), DomainError> {
        if actor_id != self.host_id {
            return Err(DomainError::OnlyHostCanAddParticipants);
        }
        if self
            .participants
            .iter()
            .any(|item| item.id == participant.id)
        {
            return Err(DomainError::DuplicateParticipant);
        }
        if participant.role == Role::Agent {
            let valid_owner = participant
                .owner_id
                .as_ref()
                .and_then(|owner_id| self.participants.iter().find(|item| &item.id == owner_id))
                .map(|owner| owner.role == Role::Human)
                .unwrap_or(false);
            if !valid_owner {
                return Err(DomainError::AgentOwnerMissing);
            }
        }
        let detail = format!("{} joined as {:?}", participant.name, participant.role);
        self.participants.push(participant);
        self.record_activity(actor_id, detail);
        Ok(())
    }

    pub fn create_task(&mut self, actor_id: &str, task: Task) -> Result<(), DomainError> {
        self.require_connected(actor_id)?;
        validate_task_shape(&task)?;
        if task.status != TaskStatus::Open || task.worktree_branch.is_some() {
            return Err(DomainError::NewTaskMustStartOpen);
        }
        if self.tasks.iter().any(|item| item.id == task.id) {
            return Err(DomainError::DuplicateTask);
        }
        if !self.is_connected(&task.owner_id) {
            return Err(DomainError::TargetNotConnected);
        }
        if task
            .dependencies
            .iter()
            .any(|dependency| dependency == &task.id)
        {
            return Err(DomainError::DependencyBlocked(vec![task.id.clone()]));
        }
        self.tasks.push(task.clone());
        self.record_activity(actor_id, format!("created task {}", task.id));
        Ok(())
    }

    pub fn transition_task(
        &mut self,
        actor_id: &str,
        task_id: &str,
        status: TaskStatus,
        completed_criteria: Vec<String>,
    ) -> Result<(), DomainError> {
        self.require_connected(actor_id)?;
        let index = self.task_index(task_id)?;
        let current = self.tasks[index].clone();
        self.require_task_writer(actor_id, &current)?;
        if current.status == TaskStatus::Done
            && status != TaskStatus::Done
            && actor_id != self.host_id
        {
            return Err(DomainError::OnlyHostCanReopen);
        }
        if status != TaskStatus::Open {
            let blocked: Vec<String> = current
                .dependencies
                .iter()
                .filter(|dependency| {
                    self.tasks
                        .iter()
                        .find(|item| &item.id == *dependency)
                        .map(|item| item.status)
                        != Some(TaskStatus::Done)
                })
                .cloned()
                .collect();
            if !blocked.is_empty() {
                return Err(DomainError::DependencyBlocked(blocked));
            }
        }
        if current.status != TaskStatus::Open && status == TaskStatus::Open {
            let active: Vec<String> = self
                .tasks
                .iter()
                .filter(|candidate| {
                    candidate.id != current.id
                        && candidate.status != TaskStatus::Open
                        && self.depends_on(&candidate.id, &current.id, &mut HashSet::new())
                })
                .map(|candidate| candidate.id.clone())
                .collect();
            if !active.is_empty() {
                return Err(DomainError::ReopenWouldBlock(active));
            }
        }
        validate_completion(&current, status, &completed_criteria)?;
        let task = &mut self.tasks[index];
        task.status = status;
        task.completed_criteria = completed_criteria;
        task.updated_at_epoch_seconds = now_epoch_seconds();
        self.record_activity(actor_id, format!("set task {task_id} to {status}"));
        Ok(())
    }

    pub fn request_handoff(
        &mut self,
        actor_id: &str,
        task_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError> {
        self.require_connected(actor_id)?;
        let index = self.task_index(task_id)?;
        let current = self.tasks[index].clone();
        if actor_id != current.owner_id {
            return Err(DomainError::OnlyOwnerCanRequestHandoff);
        }
        if current.status == TaskStatus::Done {
            return Err(DomainError::CompletedTaskHandoff);
        }
        if target_id == current.owner_id {
            return Err(DomainError::SelfHandoff);
        }
        if !self.is_connected(target_id) {
            return Err(DomainError::TargetNotConnected);
        }
        self.pending_handoffs
            .retain(|request| request.task_id != task_id);
        self.pending_handoffs.push(HandoffRequest {
            task_id: task_id.to_string(),
            from_owner_id: current.owner_id,
            target_owner_id: target_id.to_string(),
            requested_at_epoch_seconds: now_epoch_seconds(),
        });
        self.record_activity(
            actor_id,
            format!("requested handoff of task {task_id} to {target_id}"),
        );
        Ok(())
    }

    pub fn approve_handoff(&mut self, actor_id: &str, task_id: &str) -> Result<(), DomainError> {
        if actor_id != self.host_id {
            return Err(DomainError::OnlyHostCanApproveHandoff);
        }
        let request_index = self
            .pending_handoffs
            .iter()
            .position(|request| request.task_id == task_id)
            .ok_or(DomainError::HandoffRequestNotFound)?;
        let request = self.pending_handoffs[request_index].clone();
        let task_index = self.task_index(task_id)?;
        if self.tasks[task_index].owner_id != request.from_owner_id {
            return Err(DomainError::HandoffStale);
        }
        if self.tasks[task_index].status == TaskStatus::Done {
            return Err(DomainError::CompletedTaskHandoff);
        }
        let target_name = self
            .participants
            .iter()
            .find(|participant| participant.id == request.target_owner_id)
            .map(|participant| participant.name.clone())
            .ok_or(DomainError::TargetNotConnected)?;
        let task = &mut self.tasks[task_index];
        task.owner_id = request.target_owner_id.clone();
        task.owner_name = target_name;
        task.updated_at_epoch_seconds = now_epoch_seconds();
        self.pending_handoffs.remove(request_index);
        self.record_activity(
            actor_id,
            format!(
                "approved handoff of task {task_id} to {}",
                request.target_owner_id
            ),
        );
        Ok(())
    }

    pub fn record_evidence(
        &mut self,
        actor_id: &str,
        task_id: &str,
        kind: EvidenceKind,
        summary: impl Into<String>,
    ) -> Result<(), DomainError> {
        self.require_connected(actor_id)?;
        let task = self
            .tasks
            .get(self.task_index(task_id)?)
            .cloned()
            .ok_or(DomainError::TaskNotFound)?;
        self.require_task_writer(actor_id, &task)?;
        let summary = summary.into();
        if summary.trim().is_empty() || summary.chars().count() > 500 {
            return Err(DomainError::EvidenceTooLong);
        }
        let at = now_epoch_seconds();
        self.evidence.push(Evidence {
            id: format!("evidence-{at}-{}", self.evidence.len() + 1),
            task_id: task_id.to_string(),
            actor_id: actor_id.to_string(),
            kind,
            summary,
            recorded_at_epoch_seconds: at,
        });
        self.record_activity(actor_id, format!("recorded evidence for task {task_id}"));
        Ok(())
    }

    fn require_task_writer(&self, actor_id: &str, task: &Task) -> Result<(), DomainError> {
        if actor_id == self.host_id || actor_id == task.owner_id {
            Ok(())
        } else {
            Err(DomainError::OnlyOwnerOrHostCanUpdate)
        }
    }

    fn require_connected(&self, actor_id: &str) -> Result<(), DomainError> {
        if self.is_connected(actor_id) {
            Ok(())
        } else {
            Err(DomainError::ActorNotConnected)
        }
    }

    fn is_connected(&self, participant_id: &str) -> bool {
        self.participants
            .iter()
            .any(|participant| participant.id == participant_id)
    }

    fn task_index(&self, task_id: &str) -> Result<usize, DomainError> {
        self.tasks
            .iter()
            .position(|task| task.id == task_id)
            .ok_or(DomainError::TaskNotFound)
    }

    fn depends_on(&self, task_id: &str, target_id: &str, seen: &mut HashSet<String>) -> bool {
        if task_id == target_id {
            return true;
        }
        if !seen.insert(task_id.to_string()) {
            return false;
        }
        self.tasks
            .iter()
            .find(|task| task.id == task_id)
            .map(|task| {
                task.dependencies
                    .iter()
                    .any(|dependency| self.depends_on(dependency, target_id, seen))
            })
            .unwrap_or(false)
    }

    fn record_activity(&mut self, actor_id: &str, detail: String) {
        let at = now_epoch_seconds();
        self.activity.push(ActivityEvent {
            id: format!("event-{at}-{}", self.activity.len() + 1),
            actor_id: actor_id.to_string(),
            detail,
            recorded_at_epoch_seconds: at,
        });
        if self.activity.len() > 500 {
            let excess = self.activity.len() - 500;
            self.activity.drain(0..excess);
        }
    }
}

fn validate_task_shape(task: &Task) -> Result<(), DomainError> {
    let title_length = task.title.trim().chars().count();
    if title_length == 0 || title_length > 200 {
        return Err(DomainError::InvalidTitle);
    }
    if task.acceptance_criteria.len() > 8 {
        return Err(DomainError::AcceptanceCriteriaTooLarge);
    }
    Ok(())
}

fn validate_completion(
    current: &Task,
    status: TaskStatus,
    completed: &[String],
) -> Result<(), DomainError> {
    if current.status == TaskStatus::Done
        && status != TaskStatus::Open
        && current.completed_criteria != completed
    {
        return Err(DomainError::ReopenBeforeEditingCompletion);
    }
    if status != TaskStatus::Done && !completed.is_empty() {
        return Err(DomainError::InvalidCompletedCriteria);
    }
    let unique: HashSet<&String> = completed.iter().collect();
    if unique.len() != completed.len()
        || completed
            .iter()
            .any(|criterion| !current.acceptance_criteria.contains(criterion))
    {
        return Err(DomainError::InvalidCompletedCriteria);
    }
    if status == TaskStatus::Done {
        let required: HashSet<&String> = current.acceptance_criteria.iter().collect();
        if unique != required {
            return Err(DomainError::InvalidCompletedCriteria);
        }
    }
    Ok(())
}

pub fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participant(id: &str, name: &str) -> Participant {
        Participant {
            id: id.into(),
            name: name.into(),
            role: Role::Human,
            owner_id: None,
        }
    }

    fn task(id: &str, owner: &str, dependencies: Vec<String>) -> Task {
        Task {
            id: id.into(),
            title: format!("Task {id}"),
            owner_id: owner.into(),
            owner_name: owner.into(),
            status: TaskStatus::Open,
            dependencies,
            acceptance_criteria: vec!["tests pass".into()],
            completed_criteria: Vec::new(),
            worktree_branch: None,
            updated_at_epoch_seconds: now_epoch_seconds(),
        }
    }

    fn room() -> RoomState {
        let mut room = RoomState::new("room", participant("host", "Host"));
        room.add_participant("host", participant("alice", "Alice"))
            .unwrap();
        room.add_participant("host", participant("bob", "Bob"))
            .unwrap();
        room
    }

    #[test]
    fn only_owner_or_host_can_update_and_only_host_can_reopen() {
        let mut room = room();
        room.create_task("bob", task("api", "alice", vec![]))
            .unwrap();
        assert_eq!(
            room.transition_task("bob", "api", TaskStatus::InProgress, vec![]),
            Err(DomainError::OnlyOwnerOrHostCanUpdate)
        );
        room.transition_task("alice", "api", TaskStatus::InProgress, vec![])
            .unwrap();
        room.transition_task("alice", "api", TaskStatus::Done, vec!["tests pass".into()])
            .unwrap();
        assert_eq!(
            room.transition_task("alice", "api", TaskStatus::Open, vec![]),
            Err(DomainError::OnlyHostCanReopen)
        );
        room.transition_task("host", "api", TaskStatus::Open, vec![])
            .unwrap();
    }

    #[test]
    fn dependencies_and_completion_confirmations_are_enforced() {
        let mut room = room();
        room.create_task("host", task("api", "alice", vec![]))
            .unwrap();
        room.create_task("host", task("ui", "bob", vec!["api".into()]))
            .unwrap();
        assert_eq!(
            room.transition_task("bob", "ui", TaskStatus::InProgress, vec![]),
            Err(DomainError::DependencyBlocked(vec!["api".into()]))
        );
        assert_eq!(
            room.transition_task("alice", "api", TaskStatus::Done, vec![]),
            Err(DomainError::InvalidCompletedCriteria)
        );
        room.transition_task("alice", "api", TaskStatus::Done, vec!["tests pass".into()])
            .unwrap();
        room.transition_task("bob", "ui", TaskStatus::InProgress, vec![])
            .unwrap();
        assert_eq!(
            room.transition_task("host", "api", TaskStatus::Open, vec![]),
            Err(DomainError::ReopenWouldBlock(vec!["ui".into()]))
        );
    }

    #[test]
    fn handoff_requires_connected_target_and_preserves_attribution() {
        let mut room = room();
        room.create_task("host", task("api", "alice", vec![]))
            .unwrap();
        assert_eq!(
            room.request_handoff("alice", "api", "offline"),
            Err(DomainError::TargetNotConnected)
        );
        room.request_handoff("alice", "api", "bob").unwrap();
        assert_eq!(room.tasks[0].owner_id, "alice");
        assert_eq!(
            room.approve_handoff("bob", "api"),
            Err(DomainError::OnlyHostCanApproveHandoff)
        );
        room.approve_handoff("host", "api").unwrap();
        assert_eq!(room.tasks[0].owner_id, "bob");
        assert_eq!(room.tasks[0].owner_name, "Bob");
        assert!(room
            .activity
            .iter()
            .any(|event| event.detail.contains("approved handoff")));
    }

    #[test]
    fn agents_require_a_connected_human_owner() {
        let mut room = room();
        let unowned = Participant {
            id: "agent".into(),
            name: "Agent".into(),
            role: Role::Agent,
            owner_id: None,
        };
        assert_eq!(
            room.add_participant("host", unowned),
            Err(DomainError::AgentOwnerMissing)
        );
        let owned = Participant {
            id: "agent".into(),
            name: "Agent".into(),
            role: Role::Agent,
            owner_id: Some("alice".into()),
        };
        room.add_participant("host", owned).unwrap();
    }

    #[test]
    fn sharing_starts_private() {
        let room = room();
        assert_eq!(room.sharing, SharingPolicy::default());
        assert!(!room.sharing.share_file_paths);
        assert!(!room.sharing.share_saved_diffs);
        assert!(!room.sharing.share_unsaved_buffers);
        assert!(!room.sharing.share_terminal_output);
    }
}
