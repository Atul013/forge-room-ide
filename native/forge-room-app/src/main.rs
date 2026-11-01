#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod document;
mod explorer;
mod icons;
mod position;
mod tabs;
mod theme;
mod workspace;

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use std::{fs, path::PathBuf};
use tabs::{Tabs, normalize};
use workspace::Workspace;

const APP_NAME: &str = "Forge Room Early Preview";
const TAB_HEIGHT: f32 = 34.0;
const ACTIVITY_WIDTH: f32 = 48.0;

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
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([760.0, 480.0]),
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
}

fn shortcut(ui: &egui::Ui, modifiers: Modifiers, key: Key) -> bool {
    ui.input_mut(|input| input.consume_shortcut(&KeyboardShortcut::new(modifiers, key)))
}

impl ForgeRoomApp {
    fn new(creation_context: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        theme::configure(&creation_context.egui_ctx);
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

    fn handle_shortcuts(&mut self, ui: &egui::Ui) {
        let command = Modifiers::COMMAND;
        let command_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        // Most specific first, so Ctrl+Shift+S is not taken by Ctrl+S.
        if shortcut(ui, command_shift, Key::S) {
            self.save_active(true);
        } else if shortcut(ui, command_shift, Key::O) {
            self.open_folder_dialog();
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

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
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
            ui.menu_button("View", |ui| {
                if ui
                    .add(egui::Button::new("Toggle Sidebar").shortcut_text("Ctrl+B"))
                    .clicked()
                {
                    self.show_sidebar = !self.show_sidebar;
                    ui.close();
                }
            });
        });
    }

    fn activity_bar(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ACTIVITY_WIDTH, ACTIVITY_WIDTH),
            egui::Sense::click(),
        );
        let active = self.show_sidebar;
        let color = if active || response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        let painter = ui.painter();
        if active {
            painter.rect_filled(
                egui::Rect::from_min_size(rect.min, egui::vec2(2.0, rect.height())),
                0.0,
                theme::ACCENT,
            );
        }
        icons::explorer(
            painter,
            egui::Rect::from_center_size(rect.center(), egui::vec2(22.0, 24.0)),
            color,
        );
        if response.on_hover_text("Explorer (Ctrl+B)").clicked() {
            self.show_sidebar = !self.show_sidebar;
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let active = self.tabs.active_path().map(|path| path.to_path_buf());
        match explorer::show(ui, &mut self.workspace, active.as_deref()) {
            explorer::Action::None => {}
            explorer::Action::OpenFile(path) => self.open_path(path),
            explorer::Action::OpenFolder => self.open_folder_dialog(),
            explorer::Action::Refresh => {
                if let Some(workspace) = self.workspace.as_mut() {
                    workspace.refresh();
                }
            }
        }
    }

    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        let bar = ui.available_rect_before_wrap();
        ui.painter().rect_filled(
            egui::Rect::from_min_size(bar.min, egui::vec2(bar.width(), TAB_HEIGHT)),
            0.0,
            theme::BG_TAB_IDLE,
        );
        let mut activate = None;
        let mut close = None;

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
                            egui::FontId::proportional(13.0),
                            theme::TEXT,
                        );
                        let width = (galley.size().x + 52.0).max(104.0);
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
                        let fill = if is_active {
                            theme::BG_EDITOR
                        } else if hovered {
                            egui::Color32::from_rgb(28, 29, 31)
                        } else {
                            theme::BG_TAB_IDLE
                        };
                        painter.rect_filled(rect, 0.0, fill);
                        if is_active {
                            painter.rect_filled(
                                egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 2.0)),
                                0.0,
                                theme::ACCENT,
                            );
                        }
                        painter.line_segment(
                            [rect.right_top(), rect.right_bottom()],
                            egui::Stroke::new(1.0, theme::BORDER),
                        );
                        let text_color = if is_active {
                            theme::TEXT
                        } else {
                            theme::TEXT_DIM
                        };
                        painter.galley(
                            egui::pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0),
                            galley,
                            text_color,
                        );
                        if tab.doc.is_dirty() && !hovered {
                            painter.circle_filled(close_rect.center(), 4.0, theme::TEXT_DIM);
                        } else if is_active || hovered {
                            if close_response.hovered() {
                                painter.circle_filled(
                                    close_rect.center(),
                                    9.0,
                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 22),
                                );
                            }
                            icons::close_cross(painter, close_rect.center(), theme::TEXT);
                        }

                        if close_response.clicked() || response.middle_clicked() {
                            close = Some(index);
                        } else if response.clicked() {
                            activate = Some(index);
                        }
                    }
                });
            });

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
        let line_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let mut cursor_index = None;
        let Some(tab) = self.tabs.active_tab() else {
            self.welcome(ui);
            return;
        };
        let id = egui::Id::unique(("editor", tab.id));
        egui::ScrollArea::both()
            .id_salt(("scroll", tab.id))
            .auto_shrink(false)
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        let rows = (ui.available_height() / line_height).floor() as usize;
                        let output = egui::TextEdit::multiline(tab.doc.text_mut())
                            .id(id)
                            .code_editor()
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .desired_rows(rows.max(10))
                            .lock_focus(true)
                            .show(ui);
                        if focus {
                            output.response.request_focus();
                        }
                        cursor_index = output.cursor_range.map(|range| range.primary.index.0);
                    });
            });
        if let (Some(index), Some(tab)) = (cursor_index, self.tabs.tabs.get(self.tabs.active)) {
            self.cursor = position::line_col(tab.doc.text(), index);
        }
    }

    fn welcome(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.26).max(24.0));
            ui.label(
                egui::RichText::new("Forge Room")
                    .size(30.0)
                    .color(theme::TEXT),
            );
            ui.label(egui::RichText::new("Early Preview").color(theme::TEXT_DIM));
            ui.add_space(24.0);
            let size = egui::vec2(240.0, 32.0);
            let entries: [(&str, &str); 3] = [
                ("New File", "Ctrl+N"),
                ("Open File…", "Ctrl+O"),
                ("Open Folder…", "Ctrl+Shift+O"),
            ];
            for (index, (label, keys)) in entries.iter().enumerate() {
                if ui
                    .add(
                        egui::Button::new(*label)
                            .shortcut_text(*keys)
                            .min_size(size),
                    )
                    .clicked()
                {
                    match index {
                        0 => self.new_file(),
                        1 => self.open_file_dialog(),
                        _ => self.open_folder_dialog(),
                    }
                }
            }
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&self.status).color(theme::TEXT_DIM));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dim = |text: String| egui::RichText::new(text).color(theme::TEXT_DIM);
                ui.label(dim(forge_room_core::NATIVE_PREVIEW_VERSION.to_string()));
                if let Some(tab) = self.tabs.tabs.get(self.tabs.active) {
                    ui.label(dim(position::line_ending(tab.doc.text()).to_string()));
                    ui.label(dim("UTF-8".to_string()));
                    ui.label(dim(format!("Ln {}, Col {}", self.cursor.0, self.cursor.1)));
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
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

        let bar = |fill| {
            egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(8, 3))
        };

        egui::Panel::top("menu")
            .frame(bar(theme::BG_BAR))
            .show(ui, |ui| self.menu_bar(ui));

        egui::Panel::bottom("status")
            .frame(bar(theme::BG_BAR))
            .show(ui, |ui| self.status_bar(ui));

        egui::Panel::left("activity")
            .exact_size(ACTIVITY_WIDTH)
            .resizable(false)
            .frame(egui::Frame::new().fill(theme::BG_ACTIVITY))
            .show(ui, |ui| self.activity_bar(ui));

        if self.show_sidebar {
            egui::Panel::left("sidebar")
                .default_size(270.0)
                .size_range(160.0..=520.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(theme::BG_SIDEBAR))
                .show(ui, |ui| self.sidebar(ui));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG_EDITOR))
            .show(ui, |ui| {
                if !self.tabs.tabs.is_empty() {
                    self.tab_bar(ui);
                }
                self.editor(ui);
            });

        self.pending_dialog(&context);
        self.update_title(&context);
    }
}
