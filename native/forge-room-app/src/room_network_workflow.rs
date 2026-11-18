//! Forms for shared Room intents. Authorization and visibility belong to the server.
use eframe::egui;
use forge_room_core::{
    ActivityEvent, EvidenceKind, TaskPriority, TaskStatus,
    transport::{Action, Snapshot},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

#[derive(Default)]
pub struct Workflow {
    pub readiness: crate::room_readiness::Panel,
    failed_drafts: Vec<Action>,
    milestone_title: String,
    milestone_due: Option<u64>,
    tab: usize,
    draft: String,
    attach_code_reference: bool,
    code_reference_path: String,
    code_reference_line: String,
    recipient: Option<String>,
    task_link: Option<String>,
    decision: bool,
    mention_target: Option<String>,
    mentions_only: bool,
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
    activity_seen: Option<u64>,
    activity_query: String,
    activity_actor: Option<String>,
    activity_task: Option<String>,
    remind_minutes: u64,
    reminded: HashSet<String>,
    reminder_order: VecDeque<String>,
    reminder_notice: Vec<String>,
    shared_focus: Option<String>,
    focus_initialized: bool,
    following: Option<String>,
    location_path: String,
    location_line: String,
    location_initialized: bool,
    following_location: Option<String>,
    requested_open: Option<forge_room_core::transport::CodeReference>,
}
impl Workflow {
    pub fn take_open_location(&mut self) -> Option<forge_room_core::transport::CodeReference> {
        self.requested_open.take()
    }
    pub fn observe_activity(&mut self, snapshot: &Snapshot) -> usize {
        let latest = snapshot
            .shared
            .activity
            .iter()
            .filter_map(|event| event.id.strip_prefix("network-")?.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        let baseline = *self.activity_seen.get_or_insert(latest);
        unread_activity(&snapshot.shared.activity, baseline)
    }
    pub fn mark_activity_seen(&mut self, snapshot: &Snapshot) {
        self.activity_seen = Some(
            snapshot
                .shared
                .activity
                .iter()
                .filter_map(|event| event.id.strip_prefix("network-")?.parse::<u64>().ok())
                .max()
                .unwrap_or(0),
        );
    }
    fn slot_free(&self, action: &Action) -> bool {
        match action {
            Action::Message { .. }
            | Action::ContextMessage { .. }
            | Action::MentionMessage { .. } => self.draft.is_empty(),
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
            Action::ContextMessage {
                text,
                recipient,
                task,
                decision,
                parent,
                path,
                line,
            } => {
                self.draft = text;
                self.recipient = recipient;
                self.task_link = task;
                self.decision = decision;
                self.reply = parent;
                self.attach_code_reference = true;
                self.code_reference_path = path;
                self.code_reference_line = line.to_string();
            }
            Action::MentionMessage {
                text,
                mentions,
                task,
                decision,
                parent,
            } => {
                self.draft = text;
                self.mention_target = mentions.into_iter().next();
                self.recipient = None;
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
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        host: bool,
        root: Option<&std::path::Path>,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        if !self.failed_drafts.is_empty() {
            egui::CollapsingHeader::new(format!("Rejected drafts ({})", self.failed_drafts.len()))
                .default_open(true)
                .show(ui, |ui| {
                    let mut restore = None;
                    let mut discard = None;
                    for (index, action) in self.failed_drafts.iter().enumerate() {
                        ui.label(match action {
                            Action::Message { text, .. }
                            | Action::ContextMessage { text, .. }
                            | Action::MentionMessage { text, .. } => {
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
                            Action::Message { .. }
                            | Action::ContextMessage { .. }
                            | Action::MentionMessage { .. } => self.tab = 0,
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

        let unread = self.observe_activity(snapshot);
        let mention_count = snapshot
            .events
            .iter()
            .filter(|event| event.mentions.contains(&snapshot.you))
            .count();
        ui.horizontal_wrapped(|ui| {
            for (index, label) in [
                if mention_count > 0 {
                    format!("Chat · {mention_count} mentions")
                } else {
                    "Chat".to_owned()
                },
                "Tasks".to_owned(),
                "Activity".to_owned(),
                "Readiness".to_owned(),
                "Follow".to_owned(),
            ]
            .iter()
            .enumerate()
            {
                let title = if label == "Activity" && unread > 0 {
                    format!("Activity · {unread} new")
                } else {
                    (*label).to_owned()
                };
                ui.selectable_value(&mut self.tab, index, title);
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
        let mut reminders = Vec::new();
        if self.remind_minutes == 0 {
            self.remind_minutes = 15;
        }
        ui.horizontal(|ui| {
            ui.label("Remind me");
            egui::ComboBox::from_id_salt("network-room-reminder-window")
                .selected_text(format!("{} min before", self.remind_minutes))
                .show_ui(ui, |ui| {
                    for minutes in [5, 15, 60] {
                        ui.selectable_value(
                            &mut self.remind_minutes,
                            minutes,
                            format!("{minutes} min before"),
                        );
                    }
                });
        });
        let reminder_seconds = self.remind_minutes.saturating_mul(60);
        let due_notices = reminder_items(snapshot, reminder_seconds, remaining, clock.running);
        if !clock.running || remaining > reminder_seconds {
            self.reminded.remove("countdown");
            self.reminder_order.retain(|key| key != "countdown");
        }
        for (key, message) in due_notices {
            if self.remember_reminder(key) {
                reminders.push(message);
            }
        }
        self.reminder_notice.extend(reminders);
        if self.reminder_notice.len() > 16 {
            let excess = self.reminder_notice.len() - 16;
            self.reminder_notice.drain(..excess);
        }
        if !self.reminder_notice.is_empty() {
            ui.group(|ui| {
                ui.label("Room reminders (visible while this panel is open)");
                for notice in &self.reminder_notice {
                    ui.label(format!("• {notice}"));
                }
                if ui.small_button("Dismiss reminders").clicked() {
                    self.reminder_notice.clear();
                }
            });
        }
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
            4 => self.follow(ui, snapshot, &mut actions),
            3 => self
                .readiness
                .show(ui, snapshot, root, remaining, &mut actions),
            0 => self.chat(ui, snapshot, &mut actions),
            1 => self.tasks(ui, snapshot, host, &mut actions),
            _ => {
                ui.label("Shared activity excludes message bodies and private-message metadata.");
                ui.label(if unread == 0 {
                    "You are caught up with shared activity."
                } else {
                    "New shared activity since you last opened this feed."
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.activity_query)
                        .hint_text("Search shared activity")
                        .char_limit(200),
                );
                ui.horizontal_wrapped(|ui| {
                    egui::ComboBox::from_id_salt("network-activity-actor")
                        .selected_text(
                            self.activity_actor
                                .as_ref()
                                .map(|id| name(snapshot, id))
                                .unwrap_or("All members".into()),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.activity_actor, None, "All members");
                            for member in &snapshot.members {
                                ui.selectable_value(
                                    &mut self.activity_actor,
                                    Some(member.id.clone()),
                                    &member.name,
                                );
                            }
                        });
                    egui::ComboBox::from_id_salt("network-activity-task")
                        .selected_text(self.activity_task.as_deref().unwrap_or("All tasks"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.activity_task, None, "All tasks");
                            for task in &snapshot.shared.tasks {
                                ui.selectable_value(
                                    &mut self.activity_task,
                                    Some(task.id.clone()),
                                    format!("{}: {}", task.id, task.title),
                                );
                            }
                        });
                    if (!self.activity_query.is_empty()
                        || self.activity_actor.is_some()
                        || self.activity_task.is_some())
                        && ui.small_button("Clear activity filters").clicked()
                    {
                        self.activity_query.clear();
                        self.activity_actor = None;
                        self.activity_task = None;
                    }
                });
                let query = self.activity_query.clone();
                let actor = self.activity_actor.clone();
                let task_filter = self.activity_task.clone();
                let matches: Vec<_> = snapshot
                    .shared
                    .activity
                    .iter()
                    .rev()
                    .filter(|event| {
                        activity_matches(
                            snapshot,
                            event,
                            &query,
                            actor.as_deref(),
                            task_filter.as_deref(),
                        )
                    })
                    .collect();
                ui.small(format!(
                    "Showing {} of {} retained events",
                    matches.len(),
                    snapshot.shared.activity.len()
                ));
                if matches.is_empty() {
                    ui.label("No shared activity matches these filters.");
                }
                for event in matches {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "{}: {}",
                            name(snapshot, &event.actor_id),
                            event.detail
                        ));
                        if let Some(task_id) = event.detail.split_whitespace().find(|part| {
                            part.len() <= 16
                                && part.starts_with('T')
                                && part[1..].chars().all(|ch| ch.is_ascii_digit())
                        }) && snapshot.shared.tasks.iter().any(|task| task.id == task_id)
                            && ui.small_button(format!("Open {task_id}")).clicked()
                        {
                            self.selected = Some(task_id.to_owned());
                            self.tab = 1;
                        }
                    });
                }
                self.mark_activity_seen(snapshot);
            }
        }
        actions
    }
    fn follow(&mut self, ui: &mut egui::Ui, snapshot: &Snapshot, actions: &mut Vec<Action>) {
        ui.label("Share only a task ID as your Room focus. No file, editor location or unsaved content is sent.");
        if !self.focus_initialized {
            self.shared_focus = snapshot
                .shared
                .task_focus
                .iter()
                .find(|focus| focus.actor == snapshot.you)
                .map(|focus| focus.task.clone());
            self.focus_initialized = true;
        }
        if self.shared_focus.as_ref().is_some_and(|task_id| {
            !snapshot
                .shared
                .tasks
                .iter()
                .any(|task| &task.id == task_id && task.status != TaskStatus::Done)
        }) {
            self.shared_focus = None;
            actions.push(Action::ShareFocus { task: None });
        }
        let mut selected = self.shared_focus.clone();
        egui::ComboBox::from_id_salt("network-share-focus")
            .selected_text(selected.as_deref().unwrap_or("Do not share task focus"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, None, "Do not share task focus");
                for task in snapshot
                    .shared
                    .tasks
                    .iter()
                    .filter(|task| task.status != TaskStatus::Done)
                {
                    ui.selectable_value(
                        &mut selected,
                        Some(task.id.clone()),
                        format!("{}: {}", task.id, task.title),
                    );
                }
            });
        if selected != self.shared_focus {
            actions.push(Action::ShareFocus {
                task: selected.clone(),
            });
            self.shared_focus = selected;
        }
        ui.separator();
        ui.label("Members who chose to share a task focus");
        let focuses = &snapshot.shared.task_focus;
        let mut next_follow = self.following.clone();
        if focuses.is_empty() {
            ui.label("No one is sharing a task focus.");
            next_follow = None;
        }
        for focus in focuses {
            let member_name = name(snapshot, &focus.actor);
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{member_name} is focused on {}", focus.task));
                if ui
                    .selectable_label(self.following.as_deref() == Some(&focus.actor), "Follow")
                    .clicked()
                {
                    next_follow = Some(focus.actor.clone());
                }
                if self.following.as_deref() == Some(&focus.actor)
                    && ui.small_button("Stop following").clicked()
                {
                    next_follow = None;
                }
            });
        }
        self.following = next_follow;
        if let Some(actor) = &self.following {
            if let Some(focus) = focuses.iter().find(|focus| &focus.actor == actor)
                && let Some(task) = snapshot
                    .shared
                    .tasks
                    .iter()
                    .find(|task| task.id == focus.task)
            {
                ui.group(|ui| {
                    ui.heading(format!("Following {} · {}", name(snapshot, actor), task.id));
                    ui.label(&task.title);
                    ui.label(format!("{} · owner {} · {:?}", task.status, task.owner_name, task.priority));
                    if let Some(deadline) = task.due_at_epoch_seconds {
                        ui.label(if deadline <= snapshot.server_time_epoch_seconds { "Overdue".to_owned() } else { format!("Due in {} min", (deadline - snapshot.server_time_epoch_seconds) / 60) });
                    }
                    ui.label("Read-only task summary. Your task selection and editor navigation are unchanged.");
                    for evidence in snapshot.shared.evidence.iter().filter(|evidence| evidence.task_id == task.id).rev().take(5) {
                        ui.small(format!("{} · {}", name(snapshot, &evidence.actor_id), evidence.summary));
                    }
                });
            } else {
                self.following = None;
            }
        }
        ui.separator();
        ui.heading("Code location sharing");
        ui.label("Share a workspace-relative path and line with every joined member. Confirm it is a saved file; the Room does not check your disk. Source text, unsaved content, cursor selection and edits stay private.");
        if !self.location_initialized {
            if let Some(location) = snapshot
                .shared
                .code_locations
                .iter()
                .find(|location| location.actor == snapshot.you)
            {
                self.location_path = location.path.clone();
                self.location_line = location.line.to_string();
            }
            self.location_initialized = true;
        }
        ui.horizontal(|ui| {
            ui.label("Workspace-relative saved path");
            ui.add(egui::TextEdit::singleline(&mut self.location_path).char_limit(512));
        });
        ui.horizontal(|ui| {
            ui.label("Line");
            ui.add(egui::TextEdit::singleline(&mut self.location_line).char_limit(10));
            if ui.button("Share code location").clicked() {
                match self.location_line.trim().parse::<u32>() {
                    Ok(line) if !self.location_path.trim().is_empty() => {
                        actions.push(Action::ShareLocation {
                            path: Some(self.location_path.trim().to_owned()),
                            line: Some(line),
                        });
                    }
                    _ => {
                        ui.label("Enter a path and positive line number.");
                    }
                };
            }
            if ui.button("Stop sharing code location").clicked() {
                actions.push(Action::ShareLocation {
                    path: None,
                    line: None,
                });
            }
        });
        let locations = &snapshot.shared.code_locations;
        let available_actors: HashSet<_> =
            locations.iter().map(|item| item.actor.as_str()).collect();
        if self
            .following_location
            .as_deref()
            .is_some_and(|actor| !available_actors.contains(actor))
        {
            self.following_location = None;
        }
        ui.label("Members sharing a code location");
        if locations.is_empty() {
            ui.label("No one is sharing a code location.");
        }
        for location in locations {
            let member_name = name(snapshot, &location.actor);
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "{member_name} · {}:{}",
                    location.path, location.line
                ));
                if ui
                    .selectable_label(
                        self.following_location.as_deref() == Some(&location.actor),
                        "Follow location",
                    )
                    .clicked()
                {
                    self.following_location = Some(location.actor.clone());
                }
                if self.following_location.as_deref() == Some(&location.actor)
                    && ui.small_button("Stop following location").clicked()
                {
                    self.following_location = None;
                }
            });
        }
        if let Some(actor) = self.following_location.as_deref()
            && let Some(location) = locations.iter().find(|location| location.actor == actor)
        {
            ui.group(|ui| {
                ui.heading(format!("Following {}", name(snapshot, actor)));
                ui.monospace(format!("{}:{}", location.path, location.line));
                ui.label(
                    "Read-only location preview. Opening checks this path in your own workspace.",
                );
                if ui.button("Open in my workspace").clicked() {
                    self.requested_open = Some(forge_room_core::transport::CodeReference {
                        path: location.path.clone(),
                        line: location.line,
                    });
                }
            });
        }
    }
    fn remember_reminder(&mut self, key: String) -> bool {
        if !self.reminded.insert(key.clone()) {
            return false;
        }
        self.reminder_order.push_back(key);
        if self.reminder_order.len() > 256
            && let Some(expired) = self.reminder_order.pop_front()
        {
            self.reminded.remove(&expired);
        }
        true
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
            self.mention_target = None;
        }
        if self
            .mention_target
            .as_ref()
            .is_some_and(|id| !snapshot.members.iter().any(|member| &member.id == id))
        {
            self.mention_target = None;
        }
        if self.mention_target.is_some() {
            self.attach_code_reference = false;
        }
        ui.label(if self.recipient.is_some() {
            "Visible only to you and the selected recipient."
        } else {
            "Visible to all joined members, including recent history on join."
        });
        ui.label("Search conversation");
        ui.add(egui::TextEdit::singleline(&mut self.query).char_limit(200));
        ui.checkbox(&mut self.decisions_only, "Decisions only");
        ui.checkbox(&mut self.mentions_only, "Only messages mentioning me");
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
                        || (self.mentions_only && !event.mentions.contains(&snapshot.you))
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
                    if let Some(reference) = &event.code_reference {
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(format!(
                                "Code context · {}:{}",
                                reference.path, reference.line
                            ));
                            if ui.small_button("Open in my workspace").clicked() {
                                self.requested_open = Some(reference.clone());
                            }
                        });
                    }
                    if !event.mentions.is_empty() {
                        ui.small(format!(
                            "Mentioned: {}{}",
                            event
                                .mentions
                                .iter()
                                .map(|id| format!("@{}", name(snapshot, id)))
                                .collect::<Vec<_>>()
                                .join(", "),
                            if event.mentions.contains(&snapshot.you) {
                                " · You were mentioned"
                            } else {
                                ""
                            }
                        ));
                    }
                    if let Some(parent) = event.parent {
                        ui.small(format!("Reply to #{parent}"));
                    }
                    if ui
                        .small_button(format!("Reply #{}", event.sequence))
                        .clicked()
                    {
                        self.reply = Some(event.sequence);
                    }
                    ui.horizontal_wrapped(|ui| {
                        for emoji in ["👍", "✅", "👀", "🎉", "❤️"] {
                            let matching: Vec<_> = snapshot
                                .reactions
                                .iter()
                                .filter(|reaction| {
                                    reaction.sequence == event.sequence && reaction.emoji == emoji
                                })
                                .collect();
                            let mine = matching
                                .iter()
                                .any(|reaction| reaction.actor == snapshot.you);
                            let label = if matching.is_empty() {
                                emoji.to_owned()
                            } else {
                                format!("{emoji} {}", matching.len())
                            };
                            if ui.small_button(label).clicked() {
                                actions.push(Action::React {
                                    sequence: event.sequence,
                                    emoji: emoji.into(),
                                    active: !mine,
                                });
                            }
                        }
                    });
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
        if self.recipient.is_none() {
            egui::ComboBox::from_id_salt("network-message-mention")
                .selected_text(
                    self.mention_target
                        .as_ref()
                        .map(|id| format!("Mention {}", name(snapshot, id)))
                        .unwrap_or("No mention".into()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.mention_target, None, "No mention");
                    for member in snapshot
                        .members
                        .iter()
                        .filter(|member| member.id != snapshot.you)
                    {
                        ui.selectable_value(
                            &mut self.mention_target,
                            Some(member.id.clone()),
                            format!("Mention {}", member.name),
                        );
                    }
                });
        }
        ui.add_enabled_ui(self.mention_target.is_none(), |ui| {
            ui.checkbox(
                &mut self.attach_code_reference,
                "Attach a code path and line",
            );
        });
        if self.attach_code_reference {
            ui.label("This path and line will be included with the message for its selected audience. No code is sent.");
            ui.horizontal(|ui| {
                ui.label("Workspace-relative path");
                ui.add(egui::TextEdit::singleline(&mut self.code_reference_path).char_limit(512));
            });
            ui.horizontal(|ui| {
                ui.label("Line");
                ui.add(egui::TextEdit::singleline(&mut self.code_reference_line).char_limit(10));
            });
        }
        let parsed_reference_line = self.code_reference_line.trim().parse::<u32>().ok();
        let valid_code_reference = !self.code_reference_path.trim().is_empty()
            && parsed_reference_line.is_some_and(|line| (1..=10_000_000).contains(&line));
        if self.attach_code_reference && !valid_code_reference {
            ui.small("Enter a workspace-relative path and line from 1 to 10,000,000.");
        }
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
        let send_enabled = !self.draft.trim().is_empty()
            && self.failed_drafts.len() < 16
            && (!self.attach_code_reference || valid_code_reference);
        if ui
            .add_enabled(send_enabled, egui::Button::new("Send message to Room"))
            .clicked()
        {
            if self.recipient.is_none()
                && let Some(mention) = self.mention_target.clone()
            {
                actions.push(Action::MentionMessage {
                    text: self.draft.clone(),
                    mentions: vec![mention],
                    task: self.task_link.clone(),
                    decision: self.decision,
                    parent: self.reply,
                });
            } else if self.attach_code_reference {
                actions.push(Action::ContextMessage {
                    text: self.draft.clone(),
                    recipient: self.recipient.clone(),
                    task: self.task_link.clone(),
                    decision: self.decision,
                    parent: self.reply,
                    path: self.code_reference_path.trim().to_owned(),
                    line: parsed_reference_line.expect("validated above"),
                });
            } else {
                actions.push(Action::Message {
                    text: self.draft.clone(),
                    recipient: self.recipient.clone(),
                    task: self.task_link.clone(),
                    decision: self.decision,
                    parent: self.reply,
                });
            }
            self.draft.clear();
            self.decision = false;
            self.reply = None;
            self.mention_target = None;
            self.attach_code_reference = false;
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
fn unread_activity(activity: &[ActivityEvent], after: u64) -> usize {
    activity
        .iter()
        .filter(|event| {
            event
                .id
                .strip_prefix("network-")
                .and_then(|sequence| sequence.parse::<u64>().ok())
                .is_some_and(|sequence| sequence > after)
        })
        .count()
}

fn activity_matches(
    snapshot: &Snapshot,
    event: &ActivityEvent,
    query: &str,
    actor: Option<&str>,
    task: Option<&str>,
) -> bool {
    if actor.is_some_and(|actor| event.actor_id != actor) {
        return false;
    }
    if task.is_some_and(|task| {
        !event
            .detail
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|part| part == task)
    }) {
        return false;
    }
    let query = query.trim().to_lowercase();
    query.is_empty()
        || event.detail.to_lowercase().contains(&query)
        || name(snapshot, &event.actor_id)
            .to_lowercase()
            .contains(&query)
}

fn reminder_items(
    snapshot: &Snapshot,
    reminder_seconds: u64,
    remaining: u64,
    countdown_running: bool,
) -> Vec<(String, String)> {
    let mut reminders = Vec::new();
    if countdown_running && remaining <= reminder_seconds {
        reminders.push(("countdown".into(), "Room countdown is almost up".into()));
    }
    for task in &snapshot.shared.tasks {
        if task.status != TaskStatus::Done
            && let Some(deadline) = task.due_at_epoch_seconds
            && deadline
                <= snapshot
                    .server_time_epoch_seconds
                    .saturating_add(reminder_seconds)
        {
            reminders.push((
                format!("task:{}:{deadline}", task.id),
                format!("{} is due or overdue", task.title),
            ));
        }
    }
    for milestone in &snapshot.shared.milestones {
        if !milestone.done
            && let Some(deadline) = milestone.due_at_epoch_seconds
            && deadline
                <= snapshot
                    .server_time_epoch_seconds
                    .saturating_add(reminder_seconds)
        {
            reminders.push((
                format!("milestone:{}:{deadline}", milestone.id),
                format!("Milestone: {} is due or overdue", milestone.title),
            ));
        }
    }
    reminders
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
    #[test]
    fn activity_unread_count_is_per_viewer_cursor_and_ignores_bad_ids() {
        let activity = vec![
            ActivityEvent {
                id: "network-4".into(),
                actor_id: "a".repeat(64),
                detail: "started task T1".into(),
                recorded_at_epoch_seconds: 0,
            },
            ActivityEvent {
                id: "network-5".into(),
                actor_id: "b".repeat(64),
                detail: "completed task T1".into(),
                recorded_at_epoch_seconds: 0,
            },
            ActivityEvent {
                id: "invalid".into(),
                actor_id: "b".repeat(64),
                detail: "ignored".into(),
                recorded_at_epoch_seconds: 0,
            },
        ];
        assert_eq!(unread_activity(&activity, 4), 1);
        assert_eq!(unread_activity(&activity, 5), 0);
    }
    #[test]
    fn collapsed_header_cursor_counts_new_events_until_activity_is_opened() {
        let mut workflow = Workflow::default();
        let mut snapshot = Snapshot::default();
        snapshot.shared.activity.push(ActivityEvent {
            id: "network-4".into(),
            actor_id: "a".repeat(64),
            detail: "joined".into(),
            recorded_at_epoch_seconds: 0,
        });
        assert_eq!(workflow.observe_activity(&snapshot), 0);
        snapshot.shared.activity.push(ActivityEvent {
            id: "network-5".into(),
            actor_id: "b".repeat(64),
            detail: "started task T1".into(),
            recorded_at_epoch_seconds: 0,
        });
        assert_eq!(workflow.observe_activity(&snapshot), 1);
        workflow.mark_activity_seen(&snapshot);
        assert_eq!(workflow.observe_activity(&snapshot), 0);
    }
    #[test]
    fn activity_filters_match_exact_task_member_and_text_without_prefix_leaks() {
        let actor = "a".repeat(64);
        let event = ActivityEvent {
            id: "network-9".into(),
            actor_id: actor.clone(),
            detail: "started task T10".into(),
            recorded_at_epoch_seconds: 0,
        };
        let snapshot = Snapshot {
            members: vec![forge_room_core::transport::Member {
                id: actor.clone(),
                name: "Ada".into(),
                online: true,
            }],
            ..Snapshot::default()
        };
        assert!(activity_matches(
            &snapshot,
            &event,
            "started",
            Some(&actor),
            Some("T10")
        ));
        assert!(!activity_matches(&snapshot, &event, "", None, Some("T1")));
        assert!(!activity_matches(&snapshot, &event, "other", None, None));
        assert!(activity_matches(&snapshot, &event, "ada", None, None));
    }
    #[test]
    fn due_task_milestone_and_countdown_reminders_are_bounded_and_repeatable_by_key() {
        let mut snapshot = Snapshot {
            server_time_epoch_seconds: 10_000,
            ..Snapshot::default()
        };
        snapshot.shared.tasks.push(forge_room_core::Task {
            id: "T1".into(),
            title: "API task".into(),
            owner_id: "a".repeat(64),
            owner_name: "Ada".into(),
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            dependencies: vec![],
            acceptance_criteria: vec![],
            completed_criteria: vec![],
            worktree_branch: None,
            due_at_epoch_seconds: Some(10_300),
            updated_at_epoch_seconds: 0,
        });
        snapshot
            .shared
            .milestones
            .push(forge_room_core::transport::Milestone {
                id: "M1".into(),
                title: "Demo".into(),
                due_at_epoch_seconds: Some(10_200),
                done: false,
            });
        let reminders = reminder_items(&snapshot, 300, 250, true);
        assert_eq!(reminders.len(), 3);
        assert!(
            reminders
                .iter()
                .any(|(_, text)| text == "Room countdown is almost up")
        );
        assert!(reminders.iter().any(|(key, _)| key == "task:T1:10300"));
        assert!(reminders.iter().any(|(key, _)| key == "milestone:M1:10200"));
        let mut seen = HashSet::new();
        let once: Vec<_> = reminders
            .iter()
            .filter(|(key, _)| seen.insert(key.clone()))
            .collect();
        let again: Vec<_> = reminders
            .iter()
            .filter(|(key, _)| seen.insert(key.clone()))
            .collect();
        assert_eq!(once.len(), 3);
        assert!(again.is_empty());
        assert!(
            reminder_items(&snapshot, 300, 250, false)
                .iter()
                .all(|(key, _)| key != "countdown")
        );
    }
    #[test]
    fn reminder_deduplication_stays_bounded_and_keeps_recent_deadlines() {
        let mut state = Workflow::default();
        for index in 0..257 {
            assert!(state.remember_reminder(format!("deadline-{index}")));
        }
        assert_eq!(state.reminded.len(), 256);
        assert!(!state.remember_reminder("deadline-256".into()));
        assert!(state.remember_reminder("deadline-0".into()));
    }
}
