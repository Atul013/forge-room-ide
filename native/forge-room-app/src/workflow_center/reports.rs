use super::storage::{Record, Store};
use eframe::egui;
use forge_room_core::workflows::{RunStatus, StepKind, StepStatus};

pub fn brief(record: &Record) -> String {
    let run = &record.saved.run;
    let mut text = format!(
        "Local workflow run {}\n{}\nPlan seal {}\nAttempts {}/{}; output {}/{} bytes; artifacts {}/{} bytes; model requests {}/{}; reserved output tokens {}/{}\nNative processes, manual reports, approvals and AI suggestions are separate evidence. No desktop/provider/physical acceptance inferred.\n",
        run.id,
        run.status_summary(),
        run.plan.seal,
        run.attempts_used,
        run.plan.workflow.policy.max_attempts,
        run.output_used,
        run.plan.workflow.policy.output_bytes,
        run.artifact_used,
        run.plan.workflow.policy.artifact_bytes,
        run.model_calls_used,
        run.plan.workflow.policy.model_calls,
        run.model_tokens_reserved,
        run.plan.workflow.policy.model_output_tokens
    );
    for step in &run.steps {
        let provenance = step
            .attempts
            .last()
            .and_then(|attempt| attempt.outcome.as_ref())
            .map(|outcome| format!("{:?}", outcome.provenance))
            .unwrap_or_else(|| "No completed receipt".into());
        text.push_str(&format!(
            "{}: {:?}; {}; {} attempts\n",
            step.id,
            step.status,
            provenance,
            step.attempts.len()
        ));
    }
    text
}

pub fn receipt(record: &Record, step: &str) -> Result<String, String> {
    let state = record
        .saved
        .run
        .steps
        .iter()
        .find(|state| state.id == step)
        .ok_or("No selected step")?;
    let provenance = state
        .attempts
        .last()
        .and_then(|attempt| attempt.outcome.as_ref())
        .map(|outcome| {
            format!(
                "{:?}; exit {:?}; timeout {}; complete capture {}; truncated {}",
                outcome.provenance,
                outcome.exit_code,
                outcome.timed_out,
                outcome.capture_complete,
                outcome.truncated
            )
        })
        .unwrap_or_else(|| "No completed attempt".into());
    Ok(format!("Local workflow {} step {}: {:?}; {}. Attempt count {}. Status-only receipt; no logs/source/artifacts. Desktop, native-provider and physical-client acceptance are separate.", &record.saved.run.id[..12],step,state.status,provenance,state.attempts.len()).chars().take(500).collect())
}

pub fn comparison(left: &Record, right: &Record) -> String {
    let mut text = format!(
        "{} ({:?}) → {} ({:?})\nSame expanded plan: {}\nSame local review: {}\n",
        left.saved.run.id,
        left.saved.run.status,
        right.saved.run.id,
        right.saved.run.status,
        left.saved.run.plan.seal == right.saved.run.plan.seal,
        left.review.seal == right.review.seal
    );
    for state in &right.saved.run.steps {
        let prior = left.saved.run.steps.iter().find(|old| old.id == state.id);
        let prior_ms = prior
            .and_then(|old| old.attempts.last())
            .and_then(|attempt| attempt.outcome.as_ref())
            .map(|outcome| outcome.elapsed_ms);
        let next_ms = state
            .attempts
            .last()
            .and_then(|attempt| attempt.outcome.as_ref())
            .map(|outcome| outcome.elapsed_ms);
        let inputs_same = left
            .review
            .steps
            .get(&state.id)
            .zip(right.review.steps.get(&state.id))
            .is_some_and(|(a, b)| a.inputs == b.inputs);
        text.push_str(&format!(
            "{}: {:?} → {:?}; last elapsed {:?} → {:?} ms; same input seals {}\n",
            state.id,
            prior.map(|old| old.status),
            state.status,
            prior_ms,
            next_ms,
            inputs_same
        ));
    }
    text
}
pub enum Action {
    Approve,
    Pause,
    Resume,
    Stop,
    ApproveAttempt(String),
    Decision(String, bool, String),
    Skip(String, String),
    Retry(String),
    Delete,
    StageAi,
    StageRoom(String),
    Fork,
}

pub fn show(
    ui: &mut egui::Ui,
    record: &Record,
    store: &Store,
    selected: &mut String,
    note: &mut String,
    delete_confirm: &mut bool,
) -> Option<Action> {
    let run = &record.saved.run;
    let mut action = None;
    ui.heading(&run.plan.workflow.title);
    ui.monospace(&run.id);
    ui.label(run.status_summary());
    ui.horizontal_wrapped(|ui|{
        if matches!(run.status,RunStatus::Draft|RunStatus::Interrupted)&&ui.button("Approve frozen plan and queue").clicked(){
action=Some(Action::Approve);
}
        if matches!(run.status,RunStatus::Queued|RunStatus::Running)&&ui.button("Pause launches").clicked(){
action=Some(Action::Pause);
}
        if run.status==RunStatus::Paused&&ui.button("Resume launches").clicked(){
action=Some(Action::Resume);
}
        if !run.terminal()&&ui.button("Cancel owned work").clicked(){
action=Some(Action::Stop);
}
        if ui.button("New run from this definition").clicked(){
action=Some(Action::Fork);
}
        if ui.button("Stage status brief in AI context").clicked(){
action=Some(Action::StageAi);
}
        if ui.button("Copy status report").clicked(){
ui.ctx().copy_text(brief(record));
}
        if ui.button("Export status JSON…").clicked()&& let Some(path)=rfd::FileDialog::new().set_file_name("workflow-status.json").save_file(){
let export=serde_json::json!({
"version":1,"run":run.id,"plan":run.plan.seal,"status":format!("{:?}",run.status),"projection":"Status-only: no source, absolute paths, manual notes, model text, logs or artifact bodies","summary":brief(record)}
);
match super::library::create_file(&path,serde_json::to_string_pretty(&export).unwrap_or_default().as_bytes()){
Ok(())=>{
ui.label("Export saved");
}
,Err(error)=>{
ui.colored_label(egui::Color32::LIGHT_RED,error);
}
}
}
    }
);
    ui.collapsing("Frozen execution plan and local review", |ui| {
        ui.label(format!("Source: {}", record.review.source.display()));
        ui.label(format!("Review seal: {}", record.review.seal));
        ui.label(format!(
            "Tracked-file baseline: {}",
            record.review.seal_tracked
        ));
        ui.label(format!("Targets: {}", run.plan.targets.join(", ")));
        ui.label(format!("Parameters: {:?}", run.plan.parameters));
        for step in record.review.steps.values() {
            ui.separator();
            ui.strong(format!(
                "{} · {} · {}",
                step.id,
                step.context.alias,
                step.cwd.display()
            ));
            if let Some(command) = &step.command {
                ui.monospace(format!("{} {:?}", command.program.display(), command.args));
                ui.monospace(format!("Executable SHA-256 {}", command.binary.digest));
            }
            for input in &step.inputs {
                ui.monospace(format!(
                    "{} · {} bytes · {}",
                    input.path.display(),
                    input.bytes,
                    input.digest
                ));
            }
            ui.label(format!("Capture: {}", step.artifacts.join(", ")));
        }
        ui.collapsing("Expanded definition JSON", |ui| {
            ui.monospace(serde_json::to_string_pretty(&run.plan.workflow).unwrap_or_default());
        });
    });
    egui::Grid::new(("run-graph", &run.id))
        .striped(true)
        .num_columns(5)
        .show(ui, |ui| {
            ui.strong("Step");
            ui.strong("State");
            ui.strong("Prerequisites / condition");
            ui.strong("Attempts");
            ui.strong("Evidence type");
            ui.end_row();
            for definition in &run.plan.workflow.steps {
                let state = run
                    .steps
                    .iter()
                    .find(|state| state.id == definition.id)
                    .unwrap();
                if ui
                    .selectable_label(*selected == state.id, &definition.title)
                    .clicked()
                {
                    *selected = state.id.clone();
                    note.clear();
                }
                ui.label(format!("{:?}", state.status));
                ui.label(format!(
                    "{} / {:?}",
                    definition.needs.join(", "),
                    definition.when
                ));
                ui.label(state.attempts.len().to_string());
                ui.label(match definition.action {
                    StepKind::Check { .. } => "Native process",
                    StepKind::Manual { .. } => "Manual declaration",
                    StepKind::Approval { .. } => "Human approval",
                    StepKind::Checkpoint { .. } => "Run checkpoint",
                    StepKind::AiReview { .. } => "AI suggestion",
                });
                ui.end_row();
            }
        });
    if let Some(state) = run.steps.iter().find(|state| state.id == *selected) {
        let definition = run
            .plan
            .workflow
            .steps
            .iter()
            .find(|step| step.id == state.id)
            .unwrap();
        ui.separator();
        ui.strong(format!("{} · {}", state.id, definition.title));
        ui.label(&state.reason);
        match &definition.action {
            StepKind::Manual { instructions } => {
                ui.label(instructions);
            }
            StepKind::Approval { prompt } => {
                ui.label(prompt);
            }
            _ => (),
        }
        ui.label("Manual result / skip explanation (local private history)");
        ui.add(
            egui::TextEdit::multiline(note)
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );
        let ready = matches!(run.status, RunStatus::Queued | RunStatus::Running)
            && matches!(
                state.status,
                StepStatus::Pending | StepStatus::WaitingApproval
            )
            && definition.needs.iter().all(|need| {
                run.steps
                    .iter()
                    .any(|state| &state.id == need && state.status.terminal())
            });
        ui.horizontal_wrapped(|ui| {
            if ready {
                if matches!(
                    definition.action,
                    StepKind::Manual { .. } | StepKind::Approval { .. }
                ) {
                    if ui
                        .add_enabled(
                            !note.trim().is_empty(),
                            egui::Button::new("Record manual pass / approve"),
                        )
                        .clicked()
                    {
                        action = Some(Action::Decision(state.id.clone(), true, note.clone()));
                    }
                    if ui
                        .add_enabled(
                            !note.trim().is_empty(),
                            egui::Button::new("Record manual failure / deny"),
                        )
                        .clicked()
                    {
                        action = Some(Action::Decision(state.id.clone(), false, note.clone()));
                    }
                } else if state.status == StepStatus::WaitingApproval
                    && ui.button("Approve one attempt").clicked()
                {
                    action = Some(Action::ApproveAttempt(state.id.clone()));
                }
            }
            if definition.optional
                && matches!(
                    state.status,
                    StepStatus::Pending | StepStatus::WaitingApproval
                )
                && ui
                    .add_enabled(
                        !note.trim().is_empty(),
                        egui::Button::new("Skip optional step"),
                    )
                    .clicked()
            {
                action = Some(Action::Skip(state.id.clone(), note.clone()));
            }
            if run.plan.workflow.policy.allow_retry
                && run.running() == 0
                && state.status.terminal()
                && ui.button("Stage retry and downstream reset").clicked()
            {
                action = Some(Action::Retry(state.id.clone()));
            }
            if definition.room_task.is_some()
                && ui.button("Preview status receipt for Room").clicked()
            {
                action = Some(Action::StageRoom(state.id.clone()));
            }
        });
        for reference in record
            .saved
            .details
            .iter()
            .filter(|reference| reference.step == state.id)
        {
            ui.collapsing(
                format!("Attempt {} receipt and local output", reference.attempt),
                |ui| match store.detail(&run.id, reference) {
                    Err(error) => {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                    Ok(detail) => {
                        ui.label(&detail.note);
                        if let Some(command) = &detail.command {
                            ui.monospace(command);
                        }
                        if let Some(evidence) = &detail.native {
                            ui.label(format!(
                                "Exit {:?} · {} ms · complete capture {} · truncated {}",
                                evidence.exit_code,
                                evidence.elapsed_ms,
                                evidence.capture_complete,
                                evidence.truncated
                            ));
                            ui.collapsing("Private stdout / stderr", |ui| {
                                ui.monospace(&evidence.stdout);
                                ui.monospace(&evidence.stderr);
                                if ui.button("Copy local command output").clicked() {
                                    ui.ctx().copy_text(format!(
                                        "{}\n{}",
                                        evidence.stdout, evidence.stderr
                                    ));
                                }
                            });
                        }
                        if let Some(model) = &detail.model {
                            ui.label(format!(
                                "Native model {} at {}",
                                model,
                                detail.model_endpoint.as_deref().unwrap_or("")
                            ));
                        }
                        if let Some(text) = &detail.model_text {
                            ui.collapsing("Private model response (suggestion)", |ui| {
                                ui.label(text);
                            });
                        }
                        ui.label(format!("Inputs unchanged: {}", detail.inputs_unchanged));
                        let verified = store.verify_artifacts(&run.id, &detail);
                        for artifact in &detail.artifacts {
                            ui.label(format!(
                                "{} · {} bytes · SHA-256 {} · stored integrity {} · {}",
                                artifact.requested,
                                artifact.bytes,
                                artifact.digest,
                                verified.get(&artifact.requested).copied().unwrap_or(false),
                                artifact.error.as_deref().unwrap_or("captured locally")
                            ));
                            if verified.get(&artifact.requested) == Some(&true)
                                && ui
                                    .button(format!("Export {}…", artifact.requested))
                                    .clicked()
                                && let (Some(name), Some(target)) =
                                    (&artifact.stored, rfd::FileDialog::new().save_file())
                            {
                                let _ = name;
                                match store.export_artifact(&run.id, artifact, &target) {
                                    Ok(()) => {
                                        ui.label("Artifact exported");
                                    }
                                    Err(error) => {
                                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                                    }
                                }
                            }
                        }
                    }
                },
            );
        }
    }
    ui.collapsing("Run timeline", |ui| {
        for event in run.events.iter().rev() {
            ui.label(format!(
                "{} · {} · {} · {}",
                event.sequence, event.at, event.kind, event.detail
            ));
        }
    });
    ui.collapsing("Delete private run history", |ui| {
        ui.checkbox(
            delete_confirm,
            "Delete this inactive run's records, private output and captured artifacts",
        );
        if ui
            .add_enabled(
                *delete_confirm
                    && (run.terminal()
                        || matches!(run.status, RunStatus::Draft | RunStatus::Interrupted)),
                egui::Button::new("Delete history permanently"),
            )
            .clicked()
        {
            action = Some(Action::Delete);
        }
    });
    action
}
