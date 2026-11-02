//! The Room Lobby: the room's overview page (goal, team, agents, milestones, setup, sharing).
//! People, agents and activity come from the room state; everything the core does not model yet
//! (goal, milestones, hackathon setup) is plain local configuration saved beside the room file.

use crate::{icons, theme};
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Layout, Margin, Rect, RichText, Sense, Stroke, Ui, vec2,
};
use forge_room_core::{Participant, Role, RoomState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum MilestoneStatus {
    #[default]
    Pending,
    InProgress,
    Done,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Milestone {
    pub title: String,
    pub due_epoch: u64,
    #[serde(default)]
    pub status: MilestoneStatus,
    #[serde(default)]
    pub percent: u8,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct LobbyConfig {
    pub tagline: String,
    pub kind: String,
    pub duration_hours: u32,
    pub start_epoch: u64,
    pub private: bool,
    pub goal: String,
    pub goals: Vec<(String, bool)>,
    pub repo_url: String,
    pub permissions: [bool; 4],
    pub mode: String,
    pub submission: String,
    pub team_size: String,
    pub sharing: [bool; 4],
    pub milestones: Vec<Milestone>,
    /// Participant id -> short label such as "Lead" or "Backend".
    pub labels: BTreeMap<String, String>,
    /// Participant id -> description shown under the name.
    pub notes: BTreeMap<String, String>,
}

const PERMISSIONS: [&str; 4] = ["Shared Edit", "Run", "Build", "Deploy"];
const SHARING: [&str; 4] = [
    "Follow IDE Session (Read-Only)",
    "Shared Control (On Request)",
    "Agent Activity Visibility",
    "Record Session (Optional)",
];
const MODES: [&str; 4] = [
    "Competition (24 Hours)",
    "Competition (48 Hours)",
    "Casual Build",
    "Workshop",
];
const TEAM_SIZES: [&str; 4] = [
    "Solo",
    "2 Humans + AI Agents",
    "3 – 5 Humans + AI Agents",
    "6+ Humans + AI Agents",
];

impl Default for LobbyConfig {
    fn default() -> Self {
        let start = crate::room_state::now();
        let hour = 3600;
        let steps = [
            ("Setup & Plan", 0),
            ("Core Dev", 4),
            ("Integration", 10),
            ("Demo & Test", 16),
            ("Submit", 24),
        ];
        Self {
            tagline: String::new(),
            kind: "Hackathon".into(),
            duration_hours: 24,
            start_epoch: start,
            private: true,
            goal: String::new(),
            goals: Vec::new(),
            repo_url: String::new(),
            permissions: [true, true, true, false],
            mode: MODES[0].into(),
            submission: "Git Tag + Demo Link + Pitch Deck".into(),
            team_size: TEAM_SIZES[2].into(),
            sharing: [true, true, true, false],
            milestones: steps
                .into_iter()
                .map(|(title, offset)| Milestone {
                    title: title.into(),
                    due_epoch: start + offset * hour,
                    status: MilestoneStatus::Pending,
                    percent: 0,
                })
                .collect(),
            labels: BTreeMap::new(),
            notes: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Tab {
    Lobby,
    Team,
    Agents,
    Setup,
    Milestones,
    Repository,
    Settings,
}

const TABS: [(Tab, &str); 7] = [
    (Tab::Lobby, "Lobby"),
    (Tab::Team, "Team"),
    (Tab::Agents, "Agents"),
    (Tab::Setup, "Setup"),
    (Tab::Milestones, "Milestones"),
    (Tab::Repository, "Repository"),
    (Tab::Settings, "Settings"),
];

/// What the lobby asks the app to do.
pub enum Action {
    EnterRoom,
    CreateRoom,
    AddMember,
    AddAgent,
}

/// Facts the lobby reads from the app each frame.
pub struct Context<'a> {
    pub workspace: &'a str,
    pub branch: Option<&'a str>,
    pub runtimes: &'a [String],
    pub state: Option<&'a RoomState>,
    pub acting_as: &'a str,
}

pub struct Lobby {
    config: LobbyConfig,
    path: Option<PathBuf>,
    tab: Tab,
    invite: String,
    invite_role: usize,
    invite_note: Option<String>,
}

impl Default for Lobby {
    fn default() -> Self {
        Self {
            config: LobbyConfig::default(),
            path: None,
            tab: Tab::Lobby,
            invite: String::new(),
            invite_role: 0,
            invite_note: None,
        }
    }
}

fn now() -> u64 {
    crate::room_state::now()
}

fn card_fill() -> Color32 {
    theme::mix(theme::bg_editor(), theme::text(), 0.035)
}

fn inner_fill() -> Color32 {
    theme::mix(theme::bg_editor(), theme::text(), 0.06)
}

fn accent_soft() -> Color32 {
    theme::mix(theme::bg_editor(), theme::accent(), 0.22)
}

fn load_config(path: &Path) -> Option<LobbyConfig> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

impl Lobby {
    /// Switch to the lobby saved at `path` (or a fresh one), for the project that was just opened.
    pub fn load_for(&mut self, path: Option<PathBuf>) {
        let config = path.as_deref().and_then(load_config);
        *self = Self {
            config: config.unwrap_or_default(),
            path,
            ..Self::default()
        };
    }

    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.config) {
            let _ = std::fs::write(path, bytes);
        }
    }

    /// Seconds left in the room's time box.
    pub fn remaining(&self) -> u64 {
        let end = self.config.start_epoch + u64::from(self.config.duration_hours) * 3600;
        end.saturating_sub(now())
    }

    fn setup_items(&self, context: &Context) -> Vec<&'static str> {
        let mut items = Vec::new();
        if self.config.repo_url.trim().is_empty() {
            items.push("Link a repository");
        }
        if self.config.goal.trim().is_empty() {
            items.push("Write the project goal");
        }
        let agents = context
            .state
            .map(|state| {
                state
                    .participants
                    .iter()
                    .any(|person| person.role == Role::Agent)
            })
            .unwrap_or(false);
        if !agents {
            items.push("Add an AI agent");
        }
        items
    }

    pub fn show(&mut self, ui: &mut Ui, context: &Context) -> Option<Action> {
        let before = serde_json::to_string(&self.config).unwrap_or_default();
        let mut action = None;
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));

        self.pulse(ui, context);
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                        self.header(ui, context, &mut action);
                        self.tabs(ui);
                        ui.add_space(2.0);
                        self.body(ui, context, &mut action);
                    });
            });

        if serde_json::to_string(&self.config).unwrap_or_default() != before {
            self.save();
        }
        action
    }

    // ---- Room Pulse ---------------------------------------------------------------------

    fn pulse(&mut self, ui: &mut Ui, context: &Context) {
        let items = self.setup_items(context);
        egui::Frame::new()
            .fill(theme::bg_sidebar())
            .inner_margin(Margin::symmetric(14, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    icons::draw(ui.painter(), "pulse", rect.center(), 18.0, theme::accent());
                    ui.label(RichText::new("Room Pulse").strong().color(theme::text()));
                    ui.add_space(8.0);
                    if let Some(state) = context.state {
                        let clock = now();
                        for event in state.activity.iter().rev().take(4) {
                            let actor = state
                                .participants
                                .iter()
                                .find(|person| person.id == event.actor_id)
                                .map(|person| person.name.as_str())
                                .unwrap_or("Someone");
                            let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                            ui.painter().circle_filled(
                                rect.center(),
                                3.5,
                                Color32::from_rgb(110, 200, 120),
                            );
                            let text = format!(
                                "{actor}: {}  ·  {}",
                                crate::rooms::short(&event.detail, 38),
                                crate::room_state::ago(clock, event.recorded_at_epoch_seconds)
                            );
                            ui.label(RichText::new(text).color(theme::text_dim()).size(13.0));
                            ui.add_space(6.0);
                        }
                    } else {
                        ui.label(
                            RichText::new("No room yet. Create one to see live activity here.")
                                .color(theme::text_dim()),
                        );
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !items.is_empty() {
                            let label = format!(
                                "{} Setup Item{}",
                                items.len(),
                                if items.len() == 1 { "" } else { "s" }
                            );
                            egui::Frame::new()
                                .fill(Color32::from_rgba_unmultiplied(226, 160, 60, 30))
                                .stroke(Stroke::new(
                                    1.0,
                                    Color32::from_rgba_unmultiplied(226, 160, 60, 140),
                                ))
                                .corner_radius(8.0)
                                .inner_margin(Margin::symmetric(10, 4))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (rect, _) = ui
                                            .allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                                        icons::draw(
                                            ui.painter(),
                                            "warning",
                                            rect.center(),
                                            16.0,
                                            Color32::from_rgb(226, 160, 60),
                                        );
                                        ui.label(RichText::new(label).color(theme::text()));
                                        if ghost_button(ui, "Review", None).clicked() {
                                            self.tab = Tab::Setup;
                                        }
                                    });
                                });
                        }
                    });
                });
            });
        let rect = ui.min_rect();
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, theme::border()),
        );
    }

    // ---- Header -------------------------------------------------------------------------

    fn header(&mut self, ui: &mut Ui, context: &Context, action: &mut Option<Action>) {
        let width = ui.available_width();
        egui::Frame::new().show(ui, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                // Project mark.
                let (mark, _) = ui.allocate_exact_size(vec2(92.0, 92.0), Sense::hover());
                ui.painter().rect_filled(mark, 12.0, accent_soft());
                ui.painter().rect_stroke(
                    mark,
                    12.0,
                    Stroke::new(1.0, theme::border()),
                    egui::StrokeKind::Inside,
                );
                project_mark(ui.painter(), mark.center(), 40.0, theme::accent());

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(context.workspace)
                            .size(30.0)
                            .strong()
                            .color(theme::text()),
                    );
                    ui.label(
                        RichText::new("Project Lobby")
                            .size(18.0)
                            .color(theme::text_dim()),
                    );
                });

                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    let tagline = &mut self.config.tagline;
                    ui.add(
                        egui::TextEdit::singleline(tagline)
                            .hint_text("One line about this room")
                            .frame(egui::Frame::NONE)
                            .desired_width(360.0)
                            .font(FontId::proportional(15.0)),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let hackathon = self.config.kind.clone();
                        chip_icon(ui, "rocket", &hackathon, theme::accent(), true);
                        chip_icon(
                            ui,
                            "clock",
                            &format!("{} Hours", self.config.duration_hours),
                            theme::text_dim(),
                            false,
                        );
                        let humans = context
                            .state
                            .map(|state| {
                                state
                                    .participants
                                    .iter()
                                    .filter(|person| person.role == Role::Human)
                                    .count()
                            })
                            .unwrap_or(1);
                        chip_icon(
                            ui,
                            "users",
                            if humans > 1 { "Multiplayer" } else { "Solo" },
                            theme::text_dim(),
                            false,
                        );
                    });
                });

                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    if ghost_button(ui, "···", None).clicked() {
                        self.tab = Tab::Settings;
                    }
                    if accent_button(ui, "Enter Room", Some("arrow-right")).clicked() {
                        *action = Some(Action::EnterRoom);
                    }
                    self.countdown(ui);
                });
            });
        });
    }

    fn countdown(&self, ui: &mut Ui) {
        let remaining = self.remaining();
        let (hours, minutes, seconds) = (remaining / 3600, remaining / 60 % 60, remaining % 60);
        let size = vec2(260.0, 88.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 10.0, card_fill());
        painter.rect_stroke(
            rect,
            10.0,
            Stroke::new(1.0, theme::border()),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center_top() + vec2(0.0, 16.0),
            Align2::CENTER_CENTER,
            format!(
                "{} · {}-HOUR Build",
                self.config.mode.split(" (").next().unwrap_or("Competition"),
                self.config.duration_hours
            ),
            FontId::proportional(13.0),
            theme::text(),
        );
        for (index, (value, unit)) in [(hours, "HRS"), (minutes, "MIN"), (seconds, "SEC")]
            .into_iter()
            .enumerate()
        {
            let x = rect.left() + 46.0 + index as f32 * 84.0;
            painter.text(
                egui::pos2(x, rect.center().y + 6.0),
                Align2::CENTER_CENTER,
                format!("{value:02}"),
                FontId::proportional(34.0),
                theme::accent(),
            );
            painter.text(
                egui::pos2(x, rect.bottom() - 12.0),
                Align2::CENTER_CENTER,
                unit,
                FontId::proportional(11.0),
                theme::text_dim(),
            );
            if index < 2 {
                painter.text(
                    egui::pos2(x + 42.0, rect.center().y + 4.0),
                    Align2::CENTER_CENTER,
                    ":",
                    FontId::proportional(30.0),
                    theme::accent(),
                );
            }
        }
    }

    fn tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (tab, label) in TABS {
                let galley = ui.painter().layout_no_wrap(
                    label.to_string(),
                    FontId::proportional(16.0),
                    if self.tab == tab {
                        theme::text()
                    } else {
                        theme::text_dim()
                    },
                );
                let (rect, response) =
                    ui.allocate_exact_size(vec2(galley.size().x + 56.0, 38.0), Sense::click());
                if self.tab == tab {
                    ui.painter().rect_filled(rect, 6.0, card_fill());
                    ui.painter().line_segment(
                        [rect.left_bottom(), rect.right_bottom()],
                        Stroke::new(2.0, theme::accent()),
                    );
                } else if response.hovered() {
                    ui.painter().rect_filled(rect, 6.0, card_fill());
                }
                ui.painter().galley(
                    egui::pos2(
                        rect.center().x - galley.size().x / 2.0,
                        rect.center().y - galley.size().y / 2.0,
                    ),
                    galley,
                    theme::text(),
                );
                if response.clicked() {
                    self.tab = tab;
                }
            }
        });
        let rect = ui.min_rect();
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.bottom() + 1.0),
                egui::pos2(rect.right(), rect.bottom() + 1.0),
            ],
            Stroke::new(1.0, theme::border()),
        );
    }

    // ---- Body ---------------------------------------------------------------------------

    fn body(&mut self, ui: &mut Ui, context: &Context, action: &mut Option<Action>) {
        let total = ui.available_width();
        let gap = 12.0;
        let right = (total * 0.37).clamp(320.0, 520.0);
        let left = total - right - gap;
        let tab = self.tab;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, 12.0);
            ui.allocate_ui_with_layout(vec2(left, 10.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(left);
                ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                if matches!(tab, Tab::Lobby | Tab::Settings) {
                    self.overview(ui, context);
                }
                if matches!(tab, Tab::Lobby | Tab::Team | Tab::Agents) {
                    let both = tab == Tab::Lobby;
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        let each = if both { (left - 12.0) / 2.0 } else { left };
                        if tab != Tab::Agents {
                            ui.allocate_ui_with_layout(
                                vec2(each, 10.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    ui.set_width(each);
                                    self.team(ui, context, action);
                                },
                            );
                        }
                        if tab != Tab::Team {
                            ui.allocate_ui_with_layout(
                                vec2(each, 10.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    ui.set_width(each);
                                    self.agents(ui, context, action);
                                },
                            );
                        }
                    });
                }
                if matches!(tab, Tab::Lobby | Tab::Milestones) {
                    self.milestones(ui);
                }
            });
            ui.allocate_ui_with_layout(vec2(right, 10.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(right);
                ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                if matches!(tab, Tab::Lobby | Tab::Setup | Tab::Repository) {
                    self.project_setup(ui, context);
                }
                if matches!(tab, Tab::Lobby | Tab::Setup | Tab::Settings) {
                    self.hackathon(ui);
                }
                if matches!(tab, Tab::Lobby | Tab::Team) {
                    self.invite(ui);
                }
            });
        });
    }

    fn overview(&mut self, ui: &mut Ui, context: &Context) {
        card(
            ui,
            "room",
            "Room Overview",
            |_| {},
            |ui| {
                let width = ui.available_width();
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    let half = (width - 14.0) * 0.5;
                    ui.allocate_ui_with_layout(
                        vec2(half, 10.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(half);
                            let kind = match context.state {
                                Some(state)
                                    if state.participants.iter().any(|p| p.role == Role::Agent) =>
                                {
                                    "Mixed (Humans + AI)"
                                }
                                Some(_) => "Humans",
                                None => "Not created yet",
                            };
                            let start = self.config.start_epoch;
                            let end = start + u64::from(self.config.duration_hours) * 3600;
                            kv(ui, "room", "Room Name", |ui| {
                                ui.label(format!("{} Room", context.workspace));
                            });
                            kv(ui, "users", "Room Type", |ui| {
                                ui.label(kind);
                            });
                            kv(ui, "eye", "Visibility", |ui| {
                                let label = if self.config.private {
                                    "Private (Invite Only)"
                                } else {
                                    "Open to anyone with the link"
                                };
                                if ui
                                    .add(egui::Label::new(label).sense(Sense::click()))
                                    .on_hover_text("Click to change")
                                    .clicked()
                                {
                                    self.config.private = !self.config.private;
                                }
                            });
                            kv(ui, "clock", "Duration", |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::DragValue::new(&mut self.config.duration_hours)
                                            .range(1..=720)
                                            .suffix(" Hours"),
                                    );
                                });
                            });
                            kv(ui, "calendar", "Start Time", |ui| {
                                ui.label(format_time(start));
                            });
                            kv(ui, "calendar", "End Time", |ui| {
                                ui.label(format_time(end));
                            });
                        },
                    );
                    ui.allocate_ui_with_layout(
                        vec2(half, 10.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(half);
                            egui::Frame::new()
                                .fill(inner_fill())
                                .stroke(Stroke::new(1.0, theme::border()))
                                .corner_radius(8.0)
                                .inner_margin(Margin::same(12))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let (rect, _) = ui
                                            .allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                                        icons::draw(
                                            ui.painter(),
                                            "target",
                                            rect.center(),
                                            18.0,
                                            theme::text_dim(),
                                        );
                                        ui.label(RichText::new("Project Goal").size(16.0).strong());
                                    });
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.config.goal)
                                            .hint_text("What will this room build or ship?")
                                            .frame(egui::Frame::NONE)
                                            .desired_rows(2)
                                            .desired_width(f32::INFINITY),
                                    );
                                    ui.add_space(4.0);
                                    let mut remove = None;
                                    for (index, (text, done)) in
                                        self.config.goals.iter_mut().enumerate()
                                    {
                                        ui.horizontal(|ui| {
                                            ui.checkbox(done, "");
                                            ui.add(
                                                egui::TextEdit::singleline(text)
                                                    .frame(egui::Frame::NONE)
                                                    .desired_width(f32::INFINITY),
                                            );
                                        });
                                        if text.is_empty() && !ui.memory(|m| m.focused().is_some())
                                        {
                                            remove = Some(index);
                                        }
                                    }
                                    if let Some(index) = remove {
                                        self.config.goals.remove(index);
                                    }
                                    if ghost_button(ui, "+ Add item", None).clicked() {
                                        self.config.goals.push((String::new(), false));
                                    }
                                });
                        },
                    );
                });
            },
        );
    }

    fn team(&mut self, ui: &mut Ui, context: &Context, action: &mut Option<Action>) {
        let people: Vec<&Participant> = context
            .state
            .map(|state| {
                state
                    .participants
                    .iter()
                    .filter(|person| person.role == Role::Human)
                    .collect()
            })
            .unwrap_or_default();
        let title = format!("Team Members ({})", people.len());
        let add = std::cell::Cell::new(false);
        card(
            ui,
            "users",
            &title,
            |ui| {
                if outline_button(ui, "+ Add Member").clicked() {
                    add.set(true);
                }
            },
            |ui| {
                if context.state.is_none() {
                    ui.label(
                        RichText::new("Create the room to add people.").color(theme::text_dim()),
                    );
                    if accent_button(ui, "Create Room", None).clicked() {
                        *action = Some(Action::CreateRoom);
                    }
                    return;
                }
                for person in people {
                    let you = person.id == context.acting_as;
                    let host = context
                        .state
                        .is_some_and(|state| state.host_id == person.id);
                    let label = self
                        .config
                        .labels
                        .get(&person.id)
                        .cloned()
                        .unwrap_or_else(|| if host { "Lead".into() } else { "Member".into() });
                    self.person_row(ui, person, you, &label, false);
                }
            },
        );
        if add.get() {
            *action = Some(Action::AddMember);
        }
    }

    fn agents(&mut self, ui: &mut Ui, context: &Context, action: &mut Option<Action>) {
        let agents: Vec<&Participant> = context
            .state
            .map(|state| {
                state
                    .participants
                    .iter()
                    .filter(|person| person.role == Role::Agent)
                    .collect()
            })
            .unwrap_or_default();
        let title = format!("AI Agents ({})", agents.len());
        let add = std::cell::Cell::new(false);
        card(
            ui,
            "bot",
            &title,
            |ui| {
                if outline_button(ui, "+ Add Agent").clicked() {
                    add.set(true);
                }
            },
            |ui| {
                if agents.is_empty() {
                    ui.label(
                        RichText::new("No agents yet. Agents act for a person and need an owner.")
                            .color(theme::text_dim()),
                    );
                }
                for person in agents {
                    let label = self
                        .config
                        .labels
                        .get(&person.id)
                        .cloned()
                        .unwrap_or_else(|| "Agent".into());
                    self.person_row(ui, person, false, &label, true);
                }
            },
        );
        if add.get() {
            *action = Some(Action::AddAgent);
        }
    }

    fn person_row(
        &mut self,
        ui: &mut Ui,
        person: &Participant,
        you: bool,
        label: &str,
        agent: bool,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let (rect, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            avatar(ui.painter(), rect, &person.name, agent, Some(you));
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let name = if you {
                        format!("{} (You)", person.name)
                    } else {
                        person.name.clone()
                    };
                    ui.label(RichText::new(name).strong().size(15.0));
                    chip(ui, label, theme::accent());
                });
                let state = if agent {
                    "Ready"
                } else if you {
                    "Online"
                } else {
                    "In room"
                };
                let note = self
                    .config
                    .notes
                    .get(&person.id)
                    .cloned()
                    .unwrap_or_default();
                ui.horizontal(|ui| {
                    let color = if agent || you {
                        Color32::from_rgb(240, 113, 120)
                    } else {
                        theme::text_dim()
                    };
                    ui.label(RichText::new(state).color(color).size(13.0));
                    if !note.is_empty() {
                        ui.label(
                            RichText::new(format!("· {note}"))
                                .color(theme::text_dim())
                                .size(13.0),
                        );
                    }
                });
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.menu_button("···", |ui| {
                    ui.label(RichText::new("Role label").color(theme::text_dim()));
                    let mut text = self
                        .config
                        .labels
                        .get(&person.id)
                        .cloned()
                        .unwrap_or_default();
                    if ui.text_edit_singleline(&mut text).changed() {
                        self.config.labels.insert(person.id.clone(), text);
                    }
                    ui.label(RichText::new("Description").color(theme::text_dim()));
                    let mut note = self
                        .config
                        .notes
                        .get(&person.id)
                        .cloned()
                        .unwrap_or_default();
                    if ui.text_edit_singleline(&mut note).changed() {
                        self.config.notes.insert(person.id.clone(), note);
                    }
                });
                if agent {
                    let slug = crate::room_state::slug(&person.name);
                    chip(ui, &format!("WT: agent/{slug}"), theme::text_dim());
                }
            });
        });
        ui.add_space(2.0);
    }

    fn milestones(&mut self, ui: &mut Ui) {
        let done = self
            .config
            .milestones
            .iter()
            .filter(|m| m.status == MilestoneStatus::Done)
            .count();
        let total = self.config.milestones.len().max(1);
        card(
            ui,
            "flag",
            "Milestones & Plan",
            |_| {},
            |ui| {
                let width = ui.available_width();
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    let track = width - 220.0 - 14.0;
                    ui.allocate_ui_with_layout(
                        vec2(track, 10.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(track);
                            let count = self.config.milestones.len().max(1);
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(track, 96.0), Sense::hover());
                            let step = track / count as f32;
                            let line_y = rect.top() + 16.0;
                            ui.painter().line_segment(
                                [
                                    egui::pos2(rect.left() + step / 2.0, line_y),
                                    egui::pos2(rect.right() - step / 2.0, line_y),
                                ],
                                Stroke::new(2.0, theme::border()),
                            );
                            for (index, milestone) in self.config.milestones.iter_mut().enumerate()
                            {
                                let x = rect.left() + step * (index as f32 + 0.5);
                                let center = egui::pos2(x, line_y);
                                let hit = Rect::from_center_size(center, vec2(34.0, 34.0));
                                let response = ui
                                    .interact(
                                        hit,
                                        ui.scope_id().with(("milestone", index)),
                                        Sense::click(),
                                    )
                                    .on_hover_text("Click to change status");
                                if response.clicked() {
                                    milestone.status = match milestone.status {
                                        MilestoneStatus::Pending => MilestoneStatus::InProgress,
                                        MilestoneStatus::InProgress => MilestoneStatus::Done,
                                        MilestoneStatus::Done => MilestoneStatus::Pending,
                                    };
                                }
                                let painter = ui.painter();
                                match milestone.status {
                                    MilestoneStatus::Done => {
                                        painter.circle_filled(center, 13.0, theme::accent());
                                        icons::draw(painter, "check", center, 16.0, Color32::BLACK);
                                    }
                                    MilestoneStatus::InProgress => {
                                        painter.circle_filled(center, 13.0, theme::bg_editor());
                                        painter.circle_stroke(
                                            center,
                                            13.0,
                                            Stroke::new(3.0, Color32::from_rgb(240, 170, 60)),
                                        );
                                        painter.circle_filled(
                                            center,
                                            4.5,
                                            Color32::from_rgb(240, 170, 60),
                                        );
                                    }
                                    MilestoneStatus::Pending => {
                                        painter.circle_filled(center, 13.0, theme::bg_editor());
                                        painter.circle_stroke(
                                            center,
                                            13.0,
                                            Stroke::new(1.5, theme::text_dim()),
                                        );
                                    }
                                }
                                painter.text(
                                    egui::pos2(x, line_y + 32.0),
                                    Align2::CENTER_CENTER,
                                    format!("M{}: {}", index + 1, milestone.title),
                                    FontId::proportional(13.5),
                                    theme::text(),
                                );
                                let (status, color) = match milestone.status {
                                    MilestoneStatus::Done => ("Done".to_string(), theme::accent()),
                                    MilestoneStatus::InProgress => (
                                        format!("In Progress ({}%)", milestone.percent),
                                        Color32::from_rgb(240, 170, 60),
                                    ),
                                    MilestoneStatus::Pending => {
                                        ("Pending".to_string(), theme::text_dim())
                                    }
                                };
                                painter.text(
                                    egui::pos2(x, line_y + 52.0),
                                    Align2::CENTER_CENTER,
                                    status,
                                    FontId::proportional(13.0),
                                    color,
                                );
                                painter.text(
                                    egui::pos2(x, line_y + 72.0),
                                    Align2::CENTER_CENTER,
                                    format_short(milestone.due_epoch),
                                    FontId::proportional(12.5),
                                    theme::text_dim(),
                                );
                            }
                        },
                    );
                    ui.allocate_ui_with_layout(
                        vec2(220.0, 10.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .fill(inner_fill())
                                .stroke(Stroke::new(1.0, theme::border()))
                                .corner_radius(8.0)
                                .inner_margin(Margin::same(12))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(RichText::new("Progress").strong().size(15.0));
                                    ui.label(
                                        RichText::new(format!(
                                            "{done} / {} milestones",
                                            self.config.milestones.len()
                                        ))
                                        .color(theme::text_dim()),
                                    );
                                    let percent = done as f32 / total as f32;
                                    ui.horizontal(|ui| {
                                        let (bar, _) = ui.allocate_exact_size(
                                            vec2(ui.available_width() - 44.0, 12.0),
                                            Sense::hover(),
                                        );
                                        ui.painter().rect_filled(bar, 6.0, theme::border());
                                        let mut fill = bar;
                                        fill.set_width(bar.width() * percent);
                                        ui.painter().rect_filled(fill, 6.0, theme::accent());
                                        ui.label(format!("{:.0}%", percent * 100.0));
                                    });
                                });
                        },
                    );
                });
            },
        );
    }

    fn project_setup(&mut self, ui: &mut Ui, context: &Context) {
        card(
            ui,
            "settings",
            "Project Setup",
            |_| {},
            |ui| {
                kv(ui, "github", "Repository", |ui| {
                    if self.config.repo_url.trim().is_empty() {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.repo_url)
                                .hint_text("github.com/owner/repo")
                                .desired_width(f32::INFINITY),
                        );
                    } else {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let url = self.config.repo_url.clone();
                            let full = if url.starts_with("http") {
                                url.clone()
                            } else {
                                format!("https://{url}")
                            };
                            if icon_button(ui, "external-link", "Open in browser").clicked() {
                                ui.ctx().open_url(egui::OpenUrl::new_tab(full));
                            }
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(&url).color(Color32::from_rgb(110, 165, 255)),
                                    )
                                    .sense(Sense::click()),
                                )
                                .on_hover_text("Click to clear and edit")
                                .clicked()
                            {
                                self.config.repo_url.clear();
                            }
                        });
                    }
                });
                kv(ui, "source-control", "Branch", |ui| {
                    field(ui, context.branch.unwrap_or("no repository"));
                });
                kv(ui, "layers", "Workspace", |ui| {
                    field(ui, &format!("{} (workspace)", context.workspace));
                });
                kv(ui, "code", "Runtime", |ui| {
                    ui.horizontal(|ui| {
                        if context.runtimes.is_empty() {
                            ui.label(RichText::new("Not detected").color(theme::text_dim()));
                        }
                        for runtime in context.runtimes {
                            chip(ui, runtime, theme::text());
                        }
                    });
                });
                kv(ui, "shield", "Permissions", |ui| {
                    ui.horizontal(|ui| {
                        let granted: Vec<&str> = PERMISSIONS
                            .iter()
                            .zip(self.config.permissions)
                            .filter(|(_, on)| *on)
                            .map(|(name, _)| *name)
                            .collect();
                        ui.label(granted.join(" · "));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.menu_button("Manage", |ui| {
                                for (name, on) in
                                    PERMISSIONS.iter().zip(self.config.permissions.iter_mut())
                                {
                                    ui.checkbox(on, *name);
                                }
                            });
                        });
                    });
                });
            },
        );
    }

    fn hackathon(&mut self, ui: &mut Ui) {
        card(
            ui,
            "flag",
            "Hackathon Configuration",
            |_| {},
            |ui| {
                kv(ui, "calendar", "Mode", |ui| {
                    combo(ui, "mode", &mut self.config.mode, &MODES);
                });
                kv(ui, "upload-box", "Submission", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.config.submission)
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY),
                    );
                });
                kv(ui, "users", "Team Size", |ui| {
                    combo(ui, "team-size", &mut self.config.team_size, &TEAM_SIZES);
                });
                ui.horizontal_top(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    icons::draw(
                        ui.painter(),
                        "share",
                        rect.center(),
                        18.0,
                        theme::text_dim(),
                    );
                    ui.add_space(14.0);
                    ui.label(RichText::new("Content Sharing").color(theme::text()));
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        for (name, on) in SHARING.iter().zip(self.config.sharing.iter_mut()) {
                            ui.checkbox(on, *name);
                        }
                    });
                });
            },
        );
    }

    fn invite(&mut self, ui: &mut Ui) {
        card(
            ui,
            "user-plus",
            "Invite to Room",
            |_| {},
            |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.invite)
                            .hint_text("Enter email or team code...")
                            .desired_width((ui.available_width() - 190.0).max(80.0)),
                    );
                    let roles = ["Member", "Viewer", "Agent owner"];
                    egui::ComboBox::from_id_salt("invite-role")
                        .selected_text(roles[self.invite_role])
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            for (index, role) in roles.iter().enumerate() {
                                ui.selectable_value(&mut self.invite_role, index, *role);
                            }
                        });
                    if accent_button(ui, "Invite", Some("send")).clicked() {
                        self.invite_note = Some(
                        "Network invites are not available in this preview yet; rooms are local."
                            .into(),
                    );
                    }
                });
                if let Some(note) = &self.invite_note {
                    ui.label(RichText::new(note).size(12.5).color(theme::text_dim()));
                }
            },
        );
    }
}

// ---- Small widgets --------------------------------------------------------------------

fn card(
    ui: &mut Ui,
    icon: &str,
    title: &str,
    right: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui),
) {
    egui::Frame::new()
        .fill(card_fill())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(10.0)
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                icons::draw(ui.painter(), icon, rect.center(), 20.0, theme::text_dim());
                ui.label(
                    RichText::new(title)
                        .size(17.0)
                        .strong()
                        .color(theme::text()),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), right);
            });
            ui.add_space(6.0);
            body(ui);
        });
}

/// A labelled row: icon and label on the left, the value widget in a fixed column.
fn kv(ui: &mut Ui, icon: &str, label: &str, value: impl FnOnce(&mut Ui)) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
    let painter = ui.painter();
    icons::draw(
        painter,
        icon,
        egui::pos2(rect.left() + 10.0, rect.center().y),
        18.0,
        theme::text_dim(),
    );
    painter.text(
        egui::pos2(rect.left() + 34.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        theme::text(),
    );
    let value_rect = Rect::from_min_max(
        egui::pos2(
            rect.left() + (rect.width() * 0.34).clamp(110.0, 190.0),
            rect.top(),
        ),
        rect.right_bottom(),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(value_rect)
            .layout(Layout::left_to_right(Align::Center)),
        value,
    );
}

fn field(ui: &mut Ui, text: &str) {
    let width = ui.available_width();
    egui::Frame::new()
        .fill(inner_fill())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(6.0)
        .inner_margin(Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.set_width(width - 22.0);
            ui.label(text);
        });
}

fn combo(ui: &mut Ui, id: &str, value: &mut String, options: &[&str]) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(value.clone())
        .width(ui.available_width() - 8.0)
        .show_ui(ui, |ui| {
            for option in options {
                ui.selectable_value(value, (*option).to_string(), *option);
            }
        });
}

fn chip(ui: &mut Ui, text: &str, color: Color32) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::proportional(12.5), color);
    let (rect, _) = ui.allocate_exact_size(galley.size() + vec2(16.0, 6.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 6.0, theme::mix(theme::bg_editor(), color, 0.12));
    ui.painter().rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, theme::mix(theme::bg_editor(), color, 0.4)),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
}

fn chip_icon(ui: &mut Ui, icon: &str, text: &str, color: Color32, strong: bool) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::proportional(14.0), color);
    let (rect, _) = ui.allocate_exact_size(galley.size() + vec2(40.0, 12.0), Sense::hover());
    let fill = if strong {
        theme::mix(theme::bg_editor(), color, 0.15)
    } else {
        inner_fill()
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    ui.painter().rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0, theme::mix(theme::bg_editor(), color, 0.5)),
        egui::StrokeKind::Inside,
    );
    icons::draw(
        ui.painter(),
        icon,
        egui::pos2(rect.left() + 17.0, rect.center().y),
        16.0,
        color,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 30.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}

fn icon_button(ui: &mut Ui, icon: &str, hint: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
    icons::draw(
        ui.painter(),
        icon,
        rect.center(),
        18.0,
        if response.hovered() {
            theme::text()
        } else {
            theme::text_dim()
        },
    );
    response.on_hover_text(hint)
}

#[allow(clippy::too_many_arguments)]
fn button_frame(
    ui: &mut Ui,
    label: &str,
    icon: Option<&str>,
    fill: Color32,
    stroke: Color32,
    text: Color32,
    size: f32,
    height: f32,
) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), FontId::proportional(size), text);
    let extra = if icon.is_some() { 30.0 } else { 0.0 };
    let (rect, response) =
        ui.allocate_exact_size(vec2(galley.size().x + 28.0 + extra, height), Sense::click());
    let fill = if response.hovered() {
        theme::mix(fill, Color32::WHITE, 0.08)
    } else {
        fill
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    ui.painter().rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    let text_x = rect.left() + 14.0;
    ui.painter().galley(
        egui::pos2(text_x, rect.center().y - galley.size().y / 2.0),
        galley.clone(),
        text,
    );
    if let Some(icon) = icon {
        icons::draw(
            ui.painter(),
            icon,
            egui::pos2(text_x + galley.size().x + 16.0, rect.center().y),
            18.0,
            text,
        );
    }
    response
}

fn accent_button(ui: &mut Ui, label: &str, icon: Option<&str>) -> egui::Response {
    button_frame(
        ui,
        label,
        icon,
        theme::accent(),
        theme::accent(),
        Color32::BLACK,
        16.0,
        42.0,
    )
}

fn outline_button(ui: &mut Ui, label: &str) -> egui::Response {
    button_frame(
        ui,
        label,
        None,
        accent_soft(),
        theme::accent(),
        theme::text(),
        13.5,
        30.0,
    )
}

fn ghost_button(ui: &mut Ui, label: &str, icon: Option<&str>) -> egui::Response {
    button_frame(
        ui,
        label,
        icon,
        inner_fill(),
        theme::border(),
        theme::text(),
        13.5,
        26.0,
    )
}

fn avatar(painter: &egui::Painter, rect: Rect, name: &str, agent: bool, online: Option<bool>) {
    let hue = name.bytes().fold(7u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(u32::from(b))
    });
    let fill = if agent {
        theme::mix(theme::bg_editor(), theme::accent(), 0.25)
    } else {
        let palette = [
            Color32::from_rgb(88, 120, 170),
            Color32::from_rgb(120, 96, 160),
            Color32::from_rgb(90, 140, 120),
            Color32::from_rgb(170, 110, 90),
            Color32::from_rgb(150, 120, 80),
        ];
        palette[hue as usize % palette.len()]
    };
    painter.circle_filled(rect.center(), rect.width() / 2.0, fill);
    if agent {
        project_mark(painter, rect.center(), rect.width() * 0.5, theme::accent());
    } else {
        let initials: String = name
            .split_whitespace()
            .filter_map(|part| part.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            initials,
            FontId::proportional(rect.width() * 0.38),
            Color32::WHITE,
        );
    }
    if online == Some(true) {
        let dot = rect.right_bottom() - vec2(6.0, 6.0);
        painter.circle_filled(dot, 6.0, theme::bg_editor());
        painter.circle_filled(dot, 4.5, Color32::from_rgb(240, 113, 120));
    }
}

/// The Forge Room project mark: a chevron-shaped "A" built from strokes.
fn project_mark(painter: &egui::Painter, center: egui::Pos2, size: f32, color: Color32) {
    let half = size / 2.0;
    let stroke = Stroke::new((size * 0.13).max(2.0), color);
    painter.add(egui::Shape::line(
        vec![
            center + vec2(-half, half * 0.8),
            center + vec2(0.0, -half * 0.9),
            center + vec2(half, half * 0.8),
        ],
        stroke,
    ));
    painter.add(egui::Shape::line(
        vec![
            center + vec2(-half * 0.45, half * 0.55),
            center + vec2(0.0, -half * 0.15),
            center + vec2(half * 0.45, half * 0.55),
        ],
        stroke,
    ));
}

// ---- Dates (UTC) ----------------------------------------------------------------------

fn civil(epoch: u64) -> (i64, u32, u32, u32, u32) {
    let days = (epoch / 86_400) as i64;
    let seconds = epoch % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { year + 1 } else { year };
    (
        year,
        month,
        day,
        (seconds / 3600) as u32,
        (seconds / 60 % 60) as u32,
    )
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn format_time(epoch: u64) -> String {
    let (_, month, day, hour, minute) = civil(epoch);
    let (twelve, suffix) = match hour {
        0 => (12, "AM"),
        1..=11 => (hour, "AM"),
        12 => (12, "PM"),
        _ => (hour - 12, "PM"),
    };
    format!(
        "{} {day}, {twelve}:{minute:02} {suffix} (UTC)",
        MONTHS[(month - 1) as usize]
    )
}

fn format_short(epoch: u64) -> String {
    let (_, month, day, hour, minute) = civil(epoch);
    format!(
        "{} {day}, {hour:02}:{minute:02}",
        MONTHS[(month - 1) as usize]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_match_known_epochs() {
        // 2026-11-02 10:00:00 UTC
        assert_eq!(civil(1_793_613_600), (2026, 11, 2, 10, 0));
        assert_eq!(format_time(1_793_613_600), "Nov 2, 10:00 AM (UTC)");
        assert_eq!(format_short(0), "Jan 1, 00:00");
        assert_eq!(civil(951_782_400).1, 2); // 2000-02-29 exists
        assert_eq!(civil(951_782_400).2, 29);
    }

    #[test]
    fn default_lobby_has_five_milestones_in_order() {
        let config = LobbyConfig::default();
        assert_eq!(config.milestones.len(), 5);
        assert!(
            config
                .milestones
                .windows(2)
                .all(|w| w[0].due_epoch <= w[1].due_epoch)
        );
    }
}
