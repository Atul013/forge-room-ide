#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod document;

use document::DocumentSession;
use eframe::egui;
use std::path::PathBuf;

const APP_NAME: &str = "Forge Room Early Preview";
const ACCENT: egui::Color32 = egui::Color32::from_rgb(173, 72, 72);

fn main() -> eframe::Result {
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("forge-room-early-preview")
            .with_title(APP_NAME)
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([720.0, 480.0]),
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

struct ForgeRoomApp {
    document: DocumentSession,
    path_input: String,
    status: String,
}

impl ForgeRoomApp {
    fn new(creation_context: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        configure_style(&creation_context.egui_ctx);
        let mut app = Self {
            document: DocumentSession::default(),
            path_input: initial_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            status: "Ready — enter a path or pass one on the command line".into(),
        };
        if initial_path.is_some() {
            app.open_path();
        }
        app
    }

    fn open_path(&mut self) {
        let path = PathBuf::from(self.path_input.trim());
        match self.document.open(path) {
            Ok(()) => self.status = "Opened file".into(),
            Err(error) => self.status = error.to_string(),
        }
    }

    fn save(&mut self) {
        let requested_path = PathBuf::from(self.path_input.trim());
        let result = match self.document.path() {
            Some(current_path) if current_path == requested_path => self.document.save(),
            _ => self.document.save_as(requested_path),
        };
        match result {
            Ok(()) => self.status = "Saved file".into(),
            Err(error) => self.status = error.to_string(),
        }
    }
}

impl eframe::App for ForgeRoomApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let save_requested = ui.input_mut(|input| {
            input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::S,
            ))
        });
        if save_requested {
            self.save();
        }

        egui::Panel::top("title_bar")
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(28, 29, 31)))
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new(APP_NAME).color(egui::Color32::WHITE));
                    ui.separator();
                    ui.label(
                        egui::RichText::new(forge_room_core::NATIVE_PREVIEW_VERSION)
                            .color(egui::Color32::GRAY),
                    );
                    if self.document.is_dirty() {
                        ui.label(egui::RichText::new("Unsaved").color(ACCENT));
                    }
                });
                ui.add_space(8.0);
            });

        egui::Panel::bottom("status_bar")
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(28, 29, 31)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("Ctrl/Cmd+S to save");
                    });
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                let path_label = ui.label("File path");
                ui.add(
                    egui::TextEdit::singleline(&mut self.path_input)
                        .desired_width(f32::INFINITY)
                        .hint_text("Absolute or working-directory-relative path"),
                )
                .labelled_by(path_label.id);
                if ui.button("Open").clicked() {
                    self.open_path();
                }
                if ui
                    .add_enabled(
                        !self.path_input.trim().is_empty(),
                        egui::Button::new("Save"),
                    )
                    .clicked()
                {
                    self.save();
                }
            });
            ui.separator();
            ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(self.document.text_mut())
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .lock_focus(true)
                    .hint_text("Open a UTF-8 text file to begin editing"),
            );
        });
    }
}

fn configure_style(context: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(35, 36, 39);
    visuals.window_fill = egui::Color32::from_rgb(35, 36, 39);
    visuals.selection.bg_fill = ACCENT;
    visuals.hyperlink_color = egui::Color32::from_rgb(218, 126, 126);
    visuals.widgets.active.bg_fill = ACCENT;
    context.set_visuals(visuals);
}
