//! Forms for shared Room intents. Authorization and visibility belong to the server.
use eframe::egui;
use forge_room_core::{
    EvidenceKind, TaskPriority, TaskStatus,
    transport::{Action, Snapshot},
};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

#[derive(Default)]
pub struct Workflow {
    failed_drafts: Vec<Action>,
    milestone_title: String,
    milestone_due: Option<u64>,
    tab: usize,
    draft: String,
    recipient: Option<String>,
    task_link: Option<String>,
    decision: bool,
    reply: Option<u64>,
    query: String,
    decisions_only: bool,
    title: String,
    owner: String,
    criteria: String,
    dependencies: String,
    priority: TaskPriority,
    due: Option<u64>,
    selected: Option<String>,
    checked: HashSet<String>,
    target: String,
    evidence: HashMap<String, String>,
    minutes: u64,
    clock_sample: Option<(u64, bool, Instant)>,
}
impl Workflow {
    fn slot_free(&self, action: &Action) -> bool {
        match action {
            Action::Message { .. } => self.draft.is_empty(),
            Action::CreateTask { .. } => self.title.is_empty(),
            Action::Evidence { task, .. } => {
                self.evidence.get(task).is_none_or(|text| text.is_empty())
            }
            _ => true,
        }
    }
    pub fn restore(&mut self, action: Action) {
        if !self.slot_free(&action) || matches!(&action, Action::Evidence { .. }) {
            self.failed_drafts.push(action);
            return;
        }
        self.restore_fields(action);
    }
    fn restore_fields(&mut self, action: Action) {
        match action {
            Action::Message {
                text,
                recipient,
                task,
                decision,
                parent,
            } => {
                self.draft = text;
                self.recipient = recipient;
                self.task_link = task;
                self.decision = decision;
                self.reply = parent;
            }
            Action::CreateTask {
                title,
                owner,
                criteria,
                dependencies,
                priority,
                due_hours,
            } => {
                self.title = title;
                self.owner = owner;
                self.criteria = criteria.join("\n");
                self.dependencies = dependencies.join(",");
                self.priority = priority;
                self.due = due_hours;
            }
            Action::Evidence { task, text, .. } => {
                self.evidence.insert(task, text);
            }
            _ => {}
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, snapshot: &Snapshot, host: bool) -> Vec<Action> {
        let mut actions = Vec::new();
        if !self.failed_drafts.is_empty() {
            egui::CollapsingHeader::new(format!("Rejected drafts ({})", self.failed_drafts.len()))
                .default_open(true)
                .show(ui, |ui| {
                    let mut restore = None;
                    let mut discard = None;
                    for (index, action) in self.failed_drafts.iter().enumerate() {
                        ui.label(match action {
                            Action::Message { text, .. } => {
                                format!("Message: {}", text.chars().take(120).collect::<String>())
                            }
                            Action::CreateTask { title, .. } => format!("Task: {title}"),
                            Action::Evidence { task, text, .. } => format!(
                                "Evidence for {task}: {}",
                                text.chars().take(120).collect::<String>()
                            ),
                            _ => "Rejected action".into(),
                        });
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    self.slot_free(action)
                                        && match action {
                                            Action::Evidence { task, .. } => {
                                                host || snapshot.shared.tasks.iter().any(|t| {
                                                    &t.id == task && t.owner_id == snapshot.you
                                                })
                                            }
                                            _ => true,
                                        },
                                    egui::Button::new("Restore draft"),
                                )
                                .clicked()
                            {
                                restore = Some(index);
                            }
                            if ui.button("Copy draft data").clicked()
                                && let Ok(text) = serde_json::to_string_pretty(action)
                            {
                                ui.ctx().copy_text(text);
                            }
                            if ui.button("Discard draft").clicked() {
                                discard = Some(index);
                            }
                        });
                    }
                    if let Some(index) = restore {
                        let action = self.failed_drafts.remove(index);
                        match &action {
                            Action::Message { .. } => self.tab = 0,
                            Action::Evidence { task, .. } => {
                                self.selected = Some(task.clone());
                                self.checked.clear();
                                self.tab = 1;
                            }
                            _ => self.tab = 1,
                        }
                        self.restore_fields(action);
                    } else if let Some(index) = discard {
                        self.failed_drafts.remove(index);
                    }
                });
        }
        if self.failed_drafts.len() >= 16 {
            ui.label("Resolve rejected drafts before sending more messages or task text.");
        }

        ui.horizontal_wrapped(|ui| {
            for (index, label) in ["Chat", "Tasks", "Activity"].iter().enumerate() {
                ui.selectable_value(&mut self.tab, index, *label);
            }
        });
        let clock = &snapshot.shared.countdown;
        // Poll samples come from the host. Between samples the displayed clock advances locally.
        if self
            .clock_sample
            .as_ref()
            .is_none_or(|(seconds, running, _)| {
                *seconds != clock.remaining_seconds || *running != clock.running
            })
        {
            self.clock_sample = Some((clock.remaining_seconds, clock.running, Instant::now()));
        }
        let remaining = self
            .clock_sample
            .as_ref()
            .map(|(seconds, running, at)| {
                seconds.saturating_sub(if *running { at.elapsed().as_secs() } else { 0 })
            })
            .unwrap_or(0);
        ui.label(format!(
            "Room countdown {:02}:{:02}:{:02} | {}",
            remaining / 3600,
            (remaining / 60) % 60,
            remaining % 60,
            if clock.running {
                "running"
            } else {
                "paused / ended"
            }
        ));
        if host {
            ui.horizontal_wrapped(|ui| {
                if self.minutes == 0 {
                    self.minutes = 60;
                }
                ui.add(
                    egui::DragValue::new(&mut self.minutes)
                        .range(1..=10080)
                        .suffix(" min"),
                );
                if ui.small_button("Start / reset").clicked() {
                    actions.push(Action::Countdown {
                        seconds: Some(self.minutes * 60),
                        running: true,
                    });
                }
                if ui.small_button("Pause").clicked() {
                    actions.push(Action::Countdown {
                        seconds: None,
                        running: false,
                    });
                }
                if ui.small_button("Resume").clicked() {
                    actions.push(Action::Countdown {
                        seconds: None,
                        running: true,
                    });
                }
            });
        }
        egui::CollapsingHeader::new("Room milestones").show(ui, |ui| {
            for milestone in &snapshot.shared.milestones {
                let mut done = milestone.done;
                if ui
                    .add_enabled(
                        host,
                        egui::Checkbox::new(
                            &mut done,
                            format!("{}: {}", milestone.id, milestone.title),
                        ),
                    )
                    .changed()
                {
                    actions.push(Action::MilestoneDone {
                        id: milestone.id.clone(),
                        done,
                    });
                }
                if !milestone.done
                    && let Some(deadline) = milestone.due_at_epoch_seconds
                {
                    ui.small(if deadline <= snapshot.server_time_epoch_seconds {
                        "Overdue".into()
                    } else {
                        format!(
                            "Due in {} min",
                            (deadline - snapshot.server_time_epoch_seconds) / 60
                        )
                    });
                }
            }
            if host {
                ui.label("New shared milestone");
                ui.add(egui::TextEdit::singleline(&mut self.milestone_title).char_limit(200));
                due(ui, "network-milestone-due", &mut self.milestone_due);
                if ui
                    .add_enabled(
                        !self.milestone_title.trim().is_empty()
                            && snapshot.shared.milestones.len() < 20,
                        egui::Button::new("Create milestone"),
                    )
                    .clicked()
                {
                    actions.push(Action::CreateMilestone {
                        title: self.milestone_title.clone(),
                        due_hours: self.milestone_due,
                    });
                    self.milestone_title.clear();
                }
            }
        });
        match self.tab {
            0 => self.chat(ui, snapshot, &mut actions),
            1 => self.tasks(ui, snapshot, host, &mut actions),
            _ => {
                ui.label("Shared activity excludes message bodies and private-message metadata.");
                for event in snapshot.shared.activity.iter().rev() {
                    ui.label(format!(
                        "{}: {}",
                        name(snapshot, &event.actor_id),
                        event.detail
                    ));
                }
            }
        }
        actions
    }
    fn chat(&mut self, ui: &mut egui::Ui, snapshot: &Snapshot, actions: &mut Vec<Action>) {
        let previous = self.recipient.clone();
        egui::ComboBox::from_id_salt("network-conversation")
            .selected_text(
                self.recipient
                    .as_ref()
                    .map(|id| format!("Private: {}", name(snapshot, id)))
                    .unwrap_or("Public Room chat".into()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.recipient, None, "Public Room chat");
                for member in snapshot.members.iter().filter(|m| m.id != snapshot.you) {
                    ui.selectable_value(
                        &mut self.recipient,
                        Some(member.id.clone()),
                        format!("Private: {}", name(snapshot, &member.id)),
                    );
                }
            });
        if self.recipient != previous {
            self.reply = None;
        }
        ui.label(if self.recipient.is_some() {
            "Visible only to you and the selected recipient."
        } else {
            "Visible to all joined members, including recent history on join."
        });
        ui.label("Search conversation");
        ui.add(egui::TextEdit::singleline(&mut self.query).char_limit(200));
        ui.checkbox(&mut self.decisions_only, "Decisions only");
        let query = self.query.to_lowercase();
        egui::ScrollArea::vertical()
            .id_salt("network-message-history")
            .max_height(180.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for event in &snapshot.events {
                    let channel = match &self.recipient {
                        None => event.recipient.is_none(),
                        Some(id) => {
                            (event.actor == snapshot.you && event.recipient.as_ref() == Some(id))
                                || (event.actor == *id
                                    && event.recipient.as_deref() == Some(&snapshot.you))
                        }
                    };
                    if !channel
                        || (self.decisions_only && !event.decision)
                        || !event.text.to_lowercase().contains(&query)
                    {
                        continue;
                    }
                    ui.label(format!(
                        "{}{}{}: {}",
                        name(snapshot, &event.actor),
                        if event.decision { " [decision]" } else { "" },
                        event
                            .task
                            .as_ref()
                            .map(|t| format!(" [{t}]"))
                            .unwrap_or_default(),
                        event.text
                    ));
                    if let Some(parent) = event.parent {
                        ui.small(format!("Reply to #{parent}"));
                    }
                    if ui
                        .small_button(format!("Reply #{}", event.sequence))
                        .clicked()
                    {
                        self.reply = Some(event.sequence);
                    }
                }
            });
        egui::ComboBox::from_id_salt("network-message-task")
            .selected_text(self.task_link.clone().unwrap_or("No task link".into()))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.task_link, None, "No task link");
                for task in &snapshot.shared.tasks {
                    ui.selectable_value(
                        &mut self.task_link,
                        Some(task.id.clone()),
                        format!("{}: {}", task.id, task.title),
                    );
                }
            });
        ui.checkbox(&mut self.decision, "Record as decision");
        if let Some(parent) = self.reply {
            ui.horizontal(|ui| {
                ui.label(format!("Replying to #{parent}"));
                if ui.small_button("Cancel reply").clicked() {
                    self.reply = None;
                }
            });
        }
        ui.add(
            egui::TextEdit::multiline(&mut self.draft)
                .desired_rows(2)
                .char_limit(2000),
        );
        if ui
            .add_enabled(
                !self.draft.trim().is_empty() && self.failed_drafts.len() < 16,
                egui::Button::new("Send message to Room"),
            )
            .clicked()
        {
            actions.push(Action::Message {
                text: self.draft.clone(),
                recipient: self.recipient.clone(),
                task: self.task_link.clone(),
                decision: self.decision,
                parent: self.reply,
            });
            self.draft.clear();
            self.decision = false;
            self.reply = None;
        }
    }
    fn tasks(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        host: bool,
        actions: &mut Vec<Action>,
    ) {
        egui::CollapsingHeader::new("Create shared task").show(ui, |ui| {
            ui.label("Title");
            ui.add(egui::TextEdit::singleline(&mut self.title).char_limit(200));
            if self.owner.is_empty() {
                self.owner = snapshot.you.clone();
            }
            egui::ComboBox::from_id_salt("network-task-owner")
                .selected_text(name(snapshot, &self.owner))
                .show_ui(ui, |ui| {
                    for member in &snapshot.members {
                        ui.selectable_value(
                            &mut self.owner,
                            member.id.clone(),
                            name(snapshot, &member.id),
                        );
                    }
                });
            ui.label("Acceptance criteria (up to 8, one per line)");
            ui.add(
                egui::TextEdit::multiline(&mut self.criteria)
                    .desired_rows(2)
                    .char_limit(1608),
            );
            ui.label("Dependencies (shared task IDs, comma-separated)");
            ui.add(egui::TextEdit::singleline(&mut self.dependencies).char_limit(256));
            priority(ui, "network-create-priority", &mut self.priority);
            due(ui, "network-create-due", &mut self.due);
            if ui
                .add_enabled(
                    !self.title.trim().is_empty() && self.failed_drafts.len() < 16,
                    egui::Button::new("Create shared task"),
                )
                .clicked()
            {
                actions.push(Action::CreateTask {
                    title: self.title.clone(),
                    owner: self.owner.clone(),
                    criteria: self
                        .criteria
                        .lines()
                        .map(str::trim)
                        .filter(|c| !c.is_empty())
                        .map(str::to_string)
                        .collect(),
                    dependencies: self
                        .dependencies
                        .split(',')
                        .map(str::trim)
                        .filter(|d| !d.is_empty())
                        .map(str::to_string)
                        .collect(),
                    priority: self.priority,
                    due_hours: self.due,
                });
                self.title.clear();
                self.criteria.clear();
                self.dependencies.clear();
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("network-tasks")
            .max_height(140.0)
            .show(ui, |ui| {
                for task in &snapshot.shared.tasks {
                    if ui
                        .selectable_label(
                            self.selected.as_ref() == Some(&task.id),
                            format!(
                                "{} | {} | {} | {}",
                                task.id, task.title, task.owner_name, task.status
                            ),
                        )
                        .clicked()
                    {
                        self.selected = Some(task.id.clone());
                        self.checked = task.completed_criteria.iter().cloned().collect();
                    }
                }
            });
        let Some(task) = snapshot
            .shared
            .tasks
            .iter()
            .find(|t| Some(&t.id) == self.selected.as_ref())
        else {
            return;
        };
        let writer = host || task.owner_id == snapshot.you;
        ui.label(format!("{}: {} | {:?}", task.id, task.title, task.priority));
        if let Some(deadline) = task.due_at_epoch_seconds {
            let now = snapshot.server_time_epoch_seconds;
            ui.label(if deadline <= now {
                "Overdue".into()
            } else {
                format!("Due in {} min (host deadline)", (deadline - now) / 60)
            });
        }
        for criterion in &task.acceptance_criteria {
            let mut checked = self.checked.contains(criterion);
            if ui
                .add_enabled(writer, egui::Checkbox::new(&mut checked, criterion))
                .changed()
            {
                if checked {
                    self.checked.insert(criterion.clone());
                } else {
                    self.checked.remove(criterion);
                }
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    writer && task.status == TaskStatus::Open,
                    egui::Button::new("Start task"),
                )
                .clicked()
            {
                actions.push(Action::Transition {
                    task: task.id.clone(),
                    status: TaskStatus::InProgress,
                    completed: Vec::new(),
                });
            }
            if ui
                .add_enabled(
                    writer && task.status != TaskStatus::Done,
                    egui::Button::new("Complete task"),
                )
                .clicked()
            {
                actions.push(Action::Transition {
                    task: task.id.clone(),
                    status: TaskStatus::Done,
                    completed: task
                        .acceptance_criteria
                        .iter()
                        .filter(|c| self.checked.contains(*c))
                        .cloned()
                        .collect(),
                });
            }
            if ui
                .add_enabled(
                    host && task.status != TaskStatus::Open,
                    egui::Button::new("Reopen as host"),
                )
                .clicked()
            {
                actions.push(Action::Transition {
                    task: task.id.clone(),
                    status: TaskStatus::Open,
                    completed: Vec::new(),
                });
            }
        });
        if writer && task.status != TaskStatus::Done {
            let mut selected = task.priority;
            priority(ui, "network-edit-priority", &mut selected);
            if selected != task.priority {
                actions.push(Action::Priority {
                    task: task.id.clone(),
                    priority: selected,
                });
            }
            due(ui, "network-edit-due", &mut self.due);
            if ui.button("Set / clear due time").clicked() {
                actions.push(Action::Due {
                    task: task.id.clone(),
                    hours: self.due,
                });
            }
        }
        if writer {
            let mut evidence = self.evidence.remove(&task.id).unwrap_or_default();
            ui.label("Verification evidence (up to 500 characters)");
            ui.add(
                egui::TextEdit::multiline(&mut evidence)
                    .desired_rows(2)
                    .char_limit(500),
            );
            if ui
                .add_enabled(
                    !evidence.trim().is_empty() && self.failed_drafts.len() < 16,
                    egui::Button::new("Attach evidence"),
                )
                .clicked()
            {
                actions.push(Action::Evidence {
                    task: task.id.clone(),
                    kind: EvidenceKind::Check,
                    text: evidence.clone(),
                });
                evidence.clear();
            }
            self.evidence.insert(task.id.clone(), evidence);
        }
        if task.owner_id == snapshot.you && task.status != TaskStatus::Done {
            egui::ComboBox::from_id_salt("network-handoff-target")
                .selected_text(name(snapshot, &self.target))
                .show_ui(ui, |ui| {
                    for member in snapshot.members.iter().filter(|m| m.id != task.owner_id) {
                        ui.selectable_value(
                            &mut self.target,
                            member.id.clone(),
                            name(snapshot, &member.id),
                        );
                    }
                });
            if ui
                .add_enabled(
                    !self.target.is_empty(),
                    egui::Button::new("Request handoff"),
                )
                .clicked()
            {
                actions.push(Action::RequestHandoff {
                    task: task.id.clone(),
                    target: self.target.clone(),
                });
            }
        }
        for handoff in snapshot
            .shared
            .handoffs
            .iter()
            .filter(|h| h.task_id == task.id)
        {
            ui.label(format!(
                "Handoff requested to {}",
                name(snapshot, &handoff.target_owner_id)
            ));
            if host && ui.button("Approve handoff as host").clicked() {
                actions.push(Action::ApproveHandoff {
                    task: task.id.clone(),
                });
            }
        }
        for evidence in snapshot
            .shared
            .evidence
            .iter()
            .filter(|e| e.task_id == task.id)
        {
            ui.label(format!(
                "Evidence by {}: {}",
                name(snapshot, &evidence.actor_id),
                evidence.summary
            ));
        }
    }
}
fn name(snapshot: &Snapshot, id: &str) -> String {
    snapshot
        .members
        .iter()
        .find(|m| m.id == id)
        .map(|m| format!("{} ({})", m.name, id.chars().take(8).collect::<String>()))
        .unwrap_or_else(|| {
            if id.is_empty() {
                "Choose member".into()
            } else {
                format!(
                    "Host / former member ({})",
                    id.chars().take(8).collect::<String>()
                )
            }
        })
}
fn priority(ui: &mut egui::Ui, id: &str, value: &mut TaskPriority) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(format!("Priority: {value:?}"))
        .show_ui(ui, |ui| {
            for p in [
                TaskPriority::Normal,
                TaskPriority::High,
                TaskPriority::Critical,
            ] {
                ui.selectable_value(value, p, format!("{p:?}"));
            }
        });
}
fn due(ui: &mut egui::Ui, id: &str, value: &mut Option<u64>) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(
            value
                .map(|h| format!("Due in {h} hours"))
                .unwrap_or("No due time".into()),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, "No due time");
            for hours in [1, 4, 24, 72] {
                ui.selectable_value(value, Some(hours), format!("Due in {hours} hours"));
            }
        });
}

#[cfg(test)]
mod draft_tests {
    use super::*;
    #[test]
    fn a_rejected_message_does_not_replace_a_newer_draft() {
        let mut state = Workflow {
            draft: "newer text".into(),
            ..Default::default()
        };
        state.restore(Action::message("rejected text"));
        assert_eq!(state.draft, "newer text");
        assert_eq!(state.failed_drafts.len(), 1);
        state.draft.clear();
        let action = state.failed_drafts.remove(0);
        state.restore(action);
        assert_eq!(state.draft, "rejected text");
    }
    #[test]
    fn evidence_drafts_keep_their_task_scope_after_rejection() {
        let mut state = Workflow::default();
        state.evidence.insert("T1".into(), "new T1 draft".into());
        state.evidence.insert("T2".into(), "T2 draft".into());
        state.restore(Action::Evidence {
            task: "T1".into(),
            kind: EvidenceKind::Check,
            text: "rejected T1".into(),
        });
        assert_eq!(state.failed_drafts.len(), 1);
        assert_eq!(state.evidence["T1"], "new T1 draft");
        assert_eq!(state.evidence["T2"], "T2 draft");
    }
}
