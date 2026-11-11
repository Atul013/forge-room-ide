//! The Source Control sidebar, modelled on VS Code's: commit box, Sync/Publish, staged and
//! unstaged lists, the commit graph and the history of the open file (GitLens-style).

use crate::{git, graph, room_state, theme};
use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense, Stroke};

pub struct State {
    pub message: String,
    pub staged_open: bool,
    pub changes_open: bool,
    pub conflicts_open: bool,
    pub graph_open: bool,
    pub history_open: bool,
    /// Which page of the sidebar is showing: false = Changes, true = History (graph + file).
    pub show_history: bool,
    /// The first click on Abort merge only arms it; the second one aborts.
    pub abort_armed: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            message: String::new(),
            staged_open: true,
            changes_open: true,
            conflicts_open: true,
            graph_open: true,
            history_open: true,
            show_history: false,
            abort_armed: false,
        }
    }
}

pub struct View<'a> {
    pub status: &'a git::Status,
    pub commits: &'a [git::CommitInfo],
    pub rows: &'a [graph::Row],
    pub history: &'a [git::CommitInfo],
    pub history_name: Option<&'a str>,
    pub stashes: usize,
    /// A merge, cherry-pick or revert waiting to be finished (conflicts) or aborted.
    pub operation: Option<git::Operation>,
    /// Set while a network operation is running (shown instead of the sync button).
    pub busy: Option<&'a str>,
}

pub enum Repo<'a> {
    NoFolder,
    /// The first read of the repository is still running.
    Loading,
    NoRepository,
    Error(&'a str),
    Ready(View<'a>),
}

pub enum Action {
    None,
    Refresh,
    Init,
    Commit,
    CommitAndPush,
    Amend,
    UndoCommit,
    Stage(Vec<String>),
    Unstage(Vec<String>),
    DiscardAsk(Vec<git::Change>),
    Resolve { path: String, side: git::Side },
    ShowDiff { path: String, staged: bool },
    OpenFile(String),
    Sync,
    Publish,
    Pull,
    Push,
    Fetch,
    Stash,
    StashPop,
    AbortOperation,
    CherryPick(String),
    Revert(String),
    ShowCommit(String),
}

const ROW_HEIGHT: f32 = 26.0;
const GRAPH_ROW: f32 = 26.0;
const LANE_WIDTH: f32 = 14.0;

pub fn letter_color(letter: char) -> Color32 {
    match letter {
        'M' | 'T' => Color32::from_rgb(226, 192, 141),
        'A' | 'U' => Color32::from_rgb(129, 199, 132),
        'D' => Color32::from_rgb(240, 113, 120),
        'R' | 'C' => Color32::from_rgb(110, 170, 225),
        _ => Color32::from_rgb(255, 120, 100),
    }
}

fn lane_color(lane: usize) -> Color32 {
    let p = theme::current().palette;
    // Calm, varied lane colors; the accent is kept for the branch label, not every line.
    [
        p.function,
        p.number,
        p.string,
        p.type_color,
        p.keyword,
        p.accent,
    ][lane % 6]
}

pub fn show(ui: &mut egui::Ui, state: &mut State, repo: Repo, compact: bool) -> Action {
    let mut action = Action::None;

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(
            RichText::new("Source Control")
                .size(14.5)
                .family(crate::fonts::semibold())
                .color(theme::text()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            if let Repo::Ready(view) = &repo {
                let more = crate::icons::icon_button(ui, "More actions", crate::icons::more);
                egui::Popup::menu(&more).show(|ui| {
                    for (label, next) in [
                        ("Pull", Action::Pull),
                        ("Push", Action::Push),
                        ("Fetch", Action::Fetch),
                    ] {
                        if ui.button(label).clicked() {
                            action = next;
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("Stash changes").clicked() {
                        action = Action::Stash;
                        ui.close();
                    }
                    if ui
                        .add_enabled(view.stashes > 0, egui::Button::new("Pop latest stash"))
                        .clicked()
                    {
                        action = Action::StashPop;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Amend last commit").clicked() {
                        action = Action::Amend;
                        ui.close();
                    }
                    if ui.button("Undo last commit").clicked() {
                        action = Action::UndoCommit;
                        ui.close();
                    }
                });
            }
            if !matches!(repo, Repo::NoFolder)
                && crate::icons::icon_button(ui, "Refresh", crate::icons::refresh).clicked()
            {
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
        Repo::Loading => empty_state(ui, "Reading the repository…", "This takes a moment."),
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
        Repo::Ready(view) => {
            if let Some(next) = ready(ui, state, &view, compact) {
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
            ui.label(RichText::new(title).strong().color(theme::text()));
            ui.add_space(2.0);
            ui.label(RichText::new(detail).color(theme::text_dim()));
            ui.add_space(8.0);
        });
    });
}

fn ready(ui: &mut egui::Ui, state: &mut State, view: &View, compact: bool) -> Option<Action> {
    let mut action = None;
    let status = view.status;
    // A conflicted file is neither staged nor unstaged yet: it gets its own group, once.
    let conflicts: Vec<&git::Change> = status.changes.iter().filter(|c| c.is_conflict()).collect();
    let staged: Vec<&git::Change> = status
        .changes
        .iter()
        .filter(|c| !c.is_conflict() && c.is_staged())
        .collect();
    let unstaged: Vec<&git::Change> = status
        .changes
        .iter()
        .filter(|c| !c.is_conflict() && c.is_unstaged())
        .collect();

    if let Some(operation) = view.operation {
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(12, 0))
            .show(ui, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(226, 160, 60, 30))
                    .stroke(Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(226, 160, 60, 120),
                    ))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new(match operation {
                                git::Operation::Rebase => {
                                    "Rebase in progress. Finish it in the terminal \
                                     (git rebase --continue) or abort it here."
                                        .to_string()
                                }
                                _ => format!(
                                    "{} in progress. Resolve the conflicts, stage the files \
                                     and commit to finish it (the message is prepared).",
                                    operation.label()
                                ),
                            })
                            .size(12.5)
                            .color(theme::text()),
                        );
                        ui.add_space(4.0);
                        let label = if state.abort_armed {
                            "Click again: discard it and my resolutions"
                        } else {
                            "Abort"
                        };
                        if ui.button(label).clicked() {
                            if state.abort_armed {
                                state.abort_armed = false;
                                action = Some(Action::AbortOperation);
                            } else {
                                state.abort_armed = true;
                            }
                        }
                    });
            });
        ui.add_space(8.0);
    }
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 0))
        .show(ui, |ui| {
            let editor = egui::TextEdit::multiline(&mut state.message)
                .hint_text("Message (Ctrl+Enter to commit)")
                .desired_rows(if compact { 1 } else { 2 })
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(10, 8));
            let response = ui.add(editor);
            // Finishing a merge, cherry-pick or revert needs no typed message: Git prepared one.
            let can_commit = !staged.is_empty()
                && conflicts.is_empty()
                && (!state.message.trim().is_empty() || view.operation.is_some());
            let submit = response.has_focus()
                && ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter));
            ui.add_space(6.0);

            let label = if staged.is_empty() {
                "Commit".to_string()
            } else {
                format!("Commit {} staged", staged.len())
            };
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let button =
                    egui::Button::new(RichText::new(label).strong().color(theme::button_text()))
                        .fill(theme::button())
                        .corner_radius(6.0)
                        .min_size(egui::vec2((ui.available_width() - 32.0).max(80.0), 30.0));
                let hint = if !conflicts.is_empty() {
                    "Resolve the merge conflicts first"
                } else if can_commit {
                    "Commit the staged changes"
                } else {
                    "Stage changes and write a message first"
                };
                if ((ui.add(button).on_hover_text(hint).clicked() && can_commit)
                    || (submit && can_commit))
                    && action.is_none()
                {
                    action = Some(Action::Commit);
                }
                // Dropdown arrow drawn by hand: the UI font has no arrow glyph.
                let (arrow_rect, arrow) =
                    ui.allocate_exact_size(egui::vec2(28.0, 30.0), Sense::click());
                ui.painter().rect_filled(
                    arrow_rect,
                    6.0,
                    theme::mix(theme::button(), theme::bg_sidebar(), 0.25),
                );
                crate::icons::chevron(
                    ui.painter(),
                    arrow_rect.center(),
                    true,
                    theme::button_text(),
                );
                egui::Popup::menu(&arrow).show(|ui| {
                    if ui
                        .add_enabled(can_commit, egui::Button::new("Commit & Push"))
                        .clicked()
                    {
                        action = Some(Action::CommitAndPush);
                    }
                    if ui.button("Amend last commit").clicked() {
                        action = Some(Action::Amend);
                    }
                    if ui.button("Undo last commit").clicked() {
                        action = Some(Action::UndoCommit);
                    }
                });
            });

            // Sync / publish, like VS Code's blue button under the commit box.
            ui.add_space(6.0);
            if let Some(job) = view.busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(job).color(theme::text_dim()));
                });
            } else if status.branch.is_some() && status.upstream.is_none() {
                if sync_button(ui, "Publish Branch").clicked() {
                    action = Some(Action::Publish);
                }
            } else if status.ahead > 0 || status.behind > 0 {
                let text = match (status.behind, status.ahead) {
                    (0, ahead) => format!("Sync Changes · {ahead} to push"),
                    (behind, 0) => format!("Sync Changes · {behind} to pull"),
                    (behind, ahead) => format!("Sync Changes · {behind} to pull, {ahead} to push"),
                };
                if sync_button(ui, &text).clicked() {
                    action = Some(Action::Sync);
                }
            }
        });
    ui.add_space(8.0);
    if !compact {
        page_tabs(ui, state, status.changes.len(), view.commits.len());
    }

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            if !compact && state.show_history {
                history_page(ui, state, view, &mut action);
                return;
            }
            if status.changes.is_empty() {
                empty_state(
                    ui,
                    "Working tree clean",
                    "There is nothing to commit. Changes you make will show up here.",
                );
            }
            if !conflicts.is_empty()
                && let Some(next) = section(
                    ui,
                    "MERGE CONFLICTS",
                    &mut state.conflicts_open,
                    &conflicts,
                    false,
                )
            {
                action = Some(next);
            }
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

            ui.add_space(16.0);
        });
    action
}

/// The History page: the commit graph, then the history of the open file.
fn history_page(ui: &mut egui::Ui, state: &mut State, view: &View, action: &mut Option<Action>) {
    let status = view.status;
    // Commit graph.
    ui.add_space(6.0);
    group_header(ui, "GRAPH", &mut state.graph_open, view.commits.len());
    if state.graph_open {
        let width = view.rows.iter().map(|row| row.width).max().unwrap_or(1);
        for (commit, row) in view.commits.iter().zip(view.rows) {
            let response = graph_row(ui, commit, row, width, status.branch.as_deref());
            if response.clicked() {
                *action = Some(Action::ShowCommit(commit.hash.clone()));
            }
            response.context_menu(|ui| {
                // A merge commit has two sides: picking or reverting it needs a mainline chosen.
                let single = commit.parents.len() <= 1;
                if ui
                    .add_enabled(single, egui::Button::new("Cherry-pick onto this branch"))
                    .clicked()
                {
                    *action = Some(Action::CherryPick(commit.hash.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(single, egui::Button::new("Revert this commit"))
                    .clicked()
                {
                    *action = Some(Action::Revert(commit.hash.clone()));
                    ui.close();
                }
                ui.separator();
                if ui.button("Copy commit hash").clicked() {
                    ui.ctx().copy_text(commit.hash.clone());
                    ui.close();
                }
            });
        }
        if view.commits.is_empty() {
            empty_state(ui, "No commits yet", "Your first commit will appear here.");
        }
    }

    // History of the file that is open, like GitLens' File History view.
    if let Some(name) = view.history_name {
        ui.add_space(6.0);
        let title = format!("FILE HISTORY · {name}");
        group_header(ui, &title, &mut state.history_open, view.history.len());
        if state.history_open {
            for commit in view.history {
                if history_row(ui, commit).clicked() {
                    *action = Some(Action::ShowCommit(commit.hash.clone()));
                }
            }
            if view.history.is_empty() {
                empty_state(ui, "No history", "This file has not been committed yet.");
            }
        }
    }
    ui.add_space(16.0);
}

/// Changes | History switch under the commit box, so the two lists no longer share one scroll.
fn page_tabs(ui: &mut egui::Ui, state: &mut State, changes: usize, commits: usize) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 0))
        .show(ui, |ui| {
            // Equal widths from the parent, fixed once: a button's own text can no longer widen it.
            let width = ((ui.available_width() - 6.0) / 2.0).floor().max(40.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (history, label, count) in
                    [(false, "Changes", changes), (true, "History", commits)]
                {
                    let active = state.show_history == history;
                    // Narrow sidebars drop the count rather than overflow.
                    let caption = if width < 110.0 {
                        label.to_string()
                    } else {
                        format!("{label}  {count}")
                    };
                    let text = RichText::new(caption).size(13.0).color(if active {
                        theme::text()
                    } else {
                        theme::text_dim()
                    });
                    let button = egui::Button::new(text)
                        .fill(if active {
                            theme::overlay(18)
                        } else {
                            Color32::TRANSPARENT
                        })
                        .corner_radius(6.0)
                        .truncate();
                    if ui.add_sized(egui::vec2(width, 26.0), button).clicked() {
                        state.show_history = history;
                    }
                }
            });
        });
    ui.add_space(6.0);
}

fn sync_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(theme::text()))
            .fill(theme::mix(theme::bg_sidebar(), theme::text(), 0.04))
            .stroke(Stroke::new(1.0, theme::border()))
            .corner_radius(6.0)
            .min_size(egui::vec2(ui.available_width(), 30.0)),
    )
}

/// A plain collapsible header with a count pill.
fn group_header(ui: &mut egui::Ui, title: &str, open: &mut bool, count: usize) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let y = rect.center().y;
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, 0.0, theme::overlay(8));
    }
    crate::icons::chevron(
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
        egui::vec2(26.0, 16.0),
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
    if response.clicked() {
        *open = !*open;
    }
}

/// One collapsible group of changes. Returns the action requested by a click inside it.
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
    let discard_all =
        Rect::from_center_size(egui::pos2(rect.right() - 44.0, y), egui::vec2(22.0, 22.0));
    let discard_all_response = if staged || title == "MERGE CONFLICTS" {
        None
    } else {
        Some(ui.interact(
            discard_all,
            ui.scope_id().with(("bulk-discard", title)),
            Sense::click(),
        ))
    };
    {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(rect, 0.0, theme::overlay(8));
        }
        crate::icons::chevron(
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
        let pill_x = rect.left() + 32.0 + width + 8.0;
        let pill = Rect::from_min_size(egui::pos2(pill_x, y - 8.0), egui::vec2(24.0, 16.0));
        painter.rect_filled(
            pill,
            8.0,
            theme::mix(theme::bg_sidebar(), theme::text(), 0.14),
        );
        painter.text(
            pill.center(),
            Align2::CENTER_CENTER,
            changes.len(),
            FontId::proportional(10.5),
            theme::text(),
        );
        let hot = response.hovered()
            || bulk_response.hovered()
            || discard_all_response.as_ref().is_some_and(|r| r.hovered());
        if hot {
            icon_button(painter, bulk, staged, bulk_response.hovered());
            if let Some(discard) = &discard_all_response {
                discard_icon(painter, discard_all, discard.hovered());
            }
        }
    }
    let owned: Vec<git::Change> = changes.iter().map(|change| (*change).clone()).collect();
    let paths: Vec<String> = owned.iter().map(|c| c.path.clone()).collect();
    if bulk_response
        .on_hover_text(if staged { "Unstage all" } else { "Stage all" })
        .clicked()
    {
        action = Some(if staged {
            Action::Unstage(paths)
        } else {
            Action::Stage(paths)
        });
    } else if discard_all_response.is_some_and(|r| r.on_hover_text("Discard all changes").clicked())
    {
        action = Some(Action::DiscardAsk(owned));
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

/// `+` (stage) or `-` (unstage), drawn with the painter.
fn icon_button(painter: &egui::Painter, rect: Rect, minus: bool, hovered: bool) {
    if hovered {
        painter.rect_filled(rect, 5.0, theme::overlay(24));
    }
    let stroke = Stroke::new(1.5, theme::text());
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

/// Counter-clockwise arrow: "discard changes".
fn discard_icon(painter: &egui::Painter, rect: Rect, hovered: bool) {
    if hovered {
        painter.rect_filled(rect, 5.0, theme::overlay(24));
    }
    let stroke = Stroke::new(1.5, theme::text());
    let c = rect.center();
    let mut points = Vec::new();
    for step in 0..=9 {
        let angle = -2.4 + step as f32 / 9.0 * 4.5;
        points.push(c + egui::vec2(angle.cos() * 5.0, angle.sin() * 5.0));
    }
    painter.add(egui::Shape::line(points, stroke));
    painter.line_segment(
        [c + egui::vec2(-5.5, -1.0), c + egui::vec2(-5.5, -5.5)],
        stroke,
    );
    painter.line_segment(
        [c + egui::vec2(-5.5, -1.0), c + egui::vec2(-1.0, -1.5)],
        stroke,
    );
}

fn change_row(ui: &mut egui::Ui, change: &git::Change, staged: bool) -> Option<Action> {
    // The file name gets its own line and the folder sits underneath, so neither is cut off.
    let two_lines = change.path.contains('/');
    let height = if two_lines {
        ROW_HEIGHT + 12.0
    } else {
        ROW_HEIGHT
    };
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    let y = rect.center().y;
    let stage_rect =
        Rect::from_center_size(egui::pos2(rect.right() - 20.0, y), egui::vec2(22.0, 22.0));
    let stage_response = ui.interact(
        stage_rect,
        ui.scope_id().with(("stage", staged, change.path.as_str())),
        Sense::click(),
    );
    let discard_rect =
        Rect::from_center_size(egui::pos2(rect.right() - 44.0, y), egui::vec2(22.0, 22.0));
    let discard_response = (!staged && !change.is_conflict()).then(|| {
        ui.interact(
            discard_rect,
            ui.scope_id().with(("discard", change.path.as_str())),
            Sense::click(),
        )
    });
    let response = response.on_hover_text(&change.path);
    let hovered = response.hovered()
        || stage_response.hovered()
        || discard_response.as_ref().is_some_and(|r| r.hovered());

    let (name, directory) = match change.path.rsplit_once('/') {
        Some((directory, name)) => (name, directory),
        None => (change.path.as_str(), ""),
    };
    let letter = change.letter(staged);
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(rect, 0.0, theme::overlay(10));
    }
    // Status letter as a small badge on the left, like the design.
    let badge = Rect::from_center_size(egui::pos2(rect.left() + 26.0, y), egui::vec2(20.0, 20.0));
    painter.rect_filled(badge, 5.0, letter_color(letter).gamma_multiply(0.18));
    painter.text(
        badge.center(),
        Align2::CENTER_CENTER,
        letter,
        FontId::proportional(12.0),
        letter_color(letter),
    );
    let name_galley = painter.layout_no_wrap(
        name.to_string(),
        FontId::proportional(13.5),
        if letter == 'D' {
            theme::text_dim()
        } else {
            theme::text()
        },
    );
    let name_width = name_galley.size().x;
    let text_left = rect.left() + 46.0;
    // Room for the hover buttons only while they are shown.
    let reserved = match (hovered, staged) {
        (false, _) => 12.0,
        (true, true) => 44.0,
        (true, false) => 68.0,
    };
    let clip = Rect::from_min_max(rect.min, egui::pos2(rect.right() - reserved, rect.bottom()));
    let text_painter = painter.with_clip_rect(clip);
    let name_y = if two_lines { rect.top() + 11.0 } else { y };
    text_painter.galley(
        egui::pos2(text_left, name_y - name_galley.size().y / 2.0),
        name_galley,
        theme::text(),
    );
    let _ = name_width;
    if !directory.is_empty() {
        text_painter.text(
            egui::pos2(text_left, rect.bottom() - 10.0),
            Align2::LEFT_CENTER,
            directory,
            FontId::proportional(11.5),
            theme::mix(theme::bg_sidebar(), theme::text(), 0.45),
        );
    }
    if hovered {
        icon_button(painter, stage_rect, staged, stage_response.hovered());
        if let Some(discard) = &discard_response {
            discard_icon(painter, discard_rect, discard.hovered());
        }
    }

    if stage_response
        .on_hover_text(if staged { "Unstage" } else { "Stage" })
        .clicked()
    {
        let paths = vec![change.path.clone()];
        return Some(if staged {
            Action::Unstage(paths)
        } else {
            Action::Stage(paths)
        });
    }
    if discard_response.is_some_and(|r| r.on_hover_text("Discard changes").clicked()) {
        return Some(Action::DiscardAsk(vec![change.clone()]));
    }
    if change.is_conflict() {
        let mut chosen = None;
        response.context_menu(|ui| {
            if ui.button("Accept Current Change").clicked() {
                chosen = Some(git::Side::Current);
                ui.close();
            }
            if ui.button("Accept Incoming Change").clicked() {
                chosen = Some(git::Side::Incoming);
                ui.close();
            }
            ui.separator();
            ui.label(
                egui::RichText::new("Or edit the file, then stage it to mark it resolved.")
                    .size(12.0)
                    .color(theme::text_dim()),
            );
        });
        if let Some(side) = chosen {
            return Some(Action::Resolve {
                path: change.path.clone(),
                side,
            });
        }
        if response.clicked() {
            return Some(Action::OpenFile(change.path.clone()));
        }
        return None;
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

/// Colored pill for a branch, remote or tag label. Returns the width it used.
fn ref_pill(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    label: &str,
    color: Color32,
    filled: bool,
) -> f32 {
    let galley = painter.layout_no_wrap(
        label.to_string(),
        FontId::proportional(11.0),
        if filled { Color32::WHITE } else { color },
    );
    let width = galley.size().x + 14.0;
    let rect = Rect::from_min_size(egui::pos2(x, y - 9.0), egui::vec2(width, 18.0));
    let text_color = if filled { Color32::WHITE } else { color };
    if filled {
        painter.rect_filled(rect, 9.0, color);
    } else {
        painter.rect_filled(
            rect,
            9.0,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 38),
        );
        painter.rect_stroke(
            rect,
            9.0,
            Stroke::new(1.0, color),
            egui::epaint::StrokeKind::Inside,
        );
    }
    painter.galley(
        egui::pos2(x + 7.0, y - galley.size().y / 2.0),
        galley,
        text_color,
    );
    width + 4.0
}

fn graph_row(
    ui: &mut egui::Ui,
    commit: &git::CommitInfo,
    row: &graph::Row,
    graph_width: usize,
    current_branch: Option<&str>,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), GRAPH_ROW), Sense::click());
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, 0.0, theme::overlay(8));
    }
    // Many lanes are squeezed together so the commit message keeps room to read.
    // The graph never takes more than 40% of the row (64 px at most), so the subject keeps
    // room even at the narrowest sidebar and with dozens of lanes.
    let budget = (rect.width() * 0.4).min(64.0);
    let lane_width = LANE_WIDTH.min(budget / graph_width.max(1) as f32);
    let x_of = |lane: usize| rect.left() + 16.0 + lane as f32 * lane_width;
    let (top, middle, bottom) = (rect.top(), rect.center().y, rect.bottom());
    let node = egui::pos2(x_of(row.lane), middle);

    for lane in &row.passing {
        painter.line_segment(
            [
                egui::pos2(x_of(*lane), top),
                egui::pos2(x_of(*lane), bottom),
            ],
            Stroke::new(1.6, lane_color(*lane)),
        );
    }
    for lane in &row.incoming {
        painter.line_segment(
            [egui::pos2(x_of(*lane), top), node],
            Stroke::new(1.6, lane_color(*lane)),
        );
    }
    for target in &row.outgoing {
        painter.line_segment(
            [node, egui::pos2(x_of(*target), bottom)],
            Stroke::new(1.6, lane_color(*target)),
        );
    }
    let is_head = commit.refs.iter().any(|name| name.starts_with("HEAD"));
    painter.circle_filled(node, 4.5, lane_color(row.lane));
    if is_head {
        painter.circle_stroke(node, 7.0, Stroke::new(1.5, theme::text()));
    } else {
        painter.circle_stroke(node, 4.5, Stroke::new(1.5, theme::bg_sidebar()));
    }

    let text_left = x_of(graph_width) + 6.0;
    let clip = painter.with_clip_rect(Rect::from_min_max(
        egui::pos2(text_left - 2.0, top),
        rect.right_bottom(),
    ));
    let galley = clip.layout_no_wrap(
        commit.subject.clone(),
        FontId::proportional(13.0),
        theme::text(),
    );
    let width = galley.size().x;
    clip.galley(
        egui::pos2(text_left, middle - galley.size().y / 2.0),
        galley,
        theme::text(),
    );

    let mut x = text_left + width + 8.0;
    for name in &commit.refs {
        let p = theme::current().palette;
        let (label, color, filled) = if let Some(branch) = name.strip_prefix("HEAD -> ") {
            (branch.to_string(), p.accent, Some(branch) == current_branch)
        } else if name == "HEAD" {
            ("HEAD".to_string(), p.accent, true)
        } else if let Some(tag) = name.strip_prefix("tag: ") {
            (tag.to_string(), Color32::from_rgb(226, 192, 141), false)
        } else if name.contains('/') {
            (name.clone(), p.keyword, false)
        } else {
            (name.clone(), p.function, false)
        };
        x += ref_pill(&clip, x, middle, &label, color, filled);
    }

    response.on_hover_text(format!(
        "{}\n{} · {}\n{}",
        commit.subject,
        commit.author,
        room_state::ago(room_state::now(), commit.time.max(0) as u64),
        commit.short()
    ))
}

fn history_row(ui: &mut egui::Ui, commit: &git::CommitInfo) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT + 4.0),
        Sense::click(),
    );
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, 0.0, theme::overlay(8));
    }
    let clip = painter.with_clip_rect(rect);
    clip.circle_filled(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        3.5,
        theme::accent(),
    );
    clip.text(
        egui::pos2(rect.left() + 34.0, rect.top() + 9.0),
        Align2::LEFT_CENTER,
        &commit.subject,
        FontId::proportional(13.0),
        theme::text(),
    );
    clip.text(
        egui::pos2(rect.left() + 34.0, rect.bottom() - 8.0),
        Align2::LEFT_CENTER,
        format!(
            "{} · {} · {}",
            commit.author,
            room_state::ago(room_state::now(), commit.time.max(0) as u64),
            commit.short()
        ),
        FontId::proportional(11.0),
        theme::text_dim(),
    );
    response
}
