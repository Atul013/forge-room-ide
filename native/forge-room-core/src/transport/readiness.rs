//! Owner-authored readiness and explicit coordination. No scheduler or execution authority.
use super::{validate_patch, workflow::Action};
use crate::{now_epoch_seconds, RoomState, Task, TaskStatus};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Declaration {
    #[default]
    Unknown,
    ClaimedPass,
    ClaimedFail,
    NotRequired,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessProfile {
    pub estimate_minutes: Option<u32>,
    pub blocker: String,
    pub files: Vec<String>,
    pub checks: Declaration,
    pub preview: Declaration,
    pub submission: Declaration,
    pub help_allowed: bool,
}
impl ReadinessProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.estimate_minutes.is_some_and(|v| v == 0 || v > 10080)
            || self.blocker.len() > 500
            || self.blocker.chars().any(|c| c.is_control())
            || self.files.len() > 8
        {
            return Err("Invalid readiness limits".into());
        }
        let mut seen = std::collections::HashSet::new();
        for file in &self.files {
            validate_patch(file, "a", "b")?;
            if !seen.insert(file) {
                return Err("Duplicate linked path".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskReadiness {
    pub task: String,
    pub revision: u64,
    pub owner: String,
    pub updated: u64,
    pub profile: ReadinessProfile,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HelpRequest {
    pub task: String,
    pub actor: String,
    pub reason: String,
    pub opened: u64,
    pub closed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Impact {
    pub revision: String,
    pub id: u64,
    pub source: String,
    pub target: String,
    pub file: String,
    pub summary: String,
    pub actor: String,
    pub opened: u64,
    pub acknowledged_by: Option<String>,
    pub accepted: Option<bool>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReadinessState {
    pub supported: bool,
    pub profiles: Vec<TaskReadiness>,
    pub help: Vec<HelpRequest>,
    pub impacts: Vec<Impact>,
}
fn revision_reference(value: &str) -> Result<(), String> {
    if ![40, 64].contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Provide a full 40/64-character commit or diff digest; the host records it as an owner declaration, not verification".into());
    }
    Ok(())
}
fn text(value: &str, limit: usize) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > limit || value.chars().any(char::is_control) {
        Err("Invalid coordination text".into())
    } else {
        Ok(())
    }
}
fn owner<'a>(state: &'a RoomState, task: &str, actor: &str) -> Result<&'a Task, String> {
    let task = state
        .tasks
        .iter()
        .find(|t| t.id == task)
        .ok_or("Task not found")?;
    if task.owner_id != actor {
        return Err(
            "Only the current task owner may change readiness or request assistance".into(),
        );
    }
    Ok(task)
}
impl ReadinessState {
    pub fn profile(&self, task: &str) -> Option<&TaskReadiness> {
        self.profiles.iter().find(|p| p.task == task)
    }
    pub fn validate(&self, tasks: &[Task]) -> Result<(), String> {
        let exists = |id: &str| tasks.iter().any(|t| t.id == id);
        if self.profiles.len() > 100 || self.help.len() > 100 || self.impacts.len() > 128 {
            return Err("Readiness snapshot limit exceeded".into());
        }
        let mut seen = std::collections::HashSet::new();
        for p in &self.profiles {
            if !exists(&p.task) || p.owner.len() != 64 || p.revision == 0 || !seen.insert(&p.task) {
                return Err("Invalid readiness revision".into());
            }
            p.profile.validate()?;
        }
        seen.clear();
        for h in &self.help {
            if !exists(&h.task) || h.actor.len() != 64 || !seen.insert(&h.task) {
                return Err("Invalid help request".into());
            }
            text(&h.reason, 500)?;
        }
        let mut ids = std::collections::HashSet::new();
        for i in &self.impacts {
            if i.id == 0
                || !ids.insert(i.id)
                || !exists(&i.source)
                || !exists(&i.target)
                || i.source == i.target
                || i.actor.len() != 64
                || i.acknowledged_by.as_ref().is_some_and(|a| a.len() != 64)
                || i.acknowledged_by.is_some() != i.accepted.is_some()
            {
                return Err("Invalid impact record".into());
            }
            revision_reference(&i.revision)?;
            text(&i.summary, 500)?;
            validate_patch(&i.file, "a", "b")?;
        }
        Ok(())
    }
}
pub(super) struct Authority {
    pub view: ReadinessState,
    next: u64,
}
impl Default for Authority {
    fn default() -> Self {
        Self {
            view: ReadinessState {
                supported: true,
                ..Default::default()
            },
            next: 1,
        }
    }
}
impl Authority {
    pub(super) fn restore(view: ReadinessState) -> Result<Self, String> {
        let next = view
            .impacts
            .iter()
            .map(|impact| impact.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Impact sequence exhausted")?;
        Ok(Self { view, next })
    }

    pub fn apply(
        &mut self,
        actor: &str,
        action: &Action,
        state: &RoomState,
    ) -> Result<Option<String>, String> {
        match action {
            Action::UpdateReadiness {
                task,
                expected_revision,
                profile,
            } => {
                let task_state = owner(state, task, actor)?;
                if task_state.status == TaskStatus::Done {
                    return Err("Reopen the task before changing readiness".into());
                }
                profile.validate()?;
                let revision = self.view.profile(task).map_or(0, |p| p.revision);
                if *expected_revision != revision {
                    return Err("Readiness changed; reload before publishing".into());
                }
                let revision = revision.checked_add(1).ok_or("Revision exhausted")?;
                self.view.profiles.retain(|p| &p.task != task);
                self.view.profiles.push(TaskReadiness {
                    task: task.clone(),
                    revision,
                    owner: actor.into(),
                    updated: now_epoch_seconds(),
                    profile: profile.clone(),
                });
                if !profile.help_allowed {
                    for help in self.view.help.iter_mut().filter(|h| &h.task == task) {
                        help.closed = true;
                    }
                }
                Ok(Some(format!(
                    "published readiness for {task} revision {revision} (manual declarations)"
                )))
            }
            Action::RequestHelp {
                task,
                expected_revision,
                reason,
            } => {
                let task_state = owner(state, task, actor)?;
                if task_state.status == TaskStatus::Done {
                    return Err("Completed tasks cannot request rescue".into());
                }
                text(reason, 500)?;
                let profile = self
                    .view
                    .profile(task)
                    .ok_or("Publish readiness and opt in to assistance first")?;
                if profile.revision != *expected_revision
                    || profile.owner != actor
                    || !profile.profile.help_allowed
                {
                    return Err("Assistance is not enabled under the current owner/revision".into());
                }
                if self
                    .view
                    .help
                    .iter()
                    .any(|h| &h.task == task && h.actor == actor && !h.closed)
                {
                    return Err("A help request is already open".into());
                }
                self.view.help.retain(|h| &h.task != task);
                self.view.help.push(HelpRequest {
                    task: task.clone(),
                    actor: actor.into(),
                    reason: reason.clone(),
                    opened: now_epoch_seconds(),
                    closed: false,
                });
                Ok(Some(format!(
                    "requested assistance on {task}; ownership and execution unchanged"
                )))
            }
            Action::CloseHelp { task } => {
                owner(state, task, actor)?;
                let help = self
                    .view
                    .help
                    .iter_mut()
                    .find(|h| &h.task == task)
                    .ok_or("Help request not found")?;
                help.closed = true;
                Ok(Some(format!("closed assistance request on {task}")))
            }
            Action::DeclareImpact {
                revision,
                source,
                target,
                file,
                summary,
            } => {
                revision_reference(revision)?;
                owner(state, source, actor)?;
                let target_state = state
                    .tasks
                    .iter()
                    .find(|t| &t.id == target)
                    .ok_or("Target task not found")?;
                if source == target {
                    return Err("Impact requires a different target task".into());
                }
                text(summary, 500)?;
                validate_patch(file, "a", "b")?;
                let source_profile = self
                    .view
                    .profile(source)
                    .filter(|p| p.owner == actor)
                    .ok_or("Publish current-owner source file links first")?;
                if !source_profile.profile.files.contains(file) {
                    return Err(
                        "Impact source must name an explicitly published linked path".into(),
                    );
                }
                let overlap = self
                    .view
                    .profile(target)
                    .filter(|p| p.owner == target_state.owner_id)
                    .is_some_and(|p| p.profile.files.contains(file));
                if !target_state.dependencies.contains(source) && !overlap {
                    return Err(
                        "Target must depend on source or explicitly link the same file".into(),
                    );
                }
                if self.view.impacts.len() >= 128 {
                    return Err("Room impact history limit reached".into());
                }
                if self.view.impacts.iter().any(|i| {
                    &i.source == source
                        && &i.target == target
                        && &i.file == file
                        && &i.revision == revision
                        && &i.summary == summary
                        && i.acknowledged_by.is_none()
                }) {
                    return Err("This impact notice is already pending".into());
                }
                let id = self.next;
                self.next = self.next.checked_add(1).ok_or("Impact IDs exhausted")?;
                self.view.impacts.push(Impact {
                    revision: revision.clone(),
                    id,
                    source: source.clone(),
                    target: target.clone(),
                    file: file.clone(),
                    summary: summary.clone(),
                    actor: actor.into(),
                    opened: now_epoch_seconds(),
                    acknowledged_by: None,
                    accepted: None,
                });
                Ok(Some(format!(
                    "declared impact {id}: {source} -> {target}; owner review required"
                )))
            }
            Action::AcknowledgeImpact { id, accepted } => {
                let impact = self
                    .view
                    .impacts
                    .iter_mut()
                    .find(|i| i.id == *id)
                    .ok_or("Impact not found")?;
                owner(state, &impact.target, actor)?;
                if impact.acknowledged_by.as_deref() == Some(actor) {
                    if impact.accepted == Some(*accepted) {
                        return Ok(Some(format!(
                            "impact {id} acknowledgement already recorded"
                        )));
                    }
                    return Err("Acknowledgement already recorded; declare a new impact for new information".into());
                }
                impact.acknowledged_by = Some(actor.into());
                impact.accepted = Some(*accepted);
                Ok(Some(format!(
                    "{} impact {id}",
                    if *accepted {
                        "acknowledged"
                    } else {
                        "disputed"
                    }
                )))
            }
            _ => Ok(None),
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Assessment {
    pub recommendation: Option<String>,
    pub task: String,
    pub critical_minutes: Option<u64>,
    pub budget_seconds: Option<u64>,
    pub reasons: Vec<String>,
    pub gaps: Vec<String>,
    pub help_allowed: bool,
}
pub fn assess(
    tasks: &[Task],
    readiness: &ReadinessState,
    clock: &super::Countdown,
    now: u64,
) -> Vec<Assessment> {
    fn critical(
        id: &str,
        tasks: &[Task],
        r: &ReadinessState,
        visiting: &mut Vec<String>,
        cache: &mut std::collections::HashMap<String, Option<u64>>,
    ) -> Option<u64> {
        if let Some(value) = cache.get(id) {
            return *value;
        }
        let task = tasks.iter().find(|t| t.id == id)?;
        if task.status == TaskStatus::Done {
            return Some(0);
        }
        if visiting.iter().any(|t| t == id) {
            return None;
        }
        let own = u64::from(
            r.profile(id)
                .filter(|p| p.owner == task.owner_id)?
                .profile
                .estimate_minutes?,
        );
        visiting.push(id.into());
        let result = task
            .dependencies
            .iter()
            .map(|dep| critical(dep, tasks, r, visiting, cache))
            .collect::<Option<Vec<_>>>();
        visiting.pop();
        let value = result.map(|items| own + items.into_iter().max().unwrap_or(0));
        cache.insert(id.into(), value);
        value
    }
    let mut cache = std::collections::HashMap::new();
    tasks.iter().filter(|t|t.status!=TaskStatus::Done).map(|task|{
        let profile=readiness.profile(&task.id).filter(|p|p.owner==task.owner_id).map(|p|&p.profile);
        let mut a=Assessment{task:task.id.clone(),critical_minutes:critical(&task.id,tasks,readiness,&mut vec![],&mut cache),budget_seconds:task.due_at_epoch_seconds.map(|due|due.saturating_sub(now)),help_allowed:profile.is_some_and(|p|p.help_allowed),..Default::default()};
        if clock.running {a.budget_seconds=Some(a.budget_seconds.map_or(clock.remaining_seconds,|s|s.min(clock.remaining_seconds)));}
        if task.due_at_epoch_seconds.is_some_and(|due|due<=now){a.reasons.push("Task deadline reached".into())}
        if let Some(seconds)=a.budget_seconds {if seconds<=3600{a.reasons.push("One hour or less remains in the active deadline budget".into())}
        if a.critical_minutes.is_some_and(|m|m.saturating_mul(60)>seconds){a.reasons.push("Estimated dependency critical path exceeds the remaining budget".into())}}
        if a.budget_seconds.is_none(){a.gaps.push("Deadline budget unknown; countdown is not running and task has no due time".into());}
        if a.critical_minutes.is_none(){a.gaps.push("Critical path unknown: missing current-owner estimates or invalid dependency graph".into())}
        if task.dependencies.iter().any(|id|tasks.iter().find(|t|&t.id==id).is_none_or(|t|t.status!=TaskStatus::Done)){a.reasons.push("Unfinished or missing dependencies block completion".into())}
        if let Some(p)=profile {if !p.blocker.is_empty(){a.reasons.push(format!("Owner blocker: {}",p.blocker))}for (name,status) in [("Checks",p.checks),("Preview",p.preview),("Submission",p.submission)]{match status {Declaration::Unknown=>a.gaps.push(format!("{name}: unknown")),Declaration::ClaimedFail=>a.reasons.push(format!("{name}: owner reports failure")),_=>{}}}}else{a.gaps.push("Current owner has not published readiness".into())}
        let pending=readiness.impacts.iter().filter(|i|i.target==task.id&&i.acknowledged_by.as_deref()!=Some(task.owner_id.as_str())).count();if pending>0{a.reasons.push(format!("{pending} dependency/file impact notice(s) need current-owner review"))}
        let disputed=readiness.impacts.iter().filter(|i|i.target==task.id&&i.acknowledged_by.as_deref()==Some(task.owner_id.as_str())&&i.accepted==Some(false)).count();if disputed>0{a.reasons.push(format!("{disputed} disputed impact notice(s) need discussion"));}
        if !a.reasons.is_empty() && a.help_allowed {a.recommendation=Some("Request assistance or arrange an explicit handoff after reviewing these warnings; keep owner control until accepted.".into());}
        a
    }).collect()
}

#[cfg(test)]
mod tests;
