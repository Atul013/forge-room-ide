#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod brand;
mod commands;
mod document;
mod explorer;
mod find;
mod find_bar;
mod fonts;
mod fuzzy;
mod git;
mod graph;
mod gutter;
mod icons;
mod palette;
mod position;
mod room_state;
mod rooms;
mod search;
mod search_view;
mod settings;
mod source_control;
mod syntax;
mod tabs;
mod terminal;
mod theme;
mod theme_gallery;
mod workspace;

use commands::Command;
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use std::{fs, path::PathBuf};
use tabs::{Tabs, normalize};
use workspace::Workspace;

const APP_NAME: &str = "Forge Room Early Preview";
const TAB_HEIGHT: f32 = 38.0;
const TITLE_HEIGHT: f32 = 46.0;
const ACTIVITY_WIDTH: f32 = 64.0;

#[derive(Clone, Copy)]
enum RailIcon {
    Play,
    Extensions,
}

/// One entry of the navigation rail on the left.
#[derive(Clone, Copy)]
enum Rail {
    View(SidebarView),
    Terminal,
    Room,
    Ai,
    /// Shown in the design but not built yet: drawn dim and inert.
    Soon(RailIcon),
}

/// Tabs of the dock on the right.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Ai,
    Room,
}

fn main() {
    if let Err(error) = run() {
        let message = format!("{error:#}");
        let log_path = startup_error_path();
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&log_path, format!("{message}\n"));
        eprintln!("{APP_NAME} failed to start: {message}");
        std::process::exit(1);
    }
}

fn run() -> eframe::Result {
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("forge-room-early-preview")
            .with_title(APP_NAME)
            .with_decorations(false)
            .with_maximized(false)
            .with_inner_size([1240.0, 780.0])
            .with_min_inner_size([760.0, 480.0]),
        // A saved window can end up partly under the taskbar; always start centred and fitted.
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |creation_context| {
            Ok(Box::new(ForgeRoomApp::new(creation_context, initial_path)))
        }),
    )
}

fn startup_error_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join(APP_NAME).join("logs").join("startup-error.log")
}

enum Pending {
    CloseTab(usize),
    Quit,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SidebarView {
    Explorer,
    Search,
    SourceControl,
    Room,
}

/// A Git operation running on a background thread (network calls can take seconds).
struct GitJob {
    label: String,
    receiver: std::sync::mpsc::Receiver<Result<String, String>>,
}

enum Choice {
    Save,
    Discard,
    Cancel,
}

struct ForgeRoomApp {
    tabs: Tabs,
    workspace: Option<Workspace>,
    show_sidebar: bool,
    status: String,
    pending: Option<Pending>,
    allow_close: bool,
    focus_editor: bool,
    cursor: (usize, usize),
    last_title: String,
    branch: Option<String>,
    view: SidebarView,
    git_status: Option<Result<git::Status, String>>,
    is_repo: bool,
    sc: source_control::State,
    last_git: std::time::Instant,
    hunks: Vec<git::Hunk>,
    hunks_tab: Option<u64>,
    graph_commits: Vec<git::CommitInfo>,
    graph_rows: Vec<graph::Row>,
    file_history: Vec<git::CommitInfo>,
    history_name: Option<String>,
    stash_count: usize,
    blame: Vec<git::BlameLine>,
    blame_enabled: bool,
    git_job: Option<GitJob>,
    pending_discard: Option<Vec<git::Change>>,
    gallery: theme_gallery::Gallery,
    room: rooms::RoomView,
    terminal: Option<terminal::Terminal>,
    show_terminal: bool,
    show_right: bool,
    right_tab: RightTab,
    terminal_focus_request: bool,
    terminal_has_focus: bool,
    terminal_error: Option<String>,
    egui_ctx: egui::Context,
    palette: palette::Palette,
    search: search_view::SearchView,
    find: find_bar::FindState,
    /// Character range to select (and scroll to) in the active editor on the next frame.
    pending_select: Option<(usize, usize)>,
    last_find_query: String,
    reselect_find: bool,
}

/// Unified-diff coloring: green additions, red removals, blue hunk headers, dim file headers.
fn diff_job(text: &str, font_id: &egui::FontId) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    for line in text.split_inclusive('\n') {
        let mut format = egui::TextFormat::simple(font_id.clone(), theme::text());
        if line.starts_with("+++")
            || line.starts_with("---")
            || line.starts_with("diff ")
            || line.starts_with("index ")
        {
            format.color = theme::text_dim();
        } else if line.starts_with('+') {
            format.color = egui::Color32::from_rgb(140, 210, 145);
            format.background = egui::Color32::from_rgba_unmultiplied(87, 166, 74, 34);
        } else if line.starts_with('-') {
            format.color = egui::Color32::from_rgb(245, 130, 135);
            format.background = egui::Color32::from_rgba_unmultiplied(229, 83, 75, 34);
        } else if line.starts_with("@@") {
            format.color = egui::Color32::from_rgb(110, 170, 225);
        }
        job.append(line, 0.0, format);
    }
    job
}

fn shortcut(ui: &egui::Ui, modifiers: Modifiers, key: Key) -> bool {
    ui.input_mut(|input| input.consume_shortcut(&KeyboardShortcut::new(modifiers, key)))
}

impl ForgeRoomApp {
    fn new(creation_context: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        fonts::install(&creation_context.egui_ctx);
        match settings::load().theme {
            Some(key) if theme::find(&key).is_some() => {
                theme::set(&creation_context.egui_ctx, &key)
            }
            _ => theme::apply(&creation_context.egui_ctx),
        }
        let mut app = Self {
            tabs: Tabs::default(),
            workspace: None,
            show_sidebar: true,
            status: "Ready".into(),
            pending: None,
            allow_close: false,
            focus_editor: false,
            cursor: (1, 1),
            last_title: String::new(),
            branch: None,
            view: SidebarView::Explorer,
            git_status: None,
            is_repo: false,
            sc: source_control::State::default(),
            last_git: std::time::Instant::now(),
            hunks: Vec::new(),
            hunks_tab: None,
            graph_commits: Vec::new(),
            graph_rows: Vec::new(),
            file_history: Vec::new(),
            history_name: None,
            stash_count: 0,
            blame: Vec::new(),
            blame_enabled: true,
            git_job: None,
            pending_discard: None,
            gallery: theme_gallery::Gallery::default(),
            room: rooms::RoomView::default(),
            terminal: None,
            show_terminal: true,
            show_right: true,
            right_tab: RightTab::Room,
            terminal_focus_request: false,
            terminal_has_focus: false,
            terminal_error: None,
            egui_ctx: creation_context.egui_ctx.clone(),
            palette: palette::Palette::default(),
            search: search_view::SearchView::default(),
            find: find_bar::FindState::default(),
            pending_select: None,
            last_find_query: String::new(),
            reselect_find: false,
        };
        if let Some(path) = initial_path {
            app.open_path(path);
        }
        app
    }

    // ---- File operations ---------------------------------------------------------------

    fn open_path(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.workspace = Some(Workspace::new(normalize(&path)));
            self.show_sidebar = true;
            self.refresh_git();
            let root = self.repo_root();
            self.room.load_for(root.as_deref());
            self.search.reset();
            self.status = format!("Opened folder {}", path.display());
            return;
        }
        match self.tabs.open_file(&path) {
            Ok(()) => {
                self.focus_editor = true;
                self.status = format!("Opened {}", path.display());
            }
            Err(error) => self.status = format!("Cannot open {}: {error}", path.display()),
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_path(path);
        }
    }

    fn open_folder_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.open_path(path);
        }
    }

    fn new_file(&mut self) {
        self.tabs.new_untitled();
        self.focus_editor = true;
    }

    /// Save tab `index`, asking for a path first when it has none. False when cancelled or failed.
    fn save_index(&mut self, index: usize, force_dialog: bool) -> bool {
        let Some(tab) = self.tabs.tabs.get_mut(index) else {
            return false;
        };
        let title = tab.title();
        let result = if tab.doc.path().is_none() || force_dialog {
            let mut dialog = rfd::FileDialog::new().set_file_name(&title);
            if let Some(dir) = self.workspace.as_ref() {
                dialog = dialog.set_directory(&dir.root);
            }
            match dialog.save_file() {
                Some(path) => tab.doc.save_as(path),
                None => return false,
            }
        } else {
            tab.doc.save()
        };
        match result {
            Ok(()) => {
                self.status = format!("Saved {title}");
                self.refresh_git();
                true
            }
            Err(error) => {
                self.status = format!("Cannot save {title}: {error}");
                false
            }
        }
    }

    fn save_active(&mut self, force_dialog: bool) {
        if !self.tabs.tabs.is_empty() {
            self.save_index(self.tabs.active, force_dialog);
        }
    }

    fn request_close(&mut self, index: usize) {
        match self.tabs.tabs.get(index) {
            Some(tab) if tab.doc.is_dirty() => self.pending = Some(Pending::CloseTab(index)),
            Some(_) => {
                self.tabs.close(index);
                self.focus_editor = true;
            }
            None => {}
        }
    }

    // ---- UI ----------------------------------------------------------------------------

    /// Run any palette/menu/shortcut command. `palette_mode` only matters for `GoToLine`, where
    /// the bool selects the command palette instead of the "Go to line" prompt.
    fn run_command(&mut self, command: Command, palette_mode: bool) {
        match command {
            Command::NewFile => self.new_file(),
            Command::OpenFile => self.open_file_dialog(),
            Command::OpenFolder => self.open_folder_dialog(),
            Command::Save => self.save_active(false),
            Command::SaveAs => self.save_active(true),
            Command::CloseTab => {
                if !self.tabs.tabs.is_empty() {
                    self.request_close(self.tabs.active);
                }
            }
            Command::NextTab => {
                self.tabs.next();
                self.focus_editor = true;
            }
            Command::ToggleSidebar => self.show_sidebar = !self.show_sidebar,
            Command::ShowExplorer => self.switch_view(SidebarView::Explorer),
            Command::ShowSourceControl => self.switch_view(SidebarView::SourceControl),
            Command::ShowRoom => self.switch_view(SidebarView::Room),
            Command::ShowSearch => {
                self.view = SidebarView::Search;
                self.show_sidebar = true;
                self.search.focus_input();
            }
            Command::ColorTheme => self.gallery.open(),
            Command::GitRefresh => self.refresh_git(),
            Command::ToggleBlame => {
                self.blame_enabled = !self.blame_enabled;
                self.status = if self.blame_enabled {
                    "Line blame on".into()
                } else {
                    "Line blame off".into()
                };
            }
            Command::SwitchBranch => {
                if let Some(root) = self.repo_root() {
                    match git::branches(&root) {
                        Ok(list) => self.palette.open_branches(list),
                        Err(error) => self.status = format!("Git: {error}"),
                    }
                }
            }
            Command::GitSync => self.git_action(source_control::Action::Sync),
            Command::GitPull => self.git_action(source_control::Action::Pull),
            Command::GitPush => self.git_action(source_control::Action::Push),
            Command::GitFetch => self.git_action(source_control::Action::Fetch),
            Command::GitStash => self.git_action(source_control::Action::Stash),
            Command::Find => self.find.show_bar(false),
            Command::Replace => self.find.show_bar(true),
            Command::QuickOpen => {
                let root = self.repo_root();
                self.palette.open_files(root.as_deref());
            }
            Command::GoToLine if palette_mode => self.palette.open_commands(),
            Command::GoToLine => self.palette.open_goto(),
            Command::ToggleTerminal => {
                if self.show_terminal && self.terminal_has_focus {
                    self.show_terminal = false;
                    self.focus_editor = true;
                } else {
                    self.show_terminal = true;
                    self.terminal_focus_request = true;
                    self.ensure_terminal();
                }
            }
        }
    }

    /// Start a shell in the project folder when there is none running.
    fn ensure_terminal(&mut self) {
        let running = self.terminal.as_ref().is_some_and(|t| !t.has_exited());
        if running {
            return;
        }
        let root = self.repo_root();
        match terminal::Terminal::spawn(&self.egui_ctx, root.as_deref()) {
            Ok(terminal) => {
                self.terminal = Some(terminal);
                self.terminal_error = None;
            }
            Err(error) => {
                self.terminal = None;
                self.terminal_error = Some(error);
            }
        }
    }

    fn terminal_panel(&mut self, ui: &mut egui::Ui) {
        let exited = self.terminal.as_ref().is_some_and(|t| t.has_exited());
        let mut restart = false;
        let mut close = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(12.0);
            underline_tab(ui, "Terminal", true, true, None);
            for label in ["Output", "Problems", "Debug Console", "Ports"] {
                underline_tab(ui, label, false, false, None).on_hover_text("Coming soon");
            }
            if let Some(terminal) = &self.terminal {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(&terminal.shell)
                        .size(12.0)
                        .color(theme::text_dim()),
                );
            }
            if exited {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("process exited")
                        .size(12.0)
                        .color(theme::text_dim()),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if icons::icon_button(ui, "Hide terminal (Ctrl+`)", icons::close_cross).clicked() {
                    close = true;
                }
                if (exited || self.terminal.is_none()) && ui.small_button("Restart").clicked() {
                    restart = true;
                }
            });
        });
        if close {
            self.show_terminal = false;
            self.terminal_has_focus = false;
            self.focus_editor = true;
            return;
        }
        if restart {
            self.terminal = None;
            self.ensure_terminal();
            self.terminal_focus_request = true;
        }
        if ui.available_width() > 640.0
            && let Some(Ok(status)) = &self.git_status
            && let Some(branch) = &status.branch
        {
            egui::Panel::right("branch-card")
                .exact_size(290.0)
                .resizable(false)
                .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                .show(ui, |ui| {
                    egui::Frame::new()
                        .stroke(egui::Stroke::new(1.0, theme::border()))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                egui::RichText::new("Current Branch")
                                    .size(13.5)
                                    .color(theme::text()),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(16.0, 16.0),
                                    egui::Sense::hover(),
                                );
                                icons::branch(ui.painter(), rect, theme::text_dim());
                                ui.label(egui::RichText::new(branch).color(theme::text()));
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                for (label, count, up) in [
                                    ("Ahead", status.ahead, true),
                                    ("Behind", status.behind, false),
                                ] {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(12.0, 16.0),
                                        egui::Sense::hover(),
                                    );
                                    icons::arrow(
                                        ui.painter(),
                                        rect.center(),
                                        up,
                                        theme::text_dim(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{label} {count}"))
                                            .color(theme::text_dim()),
                                    );
                                    ui.add_space(8.0);
                                }
                            });
                        });
                });
        }
        let request = std::mem::take(&mut self.terminal_focus_request);
        match self.terminal.as_mut() {
            Some(terminal) => self.terminal_has_focus = terminal.show(ui, request),
            None => {
                self.terminal_has_focus = false;
                let message = self
                    .terminal_error
                    .clone()
                    .unwrap_or_else(|| "The terminal is not running.".into());
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new(message).color(theme::text_dim()));
                });
            }
        }
    }

    fn select_range(&mut self, range: std::ops::Range<usize>) {
        self.pending_select = Some((range.start, range.end));
    }

    fn goto_line(&mut self, line: usize) {
        if let Some(tab) = self.tabs.tabs.get(self.tabs.active) {
            let start = find::line_start(tab.doc.text(), line);
            self.pending_select = Some((start, start));
            self.focus_editor = true;
        }
    }

    /// Keep the match list current and, when the query changes, jump to the first match.
    fn refresh_find(&mut self) {
        if !self.find.open {
            return;
        }
        let Some(tab) = self.tabs.tabs.get(self.tabs.active) else {
            return;
        };
        let cursor = find::line_start(tab.doc.text(), self.cursor.0) + self.cursor.1 - 1;
        self.find.refresh(tab.id, tab.doc.text(), cursor);
        let query_changed = self.find.query != self.last_find_query;
        if query_changed || self.reselect_find {
            self.last_find_query = self.find.query.clone();
            self.reselect_find = false;
            if let Some(range) = self
                .find
                .current
                .and_then(|i| self.find.matches.get(i))
                .cloned()
            {
                self.select_range(range);
            }
        }
    }

    fn find_intent(&mut self, intent: find_bar::Intent) {
        use find_bar::Intent;
        match intent {
            Intent::Next | Intent::Previous => {
                if let Some(range) = self.find.step(matches!(intent, Intent::Next)) {
                    self.select_range(range);
                }
            }
            Intent::Close => {
                self.find.open = false;
                self.focus_editor = true;
            }
            Intent::ReplaceOne => {
                let range = self
                    .find
                    .current
                    .and_then(|i| self.find.matches.get(i))
                    .cloned();
                if let (Some(range), Some(tab)) = (range, self.tabs.tabs.get_mut(self.tabs.active))
                    && !tab.readonly
                {
                    let replaced =
                        find::replace_range(tab.doc.text(), range, &self.find.replacement);
                    *tab.doc.text_mut() = replaced;
                    self.reselect_find = true;
                }
            }
            Intent::ReplaceAll => {
                let count = self.find.matches.len();
                if let Some(tab) = self.tabs.tabs.get_mut(self.tabs.active)
                    && !tab.readonly
                {
                    let replaced = find::replace_all(
                        tab.doc.text(),
                        &self.find.matches,
                        &self.find.replacement,
                    );
                    *tab.doc.text_mut() = replaced;
                    self.status = format!("Replaced {count} occurrence(s)");
                }
            }
        }
    }

    fn handle_shortcuts(&mut self, ui: &egui::Ui) {
        let command = Modifiers::COMMAND;
        let command_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        if self.terminal_has_focus {
            // The shell owns plain Ctrl keys (Ctrl+C, Ctrl+W...); only app-level chords work here.
            if shortcut(ui, command, Key::Backtick) {
                self.run_command(Command::ToggleTerminal, false);
            } else if shortcut(ui, command_shift, Key::P) {
                self.run_command(Command::GoToLine, true);
            }
            return;
        }
        // Most specific first: every Ctrl+Shift chord is tested before its plain Ctrl twin,
        // otherwise Ctrl+G would swallow Ctrl+Shift+G.
        if shortcut(ui, command_shift, Key::P) {
            self.run_command(Command::GoToLine, true);
        } else if shortcut(ui, command_shift, Key::F) {
            self.run_command(Command::ShowSearch, false);
        } else if shortcut(ui, command_shift, Key::S) {
            self.save_active(true);
        } else if shortcut(ui, command_shift, Key::G) {
            self.switch_view(SidebarView::SourceControl);
        } else if shortcut(ui, command_shift, Key::E) {
            self.switch_view(SidebarView::Explorer);
        } else if shortcut(ui, command_shift, Key::R) {
            self.switch_view(SidebarView::Room);
        } else if shortcut(ui, command_shift, Key::O) {
            self.open_folder_dialog();
        } else if shortcut(ui, command, Key::P) {
            self.run_command(Command::QuickOpen, false);
        } else if shortcut(ui, command, Key::F) {
            self.run_command(Command::Find, false);
        } else if shortcut(ui, command, Key::H) {
            self.run_command(Command::Replace, false);
        } else if shortcut(ui, command, Key::G) {
            self.run_command(Command::GoToLine, false);
        } else if shortcut(ui, command, Key::Backtick) {
            self.run_command(Command::ToggleTerminal, false);
        } else if shortcut(ui, command, Key::S) {
            self.save_active(false);
        } else if shortcut(ui, command, Key::O) {
            self.open_file_dialog();
        } else if shortcut(ui, command, Key::N) {
            self.new_file();
        } else if shortcut(ui, command, Key::W) {
            if !self.tabs.tabs.is_empty() {
                self.request_close(self.tabs.active);
            }
        } else if shortcut(ui, command, Key::B) {
            self.show_sidebar = !self.show_sidebar;
        } else if shortcut(ui, command, Key::Tab) {
            self.tabs.next();
            self.focus_editor = true;
        }
    }

    /// Custom title bar: brand and menus on the left, the command palette pill in the middle,
    /// window controls on the right. The empty space drags the window.
    fn title_bar(&mut self, ui: &mut egui::Ui) {
        let bar = ui.max_rect();
        let drag = ui.interact(
            bar,
            ui.scope_id().with("title-drag"),
            egui::Sense::click_and_drag(),
        );
        if drag.drag_started_by(egui::PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
        if drag.double_clicked() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
        ui.painter().line_segment(
            [bar.left_bottom(), bar.right_bottom()],
            egui::Stroke::new(1.0, theme::border()),
        );

        // Command palette pill, centred on the whole bar.
        let pill = egui::Rect::from_center_size(
            egui::pos2(bar.center().x, bar.center().y),
            egui::vec2((bar.width() * 0.34).clamp(200.0, 460.0), 30.0),
        );
        let pill_response = ui.interact(
            pill,
            ui.scope_id().with("title-palette"),
            egui::Sense::click(),
        );
        let painter = ui.painter();
        painter.rect_filled(
            pill,
            9.0,
            if pill_response.hovered() {
                theme::mix(theme::bg_editor(), theme::text(), 0.07)
            } else {
                theme::mix(theme::bg_editor(), theme::text(), 0.03)
            },
        );
        painter.rect_stroke(
            pill,
            9.0,
            egui::Stroke::new(1.0, theme::border()),
            egui::StrokeKind::Inside,
        );
        icons::search(
            painter,
            egui::Rect::from_center_size(
                egui::pos2(pill.left() + 20.0, pill.center().y),
                egui::vec2(14.0, 14.0),
            ),
            theme::text_dim(),
        );
        painter.text(
            egui::pos2(pill.left() + 36.0, pill.center().y),
            egui::Align2::LEFT_CENTER,
            "Command Palette (Ctrl+Shift+P)",
            egui::FontId::proportional(13.5),
            theme::text_dim(),
        );
        if pill_response.clicked() {
            self.run_command(Command::GoToLine, true);
        }

        if bar.width() > 980.0 {
            let room_pill = egui::Rect::from_min_size(
                egui::pos2(pill.right() + 16.0, pill.top()),
                egui::vec2(230.0, 30.0),
            );
            let response = ui.interact(
                room_pill,
                ui.scope_id().with("title-room"),
                egui::Sense::click(),
            );
            let painter = ui.painter();
            let lit = self.show_right && self.right_tab == RightTab::Room;
            painter.rect_filled(
                room_pill,
                9.0,
                if lit {
                    theme::mix(theme::bg_editor(), theme::accent(), 0.18)
                } else if response.hovered() {
                    theme::mix(theme::bg_editor(), theme::text(), 0.07)
                } else {
                    theme::mix(theme::bg_editor(), theme::text(), 0.03)
                },
            );
            painter.rect_stroke(
                room_pill,
                9.0,
                egui::Stroke::new(
                    1.0,
                    if lit {
                        theme::mix(theme::bg_editor(), theme::accent(), 0.6)
                    } else {
                        theme::border()
                    },
                ),
                egui::StrokeKind::Inside,
            );
            icons::room(
                painter,
                egui::Rect::from_center_size(
                    egui::pos2(room_pill.left() + 22.0, room_pill.center().y),
                    egui::vec2(18.0, 18.0),
                ),
                theme::text(),
            );
            let name = match self.room.summary() {
                Some((name, _, _)) => format!("Room: {name}"),
                None => "Room".to_string(),
            };
            painter.text(
                egui::pos2(room_pill.left() + 40.0, room_pill.center().y),
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(13.5),
                theme::text(),
            );
            if response
                .on_hover_text("Show or hide the Room panel")
                .clicked()
            {
                self.show_dock(RightTab::Room);
            }
        }

        ui.horizontal_centered(|ui| {
            ui.add_space(14.0);
            let (logo, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
            brand::logo(ui.painter(), logo.center(), 11.5);
            ui.add_space(4.0);
            // The design has no menu strip: File, Edit and View live under the brand name.
            ui.menu_button(
                egui::RichText::new("Forge Room")
                    .size(16.0)
                    .family(fonts::semibold())
                    .color(theme::text()),
                |ui| self.menu_bar(ui),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (kind, tip) in [(0, "Close"), (1, "Maximize"), (2, "Minimize")] {
                    let (rect, response) = ui
                        .allocate_exact_size(egui::vec2(46.0, TITLE_HEIGHT), egui::Sense::click());
                    let hovered = response.hovered();
                    if hovered {
                        let fill = if kind == 0 {
                            egui::Color32::from_rgb(196, 43, 28)
                        } else {
                            theme::overlay(20)
                        };
                        ui.painter().rect_filled(rect, 0.0, fill);
                    }
                    let color = if hovered && kind == 0 {
                        egui::Color32::WHITE
                    } else {
                        theme::text()
                    };
                    let c = rect.center();
                    let stroke = egui::Stroke::new(1.2, color);
                    let painter = ui.painter();
                    match kind {
                        0 => icons::close_cross(painter, c, color),
                        1 => {
                            let square = egui::Rect::from_center_size(c, egui::vec2(10.0, 10.0));
                            painter.rect_stroke(square, 1.0, stroke, egui::StrokeKind::Inside);
                        }
                        _ => {
                            painter.line_segment(
                                [c + egui::vec2(-5.0, 0.0), c + egui::vec2(5.0, 0.0)],
                                stroke,
                            );
                        }
                    }
                    if response.on_hover_text(tip).clicked() {
                        let command = match kind {
                            0 => egui::ViewportCommand::Close,
                            1 => egui::ViewportCommand::Maximized(!maximized),
                            _ => egui::ViewportCommand::Minimized(true),
                        };
                        ui.ctx().send_viewport_cmd(command);
                    }
                }
            });
        });
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        {
            ui.menu_button("File", |ui| {
                let items: [(&str, &str); 3] = [
                    ("New File", "Ctrl+N"),
                    ("Open File…", "Ctrl+O"),
                    ("Open Folder…", "Ctrl+Shift+O"),
                ];
                for (index, (label, keys)) in items.iter().enumerate() {
                    if ui
                        .add(egui::Button::new(*label).shortcut_text(*keys))
                        .clicked()
                    {
                        match index {
                            0 => self.new_file(),
                            1 => self.open_file_dialog(),
                            _ => self.open_folder_dialog(),
                        }
                        ui.close();
                    }
                }
                ui.separator();
                if ui
                    .add(egui::Button::new("Save").shortcut_text("Ctrl+S"))
                    .clicked()
                {
                    self.save_active(false);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new("Save As…").shortcut_text("Ctrl+Shift+S"))
                    .clicked()
                {
                    self.save_active(true);
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new("Close Tab").shortcut_text("Ctrl+W"))
                    .clicked()
                {
                    if !self.tabs.tabs.is_empty() {
                        self.request_close(self.tabs.active);
                    }
                    ui.close();
                }
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                for (label, keys, command) in [
                    ("Find", "Ctrl+F", Command::Find),
                    ("Replace", "Ctrl+H", Command::Replace),
                    ("Go to Line…", "Ctrl+G", Command::GoToLine),
                    ("Go to File…", "Ctrl+P", Command::QuickOpen),
                ] {
                    if ui
                        .add(egui::Button::new(label).shortcut_text(keys))
                        .clicked()
                    {
                        self.run_command(command, false);
                        ui.close();
                    }
                }
            });
            ui.menu_button("View", |ui| {
                if ui
                    .add(egui::Button::new("Terminal").shortcut_text("Ctrl+`"))
                    .clicked()
                {
                    self.run_command(Command::ToggleTerminal, false);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new("Command Palette…").shortcut_text("Ctrl+Shift+P"))
                    .clicked()
                {
                    self.run_command(Command::GoToLine, true);
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new("Explorer").shortcut_text("Ctrl+Shift+E"))
                    .clicked()
                {
                    self.switch_view(SidebarView::Explorer);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new("Search").shortcut_text("Ctrl+Shift+F"))
                    .clicked()
                {
                    self.run_command(Command::ShowSearch, false);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new("Source Control").shortcut_text("Ctrl+Shift+G"))
                    .clicked()
                {
                    self.switch_view(SidebarView::SourceControl);
                    ui.close();
                }
                if ui
                    .add(egui::Button::new("Room").shortcut_text("Ctrl+Shift+R"))
                    .clicked()
                {
                    self.switch_view(SidebarView::Room);
                    ui.close();
                }
                ui.separator();
                if ui.button("Color Theme…").clicked() {
                    self.gallery.open();
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new("Toggle Sidebar").shortcut_text("Ctrl+B"))
                    .clicked()
                {
                    self.show_sidebar = !self.show_sidebar;
                    ui.close();
                }
            });
        }
    }

    fn show_dock(&mut self, tab: RightTab) {
        if self.show_right && self.right_tab == tab {
            self.show_right = false;
        } else {
            self.show_right = true;
            self.right_tab = tab;
        }
    }

    fn switch_view(&mut self, view: SidebarView) {
        if view == SidebarView::Room {
            self.show_dock(RightTab::Room);
            return;
        }
        if self.view == view && self.show_sidebar {
            self.show_sidebar = false;
        } else {
            self.view = view;
            self.show_sidebar = true;
            if view == SidebarView::SourceControl {
                self.refresh_git();
            }
        }
    }

    fn change_count(&self) -> usize {
        match &self.git_status {
            Some(Ok(status)) => status.changes.len(),
            _ => 0,
        }
    }

    fn activity_bar(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 6.0);
        ui.add_space(12.0);
        let badge = self.change_count();
        let items: [(Rail, &str); 8] = [
            (Rail::View(SidebarView::Explorer), "Explorer (Ctrl+Shift+E)"),
            (Rail::View(SidebarView::Search), "Search (Ctrl+Shift+F)"),
            (
                Rail::View(SidebarView::SourceControl),
                "Source Control (Ctrl+Shift+G)",
            ),
            (Rail::Soon(RailIcon::Play), "Run/Debug: coming soon"),
            (Rail::Soon(RailIcon::Extensions), "Extensions: coming soon"),
            (Rail::Ai, "AI Assistant"),
            (Rail::Terminal, "Terminal (Ctrl+`)"),
            (Rail::Room, "Room (Ctrl+Shift+R)"),
        ];
        for (item, tip) in items {
            ui.horizontal(|ui| {
                ui.add_space((ACTIVITY_WIDTH - 44.0) / 2.0);
                let soon = matches!(item, Rail::Soon(_));
                let sense = if soon {
                    egui::Sense::hover()
                } else {
                    egui::Sense::click()
                };
                let (rect, response) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), sense);
                let active = match item {
                    Rail::View(view) => self.show_sidebar && self.view == view,
                    Rail::Terminal => self.show_terminal,
                    Rail::Room => self.show_right && self.right_tab == RightTab::Room,
                    Rail::Ai => self.show_right && self.right_tab == RightTab::Ai,
                    Rail::Soon(_) => false,
                };
                let hovered = response.hovered() && !soon;
                let painter = ui.painter();
                if active && matches!(item, Rail::Room) {
                    painter.rect_filled(
                        rect,
                        10.0,
                        theme::mix(theme::bg_activity(), theme::accent(), 0.25),
                    );
                    painter.rect_stroke(
                        rect,
                        10.0,
                        egui::Stroke::new(
                            1.0,
                            theme::mix(theme::bg_activity(), theme::accent(), 0.65),
                        ),
                        egui::StrokeKind::Inside,
                    );
                } else if active {
                    painter.rect_filled(rect, 10.0, theme::overlay(20));
                } else if hovered {
                    painter.rect_filled(rect, 10.0, theme::overlay(10));
                }
                let color = if soon {
                    theme::mix(theme::bg_activity(), theme::text_dim(), 0.55)
                } else if active || hovered {
                    theme::text()
                } else {
                    theme::text_dim()
                };
                let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(22.0, 22.0));
                match item {
                    Rail::View(SidebarView::Explorer) => icons::folder(
                        painter,
                        egui::Rect::from_center_size(icon_rect.center(), egui::vec2(18.0, 14.0)),
                        color,
                    ),
                    Rail::View(SidebarView::Search) => icons::search(painter, icon_rect, color),
                    Rail::View(SidebarView::SourceControl) => {
                        icons::branch(painter, icon_rect, color)
                    }
                    Rail::View(SidebarView::Room) | Rail::Room => {
                        icons::room(painter, icon_rect, color)
                    }
                    Rail::Soon(RailIcon::Play) => icons::play(painter, icon_rect, color),
                    Rail::Soon(RailIcon::Extensions) => {
                        icons::extensions(painter, icon_rect, color)
                    }
                    Rail::Ai => icons::sparkle(painter, icon_rect, color),
                    Rail::Terminal => icons::terminal(painter, icon_rect, color),
                }
                let count = match item {
                    Rail::View(SidebarView::SourceControl) => badge,
                    Rail::Room => self.room.open_task_count(),
                    _ => 0,
                };
                if count > 0 {
                    let center = egui::pos2(rect.right() - 4.0, rect.top() + 6.0);
                    painter.circle_filled(center, 8.0, theme::badge());
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        count.min(99),
                        egui::FontId::proportional(10.5),
                        theme::badge_text(),
                    );
                }
                if response.on_hover_text(tip).clicked() {
                    match item {
                        Rail::View(view) => self.switch_view(view),
                        Rail::Terminal => self.run_command(Command::ToggleTerminal, false),
                        Rail::Room => self.show_dock(RightTab::Room),
                        Rail::Ai => self.show_dock(RightTab::Ai),
                        Rail::Soon(_) => {}
                    }
                }
            });
        }
    }

    // ---- Version control ---------------------------------------------------------------

    fn repo_root(&self) -> Option<PathBuf> {
        self.workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
    }

    /// Re-read `git status` and the change markers for the active file.
    fn refresh_git(&mut self) {
        self.last_git = std::time::Instant::now();
        let Some(root) = self.repo_root() else {
            self.git_status = None;
            self.is_repo = false;
            self.branch = None;
            self.hunks.clear();
            self.graph_commits.clear();
            self.graph_rows.clear();
            self.file_history.clear();
            self.history_name = None;
            return;
        };
        self.is_repo = git::is_repo(&root);
        if !self.is_repo {
            self.git_status = Some(Err("not a repository".into()));
            self.branch = None;
            self.hunks.clear();
            return;
        }
        let result = git::status(&root);
        self.branch = match &result {
            Ok(status) => status
                .branch
                .clone()
                .or_else(|| workspace::git_branch(&root)),
            Err(_) => workspace::git_branch(&root),
        };
        self.git_status = Some(result);
        self.graph_commits = git::log_all(&root, 80).unwrap_or_default();
        self.graph_rows = graph::layout(&self.graph_commits);
        self.stash_count = git::stash_list(&root).len();
        self.refresh_hunks();
    }

    fn refresh_hunks(&mut self) {
        self.hunks_tab = self.tabs.tabs.get(self.tabs.active).map(|tab| tab.id);
        self.hunks.clear();
        let (Some(root), Some(path)) = (self.repo_root(), self.tabs.active_path()) else {
            return;
        };
        let Ok(relative) = path.strip_prefix(&root) else {
            self.file_history.clear();
            self.history_name = None;
            self.blame.clear();
            return;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        self.file_history = git::file_log(&root, &relative, 30).unwrap_or_default();
        self.history_name = relative.rsplit('/').next().map(String::from);
        let untracked = matches!(&self.git_status, Some(Ok(status))
            if status.changes.iter().any(|c| c.path == relative && c.is_untracked()));
        self.blame = if untracked {
            Vec::new()
        } else {
            git::blame(&root, &relative)
        };
        if untracked {
            let lines = self
                .tabs
                .tabs
                .get(self.tabs.active)
                .map(|tab| gutter::line_count(tab.doc.text()))
                .unwrap_or(0);
            self.hunks.push(git::Hunk {
                kind: git::HunkKind::Added,
                start: 1,
                len: lines,
            });
        } else {
            self.hunks = git::file_hunks(&root, &relative);
        }
    }

    /// Run `work` on a background thread; the result is shown when it finishes.
    fn start_job(
        &mut self,
        label: &str,
        work: impl FnOnce(PathBuf) -> Result<String, String> + Send + 'static,
    ) {
        if self.git_job.is_some() {
            self.status = "Another Git operation is still running".into();
            return;
        }
        let Some(root) = self.repo_root() else {
            return;
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        let context = self.egui_ctx.clone();
        std::thread::spawn(move || {
            let _ = sender.send(work(root));
            context.request_repaint();
        });
        self.git_job = Some(GitJob {
            label: label.to_string(),
            receiver,
        });
    }

    fn poll_git_job(&mut self, context: &egui::Context) {
        let Some(job) = &self.git_job else {
            return;
        };
        match job.receiver.try_recv() {
            Ok(result) => {
                self.git_job = None;
                self.status = match result {
                    Ok(message) => message,
                    Err(error) => format!("Git: {error}"),
                };
                self.refresh_git();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                context.request_repaint_after(std::time::Duration::from_millis(150));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.git_job = None,
        }
    }

    fn has_upstream(&self) -> bool {
        matches!(&self.git_status, Some(Ok(status)) if status.upstream.is_some())
    }

    fn git_action(&mut self, action: source_control::Action) {
        use source_control::Action;
        let Some(root) = self.repo_root() else {
            return;
        };
        let upstream = self.has_upstream();
        let (ahead, behind) = match &self.git_status {
            Some(Ok(status)) => (status.ahead, status.behind),
            _ => (0, 0),
        };
        let outcome: Result<String, String> = match action {
            Action::None => return,
            Action::Refresh => {
                self.refresh_git();
                return;
            }
            Action::Init => {
                git::run(&root, &["init"]).map(|_| "Initialized a Git repository".into())
            }
            Action::Stage(paths) => git::stage(&root, &paths).map(|_| String::new()),
            Action::Unstage(paths) => git::unstage(&root, &paths).map(|_| String::new()),
            Action::Commit => {
                let message = self.sc.message.trim().to_string();
                git::commit(&root, &message).map(|hash| {
                    self.sc.message.clear();
                    format!("Committed {hash}")
                })
            }
            Action::CommitAndPush => {
                let message = self.sc.message.trim().to_string();
                match git::commit(&root, &message) {
                    Ok(hash) => {
                        self.sc.message.clear();
                        self.status = format!("Committed {hash}");
                        self.refresh_git();
                        self.start_job("Pushing…", move |root| git::push(&root, upstream));
                        return;
                    }
                    Err(error) => Err(error),
                }
            }
            Action::Amend => {
                let message = self.sc.message.trim().to_string();
                git::amend(&root, &message).map(|hash| {
                    self.sc.message.clear();
                    format!("Amended commit {hash}")
                })
            }
            Action::UndoCommit => git::undo_last_commit(&root)
                .map(|_| "Undid the last commit (changes kept staged)".into()),
            Action::DiscardAsk(changes) => {
                self.pending_discard = Some(changes);
                return;
            }
            Action::Sync => {
                self.start_job("Syncing changes…", move |root| {
                    if behind > 0 {
                        git::pull(&root)?;
                    }
                    if ahead > 0 {
                        git::push(&root, true)?;
                    }
                    Ok("Synced".into())
                });
                return;
            }
            Action::Publish => {
                self.start_job("Publishing branch…", |root| git::push(&root, false));
                return;
            }
            Action::Pull => {
                self.start_job("Pulling…", |root| git::pull(&root));
                return;
            }
            Action::Push => {
                self.start_job("Pushing…", move |root| git::push(&root, upstream));
                return;
            }
            Action::Fetch => {
                self.start_job("Fetching…", |root| git::fetch(&root));
                return;
            }
            Action::Stash => git::stash_push(&root, ""),
            Action::StashPop => git::stash_pop(&root, 0),
            Action::ShowCommit(hash) => {
                match git::show(&root, &hash) {
                    Ok(text) => {
                        let short = &hash[..hash.len().min(7)];
                        self.tabs
                            .open_virtual(format!("Commit {short}"), text, "diff");
                    }
                    Err(error) => self.status = format!("Cannot show commit: {error}"),
                }
                return;
            }
            Action::OpenFile(path) => {
                self.open_path(root.join(path));
                return;
            }
            Action::ShowDiff { path, staged } => {
                match git::diff(&root, &path, staged) {
                    Ok(text) if text.trim().is_empty() => self.open_path(root.join(&path)),
                    Ok(text) => {
                        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                        let label = if staged {
                            format!("{name} (staged)")
                        } else {
                            format!("{name} (changes)")
                        };
                        self.tabs.open_virtual(label, text, "diff");
                    }
                    Err(error) => self.status = format!("Cannot show diff: {error}"),
                }
                return;
            }
        };
        match outcome {
            Ok(message) if !message.is_empty() => self.status = message,
            Ok(_) => {}
            Err(error) => self.status = format!("Git: {error}"),
        }
        self.refresh_git();
    }

    /// After a branch switch, re-read open files that have no unsaved edits.
    fn reload_clean_tabs(&mut self) {
        for tab in &mut self.tabs.tabs {
            if tab.doc.is_dirty() || tab.readonly {
                continue;
            }
            if let Some(path) = tab.doc.path().map(std::path::Path::to_path_buf) {
                let _ = tab.doc.open(path);
            }
        }
    }

    /// Ask before throwing work away, then discard.
    fn discard_dialog(&mut self, context: &egui::Context) {
        let Some(changes) = &self.pending_discard else {
            return;
        };
        let untracked = changes.iter().filter(|c| c.is_untracked()).count();
        let message = if changes.len() == 1 {
            format!(
                "Discard changes to \"{}\"? This cannot be undone.",
                changes[0]
                    .path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&changes[0].path)
            )
        } else {
            format!(
                "Discard all {} changed files? This cannot be undone.",
                changes.len()
            )
        };
        let note =
            (untracked > 0).then(|| format!("{untracked} untracked file(s) will be deleted."));
        let mut choice: Option<bool> = None;
        let modal = egui::Modal::new(egui::Id::unique("discard-dialog")).show(context, |ui| {
            ui.set_width(380.0);
            ui.heading("Discard changes");
            ui.add_space(6.0);
            ui.label(message);
            if let Some(note) = note {
                ui.label(egui::RichText::new(note).color(theme::text_dim()));
            }
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                let discard = egui::Button::new(
                    egui::RichText::new("Discard")
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(190, 70, 70));
                if ui.add(discard).clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(false);
                }
            });
        });
        if modal.should_close() && choice.is_none() {
            choice = Some(false);
        }
        let Some(confirmed) = choice else {
            return;
        };
        let changes = self.pending_discard.take().unwrap_or_default();
        if confirmed && let Some(root) = self.repo_root() {
            let mut failed = None;
            for change in &changes {
                if let Err(error) = git::discard(&root, change) {
                    failed = Some(error);
                    break;
                }
            }
            self.status = match failed {
                Some(error) => format!("Git: {error}"),
                None => "Discarded changes".into(),
            };
            self.refresh_git();
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        // Keep the panel width whatever the view; short content must not shrink it.
        ui.set_min_width(ui.available_width());
        match self.view {
            SidebarView::Explorer => {
                // Like the design: the file tree on top, a compact Source Control below it.
                if self.workspace.is_some() && self.is_repo {
                    egui::Panel::bottom("sidebar-scm")
                        .resizable(true)
                        .default_size(330.0)
                        .size_range(170.0..=620.0)
                        .frame(
                            egui::Frame::new()
                                .fill(theme::bg_sidebar())
                                .stroke(egui::Stroke::new(1.0, theme::border())),
                        )
                        .show(ui, |ui| self.source_control_view(ui, true));
                }
                self.explorer_view(ui);
            }
            SidebarView::Search => {
                let root = self.repo_root();
                if let search_view::Action::Open {
                    path,
                    line,
                    column,
                    length,
                } = self.search.show(ui, root.as_deref())
                {
                    self.open_path(path);
                    if let Some(tab) = self.tabs.tabs.get(self.tabs.active) {
                        let start = find::line_start(tab.doc.text(), line) + column - 1;
                        self.pending_select = Some((start, start + length));
                    }
                }
            }
            SidebarView::Room => {
                let has_workspace = self.workspace.is_some();
                self.room.show(ui, has_workspace);
            }
            SidebarView::SourceControl => self.source_control_view(ui, false),
        }
    }

    fn source_control_view(&mut self, ui: &mut egui::Ui, compact: bool) {
        let repo = if self.workspace.is_none() {
            source_control::Repo::NoFolder
        } else if !self.is_repo {
            source_control::Repo::NoRepository
        } else {
            match &self.git_status {
                Some(Ok(status)) => source_control::Repo::Ready(source_control::View {
                    status,
                    commits: &self.graph_commits,
                    rows: &self.graph_rows,
                    history: &self.file_history,
                    history_name: self.history_name.as_deref(),
                    stashes: self.stash_count,
                    busy: self.git_job.as_ref().map(|job| job.label.as_str()),
                }),
                Some(Err(error)) => source_control::Repo::Error(error),
                None => source_control::Repo::NoRepository,
            }
        };
        let action = source_control::show(ui, &mut self.sc, repo, compact);
        self.git_action(action);
    }

    fn explorer_view(&mut self, ui: &mut egui::Ui) {
        let active = self.tabs.active_path().map(|path| path.to_path_buf());
        match explorer::show(ui, &mut self.workspace, active.as_deref()) {
            explorer::Action::None => {}
            explorer::Action::OpenFile(path) => self.open_path(path),
            explorer::Action::OpenFolder => self.open_folder_dialog(),
            explorer::Action::NewFile => self.new_file(),
            explorer::Action::Refresh => {
                if let Some(workspace) = self.workspace.as_mut() {
                    workspace.refresh();
                }
                self.refresh_git();
            }
        }
    }

    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        let bar = ui.available_rect_before_wrap();
        ui.painter().rect_filled(
            egui::Rect::from_min_size(bar.min, egui::vec2(bar.width(), TAB_HEIGHT)),
            0.0,
            theme::bg_editor(),
        );
        let mut activate = None;
        let mut close = None;
        let mut new_tab = false;

        egui::ScrollArea::horizontal()
            .auto_shrink([false, true])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (index, tab) in self.tabs.tabs.iter().enumerate() {
                        let is_active = index == self.tabs.active;
                        let title = tab.title();
                        let galley = ui.painter().layout_no_wrap(
                            title,
                            egui::FontId::proportional(14.0),
                            theme::text(),
                        );
                        let width = (galley.size().x + 56.0).max(110.0);
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(width, TAB_HEIGHT),
                            egui::Sense::click(),
                        );
                        let close_rect = egui::Rect::from_center_size(
                            egui::pos2(rect.right() - 18.0, rect.center().y),
                            egui::vec2(18.0, 18.0),
                        );
                        let close_response = ui.interact(
                            close_rect,
                            ui.scope_id().with(("tab-close", tab.id)),
                            egui::Sense::click(),
                        );
                        let hovered = response.hovered() || close_response.hovered();

                        let painter = ui.painter();
                        if hovered && !is_active {
                            painter.rect_filled(
                                rect.shrink2(egui::vec2(2.0, 5.0)),
                                7.0,
                                theme::overlay(10),
                            );
                        }
                        if is_active {
                            painter.rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(rect.left() + 8.0, rect.bottom() - 2.0),
                                    egui::pos2(rect.right() - 8.0, rect.bottom()),
                                ),
                                1.0,
                                theme::accent(),
                            );
                        }
                        let text_color = if is_active {
                            theme::text()
                        } else {
                            theme::text_dim()
                        };
                        painter.galley(
                            egui::pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0),
                            galley,
                            text_color,
                        );
                        if tab.doc.is_dirty() && !hovered {
                            painter.circle_filled(close_rect.center(), 4.0, theme::text_dim());
                        } else if is_active || hovered {
                            if close_response.hovered() {
                                painter.circle_filled(close_rect.center(), 9.0, theme::overlay(22));
                            }
                            icons::close_cross(painter, close_rect.center(), theme::text());
                        }

                        if close_response.clicked() || response.middle_clicked() {
                            close = Some(index);
                        } else if response.clicked() {
                            activate = Some(index);
                        }
                    }
                    ui.add_space(6.0);
                    if icons::icon_button(ui, "New file (Ctrl+N)", icons::plus).clicked() {
                        new_tab = true;
                    }
                });
            });

        if new_tab {
            self.new_file();
        }
        if let Some(index) = activate {
            self.tabs.active = index;
            self.focus_editor = true;
        }
        if let Some(index) = close {
            self.request_close(index);
        }
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let focus = std::mem::take(&mut self.focus_editor);
        let font_id = egui::TextStyle::Monospace.resolve(ui.style());
        let (row_height, char_width) =
            ui.fonts_mut(|fonts| (fonts.row_height(&font_id), fonts.glyph_width(&font_id, '0')));
        let current_line = self.cursor.0 - 1;
        let mut cursor_index = None;
        let hunks = self.hunks.clone();
        let blame_line = if self.blame_enabled {
            self.blame.get(self.cursor.0 - 1).cloned()
        } else {
            None
        };
        let mut blame_click: Option<String> = None;
        let select = self.pending_select.take();
        let find_matches = if self.find.open {
            self.find.matches.clone()
        } else {
            Vec::new()
        };
        let find_current = self.find.current;
        let Some(tab) = self.tabs.active_tab() else {
            self.welcome(ui);
            return;
        };
        let readonly = tab.readonly;
        let tab_dirty = tab.doc.is_dirty();
        let id = egui::Id::unique(("editor", tab.id));
        let extension = tab.syntax.clone().unwrap_or_else(|| {
            tab.doc
                .path()
                .and_then(|path| path.extension())
                .map(|extension| extension.to_string_lossy().into_owned())
                .unwrap_or_else(|| "txt".into())
        });
        let line_count = gutter::line_count(tab.doc.text());
        let gutter_width = gutter::gutter_digits(line_count) as f32 * char_width + 30.0;

        egui::ScrollArea::both()
            .id_salt(("scroll", tab.id))
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let rows = (ui.available_height() / row_height).floor() as usize;
                    // Only the width matters here: numbers are painted from the real text layout below.
                    let (gutter, _) =
                        ui.allocate_exact_size(egui::vec2(gutter_width, 1.0), egui::Sense::hover());

                    let mut layouter =
                        |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap_width: f32| {
                            let mut job = if extension == "diff" {
                                diff_job(text.as_str(), &font_id)
                            } else {
                                syntax::highlight(text.as_str(), &extension, &font_id)
                            };
                            job.wrap.max_width = f32::INFINITY; // no soft wrap, so gutter numbers match rows
                            ui.fonts_mut(|fonts| fonts.layout_job(job))
                        };
                    if tab.fresh {
                        // egui starts a new editor with the cursor at the end; VS Code starts at 1:1.
                        tab.fresh = false;
                        let mut state =
                            egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(
                                egui::text::CCursor::default(),
                            )));
                        state.store(ui.ctx(), id);
                    }
                    let mut scroll_to = None;
                    if let Some((start, end)) = select {
                        let mut state =
                            egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(start),
                                egui::text::CCursor::new(end),
                            )));
                        state.store(ui.ctx(), id);
                        scroll_to = Some(start);
                    }
                    let output = egui::TextEdit::multiline(tab.doc.text_mut())
                        .id(id)
                        .font(font_id.clone())
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .desired_width(f32::INFINITY)
                        .desired_rows(rows.max(10))
                        .lock_focus(true)
                        .interactive(!readonly)
                        .layouter(&mut layouter)
                        .show(ui);
                    if focus {
                        output.response.request_focus();
                    }

                    // Find matches, painted over the text with a soft wash.
                    {
                        let clip = ui.clip_rect();
                        let origin = output.galley_pos.to_vec2();
                        for (index, range) in find_matches.iter().enumerate() {
                            let start = output
                                .galley
                                .pos_from_cursor(egui::text::CCursor::new(range.start));
                            let end = output
                                .galley
                                .pos_from_cursor(egui::text::CCursor::new(range.end));
                            let top = start.top() + origin.y;
                            if top + start.height() < clip.top() {
                                continue;
                            }
                            if top > clip.bottom() {
                                break;
                            }
                            if (start.top() - end.top()).abs() > 1.0 {
                                continue; // a match never spans rows without wrapping; skip oddities
                            }
                            let rect = egui::Rect::from_min_max(
                                egui::pos2(start.left() + origin.x, top),
                                egui::pos2(end.left() + origin.x, top + start.height()),
                            );
                            let current = find_current == Some(index);
                            let wash = theme::accent();
                            ui.painter().rect_filled(
                                rect,
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(
                                    wash.r(),
                                    wash.g(),
                                    wash.b(),
                                    if current { 120 } else { 55 },
                                ),
                            );
                            if current {
                                ui.painter().rect_stroke(
                                    rect,
                                    2.0,
                                    egui::Stroke::new(1.0, theme::accent()),
                                    egui::epaint::StrokeKind::Outside,
                                );
                            }
                        }
                        if let Some(start) = scroll_to {
                            let rect = output
                                .galley
                                .pos_from_cursor(egui::text::CCursor::new(start))
                                .translate(origin);
                            ui.scroll_to_rect(
                                rect.expand2(egui::vec2(60.0, 80.0)),
                                Some(egui::Align::Center),
                            );
                        }
                    }

                    // Gutter numbers and current-line highlight, placed from the galley rows so
                    // they stay aligned with the text whatever the font's reported row height.
                    let clip = ui.clip_rect();
                    let painter = ui.painter();
                    for (line, placed) in output.galley.rows.iter().enumerate() {
                        let top = output.galley_pos.y + placed.min_y();
                        let height = placed.height();
                        if top + height < clip.top() {
                            continue;
                        }
                        if top > clip.bottom() {
                            break;
                        }
                        let current = line == current_line;
                        if current
                            && !tab_dirty
                            && let Some(info) = &blame_line
                        {
                            let text = if info.is_uncommitted() {
                                "You · Uncommitted changes".to_string()
                            } else {
                                format!(
                                    "{}, {} · {}",
                                    info.author,
                                    room_state::ago(room_state::now(), info.time.max(0) as u64),
                                    info.summary
                                )
                            };
                            let galley = painter.layout_no_wrap(
                                text,
                                egui::FontId::proportional(12.5),
                                theme::mix(theme::bg_editor(), theme::text(), 0.38),
                            );
                            let at = egui::pos2(
                                output.galley_pos.x + placed.rect().right() + 36.0,
                                top + (height - galley.size().y) / 2.0,
                            );
                            let area = egui::Rect::from_min_size(at, galley.size());
                            painter.galley(at, galley, theme::text_dim());
                            if !info.is_uncommitted()
                                && ui
                                    .interact(
                                        area,
                                        ui.scope_id().with("blame-annotation"),
                                        egui::Sense::click(),
                                    )
                                    .on_hover_text(format!(
                                        "{} \u{2022} {}",
                                        info.author, info.summary
                                    ))
                                    .clicked()
                            {
                                blame_click = Some(info.hash.clone());
                            }
                        }
                        if let Some(hunk) = hunks.iter().find(|hunk| hunk.covers(line + 1)) {
                            let x = gutter.right() - 7.0;
                            let (color, bar) = match hunk.kind {
                                git::HunkKind::Added => (
                                    egui::Color32::from_rgb(87, 166, 74),
                                    egui::Rect::from_min_max(
                                        egui::pos2(x, top),
                                        egui::pos2(x + 3.0, top + height),
                                    ),
                                ),
                                git::HunkKind::Modified => (
                                    egui::Color32::from_rgb(79, 152, 214),
                                    egui::Rect::from_min_max(
                                        egui::pos2(x, top),
                                        egui::pos2(x + 3.0, top + height),
                                    ),
                                ),
                                git::HunkKind::Deleted => (
                                    egui::Color32::from_rgb(229, 83, 75),
                                    egui::Rect::from_min_max(
                                        egui::pos2(x - 2.0, top - 1.5),
                                        egui::pos2(x + 6.0, top + 1.5),
                                    ),
                                ),
                            };
                            painter.rect_filled(bar, 1.0, color);
                        }
                        if current {
                            painter.rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(gutter.left(), top),
                                    egui::pos2(clip.right().max(gutter.right()), top + height),
                                ),
                                0.0,
                                theme::line_highlight(),
                            );
                        }
                        painter.text(
                            egui::pos2(gutter.right() - 16.0, top),
                            egui::Align2::RIGHT_TOP,
                            line + 1,
                            font_id.clone(),
                            if current {
                                theme::text()
                            } else {
                                theme::line_number()
                            },
                        );
                    }
                    cursor_index = output.cursor_range.map(|range| range.primary.index.0);
                });
            });
        if let Some(hash) = blame_click {
            self.git_action(source_control::Action::ShowCommit(hash));
        }
        if let (Some(index), Some(tab)) = (cursor_index, self.tabs.tabs.get(self.tabs.active)) {
            self.cursor = position::line_col(tab.doc.text(), index);
        }
    }

    /// Path of the active file relative to the opened folder, like VS Code's breadcrumb bar.
    fn breadcrumbs(&self, ui: &mut egui::Ui) {
        let Some(tab) = self.tabs.tabs.get(self.tabs.active) else {
            return;
        };
        let parts: Vec<String> = match tab.doc.path() {
            Some(path) => {
                let relative = self
                    .workspace
                    .as_ref()
                    .and_then(|workspace| path.strip_prefix(&workspace.root).ok())
                    .unwrap_or(path);
                let mut parts: Vec<String> = relative
                    .components()
                    .filter_map(|part| match part {
                        std::path::Component::Normal(name) => {
                            Some(name.to_string_lossy().into_owned())
                        }
                        _ => None,
                    })
                    .collect();
                if self.workspace.is_none() && parts.len() > 3 {
                    parts.drain(..parts.len() - 3);
                }
                parts
            }
            None => vec![tab.title()],
        };
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(16, 5))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let last = parts.len().saturating_sub(1);
                    for (index, part) in parts.iter().enumerate() {
                        if index > 0 {
                            ui.label(egui::RichText::new("\u{203a}").color(theme::text_dim()));
                        }
                        let color = if index == last {
                            theme::text()
                        } else {
                            theme::text_dim()
                        };
                        ui.label(egui::RichText::new(part).size(12.5).color(color));
                    }
                });
            });
    }

    fn welcome(&mut self, ui: &mut egui::Ui) {
        let area = ui.available_rect_before_wrap();
        brand::welcome_backdrop(&ui.painter_at(area), area);
        let center_x = area.center().x;
        let top = area.top() + (area.height() * 0.2).max(40.0);
        let painter = ui.painter_at(area);
        brand::logo(&painter, egui::pos2(center_x, top + 36.0), 34.0);
        brand::wordmark(&painter, egui::pos2(center_x, top + 104.0), 26.0);
        painter.text(
            egui::pos2(center_x, top + 134.0),
            egui::Align2::CENTER_CENTER,
            "E A R L Y   P R E V I E W",
            egui::FontId::proportional(11.5),
            theme::text_dim(),
        );

        let width = 320.0;
        let mut y = top + 178.0;
        let entries: [(&str, &str, bool); 3] = [
            ("New File", "Ctrl+N", false),
            ("Open File…", "Ctrl+O", false),
            ("Open Folder…", "Ctrl+Shift+O", true),
        ];
        for (index, (label, keys, primary)) in entries.iter().enumerate() {
            let rect = egui::Rect::from_min_size(
                egui::pos2(center_x - width / 2.0, y),
                egui::vec2(width, 38.0),
            );
            let mut button = egui::Button::new(egui::RichText::new(*label).color(if *primary {
                theme::button_text()
            } else {
                theme::text()
            }))
            .shortcut_text(*keys)
            .corner_radius(6.0);
            if *primary {
                button = button.fill(theme::button()).stroke(egui::Stroke::NONE);
            }
            if ui.put(rect, button).clicked() {
                match index {
                    0 => self.new_file(),
                    1 => self.open_file_dialog(),
                    _ => self.open_folder_dialog(),
                }
            }
            y += 46.0;
        }
        painter.text(
            egui::pos2(center_x, y + 22.0),
            egui::Align2::CENTER_CENTER,
            "Ctrl+Shift+P  Command Palette      Ctrl+P  Go to File",
            egui::FontId::proportional(12.0),
            theme::text_dim(),
        );
    }

    fn right_dock(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(8.0);
            for (tab, label, icon) in [
                (
                    RightTab::Ai,
                    "AI Assistant",
                    icons::sparkle as fn(&egui::Painter, egui::Rect, egui::Color32),
                ),
                (RightTab::Room, "Room", icons::room),
            ] {
                if underline_tab(ui, label, self.right_tab == tab, true, Some(icon)).clicked() {
                    self.right_tab = tab;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                if icons::icon_button(ui, "Hide panel", icons::close_cross).clicked() {
                    self.show_right = false;
                }
            });
        });
        ui.painter().line_segment(
            [
                egui::pos2(ui.max_rect().left(), ui.cursor().top()),
                egui::pos2(ui.max_rect().right(), ui.cursor().top()),
            ],
            egui::Stroke::new(1.0, theme::border()),
        );
        match self.right_tab {
            RightTab::Room => {
                let has_workspace = self.workspace.is_some();
                self.room.show(ui, has_workspace);
            }
            RightTab::Ai => ai_panel(ui),
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(branch) = self.branch.clone() {
                let (rect, icon) =
                    ui.allocate_exact_size(egui::vec2(14.0, 16.0), egui::Sense::click());
                icons::branch(ui.painter(), rect, theme::status_text());
                let text = branch;
                let (ahead, behind) = match &self.git_status {
                    Some(Ok(status)) => (status.ahead, status.behind),
                    _ => (0, 0),
                };
                let label = ui.add(
                    egui::Label::new(egui::RichText::new(text).color(theme::status_text()))
                        .sense(egui::Sense::click()),
                );
                if icon.clicked() || label.on_hover_text("Switch branch").clicked() {
                    self.run_command(Command::SwitchBranch, false);
                }
                for (count, up) in [(behind, false), (ahead, true)] {
                    if count > 0 {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, 16.0), egui::Sense::hover());
                        icons::arrow(ui.painter(), rect.center(), up, theme::status_text());
                        ui.label(
                            egui::RichText::new(count.to_string()).color(theme::status_text()),
                        );
                    }
                }
                ui.add_space(10.0);
            }
            ui.label(egui::RichText::new(&self.status).color(theme::status_dim()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dim = |text: String| egui::RichText::new(text).color(theme::status_dim());
                ui.label(dim(forge_room_core::NATIVE_PREVIEW_VERSION.to_string()));
                ui.add_space(6.0);
                if let Some(tab) = self.tabs.tabs.get(self.tabs.active) {
                    ui.label(dim(position::line_ending(tab.doc.text()).to_string()));
                    ui.label(dim("UTF-8".to_string()));
                    ui.label(dim(format!("Ln {}, Col {}", self.cursor.0, self.cursor.1)));
                }
                if let Some((name, people, agents)) = self.room.summary() {
                    ui.add_space(18.0);
                    ui.label(dim(format!("{people} members • {agents} agents")));
                    let (dot, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 16.0), egui::Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(),
                        3.5,
                        egui::Color32::from_rgb(110, 200, 120),
                    );
                    ui.label(
                        egui::RichText::new(format!("Room: {name}")).color(theme::status_text()),
                    );
                }
            });
        });
    }

    fn pending_dialog(&mut self, context: &egui::Context) {
        let Some(pending) = &self.pending else {
            return;
        };
        let (message, save_label) = match pending {
            Pending::CloseTab(index) => {
                let name = self
                    .tabs
                    .tabs
                    .get(*index)
                    .map(|tab| tab.title())
                    .unwrap_or_default();
                (
                    format!("Do you want to save the changes to \"{name}\"?"),
                    "Save",
                )
            }
            Pending::Quit => {
                let count = self.tabs.tabs.iter().filter(|t| t.doc.is_dirty()).count();
                (
                    format!("{count} file(s) have unsaved changes. Save them before closing?"),
                    "Save All",
                )
            }
        };

        let mut choice = None;
        let modal = egui::Modal::new(egui::Id::unique("pending-dialog")).show(context, |ui| {
            ui.set_width(380.0);
            ui.heading("Unsaved changes");
            ui.add_space(6.0);
            ui.label(message);
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button(save_label).clicked() {
                    choice = Some(Choice::Save);
                }
                if ui.button("Don't Save").clicked() {
                    choice = Some(Choice::Discard);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(Choice::Cancel);
                }
            });
        });
        if modal.should_close() && choice.is_none() {
            choice = Some(Choice::Cancel);
        }
        let Some(choice) = choice else {
            return;
        };
        let pending = self.pending.take();
        match (pending, choice) {
            (_, Choice::Cancel) => {}
            (Some(Pending::CloseTab(index)), Choice::Save) => {
                if self.save_index(index, false) {
                    self.tabs.close(index);
                }
            }
            (Some(Pending::CloseTab(index)), Choice::Discard) => self.tabs.close(index),
            (Some(Pending::Quit), Choice::Save) => {
                let dirty: Vec<usize> = (0..self.tabs.tabs.len())
                    .filter(|index| self.tabs.tabs[*index].doc.is_dirty())
                    .collect();
                if dirty.into_iter().all(|index| self.save_index(index, false)) {
                    self.allow_close = true;
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            (Some(Pending::Quit), Choice::Discard) => {
                self.allow_close = true;
                context.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            (None, _) => {}
        }
        self.focus_editor = true;
    }

    fn update_title(&mut self, context: &egui::Context) {
        let mut title = String::new();
        if let Some(tab) = self.tabs.tabs.get(self.tabs.active) {
            if tab.doc.is_dirty() {
                title.push('●');
                title.push(' ');
            }
            title.push_str(&tab.title());
            title.push_str(" — ");
        }
        if let Some(workspace) = &self.workspace {
            title.push_str(&workspace.name());
            title.push_str(" — ");
        }
        title.push_str("Forge Room");
        if title != self.last_title {
            context.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }
}

impl eframe::App for ForgeRoomApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::from_rgb(10, 11, 12).to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        // Windows can restore a previous maximized state; always open as a normal window.
        if context.cumulative_pass_nr() == 3 {
            context.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            context.request_repaint();
        }
        if context.cumulative_pass_nr() == 6 {
            context.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1280.0, 780.0)));
            context.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(60.0, 30.0)));
            context.request_repaint();
        }
        if context.input(|input| input.viewport().close_requested())
            && self.tabs.any_dirty()
            && !self.allow_close
        {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Quit);
        }
        if self.pending.is_none() {
            self.handle_shortcuts(ui);
        }

        paint_backdrop(ui);

        let bar = |fill| {
            egui::Frame::new()
                .fill(theme::glass(fill, 0xCC))
                .inner_margin(egui::Margin::symmetric(14, 10))
        };

        // Panels are rounded cards on the darkest background, with small gaps between them.
        let card = |fill| {
            egui::Frame::new()
                .fill(theme::glass(fill, 0xEA))
                .corner_radius(12.0)
                .stroke(egui::Stroke::new(1.0, theme::border()))
                .outer_margin(egui::Margin::same(3))
        };

        if self.show_terminal && self.terminal.is_none() && self.terminal_error.is_none() {
            self.ensure_terminal();
        }

        egui::Panel::top("menu")
            .frame(egui::Frame::new().fill(theme::glass(theme::title_bar(), 0xC8)))
            .exact_size(TITLE_HEIGHT)
            .show(ui, |ui| self.title_bar(ui));

        egui::Panel::bottom("status")
            .frame(bar(theme::status_bar()))
            .show(ui, |ui| self.status_bar(ui));

        egui::Panel::left("activity")
            .exact_size(ACTIVITY_WIDTH)
            .resizable(false)
            .frame(egui::Frame::new().fill(theme::glass(theme::bg_activity(), 0xB8)))
            .show(ui, |ui| self.activity_bar(ui));

        if self.show_sidebar {
            egui::Panel::left("sidebar")
                .default_size(300.0)
                .size_range(180.0..=560.0)
                .resizable(true)
                .frame(card(theme::bg_sidebar()))
                .show(ui, |ui| self.sidebar(ui));
        }

        if self.show_right {
            egui::Panel::right("right-dock")
                .default_size(360.0)
                .size_range(280.0..=640.0)
                .resizable(true)
                .frame(card(theme::bg_sidebar()))
                .show(ui, |ui| self.right_dock(ui));
        }

        if self.show_terminal {
            egui::Panel::bottom("terminal")
                .resizable(true)
                .default_size(250.0)
                .size_range(120.0..=720.0)
                .frame(card(theme::bg_editor()).inner_margin(egui::Margin {
                    left: 0,
                    right: 0,
                    top: 4,
                    bottom: 0,
                }))
                .show(ui, |ui| self.terminal_panel(ui));
        }

        egui::CentralPanel::default()
            .frame(card(theme::bg_editor()))
            .show(ui, |ui| {
                if !self.tabs.tabs.is_empty() {
                    self.tab_bar(ui);
                    self.breadcrumbs(ui);
                }
                self.editor(ui);
            });

        let active_id = self.tabs.tabs.get(self.tabs.active).map(|tab| tab.id);
        if self.workspace.is_some() {
            if self.last_git.elapsed() >= std::time::Duration::from_secs(3) {
                self.refresh_git();
            } else if self.hunks_tab != active_id {
                self.refresh_hunks();
            }
            context.request_repaint_after(std::time::Duration::from_secs(3));
        }

        if self.search.poll() {
            context.request_repaint_after(std::time::Duration::from_millis(120));
        }
        self.refresh_find();
        for intent in find_bar::show(&context, &mut self.find) {
            self.find_intent(intent);
        }
        match palette::show(&context, &mut self.palette) {
            Some(palette::Outcome::Checkout { name, remote }) => {
                if let Some(root) = self.repo_root() {
                    self.status = match git::checkout(&root, &name, remote) {
                        Ok(()) => format!("Switched to {name}"),
                        Err(error) => format!("Git: {error}"),
                    };
                    self.refresh_git();
                    self.reload_clean_tabs();
                }
            }
            Some(palette::Outcome::CreateBranch(name)) => {
                if let Some(root) = self.repo_root() {
                    self.status = match git::create_branch(&root, &name) {
                        Ok(()) => format!("Created and switched to {name}"),
                        Err(error) => format!("Git: {error}"),
                    };
                    self.refresh_git();
                }
            }
            Some(palette::Outcome::Run(command)) => self.run_command(command, false),
            Some(palette::Outcome::Open(path)) => self.open_path(path),
            Some(palette::Outcome::GoToLine(line)) => self.goto_line(line),
            None => {}
        }

        match theme_gallery::show(&context, &mut self.gallery) {
            Some(theme_gallery::Outcome::Keep) => {
                let settings = settings::Settings {
                    theme: Some(theme::current().key.clone()),
                };
                if let Err(error) = settings::save(&settings) {
                    self.status = format!("Could not save settings: {error}");
                }
            }
            Some(theme_gallery::Outcome::Cancel(key)) => theme::set(&context, &key),
            None => {}
        }

        self.poll_git_job(&context);
        self.discard_dialog(&context);
        self.pending_dialog(&context);
        self.update_title(&context);
    }
}

/// A text tab with a red underline when active, as in the editor, dock and terminal headers.
fn underline_tab(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    enabled: bool,
    icon: Option<fn(&egui::Painter, egui::Rect, egui::Color32)>,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(14.0),
        theme::text(),
    );
    let icon_width = if icon.is_some() { 26.0 } else { 0.0 };
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(galley.size().x + icon_width + 30.0, 36.0), sense);
    let color = if active || (enabled && response.hovered()) {
        theme::text()
    } else if enabled {
        theme::text_dim()
    } else {
        theme::mix(theme::bg_editor(), theme::text_dim(), 0.55)
    };
    let painter = ui.painter();
    let mut x = rect.left() + 15.0;
    if let Some(icon) = icon {
        icon(
            painter,
            egui::Rect::from_center_size(
                egui::pos2(x + 8.0, rect.center().y),
                egui::vec2(16.0, 16.0),
            ),
            if active { theme::accent() } else { color },
        );
        x += icon_width;
    }
    painter.galley(
        egui::pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    if active {
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(rect.left() + 8.0, rect.bottom() - 2.0),
                egui::pos2(rect.right() - 8.0, rect.bottom()),
            ),
            1.0,
            theme::accent(),
        );
    }
    response
}

/// The AI Assistant tab. There is no assistant backend yet, so this shows the layout from the
/// design with every control inert and says so, rather than pretending to answer.
fn ai_panel(ui: &mut egui::Ui) {
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let (logo, _) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::hover());
        brand::logo(ui.painter(), logo.center(), 16.0);
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(
                egui::RichText::new("Forge Room AI")
                    .size(15.0)
                    .family(fonts::semibold())
                    .color(theme::text()),
            );
            ui.label(
                egui::RichText::new("Agentic coding in your workspace")
                    .size(12.5)
                    .color(theme::text_dim()),
            );
        });
    });
    ui.add_space(14.0);
    egui::Frame::new()
        .fill(theme::mix(theme::bg_sidebar(), theme::text(), 0.03))
        .stroke(egui::Stroke::new(1.0, theme::border()))
        .corner_radius(10.0)
        .outer_margin(egui::Margin::symmetric(14, 0))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new("How can I help you today?")
                    .size(16.0)
                    .family(fonts::semibold())
                    .color(theme::text()),
            );
            ui.add_space(10.0);
            ui.add_enabled_ui(false, |ui| {
                let width = (ui.available_width() - 8.0) / 2.0;
                for pair in [
                    ["Implement a feature", "Explain this code"],
                    ["Write tests", "Find and fix issues"],
                ] {
                    ui.horizontal(|ui| {
                        for label in pair {
                            ui.add(
                                egui::Button::new(label)
                                    .corner_radius(8.0)
                                    .min_size(egui::vec2(width, 34.0)),
                            );
                        }
                    });
                }
            });
        });
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.add_space(18.0);
        ui.vertical(|ui| {
            ui.set_max_width(ui.available_width() - 18.0);
            ui.label(
                egui::RichText::new("Not connected yet")
                    .strong()
                    .color(theme::text()),
            );
            ui.label(
                egui::RichText::new(
                    "Rooms already track agents and the tasks they own. Running agents from this panel is on the roadmap.",
                )
                .color(theme::text_dim()),
            );
        });
    });
    ui.add_space(14.0);
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, theme::border()))
        .corner_radius(10.0)
        .outer_margin(egui::Margin::symmetric(14, 0))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(
                    "Ask anything, or use @ to reference files, agents, or Room context…",
                )
                .color(theme::mix(theme::bg_sidebar(), theme::text_dim(), 0.7)),
            );
            ui.add_space(24.0);
        });
}

/// The window backdrop: near-black with a soft red glow in the bottom-left corner and a faint
/// cool light at the top right. Panels are drawn translucent over it, so it shows as a gentle
/// frosted-glass tint between and behind them. Light themes keep a plain background.
fn paint_backdrop(ui: &egui::Ui) {
    let screen = ui.ctx().content_rect();
    let painter = ui.painter();
    if !theme::current().palette.dark {
        painter.rect_filled(screen, 0.0, theme::bg_activity());
        return;
    }
    painter.rect_filled(screen, 0.0, egui::Color32::from_rgb(10, 11, 12));
    let accent = theme::accent();
    let glow = |center: egui::Pos2, radius: f32, color: egui::Color32, peak: u8| {
        const STEPS: u32 = 26;
        for step in 0..STEPS {
            let t = step as f32 / STEPS as f32;
            let alpha = (peak as f32 / STEPS as f32 * 1.8).max(1.0) as u8;
            painter.circle_filled(
                center,
                radius * (1.0 - t),
                egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha),
            );
        }
    };
    glow(
        egui::pos2(
            screen.left() + screen.width() * 0.04,
            screen.bottom() + 40.0,
        ),
        screen.width() * 0.34,
        accent,
        60,
    );
    glow(
        egui::pos2(screen.right() - screen.width() * 0.05, screen.top() - 30.0),
        screen.width() * 0.28,
        egui::Color32::from_rgb(120, 140, 170),
        26,
    );
}
