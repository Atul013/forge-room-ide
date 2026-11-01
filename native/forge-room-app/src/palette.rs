//! Command palette and Quick Open, one box like VS Code: plain text searches files, `>` runs
//! commands, `:` jumps to a line.

use crate::commands::{self, Command};
use crate::{fuzzy, search, theme};
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
}

pub enum Outcome {
    Run(Command),
    Open(PathBuf),
    GoToLine(usize),
}

impl Palette {
    pub fn open_commands(&mut self) {
        self.query = ">".into();
        self.selected = 0;
        self.open = true;
        self.focus = true;
    }

    pub fn open_files(&mut self, root: Option<&Path>) {
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

    pub fn open_goto(&mut self) {
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
}

fn kind(query: &str) -> Kind<'_> {
    if let Some(rest) = query.strip_prefix('>') {
        Kind::Commands(rest.trim_start())
    } else if let Some(rest) = query.strip_prefix(':') {
        Kind::Line(rest.trim())
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
        Command(Command, &'static str, &'static str),
        File(usize),
        Line(usize),
        Hint(&'static str),
    }
    let rows: Vec<Row> = match kind(&palette.query) {
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
        Kind::Line(text) => match text.parse::<usize>() {
            Ok(line) if line > 0 => vec![Row::Line(line)],
            _ => vec![Row::Hint("Type a line number to jump to")],
        },
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
            Some(Row::Command(command, _, _)) => outcome = Some(Outcome::Run(*command)),
            Some(Row::File(file_index)) => {
                if let Some(root) = &palette.root {
                    outcome = Some(Outcome::Open(root.join(&palette.files[*file_index])));
                }
            }
            Some(Row::Line(line)) => outcome = Some(Outcome::GoToLine(*line)),
            _ => {}
        }
        if outcome.is_some() {
            close = true;
        }
    }
    if close {
        palette.open = false;
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
