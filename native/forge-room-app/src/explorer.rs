use crate::{icons, theme, workspace::Workspace};
use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense};
use std::path::{Path, PathBuf};

pub enum Action {
    None,
    OpenFile(PathBuf),
    OpenFolder,
    Refresh,
    NewFile,
    NewFileIn(PathBuf),
    NewFolder(PathBuf),
    Rename(PathBuf),
}

#[derive(Clone, Copy)]
enum Kind {
    Dir { open: bool },
    File,
}

const ROW_HEIGHT: f32 = 24.0;

pub fn show(ui: &mut egui::Ui, workspace: &mut Option<Workspace>, active: Option<&Path>) -> Action {
    let mut action = Action::None;

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(
            RichText::new("Explorer")
                .size(14.5)
                .family(crate::fonts::semibold())
                .color(theme::text()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            if workspace.is_some() && icons::icon_button(ui, "Refresh", icons::refresh).clicked() {
                action = Action::Refresh;
            }
            if icons::icon_button(ui, "New file (Ctrl+N)", icons::new_file).clicked() {
                action = Action::NewFile;
            }
            if workspace.is_some() && ui.small_button("+ Folder").clicked() {
                action = Action::NewFolder(workspace.as_ref().unwrap().root.clone());
            }
        });
    });
    ui.add_space(6.0);

    let Some(workspace) = workspace else {
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(RichText::new("You have not opened a folder.").color(theme::text_dim()));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                if ui.button("Open Folder").clicked() {
                    action = Action::OpenFolder;
                }
            });
        });
        return action;
    };

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            let root = workspace.root.clone();
            let open = workspace.is_expanded(&root);
            let title = workspace.name();
            let response = row(ui, 0, &title, Kind::Dir { open }, false, true);
            response.context_menu(|ui| {
                if ui.button("New file here").clicked() {
                    action = Action::NewFileIn(root.clone());
                    ui.close();
                }
                if ui.button("New folder here").clicked() {
                    action = Action::NewFolder(root.clone());
                    ui.close();
                }
                if ui.button("Copy path").clicked() {
                    ui.ctx().copy_text(root.display().to_string());
                    ui.close();
                }
            });
            if response.clicked() {
                workspace.toggle(&root);
            }
            if open {
                tree(ui, workspace, &root, 1, active, &mut action);
            }
        });
    action
}

fn tree(
    ui: &mut egui::Ui,
    workspace: &mut Workspace,
    dir: &Path,
    depth: usize,
    active: Option<&Path>,
    action: &mut Action,
) {
    for entry in workspace.entries(dir) {
        if entry.is_dir {
            let open = workspace.is_expanded(&entry.path);
            let response = row(ui, depth, &entry.name, Kind::Dir { open }, false, false);
            response.context_menu(|ui| {
                if ui.button("New file here").clicked() {
                    *action = Action::NewFileIn(entry.path.clone());
                    ui.close();
                }
                if ui.button("New folder here").clicked() {
                    *action = Action::NewFolder(entry.path.clone());
                    ui.close();
                }
                if ui.button("Rename…").clicked() {
                    *action = Action::Rename(entry.path.clone());
                    ui.close();
                }
                if ui.button("Copy path").clicked() {
                    ui.ctx().copy_text(entry.path.display().to_string());
                    ui.close();
                }
            });
            if response.clicked() {
                workspace.toggle(&entry.path);
            }
            if open {
                tree(ui, workspace, &entry.path, depth + 1, active, action);
            }
        } else {
            let selected = active == Some(entry.path.as_path());
            let response = row(ui, depth, &entry.name, Kind::File, selected, false);
            response.context_menu(|ui| {
                if ui.button("Rename…").clicked() {
                    *action = Action::Rename(entry.path.clone());
                    ui.close();
                }
                if ui.button("Copy path").clicked() {
                    ui.ctx().copy_text(entry.path.display().to_string());
                    ui.close();
                }
            });
            if response.clicked() {
                *action = Action::OpenFile(entry.path.clone());
            }
        }
    }
}

fn row(
    ui: &mut egui::Ui,
    depth: usize,
    label: &str,
    kind: Kind,
    selected: bool,
    bold: bool,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    let pill = rect.shrink2(egui::vec2(8.0, 1.0));
    if selected {
        painter.rect_filled(pill, 6.0, theme::overlay(26));
    } else if response.hovered() {
        painter.rect_filled(pill, 6.0, theme::overlay(12));
    }
    let x = rect.left() + 10.0 + depth as f32 * 14.0;
    let y = rect.center().y;
    match kind {
        Kind::Dir { open } => {
            icons::chevron(painter, egui::pos2(x + 4.0, y), open, theme::text_dim());
            icons::folder(
                painter,
                Rect::from_center_size(egui::pos2(x + 22.0, y), egui::vec2(14.0, 11.0)),
                theme::text_dim(),
            );
        }
        Kind::File => icons::file(
            painter,
            Rect::from_center_size(egui::pos2(x + 22.0, y), egui::vec2(10.0, 13.0)),
            file_color(label),
        ),
    }
    let font = FontId::proportional(13.0);
    let color = if selected || bold {
        theme::text()
    } else {
        theme::mix(theme::text(), theme::bg_sidebar(), 0.14)
    };
    painter.text(
        egui::pos2(x + 36.0, y),
        Align2::LEFT_CENTER,
        label,
        font,
        color,
    );
    response
}

/// Tint for a file icon by extension, so file types are easy to tell apart at a glance.
pub fn file_color(name: &str) -> Color32 {
    match name.rsplit('.').next().unwrap_or("") {
        "rs" => Color32::from_rgb(222, 150, 110),
        "md" | "txt" => Color32::from_rgb(110, 170, 225),
        "toml" | "yml" | "yaml" | "json" => Color32::from_rgb(205, 190, 90),
        "ps1" | "iss" | "cs" | "bat" => Color32::from_rgb(120, 170, 235),
        "js" | "ts" => Color32::from_rgb(235, 215, 95),
        "py" => Color32::from_rgb(110, 165, 215),
        "png" | "ico" | "svg" | "jpg" => Color32::from_rgb(160, 200, 120),
        _ => theme::text_dim(),
    }
}
