#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent;
mod ai_panel;
mod autosave;
mod background;
mod brand;
mod commands;
mod diagnostics;
mod document;
mod editing;
mod explorer;
mod file_ops;
mod find;
mod find_bar;
mod fonts;
mod fuzzy;
mod git;
mod graph;
mod gutter;
mod icons;
mod line_ops;
mod lobby;
mod mark;
mod palette;
mod path_links;
mod position;
mod preview;
mod project_tools;
mod recovery;
mod room_state;
mod rooms;
mod search;
mod search_view;
mod session;
mod settings;
mod source_control;
mod symbols;
mod syntax;
mod tabs;
mod terminal;
mod theme;
mod theme_gallery;
mod transforms;
mod workspace;

use commands::Command;
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use std::{fs, path::PathBuf};
use tabs::{Tabs, normalize};
use workspace::Workspace;

const APP_NAME: &str = "Forge Room Early Preview";
const TAB_HEIGHT: f32 = 42.0;
const TITLE_HEIGHT: f32 = 36.0;
const ACTIVITY_WIDTH: f32 = 112.0;

/// One entry of the navigation rail on the left.
#[derive(Clone, Copy)]
enum Rail {
    View(SidebarView),
    Terminal,
    Room,
    Ai,
    Studio,
    Project(project_tools::Page),
}

/// Tabs of the dock on the right.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Ai,
    Room,
    Chat,
    Agents,
    Tasks,
    Activity,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Terminal,
    Output,
    Problems,
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

/// Older builds let eframe save the window (maximized flag, zoom) next to the app data, and it
/// is read back on every start. Delete that file so existing installs open normally.
fn discard_saved_window_state() {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let file = PathBuf::from(appdata)
            .join("forge-room-early-preview")
            .join("data")
            .join("app.ron");
        let _ = fs::remove_file(file);
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
        // Always open as a normal, centred window. eframe's saved window state is not used:
        // it restored a maximized flag from an old session and then fought our own sizing.
        persist_window: false,
        centered: true,
        ..Default::default()
    };
    discard_saved_window_state();
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
    /// Tab `index` has unsaved edits and its file changed on disk; `Resume` is what the save was
    /// for, to carry on with once the conflict is settled.
    DiskConflict(usize, Resume),
}

/// What to do after a disk conflict that interrupted a save has been resolved.
#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum Resume {
    #[default]
    Nothing,
    /// Close the tab that was being saved.
    CloseTab,
    /// Carry on saving every dirty tab, then quit.
    Quit,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SidebarView {
    Explorer,
    Search,
    SourceControl,
    Room,
    Project(project_tools::Page),
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
    last_disk_check: std::time::Instant,
    /// Set around a save that may hit a conflict, so the conflict dialog knows what to resume.
    resume: Resume,
    lobby: lobby::Lobby,
    show_lobby: bool,
    runtimes: Vec<String>,
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
    /// The file's history, blame and changes are read on a worker thread.
    hunks_job: background::Latest<FileGit>,
    /// Folders in which the user agreed to run cargo check during this run.
    checked_folders: std::collections::HashSet<PathBuf>,
    auto_checked: std::time::Instant,
    recovery_warned: bool,
    /// Sidebar width, right dock width and terminal height as last drawn, in points.
    panel_sizes: (u16, u16, u16),
    /// The terminal panel fills most of the window.
    terminal_maximized: bool,
    /// Two terminals shown side by side: their indexes.
    split_pair: Option<(usize, usize)>,
    /// The user trusts the open folder: until then Git and project tools stay off.
    trusted: bool,
    /// The folder waiting for the answer to the trust question.
    trust_prompt: Option<PathBuf>,
    /// The top folder of the repository when the open folder is a subfolder of it.
    git_top: Option<PathBuf>,
    /// The files git reported as changed at the last read; the Explorer refreshes when it differs.
    change_paths: Vec<String>,
    terminals_confirmed: bool,
    graph_commits: Vec<git::CommitInfo>,
    graph_rows: Vec<graph::Row>,
    file_history: Vec<git::CommitInfo>,
    history_name: Option<String>,
    stash_count: usize,
    /// Editor text size relative to the default, kept in the settings.
    editor_scale: f32,
    /// Save file tabs automatically once their text rests.
    auto_save: bool,
    /// The last `cargo check`, and the worker that makes the next one.
    problems: Option<diagnostics::Report>,
    checking: background::Latest<diagnostics::Report>,
    /// The last session written to disk, and when it was last compared with the window.
    session_saved: session::Session,
    session_checked: std::time::Instant,
    auto_saves: autosave::Tracker,
    /// Unsaved text kept on disk so a crash does not lose it.
    recovery: recovery::Store,
    last_recovery: std::time::Instant,
    /// Hash of the text last written to the recovery folder, per tab id.
    recovery_written: std::collections::HashMap<u64, u64>,
    /// The repository snapshot being read on a worker thread.
    git_refresh: background::Latest<git::Snapshot>,
    operation: Option<git::Operation>,
    blame: Vec<git::BlameLine>,
    blame_enabled: bool,
    git_job: Option<GitJob>,
    pending_discard: Option<Vec<git::Change>>,
    gallery: theme_gallery::Gallery,
    room: rooms::RoomView,
    /// Every terminal, in tab order; `active_terminal` is the one shown.
    terminals: Vec<terminal::Terminal>,
    active_terminal: usize,
    show_terminal: bool,
    bottom_tab: BottomTab,
    show_right: bool,
    show_preview: bool,
    lobby_close_warned: bool,
    preview: preview::Preview,
    ai: ai_panel::State,
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
    /// A palette text transformation waiting for the editor to apply it to its selection.
    pending_edit: Option<transforms::Kind>,
    last_find_query: String,
    reselect_find: bool,
    new_folder_parent: Option<PathBuf>,
    new_folder_name: String,
    /// The file or folder being renamed and the name typed so far.
    rename: Option<(PathBuf, String)>,
    /// The file or folder the user is being asked to move to the Recycle Bin.
    delete: Option<PathBuf>,
    /// A question that waits for a yes: dropping a stash, ending a terminal.
    confirm: Option<Confirm>,
    /// A move to the Recycle Bin running on a worker thread.
    recycling: background::Latest<Recycled>,
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
        // The layout is tuned for 1 pixel per point; on a 125% display that means a 0.8 zoom.
        let native = creation_context.egui_ctx.pixels_per_point();
        creation_context
            .egui_ctx
            .set_zoom_factor((1.0 / native).clamp(0.8, 1.0));
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
            last_disk_check: std::time::Instant::now(),
            resume: Resume::Nothing,
            lobby: lobby::Lobby::default(),
            show_lobby: false,
            runtimes: Vec::new(),
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
            hunks_job: background::Latest::default(),
            checked_folders: std::collections::HashSet::new(),
            auto_checked: std::time::Instant::now(),
            recovery_warned: false,
            panel_sizes: (250, 364, 200),
            terminal_maximized: false,
            split_pair: None,
            trusted: true,
            trust_prompt: None,
            git_top: None,
            change_paths: Vec::new(),
            terminals_confirmed: false,
            graph_commits: Vec::new(),
            graph_rows: Vec::new(),
            file_history: Vec::new(),
            history_name: None,
            stash_count: 0,
            editor_scale: settings::clean_scale(settings::load().editor_scale),
            auto_save: settings::load().auto_save.unwrap_or(false),
            problems: None,
            checking: background::Latest::default(),
            session_saved: session::Session::default(),
            session_checked: std::time::Instant::now(),
            auto_saves: autosave::Tracker::default(),
            recovery: recovery::Store::default_location(),
            last_recovery: std::time::Instant::now(),
            recovery_written: std::collections::HashMap::new(),
            git_refresh: background::Latest::default(),
            operation: None,
            blame: Vec::new(),
            blame_enabled: true,
            git_job: None,
            pending_discard: None,
            gallery: theme_gallery::Gallery::default(),
            room: rooms::RoomView::default(),
            terminals: Vec::new(),
            active_terminal: 0,
            show_terminal: true,
            bottom_tab: BottomTab::Terminal,
            show_right: true,
            show_preview: true,
            lobby_close_warned: false,
            preview: preview::Preview::default(),
            ai: ai_panel::State::default(),
            right_tab: RightTab::Room,
            terminal_focus_request: false,
            terminal_has_focus: false,
            terminal_error: None,
            egui_ctx: creation_context.egui_ctx.clone(),
            palette: palette::Palette::default(),
            search: search_view::SearchView::default(),
            new_folder_parent: None,
            new_folder_name: String::new(),
            rename: None,
            delete: None,
            confirm: None,
            recycling: background::Latest::default(),
            find: find_bar::FindState::default(),
            pending_select: None,
            pending_edit: None,
            last_find_query: String::new(),
            reselect_find: false,
        };
        app.restore_recovered();
        // The last session comes back first, so a file given on the command line is added to it
        // instead of replacing it in the session file a moment later.
        app.restore_session();
        if let Some(path) = initial_path {
            app.open_path(path);
        }
        // Text brought back by crash recovery is for the user to look at: Auto Save leaves it until
        // they change it.
        let now = std::time::Instant::now();
        for tab in &app.tabs.tabs {
            if tab.doc.is_dirty() {
                let hash = recovery::text_hash(tab.doc.text());
                app.auto_saves.hold(tab.id, hash, now);
            }
        }
        app
    }

    /// Open the folder and files of the last session (nothing was asked for on the command line).
    fn restore_session(&mut self) {
        let session = session::load_from(&session::default_path()).clean();
        if let Some(folder) = &session.folder {
            self.open_path(PathBuf::from(folder));
        }
        for file in &session.files {
            self.open_path(PathBuf::from(file));
        }
        if let Some(index) = session.active
            && let Some(file) = session.files.get(index)
        {
            self.open_path(PathBuf::from(file));
        }
        self.focus_editor = !session.files.is_empty();
        // The panels as they were.
        if let Some(layout) = &session.layout {
            self.show_sidebar = layout.sidebar;
            self.show_right = layout.right;
            self.show_terminal = layout.terminal;
            self.show_preview = layout.preview;
            self.bottom_tab = match layout.bottom_tab {
                1 => BottomTab::Output,
                2 => BottomTab::Problems,
                _ => BottomTab::Terminal,
            };
            self.panel_sizes = (
                layout.sidebar_width,
                layout.right_width,
                layout.terminal_height,
            );
        }
        // Fresh shells in place of the ones that were open.
        for _ in 0..session.terminals {
            self.spawn_terminal(true);
        }
        if session.terminals > 0 {
            self.terminal_focus_request = false;
            self.active_terminal = 0;
        }
        self.session_saved = self.current_session();
    }

    /// The folder and the file tabs as they are now.
    fn current_session(&self) -> session::Session {
        let mut files = Vec::new();
        let mut active = None;
        for (index, tab) in self.tabs.tabs.iter().enumerate() {
            if tab.readonly {
                continue;
            }
            if let Some(path) = tab.doc.path() {
                if index == self.tabs.active {
                    active = Some(files.len());
                }
                files.push(path.display().to_string());
            }
        }
        session::Session {
            layout: Some(session::Layout {
                sidebar: self.show_sidebar,
                right: self.show_right,
                terminal: self.show_terminal,
                preview: self.show_preview,
                bottom_tab: match self.bottom_tab {
                    BottomTab::Terminal => 0,
                    BottomTab::Output => 1,
                    BottomTab::Problems => 2,
                },
                sidebar_width: self.panel_sizes.0,
                right_width: self.panel_sizes.1,
                terminal_height: self.panel_sizes.2,
            }),
            terminals: self.terminals.len().min(session::MAX_TERMINALS),
            folder: self
                .workspace
                .as_ref()
                .map(|workspace| workspace.root.display().to_string()),
            files,
            active,
        }
    }

    /// Write the session when it changed (checked every couple of seconds, and on exit).
    fn save_session(&mut self, force: bool) {
        const EVERY: std::time::Duration = std::time::Duration::from_secs(2);
        if !force && self.session_checked.elapsed() < EVERY {
            // Nothing else may wake an idle window, so ask to be woken for the next check.
            self.egui_ctx
                .request_repaint_after(EVERY - self.session_checked.elapsed());
            return;
        }
        self.session_checked = std::time::Instant::now();
        let current = self.current_session();
        if current != self.session_saved
            && session::save_to(&session::default_path(), &current).is_ok()
        {
            self.session_saved = current;
        }
    }

    /// Bring back the unsaved tabs a dead instance left in the recovery folder (it ended without a
    /// normal exit). They come back as unsaved tabs, and are written under this instance right
    /// away, so a second crash a moment later loses nothing either.
    fn restore_recovered(&mut self) {
        let entries = self.recovery.take_orphans();
        if entries.is_empty() {
            return;
        }
        let mut changed = 0;
        for entry in &entries {
            let outcome = self.tabs.open_recovered(
                entry.path.as_deref().map(std::path::Path::new),
                entry.text.clone(),
                entry.disk_hash,
            );
            if outcome == tabs::Recovered::DiskChanged {
                changed += 1;
            }
        }
        self.status = format!(
            "Recovered {} unsaved tab{} from the last session",
            entries.len(),
            if entries.len() == 1 { "" } else { "s" }
        );
        if changed > 0 {
            self.status.push_str(&format!(
                "; {changed} file{} changed on disk since, so its text is in an untitled tab",
                if changed == 1 { "" } else { "s" }
            ));
        }
        self.last_recovery = std::time::Instant::now() - std::time::Duration::from_secs(10);
        self.autosave_recovery();
    }

    /// With Auto Save on, save the file tabs whose text has rested. Tabs without a file are never
    /// saved here (that would need a dialog), and a file that changed on disk is left alone: the
    /// user decides with the usual conflict prompt when saving by hand.
    fn auto_save_tabs(&mut self) {
        if !self.auto_save || self.pending.is_some() {
            return;
        }
        // Looking at every dirty text each frame is wasted work: twice a second is plenty.
        let wait = std::time::Duration::from_millis(500);
        if self.auto_checked.elapsed() < wait {
            self.egui_ctx.request_repaint_after(wait);
            return;
        }
        self.auto_checked = std::time::Instant::now();
        let now = std::time::Instant::now();
        let mut live = Vec::new();
        let mut saved = None;
        let mut failed = None;
        for tab in &mut self.tabs.tabs {
            if tab.readonly || !tab.doc.is_dirty() {
                continue;
            }
            // A file that is gone (deleted or moved away) is not created again behind the user's back.
            if !tab.doc.path().is_some_and(|path| path.exists()) {
                continue;
            }
            live.push(tab.id);
            let hash = recovery::text_hash(tab.doc.text());
            if !self.auto_saves.due(tab.id, hash, now) {
                continue;
            }
            match tab.doc.save() {
                Ok(()) => saved = Some(tab.title()),
                Err(error) => {
                    self.auto_saves.failed(tab.id);
                    failed = Some(format!("Auto Save skipped {}: {error}", tab.title()));
                }
            }
        }
        self.auto_saves.keep_only(&live);
        if !live.is_empty() {
            // Nothing else may wake the window while the user is not typing.
            self.egui_ctx.request_repaint_after(autosave::DELAY / 2);
        }
        if let Some(message) = failed {
            self.status = message;
        } else if let Some(title) = saved {
            self.status = format!("Auto-saved {title}");
            self.refresh_git();
        }
    }

    /// Every few seconds write the text of tabs with unsaved changes to the recovery folder, and
    /// forget the entries of tabs that were saved or closed. Only changed text is rewritten.
    fn autosave_recovery(&mut self) {
        // Once the app is on its way out (the user chose to quit) nothing more is kept.
        if self.allow_close || self.last_recovery.elapsed() < std::time::Duration::from_secs(3) {
            return;
        }
        self.last_recovery = std::time::Instant::now();
        let mut live = Vec::new();
        for tab in &self.tabs.tabs {
            if tab.readonly || !tab.doc.is_dirty() {
                continue;
            }
            live.push(tab.id);
            let path = tab.doc.path();
            // The path is part of the fingerprint, so a renamed file is written again.
            let hash = recovery::text_hash(tab.doc.text())
                ^ recovery::text_hash(&path.map(|p| p.display().to_string()).unwrap_or_default())
                    .rotate_left(1);
            if self.recovery_written.get(&tab.id) == Some(&hash) {
                continue;
            }
            let entry = recovery::Entry {
                path: path.map(|path| path.display().to_string()),
                text: tab.doc.text().to_string(),
                disk_hash: path.map(|_| recovery::text_hash(tab.doc.saved_text())),
            };
            match self.recovery.save(tab.id, &entry) {
                Ok(()) => {
                    self.recovery_written.insert(tab.id, hash);
                }
                Err(error) if !self.recovery_warned => {
                    // The safety net is not working: say so once instead of retrying in silence.
                    self.recovery_warned = true;
                    self.status = format!("Crash recovery cannot write its files: {error}");
                }
                Err(_) => {}
            }
        }
        let gone: Vec<u64> = self
            .recovery_written
            .keys()
            .copied()
            .filter(|id| !live.contains(id))
            .collect();
        for id in gone {
            self.recovery.remove(id);
            self.recovery_written.remove(&id);
        }
    }

    // ---- File operations ---------------------------------------------------------------

    fn open_path(&mut self, path: PathBuf) {
        if path.is_dir() {
            if let Err(error) = self.lobby.finish() {
                self.status = format!("Folder not opened: {error}");
                return;
            }
            let folder = normalize(&path);
            // The recent list is a convenience: failing to save it is not worth a message.
            let _ = settings::update(|settings| {
                settings::remember_folder(
                    &mut settings.recent_folders,
                    &folder.display().to_string(),
                );
            });
            let previous = self.git_root();
            self.workspace = Some(Workspace::new(normalize(&path)));
            self.show_sidebar = true;
            self.clear_git_state();
            // Tabs of the folder that was left would otherwise sit among the new project's files, and
            // a shell would go on working in the old one.
            let mut moved = String::new();
            if previous.as_ref().is_some_and(|old| *old != folder) {
                let closed = self.tabs.close_clean_outside(&folder);
                if closed > 0 {
                    moved.push_str(&format!(
                        "; closed {closed} saved tab(s) of the other folder"
                    ));
                }
                if self.terminals.iter().any(|t| !t.has_exited()) {
                    self.spawn_terminal(true);
                    moved.push_str(
                        "; a new terminal is open here, the old ones still run in the other folder",
                    );
                }
            }
            // Git and the project's tools only run where the user said so: a folder can carry
            // settings that make them start programs.
            let known = settings::load().trusted_folders;
            if settings::is_trusted(&known, &folder.display().to_string()) {
                self.set_trust(true);
            } else {
                self.set_trust(false);
                self.trust_prompt = Some(folder.clone());
            }
            let root = self.repo_root();
            self.room.load_for(root.as_deref());
            let kept = self
                .lobby
                .load_for(root.as_deref().map(room_state::lobby_path_for));
            self.runtimes = root.as_deref().map(detect_runtimes).unwrap_or_default();
            self.search.reset();
            self.preview.forget_project();
            self.status = match kept {
                Ok(()) => format!("Opened folder {}{moved}", path.display()),
                Err(error) => format!("Opened folder {}. {error}{moved}", path.display()),
            };
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

    fn new_file_in(&mut self, directory: &std::path::Path) {
        let Some(path) = rfd::FileDialog::new()
            .set_directory(directory)
            .set_file_name("untitled.txt")
            .save_file()
        else {
            return;
        };
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => {
                if let Some(workspace) = &mut self.workspace {
                    workspace.refresh();
                }
                self.open_path(path);
                self.refresh_git();
            }
            Err(error) => {
                self.status = format!("Cannot create {}: {error}", path.display());
            }
        }
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
            Err(document::DocumentError::ChangedOnDisk) => {
                self.pending = Some(Pending::DiskConflict(
                    index,
                    std::mem::take(&mut self.resume),
                ));
                false
            }
            Err(error) => {
                self.status = format!("Cannot save {title}: {error}");
                false
            }
        }
    }

    /// Reload tabs whose file changed on disk when they have no unsaved edits; dirty tabs are
    /// left alone and surface a conflict choice when saved.
    fn sync_external_changes(&mut self) {
        let mut reloaded = None;
        let mut warned = None;
        for tab in &mut self.tabs.tabs {
            if tab.readonly || tab.doc.path().is_none() {
                continue;
            }
            let dirty = tab.doc.is_dirty();
            if !dirty && tab.doc.changed_on_disk() && tab.doc.reload().is_ok() {
                reloaded = Some(tab.title());
            }
            // What the tab cannot show on its own: the file is gone, or it changed under unsaved
            // edits (saving would then replace the other change, or bring a deleted file back).
            let gone = tab.doc.path().is_some_and(|path| !path.exists());
            let now = if gone {
                Some("deleted on disk")
            } else if dirty && tab.doc.changed_on_disk() {
                Some("changed on disk")
            } else {
                None
            };
            if now.is_some() && now != tab.external {
                warned = Some((tab.title(), now.unwrap_or_default()));
            }
            tab.external = now;
        }
        if let Some(title) = reloaded {
            self.status = format!("Reloaded {title}: it changed on disk");
        }
        if let Some((title, what)) = warned {
            self.status = format!("{title} was {what}; look at the tab before saving");
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
            Command::SelectNextOccurrence
            | Command::GoToMatchingBracket
            | Command::SelectToMatchingBracket
            | Command::TransformUppercase
            | Command::TransformLowercase
            | Command::SortLinesAscending
            | Command::SortLinesDescending
            | Command::RemoveDuplicateLines
            | Command::TrimTrailingWhitespace
            | Command::JoinLines => {
                use transforms::Kind;
                self.pending_edit = Some(match command {
                    Command::SelectNextOccurrence => Kind::SelectNextOccurrence,
                    Command::GoToMatchingBracket => Kind::GoToMatchingBracket,
                    Command::SelectToMatchingBracket => Kind::SelectToMatchingBracket,
                    Command::TransformUppercase => Kind::Uppercase,
                    Command::TransformLowercase => Kind::Lowercase,
                    Command::SortLinesAscending => Kind::SortAscending,
                    Command::SortLinesDescending => Kind::SortDescending,
                    Command::RemoveDuplicateLines => Kind::RemoveDuplicateLines,
                    Command::TrimTrailingWhitespace => Kind::TrimTrailingWhitespace,
                    _ => Kind::JoinLines,
                });
                self.focus_editor = true;
            }
            Command::ZoomIn | Command::ZoomOut | Command::ZoomReset => {
                let scale = match command {
                    Command::ZoomIn => settings::step_scale(self.editor_scale, 1),
                    Command::ZoomOut => settings::step_scale(self.editor_scale, -1),
                    _ => 1.0,
                };
                self.editor_scale = scale;
                // A convenience: failing to remember the size is not worth interrupting for.
                let _ = settings::update(|settings| {
                    settings.editor_scale = (scale != 1.0).then_some(scale);
                });
                self.status = format!("Editor font size {}%", (scale * 100.0).round());
            }
            Command::GoToSymbol => {
                let outline =
                    self.tabs
                        .tabs
                        .get(self.tabs.active)
                        .map(|tab| {
                            let extension =
                                tab.syntax
                                    .clone()
                                    .or_else(|| {
                                        tab.doc.path().and_then(|path| path.extension()).map(
                                            |extension| extension.to_string_lossy().into_owned(),
                                        )
                                    })
                                    .unwrap_or_default();
                            symbols::outline(tab.doc.text(), &extension)
                        })
                        .unwrap_or_default();
                self.palette.open_symbols(outline);
            }
            Command::ReopenClosedTab => match self.tabs.reopen_closed() {
                Ok(Some(path)) => {
                    self.status = format!("Reopened {}", path.display());
                    self.focus_editor = true;
                }
                Ok(None) => self.status = "No closed tab to reopen".into(),
                Err(error) => self.status = format!("Cannot reopen: {error}"),
            },
            Command::CloseSavedTabs if self.pending.is_some() => {
                // A dialog holds a tab's position: closing tabs now would point it at another one.
                self.status = "Finish the open dialog first".into();
            }
            Command::CloseSavedTabs => {
                let closed = self.tabs.close_saved();
                self.status = match closed {
                    0 => "Every open tab has unsaved changes".into(),
                    1 => "Closed 1 saved tab".into(),
                    count => format!("Closed {count} saved tabs"),
                };
            }
            Command::NextTab => {
                self.tabs.next();
                self.focus_editor = true;
            }
            Command::ToggleSidebar => self.show_sidebar = !self.show_sidebar,
            Command::ShowExplorer => self.switch_view(SidebarView::Explorer),
            Command::ShowSourceControl => self.switch_view(SidebarView::SourceControl),
            Command::ShowRoom => self.switch_view(SidebarView::Room),
            Command::ShowLobby => self.show_lobby = !self.show_lobby,
            Command::TogglePreview => self.show_preview = !self.show_preview,
            Command::ShowSearch => {
                self.view = SidebarView::Search;
                self.show_sidebar = true;
                self.search.focus_input();
            }
            Command::ColorTheme => self.gallery.open(),
            Command::GitRefresh => self.refresh_git(),
            Command::ToggleAutoSave => {
                self.auto_save = !self.auto_save;
                let on = self.auto_save;
                let _ = settings::update(|settings| settings.auto_save = on.then_some(true));
                self.status = if on {
                    "Auto Save on: files are saved when you stop typing".into()
                } else {
                    "Auto Save off".into()
                };
            }
            Command::CopyPath | Command::CopyRelativePath | Command::RevealInExplorer => {
                let path = self
                    .tabs
                    .active_tab()
                    .and_then(|tab| tab.doc.path().map(std::path::Path::to_path_buf));
                match (path, command) {
                    (None, _) => self.status = "The active tab is not a file on disk".into(),
                    (Some(path), Command::CopyPath) => {
                        self.egui_ctx.copy_text(path.display().to_string());
                        self.status = "Copied the path".into();
                    }
                    (Some(path), Command::CopyRelativePath) => {
                        let text = match self.repo_root() {
                            Some(root) => file_ops::relative_display(&root, &path),
                            None => path.display().to_string(),
                        };
                        self.egui_ctx.copy_text(text);
                        self.status = "Copied the relative path".into();
                    }
                    (Some(path), _) => {
                        if let Err(error) = file_ops::reveal(&path) {
                            self.status = error;
                        }
                    }
                }
            }
            Command::ToggleBlame => {
                self.blame_enabled = !self.blame_enabled;
                self.status = if self.blame_enabled {
                    "Line blame on".into()
                } else {
                    "Line blame off".into()
                };
            }
            Command::SwitchBranch => {
                if let Some(root) = self.git_root() {
                    match git::branches(&root) {
                        Ok(list) => self
                            .palette
                            .open_branches(list, palette::BranchPurpose::Checkout),
                        Err(error) => self.status = format!("Git: {error}"),
                    }
                }
            }
            Command::MergeBranch | Command::DeleteBranch => {
                let purpose = if command == Command::MergeBranch {
                    palette::BranchPurpose::Merge
                } else {
                    palette::BranchPurpose::Delete
                };
                if let Some(root) = self.git_root() {
                    match git::branches(&root) {
                        Ok(list) => self.palette.open_branches(list, purpose),
                        Err(error) => self.status = format!("Git: {error}"),
                    }
                }
            }
            Command::CreateTag | Command::DeleteTag => {
                let purpose = if command == Command::CreateTag {
                    palette::BranchPurpose::TagCreate
                } else {
                    palette::BranchPurpose::TagDelete
                };
                if let Some(root) = self.git_root() {
                    match git::tags(&root) {
                        Ok(list) => {
                            let tags = list
                                .into_iter()
                                .map(|name| git::Branch {
                                    name,
                                    current: false,
                                    remote: false,
                                    upstream: None,
                                })
                                .collect();
                            self.palette.open_branches(tags, purpose);
                        }
                        Err(error) => self.status = format!("Git: {error}"),
                    }
                }
            }
            Command::AbortOperation => self.git_action(source_control::Action::AbortOperation),
            Command::GitSync => self.git_action(source_control::Action::Sync),
            Command::GitPull => self.git_action(source_control::Action::Pull),
            Command::GitPush => self.git_action(source_control::Action::Push),
            Command::GitFetch => self.git_action(source_control::Action::Fetch),
            Command::GitStash => self.git_action(source_control::Action::Stash),
            Command::GitStashApply => self.git_action(source_control::Action::StashApply),
            Command::Find => self.find.show_bar(false),
            Command::Replace => self.find.show_bar(true),
            Command::QuickOpen => {
                let root = self.repo_root();
                self.palette.open_files(root.as_deref());
            }
            Command::GoToLine if palette_mode => self.palette.open_commands(),
            Command::GoToLine => self.palette.open_goto(),
            Command::OpenAgent => {
                self.show_right = true;
                self.right_tab = RightTab::Ai;
            }
            Command::TrustFolder => {
                if let Some(root) = self.repo_root() {
                    if self.trusted {
                        self.status = "This folder is already trusted".into();
                    } else {
                        self.trust_prompt = Some(root);
                    }
                } else {
                    self.status = "Open a folder first".into();
                }
            }
            Command::CheckProject => {
                self.show_terminal = true;
                self.bottom_tab = BottomTab::Problems;
                self.start_check();
            }
            Command::CopyTerminalOutput => match self.terminals.get(self.active_terminal) {
                Some(terminal) => {
                    let text = terminal.all_text();
                    let lines = text.lines().count();
                    self.egui_ctx.copy_text(text);
                    self.status = format!("Copied the whole terminal output ({lines} lines)");
                }
                None => self.status = "There is no terminal".into(),
            },
            Command::NewTerminal => {
                self.show_terminal = true;
                self.bottom_tab = BottomTab::Terminal;
                self.spawn_terminal(true);
            }
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
        let running = self
            .terminals
            .get(self.active_terminal)
            .is_some_and(|t| !t.has_exited());
        if running {
            return;
        }
        self.spawn_terminal(false);
    }

    /// Start a shell in the project folder: as one more terminal, or in place of the active one.
    fn spawn_terminal(&mut self, add: bool) {
        let root = self.repo_root();
        match terminal::Terminal::spawn(&self.egui_ctx, root.as_deref()) {
            Ok(terminal) => {
                self.terminal_error = None;
                if add || self.terminals.is_empty() {
                    self.terminals.push(terminal);
                    self.active_terminal = self.terminals.len() - 1;
                } else {
                    let index = self.active_terminal.min(self.terminals.len() - 1);
                    self.terminals[index] = terminal;
                    self.active_terminal = index;
                }
            }
            Err(error) => self.terminal_error = Some(error),
        }
        self.terminal_focus_request = true;
    }

    /// Run `cargo check` for the open folder on a worker thread.
    fn start_check(&mut self) {
        let Some(root) = self.repo_root() else {
            self.status = "Open a folder to check it".into();
            return;
        };
        if !self.trusted {
            self.status = "Trust this folder first (Folder: Trust This Folder): cargo check runs its build scripts".into();
            return;
        }
        if !self.checked_folders.contains(&root) {
            self.confirm = Some(Confirm::RunCheck);
            return;
        }
        let context = self.egui_ctx.clone();
        if self
            .checking
            .request(&context, move || diagnostics::check(&root))
        {
            self.status = "Running cargo check…".into();
        } else {
            self.status = "A check is already running".into();
        }
    }

    /// Take a finished check.
    fn poll_check(&mut self) {
        if let Some(report) = self.checking.poll() {
            let errors = report.problems.iter().filter(|p| p.error).count();
            let warnings = report.problems.len() - errors;
            self.status = match &report.failure {
                Some(failure) => format!("cargo check: {failure}"),
                None => format!("cargo check: {errors} error(s), {warnings} warning(s)"),
            };
            self.problems = Some(report);
        } else if self.checking.take_failed() {
            self.status = "cargo check stopped unexpectedly".into();
        }
    }

    /// Close the active terminal (its shell is ended); the one before it comes forward.
    fn close_terminal(&mut self) {
        self.split_pair = None;
        if self.active_terminal < self.terminals.len() {
            self.terminals.remove(self.active_terminal);
        }
        self.active_terminal = self
            .active_terminal
            .saturating_sub(1)
            .min(self.terminals.len().saturating_sub(1));
        self.terminal_focus_request = true;
        if self.terminals.is_empty() {
            self.show_terminal = false;
            self.terminal_has_focus = false;
            self.focus_editor = true;
        }
    }

    fn terminal_panel(&mut self, ui: &mut egui::Ui) {
        let exited = self
            .terminals
            .get(self.active_terminal)
            .is_some_and(|t| t.has_exited());
        let mut restart = false;
        let mut close = false;
        let mut add_terminal = false;
        let mut close_terminal = false;
        let mut switch_to = None;
        let mut toggle_split = false;
        let mut toggle_maximize = false;
        let mut copy_all = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(12.0);
            for (tab, label) in [
                (BottomTab::Terminal, "Terminal"),
                (BottomTab::Output, "Output"),
                (BottomTab::Problems, "Problems"),
            ] {
                if underline_tab(ui, label, self.bottom_tab == tab, true).clicked() {
                    self.bottom_tab = tab;
                }
            }
            for label in ["Debug Console", "Ports"] {
                underline_tab(ui, label, false, false).on_hover_text("Adapter not connected");
            }
            if self.bottom_tab == BottomTab::Terminal {
                // One small number per terminal, a plus for another and a cross to end the shown
                // one, all drawn with the app's own icons (a font glyph may be missing).
                ui.add_space(8.0);
                if self.terminals.len() > 1 {
                    for index in 0..self.terminals.len() {
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(24.0, 26.0), egui::Sense::click());
                        let active = index == self.active_terminal;
                        if active {
                            ui.painter().rect_filled(rect, 6.0, theme::overlay(22));
                        } else if response.hovered() {
                            ui.painter().rect_filled(rect, 6.0, theme::overlay(12));
                        }
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            index + 1,
                            egui::FontId::proportional(13.0),
                            if active {
                                theme::text()
                            } else {
                                theme::text_dim()
                            },
                        );
                        if response
                            .on_hover_text(format!("Terminal {}", index + 1))
                            .clicked()
                        {
                            switch_to = Some(index);
                        }
                    }
                }
            }
            if ui.available_width() > 760.0
                && let Some(terminal) = self.terminals.get(self.active_terminal)
            {
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
                // Right to left: close, maximize, a divider, then the terminal's own actions.
                if icons::icon_button(ui, "Hide terminal (Ctrl+`)", icons::close_cross).clicked() {
                    close = true;
                }
                let maximize_tip = if self.terminal_maximized {
                    "Restore panel size"
                } else {
                    "Maximize panel"
                };
                if icons::icon_button(ui, maximize_tip, icons::maximize).clicked() {
                    toggle_maximize = true;
                }
                if self.bottom_tab == BottomTab::Terminal {
                    let (divider, _) =
                        ui.allocate_exact_size(egui::vec2(9.0, 16.0), egui::Sense::hover());
                    ui.painter().line_segment(
                        [
                            egui::pos2(divider.center().x, divider.top()),
                            egui::pos2(divider.center().x, divider.bottom()),
                        ],
                        egui::Stroke::new(1.0, theme::border()),
                    );
                    let more = icons::icon_button(ui, "More terminal actions", icons::more);
                    egui::Popup::menu(&more).show(|ui| {
                        if ui.button("Copy all output").clicked() {
                            copy_all = true;
                            ui.close();
                        }
                        if ui.button("Restart this terminal").clicked() {
                            restart = true;
                            ui.close();
                        }
                    });
                    if icons::icon_button(ui, "Kill terminal", icons::trash).clicked() {
                        close_terminal = true;
                    }
                    let split_tip = if self.split_pair.is_some() {
                        "Unsplit terminals"
                    } else {
                        "Split terminal"
                    };
                    if icons::icon_button(ui, split_tip, icons::split).clicked() {
                        toggle_split = true;
                    }
                    let menu = icons::icon_button(ui, "Terminal types", icons::chevron_small);
                    egui::Popup::menu(&menu).show(|ui| {
                        if ui
                            .button(format!(
                                "New {}",
                                self.terminals
                                    .first()
                                    .map_or("terminal", |t| t.shell.as_str())
                            ))
                            .clicked()
                        {
                            add_terminal = true;
                            ui.close();
                        }
                    });
                    if icons::icon_button(ui, "New terminal (Ctrl+Shift+`)", icons::plus).clicked()
                    {
                        add_terminal = true;
                    }
                }
                if (exited || self.terminals.is_empty()) && ui.small_button("Restart").clicked() {
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
        if toggle_maximize {
            self.terminal_maximized = !self.terminal_maximized;
        }
        if copy_all && let Some(terminal) = self.terminals.get(self.active_terminal) {
            let text = terminal.all_text();
            self.status = format!(
                "Copied the whole terminal output ({} lines)",
                text.lines().count()
            );
            ui.ctx().copy_text(text);
        }
        if toggle_split {
            if self.split_pair.is_some() {
                self.split_pair = None;
            } else {
                // Two terminals side by side: the shown one and another (made when there is none).
                if self.terminals.len() < 2 {
                    self.spawn_terminal(true);
                }
                if self.terminals.len() >= 2 {
                    let shown = self.active_terminal.min(self.terminals.len() - 1);
                    let other = if shown + 1 < self.terminals.len() {
                        shown + 1
                    } else {
                        shown - 1
                    };
                    self.split_pair = Some((shown.min(other), shown.max(other)));
                    self.active_terminal = shown;
                }
            }
        }
        if restart {
            self.spawn_terminal(false);
        }
        if add_terminal {
            self.spawn_terminal(true);
        }
        if close_terminal {
            let running = self
                .terminals
                .get(self.active_terminal)
                .is_some_and(|t| !t.has_exited());
            if running {
                self.confirm = Some(Confirm::EndTerminal);
            } else {
                self.close_terminal();
            }
        }
        if let Some(index) = switch_to {
            if self
                .split_pair
                .is_some_and(|(a, b)| index != a && index != b)
            {
                self.split_pair = None;
            }
            self.active_terminal = index;
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
        if self.bottom_tab == BottomTab::Problems {
            self.terminal_has_focus = false;
            self.problems_view(ui);
            return;
        }
        if self.bottom_tab != BottomTab::Terminal {
            self.terminal_has_focus = false;
            self.terminal_output_view(ui);
            return;
        }
        let request = std::mem::take(&mut self.terminal_focus_request);
        let mut link = None;
        let active = self.active_terminal;
        // A split that no longer fits the terminals that exist is dropped.
        if self
            .split_pair
            .is_some_and(|(a, b)| a >= b || b >= self.terminals.len())
        {
            self.split_pair = None;
        }
        if let Some((a, b)) = self.split_pair {
            let (first, second) = self.terminals.split_at_mut(b);
            let (left, right) = (&mut first[a], &mut second[0]);
            let (mut left_focus, mut right_focus) = (false, false);
            ui.columns(2, |columns| {
                left_focus = left.show(&mut columns[0], request && active == a);
                right_focus = right.show(&mut columns[1], request && active == b);
            });
            if let Some(found) = left.take_link().or_else(|| right.take_link()) {
                link = Some(found);
            }
            self.terminal_has_focus = left_focus || right_focus;
            // Actions that follow the shown terminal follow the one that has the keyboard.
            if right_focus {
                self.active_terminal = b;
            } else if left_focus {
                self.active_terminal = a;
            }
            if let Some(link) = link {
                self.open_terminal_link(&link);
            }
            return;
        }
        match self.terminals.get_mut(active) {
            Some(terminal) => {
                self.terminal_has_focus = terminal.show(ui, request);
                link = terminal.take_link();
            }
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
        if let Some(link) = link {
            self.open_terminal_link(&link);
        }
    }

    /// Open the file a Ctrl+clicked reference in the terminal output points at, at its line and
    /// column. References are resolved inside the open folder (or as absolute paths).
    fn open_terminal_link(&mut self, link: &path_links::PathRef) {
        let root = self
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default();
        let Some(path) = path_links::resolve(link, &root) else {
            self.status = format!("No file {} to open", link.path);
            return;
        };
        self.open_path(path.clone());
        // `open_path` reports a failure in the status bar and leaves the previous tab active: only
        // place the caret when the file really is the active tab.
        let opened = self.tabs.active_path() == Some(tabs::normalize(&path).as_path());
        if !opened {
            return;
        }
        if let (Some(line), Some(tab)) = (link.line, self.tabs.tabs.get(self.tabs.active)) {
            let at = path_links::locate(tab.doc.text(), line, link.column.unwrap_or(1));
            self.pending_select = Some((at, at));
            self.focus_editor = true;
        }
    }

    /// The Problems tab: a button that runs `cargo check` and the compiler's errors and warnings,
    /// each opening its file at the right line.
    fn problems_view(&mut self, ui: &mut egui::Ui) {
        let mut run = false;
        let mut open: Option<diagnostics::Problem> = None;
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            let busy = self.checking.busy();
            if ui
                .add_enabled(!busy, egui::Button::new("Run cargo check"))
                .clicked()
            {
                run = true;
            }
            if busy {
                ui.spinner();
                ui.label(egui::RichText::new("Checking…").color(theme::text_dim()));
            } else if let Some(report) = &self.problems {
                let errors = report.problems.iter().filter(|p| p.error).count();
                let text = match &report.failure {
                    Some(failure) => failure.clone(),
                    None => format!(
                        "{errors} error(s), {} warning(s){}",
                        report.problems.len() - errors,
                        if report.truncated {
                            " (first 500 shown)"
                        } else {
                            ""
                        }
                    ),
                };
                ui.label(egui::RichText::new(text).color(theme::text_dim()));
            }
        });
        let Some(report) = &self.problems else {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(
                        "Run cargo check to list the compiler's errors and warnings. \
                         It is not a language server: the list is as old as the last run.",
                    )
                    .color(theme::text_dim()),
                );
            });
            if run {
                self.start_check();
            }
            return;
        };
        let root = self
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone());
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for problem in &report.problems {
                    let shown = root
                        .as_deref()
                        .map(|root| file_ops::relative_display(root, &problem.file))
                        .unwrap_or_else(|| problem.file.display().to_string());
                    let color = if problem.error {
                        egui::Color32::from_rgb(235, 100, 100)
                    } else {
                        egui::Color32::from_rgb(220, 180, 80)
                    };
                    let line = format!(
                        "{}  {shown}:{}:{}  {}",
                        if problem.error { "error" } else { "warning" },
                        problem.line,
                        problem.column,
                        problem.message.lines().next().unwrap_or("")
                    );
                    let response = ui.add(
                        egui::Label::new(egui::RichText::new(line).monospace().color(color))
                            .sense(egui::Sense::click())
                            .truncate(),
                    );
                    if response.clicked() {
                        open = Some(problem.clone());
                    }
                }
            });
        if let Some(problem) = open {
            self.open_path(problem.file.clone());
            if self.tabs.active_path() == Some(tabs::normalize(&problem.file).as_path())
                && let Some(tab) = self.tabs.tabs.get(self.tabs.active)
            {
                let at = path_links::locate(tab.doc.text(), problem.line, problem.column);
                self.pending_select = Some((at, at));
                self.focus_editor = true;
            }
        }
        if run {
            self.start_check();
        }
    }

    fn terminal_output_view(&mut self, ui: &mut egui::Ui) {
        let Some(terminal) = self.terminals.get(self.active_terminal) else {
            ui.label("Start the terminal to see local output.");
            return;
        };
        let text = terminal.screen_text();
        match self.bottom_tab {
            BottomTab::Output => {
                ui.horizontal(|ui| {
                    ui.label("Current terminal screen");
                    if ui.button("Copy output").clicked() {
                        ui.ctx().copy_text(terminal.all_text());
                    }
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.monospace(if text.trim().is_empty() {
                        "No output yet."
                    } else {
                        &text
                    });
                });
            }
            BottomTab::Problems | BottomTab::Terminal => {}
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
            if shortcut(ui, command_shift, Key::Backtick) {
                self.run_command(Command::NewTerminal, false);
            } else if shortcut(ui, command, Key::Backtick) {
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
        } else if shortcut(ui, command_shift, Key::Y) {
            self.run_command(Command::GoToSymbol, false);
        } else if shortcut(ui, command_shift, Key::T) {
            self.run_command(Command::ReopenClosedTab, false);
        } else if shortcut(ui, command, Key::P) {
            self.run_command(Command::QuickOpen, false);
        } else if shortcut(ui, command, Key::F) {
            self.run_command(Command::Find, false);
        } else if shortcut(ui, command, Key::H) {
            self.run_command(Command::Replace, false);
        } else if shortcut(ui, command, Key::G) {
            self.run_command(Command::GoToLine, false);
        } else if shortcut(ui, command_shift, Key::Backtick) {
            self.run_command(Command::NewTerminal, false);
        } else if shortcut(ui, command, Key::Backtick) {
            self.run_command(Command::ToggleTerminal, false);
        } else if shortcut(ui, command, Key::Equals) || shortcut(ui, command, Key::Plus) {
            self.run_command(Command::ZoomIn, false);
        } else if shortcut(ui, command, Key::Minus) {
            self.run_command(Command::ZoomOut, false);
        } else if shortcut(ui, command, Key::Num0) {
            self.run_command(Command::ZoomReset, false);
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

        ui.horizontal_centered(|ui| {
            ui.add_space(14.0);
            let (logo, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
            mark::draw(ui.painter(), logo.center(), 24.0);
            ui.add_space(4.0);
            // The design has no menu strip: File, Edit and View live under the brand name.
            ui.menu_button(
                egui::RichText::new("Forge Room")
                    .size(16.0)
                    .family(fonts::semibold())
                    .color(theme::text()),
                |ui| self.menu_bar(ui),
            );
            ui.add_space(10.0);
            self.title_chips(ui, bar.width());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (kind, tip) in [(0, "Close"), (1, "Maximize"), (2, "Minimize")] {
                    let (rect, response) = ui
                        .allocate_exact_size(egui::vec2(40.0, TITLE_HEIGHT), egui::Sense::click());
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
                ui.add_space(10.0);
                for (icon, tip, command) in [
                    ("panel-right", "Toggle side panel", Command::ShowRoom),
                    ("panel-bottom", "Toggle terminal", Command::ToggleTerminal),
                    ("panel-left", "Toggle sidebar", Command::ToggleSidebar),
                ] {
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                    if response.hovered() {
                        ui.painter().rect_filled(rect, 8.0, theme::overlay(14));
                    }
                    icons::draw(ui.painter(), icon, rect.center(), 20.0, theme::text_dim());
                    if response.on_hover_text(tip).clicked() {
                        match command {
                            Command::ShowRoom => self.show_dock(RightTab::Ai),
                            other => self.run_command(other, false),
                        }
                    }
                }
                ui.add_space(8.0);
                if bar.width() > 900.0 {
                    let (pill, response) =
                        ui.allocate_exact_size(egui::vec2(300.0, 28.0), egui::Sense::click());
                    let painter = ui.painter();
                    painter.rect_filled(
                        pill,
                        9.0,
                        if response.hovered() {
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
                    icons::draw(
                        painter,
                        "search",
                        egui::pos2(pill.left() + 18.0, pill.center().y),
                        16.0,
                        theme::text_dim(),
                    );
                    painter.text(
                        egui::pos2(pill.left() + 34.0, pill.center().y),
                        egui::Align2::LEFT_CENTER,
                        "Search, run, or ask Forge...",
                        egui::FontId::proportional(13.0),
                        theme::text_dim(),
                    );
                    painter.text(
                        egui::pos2(pill.right() - 12.0, pill.center().y),
                        egui::Align2::RIGHT_CENTER,
                        "Ctrl+Shift+P",
                        egui::FontId::proportional(11.5),
                        theme::text_dim(),
                    );
                    if response.on_hover_text("Command Palette").clicked() {
                        self.run_command(Command::GoToLine, true);
                    }
                    ui.add_space(10.0);
                }
                if bar.width() > 1440.0 && self.lobby.timer_active() {
                    let remaining = self.lobby.remaining();
                    ui.label(
                        egui::RichText::new(format!(
                            "{:02}:{:02}:{:02}",
                            remaining / 3600,
                            remaining / 60 % 60,
                            remaining % 60
                        ))
                        .size(17.0)
                        .family(fonts::semibold())
                        .color(theme::accent()),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(self.lobby.kind_label())
                            .size(14.0)
                            .color(theme::text()),
                    );
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_secs(1));
                }
            });
        });
    }

    /// Room, branch, lobby, privacy and people shown after the brand name.
    fn title_chips(&mut self, ui: &mut egui::Ui, width: f32) {
        let Some((room_name, _, _)) = self.room.summary() else {
            return;
        };
        if width < 900.0 {
            return;
        }
        if title_chip(ui, Some("room"), &room_name, None, true)
            .on_hover_text("Open the Room Lobby")
            .clicked()
        {
            self.show_lobby = !self.show_lobby;
        }
        if let Some(branch) = self.branch.clone()
            && title_chip(ui, Some("source-control"), &branch, None, true)
                .on_hover_text("Switch branch")
                .clicked()
        {
            self.run_command(Command::SwitchBranch, false);
        }
        if width < 1260.0 {
            return;
        }
        let dot = Some(egui::Color32::from_rgb(240, 113, 120));
        if title_chip(ui, None, &format!("Lobby: {room_name}"), dot, false).clicked() {
            self.show_lobby = true;
        }
        title_chip(ui, Some("lock"), "Private", None, false);
        if width >= 1360.0
            && let Some(state) = self.room.state()
        {
            let people: Vec<&str> = state
                .participants
                .iter()
                .map(|person| person.name.as_str())
                .collect();
            avatar_stack(ui, &people);
        }
    }

    /// File > Open Recent: the folders opened before that still exist, newest first.
    fn recent_menu(&mut self, ui: &mut egui::Ui) {
        let recent: Vec<String> = settings::load()
            .recent_folders
            .into_iter()
            .filter(|folder| std::path::Path::new(folder).is_dir())
            .collect();
        let mut chosen = None;
        let mut clear = false;
        ui.add_enabled_ui(!recent.is_empty(), |ui| {
            ui.menu_button("Open Recent", |ui| {
                for folder in &recent {
                    if ui.button(folder).clicked() {
                        chosen = Some(PathBuf::from(folder));
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Clear Recent").clicked() {
                    clear = true;
                    ui.close();
                }
            });
        });
        if clear {
            let _ = settings::update(|settings| settings.recent_folders.clear());
        }
        if let Some(path) = chosen {
            self.open_path(path);
            ui.close();
        }
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
                self.recent_menu(ui);
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
                let mut auto_save = self.auto_save;
                if ui.checkbox(&mut auto_save, "Auto Save").clicked() {
                    self.run_command(Command::ToggleAutoSave, false);
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
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
        ui.add_space(10.0);
        let badge = self.change_count();
        let rooms = self.room.open_task_count();
        // (item, icon, label, tooltip); `None` is a divider.
        type Entry = Option<(Rail, &'static str, &'static str, &'static str)>;
        let top: [Entry; 13] = [
            Some((
                Rail::View(SidebarView::Explorer),
                "code",
                "Code",
                "Code (Ctrl+Shift+E)",
            )),
            Some((
                Rail::View(SidebarView::Search),
                "search",
                "Search",
                "Search (Ctrl+Shift+F)",
            )),
            Some((Rail::Room, "room", "Rooms", "Room Lobby")),
            Some((Rail::Studio, "studio", "Studio", "Studio: themes")),
            Some((
                Rail::Project(project_tools::Page::Workflows),
                "workflows",
                "Workflows",
                "Local project workflows",
            )),
            Some((
                Rail::Project(project_tools::Page::Models),
                "models",
                "Models",
                "Project runtimes and provider status",
            )),
            None,
            Some((
                Rail::View(SidebarView::SourceControl),
                "source-control",
                "Source\nControl",
                "Source Control (Ctrl+Shift+G)",
            )),
            Some((
                Rail::Project(project_tools::Page::RunDebug),
                "run-debug",
                "Run & Debug",
                "Run project; debugger availability",
            )),
            Some((
                Rail::Project(project_tools::Page::Tests),
                "tests",
                "Tests",
                "Run project tests",
            )),
            Some((
                Rail::Project(project_tools::Page::Extensions),
                "extensions",
                "Extensions",
                "Extension support status",
            )),
            None,
            Some((Rail::Ai, "ai", "AI Assistant", "AI Assistant")),
        ];
        for entry in top {
            match entry {
                None => {
                    ui.add_space(4.0);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 1.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().line_segment(
                        [
                            rect.left_center() + egui::vec2(12.0, 0.0),
                            rect.right_center() - egui::vec2(12.0, 0.0),
                        ],
                        egui::Stroke::new(1.0, theme::border()),
                    );
                    ui.add_space(4.0);
                }
                Some((item, icon, label, tip)) => {
                    let count = match item {
                        Rail::View(SidebarView::SourceControl) => badge,
                        Rail::Room => rooms,
                        _ => 0,
                    };
                    self.rail_item(ui, item, icon, label, tip, count);
                }
            }
        }
        ui.add_space(4.0);
        self.rail_item(
            ui,
            Rail::Terminal,
            "terminal",
            "Terminal",
            "Terminal (Ctrl+`)",
            0,
        );

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                ui.add_space(14.0);
                for (icon, tip, enabled) in [
                    ("cloud", "Cloud: coming soon", false),
                    ("settings", "Settings", true),
                ] {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(30.0, 30.0),
                        if enabled {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        },
                    );
                    icons::draw(ui.painter(), icon, rect.center(), 20.0, theme::text_dim());
                    if response.on_hover_text(tip).clicked() {
                        self.switch_view(SidebarView::Project(project_tools::Page::Settings));
                    }
                }
            });
            self.rail_item(
                ui,
                Rail::Project(project_tools::Page::Accessibility),
                "accessibility-off",
                "",
                "Keyboard and display help",
                0,
            );
        });
    }

    fn rail_item(
        &mut self,
        ui: &mut egui::Ui,
        item: Rail,
        icon: &str,
        label: &str,
        tip: &str,
        count: usize,
    ) {
        let soon = false;
        let height = if label.contains('\n') { 46.0 } else { 35.0 };
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ACTIVITY_WIDTH - 12.0, height),
            if soon {
                egui::Sense::hover()
            } else {
                egui::Sense::click()
            },
        );
        let rect = rect.translate(egui::vec2(6.0, 0.0));
        let active = match item {
            Rail::View(view) => !self.show_lobby && self.show_sidebar && self.view == view,
            Rail::Terminal => !self.show_lobby && self.show_terminal,
            Rail::Room => self.show_lobby,
            Rail::Ai => !self.show_lobby && self.show_right && self.right_tab == RightTab::Ai,
            Rail::Project(page) => {
                !self.show_lobby && self.show_sidebar && self.view == SidebarView::Project(page)
            }
            Rail::Studio => false,
        };
        let hovered = response.hovered() && !soon;
        let painter = ui.painter();
        if active {
            painter.rect_filled(rect, 5.0, theme::nav_active());
        } else if hovered {
            painter.rect_filled(rect, 8.0, theme::overlay(12));
        }
        let color = if soon {
            theme::mix(theme::bg_activity(), theme::text_dim(), 0.6)
        } else if active || hovered {
            theme::text()
        } else {
            theme::text_dim()
        };
        let icon_name = if active && matches!(item, Rail::Room) {
            "room-active"
        } else {
            icon
        };
        icons::draw(
            painter,
            icon_name,
            egui::pos2(rect.left() + 19.0, rect.center().y),
            19.0,
            color,
        );
        if !label.is_empty() {
            painter.text(
                egui::pos2(rect.left() + 35.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(if label.len() > 10 { 11.0 } else { 12.0 }),
                color,
            );
        }
        if count > 0 {
            let center = egui::pos2(rect.right() - 12.0, rect.center().y);
            painter.circle_filled(center, 7.0, theme::badge());
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                count.min(99),
                egui::FontId::proportional(9.5),
                theme::badge_text(),
            );
        }
        if response.on_hover_text(tip).clicked() {
            match item {
                Rail::View(view) => {
                    let showing = self.show_sidebar && self.view == view && !self.show_lobby;
                    if self.show_lobby {
                        self.show_lobby = false;
                        self.view = view;
                        self.show_sidebar = true;
                    } else if view != SidebarView::Search || showing {
                        self.switch_view(view);
                        return;
                    }
                    // Opening Search puts the cursor in its box, like Ctrl+Shift+F.
                    if view == SidebarView::Search {
                        self.run_command(Command::ShowSearch, false);
                    }
                }
                Rail::Terminal => {
                    self.show_lobby = false;
                    self.run_command(Command::ToggleTerminal, false);
                }
                Rail::Room => self.show_lobby = !self.show_lobby,
                Rail::Ai => {
                    self.show_lobby = false;
                    self.show_dock(RightTab::Ai);
                }
                Rail::Studio => self.gallery.open(),
                Rail::Project(page) => {
                    self.show_lobby = false;
                    self.switch_view(SidebarView::Project(page));
                }
            }
        }
    }

    // ---- Version control ---------------------------------------------------------------

    /// Where Git commands run: the top folder of the repository (which is above the open folder
    /// when a subfolder of a bigger repository was opened). `None` until the folder is trusted.
    fn git_root(&self) -> Option<PathBuf> {
        if !self.trusted {
            return None;
        }
        self.git_top.clone().or_else(|| self.repo_root())
    }

    /// Switch the open folder to trusted or restricted, and read the repository when trusted.
    fn set_trust(&mut self, trusted: bool) {
        self.trusted = trusted;
        self.git_top = None;
        if trusted {
            if let Some(root) = self.repo_root() {
                self.git_top = git::toplevel(&root);
            }
            self.refresh_git();
        } else {
            self.clear_git_state();
            self.git_refresh.discard();
        }
    }

    fn repo_root(&self) -> Option<PathBuf> {
        self.workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
    }

    /// Re-read `git status` and the change markers for the active file.
    /// Forget everything shown about the repository (a different folder was opened).
    fn clear_git_state(&mut self) {
        self.git_status = None;
        self.operation = None;
        self.is_repo = false;
        self.branch = None;
        self.hunks.clear();
        self.graph_commits.clear();
        self.graph_rows.clear();
        self.file_history.clear();
        self.history_name = None;
        self.stash_count = 0;
        // A read still running is for the folder that was just left: its result is not wanted, and
        // the next request must not wait for it.
        self.git_refresh.discard();
    }

    /// Ask for a fresh read of the repository. It runs on a worker thread, so a slow `git status`
    /// in a big repository never freezes the window; [`Self::poll_git_refresh`] applies the result.
    fn refresh_git(&mut self) {
        self.last_git = std::time::Instant::now();
        let Some(root) = self.git_root() else {
            self.clear_git_state();
            self.git_refresh.discard();
            return;
        };
        let context = self.egui_ctx.clone();
        self.git_refresh
            .request(&context, move || git::snapshot(&root));
    }

    /// Apply a finished move to the Recycle Bin: close the saved tabs of what is gone, refresh the
    /// tree and say what happened, including tabs with unsaved changes that were kept.
    fn poll_recycling(&mut self) {
        let Some(done) = self.recycling.poll() else {
            if self.recycling.take_failed() {
                self.status = "Moving to the Recycle Bin failed unexpectedly".into();
            }
            return;
        };
        // What matters is whether it is gone, even when the helper then reported a problem.
        let gone = !done.path.exists();
        let (mut closed, mut kept) = (0, 0);
        if gone {
            kept = self.tabs.dirty_under(&done.canonical);
            closed = self.tabs.close_clean_under(&done.canonical);
            if let Some(workspace) = &mut self.workspace {
                workspace.refresh();
            }
            self.refresh_git();
        }
        let mut text = match (&done.result, gone) {
            (Ok(()), _) => format!("Moved {} to the Recycle Bin", done.name),
            (Err(error), true) => format!("{} is gone, but: {error}", done.name),
            (Err(error), false) => error.clone(),
        };
        if closed > 0 {
            text.push_str(&format!(
                "; closed {closed} saved tab{}",
                if closed == 1 { "" } else { "s" }
            ));
        }
        if kept > 0 {
            text.push_str(&format!(
                "; {kept} tab{} with unsaved changes stay open (saving would create the file again)",
                if kept == 1 { "" } else { "s" }
            ));
        }
        self.status = text;
    }

    /// Take a finished repository read, if any, and start another when one was asked for meanwhile.
    fn poll_git_refresh(&mut self) {
        let Some(snapshot) = self.git_refresh.poll() else {
            if self.git_refresh.take_failed() {
                // Say so, rather than showing a repository as "not a repository" (with a button
                // that would run `git init`).
                self.is_repo = true;
                self.git_status = Some(Err("Reading the repository failed; retrying".into()));
            }
            return;
        };
        // The result of a read for a folder that is no longer open is dropped.
        if self.git_root().as_deref() == Some(snapshot.root.as_path()) {
            self.is_repo = snapshot.is_repo;
            self.operation = snapshot.operation;
            self.branch = snapshot.branch;
            // Files that appear or disappear (an agent creating or deleting them) change this list, and
            // then the cached Explorer listing is stale.
            let paths: Vec<String> = match &snapshot.status {
                Ok(status) => status.changes.iter().map(|c| c.path.clone()).collect(),
                Err(_) => Vec::new(),
            };
            if paths != self.change_paths {
                self.change_paths = paths;
                if let Some(workspace) = &mut self.workspace {
                    workspace.refresh();
                }
            }
            self.git_status = Some(snapshot.status);
            if self.is_repo {
                self.graph_commits = snapshot.commits;
                self.graph_rows = graph::layout(&self.graph_commits);
                self.stash_count = snapshot.stashes;
                self.refresh_hunks();
            } else {
                // Not a repository: nothing to blame or diff, and no Git processes to run.
                self.hunks.clear();
            }
        }
        if self.git_refresh.take_again() {
            self.refresh_git();
        }
    }

    /// Stage or revert the single change at `line` of the active file (from the gutter menu).
    fn change_at_line(&mut self, stage: bool, line: usize) {
        let (Some(root), Some(tab)) = (self.git_root(), self.tabs.tabs.get(self.tabs.active))
        else {
            return;
        };
        let Some(path) = tab.doc.path() else { return };
        let Ok(relative) = path.strip_prefix(&root) else {
            return;
        };
        // The change positions come from the file on disk: unsaved edits would not line up.
        if tab.doc.is_dirty() {
            self.status = "Save the file first, then stage or revert a change".into();
            return;
        }
        let relative = relative.to_string_lossy().replace('\\', "/");
        let result = if stage {
            git::stage_hunk(&root, &relative, line).map(|()| "Staged the change")
        } else {
            git::revert_hunk(&root, &relative, line).map(|()| "Reverted the change")
        };
        self.status = match result {
            Ok(message) => message.to_string(),
            Err(error) => format!("Git: {error}"),
        };
        self.refresh_git();
        if !stage {
            // The file changed on disk: the open (saved) tab shows the reverted text.
            self.reload_clean_tabs();
        }
    }

    fn refresh_hunks(&mut self) {
        let active = self.tabs.tabs.get(self.tabs.active).map(|tab| tab.id);
        if self.hunks_tab != active {
            // Another file: what is shown belongs to the previous one.
            self.hunks.clear();
            self.blame.clear();
            self.file_history.clear();
            self.history_name = None;
        }
        self.hunks_tab = active;
        let (Some(root), Some(path), Some(tab)) = (
            self.git_root(),
            self.tabs.active_path().map(std::path::Path::to_path_buf),
            active,
        ) else {
            return;
        };
        let Ok(relative) = path.strip_prefix(&root) else {
            self.file_history.clear();
            self.history_name = None;
            self.blame.clear();
            self.hunks.clear();
            return;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let untracked = matches!(&self.git_status, Some(Ok(status))
            if status.changes.iter().any(|c| c.path == relative && c.is_untracked()));
        let lines = self
            .tabs
            .tabs
            .get(self.tabs.active)
            .map(|tab| gutter::line_count(tab.doc.text()))
            .unwrap_or(0);
        let context = self.egui_ctx.clone();
        // Busy: the request is remembered and runs when this one finishes.
        self.hunks_job.request(&context, move || {
            FileGit::read(tab, &root, &relative, untracked, lines)
        });
    }

    /// Apply a finished read of the active file's history, blame and changes.
    fn poll_hunks(&mut self) {
        let Some(read) = self.hunks_job.poll() else {
            if self.hunks_job.take_failed() {
                self.hunks_job = background::Latest::default();
            }
            return;
        };
        if Some(read.tab) == self.hunks_tab {
            self.file_history = read.history;
            self.history_name = read.name;
            self.blame = read.blame;
            self.hunks = read.hunks;
        }
        if self.hunks_job.take_again() || Some(read.tab) != self.hunks_tab {
            self.refresh_hunks();
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
        let Some(root) = self.git_root() else {
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
        let Some(root) = self.git_root() else {
            return;
        };
        let upstream = self.has_upstream();
        let (ahead, behind) = match &self.git_status {
            Some(Ok(status)) => (status.ahead, status.behind),
            _ => (0, 0),
        };
        if matches!(action, Action::StashDrop) {
            self.confirm = Some(Confirm::DropStash);
            return;
        }
        if matches!(action, Action::AbortOperation) {
            self.confirm = Some(Confirm::AbortOperation);
            return;
        }
        let outcome: Result<String, String> = match action {
            Action::None => return,
            Action::Refresh => {
                self.refresh_git();
                return;
            }
            Action::Init => {
                git::run(&root, &["init"]).map(|_| "Initialized a Git repository".into())
            }
            Action::Stage(paths) => {
                // Staging a conflicted file marks it resolved, so refuse while markers remain.
                let conflicted: Vec<String> = match &self.git_status {
                    Some(Ok(status)) => status
                        .changes
                        .iter()
                        .filter(|change| change.is_conflict())
                        .map(|change| change.path.clone())
                        .collect(),
                    _ => Vec::new(),
                };
                let (blocked, allowed): (Vec<String>, Vec<String>) =
                    paths.into_iter().partition(|path| {
                        conflicted.contains(path)
                            && std::fs::read_to_string(root.join(path))
                                .is_ok_and(|text| git::has_conflict_markers(&text))
                    });
                let note = if blocked.is_empty() {
                    String::new()
                } else {
                    format!(
                        "Not staged: conflict markers remain in {}. Resolve them or use Accept Current/Incoming.",
                        blocked.join(", ")
                    )
                };
                if allowed.is_empty() {
                    Ok(note)
                } else {
                    git::stage(&root, &allowed).map(|_| note)
                }
            }
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
            Action::Resolve { path, side } => {
                git::resolve_conflict(&root, &path, side).map(|_| format!("Resolved {path}"))
            }
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
            Action::StashApply => git::stash_apply(&root, 0),
            Action::StashDrop => git::stash_drop(&root, 0),
            Action::AbortOperation => git::abort_operation(&root)
                .map(|operation| format!("{} aborted", operation.label())),
            Action::CherryPick(hash) => {
                git::cherry_pick(&root, &hash).map(|outcome| match outcome {
                    git::PickOutcome::Done(text) => format!("Cherry-picked: {text}"),
                    git::PickOutcome::Conflicts => {
                        "The cherry-pick has conflicts. Resolve them in \
                     Source Control, then commit, or abort it"
                            .into()
                    }
                })
            }
            Action::Revert(hash) => git::revert(&root, &hash).map(|outcome| match outcome {
                git::PickOutcome::Done(text) => format!("Reverted: {text}"),
                git::PickOutcome::Conflicts => "The revert has conflicts. Resolve them in \
                     Source Control, then commit, or abort it"
                    .into(),
            }),
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
        if confirmed && let Some(root) = self.git_root() {
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
                } = self
                    .search
                    .show(ui, root.as_deref(), &self.tabs.dirty_paths())
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
                self.room.show(ui, has_workspace, rooms::Section::Team);
            }
            SidebarView::SourceControl => self.source_control_view(ui, false),
            SidebarView::Project(page) => {
                let root = self.repo_root();
                match project_tools::show(ui, page, root.as_deref(), &self.runtimes) {
                    Some(project_tools::Selected::Command(command)) => {
                        self.show_terminal = true;
                        self.bottom_tab = BottomTab::Terminal;
                        // A shell that is already running may be busy with an agent or a server, and
                        // the command would be typed into its prompt: it gets a terminal of its own.
                        if self.terminals.iter().any(|t| !t.has_exited()) {
                            self.spawn_terminal(true);
                        } else {
                            self.ensure_terminal();
                        }
                        let active = self.active_terminal;
                        if let Some(terminal) = self.terminals.get_mut(active) {
                            terminal.send(format!("{command}\r").as_bytes());
                            self.status = format!("Running {command} in terminal");
                        }
                    }
                    Some(project_tools::Selected::Studio) => self.gallery.open(),
                    Some(project_tools::Selected::ToggleTerminal) => {
                        self.show_terminal = !self.show_terminal;
                    }
                    Some(project_tools::Selected::ToggleDock) => self.show_right = !self.show_right,
                    Some(project_tools::Selected::TogglePreview) => {
                        self.show_preview = !self.show_preview;
                    }
                    None => {}
                }
            }
        }
    }

    fn source_control_view(&mut self, ui: &mut egui::Ui, compact: bool) {
        let repo = if self.workspace.is_none() {
            source_control::Repo::NoFolder
        } else if self.git_status.is_none() && self.git_refresh.busy() {
            source_control::Repo::Loading
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
                    operation: self.operation,
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
            explorer::Action::NewFileIn(directory) => self.new_file_in(&directory),
            explorer::Action::NewFolder(parent) => {
                self.new_folder_parent = Some(parent);
                self.new_folder_name.clear();
            }
            explorer::Action::Rename(path) => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.rename = Some((path, name));
            }
            explorer::Action::Delete(path) => self.delete = Some(path),
            explorer::Action::CopyRelative(path) => {
                let text = match self.repo_root() {
                    Some(root) => file_ops::relative_display(&root, &path),
                    None => path.display().to_string(),
                };
                self.egui_ctx.copy_text(text);
                self.status = "Copied the relative path".into();
            }
            explorer::Action::Reveal(path) => {
                if let Err(error) = file_ops::reveal(&path) {
                    self.status = error;
                }
            }
            explorer::Action::Refresh => {
                if let Some(workspace) = self.workspace.as_mut() {
                    workspace.refresh();
                }
                self.refresh_git();
            }
        }
        if let Some(folder) = self.trust_prompt.take() {
            let (mut trust, mut restricted) = (false, false);
            egui::Window::new("Trust this folder?")
                .collapsible(false)
                .resizable(false)
                .show(&self.egui_ctx, |ui| {
                    ui.set_max_width(440.0);
                    ui.label(format!("{}", folder.display()));
                    ui.add_space(4.0);
                    ui.label(
                        "Forge Room runs Git and your project's tools in this folder. A folder from \
                         someone you do not trust can carry settings that make them start \
                         programs. Trust it only if you know where it came from.",
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        trust = ui.button("Trust this folder").clicked();
                        restricted = ui.button("Browse without Git and tools").clicked();
                    });
                });
            if ui_escape(&self.egui_ctx) {
                restricted = true;
            }
            if trust {
                let shown = folder.display().to_string();
                let _ = settings::update(|settings| {
                    settings::trust(&mut settings.trusted_folders, &shown)
                });
                self.set_trust(true);
                self.status = format!("Trusted {shown}");
            } else if restricted {
                self.status = "Restricted mode: Git and cargo check are off (Folder: Trust This Folder turns them on)".into();
            } else {
                self.trust_prompt = Some(folder);
            }
        }
        if let Some(question) = self.confirm.take() {
            let (mut yes, mut no) = (false, false);
            let (title, text, button) = match question {
                Confirm::DropStash => {
                    let latest = self
                        .git_root()
                        .and_then(|root| git::stash_list(&root).into_iter().next())
                        .map(|stash| stash.message)
                        .unwrap_or_default();
                    (
                        "Drop stash",
                        format!(
                            "Drop the latest stash ({latest})? Its changes are removed from the \
                             stash list; the status line shows the commit so it can be brought back \
                             with git stash store."
                        ),
                        "Drop stash",
                    )
                }
                Confirm::AbortOperation => (
                    "Abort",
                    format!(
                        "Abort the {}? Everything done since it started is thrown away, including \
                         conflicts you have already resolved and saved: the files go back to how \
                         they were before.",
                        self.operation
                            .map(|operation| operation.label().to_lowercase())
                            .unwrap_or_else(|| "operation".into())
                    ),
                    "Abort",
                ),
                Confirm::QuitWithTerminals => (
                    "Quit",
                    format!(
                        "{} terminal(s) are still running. Quitting stops the programs in them \
                         (coding agents, dev servers).",
                        self.terminals.iter().filter(|t| !t.has_exited()).count()
                    ),
                    "Quit anyway",
                ),
                Confirm::RunCheck => {
                    let manifest = self
                        .repo_root()
                        .and_then(|root| diagnostics::find_manifest(&root))
                        .map(|path| path.display().to_string())
                        .unwrap_or_default();
                    (
                        "Run cargo check",
                        format!(
                            "cargo check runs the build scripts and macros of this project \
                             ({manifest}) with your rights. Only do that for code you trust."
                        ),
                        "Run cargo check",
                    )
                }
                Confirm::EndTerminal => (
                    "End terminal",
                    format!(
                        "End terminal {}? A program still running in it (a dev server, a build, a \
                         coding agent) is stopped.",
                        self.active_terminal + 1
                    ),
                    "End terminal",
                ),
            };
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .show(&self.egui_ctx, |ui| {
                    ui.set_max_width(380.0);
                    ui.label(text);
                    ui.horizontal(|ui| {
                        yes = ui.button(button).clicked();
                        no = ui.button("Cancel").clicked();
                    });
                });
            if ui_escape(&self.egui_ctx) {
                no = true;
            }
            if yes {
                match question {
                    Confirm::DropStash => {
                        if let Some(root) = self.git_root() {
                            self.status = match git::stash_drop(&root, 0) {
                                Ok(text) => format!(
                                    "{text}. To bring it back: git stash store <that commit>"
                                ),
                                Err(error) => format!("Git: {error}"),
                            };
                            self.refresh_git();
                        }
                    }
                    Confirm::EndTerminal => self.close_terminal(),
                    Confirm::AbortOperation => {
                        if let Some(root) = self.git_root() {
                            self.status = match git::abort_operation(&root) {
                                Ok(operation) => format!("{} aborted", operation.label()),
                                Err(error) => format!("Git: {error}"),
                            };
                            self.refresh_git();
                            self.reload_clean_tabs();
                        }
                    }
                    Confirm::QuitWithTerminals => {
                        self.terminals_confirmed = true;
                        self.egui_ctx
                            .send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Confirm::RunCheck => {
                        if let Some(root) = self.repo_root() {
                            self.checked_folders.insert(root);
                        }
                        self.start_check();
                    }
                }
            } else if !no {
                self.confirm = Some(question);
            }
        }
        if let Some(path) = self.delete.take() {
            let (mut confirm, mut cancel) = (false, false);
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            egui::Window::new("Delete")
                .collapsible(false)
                .resizable(false)
                .show(&self.egui_ctx, |ui| {
                    ui.label(format!(
                        "Move {} {name:?} to the Recycle Bin?",
                        if path.is_dir() {
                            "the folder"
                        } else {
                            "the file"
                        }
                    ));
                    ui.label(
                        egui::RichText::new("You can restore it from the Recycle Bin.")
                            .color(theme::text_dim()),
                    );
                    ui.horizontal(|ui| {
                        confirm = ui.button("Move to Recycle Bin").clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
            if ui_escape(&self.egui_ctx) {
                cancel = true;
            }
            // The open folder itself, or a folder that holds it, is not something to delete from here.
            let holds_the_project = self.workspace.as_ref().is_some_and(|workspace| {
                let (target, root) = (tabs::normalize(&path), tabs::normalize(&workspace.root));
                root.starts_with(&target)
            });
            if confirm && holds_the_project {
                self.status =
                    "The open folder (or a folder that holds it) cannot be deleted here".into();
            } else if confirm {
                // The tabs hold canonical paths, and this one can only be made canonical while the
                // file still exists.
                let canonical = tabs::normalize(&path);
                let context = self.egui_ctx.clone();
                let (moved, label) = (path.clone(), name.clone());
                if self.recycling.request(&context, move || Recycled {
                    result: file_ops::recycle(&moved),
                    path: moved,
                    canonical,
                    name: label,
                }) {
                    self.status = format!("Moving {name} to the Recycle Bin…");
                } else {
                    self.status = "Another delete is still running".into();
                }
            } else if !cancel {
                self.delete = Some(path);
            }
        }
        if let Some((path, mut name)) = self.rename.take() {
            let (mut confirm, mut cancel) = (false, false);
            let error = file_ops::valid_name(&name).err();
            egui::Window::new("Rename")
                .collapsible(false)
                .resizable(false)
                .show(&self.egui_ctx, |ui| {
                    ui.label(format!("{}", path.display()));
                    let field = ui.text_edit_singleline(&mut name);
                    field.request_focus();
                    if field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                        confirm = true;
                    }
                    if let Some(error) = &error {
                        ui.colored_label(egui::Color32::from_rgb(240, 113, 120), error);
                    }
                    ui.horizontal(|ui| {
                        confirm |= ui
                            .add_enabled(error.is_none(), egui::Button::new("Rename"))
                            .clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
            if ui_escape(&self.egui_ctx) {
                cancel = true;
            }
            if confirm && error.is_none() {
                // The tabs hold canonical paths, and the old path can only be made canonical
                // while it still exists.
                let old_path = tabs::normalize(&path);
                match file_ops::rename(&path, &name) {
                    Ok(new_path) => {
                        self.tabs.path_moved(&old_path, &new_path);
                        if let Some(workspace) = &mut self.workspace {
                            workspace.refresh();
                        }
                        self.refresh_git();
                        self.status = format!("Renamed to {}", name.trim());
                    }
                    Err(message) => {
                        self.status = message;
                        self.rename = Some((path, name));
                    }
                }
            } else if !cancel {
                self.rename = Some((path, name));
            }
        }
        if let Some(parent) = self.new_folder_parent.clone() {
            let mut create = false;
            let mut cancel = false;
            egui::Window::new("New folder")
                .collapsible(false)
                .resizable(false)
                .show(&self.egui_ctx, |ui| {
                    ui.label(format!("In {}", parent.display()));
                    let field = ui.text_edit_singleline(&mut self.new_folder_name);
                    if field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                        create = true;
                    }
                    ui.horizontal(|ui| {
                        create |= ui.button("Create").clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
            if cancel {
                self.new_folder_parent = None;
            }
            if create {
                let name = self.new_folder_name.trim();
                if name.is_empty()
                    || name == "."
                    || name == ".."
                    || name.contains('/')
                    || name.contains('\\')
                {
                    self.status = "Enter one valid folder name".into();
                } else {
                    match fs::create_dir(parent.join(name)) {
                        Ok(()) => {
                            if let Some(workspace) = &mut self.workspace {
                                workspace.refresh();
                            }
                            self.new_folder_parent = None;
                            self.status = format!("Created folder {name}");
                        }
                        Err(error) => self.status = format!("Cannot create folder: {error}"),
                    }
                }
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
                        let mut title = tab.title();
                        if tab.external.is_some() {
                            title.push_str("  (!)");
                        }
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
                        if let Some(what) = tab.external {
                            response.clone().on_hover_text(format!(
                                "This file was {what}. Saving would {}.",
                                if what == "deleted on disk" {
                                    "create it again"
                                } else {
                                    "replace the other change"
                                }
                            ));
                        }

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
        let mut font_id = egui::TextStyle::Monospace.resolve(ui.style());
        font_id.size *= self.editor_scale;
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
        // A change menu choice in the gutter: (stage it, line).
        let mut hunk_choice: Option<(bool, usize)> = None;
        let select = self.pending_select.take();
        let transform = self.pending_edit.take();
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
                            if syntax::pending() {
                                ui.ctx().request_repaint(); // highlighting is catching up
                            }
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
                    // The current-line wash is opaque, so it must sit behind the text: reserve its
                    // place in the paint order now and fill it once the line's position is known.
                    let line_wash = ui.painter().add(egui::Shape::Noop);
                    if !readonly {
                        // Enter keeps the indentation and opens bracket blocks; Tab and Shift+Tab
                        // indent and unindent the line or the selected lines.
                        if !editing::smart_tab(ui, id, tab.doc.text_mut()) {
                            editing::smart_enter(ui, id, tab.doc.text_mut());
                        }
                        // A transformation chosen in the command palette works on the selection.
                        if let Some(kind) = transform {
                            transforms::run(ui, id, tab.doc.text_mut(), kind);
                        }
                        // Brackets and quotes are completed, stepped over and deleted in pairs.
                        editing::smart_pairs(ui, id, tab.doc.text_mut(), &extension);
                        // Ctrl+/ comments lines; Alt+Up/Down move them, Shift+Alt+Up/Down
                        // duplicate them and Ctrl+Shift+K deletes them.
                        line_ops::handle(ui, id, tab.doc.text_mut(), &extension);
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
                            // Right-click the coloured bar to stage or revert just this change.
                            let strip = egui::Rect::from_min_size(
                                egui::pos2(gutter.right() - 12.0, top),
                                egui::vec2(14.0, height),
                            );
                            ui.interact(
                                strip,
                                ui.scope_id().with(("hunk-bar", line)),
                                egui::Sense::click(),
                            )
                            .on_hover_text("Right-click: stage or revert this change")
                            .context_menu(|ui| {
                                if ui.button("Stage this change").clicked() {
                                    hunk_choice = Some((true, line + 1));
                                    ui.close();
                                }
                                if ui.button("Revert this change").clicked() {
                                    hunk_choice = Some((false, line + 1));
                                    ui.close();
                                }
                            });
                        }
                        if current {
                            painter.set(
                                line_wash,
                                egui::Shape::rect_filled(
                                    egui::Rect::from_min_max(
                                        egui::pos2(gutter.left(), top),
                                        egui::pos2(clip.right().max(gutter.right()), top + height),
                                    ),
                                    0.0,
                                    theme::line_highlight(),
                                ),
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
        if let Some((stage, line)) = hunk_choice {
            self.change_at_line(stage, line);
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
            let tabs_width = (ui.available_width() - 40.0).max(160.0);
            ui.allocate_ui_with_layout(
                egui::vec2(tabs_width, 36.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    egui::ScrollArea::horizontal()
                        .auto_shrink([false, true])
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                // The AI agent comes first: it is the tab people open most, and the
                                // last tabs are the ones a narrow dock cuts off.
                                for (tab, label) in [
                                    (RightTab::Ai, "AI"),
                                    (RightTab::Room, "Team"),
                                    (RightTab::Chat, "Chat"),
                                    (RightTab::Agents, "Agents"),
                                    (RightTab::Tasks, "Tasks"),
                                    (RightTab::Activity, "Activity"),
                                ] {
                                    if underline_tab_padded(
                                        ui,
                                        label,
                                        self.right_tab == tab,
                                        true,
                                        16.0,
                                    )
                                    .clicked()
                                    {
                                        self.right_tab = tab;
                                    }
                                }
                            });
                        });
                },
            );
            if icons::icon_button(ui, "Hide panel", icons::close_cross).clicked() {
                self.show_right = false;
            }
        });
        ui.painter().line_segment(
            [
                egui::pos2(ui.max_rect().left(), ui.cursor().top()),
                egui::pos2(ui.max_rect().right(), ui.cursor().top()),
            ],
            egui::Stroke::new(1.0, theme::border()),
        );
        if self.right_tab != RightTab::Ai {
            let review = self
                .git_status
                .as_ref()
                .and_then(|result| result.as_ref().ok())
                .map(|status| {
                    let conflicts = status
                        .changes
                        .iter()
                        .filter(|change| change.is_conflict())
                        .count();
                    (status.changes.len(), conflicts)
                });
            if let Some((changes, conflicts)) = review.filter(|(changes, _)| *changes > 0) {
                egui::Frame::new()
                    .fill(theme::mix(theme::bg_sidebar(), theme::text(), 0.04))
                    .inner_margin(egui::Margin::symmetric(14, 7))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let label = if conflicts > 0 {
                                format!(
                                    "{conflicts} merge conflict{}",
                                    if conflicts == 1 { "" } else { "s" }
                                )
                            } else {
                                format!(
                                    "{changes} changed file{}",
                                    if changes == 1 { "" } else { "s" }
                                )
                            };
                            let color = if conflicts > 0 {
                                egui::Color32::from_rgb(240, 113, 120)
                            } else {
                                theme::text_dim()
                            };
                            ui.label(egui::RichText::new(label).color(color));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Review").clicked() {
                                        self.view = SidebarView::SourceControl;
                                        self.show_sidebar = true;
                                        self.refresh_git();
                                    }
                                },
                            );
                        });
                    });
                ui.separator();
            }
        }
        match self.right_tab {
            RightTab::Room
            | RightTab::Chat
            | RightTab::Agents
            | RightTab::Tasks
            | RightTab::Activity => {
                let has_workspace = self.workspace.is_some();
                let part = match self.right_tab {
                    RightTab::Agents => rooms::Section::Agents,
                    RightTab::Chat => rooms::Section::Chat,
                    RightTab::Tasks => rooms::Section::Tasks,
                    RightTab::Activity => rooms::Section::Activity,
                    _ => rooms::Section::Team,
                };
                self.room.show(ui, has_workspace, part);
            }
            RightTab::Ai => {
                let name = self.tabs.tabs.get(self.tabs.active).map(|tab| tab.title());
                let cwd = self.repo_root();
                ai_panel::show(
                    ui,
                    &mut self.ai,
                    name.as_deref(),
                    cwd.as_deref(),
                    self.trusted,
                );
            }
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
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
            if let Some((name, _, _)) = self.room.summary() {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(12.0, 16.0), egui::Sense::hover());
                ui.painter().circle_filled(
                    dot.center(),
                    4.0,
                    egui::Color32::from_rgb(240, 113, 120),
                );
                ui.label(egui::RichText::new(name).color(theme::status_text()));
                let pill = ui.add(
                    egui::Button::new(
                        egui::RichText::new("Lobby")
                            .color(theme::status_text())
                            .size(13.0),
                    )
                    .fill(if self.show_lobby {
                        theme::mix(theme::status_bar(), theme::accent(), 0.3)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .corner_radius(6.0),
                );
                if pill.on_hover_text("Room Lobby").clicked() {
                    self.show_lobby = !self.show_lobby;
                }
                ui.add_space(10.0);
            }
            ui.label(egui::RichText::new(&self.status).color(theme::status_dim()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dim = |text: String| egui::RichText::new(text).color(theme::status_dim());
                ui.label(dim(forge_room_core::NATIVE_PREVIEW_VERSION.to_string()));
                ui.add_space(6.0);
                if let Some(tab) = self.tabs.tabs.get(self.tabs.active) {
                    ui.label(dim(tab.doc.line_ending().label().to_string()));
                    ui.label(dim(tab.doc.encoding().label().to_string()));
                    if let Some(name) = tab.doc.file_name()
                        && let Some(language) = language_name(&name)
                    {
                        ui.label(dim(language.to_string()));
                    }
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
        if let Pending::DiskConflict(index, resume) = pending {
            let (index, resume) = (*index, *resume);
            self.disk_conflict_dialog(context, index, resume);
            return;
        }
        let (message, save_label) = match pending {
            Pending::DiskConflict(..) => return,
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
                self.resume = Resume::CloseTab;
                let saved = self.save_index(index, false);
                self.resume = Resume::Nothing;
                if saved {
                    self.tabs.close(index);
                }
            }
            (Some(Pending::CloseTab(index)), Choice::Discard) => self.tabs.close(index),
            (Some(Pending::Quit), Choice::Save) => self.save_all_and_quit(context),
            (Some(Pending::Quit), Choice::Discard) => {
                self.allow_close = true;
                // Discarded means gone: not even a recovery copy may bring it back.
                self.recovery.clear();
                context.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            (Some(Pending::DiskConflict(..)) | None, _) => {}
        }
        self.focus_editor = true;
    }

    /// Save every dirty tab and close the window. If a tab hits a disk conflict the dialog takes
    /// over, remembers this intent, and calls here again once the conflict is settled.
    fn save_all_and_quit(&mut self, context: &egui::Context) {
        let dirty: Vec<usize> = (0..self.tabs.tabs.len())
            .filter(|index| self.tabs.tabs[*index].doc.is_dirty())
            .collect();
        self.resume = Resume::Quit;
        let all_saved = dirty.into_iter().all(|index| self.save_index(index, false));
        self.resume = Resume::Nothing;
        if all_saved {
            self.allow_close = true;
            self.recovery.clear();
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Carry on with what a conflicted save was for.
    fn resume_after_conflict(&mut self, context: &egui::Context, index: usize, resume: Resume) {
        match resume {
            Resume::Nothing => {}
            Resume::CloseTab => {
                if index < self.tabs.tabs.len() {
                    self.tabs.close(index);
                }
            }
            Resume::Quit => self.save_all_and_quit(context),
        }
    }

    fn disk_conflict_dialog(&mut self, context: &egui::Context, index: usize, resume: Resume) {
        let Some(name) = self.tabs.tabs.get(index).map(|tab| tab.title()) else {
            self.pending = None;
            return;
        };
        let mut action = None;
        let modal = egui::Modal::new(egui::Id::unique("disk-conflict-dialog")).show(context, |ui| {
            ui.set_width(420.0);
            ui.heading("File changed on disk");
            ui.add_space(6.0);
            ui.label(format!(
                "\"{name}\" was modified outside Forge Room after you opened it, and you have unsaved edits. Saving now would erase the other changes."
            ));
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button("Overwrite").clicked() {
                    action = Some(0);
                }
                if ui.button("Reload from Disk").clicked() {
                    action = Some(1);
                }
                if ui.button("Save As…").clicked() {
                    action = Some(2);
                }
                if ui.button("Cancel").clicked() {
                    action = Some(3);
                }
            });
        });
        if modal.should_close() && action.is_none() {
            action = Some(3);
        }
        let Some(action) = action else {
            return;
        };
        self.pending = None;
        let Some(tab) = self.tabs.tabs.get_mut(index) else {
            return;
        };
        // True when the choice settled the conflict, so the interrupted operation can go on.
        let settled = match action {
            0 => match tab.doc.overwrite() {
                Ok(()) => {
                    self.status = format!("Saved {name}, replacing the outside changes");
                    self.refresh_git();
                    true
                }
                Err(error) => {
                    self.status = format!("Cannot save {name}: {error}");
                    false
                }
            },
            1 => match tab.doc.reload() {
                Ok(()) => {
                    self.status = format!("Reloaded {name} from disk; your edits were discarded");
                    // Not "settled": a close or quit that led here is asked again, so discarding
                    // work never ends with the window closing by itself.
                    false
                }
                Err(error) => {
                    self.status = format!("Cannot reload {name}: {error}");
                    false
                }
            },
            2 => self.save_index(index, true),
            // Cancel also cancels the close or quit that led here.
            _ => false,
        };
        if settled {
            self.resume_after_conflict(context, index, resume);
        }
        self.focus_editor = true;
    }

    /// A slim Room Pulse strip over the workspace: the latest real room events, or a hint.
    fn pulse_strip(&mut self, ui: &mut egui::Ui) {
        let painter = ui.painter().clone();
        let rect = ui.max_rect();
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0, theme::border()),
        );
        let toggle = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 104.0, rect.center().y - 12.0),
            egui::vec2(96.0, 24.0),
        );
        let label = if self.show_preview {
            "Hide Preview"
        } else {
            "Preview"
        };
        let response = ui
            .interact(
                toggle,
                ui.make_persistent_id("preview-toggle"),
                egui::Sense::click(),
            )
            .on_hover_text("Show or hide the Preview pane");
        let fill = if response.hovered() {
            theme::overlay(18)
        } else {
            egui::Color32::TRANSPARENT
        };
        painter.rect(
            toggle,
            6.0,
            fill,
            egui::Stroke::new(1.0, theme::border()),
            egui::StrokeKind::Inside,
        );
        painter.text(
            toggle.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.5),
            theme::text(),
        );
        if response.clicked() {
            self.show_preview = !self.show_preview;
        }
        let mut open_activity = false;
        ui.horizontal_centered(|ui| {
            let (icon, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
            icons::draw(ui.painter(), "pulse", icon.center(), 18.0, theme::accent());
            ui.label(
                egui::RichText::new("Room Pulse")
                    .family(fonts::semibold())
                    .color(theme::text()),
            )
            .on_hover_text("Recent activity saved in this local Room");
            let Some(state) = self.room.state() else {
                ui.label(
                    egui::RichText::new("No room yet. Create one from the Lobby.")
                        .color(theme::text_dim()),
                );
                return;
            };
            if state.activity.is_empty() {
                ui.label(egui::RichText::new("No activity yet").color(theme::text_dim()));
                return;
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|since| since.as_secs())
                .unwrap_or(0);
            // Reserve the preview control at the right and keep each event in its own segment.
            let slots = ((ui.available_width() - 120.0) / 230.0).floor().max(0.0) as usize;
            for event in state.activity.iter().rev().take(slots.min(4)) {
                let actor = state
                    .participants
                    .iter()
                    .find(|person| person.id == event.actor_id)
                    .map(|person| person.name.as_str())
                    .unwrap_or("Someone");
                ui.separator();
                let text = format!(
                    "{}: {}\n{}",
                    rooms::short(actor, 12),
                    rooms::short(&event.detail, 22),
                    room_state::ago(now, event.recorded_at_epoch_seconds)
                );
                if ui
                    .add_sized(
                        [216.0, 28.0],
                        egui::Button::new(
                            egui::RichText::new(text)
                                .size(12.0)
                                .color(theme::text_dim()),
                        )
                        .frame(false),
                    )
                    .on_hover_text(format!("{actor}: {}\nOpen local activity", event.detail))
                    .clicked()
                {
                    open_activity = true;
                }
            }
        });
        if open_activity {
            self.show_dock(RightTab::Activity);
        }
    }

    fn lobby_page(&mut self, ui: &mut egui::Ui) {
        let name = self
            .workspace
            .as_ref()
            .map(|workspace| workspace.name())
            .unwrap_or_else(|| "Forge Room".into());
        let context = lobby::Context {
            workspace: &name,
            branch: self.branch.as_deref(),
            runtimes: &self.runtimes,
            state: self.room.state(),
            room_unavailable: self.room.is_unavailable(),
            acting_as: self.room.acting_as(),
        };
        match self.lobby.show(ui, &context) {
            Some(lobby::Action::EnterRoom) => {
                self.show_lobby = false;
                self.show_right = true;
                self.right_tab = RightTab::Room;
            }
            Some(lobby::Action::OpenActivity) => {
                self.show_lobby = false;
                self.show_dock(RightTab::Activity);
            }
            Some(lobby::Action::CreateRoom) => {
                if !self.room.create_room() {
                    self.show_dock(RightTab::Room);
                }
            }
            Some(lobby::Action::AddMember | lobby::Action::AddAgent) => {
                self.show_dock(RightTab::Room)
            }
            None => {}
        }
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
    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::from_rgb(10, 11, 12).to_normalized_gamma_f32()
    }

    fn on_exit(&mut self) {
        let _ = self.lobby.finish();
        self.save_session(true);
        // A normal exit already asked about every unsaved tab: nothing is left to recover.
        self.recovery.clear();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        self.lobby.flush_pending();
        for terminal in &self.terminals {
            terminal.set_hidden();
        }
        // An agent that was started from the AI panel: show what it says, and when it ends look at
        // the files again (it may have edited some).
        if self.ai.poll() {
            self.reload_clean_tabs();
            self.refresh_git();
            self.status = "The agent finished".into();
        }
        self.auto_save_tabs();
        self.poll_check();
        self.save_session(false);
        self.autosave_recovery();
        if context.input(|input| input.viewport().close_requested())
            && self.tabs.any_dirty()
            && !self.allow_close
        {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Quit);
        }
        // Shells that are still running are stopped by quitting: ask once.
        if context.input(|input| input.viewport().close_requested())
            && !self.allow_close
            && !self.terminals_confirmed
            && self.pending.is_none()
            && self.terminals.iter().any(|t| !t.has_exited())
        {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm = Some(Confirm::QuitWithTerminals);
        }
        // Closing with lobby edits that cannot be saved asks once before losing them.
        if context.input(|input| input.viewport().close_requested())
            && !self.lobby_close_warned
            && let Err(error) = self.lobby.finish()
        {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.lobby_close_warned = true;
            self.status = format!("{error} Close again to quit and lose those edits.");
        }
        if self.pending.is_none() {
            self.handle_shortcuts(ui);
            if self.last_disk_check.elapsed() >= std::time::Duration::from_secs(1) {
                self.last_disk_check = std::time::Instant::now();
                self.sync_external_changes();
            }
        }
        context.request_repaint_after(std::time::Duration::from_secs(1));

        let bar = |fill| {
            egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(14, 0))
        };

        // Flat panels separated by hairline dividers, as in the reference.
        let card = |fill| egui::Frame::new().fill(fill);

        if self.show_terminal && self.terminals.is_empty() && self.terminal_error.is_none() {
            self.ensure_terminal();
        }

        egui::Panel::top("menu")
            .frame(egui::Frame::new().fill(theme::title_bar()))
            .exact_size(TITLE_HEIGHT)
            .show(ui, |ui| self.title_bar(ui));

        egui::Panel::bottom("status")
            .exact_size(29.0)
            .frame(bar(theme::status_bar()))
            .show(ui, |ui| self.status_bar(ui));

        egui::Panel::left("activity")
            .exact_size(ACTIVITY_WIDTH)
            .resizable(false)
            .frame(egui::Frame::new().fill(theme::bg_activity()))
            .show(ui, |ui| self.activity_bar(ui));

        if !self.show_lobby {
            egui::Panel::top("pulse")
                .exact_size(34.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(theme::bg_sidebar())
                        .inner_margin(egui::Margin::symmetric(12, 0)),
                )
                .show(ui, |ui| self.pulse_strip(ui));
        }

        if self.show_sidebar && !self.show_lobby {
            let drawn = egui::Panel::left("sidebar")
                .default_size(f32::from(self.panel_sizes.0))
                .size_range(180.0..=560.0)
                .resizable(true)
                .frame(card(theme::bg_sidebar()))
                .show(ui, |ui| self.sidebar(ui));
            self.panel_sizes.0 = drawn.response.rect.width().round().clamp(180.0, 560.0) as u16;
        }

        if self.show_right && !self.show_lobby {
            let drawn = egui::Panel::right("right-dock")
                .default_size(f32::from(self.panel_sizes.1))
                .size_range(280.0..=640.0)
                .resizable(true)
                .frame(card(theme::bg_sidebar()))
                .show(ui, |ui| self.right_dock(ui));
            self.panel_sizes.1 = drawn.response.rect.width().round().clamp(280.0, 640.0) as u16;
        }

        if self.show_terminal && !self.show_lobby {
            let panel = if self.terminal_maximized {
                // Most of the window; the editor keeps a strip above it.
                egui::Panel::bottom("terminal-maximized")
                    .exact_size((ui.available_height() - 90.0).max(200.0))
            } else {
                egui::Panel::bottom("terminal")
                    .resizable(true)
                    .default_size(f32::from(self.panel_sizes.2))
                    .size_range(120.0..=720.0)
            };
            let drawn = panel
                .frame(card(theme::bg_editor()).inner_margin(egui::Margin {
                    left: 0,
                    right: 0,
                    top: 4,
                    bottom: 0,
                }))
                .show(ui, |ui| self.terminal_panel(ui));
            if !self.terminal_maximized {
                self.panel_sizes.2 =
                    drawn.response.rect.height().round().clamp(120.0, 720.0) as u16;
            }
        }

        if self.show_preview && !self.show_lobby && ui.available_width() > 760.0 {
            if let Some(terminal) = self.terminals.get(self.active_terminal) {
                self.preview.detect(&terminal.screen_text());
            }
            let mut close = false;
            egui::Panel::right("preview")
                .default_size(220.0)
                .size_range(170.0..=420.0)
                .resizable(true)
                .frame(card(theme::bg_sidebar()))
                .show(ui, |ui| close = self.preview.show(ui));
            if close {
                self.show_preview = false;
            }
        }

        egui::CentralPanel::default()
            .frame(card(theme::bg_editor()))
            .show(ui, |ui| {
                if self.show_lobby {
                    self.lobby_page(ui);
                    return;
                }
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

        if let Some(done) = self.search.take_replaced() {
            self.status = replaced_status(&done);
            self.reload_clean_tabs();
            self.refresh_git();
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
                if let Some(root) = self.git_root() {
                    self.status = match git::checkout(&root, &name, remote) {
                        Ok(()) => format!("Switched to {name}"),
                        Err(error) => format!("Git: {error}"),
                    };
                    self.refresh_git();
                    self.reload_clean_tabs();
                }
            }
            Some(palette::Outcome::CreateBranch(name)) => {
                if let Some(root) = self.git_root() {
                    self.status = match git::create_branch(&root, &name) {
                        Ok(()) => format!("Created and switched to {name}"),
                        Err(error) => format!("Git: {error}"),
                    };
                    self.refresh_git();
                }
            }
            Some(palette::Outcome::Merge(name)) => {
                if let Some(root) = self.git_root() {
                    self.status = match git::merge(&root, &name) {
                        Ok(git::MergeOutcome::Merged(text)) => format!("Merged {name}: {text}"),
                        Ok(git::MergeOutcome::UpToDate) => {
                            format!("Already up to date with {name}")
                        }
                        Ok(git::MergeOutcome::Conflicts) => format!(
                            "Merging {name} has conflicts. Resolve them in Source Control, \
                             then commit, or run Git: Abort Merge"
                        ),
                        Err(error) => format!("Git: {error}"),
                    };
                    self.refresh_git();
                    self.reload_clean_tabs();
                }
            }
            Some(palette::Outcome::DeleteBranch(name)) => {
                if let Some(root) = self.git_root() {
                    self.status = match git::delete_branch(&root, &name, false) {
                        Ok(()) => format!("Deleted branch {name}"),
                        Err(error) => format!("Git: {error}"),
                    };
                    self.refresh_git();
                }
            }
            Some(palette::Outcome::CreateTag(name)) => {
                if let Some(root) = self.git_root() {
                    self.status = match git::create_tag(&root, &name) {
                        Ok(()) => format!("Tagged the current commit {name}"),
                        Err(error) => format!("Git: {error}"),
                    };
                    self.refresh_git();
                }
            }
            Some(palette::Outcome::DeleteTag(name)) => {
                if let Some(root) = self.git_root() {
                    self.status = match git::delete_tag(&root, &name) {
                        Ok(text) => match text.split("(was ").nth(1) {
                            Some(rest) => format!(
                                "{text}. To make it again: git tag {name} {}",
                                rest.trim_end_matches(')')
                            ),
                            None => text,
                        },
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
                let key = theme::current().key.clone();
                if let Err(error) = settings::update(|settings| settings.theme = Some(key)) {
                    self.status = format!("Could not save settings: {error}");
                }
            }
            Some(theme_gallery::Outcome::Cancel(key)) => theme::set(&context, &key),
            None => {}
        }

        self.poll_git_job(&context);
        self.poll_git_refresh();
        self.poll_hunks();
        self.poll_recycling();
        self.discard_dialog(&context);
        self.pending_dialog(&context);
        self.update_title(&context);
    }
}

/// A rounded pill in the title bar: optional icon or status dot, text, optional chevron.
fn title_chip(
    ui: &mut egui::Ui,
    icon: Option<&str>,
    text: &str,
    dot: Option<egui::Color32>,
    chevron: bool,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(13.5),
        theme::text(),
    );
    let lead = if icon.is_some() || dot.is_some() {
        26.0
    } else {
        10.0
    };
    let tail = if chevron { 22.0 } else { 10.0 };
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(lead + galley.size().x + tail, 28.0),
        egui::Sense::click(),
    );
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, 8.0, theme::overlay(14));
    }
    if let Some(icon) = icon {
        icons::draw(
            painter,
            icon,
            egui::pos2(rect.left() + 15.0, rect.center().y),
            16.0,
            theme::text_dim(),
        );
    }
    if let Some(color) = dot {
        painter.circle_filled(egui::pos2(rect.left() + 15.0, rect.center().y), 4.0, color);
    }
    painter.galley(
        egui::pos2(rect.left() + lead, rect.center().y - galley.size().y / 2.0),
        galley.clone(),
        theme::text(),
    );
    if chevron {
        icons::chevron(
            painter,
            egui::pos2(rect.right() - 12.0, rect.center().y),
            true,
            theme::text_dim(),
        );
    }
    response
}

/// Overlapping initials circles for the people in the room, with a "+N" for the rest.
fn avatar_stack(ui: &mut egui::Ui, names: &[&str]) {
    let shown = names.len().min(4);
    let extra = names.len() - shown;
    let step = 20.0;
    let width = 28.0 + step * shown.saturating_sub(1) as f32 + if extra > 0 { 30.0 } else { 0.0 };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::hover());
    let painter = ui.painter();
    for (index, name) in names.iter().take(shown).enumerate() {
        let center = egui::pos2(rect.left() + 14.0 + step * index as f32, rect.center().y);
        let hue = name.bytes().fold(7u32, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(u32::from(b))
        });
        let palette = [
            egui::Color32::from_rgb(88, 120, 170),
            egui::Color32::from_rgb(120, 96, 160),
            egui::Color32::from_rgb(90, 140, 120),
            egui::Color32::from_rgb(170, 110, 90),
        ];
        painter.circle_filled(center, 13.0, theme::title_bar());
        painter.circle_filled(center, 11.5, palette[hue as usize % palette.len()]);
        let initial: String = name.chars().next().into_iter().collect();
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            initial.to_uppercase(),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }
    if extra > 0 {
        painter.text(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            egui::Align2::CENTER_CENTER,
            format!("+{extra}"),
            egui::FontId::proportional(12.5),
            theme::text(),
        );
    }
    response.on_hover_text(names.join(", "));
}

/// Display name of the language for a file name, judged from its extension.
/// What is read about the active file from Git, on a worker thread.
struct FileGit {
    tab: u64,
    history: Vec<git::CommitInfo>,
    name: Option<String>,
    blame: Vec<git::BlameLine>,
    hunks: Vec<git::Hunk>,
}

impl FileGit {
    fn read(
        tab: u64,
        root: &std::path::Path,
        relative: &str,
        untracked: bool,
        lines: usize,
    ) -> Self {
        let history = git::file_log(root, relative, 30).unwrap_or_default();
        let name = relative.rsplit('/').next().map(String::from);
        let blame = if untracked {
            Vec::new()
        } else {
            git::blame(root, relative)
        };
        let hunks = if untracked {
            vec![git::Hunk {
                kind: git::HunkKind::Added,
                start: 1,
                len: lines,
            }]
        } else {
            git::file_hunks(root, relative)
        };
        FileGit {
            tab,
            history,
            name,
            blame,
            hunks,
        }
    }
}

/// A question the user must answer before something that cannot be undone happens.
#[derive(Clone, Copy)]
enum Confirm {
    DropStash,
    EndTerminal,
    /// `cargo check` runs the folder's build scripts and macros.
    RunCheck,
    /// Abort the merge, cherry-pick, revert or rebase in progress.
    AbortOperation,
    /// Quit while shells are still running.
    QuitWithTerminals,
}

/// What to tell the user after Replace All.
fn replaced_status(done: &search::Replaced) -> String {
    if let Some(error) = &done.error {
        return format!("Replace failed: {}", error.lines().next().unwrap_or(""));
    }
    let mut text = format!(
        "Replaced {} match{} in {} file{}",
        done.replacements,
        if done.replacements == 1 { "" } else { "es" },
        done.files,
        if done.files == 1 { "" } else { "s" }
    );
    if !done.skipped.is_empty() {
        let (path, why) = &done.skipped[0];
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        text.push_str(&format!(
            "; skipped {} (first: {name}, {why})",
            done.skipped.len()
        ));
    }
    text
}

/// The outcome of moving a file or folder to the Recycle Bin on a worker thread.
struct Recycled {
    path: PathBuf,
    canonical: PathBuf,
    name: String,
    result: Result<(), String>,
}

/// Whether Escape was pressed this frame (and takes it, so nothing else reacts).
fn ui_escape(context: &egui::Context) -> bool {
    context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
}

fn language_name(name: &str) -> Option<&'static str> {
    let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match extension.as_str() {
        "rs" => "Rust",
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "py" => "Python",
        "go" => "Go",
        "json" => "JSON",
        "md" => "Markdown",
        "toml" => "TOML",
        "html" => "HTML",
        "css" => "CSS",
        "yml" | "yaml" => "YAML",
        "c" | "h" => "C",
        "cpp" | "hpp" | "cc" => "C++",
        "cs" => "C#",
        "java" => "Java",
        "sh" => "Shell",
        _ => return None,
    })
}

/// Languages and runtimes a project folder uses, judged from its marker files.
fn detect_runtimes(root: &std::path::Path) -> Vec<String> {
    [
        ("Cargo.toml", "Rust"),
        ("package.json", "Node"),
        ("tsconfig.json", "TypeScript"),
        ("pyproject.toml", "Python"),
        ("requirements.txt", "Python"),
        ("go.mod", "Go"),
        ("pom.xml", "Java"),
    ]
    .into_iter()
    .filter(|(file, _)| root.join(file).exists())
    .map(|(_, name)| name.to_string())
    .fold(Vec::new(), |mut found, name| {
        if !found.contains(&name) {
            found.push(name);
        }
        found
    })
}

/// Native tab button with the active theme underline, shared by dock and terminal headers.
fn underline_tab(ui: &mut egui::Ui, label: &str, active: bool, enabled: bool) -> egui::Response {
    underline_tab_padded(ui, label, active, enabled, 30.0)
}

/// A tab label with an underline when active; `pad` is the space added around the text.
fn underline_tab_padded(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    enabled: bool,
    pad: f32,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(14.0),
        theme::text(),
    );
    let color = if active {
        theme::text()
    } else if enabled {
        theme::text_dim()
    } else {
        theme::mix(theme::bg_editor(), theme::text_dim(), 0.55)
    };
    let button = egui::Button::new(egui::RichText::new(label).size(14.0).color(color))
        .frame(false)
        .min_size(egui::vec2(galley.size().x + pad, 36.0));
    let response = ui.add_enabled(enabled, button);
    if active {
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(response.rect.left() + 8.0, response.rect.bottom() - 2.0),
                egui::pos2(response.rect.right() - 8.0, response.rect.bottom()),
            ),
            1.0,
            theme::accent(),
        );
    }
    response
}
