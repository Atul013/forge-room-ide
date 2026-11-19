use super::{identifier, text, Expanded, StepKind, When};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Draft,
    Queued,
    Running,
    Paused,
    Stopping,
    Succeeded,
    Failed,
    Canceled,
    Interrupted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    WaitingApproval,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Canceled,
    Interrupted,
}
impl StepStatus {
    pub fn terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Skipped | Self::Canceled | Self::Interrupted
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    NativeModel,
    NativeProcess,
    ManualReport,
    Approval,
    RunCheckpoint,
    Interrupted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub passed: bool,
    pub provenance: Provenance,
    pub note: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub canceled: bool,
    pub capture_complete: bool,
    pub truncated: bool,
    pub elapsed_ms: u128,
    pub output_bytes: u64,
    pub artifact_bytes: u64,
}
impl Outcome {
    pub fn declared(passed: bool, provenance: Provenance, note: String) -> Result<Self, String> {
        text(&note, 2000, true)?;
        Ok(Self {
            passed,
            provenance,
            note,
            exit_code: None,
            timed_out: false,
            canceled: false,
            capture_complete: true,
            truncated: false,
            elapsed_ms: 0,
            output_bytes: 0,
            artifact_bytes: 0,
        })
    }
    fn validate(&self) -> Result<(), String> {
        text(&self.note, 2000, false)?;
        if self.passed
            && (self.timed_out
                || self.canceled
                || !self.capture_complete
                || (self.provenance == Provenance::NativeProcess && self.exit_code != Some(0)))
        {
            return Err(
                "A successful native result needs exit zero and complete owned capture".into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub number: u32,
    pub token: String,
    pub started: u64,
    pub finished: Option<u64>,
    pub outcome: Option<Outcome>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepState {
    pub id: String,
    pub status: StepStatus,
    pub reason: String,
    pub attempts: Vec<Attempt>,
    pub approved_attempt: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEvent {
    pub sequence: u64,
    pub at: u64,
    pub step: Option<String>,
    pub kind: String,
    pub detail: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub version: u32,
    pub id: String,
    pub workspace: String,
    pub plan: Expanded,
    pub status: RunStatus,
    pub created: u64,
    pub started: Option<u64>,
    pub finished: Option<u64>,
    pub steps: Vec<StepState>,
    pub events: Vec<RunEvent>,
    pub approved_seal: Option<String>,
    pub attempts_used: u32,
    pub output_used: u64,
    pub artifact_used: u64,
    pub model_calls_used: u32,
    pub model_tokens_reserved: u64,
    pub stop_reason: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub step: String,
    pub requires_approval: bool,
    pub manual: bool,
    pub resources: Vec<String>,
}
pub fn new_id() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
impl Run {
    pub fn new(plan: Expanded, workspace: String, now: u64) -> Result<Self, String> {
        plan.validate()?;
        text(&workspace, 4096, true)?;
        if super::seal(&(
            &plan.workflow,
            &plan.parameters,
            &plan.targets,
            &plan.included,
        ))? != plan.seal
        {
            return Err("Expanded workflow seal mismatch".into());
        }
        let steps = plan
            .workflow
            .steps
            .iter()
            .map(|step| StepState {
                id: step.id.clone(),
                status: if plan.included.contains(&step.id) {
                    StepStatus::Pending
                } else {
                    StepStatus::Skipped
                },
                reason: if plan.included.contains(&step.id) {
                    String::new()
                } else {
                    "Outside selected target closure".into()
                },
                attempts: vec![],
                approved_attempt: None,
            })
            .collect();
        let mut run = Self {
            version: 1,
            id: new_id()?,
            workspace,
            plan,
            status: RunStatus::Draft,
            created: now,
            started: None,
            finished: None,
            steps,
            events: vec![],
            approved_seal: None,
            attempts_used: 0,
            output_used: 0,
            artifact_used: 0,
            model_calls_used: 0,
            model_tokens_reserved: 0,
            stop_reason: None,
        };
        run.event(
            now,
            None,
            "created",
            "Run created locally; approval required",
        );
        Ok(run)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.id.len() != 32
            || !self.id.bytes().all(|b| b.is_ascii_hexdigit())
            || self.steps.len() != self.plan.workflow.steps.len()
            || self.events.len() > 1024
            || self.attempts_used > 512
        {
            return Err("Invalid run record limits".into());
        }
        self.plan.validate()?;
        if super::seal(&(
            &self.plan.workflow,
            &self.plan.parameters,
            &self.plan.targets,
            &self.plan.included,
        ))? != self.plan.seal
        {
            return Err("Saved workflow plan changed".into());
        }
        let mut ids = BTreeSet::new();
        let mut tokens = BTreeSet::new();
        let mut total = 0u32;
        let mut output = 0u64;
        let mut artifacts = 0u64;
        let mut calls = 0u32;
        let mut reserved = 0u64;
        for state in &self.steps {
            if !ids.insert(&state.id)
                || !self
                    .plan
                    .workflow
                    .steps
                    .iter()
                    .any(|step| step.id == state.id)
                || state.attempts.len() > 6
            {
                return Err("Invalid saved step identity or attempt count".into());
            }
            text(&state.reason, 2000, false)?;
            let definition = self.definition(&state.id)?;
            let last = state.attempts.last();
            if (state.status == StepStatus::Running
                && last.is_none_or(|attempt| attempt.outcome.is_some()))
                || (matches!(
                    state.status,
                    StepStatus::Succeeded | StepStatus::Failed | StepStatus::Interrupted
                ) && last
                    .and_then(|attempt| attempt.outcome.as_ref())
                    .is_none_or(|outcome| {
                        outcome.passed != (state.status == StepStatus::Succeeded)
                    }))
            {
                return Err("Saved step status has no matching outcome".into());
            }
            if !self.plan.included.contains(&state.id)
                && (state.status != StepStatus::Skipped || !state.attempts.is_empty())
            {
                return Err("Excluded step acquired execution state".into());
            }
            for (index, attempt) in state.attempts.iter().enumerate() {
                if attempt.number != index as u32 + 1
                    || attempt.token.len() != 32
                    || !attempt.token.bytes().all(|b| b.is_ascii_hexdigit())
                    || !tokens.insert(&attempt.token)
                    || attempt.finished.is_some() != attempt.outcome.is_some()
                    || (attempt.outcome.is_none()
                        && (index + 1 != state.attempts.len()
                            || state.status != StepStatus::Running))
                    || attempt.finished.is_some_and(|end| end < attempt.started)
                {
                    return Err("Invalid saved attempt".into());
                }
                if let Some(outcome) = &attempt.outcome {
                    outcome.validate()?;
                    let expected = match definition.action {
                        StepKind::AiReview { .. } => Provenance::NativeModel,
                        StepKind::Check { .. } => Provenance::NativeProcess,
                        StepKind::Manual { .. } => Provenance::ManualReport,
                        StepKind::Approval { .. } => Provenance::Approval,
                        StepKind::Checkpoint { .. } => Provenance::RunCheckpoint,
                    };
                    if outcome.provenance != expected
                        && !(outcome.provenance == Provenance::Interrupted && !outcome.passed)
                    {
                        return Err("Saved provenance mismatch".into());
                    }
                    output = output.saturating_add(outcome.output_bytes);
                    artifacts = artifacts.saturating_add(outcome.artifact_bytes);
                }
                if let StepKind::AiReview { max_tokens, .. } = definition.action {
                    calls += 1;
                    reserved += u64::from(max_tokens);
                }
                total += 1;
            }
        }
        if total != self.attempts_used
            || output != self.output_used
            || artifacts != self.artifact_used
            || calls != self.model_calls_used
            || reserved != self.model_tokens_reserved
        {
            return Err("Run attempt ledger mismatch".into());
        }
        if self.terminal()
            && (self.finished.is_none() || self.steps.iter().any(|state| !state.status.terminal()))
        {
            return Err("Terminal run has unfinished steps".into());
        }
        if self.status == RunStatus::Succeeded
            && (self.stop_reason.is_some()
                || self.steps.iter().any(|state| {
                    state.status == StepStatus::Canceled
                        || (matches!(state.status, StepStatus::Failed | StepStatus::Interrupted)
                            && self.definition(&state.id).is_ok_and(|step| !step.optional))
                }))
        {
            return Err("Successful run contains a required failure".into());
        }
        if self
            .events
            .windows(2)
            .any(|pair| pair[0].sequence >= pair[1].sequence)
        {
            return Err("Run events are not ordered".into());
        }
        for event in &self.events {
            text(&event.kind, 80, true)?;
            text(&event.detail, 2000, false)?;
            if event.step.as_ref().is_some_and(|id| !ids.contains(id)) {
                return Err("Run event references an unknown step".into());
            }
        }
        Ok(())
    }
    pub fn approve(&mut self, seal: &str, now: u64) -> Result<(), String> {
        self.validate()?;
        if seal != self.plan.seal
            || !matches!(self.status, RunStatus::Draft | RunStatus::Interrupted)
        {
            return Err("Review the current draft/interrupted plan before queueing".into());
        }
        self.approved_seal = Some(seal.into());
        self.status = RunStatus::Queued;
        self.finished = None;
        self.stop_reason = None;
        self.event(
            now,
            None,
            "approved",
            "Exact local run plan approved; manual gates remain separate",
        );
        Ok(())
    }
    pub fn elapsed(&self, now: u64) -> u64 {
        self.started.map_or(0, |start| {
            self.finished.unwrap_or(now).saturating_sub(start)
        })
    }
    pub fn terminal(&self) -> bool {
        matches!(
            self.status,
            RunStatus::Succeeded | RunStatus::Failed | RunStatus::Canceled
        )
    }
    pub fn running(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status == StepStatus::Running)
            .count()
    }
    fn event(&mut self, now: u64, step: Option<String>, kind: &str, detail: &str) {
        let sequence = self
            .events
            .last()
            .map_or(1, |event| event.sequence.saturating_add(1));
        self.events.push(RunEvent {
            sequence,
            at: now,
            step,
            kind: kind.into(),
            detail: detail.chars().take(2000).collect(),
        });
        if self.events.len() > 1024 {
            self.events.remove(0);
        }
    }
    fn definition(&self, id: &str) -> Result<&super::Step, String> {
        self.plan
            .workflow
            .steps
            .iter()
            .find(|step| step.id == id)
            .ok_or_else(|| "Step is not in this plan".into())
    }
    fn state(&self, id: &str) -> Result<&StepState, String> {
        self.steps
            .iter()
            .find(|step| step.id == id)
            .ok_or_else(|| "Step state missing".into())
    }
    fn state_mut(&mut self, id: &str) -> Result<&mut StepState, String> {
        self.steps
            .iter_mut()
            .find(|step| step.id == id)
            .ok_or_else(|| "Step state missing".into())
    }
    fn failed_dependency(&self, id: &str) -> bool {
        let state = self.state(id).ok();
        let definition = self.definition(id).ok();
        state.is_some_and(|state| {
            matches!(
                state.status,
                StepStatus::Failed | StepStatus::Canceled | StepStatus::Interrupted
            ) || (state.status == StepStatus::Skipped
                && definition.is_none_or(|step| !step.optional))
        })
    }
    pub fn offers(&mut self, now: u64) -> Result<Vec<Offer>, String> {
        if self.terminal()
            || matches!(
                self.status,
                RunStatus::Draft | RunStatus::Paused | RunStatus::Interrupted | RunStatus::Stopping
            )
        {
            return Ok(vec![]);
        }
        if self.approved_seal.as_deref() != Some(&self.plan.seal) {
            return Err("Run has no current approval".into());
        }
        if self.elapsed(now) >= self.plan.workflow.policy.wall_seconds
            || self.output_used >= self.plan.workflow.policy.output_bytes
            || self.artifact_used > self.plan.workflow.policy.artifact_bytes
        {
            self.stop("Run budget exhausted", now);
            return Ok(vec![]);
        }
        let mut offers = Vec::new();
        for definition in self.plan.workflow.steps.clone() {
            let state = self.state(&definition.id)?;
            if !matches!(
                state.status,
                StepStatus::Pending | StepStatus::WaitingApproval
            ) || !self.plan.included.contains(&definition.id)
            {
                continue;
            }
            if !definition
                .needs
                .iter()
                .all(|id| self.state(id).is_ok_and(|state| state.status.terminal()))
            {
                continue;
            }
            let failed = definition.needs.iter().any(|id| self.failed_dependency(id));
            let permitted = match definition.when {
                When::Success => !failed,
                When::Failure => failed,
                When::Always => true,
            };
            if !permitted {
                let state = self.state_mut(&definition.id)?;
                state.status = StepStatus::Skipped;
                state.reason = "Dependency condition was not satisfied".into();
                self.event(
                    now,
                    Some(definition.id),
                    "skipped",
                    "Dependency condition was not satisfied",
                );
                continue;
            }
            let manual = matches!(
                definition.action,
                StepKind::Manual { .. } | StepKind::Approval { .. }
            );
            let attempt = self.state(&definition.id)?.attempts.len() as u32 + 1;
            let requires_approval = manual
                || matches!(definition.action, StepKind::AiReview { .. })
                || definition.review_each_attempt
                || attempt > 1;
            if requires_approval && self.state(&definition.id)?.approved_attempt != Some(attempt) {
                self.state_mut(&definition.id)?.status = StepStatus::WaitingApproval;
            }
            offers.push(Offer {
                step: definition.id.clone(),
                requires_approval: requires_approval
                    && self.state(&definition.id)?.approved_attempt != Some(attempt),
                manual,
                resources: definition.resources,
            });
        }
        self.settle(now);
        Ok(offers)
    }
    pub fn approve_attempt(&mut self, id: &str, now: u64) -> Result<(), String> {
        if !matches!(
            self.status,
            RunStatus::Queued | RunStatus::Running | RunStatus::Paused
        ) {
            return Err("Run is not awaiting attempt approval".into());
        }
        let state = self.state_mut(id)?;
        if !matches!(
            state.status,
            StepStatus::Pending | StepStatus::WaitingApproval
        ) {
            return Err("Step is not awaiting approval".into());
        }
        state.approved_attempt = Some(state.attempts.len() as u32 + 1);
        state.status = StepStatus::Pending;
        self.event(
            now,
            Some(id.into()),
            "attempt_approved",
            "One exact attempt approved",
        );
        Ok(())
    }
    /// Caller must persist this lease before starting a process or applying a manual decision.
    pub fn claim(&mut self, id: &str, now: u64) -> Result<String, String> {
        let offer = self
            .offers(now)?
            .into_iter()
            .find(|offer| offer.step == id)
            .ok_or("Step is not ready")?;
        if offer.requires_approval || self.running() >= self.plan.workflow.policy.parallelism {
            return Err("Step needs approval or a free concurrency slot".into());
        }
        if self
            .steps
            .iter()
            .filter(|step| step.status == StepStatus::Running)
            .any(|state| {
                self.definition(&state.id).is_ok_and(|step| {
                    step.resources
                        .iter()
                        .any(|resource| offer.resources.contains(resource))
                })
            })
        {
            return Err("Step resource is already owned".into());
        }
        if self.attempts_used >= self.plan.workflow.policy.max_attempts {
            return Err("Attempt budget exhausted".into());
        }
        let number = self.state(id)?.attempts.len() as u32 + 1;
        if number > self.definition(id)?.retries + 1 {
            return Err("Step retry budget exhausted".into());
        }
        if let StepKind::AiReview { max_tokens, .. } = self.definition(id)?.action {
            if self.model_calls_used >= self.plan.workflow.policy.model_calls
                || self
                    .model_tokens_reserved
                    .saturating_add(u64::from(max_tokens))
                    > self.plan.workflow.policy.model_output_tokens
            {
                return Err("AI request/token reservation budget exhausted".into());
            }
            self.model_calls_used += 1;
            self.model_tokens_reserved += u64::from(max_tokens);
        }
        let token = new_id()?;
        let state = self.state_mut(id)?;
        state.status = StepStatus::Running;
        state.reason.clear();
        state.approved_attempt = None;
        state.attempts.push(Attempt {
            number,
            token: token.clone(),
            started: now,
            finished: None,
            outcome: None,
        });
        self.attempts_used += 1;
        self.started.get_or_insert(now);
        self.status = RunStatus::Running;
        self.event(
            now,
            Some(id.into()),
            "started",
            "Attempt lease created; execution follows durable storage",
        );
        Ok(token)
    }
    pub fn complete(
        &mut self,
        id: &str,
        token: &str,
        outcome: Outcome,
        now: u64,
    ) -> Result<(), String> {
        outcome.validate()?;
        let definition = self.definition(id)?.clone();
        let expected = match definition.action {
            StepKind::AiReview { .. } => Provenance::NativeModel,
            StepKind::Check { .. } => Provenance::NativeProcess,
            StepKind::Manual { .. } => Provenance::ManualReport,
            StepKind::Approval { .. } => Provenance::Approval,
            StepKind::Checkpoint { .. } => Provenance::RunCheckpoint,
        };
        if outcome.provenance != expected {
            return Err("Outcome provenance does not match the step".into());
        }
        let stopping = self.status == RunStatus::Stopping;
        let state = self.state_mut(id)?;
        if state.status != StepStatus::Running
            || state
                .attempts
                .last()
                .is_none_or(|attempt| attempt.token != token || attempt.outcome.is_some())
        {
            return Err("Stale, duplicate or foreign attempt completion".into());
        }
        let attempt = state.attempts.last_mut().ok_or("Attempt missing")?;
        attempt.finished = Some(now.max(attempt.started));
        attempt.outcome = Some(outcome.clone());
        state.status = if outcome.canceled || stopping {
            StepStatus::Canceled
        } else if outcome.passed {
            StepStatus::Succeeded
        } else {
            StepStatus::Failed
        };
        state.reason = outcome.note.clone();
        let failed = state.status == StepStatus::Failed;
        self.output_used = self.output_used.saturating_add(outcome.output_bytes);
        self.artifact_used = self.artifact_used.saturating_add(outcome.artifact_bytes);
        self.event(
            now,
            Some(id.into()),
            if outcome.passed { "passed" } else { "failed" },
            &outcome.note,
        );
        if failed && !definition.optional && !self.plan.workflow.policy.continue_independent {
            for state in &mut self.steps {
                if matches!(
                    state.status,
                    StepStatus::Pending | StepStatus::WaitingApproval
                ) && self
                    .plan
                    .workflow
                    .steps
                    .iter()
                    .find(|step| step.id == state.id)
                    .is_some_and(|step| step.when != When::Always && step.when != When::Failure)
                {
                    state.status = StepStatus::Skipped;
                    state.reason = "Stopped by required-step failure".into();
                }
            }
        }
        self.settle(now);
        Ok(())
    }
    pub fn decide(&mut self, id: &str, passed: bool, note: String, now: u64) -> Result<(), String> {
        if !matches!(self.status, RunStatus::Queued | RunStatus::Running) {
            return Err("Resume the run before deciding a gate".into());
        }
        let provenance = match self.definition(id)?.action {
            StepKind::Manual { .. } => Provenance::ManualReport,
            StepKind::Approval { .. } => Provenance::Approval,
            _ => return Err("Only manual and approval gates accept a human decision".into()),
        };
        let outcome = Outcome::declared(passed, provenance, note)?;
        let mut next = self.clone();
        next.approve_attempt(id, now)?;
        let token = next.claim(id, now)?;
        next.complete(id, &token, outcome, now)?;
        *self = next;
        Ok(())
    }
    pub fn pause(&mut self, now: u64) -> Result<(), String> {
        if !matches!(self.status, RunStatus::Queued | RunStatus::Running) {
            return Err("Only queued/running work can be paused".into());
        }
        self.status = RunStatus::Paused;
        self.event(
            now,
            None,
            "paused",
            "New launches paused; owned running attempts may finish",
        );
        Ok(())
    }
    pub fn resume(&mut self, now: u64) -> Result<(), String> {
        if self.status != RunStatus::Paused {
            return Err("Only a paused run can resume".into());
        }
        self.status = if self.running() > 0 {
            RunStatus::Running
        } else {
            RunStatus::Queued
        };
        self.event(now, None, "resumed", "Resumed reviewed run");
        Ok(())
    }
    pub fn stop(&mut self, reason: &str, now: u64) {
        if self.terminal() {
            return;
        }
        self.stop_reason = Some(reason.chars().take(500).collect());
        self.approved_seal = None;
        for state in &mut self.steps {
            if matches!(
                state.status,
                StepStatus::Pending | StepStatus::WaitingApproval
            ) {
                state.status = StepStatus::Canceled;
                state.reason = reason.chars().take(500).collect();
                state.approved_attempt = None;
            }
        }
        self.status = if self.running() > 0 {
            RunStatus::Stopping
        } else {
            RunStatus::Canceled
        };
        self.event(now, None, "stop_requested", reason);
        self.settle(now);
    }
    pub fn skip_optional(&mut self, id: &str, note: &str, now: u64) -> Result<(), String> {
        text(note, 500, true)?;
        if !self.definition(id)?.optional
            || !matches!(
                self.state(id)?.status,
                StepStatus::Pending | StepStatus::WaitingApproval
            )
        {
            return Err("Only an optional, unstarted step can be skipped".into());
        }
        let state = self.state_mut(id)?;
        state.status = StepStatus::Skipped;
        state.reason = note.into();
        state.approved_attempt = None;
        self.event(now, Some(id.into()), "optional_skipped", note);
        self.settle(now);
        Ok(())
    }
    pub fn retry_from(&mut self, id: &str, now: u64) -> Result<(), String> {
        if !self.plan.workflow.policy.allow_retry
            || self.running() > 0
            || !self.state(id)?.status.terminal()
        {
            return Err(
                "Stop active work and enable retries before retrying a completed step".into(),
            );
        }
        let definition = self.definition(id)?;
        let used = self.state(id)?.attempts.len() as u32;
        if used > definition.retries || self.attempts_used >= self.plan.workflow.policy.max_attempts
        {
            return Err("Step retry or run attempt budget exhausted".into());
        }
        let mut reset: BTreeSet<String> = [id.to_owned()].into_iter().collect();
        loop {
            let before = reset.len();
            for step in &self.plan.workflow.steps {
                if step.needs.iter().any(|need| reset.contains(need)) {
                    reset.insert(step.id.clone());
                }
            }
            if before == reset.len() {
                break;
            }
        }
        for state in &mut self.steps {
            if reset.contains(&state.id) && self.plan.included.contains(&state.id) {
                state.status = StepStatus::Pending;
                state.reason = "Retry requires fresh full-plan review".into();
                state.approved_attempt = None;
            }
        }
        self.status = RunStatus::Draft;
        self.approved_seal = None;
        self.finished = None;
        self.stop_reason = None;
        self.event(
            now,
            Some(id.into()),
            "retry_staged",
            "Step and downstream steps reset; prior attempts remain in history",
        );
        Ok(())
    }
    pub fn recover(&mut self, now: u64) -> Result<(), String> {
        self.validate()?;
        if self.terminal() {
            return Ok(());
        }
        for state in &mut self.steps {
            state.approved_attempt = None;
            if state.status == StepStatus::Running {
                state.status = StepStatus::Interrupted;
                state.reason = "Process ownership cannot be restored after app restart".into();
                if let Some(attempt) = state.attempts.last_mut() {
                    attempt.finished = Some(now.max(attempt.started));
                    attempt.outcome = Some(Outcome::declared(
                        false,
                        Provenance::Interrupted,
                        "Interrupted by app restart".into(),
                    )?);
                }
            }
        }
        self.approved_seal = None;
        self.status = RunStatus::Interrupted;
        self.event(
            now,
            None,
            "recovered",
            "No process or approval resumed automatically",
        );
        Ok(())
    }
    fn settle(&mut self, now: u64) {
        if self.steps.iter().any(|state| !state.status.terminal()) {
            return;
        }
        let canceled = self.stop_reason.is_some()
            || self
                .steps
                .iter()
                .any(|state| state.status == StepStatus::Canceled);
        let failed = self.steps.iter().any(|state| {
            self.plan.included.contains(&state.id)
                && self.definition(&state.id).is_ok_and(|step| !step.optional)
                && matches!(state.status, StepStatus::Failed | StepStatus::Interrupted)
        });
        self.status = if canceled {
            RunStatus::Canceled
        } else if failed {
            RunStatus::Failed
        } else {
            RunStatus::Succeeded
        };
        self.finished.get_or_insert(now);
        self.approved_seal = None;
    }
    pub fn blockers(&self) -> BTreeMap<String, Vec<String>> {
        self.plan
            .workflow
            .steps
            .iter()
            .filter(|step| self.plan.included.contains(&step.id))
            .map(|step| {
                (
                    step.id.clone(),
                    step.needs
                        .iter()
                        .filter(|id| {
                            self.state(id)
                                .is_ok_and(|state| state.status != StepStatus::Succeeded)
                        })
                        .cloned()
                        .collect(),
                )
            })
            .collect()
    }
    pub fn status_summary(&self) -> String {
        let counts =
            self.steps
                .iter()
                .fold(BTreeMap::<String, usize>::new(), |mut counts, state| {
                    *counts.entry(format!("{:?}", state.status)).or_default() += 1;
                    counts
                });
        format!(
            "{}: {:?}; {}",
            self.plan.workflow.title,
            self.status,
            counts
                .into_iter()
                .map(|(status, count)| format!("{count} {status}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub fn validate_resource_names(resources: &[String]) -> Result<(), String> {
    if resources.len() > 8 || resources.iter().any(|resource| !identifier(resource)) {
        Err("Invalid resource set".into())
    } else {
        Ok(())
    }
}
