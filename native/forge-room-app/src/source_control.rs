//! The Source Control sidebar: commit box, staged and unstaged lists, per-file actions.

use crate::{explorer, git, theme};
use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense, Stroke};

pub struct State {
    pub message: String,
    pub staged_open: bool,
    pub changes_open: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            message: String::new(),
            staged_open: true,
            changes_open: true,
        }
    }
}

pub enum Repo<'a> {
    NoFolder,
    NoRepository,
    Error(&'a str),
    Ready(&'a git::Status),
}

pub enum Action {
    None,
    Refresh,
    Init,
    Commit,
    Stage(Vec<String>),
    Unstage(Vec<String>),
    ShowDiff { path: String, staged: bool },
    OpenFile(String),
}

const ROW_HEIGHT: f32 = 26.0;

pub fn letter_color(letter: char) -> Color32 {
    match letter {
        'M' | 'T' => Color32::from_rgb(226, 192, 141),
        'A' | 'U' => Color32::from_rgb(129, 199, 132),
        'D' => Color32::from_rgb(240, 113, 120),
        'R' | 'C' => Color32::from_rgb(110, 170, 225),
        _ => Color32::from_rgb(255, 120, 100),
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut State, repo: Repo) -> Action {
    let mut action = Action::None;

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(
            RichText::new("SOURCE CONTROL")
                .size(11.0)
                .strong()
                .color(theme::TEXT_DIM),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            if !matches!(repo, Repo::NoFolder) && ui.small_button("Refresh").clicked() {
                action = Action::Refresh;
            }
        });
    });
    ui.add_space(8.0);

    match repo {
        Repo::NoFolder => empty_state(
            ui,
            "No folder open",
            "Open a folder from the Explorer to see its changes here.",
        ),
        Repo::Error(message) => empty_state(ui, "Git is unavailable", message),
        Repo::NoRepository => {
            empty_state(
                ui,
                "Not a Git repository",
                "Track changes, stage files and commit by turning this folder into a repository.",
            );
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                if ui.button("Initialize Repository").clicked() {
                    action = Action::Init;
                }
            });
        }
        Repo::Ready(status) => {
            if let Some(next) = ready(ui, state, status) {
                action = next;
            }
        }
    }
    action
}

fn empty_state(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_max_width((ui.available_width() - 14.0).max(120.0));
            ui.label(RichText::new(title).strong().color(theme::TEXT));
            ui.add_space(2.0);
            ui.label(RichText::new(detail).color(theme::TEXT_DIM));
            ui.add_space(8.0);
        });
    });
}

fn ready(ui: &mut egui::Ui, state: &mut State, status: &git::Status) -> Option<Action> {
    let mut action = None;
    let staged: Vec<&git::Change> = status.changes.iter().filter(|c| c.is_staged()).collect();
    let unstaged: Vec<&git::Change> = status.changes.iter().filter(|c| c.is_unstaged()).collect();

    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 0))
        .show(ui, |ui| {
            let editor = egui::TextEdit::multiline(&mut state.message)
                .hint_text("Message (Ctrl+Enter to commit)")
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(10, 8));
            let response = ui.add(editor);
            let can_commit = !staged.is_empty() && !state.message.trim().is_empty();
            let submit = response.has_focus()
                && ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter));
            ui.add_space(6.0);
            let label = if staged.is_empty() {
                "Commit".to_string()
            } else {
                format!("Commit {} staged", staged.len())
            };
            let button = egui::Button::new(RichText::new(label).strong().color(Color32::WHITE))
                .fill(theme::ACCENT)
                .corner_radius(6.0)
                .min_size(egui::vec2(ui.available_width(), 30.0));
            if (ui.add_enabled(can_commit, button).clicked() || (submit && can_commit))
                && action.is_none()
            {
                action = Some(Action::Commit);
            }
        });
    ui.add_space(10.0);

    if status.changes.is_empty() {
        empty_state(
            ui,
            "Working tree clean",
            "There is nothing to commit. Changes you make will show up here.",
        );
        return action;
    }

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            if !staged.is_empty()
                && let Some(next) =
                    section(ui, "STAGED CHANGES", &mut state.staged_open, &staged, true)
            {
                action = Some(next);
            }
            if !unstaged.is_empty()
                && let Some(next) =
                    section(ui, "CHANGES", &mut state.changes_open, &unstaged, false)
            {
                action = Some(next);
            }
        });
    action
}

/// One collapsible group. Returns the action requested by a click inside it.
fn section(
    ui: &mut egui::Ui,
    title: &str,
    open: &mut bool,
    changes: &[&git::Change],
    staged: bool,
) -> Option<Action> {
    let mut action = None;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let y = rect.center().y;
    let bulk = Rect::from_center_size(egui::pos2(rect.right() - 20.0, y), egui::vec2(22.0, 22.0));
    let bulk_response = ui.interact(bulk, ui.scope_id().with(("bulk", title)), Sense::click());
    {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 8));
        }
        crate::icons::chevron(
            painter,
            egui::pos2(rect.left() + 18.0, y),
            *open,
            theme::TEXT_DIM,
        );
        let galley = painter.layout_no_wrap(
            title.to_string(),
            FontId::proportional(11.0),
            theme::TEXT_DIM,
        );
        let width = galley.size().x;
        painter.galley(
            egui::pos2(rect.left() + 32.0, y - galley.size().y / 2.0),
            galley,
            theme::TEXT_DIM,
        );
        // Count pill.
        let pill_x = rect.left() + 32.0 + width + 8.0;
        let pill = Rect::from_min_size(egui::pos2(pill_x, y - 8.0), egui::vec2(24.0, 16.0));
        painter.rect_filled(pill, 8.0, Color32::from_rgb(52, 54, 58));
        painter.text(
            pill.center(),
            Align2::CENTER_CENTER,
            changes.len(),
            FontId::proportional(10.5),
            theme::TEXT,
        );
        if response.hovered() || bulk_response.hovered() {
            icon_button(painter, bulk, staged, bulk_response.hovered());
        }
    }
    let paths: Vec<String> = changes.iter().map(|c| c.path.clone()).collect();
    if bulk_response
        .on_hover_text(if staged { "Unstage all" } else { "Stage all" })
        .clicked()
    {
        action = Some(if staged {
            Action::Unstage(paths)
        } else {
            Action::Stage(paths)
        });
    } else if response.clicked() {
        *open = !*open;
    }

    if *open {
        for change in changes {
            if let Some(next) = change_row(ui, change, staged) {
                action = Some(next);
            }
        }
    }
    action
}

/// `+` (stage) or `-` (unstage) button, drawn with the painter.
fn icon_button(painter: &egui::Painter, rect: Rect, minus: bool, hovered: bool) {
    if hovered {
        painter.rect_filled(
            rect,
            5.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 24),
        );
    }
    let stroke = Stroke::new(1.5, theme::TEXT);
    let c = rect.center();
    painter.line_segment(
        [c + egui::vec2(-4.5, 0.0), c + egui::vec2(4.5, 0.0)],
        stroke,
    );
    if !minus {
        painter.line_segment(
            [c + egui::vec2(0.0, -4.5), c + egui::vec2(0.0, 4.5)],
            stroke,
        );
    }
}

fn change_row(ui: &mut egui::Ui, change: &git::Change, staged: bool) -> Option<Action> {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let y = rect.center().y;
    let button = Rect::from_center_size(egui::pos2(rect.right() - 20.0, y), egui::vec2(22.0, 22.0));
    let button_response = ui.interact(
        button,
        ui.scope_id().with(("stage", staged, change.path.as_str())),
        Sense::click(),
    );
    let hovered = response.hovered() || button_response.hovered();

    let (name, directory) = match change.path.rsplit_once('/') {
        Some((directory, name)) => (name, directory),
        None => (change.path.as_str(), ""),
    };
    let letter = change.letter(staged);
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 10),
        );
    }
    crate::icons::file(
        painter,
        Rect::from_center_size(egui::pos2(rect.left() + 22.0, y), egui::vec2(10.0, 13.0)),
        explorer::file_color(name),
    );
    let name_galley = painter.layout_no_wrap(
        name.to_string(),
        FontId::proportional(13.0),
        if letter == 'D' {
            theme::TEXT_DIM
        } else {
            theme::TEXT
        },
    );
    let name_width = name_galley.size().x;
    let text_left = rect.left() + 38.0;
    let clip = Rect::from_min_max(rect.min, egui::pos2(rect.right() - 44.0, rect.bottom()));
    let text_painter = painter.with_clip_rect(clip);
    text_painter.galley(
        egui::pos2(text_left, y - name_galley.size().y / 2.0),
        name_galley,
        theme::TEXT,
    );
    if !directory.is_empty() {
        text_painter.text(
            egui::pos2(text_left + name_width + 8.0, y),
            Align2::LEFT_CENTER,
            directory,
            FontId::proportional(11.5),
            Color32::from_rgb(112, 114, 120),
        );
    }
    if hovered {
        icon_button(painter, button, staged, button_response.hovered());
    } else {
        painter.text(
            egui::pos2(rect.right() - 20.0, y),
            Align2::CENTER_CENTER,
            letter,
            FontId::proportional(12.0),
            letter_color(letter),
        );
    }

    let hint = if staged { "Unstage" } else { "Stage" };
    if button_response.on_hover_text(hint).clicked() {
        let paths = vec![change.path.clone()];
        return Some(if staged {
            Action::Unstage(paths)
        } else {
            Action::Stage(paths)
        });
    }
    if response.clicked() {
        return Some(if change.is_untracked() || letter == 'D' {
            Action::OpenFile(change.path.clone())
        } else {
            Action::ShowDiff {
                path: change.path.clone(),
                staged,
            }
        });
    }
    None
}
