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
    /// The countdown only runs once someone starts it; a new room has no clock.
    pub timer_running: bool,
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

/// Longest time box the lobby accepts (ten years); keeps date arithmetic and the DragValue sane.
const MAX_DURATION_HOURS: u32 = 24 * 365 * 10;

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
            timer_running: false,
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
    pub room_unavailable: bool,
    pub acting_as: &'a str,
}

pub struct Lobby {
    config: LobbyConfig,
    path: Option<PathBuf>,
    /// Edits not yet safely on disk; stays set until a save succeeds.
    dirty: bool,
    save_error: Option<String>,
    last_save_attempt: Option<std::time::Instant>,
    tab: Tab,
    invite: String,
    invite_role: usize,
    invite_note: Option<String>,
    /// Set when a checklist item was just added, so its text field takes focus.
    focus_new_goal: bool,
    /// Focus the recovery action once when an unreadable room is first shown.
    recovery_warning_shown: bool,
}

impl Default for Lobby {
    fn default() -> Self {
        Self {
            config: LobbyConfig::default(),
            path: None,
            dirty: false,
            save_error: None,
            last_save_attempt: None,
            tab: Tab::Lobby,
            invite: String::new(),
            invite_role: 0,
            invite_note: None,
            focus_new_goal: false,
            recovery_warning_shown: false,
        }
    }
}

fn now() -> u64 {
    crate::room_state::now()
}

fn card_fill() -> Color32 {
    theme::mix(theme::bg_editor(), theme::text(), 0.02)
}

fn header_fill() -> Color32 {
    theme::mix(theme::bg_editor(), theme::text(), 0.05)
}

/// Warm amber used for the countdown and in-progress marks, apart from the primary accent.
const AMBER: Color32 = Color32::from_rgb(244, 157, 120);

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
    /// Unsaved edits are written first; the error says why they could not be kept.
    pub fn load_for(&mut self, path: Option<PathBuf>) -> Result<(), String> {
        // Unsaved edits that cannot be written must not be replaced: the switch is refused.
        self.finish()?;
        let config = path.as_deref().and_then(load_config);
        *self = Self {
            config: config.unwrap_or_default(),
            path,
            ..Self::default()
        };
        Ok(())
    }

    /// Save pending edits now (leaving the project or quitting).
    pub fn finish(&mut self) -> Result<(), String> {
        if self.dirty { self.save() } else { Ok(()) }
    }

    /// Retry pending edits from the app loop, whichever page is showing.
    pub fn flush_pending(&mut self) {
        self.flush();
    }

    /// Write the settings next to the room file. The new content goes to a temporary file first and
    /// replaces the old one in a single step, so a failed or interrupted save never leaves a
    /// truncated settings file.
    fn save(&mut self) -> Result<(), String> {
        let Some(path) = &self.path else {
            self.dirty = false;
            return Ok(());
        };
        let write = || -> std::io::Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let bytes = serde_json::to_vec_pretty(&self.config).map_err(std::io::Error::other)?;
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, bytes)?;
            std::fs::rename(&temporary, path)
        };
        match write() {
            Ok(()) => {
                self.dirty = false;
                self.save_error = None;
                Ok(())
            }
            Err(error) => {
                let message = format!("Could not save the lobby settings: {error}");
                self.save_error = Some(message.clone());
                Err(message)
            }
        }
    }

    /// Save pending edits, retrying a failed save at most once a second.
    fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        let due = self
            .last_save_attempt
            .is_none_or(|at| at.elapsed() >= std::time::Duration::from_secs(1));
        if due {
            self.last_save_attempt = Some(std::time::Instant::now());
            let _ = self.save();
        }
    }

    /// When the room's time box ends. A damaged or hand-edited settings file can hold any numbers, so
    /// this saturates instead of overflowing.
    fn end_epoch(&self) -> u64 {
        let hours = u64::from(self.config.duration_hours.min(MAX_DURATION_HOURS));
        self.config.start_epoch.saturating_add(hours * 3600)
    }

    /// Seconds left in the room's time box.
    pub fn remaining(&self) -> u64 {
        self.end_epoch().saturating_sub(now())
    }

    /// True while a started timer still has time left; this is what the title bar shows.
    pub fn timer_active(&self) -> bool {
        self.config.timer_running && self.remaining() > 0
    }

    /// Start the time box now. Milestone dates were laid out from the old start, so they move with it.
    fn start_timer(&mut self) {
        let started = now();
        let shift = started.saturating_sub(self.config.start_epoch);
        for milestone in &mut self.config.milestones {
            milestone.due_epoch = milestone.due_epoch.saturating_add(shift);
        }
        self.config.start_epoch = started;
        self.config.timer_running = true;
    }

    pub fn kind_label(&self) -> &str {
        &self.config.kind
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
        let focus_recovery = context.room_unavailable && !self.recovery_warning_shown;
        self.recovery_warning_shown = context.room_unavailable;
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));

        self.pulse(ui, context);
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(Margin {
                        left: 26,
                        right: 18,
                        top: 10,
                        bottom: 10,
                    })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                        if let Some(error) = &self.save_error {
                            ui.label(
                                RichText::new(format!(
                                    "{error} Your edits are kept and saving will be retried."
                                ))
                                .color(Color32::from_rgb(240, 113, 120)),
                            );
                        }
                        if context.room_unavailable {
                            egui::Frame::new()
                                .fill(theme::mix(theme::bg_sidebar(), Color32::from_rgb(240, 113, 120), 0.12))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(240, 113, 120)))
                                .corner_radius(6.0)
                                .inner_margin(Margin::symmetric(12, 10))
                                .show(ui, |ui| {
                                    ui.label("Room unavailable: saved room files could not be read. They were left unchanged.");
                                    let details = ui.add(egui::Button::new("Open Room details"));
                                    if focus_recovery {
                                        details.request_focus();
                                    }
                                    if details.clicked() {
                                        action = Some(Action::EnterRoom);
                                    }
                                });
                        }
                        self.header(ui, context, &mut action);
                        self.tabs(ui);
                        ui.add_space(2.0);
                        self.body(ui, context, &mut action);
                    });
            });

        if serde_json::to_string(&self.config).unwrap_or_default() != before {
            self.dirty = true;
            self.last_save_attempt = None;
        }
        self.flush();
        action
    }

    // ---- Room Pulse ---------------------------------------------------------------------

    fn pulse(&mut self, ui: &mut Ui, context: &Context) {
        let items = self.setup_items(context);
        egui::Frame::new()
            .outer_margin(Margin::symmetric(5, 4))
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(6.0)
            .inner_margin(Margin::symmetric(12, 5))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(28.0);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    icons::draw(ui.painter(), "pulse", rect.center(), 18.0, theme::accent());
                    ui.label(RichText::new("Room Pulse").strong().color(theme::text()));
                    ui.add_space(8.0);
                    if let Some(state) = context.state {
                        let clock = now();
                        if state.activity.is_empty() {
                            ui.label(RichText::new("No activity yet").color(theme::text_dim()));
                        }
                        // An overflowing row would also widen everything laid out below it, so only
                        // as many recent events as fit (about 230 px each) are shown.
                        let slots = ((ui.available_width() - 430.0) / 230.0).floor().max(0.0);
                        let shown = (slots as usize).min(4);
                        for event in state.activity.iter().rev().take(shown) {
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
                    } else if context.room_unavailable {
                        ui.label(
                            RichText::new("Room unavailable. Open Room for details.")
                                .color(theme::text_dim()),
                        );
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
                            let amber = Color32::from_rgb(226, 160, 60);
                            let galley = ui.painter().layout_no_wrap(
                                label,
                                FontId::proportional(14.0),
                                theme::text(),
                            );
                            let review = ui.painter().layout_no_wrap(
                                "Review".into(),
                                FontId::proportional(14.0),
                                theme::text(),
                            );
                            let width = 14.0
                                + 20.0
                                + 8.0
                                + galley.size().x
                                + 14.0
                                + review.size().x
                                + 22.0
                                + 6.0;
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(width, 34.0), Sense::hover());
                            let painter = ui.painter();
                            painter.rect_filled(
                                rect,
                                8.0,
                                Color32::from_rgba_unmultiplied(226, 160, 60, 30),
                            );
                            painter.rect_stroke(
                                rect,
                                8.0,
                                Stroke::new(
                                    1.0,
                                    Color32::from_rgba_unmultiplied(226, 160, 60, 140),
                                ),
                                egui::StrokeKind::Inside,
                            );
                            icons::draw(
                                painter,
                                "warning",
                                egui::pos2(rect.left() + 24.0, rect.center().y),
                                20.0,
                                amber,
                            );
                            painter.galley(
                                egui::pos2(
                                    rect.left() + 42.0,
                                    rect.center().y - galley.size().y / 2.0,
                                ),
                                galley.clone(),
                                theme::text(),
                            );
                            let button = Rect::from_min_max(
                                egui::pos2(
                                    rect.left() + 42.0 + galley.size().x + 12.0,
                                    rect.top() + 4.0,
                                ),
                                egui::pos2(rect.right() - 4.0, rect.bottom() - 4.0),
                            );
                            let response =
                                ui.interact(button, ui.scope_id().with("review"), Sense::click());
                            let painter = ui.painter();
                            painter.rect_filled(
                                button,
                                6.0,
                                if response.hovered() {
                                    theme::overlay(40)
                                } else {
                                    theme::overlay(22)
                                },
                            );
                            painter.galley(
                                button.center() - review.size() / 2.0,
                                review,
                                theme::text(),
                            );
                            if response.clicked() {
                                self.tab = Tab::Setup;
                            }
                        }
                    });
                });
            });
    }

    // ---- Header -------------------------------------------------------------------------

    fn header(&mut self, ui: &mut Ui, context: &Context, action: &mut Option<Action>) {
        let width = ui.available_width();
        // The countdown, Enter Room and the more button keep their size; the project identity
        // takes whatever is left, and the countdown is dropped when even that would be cramped.
        let show_countdown = width >= 960.0;
        let right_width = if show_countdown { 242.0 + 22.0 } else { 0.0 } + 165.0 + 22.0 + 40.0;
        let left_width = (width - right_width - 22.0).max(260.0);
        egui::Frame::new().show(ui, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 22.0;
                ui.allocate_ui_with_layout(
                    vec2(left_width, 92.0),
                    Layout::left_to_right(Align::TOP),
                    |ui| {
                        ui.set_width(left_width);
                        ui.spacing_mut().item_spacing.x = 22.0;
                        self.identity(ui, context, left_width);
                    },
                );
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    if square_button(ui, "···")
                        .on_hover_text("Open Lobby settings")
                        .clicked()
                    {
                        self.tab = Tab::Settings;
                    }
                    if accent_button(ui, "Enter Room", Some("arrow-right")).clicked() {
                        *action = Some(Action::EnterRoom);
                    }
                    if show_countdown {
                        self.countdown(ui);
                    }
                });
            });
        });
    }

    /// Project mark, name, tagline and tags, sized to `width`.
    fn identity(&mut self, ui: &mut Ui, context: &Context, width: f32) {
        let (mark, _) = ui.allocate_exact_size(vec2(103.0, 84.0), Sense::hover());
        ui.painter().rect_filled(
            mark,
            7.0,
            theme::mix(theme::bg_editor(), theme::accent(), 0.10),
        );
        ui.painter().rect_stroke(
            mark,
            7.0,
            Stroke::new(1.0, theme::mix(theme::bg_editor(), theme::accent(), 0.4)),
            egui::StrokeKind::Inside,
        );
        project_mark(ui.painter(), mark.center(), 58.0, theme::accent());

        let title_width = 210.0;
        ui.vertical(|ui| {
            ui.set_max_width(title_width);
            ui.label(
                RichText::new(crate::rooms::short(context.workspace, 14))
                    .size(32.0)
                    .family(crate::fonts::semibold())
                    .color(theme::text()),
            )
            .on_hover_text(context.workspace);
            ui.label(
                RichText::new("Project Lobby")
                    .size(20.0)
                    .color(theme::text_dim()),
            );
        });

        let rest = (width - 103.0 - 22.0 - title_width - 22.0).max(120.0);
        ui.vertical(|ui| {
            ui.set_width(rest);
            ui.add_space(6.0);
            // With too little room the tagline is dropped instead of running under the buttons.
            if rest >= 170.0 {
                let tagline = &mut self.config.tagline;
                ui.add(
                    egui::TextEdit::singleline(tagline)
                        .hint_text("One line about this room")
                        .frame(egui::Frame::NONE)
                        .desired_width(rest.min(420.0))
                        .font(FontId::proportional(16.0)),
                );
                ui.add_space(4.0);
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
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
    }

    fn countdown(&mut self, ui: &mut Ui) {
        let remaining = if self.config.timer_running {
            self.remaining()
        } else {
            0
        };
        let (hours, minutes, seconds) = (remaining / 3600, remaining / 60 % 60, remaining % 60);
        let size = vec2(242.0, 83.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        {
            let painter = ui.painter();
            painter.rect_filled(rect, 6.0, card_fill());
            painter.rect_stroke(
                rect,
                6.0,
                Stroke::new(1.0, theme::border()),
                egui::StrokeKind::Inside,
            );
        }
        if !self.config.timer_running {
            ui.painter().text(
                rect.center_top() + vec2(0.0, 18.0),
                Align2::CENTER_CENTER,
                format!("{}-hour time box", self.config.duration_hours),
                FontId::proportional(13.0),
                theme::text(),
            );
            ui.painter().text(
                rect.center_top() + vec2(0.0, 40.0),
                Align2::CENTER_CENTER,
                "No timer running",
                FontId::proportional(14.0),
                theme::text_dim(),
            );
            let button =
                Rect::from_center_size(rect.center_bottom() - vec2(0.0, 17.0), vec2(120.0, 26.0));
            let response = ui.interact(button, ui.scope_id().with("start-timer"), Sense::click());
            let fill = if response.hovered() {
                theme::mix(theme::button(), Color32::WHITE, 0.08)
            } else {
                theme::button()
            };
            ui.painter().rect_filled(button, 5.0, fill);
            ui.painter().text(
                button.center(),
                Align2::CENTER_CENTER,
                "Start timer",
                FontId::proportional(13.0),
                theme::button_text(),
            );
            if response.on_hover_text("Start the countdown now").clicked() {
                self.start_timer();
            }
            return;
        }
        let painter = ui.painter();
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
        // A small close mark in the corner stops the timer.
        let stop = Rect::from_center_size(rect.right_top() + vec2(-14.0, 14.0), vec2(20.0, 20.0));
        let stop_response = ui.interact(stop, ui.scope_id().with("stop-timer"), Sense::click());
        icons::draw(
            ui.painter(),
            "close",
            stop.center(),
            13.0,
            if stop_response.hovered() {
                theme::text()
            } else {
                theme::text_dim()
            },
        );
        if stop_response.on_hover_text("Stop the timer").clicked() {
            self.config.timer_running = false;
        }
        let painter = ui.painter();
        for (index, (value, unit)) in [(hours, "HRS"), (minutes, "MIN"), (seconds, "SEC")]
            .into_iter()
            .enumerate()
        {
            let x = rect.left() + 44.0 + index as f32 * 77.0;
            painter.text(
                egui::pos2(x, rect.center().y + 6.0),
                Align2::CENTER_CENTER,
                format!("{value:02}"),
                FontId::new(32.0, crate::fonts::semibold()),
                AMBER,
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
                    egui::pos2(x + 38.0, rect.center().y + 4.0),
                    Align2::CENTER_CENTER,
                    ":",
                    FontId::proportional(28.0),
                    AMBER,
                );
            }
        }
    }

    fn tabs(&mut self, ui: &mut Ui) {
        // Comfortable tabs (at least 104 px) when they fit; otherwise just the label plus padding,
        // so the row never makes the page wider than the window.
        let available = ui.available_width();
        let widths = |padding: f32, minimum: f32| -> Vec<f32> {
            TABS.iter()
                .map(|(_, label)| {
                    let galley = ui.painter().layout_no_wrap(
                        (*label).to_string(),
                        FontId::proportional(15.0),
                        theme::text(),
                    );
                    (galley.size().x + padding).max(minimum)
                })
                .collect()
        };
        let mut sizes = widths(52.0, 104.0);
        if sizes.iter().sum::<f32>() + 4.0 * TABS.len() as f32 > available {
            sizes = widths(24.0, 0.0);
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for ((tab, label), tab_width) in TABS.into_iter().zip(sizes) {
                let selected = self.tab == tab;
                let response = ui.add_sized(
                    vec2(tab_width, 37.0),
                    egui::Button::new(RichText::new(label).size(15.0).color(if selected {
                        theme::text()
                    } else {
                        theme::text_dim()
                    }))
                    .fill(if selected {
                        theme::tab_active()
                    } else {
                        card_fill()
                    })
                    .stroke(Stroke::NONE),
                );
                let rect = response.rect;
                if self.tab == tab {
                    ui.painter().line_segment(
                        [rect.left_bottom(), rect.right_bottom()],
                        Stroke::new(2.0, theme::accent()),
                    );
                }
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
        let tab = self.tab;
        // Below this the two columns would both be cramped, so the second goes under the first.
        let stacked = total < 900.0;
        let right = if tab == Tab::Milestones {
            0.0
        } else if stacked {
            total
        } else {
            ((total - gap) * 0.33).clamp(300.0, 520.0)
        };
        let left = if right == 0.0 || stacked {
            total
        } else {
            total - right - gap
        };
        if stacked {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(gap, 12.0);
                self.left_column(ui, context, action, left);
                if right > 0.0 {
                    self.right_column(ui, context, right);
                }
            });
            return;
        }
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, 12.0);
            ui.allocate_ui_with_layout(vec2(left, 10.0), Layout::top_down(Align::Min), |ui| {
                self.left_column(ui, context, action, left);
            });
            ui.allocate_ui_with_layout(vec2(right, 10.0), Layout::top_down(Align::Min), |ui| {
                self.right_column(ui, context, right);
            });
        });
    }

    fn left_column(
        &mut self,
        ui: &mut Ui,
        context: &Context,
        action: &mut Option<Action>,
        left: f32,
    ) {
        let tab = self.tab;
        ui.set_width(left);
        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
        if matches!(tab, Tab::Lobby | Tab::Settings) {
            self.overview(ui, context);
        }
        if matches!(tab, Tab::Lobby | Tab::Team | Tab::Agents) {
            let both = tab == Tab::Lobby;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                let team_width = if both { (left - 12.0) * 0.48 } else { left };
                let agents_width = if both { left - 12.0 - team_width } else { left };
                if tab != Tab::Agents {
                    ui.allocate_ui_with_layout(
                        vec2(team_width, 10.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(team_width);
                            self.team(ui, context, action);
                        },
                    );
                }
                if tab != Tab::Team {
                    ui.allocate_ui_with_layout(
                        vec2(agents_width, 10.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(agents_width);
                            self.agents(ui, context, action);
                        },
                    );
                }
            });
        }
        if matches!(tab, Tab::Lobby | Tab::Milestones) {
            self.milestones(ui);
        }
    }

    fn right_column(&mut self, ui: &mut Ui, context: &Context, right: f32) {
        let tab = self.tab;
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
                    let goal_width = (width + 24.0) * 0.503;
                    let half = width - 14.0 - goal_width;
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
                                None if context.room_unavailable => "Needs recovery",
                                None => "Not created yet",
                            };
                            let start = self.config.start_epoch;
                            let end = self.end_epoch();
                            kv(ui, "room", "Room Name", |ui| {
                                ui.label(format!("{} {}", context.workspace, self.config.kind));
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
                                ui.scope(|ui| {
                                    let widgets = &mut ui.visuals_mut().widgets;
                                    for state in [&mut widgets.inactive, &mut widgets.hovered] {
                                        state.bg_fill = Color32::TRANSPARENT;
                                        state.weak_bg_fill = Color32::TRANSPARENT;
                                        state.bg_stroke = Stroke::NONE;
                                    }
                                    ui.add(
                                        egui::DragValue::new(&mut self.config.duration_hours)
                                            .range(1..=MAX_DURATION_HOURS)
                                            .suffix(" Hours"),
                                    )
                                    .on_hover_text("Drag or click to change");
                                });
                            });
                            let running = self.config.timer_running;
                            kv(ui, "calendar", "Start Time", |ui| {
                                ui.label(if running {
                                    format_time(start)
                                } else {
                                    "Not started".to_string()
                                });
                            });
                            kv(ui, "calendar", "End Time", |ui| {
                                ui.label(if running {
                                    format_time(end)
                                } else {
                                    "Not started".to_string()
                                });
                            });
                        },
                    );
                    ui.allocate_ui_with_layout(
                        vec2(goal_width, 10.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(goal_width);
                            egui::Frame::new()
                                .fill(inner_fill())
                                .stroke(Stroke::new(1.0, theme::border()))
                                .corner_radius(6.0)
                                .inner_margin(Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.spacing_mut().item_spacing.y = 0.0;
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
                                        ui.label(
                                            RichText::new("Project Goal")
                                                .size(16.0)
                                                .family(crate::fonts::semibold()),
                                        );
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if icon_button(ui, "plus", "Add a checklist item")
                                                    .clicked()
                                                {
                                                    self.config.goals.push((String::new(), false));
                                                    self.focus_new_goal = true;
                                                }
                                            },
                                        );
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
                                    let last = self.config.goals.len().saturating_sub(1);
                                    let focus_new = std::mem::take(&mut self.focus_new_goal);
                                    for (index, (text, done)) in
                                        self.config.goals.iter_mut().enumerate()
                                    {
                                        ui.horizontal(|ui| {
                                            ui.checkbox(done, "").accessible_name(format!(
                                                "Complete checklist item {}: {}",
                                                index + 1,
                                                text
                                            ));
                                            let response = ui.add(
                                                egui::TextEdit::singleline(text)
                                                    .frame(egui::Frame::NONE)
                                                    .font(FontId::proportional(13.0))
                                                    .desired_width(f32::INFINITY),
                                            );
                                            if focus_new && index == last {
                                                response.request_focus();
                                            }
                                            // An item left empty when the cursor moves away is dropped.
                                            if response.lost_focus() && text.trim().is_empty() {
                                                remove = Some(index);
                                            }
                                        });
                                    }
                                    if let Some(index) = remove {
                                        self.config.goals.remove(index);
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
        let rows = context
            .state
            .map(|state| {
                let agents = state
                    .participants
                    .iter()
                    .filter(|p| p.role == Role::Agent)
                    .count();
                (state.participants.len() - agents).max(agents)
            })
            .unwrap_or(0)
            .clamp(2, 6);
        let min_height = 36.0 + 12.0 + rows as f32 * 51.0;
        let add = std::cell::Cell::new(false);
        card_sized(
            ui,
            "users",
            &title,
            min_height,
            |ui| {
                if outline_button(ui, "+ Add Member").clicked() {
                    add.set(true);
                }
            },
            |ui| {
                if context.state.is_none() {
                    if context.room_unavailable {
                        ui.label(
                            RichText::new("Room files need recovery before members can be added.")
                                .color(theme::text_dim()),
                        );
                    } else {
                        ui.label(
                            RichText::new("Create the room to add people.")
                                .color(theme::text_dim()),
                        );
                        if accent_button(ui, "Create Room", None).clicked() {
                            *action = Some(Action::CreateRoom);
                        }
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
        let rows = context
            .state
            .map(|state| {
                let agents = state
                    .participants
                    .iter()
                    .filter(|p| p.role == Role::Agent)
                    .count();
                (state.participants.len() - agents).max(agents)
            })
            .unwrap_or(0)
            .clamp(2, 6);
        let min_height = 36.0 + 12.0 + rows as f32 * 51.0;
        let add = std::cell::Cell::new(false);
        card_sized(
            ui,
            "bot",
            &title,
            min_height,
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
            let (rect, _) = ui.allocate_exact_size(vec2(41.0, 41.0), Sense::hover());
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
                                ui.allocate_exact_size(vec2(track, 88.0), Sense::hover());
                            let step = track / count as f32;
                            let line_y = rect.top() + 16.0;
                            let reached = self
                                .config
                                .milestones
                                .iter()
                                .rposition(|m| m.status != MilestoneStatus::Pending);
                            ui.painter().line_segment(
                                [
                                    egui::pos2(rect.left() + step / 2.0, line_y),
                                    egui::pos2(rect.right() - step / 2.0, line_y),
                                ],
                                Stroke::new(
                                    2.0,
                                    theme::mix(theme::border(), theme::text_dim(), 0.6),
                                ),
                            );
                            if let Some(last) = reached {
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(rect.left() + step / 2.0, line_y),
                                        egui::pos2(
                                            rect.left() + step * (last as f32 + 0.5),
                                            line_y,
                                        ),
                                    ],
                                    Stroke::new(2.0, theme::accent()),
                                );
                            }
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
                                        painter.circle_filled(center, 10.0, theme::accent());
                                        icons::draw(painter, "check", center, 16.0, Color32::BLACK);
                                    }
                                    MilestoneStatus::InProgress => {
                                        painter.circle_filled(center, 10.0, theme::bg_editor());
                                        painter.circle_stroke(
                                            center,
                                            13.0,
                                            Stroke::new(3.0, AMBER),
                                        );
                                        painter.circle_filled(center, 4.5, AMBER);
                                    }
                                    MilestoneStatus::Pending => {
                                        painter.circle_filled(center, 10.0, theme::bg_editor());
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
                                    MilestoneStatus::InProgress => {
                                        (format!("In Progress ({}%)", milestone.percent), AMBER)
                                    }
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
                                            vec2(ui.available_width() - 44.0, 14.0),
                                            Sense::hover(),
                                        );
                                        ui.painter().rect_filled(bar, 7.0, theme::border());
                                        let mut fill = bar;
                                        fill.set_width(bar.width() * percent);
                                        ui.painter().rect_filled(fill, 7.0, theme::accent());
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
                            // Only http(s) addresses ever reach the browser.
                            let target = crate::preview::web_url(&url);
                            let clicked =
                                icon_button(ui, "external-link", "Open in browser").clicked();
                            if clicked && let Some(full) = target {
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
                ui.spacing_mut().item_spacing.y = 3.0;
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
                // Beside its label when there is room; under it when the card is narrow, so the
                // long option names are never cut off.
                let beside = ui.available_width() >= 440.0;
                let mut sharing = |ui: &mut Ui| {
                    for (name, on) in SHARING.iter().zip(self.config.sharing.iter_mut()) {
                        ui.checkbox(on, *name);
                    }
                };
                let heading = |ui: &mut Ui| {
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
                };
                if beside {
                    ui.horizontal_top(|ui| {
                        heading(ui);
                        ui.add_space(10.0);
                        ui.vertical(&mut sharing);
                    });
                } else {
                    ui.horizontal(heading);
                    ui.horizontal(|ui| {
                        ui.add_space(32.0);
                        ui.vertical(&mut sharing);
                    });
                }
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
                let width = ui.available_width();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let input = (width - 85.0 - 78.0 - 16.0).max(60.0);
                    ui.add_sized(
                        [input, 28.0],
                        egui::TextEdit::singleline(&mut self.invite)
                            .hint_text("Enter email or team code...")
                            .align(egui::Align2::LEFT_CENTER),
                    );
                    let roles = ["Member", "Viewer", "Agent owner"];
                    egui::ComboBox::from_id_salt("invite-role")
                        .selected_text(roles[self.invite_role])
                        .width(50.0)
                        .show_ui(ui, |ui| {
                            for (index, role) in roles.iter().enumerate() {
                                ui.selectable_value(&mut self.invite_role, index, *role);
                            }
                        });
                    if compact_button(ui, "Invite", Some("send"), 78.0).clicked() {
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
    card_sized(ui, icon, title, 0.0, right, body);
}

/// A card at least `min_height` tall, so paired cards can share a bottom edge.
fn card_sized(
    ui: &mut Ui,
    icon: &str,
    title: &str,
    min_height: f32,
    right: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui),
) {
    egui::Frame::new()
        .fill(card_fill())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(9.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(min_height);
            ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
            egui::Frame::new()
                .fill(header_fill())
                .corner_radius(egui::CornerRadius {
                    nw: 9,
                    ne: 9,
                    sw: 0,
                    se: 0,
                })
                .inner_margin(Margin::symmetric(14, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(28.0);
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                        icons::draw(ui.painter(), icon, rect.center(), 18.0, theme::text_dim());
                        ui.label(
                            RichText::new(title)
                                .size(16.0)
                                .family(crate::fonts::semibold())
                                .color(theme::text()),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), right);
                    });
                });
            let line = ui.max_rect();
            let y = ui.cursor().top();
            ui.painter().line_segment(
                [egui::pos2(line.left(), y), egui::pos2(line.right(), y)],
                Stroke::new(1.0, theme::border()),
            );
            egui::Frame::new()
                .inner_margin(Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
                    body(ui);
                });
        });
}

/// A labelled row: icon and label on the left, the value widget in a fixed column.
fn kv(ui: &mut Ui, icon: &str, label: &str, value: impl FnOnce(&mut Ui)) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
    let painter = ui.painter();
    icons::draw(
        painter,
        icon,
        egui::pos2(rect.left() + 10.0, rect.center().y),
        17.0,
        theme::text_dim(),
    );
    painter.text(
        egui::pos2(rect.left() + 32.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.5),
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
        .layout_no_wrap(text.to_string(), FontId::proportional(13.0), color);
    let (rect, _) = ui.allocate_exact_size(galley.size() + vec2(38.0, 10.0), Sense::hover());
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
    let response = ui
        .add_sized(vec2(24.0, 24.0), egui::Button::new("").frame(false))
        .accessible_name(hint);
    icons::draw(
        ui.painter(),
        icon,
        response.rect.center(),
        18.0,
        if response.hovered() {
            theme::text()
        } else {
            theme::text_dim()
        },
    );
    response.on_hover_text(hint)
}

struct ButtonStyle {
    fill: Color32,
    stroke: Color32,
    text: Color32,
    size: f32,
    height: f32,
    min_width: f32,
    radius: f32,
    /// Horizontal space around the label (and the extra width taken by an icon).
    pad: f32,
}

fn button_frame(
    ui: &mut Ui,
    label: &str,
    icon: Option<&str>,
    style: ButtonStyle,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        FontId::proportional(style.size),
        style.text,
    );
    let extra = if icon.is_some() { style.pad + 2.0 } else { 0.0 };
    let width = (galley.size().x + style.pad + extra).max(style.min_width);
    let response = ui.add_sized(
        vec2(width, style.height),
        egui::Button::new(RichText::new(label).size(style.size).color(style.text))
            .fill(style.fill)
            .stroke(Stroke::new(1.0, style.stroke))
            .corner_radius(style.radius),
    );
    if let Some(icon) = icon {
        icons::draw(
            ui.painter(),
            icon,
            egui::pos2(response.rect.right() - 16.0, response.rect.center().y),
            18.0,
            style.text,
        );
    }
    response
}

/// The main call to action, painted in the theme's button colors.
fn accent_button(ui: &mut Ui, label: &str, icon: Option<&str>) -> egui::Response {
    button_frame(
        ui,
        label,
        icon,
        ButtonStyle {
            fill: theme::button(),
            stroke: theme::button(),
            text: theme::button_text(),
            size: 16.0,
            height: 45.0,
            min_width: 165.0,
            radius: 6.0,
            pad: 28.0,
        },
    )
}

/// The smaller filled button used inside cards.
fn compact_button(ui: &mut Ui, label: &str, icon: Option<&str>, min_width: f32) -> egui::Response {
    button_frame(
        ui,
        label,
        icon,
        ButtonStyle {
            fill: theme::button(),
            stroke: theme::button(),
            text: theme::button_text(),
            size: 14.0,
            height: 31.0,
            min_width,
            radius: 5.0,
            pad: 14.0,
        },
    )
}

fn outline_button(ui: &mut Ui, label: &str) -> egui::Response {
    button_frame(
        ui,
        label,
        None,
        ButtonStyle {
            fill: accent_soft(),
            stroke: theme::accent(),
            text: theme::text(),
            size: 12.5,
            height: 27.0,
            min_width: 0.0,
            radius: 4.0,
            pad: 28.0,
        },
    )
}

/// The "more" button next to Enter Room, square and level with it.
fn square_button(ui: &mut Ui, label: &str) -> egui::Response {
    button_frame(
        ui,
        label,
        None,
        ButtonStyle {
            fill: inner_fill(),
            stroke: theme::border(),
            text: theme::text(),
            size: 14.0,
            height: 40.0,
            min_width: 40.0,
            radius: 6.0,
            pad: 28.0,
        },
    )
}

fn avatar(painter: &egui::Painter, rect: Rect, name: &str, agent: bool, online: Option<bool>) {
    let hue = name.bytes().fold(7u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(u32::from(b))
    });
    let fill = if agent {
        Color32::from_rgb(41, 37, 55)
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
        project_mark(
            painter,
            rect.center(),
            rect.width() * 0.55,
            Color32::from_rgb(158, 123, 232),
        );
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

/// The Forge Room project mark.
fn project_mark(painter: &egui::Painter, center: egui::Pos2, size: f32, color: Color32) {
    let _ = color;
    crate::mark::draw(painter, center, size);
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

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "forge-room-lobby-{}-{}-{name}.json",
            std::process::id(),
            crate::room_state::now()
        ))
    }

    #[test]
    fn saved_settings_round_trip() {
        let path = temp_path("roundtrip");
        let mut lobby = Lobby::default();
        let _ = lobby.load_for(Some(path.clone()));
        lobby.config.goal = "Ship the MVP".into();
        lobby.config.goals.push(("Demo".into(), true));
        lobby.config.milestones[1].status = MilestoneStatus::Done;
        lobby.config.labels.insert("ada".into(), "Lead".into());
        lobby.save().unwrap();

        let mut reopened = Lobby::default();
        let _ = reopened.load_for(Some(path.clone()));
        assert_eq!(reopened.config.goal, "Ship the MVP");
        assert_eq!(reopened.config.goals, vec![("Demo".to_string(), true)]);
        assert!(reopened.config.milestones[1].status == MilestoneStatus::Done);
        assert_eq!(
            reopened.config.labels.get("ada").map(String::as_str),
            Some("Lead")
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn partial_or_damaged_files_fall_back_to_defaults() {
        // A file from an older version with only some fields still loads.
        let partial = temp_path("partial");
        std::fs::write(&partial, r#"{"goal": "Only a goal"}"#).unwrap();
        let mut lobby = Lobby::default();
        let _ = lobby.load_for(Some(partial.clone()));
        assert_eq!(lobby.config.goal, "Only a goal");
        assert_eq!(lobby.config.milestones.len(), 5);
        assert_eq!(lobby.config.duration_hours, 24);
        let _ = std::fs::remove_file(partial);

        // Garbage is ignored rather than blocking the lobby.
        let damaged = temp_path("damaged");
        std::fs::write(&damaged, "{ not json").unwrap();
        let mut lobby = Lobby::default();
        let _ = lobby.load_for(Some(damaged.clone()));
        assert_eq!(lobby.config.duration_hours, 24);
        let _ = std::fs::remove_file(damaged);
    }

    #[test]
    fn extreme_saved_numbers_do_not_overflow() {
        let path = temp_path("extreme");
        std::fs::write(
            &path,
            r#"{"start_epoch": 18446744073709551615, "duration_hours": 4294967295}"#,
        )
        .unwrap();
        let mut lobby = Lobby::default();
        let _ = lobby.load_for(Some(path.clone()));
        assert_eq!(lobby.end_epoch(), u64::MAX);
        assert!(lobby.remaining() > 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_failed_save_keeps_the_edits_and_is_retried() {
        // A regular file where the parent directory should be makes every write fail.
        let blocker = temp_path("blocker");
        std::fs::write(&blocker, "not a directory").unwrap();
        let path = blocker.join("lobby.json");

        let mut lobby = Lobby::default();
        let _ = lobby.load_for(Some(path.clone()));
        lobby.config.goal = "important".into();
        lobby.dirty = true;
        assert!(lobby.save().is_err());
        assert!(lobby.dirty, "edits stay pending after a failed save");
        assert!(lobby.save_error.is_some());

        // Once the problem is fixed, the next attempt succeeds and clears the error.
        std::fs::remove_file(&blocker).unwrap();
        lobby.last_save_attempt = None;
        lobby.flush();
        assert!(!lobby.dirty);
        assert!(lobby.save_error.is_none());
        let mut reopened = Lobby::default();
        let _ = reopened.load_for(Some(path.clone()));
        assert_eq!(reopened.config.goal, "important");
        let _ = std::fs::remove_dir_all(blocker);
    }

    #[test]
    fn switching_projects_is_refused_while_edits_cannot_be_saved() {
        let blocker = temp_path("blocker-switch");
        std::fs::write(&blocker, "not a directory").unwrap();
        let mut lobby = Lobby::default();
        let _ = lobby.load_for(Some(blocker.join("lobby.json")));
        lobby.config.goal = "ONLY UNSAVED COPY".into();
        lobby.dirty = true;

        assert!(lobby.load_for(None).is_err());
        assert_eq!(
            lobby.config.goal, "ONLY UNSAVED COPY",
            "the edits must survive"
        );
        assert!(lobby.dirty);
        let _ = std::fs::remove_file(blocker);
    }

    #[test]
    fn countdown_never_goes_below_zero() {
        let mut lobby = Lobby::default();
        lobby.config.start_epoch = now().saturating_sub(48 * 3600);
        lobby.config.duration_hours = 24;
        assert_eq!(lobby.remaining(), 0);
        lobby.config.start_epoch = now();
        assert!(lobby.remaining() > 23 * 3600);
    }

    #[test]
    fn a_new_room_has_no_running_timer_until_started() {
        let mut lobby = Lobby::default();
        assert!(!lobby.timer_active());
        let first_due = lobby.config.milestones[1].due_epoch;
        lobby.config.start_epoch = now().saturating_sub(3 * 3600);
        lobby.config.milestones[1].due_epoch = lobby.config.start_epoch + 4 * 3600;
        let before = lobby.config.milestones[1].due_epoch;

        lobby.start_timer();
        assert!(lobby.timer_active());
        assert!(lobby.remaining() > 23 * 3600);
        // Milestones keep their offsets from the (moved) start.
        assert_eq!(
            lobby.config.milestones[1].due_epoch - lobby.config.start_epoch,
            4 * 3600
        );
        assert!(lobby.config.milestones[1].due_epoch > before);
        let _ = first_due;

        // A finished box is not "active" even though it was started.
        lobby.config.start_epoch = now().saturating_sub(48 * 3600);
        assert!(!lobby.timer_active());
    }
}
