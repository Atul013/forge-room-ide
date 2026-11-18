//! Scheduler owns only processes/requests started here. A saved lease always precedes launch.
use super::{
    resolver::{self, Context, Review},
    storage::{Detail, Record, Store},
};
use crate::{
    controlled_build::validation::{self, Plan},
    native_models,
};
use eframe::egui;
use forge_room_core::workflows::{self, Outcome, Provenance, RunStatus, StepKind};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Instant,
};
enum Work {
    Verify(mpsc::Receiver<Result<Option<Plan>, String>>),
    Process(validation::Run),
    Model(native_models::Run),
    Send(native_models::Prepared),
}
struct Active {
    run: String,
    step: String,
    token: String,
    attempt: u32,
    work: Work,
    start: Instant,
    text: String,
    endpoint: Option<String>,
    model: Option<String>,
    prompt_digest: Option<String>,
}
pub struct Engine {
    pub store: Store,
    pub records: Vec<Record>,
    active: Vec<Active>,
    pub fault: Option<String>,
    clocks: BTreeMap<String, Instant>,
}
fn now() -> u64 {
    forge_room_core::now_epoch_seconds()
}
impl Engine {
    pub fn open(root: &Path) -> Result<Self, String> {
        let store = Store::open(root)?;
        Self::from_store(store)
    }
    pub(super) fn from_store(store: Store) -> Result<Self, String> {
        let mut records = Vec::new();
        for id in store.ids()? {
            let mut record = store.load(&id)?;
            if !record.saved.run.terminal() {
                record.saved.run.recover(now())?;
                store.save(&mut record)?;
            }
            records.push(record);
        }
        records.sort_by_key(|record| record.saved.run.created);
        Ok(Self {
            store,
            records,
            active: vec![],
            fault: None,
            clocks: BTreeMap::new(),
        })
    }
    pub fn add(&mut self, review: Review) -> Result<String, String> {
        if self.fault.is_some() {
            return Err(
                "Workflow storage is stopped; close/reopen after resolving its error".into(),
            );
        }
        if self
            .records
            .iter()
            .filter(|record| {
                !record.saved.run.terminal() && record.saved.run.status != RunStatus::Interrupted
            })
            .count()
            >= 8
        {
            return Err("Queue limit is eight active/draft runs".into());
        }
        let run = workflows::Run::new(
            review.expanded.clone(),
            review.source.to_string_lossy().into(),
            now(),
        )?;
        let id = run.id.clone();
        let mut record = Record::new(run, review)?;
        self.store.save(&mut record)?;
        self.records.push(record);
        Ok(id)
    }
    fn save_index(&mut self, index: usize) -> bool {
        if let Err(error) = self.store.save(&mut self.records[index]) {
            self.fail(error);
            false
        } else {
            true
        }
    }
    fn fail(&mut self, error: String) {
        self.fault = Some(error);
        for active in &mut self.active {
            match &mut active.work {
                Work::Process(run) => run.cancel(),
                Work::Model(run) => run.cancel(),
                _ => (),
            }
        }
    }
    pub fn change(
        &mut self,
        id: &str,
        action: impl FnOnce(&mut workflows::Run) -> Result<(), String>,
    ) -> Result<(), String> {
        if self.fault.is_some() {
            return Err("Workflow writer is stopped".into());
        }
        let index = self
            .records
            .iter()
            .position(|record| record.saved.run.id == id)
            .ok_or("Unknown run")?;
        let mut next = self.records[index].clone();
        action(&mut next.saved.run)?;
        self.store
            .save(&mut next)
            .inspect_err(|error| self.fail(error.clone()))?;
        self.records[index] = next;
        Ok(())
    }
    pub fn stop_all(&mut self, reason: &str) {
        for index in 0..self.records.len() {
            if !self.records[index].saved.run.terminal()
                && !matches!(
                    self.records[index].saved.run.status,
                    RunStatus::Draft | RunStatus::Interrupted | RunStatus::Stopping
                )
            {
                self.records[index].saved.run.stop(reason, now());
                if self.fault.is_none() {
                    self.save_index(index);
                }
            }
        }
        for active in &mut self.active {
            match &mut active.work {
                Work::Process(run) => run.cancel(),
                Work::Model(run) => run.cancel(),
                _ => (),
            }
        }
    }
    fn busy(&self, record: &Record, id: &str) -> bool {
        let Some(step) = record.review.steps.get(id) else {
            return true;
        };
        let resources = &record
            .saved
            .run
            .plan
            .workflow
            .steps
            .iter()
            .find(|step| step.id == id)
            .unwrap()
            .resources;
        self.active.iter().any(|active| {
            self.records
                .iter()
                .find(|r| r.saved.run.id == active.run)
                .is_some_and(|other| {
                    other.review.steps.get(&active.step).is_some_and(|running| {
                        running.context.root.starts_with(&step.context.root)
                            || step.context.root.starts_with(&running.context.root)
                    }) || other
                        .saved
                        .run
                        .plan
                        .workflow
                        .steps
                        .iter()
                        .find(|step| step.id == active.step)
                        .is_some_and(|step| {
                            step.resources
                                .iter()
                                .any(|resource| resources.contains(resource))
                        })
                })
        })
    }
    pub fn tick(
        &mut self,
        ctx: &egui::Context,
        root: Option<&Path>,
        trusted: bool,
        dirty: &[PathBuf],
        contexts: &BTreeMap<String, Context>,
        config: Option<&native_models::Config>,
    ) {
        let valid = trusted
            && root
                .and_then(|root| resolver::canonical(root).ok())
                .as_ref()
                == Some(&self.store.source)
            && dirty.is_empty();
        if !valid {
            self.stop_all("Workspace trust, active root or saved-buffer state changed");
        }
        let mut index = 0;
        while index < self.active.len() {
            let record_index = self
                .records
                .iter()
                .position(|record| record.saved.run.id == self.active[index].run)
                .unwrap();
            let record = &self.records[record_index];
            let stop = !valid
                || self.fault.is_some()
                || record.saved.run.status == RunStatus::Stopping
                || record
                    .review
                    .steps
                    .get(&self.active[index].step)
                    .is_none_or(|step| contexts.get(&step.context.alias) != Some(&step.context));
            let mut finished = None;
            let mut verified = None;
            match &mut self.active[index].work {
                Work::Verify(_) if stop => {
                    finished = Some(Err("Review invalidated before launch".into()))
                }
                Work::Verify(_) if record.saved.run.status == RunStatus::Paused => (),
                Work::Verify(receiver) => match receiver.try_recv() {
                    Ok(value) => verified = Some(value),
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = Some(Err("Verification worker stopped".into()))
                    }
                    Err(_) => (),
                },
                Work::Process(run) => {
                    if stop {
                        run.cancel();
                    }
                    if let Some(result) = run.poll() {
                        finished = Some(result.map(Completion::Native));
                    }
                }
                Work::Model(run) => {
                    if stop {
                        run.cancel();
                    }
                    for event in run.poll() {
                        match event {
                            crate::agent::Event::Text(text) => {
                                if self.active[index].text.len() + text.len() <= 512 * 1024 {
                                    self.active[index].text.push_str(&text);
                                }
                            }
                            crate::agent::Event::Done { error, .. } => {
                                finished = Some(if stop {
                                    Err("Model request canceled".into())
                                } else if let Some(error) = error {
                                    Err(error)
                                } else {
                                    Ok(Completion::Model)
                                })
                            }
                            _ => (),
                        }
                    }
                }
                Work::Send(_) => {
                    if stop {
                        finished = Some(Err("AI review canceled before send".into()));
                    }
                }
            }
            if let Some(result) = verified {
                if stop {
                    finished = Some(Err("Review invalidated before process launch".into()));
                } else {
                    match result {
                        Err(error) => finished = Some(Err(error)),
                        Ok(Some(plan)) => {
                            let seconds = self.records[record_index]
                                .saved
                                .run
                                .plan
                                .workflow
                                .steps
                                .iter()
                                .find(|step| step.id == self.active[index].step)
                                .unwrap()
                                .timeout_seconds;
                            let wake = ctx.clone();
                            self.active[index].work =
                                Work::Process(validation::Run::start(plan, seconds, move || {
                                    wake.request_repaint()
                                }));
                        }
                        Ok(None) => {
                            let step = self.records[record_index]
                                .saved
                                .run
                                .plan
                                .workflow
                                .steps
                                .iter()
                                .find(|step| step.id == self.active[index].step)
                                .unwrap();
                            match &step.action {
                                StepKind::Checkpoint { .. } => {
                                    finished = Some(Ok(Completion::Checkpoint))
                                }
                                StepKind::AiReview {
                                    prompt,
                                    include_statuses,
                                    include_inputs,
                                    max_tokens,
                                } => {
                                    let prepared = (|| {
                                        let mut config = config.cloned().ok_or("Enable and configure native HTTP in the AI panel first")?;
                                        config.max_tokens = *max_tokens;
                                        config.seconds =
                                            config.seconds.min(step.timeout_seconds).clamp(10, 600);
                                        config.history = false;
                                        let mut prompt = prompt.clone();
                                        if *include_statuses {
                                            prompt.push_str("\n\nLocal status summary (manual reports are not native test proof):\n");
                                            prompt.push_str(
                                                &self.records[record_index]
                                                    .saved
                                                    .run
                                                    .status_summary(),
                                            );
                                            for state in &self.records[record_index].saved.run.steps
                                            {
                                                prompt.push_str(&format!(
                                                    "\n{}: {:?}",
                                                    state.id, state.status
                                                ));
                                            }
                                        }
                                        if *include_inputs {
                                            let reviewed =
                                                &self.records[record_index].review.steps[&step.id];
                                            for input in &reviewed.inputs {
                                                let source = crate::ai_workflow::source(
                                                    &reviewed.context.root,
                                                    &input.path,
                                                )?;
                                                forge_room_core::transport::validate_patch(
                                                    &source.relative,
                                                    &source.before,
                                                    &format!("{}\n", source.before),
                                                )?;
                                                prompt.push_str(&format!(
                                                    "\n\nUntrusted input {}:\n{}",
                                                    source.relative, source.before
                                                ));
                                                if prompt.len() > 128 * 1024 {
                                                    return Err("AI step prompt exceeds 128 KiB; reduce selected inputs".into());
                                                }
                                            }
                                        }
                                        native_models::prepare(&config, prompt, &[])
                                    })();
                                    match prepared {
                                        Ok(prepared) => {
                                            self.active[index].work = Work::Send(prepared)
                                        }
                                        Err(error) => finished = Some(Err(error)),
                                    }
                                }
                                _ => finished = Some(Err("Unexpected executable step kind".into())),
                            }
                        }
                    }
                }
            }
            if let Some(result) = finished {
                let active = self.active.remove(index);
                self.finish(active, result, record_index);
            } else {
                index += 1;
            }
        }
        if !valid || self.fault.is_some() {
            return;
        }
        for record_index in 0..self.records.len() {
            let before = self.records[record_index].saved.run.clone();
            let id = before.id.clone();
            if let Some(clock) = self.clocks.get(&id)
                && clock.elapsed().as_secs() >= before.plan.workflow.policy.wall_seconds
            {
                self.records[record_index]
                    .saved
                    .run
                    .stop("Monotonic run time budget exhausted", now());
            }
            let offers = match self.records[record_index].saved.run.offers(now()) {
                Ok(offers) => offers,
                Err(error) => {
                    self.fail(error);
                    return;
                }
            };
            if before != self.records[record_index].saved.run && !self.save_index(record_index) {
                return;
            }
            for offer in offers {
                if self.active.len() >= 4 {
                    break;
                }
                if offer.manual
                    || offer.requires_approval
                    || self.busy(&self.records[record_index], &offer.step)
                    || self.records[record_index].saved.run.running()
                        >= self.records[record_index]
                            .saved
                            .run
                            .plan
                            .workflow
                            .policy
                            .parallelism
                {
                    continue;
                }
                if self.records[record_index]
                    .review
                    .steps
                    .get(&offer.step)
                    .is_none_or(|step| contexts.get(&step.context.alias) != Some(&step.context))
                {
                    self.records[record_index]
                        .saved
                        .run
                        .stop("Execution context changed; review a new run", now());
                    self.save_index(record_index);
                    break;
                }
                let token = match self.records[record_index]
                    .saved
                    .run
                    .claim(&offer.step, now())
                {
                    Ok(token) => token,
                    Err(error) => {
                        self.records[record_index].saved.run.stop(&error, now());
                        self.save_index(record_index);
                        break;
                    }
                };
                if !self.save_index(record_index) {
                    return;
                }
                self.clocks.entry(id.clone()).or_insert_with(Instant::now);
                let attempt = self.records[record_index]
                    .saved
                    .run
                    .steps
                    .iter()
                    .find(|step| step.id == offer.step)
                    .unwrap()
                    .attempts
                    .len() as u32;
                let review = self.records[record_index].review.clone();
                let step = offer.step.clone();
                let (tx, receiver) = mpsc::channel();
                let wake = ctx.clone();
                std::thread::spawn(move || {
                    let result = review.verify_step(&step);
                    let _ = tx.send(result);
                    wake.request_repaint();
                });
                self.active.push(Active {
                    run: id.clone(),
                    step: offer.step,
                    token,
                    attempt,
                    work: Work::Verify(receiver),
                    start: Instant::now(),
                    text: String::new(),
                    endpoint: None,
                    model: None,
                    prompt_digest: None,
                });
            }
        }
        if !self.active.is_empty()
            || self.records.iter().any(|record| {
                matches!(
                    record.saved.run.status,
                    RunStatus::Queued | RunStatus::Running | RunStatus::Stopping
                )
            })
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
    fn finish(&mut self, active: Active, result: Result<Completion, String>, index: usize) {
        let reviewed = &self.records[index].review.steps[&active.step];
        let unchanged = self.records[index].review.inputs_unchanged(&active.step);
        let command = reviewed.command.as_ref().map(|command| {
            format!(
                "{} {:?} (cwd {})",
                command.program.display(),
                command.args,
                command.root.display()
            )
        });
        let mut detail = Detail {
            version: 1,
            run: active.run.clone(),
            step: active.step.clone(),
            attempt: active.attempt,
            command,
            native: None,
            model_endpoint: active.endpoint,
            model: active.model,
            prompt_digest: active.prompt_digest,
            model_text: None,
            note: String::new(),
            inputs_unchanged: unchanged,
            artifacts: vec![],
        };
        let definition = self.records[index]
            .saved
            .run
            .plan
            .workflow
            .steps
            .iter()
            .find(|step| step.id == active.step)
            .unwrap();
        let provenance = match definition.action {
            StepKind::Check { .. } => Provenance::NativeProcess,
            StepKind::AiReview { .. } => Provenance::NativeModel,
            StepKind::Checkpoint { .. } => Provenance::RunCheckpoint,
            _ => unreachable!(),
        };
        let mut outcome = Outcome {
            passed: false,
            provenance,
            note: String::new(),
            exit_code: None,
            timed_out: false,
            canceled: self.records[index].saved.run.status == RunStatus::Stopping,
            capture_complete: true,
            truncated: false,
            elapsed_ms: active.start.elapsed().as_millis(),
            output_bytes: 0,
            artifact_bytes: 0,
        };
        match result {
            Err(error) => {
                outcome.note = error.chars().take(1800).collect();
            }
            Ok(Completion::Native(evidence)) => {
                outcome.passed = evidence.exit_code == Some(0)
                    && !evidence.timed_out
                    && !evidence.stopped
                    && evidence.capture_complete;
                outcome.exit_code = evidence.exit_code;
                outcome.timed_out = evidence.timed_out;
                outcome.canceled |= evidence.stopped;
                outcome.capture_complete = evidence.capture_complete;
                outcome.truncated = evidence.truncated;
                outcome.output_bytes = (evidence.stdout.len() + evidence.stderr.len()) as u64;
                outcome.note = format!(
                    "Native process exit {:?}; complete capture {}; truncated {}",
                    evidence.exit_code, evidence.capture_complete, evidence.truncated
                );
                detail.native = Some(evidence);
            }
            Ok(Completion::Model) => {
                outcome.passed = true;
                outcome.output_bytes = active.text.len() as u64;
                outcome.note =
                    "Native model response received; suggestion only, no test/desktop acceptance"
                        .into();
                detail.model_text = Some(active.text.clone());
            }
            Ok(Completion::Checkpoint) => {
                outcome.passed = true;
                outcome.note = "Local run checkpoint recorded".into();
            }
        }
        if !unchanged {
            outcome.passed = false;
            outcome.note = "Reviewed inputs changed during execution; result is stale".into();
        }
        if outcome.canceled {
            outcome.passed = false;
        }
        if outcome.output_bytes
            > self.records[index]
                .saved
                .run
                .plan
                .workflow
                .policy
                .output_bytes
                .saturating_sub(self.records[index].saved.run.output_used)
        {
            outcome.passed = false;
            outcome.note = "Captured output exceeded the remaining run budget".into();
        }
        if provenance == Provenance::NativeModel
            && detail.model_text.is_none()
            && !active.text.is_empty()
        {
            outcome.output_bytes = active.text.len() as u64;
            detail.model_text = Some(active.text);
        }
        let remaining = self.records[index]
            .saved
            .run
            .plan
            .workflow
            .policy
            .artifact_bytes
            .saturating_sub(self.records[index].saved.run.artifact_used);
        for artifact in &reviewed.artifacts {
            let artifact = self.store.capture_artifact(
                &active.run,
                &active.step,
                active.attempt,
                &reviewed.cwd,
                artifact,
                remaining.saturating_sub(outcome.artifact_bytes),
            );
            outcome.artifact_bytes += artifact.bytes;
            if artifact.error.is_some() {
                outcome.passed = false;
                outcome.note = "Declared artifact could not be captured; inspect receipt".into();
            }
            detail.artifacts.push(artifact);
        }
        detail.note = outcome.note.clone();
        if self.fault.is_some() {
            return;
        }
        if let Err(error) = self.store.save_detail(&mut self.records[index], &detail) {
            self.fail(error);
            return;
        }
        if let Err(error) =
            self.records[index]
                .saved
                .run
                .complete(&active.step, &active.token, outcome, now())
        {
            self.fail(error);
            return;
        }
        self.save_index(index);
    }
    pub fn model_review(&self) -> Option<(&str, &str, &native_models::Prepared)> {
        self.active.iter().find_map(|active| {
            if let Work::Send(prepared) = &active.work {
                Some((active.run.as_str(), active.step.as_str(), prepared))
            } else {
                None
            }
        })
    }
    pub fn send_model(&mut self, run: &str, step: &str, ctx: &egui::Context) -> Result<(), String> {
        if self.fault.is_some() {
            return Err("Workflow writer is stopped".into());
        }
        let record = self
            .records
            .iter()
            .find(|record| record.saved.run.id == run)
            .ok_or("Run missing")?;
        if !matches!(
            record.saved.run.status,
            RunStatus::Queued | RunStatus::Running
        ) {
            return Err("Resume the run before sending".into());
        }
        record.review.verify_step(step)?;
        let active = self
            .active
            .iter_mut()
            .find(|active| active.run == run && active.step == step)
            .ok_or("AI step is no longer awaiting send")?;
        let Work::Send(prepared) = &active.work else {
            return Err("AI request is not awaiting review".into());
        };
        let wake = ctx.clone();
        let request = serde_json::json!({"version":1,"run":run,"step":step,"attempt":active.attempt,"endpoint":prepared.config.base_url,"model":prepared.config.model,"max_tokens":prepared.config.max_tokens,"prompt_digest":workflows::digest(prepared.prompt.as_bytes()),"body_digest":workflows::seal(&prepared.body)?});
        if let Err(error) = self
            .store
            .request_receipt(run, step, active.attempt, &request)
        {
            self.fail(error.clone());
            return Err(error);
        }
        let worker = native_models::start(prepared.clone(), move || wake.request_repaint())?;
        active.endpoint = Some(prepared.config.base_url.clone());
        active.model = Some(prepared.config.model.clone());
        active.prompt_digest = Some(workflows::digest(prepared.prompt.as_bytes()));
        active.work = Work::Model(worker);
        Ok(())
    }
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        if self.active.iter().any(|active| active.run == id) {
            return Err("Owned attempts are still stopping".into());
        }
        self.store.delete(id)?;
        self.records.retain(|record| record.saved.run.id != id);
        self.clocks.remove(id);
        Ok(())
    }
}
enum Completion {
    Native(validation::Evidence),
    Model,
    Checkpoint,
}
