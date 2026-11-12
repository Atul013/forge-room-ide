use crate::{icons, theme, workspace::Workspace};
use eframe::egui::{self, Align2, Color32, FontId, Key, Modifiers, Rect, RichText, Sense};
use std::path::{Path, PathBuf};

/// Where the keyboard is in the file tree.
#[derive(Default)]
pub struct State {
    /// The row the arrow keys moved to (or that was clicked last).
    selected: Option<PathBuf>,
    /// The tree has the keyboard: it was clicked, or Ctrl+Shift+E brought it up.
    focused: bool,
    /// The selection moved by key: scroll it into view once.
    scroll_to_selected: bool,
}

impl State {
    /// Give the tree the keyboard (a selection is made when there is none).
    pub fn focus(&mut self) {
        self.focused = true;
    }

    /// Take the keyboard away from the tree.
    pub fn unfocus(&mut self) {
        self.focused = false;
    }
}

/// A key that moves through the tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Nav {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
}

/// What a key does to a folder row.
#[derive(Debug, Eq, PartialEq)]
enum Toggle {
    Nothing,
    Expand,
    Collapse,
}

/// Where `nav` takes the selection among the visible `rows` (path, is a folder), and whether it
/// opens or closes a folder on the way: Right opens a closed folder and then steps into it, Left
/// closes an open folder and then steps out to the parent, like the file tree of any editor.
fn navigate(
    rows: &[(PathBuf, bool)],
    expanded: impl Fn(&Path) -> bool,
    selected: Option<&Path>,
    nav: Nav,
) -> (Option<PathBuf>, Toggle) {
    if rows.is_empty() {
        return (None, Toggle::Nothing);
    }
    let at = selected.and_then(|path| rows.iter().position(|(row, _)| row == path));
    let Some(at) = at else {
        // Nothing selected yet: the first row (or the last one for End and Up).
        let index = if matches!(nav, Nav::End | Nav::Up) {
            rows.len() - 1
        } else {
            0
        };
        return (Some(rows[index].0.clone()), Toggle::Nothing);
    };
    let (path, is_dir) = (&rows[at].0, rows[at].1);
    match nav {
        Nav::Up => (Some(rows[at.saturating_sub(1)].0.clone()), Toggle::Nothing),
        Nav::Down => (
            Some(rows[(at + 1).min(rows.len() - 1)].0.clone()),
            Toggle::Nothing,
        ),
        Nav::Home => (Some(rows[0].0.clone()), Toggle::Nothing),
        Nav::End => (Some(rows[rows.len() - 1].0.clone()), Toggle::Nothing),
        Nav::Right if is_dir && !expanded(path) => (Some(path.clone()), Toggle::Expand),
        Nav::Right if is_dir && at + 1 < rows.len() && rows[at + 1].0.starts_with(path) => {
            (Some(rows[at + 1].0.clone()), Toggle::Nothing)
        }
        Nav::Right => (Some(path.clone()), Toggle::Nothing),
        Nav::Left if is_dir && expanded(path) && at > 0 => (Some(path.clone()), Toggle::Collapse),
        Nav::Left => {
            let parent = path.parent().and_then(|parent| {
                rows.iter()
                    .find(|(row, dir)| *dir && row == parent)
                    .map(|(row, _)| row.clone())
            });
            (
                Some(parent.unwrap_or_else(|| path.clone())),
                Toggle::Nothing,
            )
        }
    }
}

/// The rows the tree shows now, top to bottom: the folder itself, then what is inside the
/// folders that are open.
fn visible_rows(workspace: &mut Workspace) -> Vec<(PathBuf, bool)> {
    fn add(workspace: &mut Workspace, dir: &Path, rows: &mut Vec<(PathBuf, bool)>) {
        for entry in workspace.entries(dir) {
            rows.push((entry.path.clone(), entry.is_dir));
            if entry.is_dir && workspace.is_expanded(&entry.path) {
                add(workspace, &entry.path, rows);
            }
        }
    }
    let root = workspace.root.clone();
    let mut rows = vec![(root.clone(), true)];
    if workspace.is_expanded(&root) {
        add(workspace, &root, &mut rows);
    }
    rows
}

pub enum Action {
    None,
    OpenFile(PathBuf),
    OpenFolder,
    Refresh,
    NewFile,
    NewFileIn(PathBuf),
    NewFolder(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
    CopyRelative(PathBuf),
    Reveal(PathBuf),
}

#[derive(Clone, Copy)]
enum Kind {
    Dir { open: bool },
    File,
}

const ROW_HEIGHT: f32 = 24.0;

pub fn show(
    ui: &mut egui::Ui,
    workspace: &mut Option<Workspace>,
    active: Option<&Path>,
    state: &mut State,
    keys_enabled: bool,
) -> Action {
    let mut action = Action::None;
    // A click anywhere else takes the keyboard away from the tree.
    let area = ui.max_rect();
    if ui.input(|input| input.pointer.any_pressed())
        && let Some(position) = ui.input(|input| input.pointer.interact_pos())
        && !area.contains(position)
    {
        state.focused = false;
    }

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

    if state.focused && keys_enabled {
        keyboard(ui, workspace, state, &mut action);
    }
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            let root = workspace.root.clone();
            let open = workspace.is_expanded(&root);
            let title = workspace.name();
            let marked = Marked::of(state, &root);
            let response = row(ui, 0, &title, Kind::Dir { open }, marked, true);
            if response.clicked() {
                state.selected = Some(root.clone());
                state.focused = true;
            }
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
                tree(ui, workspace, &root, 1, active, state, &mut action);
            }
        });
    state.scroll_to_selected = false;
    action
}

/// How a row is marked: where the keyboard is (and whether the tree has it), and whether to scroll
/// to it.
#[derive(Clone, Copy, Default)]
struct Marked {
    selected: bool,
    focused: bool,
    scroll: bool,
}

impl Marked {
    fn of(state: &State, path: &Path) -> Self {
        let selected = state.selected.as_deref() == Some(path);
        Self {
            selected,
            focused: selected && state.focused,
            scroll: selected && state.scroll_to_selected,
        }
    }
}

/// Arrow keys, Home, End, Enter, F2 and Delete in the tree.
fn keyboard(ui: &mut egui::Ui, workspace: &mut Workspace, state: &mut State, action: &mut Action) {
    // Only when a key that matters is pressed: the rows are not rebuilt every frame.
    let relevant = ui.input(|input| {
        [
            Key::ArrowUp,
            Key::ArrowDown,
            Key::ArrowLeft,
            Key::ArrowRight,
            Key::Home,
            Key::End,
            Key::Enter,
            Key::F2,
            Key::Delete,
            Key::Escape,
        ]
        .iter()
        .any(|key| input.key_pressed(*key))
    });
    if !relevant {
        return;
    }
    if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        state.focused = false;
        return;
    }
    let rows = visible_rows(workspace);
    if state
        .selected
        .as_ref()
        .is_some_and(|path| !rows.iter().any(|(row, _)| row == path))
    {
        state.selected = None;
    }
    for (key, nav) in [
        (Key::ArrowUp, Nav::Up),
        (Key::ArrowDown, Nav::Down),
        (Key::ArrowLeft, Nav::Left),
        (Key::ArrowRight, Nav::Right),
        (Key::Home, Nav::Home),
        (Key::End, Nav::End),
    ] {
        if ui.input_mut(|input| input.consume_key(Modifiers::NONE, key)) {
            let (next, toggle) = navigate(
                &rows,
                |path| workspace.is_expanded(path),
                state.selected.as_deref(),
                nav,
            );
            if let Some(path) = &next {
                match toggle {
                    Toggle::Expand if !workspace.is_expanded(path) => workspace.toggle(path),
                    Toggle::Collapse if workspace.is_expanded(path) => workspace.toggle(path),
                    _ => {}
                }
            }
            state.selected = next;
            state.scroll_to_selected = true;
            return;
        }
    }
    let Some(selected) = state.selected.clone() else {
        return;
    };
    let is_dir = rows.iter().any(|(path, dir)| *path == selected && *dir);
    let is_root = selected == workspace.root;
    if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter)) {
        if is_dir {
            workspace.toggle(&selected);
        } else {
            *action = Action::OpenFile(selected.clone());
        }
    } else if !is_root && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F2)) {
        *action = Action::Rename(selected.clone());
    } else if !is_root && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Delete)) {
        *action = Action::Delete(selected);
    }
}

fn tree(
    ui: &mut egui::Ui,
    workspace: &mut Workspace,
    dir: &Path,
    depth: usize,
    active: Option<&Path>,
    state: &mut State,
    action: &mut Action,
) {
    for entry in workspace.entries(dir) {
        if entry.is_dir {
            let open = workspace.is_expanded(&entry.path);
            let marked = Marked::of(state, &entry.path);
            let response = row(ui, depth, &entry.name, Kind::Dir { open }, marked, false);
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
                if ui.button("Delete…").clicked() {
                    *action = Action::Delete(entry.path.clone());
                    ui.close();
                }
                if ui.button("Copy relative path").clicked() {
                    *action = Action::CopyRelative(entry.path.clone());
                    ui.close();
                }
                if ui.button("Reveal in File Explorer").clicked() {
                    *action = Action::Reveal(entry.path.clone());
                    ui.close();
                }
                if ui.button("Copy path").clicked() {
                    ui.ctx().copy_text(entry.path.display().to_string());
                    ui.close();
                }
            });
            if response.clicked() {
                workspace.toggle(&entry.path);
                state.selected = Some(entry.path.clone());
                state.focused = true;
            }
            if open {
                tree(ui, workspace, &entry.path, depth + 1, active, state, action);
            }
        } else {
            let mut marked = Marked::of(state, &entry.path);
            // The open file is marked even when the keyboard is elsewhere.
            marked.selected |= active == Some(entry.path.as_path());
            let response = row(ui, depth, &entry.name, Kind::File, marked, false);
            response.context_menu(|ui| {
                if ui.button("Rename…").clicked() {
                    *action = Action::Rename(entry.path.clone());
                    ui.close();
                }
                if ui.button("Delete…").clicked() {
                    *action = Action::Delete(entry.path.clone());
                    ui.close();
                }
                if ui.button("Copy relative path").clicked() {
                    *action = Action::CopyRelative(entry.path.clone());
                    ui.close();
                }
                if ui.button("Reveal in File Explorer").clicked() {
                    *action = Action::Reveal(entry.path.clone());
                    ui.close();
                }
                if ui.button("Copy path").clicked() {
                    ui.ctx().copy_text(entry.path.display().to_string());
                    ui.close();
                }
            });
            if response.clicked() {
                *action = Action::OpenFile(entry.path.clone());
                state.selected = Some(entry.path.clone());
                state.focused = true;
            }
        }
    }
}

fn row(
    ui: &mut egui::Ui,
    depth: usize,
    label: &str,
    kind: Kind,
    marked: Marked,
    bold: bool,
) -> egui::Response {
    // Not focusable: the tree keeps its own idea of where the keyboard is, and egui must not move
    // its focus around the rows with the arrow keys.
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), Sense::CLICK);
    if marked.scroll {
        ui.scroll_to_rect(rect, None);
    }
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let selected = marked.selected;
    let painter = ui.painter();
    let pill = rect.shrink2(egui::vec2(8.0, 1.0));
    if selected {
        painter.rect_filled(pill, 6.0, theme::overlay(26));
        if marked.focused {
            painter.rect_stroke(
                pill,
                6.0,
                egui::Stroke::new(1.0, theme::accent()),
                egui::StrokeKind::Inside,
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<(PathBuf, bool)> {
        ["/p", "/p/a", "/p/a/x.rs", "/p/b.txt"]
            .iter()
            .zip([true, true, false, false])
            .map(|(path, dir)| (PathBuf::from(path), dir))
            .collect()
    }

    fn go(selected: Option<&str>, nav: Nav, open: &[&str]) -> (Option<String>, Toggle) {
        let rows = rows();
        let open: Vec<PathBuf> = open.iter().map(PathBuf::from).collect();
        let (next, toggle) = navigate(
            &rows,
            |path| open.iter().any(|known| known == path),
            selected.map(Path::new),
            nav,
        );
        (
            next.map(|path| path.display().to_string().replace('\\', "/")),
            toggle,
        )
    }

    #[test]
    fn arrows_walk_the_visible_rows_and_stop_at_the_ends() {
        let open = ["/p", "/p/a"];
        assert_eq!(
            go(Some("/p/a"), Nav::Down, &open).0.as_deref(),
            Some("/p/a/x.rs")
        );
        assert_eq!(go(Some("/p/a"), Nav::Up, &open).0.as_deref(), Some("/p"));
        assert_eq!(go(Some("/p"), Nav::Up, &open).0.as_deref(), Some("/p"));
        assert_eq!(
            go(Some("/p/b.txt"), Nav::Down, &open).0.as_deref(),
            Some("/p/b.txt")
        );
        assert_eq!(
            go(Some("/p/a"), Nav::End, &open).0.as_deref(),
            Some("/p/b.txt")
        );
        assert_eq!(
            go(Some("/p/b.txt"), Nav::Home, &open).0.as_deref(),
            Some("/p")
        );
        // With nothing selected the first key picks the first (or, for Up and End, the last) row.
        assert_eq!(go(None, Nav::Down, &open).0.as_deref(), Some("/p"));
        assert_eq!(go(None, Nav::Up, &open).0.as_deref(), Some("/p/b.txt"));
    }

    #[test]
    fn right_opens_then_enters_a_folder_and_left_closes_then_leaves_it() {
        // A closed folder opens first; an open one is entered.
        assert_eq!(
            go(Some("/p/a"), Nav::Right, &["/p"]),
            (Some("/p/a".into()), Toggle::Expand)
        );
        assert_eq!(
            go(Some("/p/a"), Nav::Right, &["/p", "/p/a"]),
            (Some("/p/a/x.rs".into()), Toggle::Nothing)
        );
        // On a file Right does nothing; Left goes to the folder around it.
        assert_eq!(
            go(Some("/p/a/x.rs"), Nav::Right, &["/p", "/p/a"]).1,
            Toggle::Nothing
        );
        assert_eq!(
            go(Some("/p/a/x.rs"), Nav::Left, &["/p", "/p/a"]),
            (Some("/p/a".into()), Toggle::Nothing)
        );
        // An open folder closes first, then Left climbs to its parent.
        assert_eq!(
            go(Some("/p/a"), Nav::Left, &["/p", "/p/a"]),
            (Some("/p/a".into()), Toggle::Collapse)
        );
        assert_eq!(
            go(Some("/p/a"), Nav::Left, &["/p"]),
            (Some("/p".into()), Toggle::Nothing)
        );
        // The top folder has no parent row, so it stays put.
        assert_eq!(
            go(Some("/p"), Nav::Left, &["/p"]),
            (Some("/p".into()), Toggle::Nothing)
        );
    }
}
