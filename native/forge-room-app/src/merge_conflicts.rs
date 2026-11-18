//! Small, strict parser for Git's textual conflict markers. This is deliberately not a diff
//! engine: it preserves every byte outside marker lines and exposes each choice hunk separately.

use eframe::egui;
use std::path::{Component, Path};

pub const MAX_CONFLICT_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Part {
    Context(String),
    Conflict(Hunk),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hunk {
    pub current_label: String,
    pub current: String,
    pub base_label: Option<String>,
    pub base: Option<String>,
    pub incoming_label: String,
    pub incoming: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    Current,
    Incoming,
    Both,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictFile {
    pub parts: Vec<Part>,
}

pub struct Review {
    pub path: String,
    original: String,
    file: ConflictFile,
    choices: Vec<Option<Choice>>,
}

pub enum ReviewAction {
    Cancel,
    Apply {
        path: String,
        original: String,
        resolved: String,
    },
}

impl Review {
    pub fn new(path: String, original: String) -> Result<Self, String> {
        let file = ConflictFile::parse(&original)?;
        let choices = vec![None; file.conflict_count()];
        Ok(Self {
            path,
            original,
            file,
            choices,
        })
    }

    pub fn load(root: &Path, path: String) -> Result<Self, String> {
        let relative = Path::new(&path);
        if path.is_empty()
            || relative.is_absolute()
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err("the conflicted path is not workspace-relative".into());
        }
        let full_path = root.join(relative);
        let metadata = std::fs::symlink_metadata(&full_path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("the conflicted path is not a regular file".into());
        }
        if metadata.len() > MAX_CONFLICT_FILE_BYTES {
            return Err(format!(
                "conflict review is limited to {} MiB",
                MAX_CONFLICT_FILE_BYTES / (1024 * 1024)
            ));
        }
        let original = std::fs::read_to_string(&full_path).map_err(|error| error.to_string())?;
        Self::new(path, original)
    }
}

pub fn show(ctx: &egui::Context, review: &mut Review) -> Option<ReviewAction> {
    use egui::{Align, Color32, Layout, RichText};

    let mut action = None;
    egui::Modal::new(egui::Id::unique("git-conflict-review")).show(ctx, |ui| {
        ui.set_min_width(560.0);
        ui.set_max_width(760.0);
        ui.heading("Resolve merge conflicts");
        ui.label(RichText::new(&review.path).color(Color32::GRAY));
        ui.label("Choose a result for each marked section. Surrounding file text stays unchanged.");
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .max_height(420.0)
            .show(ui, |ui| {
                let mut choice_index = 0;
                for part in &review.file.parts {
                    let Part::Conflict(hunk) = part else { continue };
                    let selected = &mut review.choices[choice_index];
                    choice_index += 1;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(RichText::new(format!("Conflict {choice_index}")).strong());
                        ui.horizontal_wrapped(|ui| {
                            for (label, choice) in [
                                (format!("Current ({})", hunk.current_label), Choice::Current),
                                (
                                    format!("Incoming ({})", hunk.incoming_label),
                                    Choice::Incoming,
                                ),
                                ("Keep both".into(), Choice::Both),
                            ] {
                                if ui
                                    .selectable_label(*selected == Some(choice), label)
                                    .clicked()
                                {
                                    *selected = Some(choice);
                                }
                            }
                        });
                        egui::CollapsingHeader::new("Compare the two sides")
                            .id_salt(("conflict-sides", choice_index))
                            .show(ui, |ui| {
                                side_text(ui, "Current", &hunk.current);
                                if let Some(base) = &hunk.base {
                                    side_text(ui, "Base", base);
                                }
                                side_text(ui, "Incoming", &hunk.incoming);
                            });
                    });
                    ui.add_space(6.0);
                }
            });

        ui.separator();
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("Cancel").clicked() {
                action = Some(ReviewAction::Cancel);
            }
            let complete = review.choices.iter().all(Option::is_some);
            if ui
                .add_enabled(complete, egui::Button::new("Save resolved file and stage"))
                .clicked()
            {
                let choices: Vec<Choice> =
                    review.choices.iter().filter_map(|choice| *choice).collect();
                match review.file.resolve(&choices) {
                    Ok(resolved) => {
                        action = Some(ReviewAction::Apply {
                            path: review.path.clone(),
                            original: review.original.clone(),
                            resolved,
                        });
                    }
                    Err(error) => {
                        ui.colored_label(Color32::LIGHT_RED, error);
                    }
                }
            }
            ui.label(format!(
                "{} of {} chosen",
                review.choices.iter().filter(|c| c.is_some()).count(),
                review.choices.len()
            ));
        });
    });
    action
}

fn side_text(ui: &mut egui::Ui, title: &str, text: &str) {
    ui.label(egui::RichText::new(title).strong());
    egui::Frame::new()
        .fill(egui::Color32::from_black_alpha(18))
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.set_max_width(700.0);
            ui.label(
                egui::RichText::new(if text.is_empty() { "(empty)" } else { text }).monospace(),
            );
        });
}

impl ConflictFile {
    pub fn parse(text: &str) -> Result<Self, String> {
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let mut parts = Vec::new();
        let mut context = String::new();
        let mut index = 0;
        while index < lines.len() {
            let line = lines[index];
            let body = marker_body(line);
            let Some(current_label) = body.strip_prefix("<<<<<<< ") else {
                context.push_str(line);
                index += 1;
                continue;
            };

            if !context.is_empty() {
                parts.push(Part::Context(std::mem::take(&mut context)));
            }
            let current_label = current_label.trim_end().to_string();
            if current_label.is_empty() {
                return Err(format!(
                    "conflict {} has no current-side label",
                    parts.len() + 1
                ));
            }
            index += 1;
            let mut current = String::new();
            let mut base_label = None;
            let mut base = None;
            let mut incoming = String::new();
            let mut saw_separator = false;
            let mut incoming_label = None;
            while index < lines.len() {
                let body = marker_body(lines[index]);
                if let Some(label) = body.strip_prefix("||||||| ") {
                    if saw_separator || base.is_some() {
                        return Err("conflict contains more than one base marker".into());
                    }
                    base_label = Some(label.trim_end().to_string());
                    base = Some(String::new());
                } else if body == "=======" {
                    if saw_separator {
                        return Err("conflict contains more than one separator".into());
                    }
                    saw_separator = true;
                } else if let Some(closing_label) = body.strip_prefix(">>>>>>> ") {
                    if !saw_separator {
                        return Err("conflict is missing its ======= separator".into());
                    }
                    let closing_label = closing_label.trim_end().to_string();
                    if closing_label.is_empty() {
                        return Err("conflict has no incoming-side label".into());
                    }
                    incoming_label = Some(closing_label);
                    index += 1;
                    break;
                } else if body.strip_prefix("<<<<<<< ").is_some() {
                    return Err("nested conflict marker before closing marker".into());
                } else if saw_separator {
                    incoming.push_str(lines[index]);
                } else if let Some(base) = base.as_mut() {
                    base.push_str(lines[index]);
                } else {
                    current.push_str(lines[index]);
                }
                index += 1;
            }
            let Some(incoming_label) = incoming_label else {
                return Err("conflict is missing its >>>>>>> closing marker".into());
            };
            parts.push(Part::Conflict(Hunk {
                current_label,
                current,
                base_label,
                base,
                incoming_label,
                incoming,
            }));
        }
        if !context.is_empty() {
            parts.push(Part::Context(context));
        }
        if !parts.iter().any(|part| matches!(part, Part::Conflict(_))) {
            return Err("file contains no complete Git conflict markers".into());
        }
        Ok(Self { parts })
    }

    pub fn conflict_count(&self) -> usize {
        self.parts
            .iter()
            .filter(|part| matches!(part, Part::Conflict(_)))
            .count()
    }

    pub fn resolve(&self, choices: &[Choice]) -> Result<String, String> {
        if choices.len() != self.conflict_count() {
            return Err("choose a result for every conflict before saving".into());
        }
        let mut output = String::new();
        let mut choice_index = 0;
        for part in &self.parts {
            match part {
                Part::Context(text) => output.push_str(text),
                Part::Conflict(hunk) => {
                    let choice = choices[choice_index];
                    choice_index += 1;
                    match choice {
                        Choice::Current => output.push_str(&hunk.current),
                        Choice::Incoming => output.push_str(&hunk.incoming),
                        Choice::Both => {
                            output.push_str(&hunk.current);
                            output.push_str(&hunk.incoming);
                        }
                    }
                }
            }
        }
        Ok(output)
    }
}

fn marker_body(line: &str) -> &str {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_multiple_hunks_without_changing_context() {
        let file = ConflictFile::parse(
            "before\n<<<<<<< HEAD\none\n=======\ntwo\n>>>>>>> feature\nmiddle\n<<<<<<< HEAD\nthree\n=======\nfour\n>>>>>>> feature\nafter\n",
        )
        .unwrap();
        assert_eq!(file.conflict_count(), 2);
        assert_eq!(
            file.resolve(&[Choice::Incoming, Choice::Current]).unwrap(),
            "before\ntwo\nmiddle\nthree\nafter\n"
        );
    }

    #[test]
    fn recognizes_diff3_base_and_can_keep_both_sides() {
        let file = ConflictFile::parse(
            "<<<<<<< ours\ncurrent\n||||||| base\noriginal\n=======\nincoming\n>>>>>>> theirs\n",
        )
        .unwrap();
        let Part::Conflict(hunk) = &file.parts[0] else {
            panic!("expected a hunk")
        };
        assert_eq!(hunk.base_label.as_deref(), Some("base"));
        assert_eq!(hunk.base.as_deref(), Some("original\n"));
        assert_eq!(
            file.resolve(&[Choice::Both]).unwrap(),
            "current\nincoming\n"
        );
    }

    #[test]
    fn preserves_missing_final_newline_and_refuses_incomplete_or_nested_markers() {
        let file =
            ConflictFile::parse("<<<<<<< ours\nleft\n=======\nright\n>>>>>>> theirs").unwrap();
        assert_eq!(file.resolve(&[Choice::Incoming]).unwrap(), "right\n");
        assert!(ConflictFile::parse("<<<<<<< ours\nleft\n").is_err());
        assert!(
            ConflictFile::parse("<<<<<<< ours\n<<<<<<< nested\n=======\nr\n>>>>>>> theirs\n")
                .is_err()
        );
        assert!(file.resolve(&[]).is_err());
    }
}
