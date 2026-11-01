use crate::{icons, theme, workspace::Workspace};
use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense};
use std::path::{Path, PathBuf};

pub enum Action {
    None,
    OpenFile(PathBuf),
    OpenFolder,
    Refresh,
}

#[derive(Clone, Copy)]
enum Kind {
    Dir { open: bool },
    File,
}

const ROW_HEIGHT: f32 = 22.0;

pub fn show(ui: &mut egui::Ui, workspace: &mut Option<Workspace>, active: Option<&Path>) -> Action {
    let mut action = Action::None;

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(
            RichText::new("EXPLORER")
                .size(11.0)
                .strong()
                .color(theme::TEXT_DIM),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            if workspace.is_some() && ui.small_button("Refresh").clicked() {
                action = Action::Refresh;
            }
        });
    });
    ui.add_space(6.0);

    let Some(workspace) = workspace else {
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(RichText::new("You have not opened a folder.").color(theme::TEXT_DIM));
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
            let title = workspace.name().to_uppercase();
            if row(ui, 0, &title, Kind::Dir { open }, false, true).clicked() {
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
            if row(ui, depth, &entry.name, Kind::Dir { open }, false, false).clicked() {
                workspace.toggle(&entry.path);
            }
            if open {
                tree(ui, workspace, &entry.path, depth + 1, active, action);
            }
        } else {
            let selected = active == Some(entry.path.as_path());
            if row(ui, depth, &entry.name, Kind::File, selected, false).clicked() {
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
    if selected {
        painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(199, 76, 69, 46));
    } else if response.hovered() {
        painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 12));
    }
    let x = rect.left() + 10.0 + depth as f32 * 14.0;
    let y = rect.center().y;
    match kind {
        Kind::Dir { open } => {
            icons::chevron(painter, egui::pos2(x + 4.0, y), open, theme::TEXT_DIM);
            icons::folder(
                painter,
                Rect::from_center_size(egui::pos2(x + 22.0, y), egui::vec2(14.0, 11.0)),
                theme::TEXT_DIM,
            );
        }
        Kind::File => icons::file(
            painter,
            Rect::from_center_size(egui::pos2(x + 22.0, y), egui::vec2(10.0, 13.0)),
            theme::TEXT_DIM,
        ),
    }
    let font = if bold {
        FontId::proportional(12.0)
    } else {
        FontId::proportional(13.0)
    };
    let color = if selected || bold {
        theme::TEXT
    } else {
        Color32::from_rgb(205, 206, 208)
    };
    painter.text(egui::pos2(x + 36.0, y), Align2::LEFT_CENTER, label, font, color);
    response
}
