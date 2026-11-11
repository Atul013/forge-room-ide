//! Command palette and Quick Open, one box like VS Code: plain text searches files, `>` runs
//! commands, `:` jumps to a line.

use crate::commands::{self, Command};
use crate::{fuzzy, git, search, theme};
use eframe::egui::{self, Align2, Key, Modifiers, RichText, Sense};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Palette {
    pub open: bool,
    query: String,
    selected: usize,
    files: Vec<String>,
    root: Option<PathBuf>,
    focus: bool,
    /// When set, the box lists branches instead of files and commands.
    branches: Option<Vec<git::Branch>>,
    purpose: BranchPurpose,
    /// The outline of the open file, for the `@` mode (Go to Symbol in File).
    symbols: Vec<crate::symbols::Symbol>,
}

/// What picking a branch in the palette does.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BranchPurpose {
    #[default]
    Checkout,
    Merge,
    Delete,
    /// Name a new tag on the current commit (the list shows the existing tags).
    TagCreate,
    TagDelete,
}

pub enum Outcome {
    Checkout { name: String, remote: bool },
    CreateBranch(String),
    Merge(String),
    DeleteBranch(String),
    CreateTag(String),
    DeleteTag(String),
    Run(Command),
    Open(PathBuf),
    GoToLine(usize),
}

impl Palette {
    pub fn open_commands(&mut self) {
        self.symbols.clear();
        self.query = ">".into();
        self.selected = 0;
        self.open = true;
        self.focus = true;
    }

    pub fn open_files(&mut self, root: Option<&Path>) {
        self.symbols.clear();
        self.query.clear();
        self.selected = 0;
        self.open = true;
        self.focus = true;
        if root != self.root.as_deref() || self.files.is_empty() {
            self.files = root
                .map(|root| {
                    search::list_files(root)
                        .iter()
                        .filter_map(|path| path.strip_prefix(root).ok())
                        .map(|path| path.to_string_lossy().replace('\\', "/"))
                        .collect()
                })
                .unwrap_or_default();
            self.root = root.map(Path::to_path_buf);
        }
    }

    pub fn open_branches(&mut self, branches: Vec<git::Branch>, purpose: BranchPurpose) {
        // Only local branches can be deleted; the current one can be neither merged nor deleted.
        let branches = match purpose {
            BranchPurpose::Delete => branches.into_iter().filter(|b| !b.remote).collect(),
            _ => branches,
        };
        self.purpose = purpose;
        self.query.clear();
        self.selected = 0;
        self.open = true;
        self.focus = true;
        self.branches = Some(branches);
    }

    /// Open the box in its symbol mode with the outline of the open file.
    pub fn open_symbols(&mut self, symbols: Vec<crate::symbols::Symbol>) {
        self.symbols = symbols;
        self.query = "@".into();
        self.selected = 0;
        self.open = true;
        self.focus = true;
    }

    pub fn open_goto(&mut self) {
        self.symbols.clear();
        self.query = ":".into();
        self.selected = 0;
        self.open = true;
        self.focus = true;
    }
}

/// What the query currently means.
#[derive(Debug, Eq, PartialEq)]
enum Kind<'a> {
    Commands(&'a str),
    Files(&'a str),
    Line(&'a str),
    Symbols(&'a str),
}

fn kind(query: &str) -> Kind<'_> {
    if let Some(rest) = query.strip_prefix('>') {
        Kind::Commands(rest.trim_start())
    } else if let Some(rest) = query.strip_prefix(':') {
        Kind::Line(rest.trim())
    } else if let Some(rest) = query.strip_prefix('@') {
        Kind::Symbols(rest.trim_start())
    } else {
        Kind::Files(query)
    }
}

pub fn show(context: &egui::Context, palette: &mut Palette) -> Option<Outcome> {
    if !palette.open {
        return None;
    }
    let mut outcome = None;
    let mut close = false;

    // Rows for the current query.
    enum Row {
        NewBranch(String),
        Branch(usize),
        Command(Command, &'static str, &'static str),
        File(usize),
        Line(usize),
        Symbol(usize),
        Hint(&'static str),
    }
    let rows: Vec<Row> = if let Some(branches) = &palette.branches {
        let names: Vec<&str> = branches.iter().map(|branch| branch.name.as_str()).collect();
        let query = palette.query.trim();
        let mut rows = Vec::new();
        if matches!(
            palette.purpose,
            BranchPurpose::Checkout | BranchPurpose::TagCreate
        ) && !query.is_empty()
            && !names.contains(&query)
        {
            rows.push(Row::NewBranch(query.to_string()));
        }
        rows.extend(fuzzy::rank(query, &names, 14).into_iter().map(Row::Branch));
        rows
    } else {
        match kind(&palette.query) {
            Kind::Commands(text) => {
                let titles: Vec<&str> = commands::ALL.iter().map(|(_, title, _)| *title).collect();
                fuzzy::rank(text, &titles, 14)
                    .into_iter()
                    .map(|index| {
                        let (command, title, keys) = commands::ALL[index];
                        Row::Command(command, title, keys)
                    })
                    .collect()
            }
            Kind::Files(text) => {
                if palette.files.is_empty() {
                    vec![Row::Hint("Open a folder to search its files")]
                } else {
                    fuzzy::rank(text, &palette.files, 14)
                        .into_iter()
                        .map(Row::File)
                        .collect()
                }
            }
            Kind::Symbols(text) => {
                if palette.symbols.is_empty() {
                    vec![Row::Hint("No symbols found in this file")]
                } else {
                    let names: Vec<&str> = palette
                        .symbols
                        .iter()
                        .map(|symbol| symbol.name.as_str())
                        .collect();
                    fuzzy::rank(text, &names, 300)
                        .into_iter()
                        .map(Row::Symbol)
                        .collect()
                }
            }
            Kind::Line(text) => match text.parse::<usize>() {
                Ok(line) if line > 0 => vec![Row::Line(line)],
                _ => vec![Row::Hint("Type a line number to jump to")],
            },
        }
    };
    let count = rows.len();
    palette.selected = palette.selected.min(count.saturating_sub(1));

    let moved_down = context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowDown));
    let moved_up = context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowUp));
    if moved_down && count > 0 {
        palette.selected = (palette.selected + 1) % count;
    }
    if moved_up && count > 0 {
        palette.selected = (palette.selected + count - 1) % count;
    }
    let enter = context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
    if context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        close = true;
    }

    let mut clicked: Option<usize> = None;
    egui::Area::new(egui::Id::unique("palette"))
        .anchor(Align2::CENTER_TOP, [0.0, 70.0])
        .order(egui::Order::Foreground)
        .show(context, |ui| {
            egui::Frame::new()
                .fill(theme::bg_sidebar())
                .stroke(egui::Stroke::new(1.0, theme::border()))
                .corner_radius(10.0)
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 28,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(110),
                })
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(560.0);
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut palette.query)
                            .hint_text("Go to file…   type > for commands, : for a line")
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin::symmetric(10, 8)),
                    );
                    if palette.focus {
                        input.request_focus();
                        palette.focus = false;
                    }
                    if input.changed() {
                        palette.selected = 0;
                    }
                    ui.add_space(6.0);
                    for (index, row) in rows.iter().enumerate() {
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 28.0),
                            Sense::click(),
                        );
                        let selected = index == palette.selected;
                        if selected {
                            ui.painter().rect_filled(rect, 6.0, theme::list_active());
                        } else if response.hovered() {
                            ui.painter().rect_filled(rect, 6.0, theme::overlay(10));
                        }
                        let y = rect.center().y;
                        let painter = ui.painter();
                        match row {
                            Row::NewBranch(name) => {
                                painter.text(
                                    egui::pos2(rect.left() + 10.0, y),
                                    Align2::LEFT_CENTER,
                                    if palette.purpose == BranchPurpose::TagCreate {
                                        format!("Create tag \"{name}\" on the current commit")
                                    } else {
                                        format!("Create new branch \"{name}\"")
                                    },
                                    egui::FontId::proportional(13.5),
                                    theme::accent(),
                                );
                            }
                            Row::Branch(index) => {
                                let branch =
                                    palette.branches.as_ref().and_then(|list| list.get(*index));
                                if let Some(branch) = branch {
                                    painter.text(
                                        egui::pos2(rect.left() + 10.0, y),
                                        Align2::LEFT_CENTER,
                                        &branch.name,
                                        egui::FontId::proportional(13.5),
                                        theme::text(),
                                    );
                                    let tag = if branch.current {
                                        "current branch"
                                    } else if branch.remote {
                                        "remote"
                                    } else {
                                        ""
                                    };
                                    painter.text(
                                        egui::pos2(rect.right() - 10.0, y),
                                        Align2::RIGHT_CENTER,
                                        tag,
                                        egui::FontId::proportional(12.0),
                                        theme::text_dim(),
                                    );
                                }
                            }
                            Row::Command(_, title, keys) => {
                                painter.text(
                                    egui::pos2(rect.left() + 10.0, y),
                                    Align2::LEFT_CENTER,
                                    title,
                                    egui::FontId::proportional(13.5),
                                    theme::text(),
                                );
                                painter.text(
                                    egui::pos2(rect.right() - 10.0, y),
                                    Align2::RIGHT_CENTER,
                                    keys,
                                    egui::FontId::proportional(12.0),
                                    theme::text_dim(),
                                );
                            }
                            Row::File(file_index) => {
                                let path = &palette.files[*file_index];
                                let (directory, name) = match path.rsplit_once('/') {
                                    Some((directory, name)) => (directory, name),
                                    None => ("", path.as_str()),
                                };
                                let galley = painter.layout_no_wrap(
                                    name.to_string(),
                                    egui::FontId::proportional(13.5),
                                    theme::text(),
                                );
                                let width = galley.size().x;
                                painter.galley(
                                    egui::pos2(rect.left() + 10.0, y - galley.size().y / 2.0),
                                    galley,
                                    theme::text(),
                                );
                                painter.text(
                                    egui::pos2(rect.left() + 18.0 + width, y),
                                    Align2::LEFT_CENTER,
                                    directory,
                                    egui::FontId::proportional(12.0),
                                    theme::text_dim(),
                                );
                            }
                            Row::Symbol(index) => {
                                if let Some(symbol) = palette.symbols.get(*index) {
                                    painter.text(
                                        egui::pos2(rect.left() + 10.0, y),
                                        Align2::LEFT_CENTER,
                                        &symbol.name,
                                        egui::FontId::proportional(13.5),
                                        theme::text(),
                                    );
                                    painter.text(
                                        egui::pos2(rect.right() - 10.0, y),
                                        Align2::RIGHT_CENTER,
                                        format!("{}  :{}", symbol.kind, symbol.line),
                                        egui::FontId::proportional(12.0),
                                        theme::text_dim(),
                                    );
                                }
                            }
                            Row::Line(line) => {
                                painter.text(
                                    egui::pos2(rect.left() + 10.0, y),
                                    Align2::LEFT_CENTER,
                                    format!("Go to line {line}"),
                                    egui::FontId::proportional(13.5),
                                    theme::text(),
                                );
                            }
                            Row::Hint(text) => {
                                painter.text(
                                    egui::pos2(rect.left() + 10.0, y),
                                    Align2::LEFT_CENTER,
                                    text,
                                    egui::FontId::proportional(13.0),
                                    theme::text_dim(),
                                );
                            }
                        }
                        if response.clicked() {
                            clicked = Some(index);
                        }
                    }
                    if rows.is_empty() {
                        ui.label(RichText::new("No matches").color(theme::text_dim()));
                    }
                });
        });

    if let Some(index) = clicked.or(enter.then_some(palette.selected)) {
        match rows.get(index) {
            Some(Row::NewBranch(name)) => {
                outcome = Some(if palette.purpose == BranchPurpose::TagCreate {
                    Outcome::CreateTag(name.clone())
                } else {
                    Outcome::CreateBranch(name.clone())
                })
            }
            Some(Row::Branch(branch_index)) => {
                if let Some(branch) = palette
                    .branches
                    .as_ref()
                    .and_then(|list| list.get(*branch_index))
                    && !branch.current
                {
                    outcome = match palette.purpose {
                        BranchPurpose::Checkout => Some(Outcome::Checkout {
                            name: branch.name.clone(),
                            remote: branch.remote,
                        }),
                        BranchPurpose::Merge => Some(Outcome::Merge(branch.name.clone())),
                        BranchPurpose::Delete => Some(Outcome::DeleteBranch(branch.name.clone())),
                        BranchPurpose::TagDelete => Some(Outcome::DeleteTag(branch.name.clone())),
                        // The list only shows which tags exist; naming a new one is the first row.
                        BranchPurpose::TagCreate => None,
                    };
                }
            }
            Some(Row::Command(command, _, _)) => outcome = Some(Outcome::Run(*command)),
            Some(Row::File(file_index)) => {
                if let Some(root) = &palette.root {
                    outcome = Some(Outcome::Open(root.join(&palette.files[*file_index])));
                }
            }
            Some(Row::Line(line)) => outcome = Some(Outcome::GoToLine(*line)),
            Some(Row::Symbol(index)) => {
                if let Some(symbol) = palette.symbols.get(*index) {
                    outcome = Some(Outcome::GoToLine(symbol.line));
                }
            }
            _ => {}
        }
        if outcome.is_some() {
            close = true;
        }
    }
    if close {
        palette.open = false;
        palette.branches = None;
        // The outline belongs to the file it was made for: never offer it for another one.
        palette.symbols.clear();
        palette.purpose = BranchPurpose::Checkout;
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_pick_the_mode() {
        assert_eq!(kind("main"), Kind::Files("main"));
        assert_eq!(kind(">save"), Kind::Commands("save"));
        assert_eq!(kind("> save"), Kind::Commands("save"));
        assert_eq!(kind(":42"), Kind::Line("42"));
        assert_eq!(kind(""), Kind::Files(""));
    }
}
