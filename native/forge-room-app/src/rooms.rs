//! The Room sidebar: people and coding agents, tasks with ownership, handoffs and evidence.
//! Every rule (who may do what, dependency gates, acceptance criteria) is enforced by
//! `forge-room-core`; this view only draws the state and forwards intents.

use crate::{icons, room_state, theme};
use eframe::egui::{self, Align2, Color32, FontId, Margin, Rect, RichText, Sense, Stroke};
use forge_room_core::{
    EvidenceKind, Participant, Role, RoomState, StateStore, Task, TaskStatus, now_epoch_seconds,
};
use std::collections::HashSet;
use std::path::Path;

enum Intent {
    CreateRoom,
    AddPerson {
        name: String,
        agent: bool,
        owner: String,
    },
    CreateTask {
        title: String,
        owner: String,
        criteria: Vec<String>,
        dependencies: Vec<String>,
    },
    Transition {
        task: String,
        status: TaskStatus,
        completed: Vec<String>,
    },
    RequestHandoff {
        task: String,
        target: String,
    },
    ApproveHandoff(String),
    Evidence {
        task: String,
        kind: EvidenceKind,
        text: String,
    },
    ActAs(String),
}

#[derive(Default)]
struct NewTask {
    open: bool,
    title: String,
    owner: String,
    criteria: String,
    dependencies: HashSet<String>,
}

#[derive(Default)]
struct NewPerson {
    open: bool,
    name: String,
    agent: bool,
    owner: String,
}

/// Which part of the room a dock tab shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Team,
    Agents,
    Tasks,
    Activity,
}

pub struct RoomView {
    state: Option<RoomState>,
    filter: Option<Role>,
    store: Option<StateStore>,
    workspace_name: String,
    acting_as: String,
    error: Option<String>,
    load_failed: bool,
    people_open: bool,
    tasks_open: bool,
    activity_open: bool,
    expanded: Option<String>,
    checked: HashSet<(String, String)>,
    new_task: NewTask,
    new_person: NewPerson,
    evidence_text: String,
    evidence_kind: EvidenceKind,
    handoff_target: String,
}

impl Default for RoomView {
    fn default() -> Self {
        Self {
            state: None,
            filter: None,
            store: None,
            workspace_name: String::new(),
            acting_as: String::new(),
            error: None,
            load_failed: false,
            people_open: true,
            tasks_open: true,
            activity_open: true,
            expanded: None,
            checked: HashSet::new(),
            new_task: NewTask::default(),
            new_person: NewPerson::default(),
            evidence_text: String::new(),
            evidence_kind: EvidenceKind::Check,
            handoff_target: String::new(),
        }
    }
}

const GREEN: Color32 = Color32::from_rgb(129, 199, 132);
const AMBER: Color32 = Color32::from_rgb(226, 192, 141);
const RED: Color32 = Color32::from_rgb(240, 113, 120);

impl RoomView {
    /// Switch to the room of `root`, reading it from disk when one exists.
    pub fn load_for(&mut self, root: Option<&Path>) {
        *self = Self::default();
        let Some(root) = root else {
            return;
        };
        self.workspace_name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".into());
        self.load_store(room_state::store_for(root));
    }

    fn load_store(&mut self, store: StateStore) {
        if store.path().exists() || store.backup_path().exists() {
            match store.load() {
                Ok(outcome) => {
                    if outcome.recovered_from_backup {
                        self.error =
                            Some("The room file was damaged; restored from backup.".into());
                    }
                    self.acting_as = outcome.state.host_id.clone();
                    self.state = Some(outcome.state);
                }
                Err(error) => {
                    self.error = Some(format!("Could not read this room: {error}"));
                    self.load_failed = true;
                }
            }
        }
        self.store = Some(store);
    }

    pub fn state(&self) -> Option<&RoomState> {
        self.state.as_ref()
    }

    pub fn acting_as(&self) -> &str {
        &self.acting_as
    }

    pub fn create_room(&mut self) -> bool {
        self.apply(Intent::CreateRoom);
        self.state.is_some()
    }

    /// Room name plus how many people and agents are in it, for the title and status bars.
    pub fn summary(&self) -> Option<(String, usize, usize)> {
        let state = self.state.as_ref()?;
        let agents = state
            .participants
            .iter()
            .filter(|person| person.role == forge_room_core::Role::Agent)
            .count();
        Some((
            self.workspace_name.clone(),
            state.participants.len() - agents,
            agents,
        ))
    }

    pub fn open_task_count(&self) -> usize {
        self.state
            .as_ref()
            .map(|state| {
                state
                    .tasks
                    .iter()
                    .filter(|task| task.status != TaskStatus::Done)
                    .count()
            })
            .unwrap_or(0)
    }

    fn mutate(
        &mut self,
        change: impl FnOnce(&mut RoomState, &str) -> Result<(), forge_room_core::DomainError>,
    ) -> bool {
        let Some(current) = &self.state else {
            return false;
        };
        let mut next = current.clone();
        if let Err(error) = change(&mut next, &self.acting_as) {
            self.error = Some(error.to_string());
            return false;
        }
        if let Some(store) = &self.store
            && let Err(error) = store.save(&next)
        {
            self.error = Some(format!("Could not save the room: {error}"));
            return false;
        }
        self.state = Some(next);
        self.error = None;
        true
    }

    fn apply(&mut self, intent: Intent) {
        match intent {
            Intent::CreateRoom => {
                if self.load_failed || self.state.is_some() {
                    return;
                }
                let user = room_state::local_user_name();
                let state = room_state::new_room(&self.workspace_name, &user);
                self.acting_as = state.host_id.clone();
                if let Some(store) = &self.store
                    && let Err(error) = store.save(&state)
                {
                    self.error = Some(format!("Could not save the room: {error}"));
                    return;
                }
                self.state = Some(state);
                self.error = None;
            }
            Intent::ActAs(id) => self.acting_as = id,
            Intent::AddPerson { name, agent, owner } => {
                let ok = self.mutate(|state, actor| {
                    let participant = Participant {
                        id: room_state::unique_participant_id(state, &name),
                        name: name.trim().to_string(),
                        role: if agent { Role::Agent } else { Role::Human },
                        owner_id: agent.then(|| owner.clone()),
                    };
                    state.add_participant(actor, participant)
                });
                if ok {
                    self.new_person = NewPerson::default();
                }
            }
            Intent::CreateTask {
                title,
                owner,
                criteria,
                dependencies,
            } => {
                let ok = self.mutate(|state, actor| {
                    let owner_name = state
                        .participants
                        .iter()
                        .find(|person| person.id == owner)
                        .map(|person| person.name.clone())
                        .unwrap_or_default();
                    state.create_task(
                        actor,
                        Task {
                            id: room_state::next_task_id(state),
                            title: title.trim().to_string(),
                            owner_id: owner.clone(),
                            owner_name,
                            status: TaskStatus::Open,
                            dependencies: dependencies.clone(),
                            acceptance_criteria: criteria.clone(),
                            completed_criteria: Vec::new(),
                            worktree_branch: None,
                            updated_at_epoch_seconds: now_epoch_seconds(),
                        },
                    )
                });
                if ok {
                    self.new_task = NewTask::default();
                }
            }
            Intent::Transition {
                task,
                status,
                completed,
            } => {
                self.mutate(|state, actor| state.transition_task(actor, &task, status, completed));
            }
            Intent::RequestHandoff { task, target } => {
                self.mutate(|state, actor| state.request_handoff(actor, &task, &target));
            }
            Intent::ApproveHandoff(task) => {
                self.mutate(|state, actor| state.approve_handoff(actor, &task));
            }
            Intent::Evidence { task, kind, text } => {
                if self
                    .mutate(|state, actor| state.record_evidence(actor, &task, kind, text.clone()))
                {
                    self.evidence_text.clear();
                }
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, has_workspace: bool, part: Section) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            ui.label(
                RichText::new(match part {
                    Section::Team => "TEAM",
                    Section::Agents => "AI AGENTS",
                    Section::Tasks => "TASKS",
                    Section::Activity => "ACTIVITY",
                })
                .size(11.0)
                .strong()
                .color(theme::text_dim()),
            );
        });
        ui.add_space(8.0);

        if !has_workspace {
            empty_state(
                ui,
                "No folder open",
                "Open a folder from the Explorer. Each project gets its own room.",
            );
            return;
        }
        let mut intents: Vec<Intent> = Vec::new();

        if let Some(error) = self.error.clone() {
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(240, 113, 120, 28))
                .stroke(Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(240, 113, 120, 120),
                ))
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(10, 8))
                .outer_margin(Margin::symmetric(12, 0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(error).color(RED));
                });
            ui.add_space(8.0);
        }

        let Some(state) = self.state.clone() else {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 14.0).max(140.0));
                    ui.label(
                        RichText::new("Start a room")
                            .size(17.0)
                            .strong()
                            .color(theme::text()),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "See who is doing what: people and coding agents, with clear \
                             ownership, handoffs and evidence. Stored privately on this \
                             computer.",
                        )
                        .color(theme::text_dim()),
                    );
                    ui.add_space(12.0);
                    let button = egui::Button::new(
                        RichText::new(format!("Create room as {}", room_state::local_user_name()))
                            .strong()
                            .color(theme::button_text()),
                    )
                    .fill(theme::button())
                    .corner_radius(6.0)
                    .min_size(egui::vec2(ui.available_width(), 32.0));
                    if ui.add_enabled(!self.load_failed, button).clicked() {
                        intents.push(Intent::CreateRoom);
                    }
                });
            });
            for intent in intents {
                self.apply(intent);
            }
            return;
        };

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                match part {
                    Section::Team | Section::Agents => {
                        self.filter = Some(if part == Section::Team {
                            Role::Human
                        } else {
                            Role::Agent
                        });
                        if part == Section::Team {
                            self.header(ui, &state, &mut intents);
                        }
                        self.people(ui, &state, &mut intents);
                    }
                    Section::Tasks => self.tasks(ui, &state, &mut intents),
                    Section::Activity => self.activity(ui, &state),
                }
                ui.add_space(16.0);
            });

        for intent in intents {
            self.apply(intent);
        }
    }

    fn header(&mut self, ui: &mut egui::Ui, state: &RoomState, intents: &mut Vec<Intent>) {
        egui::Frame::new()
            .inner_margin(Margin::symmetric(14, 0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(&state.room_id)
                        .size(16.0)
                        .strong()
                        .color(theme::text()),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Acting as").color(theme::text_dim()));
                    let mut selected = self.acting_as.clone();
                    let name = state
                        .participants
                        .iter()
                        .find(|person| person.id == selected)
                        .map(|person| person.name.clone())
                        .unwrap_or_default();
                    egui::ComboBox::from_id_salt("acting-as")
                        .selected_text(name)
                        .show_ui(ui, |ui| {
                            for person in &state.participants {
                                ui.selectable_value(&mut selected, person.id.clone(), &person.name);
                            }
                        });
                    if selected != self.acting_as {
                        intents.push(Intent::ActAs(selected));
                    }
                    if self.acting_as == state.host_id {
                        chip(
                            ui,
                            "Host",
                            theme::accent(),
                            theme::mix(theme::bg_sidebar(), theme::accent(), 0.2),
                        );
                    }
                });
                ui.add_space(10.0);
            });
    }

    fn people(&mut self, ui: &mut egui::Ui, state: &RoomState, intents: &mut Vec<Intent>) {
        let (_, add) = section(
            ui,
            match self.filter {
                Some(Role::Agent) => "AI AGENTS",
                _ => "HUMANS",
            },
            &mut self.people_open,
            state
                .participants
                .iter()
                .filter(|person| self.filter.as_ref().is_none_or(|role| &person.role == role))
                .count(),
            true,
        );
        if add && self.acting_as == state.host_id {
            self.new_person.open = !self.new_person.open;
            self.new_person.owner = state.host_id.clone();
        } else if add {
            self.error = Some("Only the room host can add a participant.".into());
        }
        if !self.people_open {
            return;
        }
        let shown = state
            .participants
            .iter()
            .filter(|person| self.filter.as_ref().is_none_or(|role| &person.role == role));
        if shown.clone().next().is_none() {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(
                    RichText::new(if self.filter == Some(Role::Agent) {
                        "No coding agents yet. Use + to add one."
                    } else {
                        "Nobody here yet."
                    })
                    .color(theme::text_dim()),
                );
            });
        }
        for person in shown {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                avatar(ui, person);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new(&person.name).color(theme::text()));
                    let detail = match (&person.role, &person.owner_id) {
                        (Role::Agent, Some(owner)) => {
                            let owner_name = state
                                .participants
                                .iter()
                                .find(|item| &item.id == owner)
                                .map(|item| item.name.as_str())
                                .unwrap_or("?");
                            format!("Agent · owned by {owner_name}")
                        }
                        (Role::Agent, None) => "Agent".to_string(),
                        (Role::Human, _) if person.id == state.host_id => {
                            "Human · host".to_string()
                        }
                        (Role::Human, _) => "Human".to_string(),
                    };
                    ui.label(RichText::new(detail).size(11.5).color(theme::text_dim()));
                });
            });
            ui.add_space(4.0);
        }

        if self.new_person.open {
            let humans: Vec<&Participant> = state
                .participants
                .iter()
                .filter(|person| person.role == Role::Human)
                .collect();
            form_frame(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_person.name)
                        .hint_text("Name")
                        .desired_width(f32::INFINITY),
                );
                ui.checkbox(&mut self.new_person.agent, "Coding agent");
                if self.new_person.agent {
                    let mut owner = self.new_person.owner.clone();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Owner").color(theme::text_dim()));
                        let shown = humans
                            .iter()
                            .find(|person| person.id == owner)
                            .map(|person| person.name.clone())
                            .unwrap_or_default();
                        egui::ComboBox::from_id_salt("agent-owner")
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                for person in &humans {
                                    ui.selectable_value(
                                        &mut owner,
                                        person.id.clone(),
                                        &person.name,
                                    );
                                }
                            });
                    });
                    self.new_person.owner = owner;
                }
                ui.horizontal(|ui| {
                    let ready = !self.new_person.name.trim().is_empty();
                    if ui.add_enabled(ready, egui::Button::new("Add")).clicked() {
                        intents.push(Intent::AddPerson {
                            name: self.new_person.name.clone(),
                            agent: self.new_person.agent,
                            owner: self.new_person.owner.clone(),
                        });
                    }
                    if ui.button("Cancel").clicked() {
                        self.new_person = NewPerson::default();
                    }
                });
            });
        }
        ui.add_space(8.0);
    }

    fn tasks(&mut self, ui: &mut egui::Ui, state: &RoomState, intents: &mut Vec<Intent>) {
        let (_, add) = section(ui, "TASKS", &mut self.tasks_open, state.tasks.len(), true);
        if add {
            self.new_task.open = !self.new_task.open;
            if self.new_task.owner.is_empty() {
                self.new_task.owner = self.acting_as.clone();
            }
        }
        if !self.tasks_open {
            return;
        }
        if self.new_task.open {
            self.new_task_form(ui, state, intents);
        }
        if state.tasks.is_empty() && !self.new_task.open {
            empty_state(
                ui,
                "No tasks yet",
                "Add the first task with the + button. Give it an owner and what counts as done.",
            );
        }
        for (status, label) in [
            (TaskStatus::InProgress, "In progress"),
            (TaskStatus::Open, "Open"),
            (TaskStatus::Done, "Done"),
        ] {
            let group: Vec<&Task> = state
                .tasks
                .iter()
                .filter(|task| task.status == status)
                .collect();
            if group.is_empty() {
                continue;
            }
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(
                    RichText::new(format!("{label} · {}", group.len()))
                        .size(11.5)
                        .color(status_color(status)),
                );
            });
            for task in group {
                self.task_card(ui, state, task, intents);
            }
        }
        ui.add_space(8.0);
    }

    fn new_task_form(&mut self, ui: &mut egui::Ui, state: &RoomState, intents: &mut Vec<Intent>) {
        form_frame(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_task.title)
                    .hint_text("What needs doing?")
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal(|ui| {
                ui.label(RichText::new("Owner").color(theme::text_dim()));
                let mut owner = self.new_task.owner.clone();
                let shown = state
                    .participants
                    .iter()
                    .find(|person| person.id == owner)
                    .map(|person| person.name.clone())
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("task-owner")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for person in &state.participants {
                            ui.selectable_value(&mut owner, person.id.clone(), &person.name);
                        }
                    });
                self.new_task.owner = owner;
            });
            ui.add(
                egui::TextEdit::multiline(&mut self.new_task.criteria)
                    .hint_text("Done when… (one criterion per line, up to 8)")
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            if !state.tasks.is_empty() {
                ui.label(RichText::new("Needs first").color(theme::text_dim()));
                for task in &state.tasks {
                    let mut on = self.new_task.dependencies.contains(&task.id);
                    if ui
                        .checkbox(&mut on, format!("{} · {}", task.id, task.title))
                        .changed()
                    {
                        if on {
                            self.new_task.dependencies.insert(task.id.clone());
                        } else {
                            self.new_task.dependencies.remove(&task.id);
                        }
                    }
                }
            }
            ui.horizontal(|ui| {
                let ready = !self.new_task.title.trim().is_empty();
                let create = egui::Button::new(
                    RichText::new("Create task")
                        .strong()
                        .color(theme::button_text()),
                )
                .fill(theme::button());
                if ui.add_enabled(ready, create).clicked() {
                    let mut dependencies: Vec<String> =
                        self.new_task.dependencies.iter().cloned().collect();
                    dependencies.sort();
                    intents.push(Intent::CreateTask {
                        title: self.new_task.title.clone(),
                        owner: self.new_task.owner.clone(),
                        criteria: self
                            .new_task
                            .criteria
                            .lines()
                            .map(str::trim)
                            .filter(|line| !line.is_empty())
                            .map(String::from)
                            .collect(),
                        dependencies,
                    });
                }
                if ui.button("Cancel").clicked() {
                    self.new_task = NewTask::default();
                }
            });
        });
    }

    fn task_card(
        &mut self,
        ui: &mut egui::Ui,
        state: &RoomState,
        task: &Task,
        intents: &mut Vec<Intent>,
    ) {
        let expanded = self.expanded.as_deref() == Some(task.id.as_str());
        let blocked: Vec<&str> = task
            .dependencies
            .iter()
            .filter(|dependency| {
                state
                    .tasks
                    .iter()
                    .find(|item| &item.id == *dependency)
                    .map(|item| item.status)
                    != Some(TaskStatus::Done)
            })
            .map(String::as_str)
            .collect();
        let pending = state
            .pending_handoffs
            .iter()
            .find(|request| request.task_id == task.id);

        egui::Frame::new()
            .fill(theme::mix(theme::bg_sidebar(), theme::text(), 0.05))
            .stroke(Stroke::new(
                1.0,
                if expanded {
                    theme::accent()
                } else {
                    theme::border()
                },
            ))
            .corner_radius(8.0)
            .inner_margin(Margin::symmetric(10, 8))
            .outer_margin(Margin::symmetric(12, 3))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let title = ui
                    .horizontal(|ui| {
                        let (dot, _) =
                            ui.allocate_exact_size(egui::vec2(10.0, 16.0), Sense::hover());
                        ui.painter()
                            .circle_filled(dot.center(), 4.0, status_color(task.status));
                        ui.add(
                            egui::Label::new(
                                RichText::new(&task.title).strong().color(theme::text()),
                            )
                            .wrap()
                            .sense(Sense::click()),
                        )
                    })
                    .inner;
                if title.clicked() {
                    if expanded {
                        self.expanded = None;
                    } else {
                        self.expanded = Some(task.id.clone());
                        self.checked = task
                            .completed_criteria
                            .iter()
                            .map(|criterion| (task.id.clone(), criterion.clone()))
                            .collect();
                        self.handoff_target.clear();
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    chip(
                        ui,
                        &task.owner_name,
                        theme::text(),
                        theme::mix(theme::bg_sidebar(), theme::text(), 0.12),
                    );
                    if !task.acceptance_criteria.is_empty() {
                        let done = task.completed_criteria.len();
                        chip(
                            ui,
                            &format!("{done}/{} done-when", task.acceptance_criteria.len()),
                            theme::text_dim(),
                            theme::mix(theme::bg_sidebar(), theme::text(), 0.08),
                        );
                    }
                    if !blocked.is_empty() && task.status != TaskStatus::Done {
                        chip(
                            ui,
                            &format!("needs {}", blocked.join(", ")),
                            RED,
                            Color32::from_rgba_unmultiplied(240, 113, 120, 30),
                        );
                    }
                    if let Some(request) = pending {
                        chip(
                            ui,
                            &format!("handoff → {}", name_of(state, &request.target_owner_id)),
                            AMBER,
                            Color32::from_rgba_unmultiplied(226, 192, 141, 30),
                        );
                    }
                });

                if expanded {
                    ui.add_space(6.0);
                    self.task_details(ui, state, task, pending.is_some(), intents);
                }
            });
    }

    fn task_details(
        &mut self,
        ui: &mut egui::Ui,
        state: &RoomState,
        task: &Task,
        has_pending: bool,
        intents: &mut Vec<Intent>,
    ) {
        let is_host = self.acting_as == state.host_id;
        let is_owner = self.acting_as == task.owner_id;

        if !task.acceptance_criteria.is_empty() {
            ui.label(
                RichText::new("Done when")
                    .size(11.5)
                    .color(theme::text_dim()),
            );
            for criterion in &task.acceptance_criteria {
                let key = (task.id.clone(), criterion.clone());
                let mut on = self.checked.contains(&key);
                let editable = task.status != TaskStatus::Done;
                if ui
                    .add_enabled(editable, egui::Checkbox::new(&mut on, criterion))
                    .changed()
                {
                    if on {
                        self.checked.insert(key);
                    } else {
                        self.checked.remove(&key);
                    }
                }
            }
            ui.add_space(4.0);
        }

        let all_checked = task
            .acceptance_criteria
            .iter()
            .all(|criterion| self.checked.contains(&(task.id.clone(), criterion.clone())));
        ui.horizontal_wrapped(|ui| {
            let allowed = is_host || is_owner;
            match task.status {
                TaskStatus::Open => {
                    if ui
                        .add_enabled(allowed, egui::Button::new("Start"))
                        .on_disabled_hover_text("Only the owner or host can update this task")
                        .clicked()
                    {
                        intents.push(Intent::Transition {
                            task: task.id.clone(),
                            status: TaskStatus::InProgress,
                            completed: Vec::new(),
                        });
                    }
                }
                TaskStatus::InProgress => {
                    let complete = egui::Button::new(
                        RichText::new("Complete")
                            .strong()
                            .color(theme::button_text()),
                    )
                    .fill(theme::button());
                    if ui
                        .add_enabled(allowed && all_checked, complete)
                        .on_hover_text(if all_checked {
                            "Mark this task done"
                        } else {
                            "Tick every “Done when” item first"
                        })
                        .clicked()
                    {
                        intents.push(Intent::Transition {
                            task: task.id.clone(),
                            status: TaskStatus::Done,
                            completed: task.acceptance_criteria.clone(),
                        });
                    }
                    if ui
                        .add_enabled(allowed, egui::Button::new("Back to open"))
                        .clicked()
                    {
                        intents.push(Intent::Transition {
                            task: task.id.clone(),
                            status: TaskStatus::Open,
                            completed: Vec::new(),
                        });
                    }
                }
                TaskStatus::Done => {
                    if ui
                        .add_enabled(is_host, egui::Button::new("Reopen"))
                        .on_disabled_hover_text("Only the host can reopen a finished task")
                        .clicked()
                    {
                        intents.push(Intent::Transition {
                            task: task.id.clone(),
                            status: TaskStatus::Open,
                            completed: Vec::new(),
                        });
                    }
                }
            }
        });

        // Handoff.
        if has_pending && is_host {
            if ui.button("Approve handoff").clicked() {
                intents.push(Intent::ApproveHandoff(task.id.clone()));
            }
        } else if is_owner && task.status != TaskStatus::Done && !has_pending {
            ui.horizontal(|ui| {
                let mut target = self.handoff_target.clone();
                egui::ComboBox::from_id_salt(("handoff", &task.id))
                    .selected_text(if target.is_empty() {
                        "Hand off to…".to_string()
                    } else {
                        name_of(state, &target)
                    })
                    .show_ui(ui, |ui| {
                        for person in state.participants.iter().filter(|p| p.id != task.owner_id) {
                            ui.selectable_value(&mut target, person.id.clone(), &person.name);
                        }
                    });
                self.handoff_target = target.clone();
                if ui
                    .add_enabled(!target.is_empty(), egui::Button::new("Request"))
                    .clicked()
                {
                    intents.push(Intent::RequestHandoff {
                        task: task.id.clone(),
                        target,
                    });
                }
            });
        }

        // Evidence.
        ui.add_space(6.0);
        ui.label(
            RichText::new("Evidence")
                .size(11.5)
                .color(theme::text_dim()),
        );
        for item in state.evidence.iter().filter(|item| item.task_id == task.id) {
            ui.horizontal_wrapped(|ui| {
                chip(
                    ui,
                    match item.kind {
                        EvidenceKind::Check => "check",
                        EvidenceKind::Review => "review",
                        EvidenceKind::Handoff => "handoff",
                    },
                    theme::text(),
                    theme::mix(theme::bg_sidebar(), theme::text(), 0.12),
                );
                ui.label(RichText::new(&item.summary).color(theme::text()));
                ui.label(
                    RichText::new(format!(
                        "{} · {}",
                        name_of(state, &item.actor_id),
                        room_state::ago(now_epoch_seconds(), item.recorded_at_epoch_seconds)
                    ))
                    .size(11.0)
                    .color(theme::text_dim()),
                );
            });
        }
        ui.horizontal(|ui| {
            let mut kind = self.evidence_kind.clone();
            egui::ComboBox::from_id_salt(("evidence-kind", &task.id))
                .width(78.0)
                .selected_text(match kind {
                    EvidenceKind::Check => "Check",
                    EvidenceKind::Review => "Review",
                    EvidenceKind::Handoff => "Handoff",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut kind, EvidenceKind::Check, "Check");
                    ui.selectable_value(&mut kind, EvidenceKind::Review, "Review");
                    ui.selectable_value(&mut kind, EvidenceKind::Handoff, "Handoff");
                });
            self.evidence_kind = kind;
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.evidence_text)
                    .hint_text("What did you verify?")
                    .desired_width(f32::INFINITY),
            );
            let submit = response.lost_focus()
                && ui.input(|input| input.key_pressed(egui::Key::Enter))
                && !self.evidence_text.trim().is_empty();
            if submit {
                intents.push(Intent::Evidence {
                    task: task.id.clone(),
                    kind: self.evidence_kind.clone(),
                    text: self.evidence_text.clone(),
                });
            }
        });
    }

    fn activity(&mut self, ui: &mut egui::Ui, state: &RoomState) {
        let (_, _) = section(
            ui,
            "ACTIVITY",
            &mut self.activity_open,
            state.activity.len(),
            false,
        );
        if !self.activity_open {
            return;
        }
        let now = now_epoch_seconds();
        for event in state.activity.iter().rev().take(10) {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(14.0);
                ui.label(
                    RichText::new(name_of(state, &event.actor_id))
                        .size(12.0)
                        .color(theme::text()),
                );
                ui.label(
                    RichText::new(&event.detail)
                        .size(12.0)
                        .color(theme::text_dim()),
                );
                ui.label(
                    RichText::new(room_state::ago(now, event.recorded_at_epoch_seconds))
                        .size(11.0)
                        .color(theme::mix(theme::bg_sidebar(), theme::text(), 0.4)),
                );
            });
        }
    }
}

fn name_of(state: &RoomState, id: &str) -> String {
    state
        .participants
        .iter()
        .find(|person| person.id == id)
        .map(|person| person.name.clone())
        .unwrap_or_else(|| id.to_string())
}

fn status_color(status: TaskStatus) -> Color32 {
    match status {
        TaskStatus::Open => theme::text_dim(),
        TaskStatus::InProgress => AMBER,
        TaskStatus::Done => GREEN,
    }
}

fn empty_state(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_max_width((ui.available_width() - 14.0).max(120.0));
            ui.label(RichText::new(title).strong().color(theme::text()));
            ui.add_space(2.0);
            ui.label(RichText::new(detail).color(theme::text_dim()));
            ui.add_space(8.0);
        });
    });
}

fn form_frame(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::mix(theme::bg_sidebar(), theme::text(), 0.04))
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(10, 10))
        .outer_margin(Margin::symmetric(12, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui);
        });
}

fn chip(ui: &mut egui::Ui, text: &str, foreground: Color32, background: Color32) {
    egui::Frame::new()
        .fill(background)
        .corner_radius(9.0)
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(foreground));
        });
}

/// Round for people, rounded-square for agents, tinted by a stable theme color per person.
fn avatar(ui: &mut egui::Ui, person: &Participant) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
    let p = theme::current().palette;
    let palette = [
        p.accent,
        p.function,
        p.string,
        p.keyword,
        p.type_color,
        p.number,
    ];
    let hash = person.id.bytes().fold(7_usize, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as usize)
    });
    let color = palette[hash % palette.len()];
    let painter = ui.painter();
    let fill = theme::mix(theme::bg_sidebar(), color, 0.28);
    match person.role {
        Role::Human => {
            painter.circle_filled(rect.center(), 14.0, fill);
        }
        Role::Agent => {
            painter.rect_filled(rect, 8.0, fill);
        }
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        person
            .name
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string(),
        FontId::proportional(13.0),
        color,
    );
}

/// Collapsible group header. Returns (header clicked, plus button clicked).
fn section(
    ui: &mut egui::Ui,
    title: &str,
    open: &mut bool,
    count: usize,
    plus: bool,
) -> (bool, bool) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), Sense::click());
    let y = rect.center().y;
    let button = Rect::from_center_size(egui::pos2(rect.right() - 20.0, y), egui::vec2(22.0, 22.0));
    let plus_response = if plus {
        Some(ui.interact(button, ui.scope_id().with(("plus", title)), Sense::click()))
    } else {
        None
    };
    {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(rect, 0.0, theme::overlay(8));
        }
        icons::chevron(
            painter,
            egui::pos2(rect.left() + 18.0, y),
            *open,
            theme::text_dim(),
        );
        let galley = painter.layout_no_wrap(
            title.to_string(),
            FontId::proportional(11.0),
            theme::text_dim(),
        );
        let width = galley.size().x;
        painter.galley(
            egui::pos2(rect.left() + 32.0, y - galley.size().y / 2.0),
            galley,
            theme::text_dim(),
        );
        let pill = Rect::from_min_size(
            egui::pos2(rect.left() + 40.0 + width, y - 8.0),
            egui::vec2(24.0, 16.0),
        );
        painter.rect_filled(
            pill,
            8.0,
            theme::mix(theme::bg_sidebar(), theme::text(), 0.14),
        );
        painter.text(
            pill.center(),
            Align2::CENTER_CENTER,
            count,
            FontId::proportional(10.5),
            theme::text(),
        );
        if let Some(plus_response) = &plus_response
            && (response.hovered() || plus_response.hovered())
        {
            if plus_response.hovered() {
                painter.rect_filled(button, 5.0, theme::overlay(24));
            }
            let stroke = Stroke::new(1.5, theme::text());
            let c = button.center();
            painter.line_segment(
                [c + egui::vec2(-4.5, 0.0), c + egui::vec2(4.5, 0.0)],
                stroke,
            );
            painter.line_segment(
                [c + egui::vec2(0.0, -4.5), c + egui::vec2(0.0, 4.5)],
                stroke,
            );
        }
    }
    let plus_clicked = plus_response.as_ref().is_some_and(|r| r.clicked());
    if !plus_clicked && response.clicked() {
        *open = !*open;
        return (true, false);
    }
    (false, plus_clicked)
}

/// `text` cut to at most `max` characters, with an ellipsis when it was longer.
pub fn short(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn damaged_room_cannot_be_replaced_by_create_room() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("forge-room-damaged-{nonce}.json"));
        let store = StateStore::new(&path);
        fs::write(&path, b"damaged room").unwrap();
        fs::write(store.backup_path(), b"damaged backup").unwrap();

        let mut view = RoomView::default();
        view.load_store(store.clone());
        assert!(view.load_failed);
        assert!(!view.create_room());
        assert_eq!(fs::read(&path).unwrap(), b"damaged room");
        assert_eq!(fs::read(store.backup_path()).unwrap(), b"damaged backup");

        fs::remove_file(&path).unwrap();
        fs::remove_file(store.backup_path()).unwrap();
    }

    #[test]
    fn missing_primary_recovers_existing_backup() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("forge-room-backup-{nonce}.json"));
        let store = StateStore::new(&path);
        let state = room_state::new_room("sample", "Admin");
        fs::write(store.backup_path(), serde_json::to_vec(&state).unwrap()).unwrap();

        let mut view = RoomView::default();
        view.load_store(store.clone());
        assert!(view.state().is_some());
        assert!(!view.load_failed);
        assert!(
            view.error
                .as_deref()
                .unwrap()
                .contains("restored from backup")
        );
        assert!(!path.exists());

        fs::remove_file(store.backup_path()).unwrap();
    }
}
