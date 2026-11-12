//! The Search sidebar (find in files). The search runs on a background thread and a newer
//! query cancels an older one, so typing never blocks the window.

use crate::{explorer, find, search, theme};
use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

pub struct SearchView {
    pub query: String,
    pub mode: find::Mode,
    results: Option<search::Results>,
    searched_query: String,
    running: bool,
    cancel: Arc<AtomicBool>,
    receiver: Option<mpsc::Receiver<search::Results>>,
    dirty_since: Option<Instant>,
    focus: bool,
    /// The text that replaces every match (Replace All).
    pub replacement: String,
    /// The mode the shown results were searched with.
    searched_mode: find::Mode,
    /// Replace All was clicked and waits for a second click.
    confirming: bool,
    /// Replace was clicked; it starts at the top of the next frame.
    replace_requested: bool,
    replacing: Option<mpsc::Receiver<search::Replaced>>,
    replaced: Option<search::Replaced>,
}

impl Default for SearchView {
    fn default() -> Self {
        Self {
            query: String::new(),
            mode: find::Mode::default(),
            results: None,
            searched_query: String::new(),
            running: false,
            cancel: Arc::new(AtomicBool::new(false)),
            receiver: None,
            dirty_since: None,
            focus: false,
            replacement: String::new(),
            searched_mode: find::Mode::default(),
            confirming: false,
            replace_requested: false,
            replacing: None,
            replaced: None,
        }
    }
}

pub enum Action {
    None,
    Open {
        path: PathBuf,
        line: usize,
        column: usize,
        length: usize,
    },
}

const ROW_HEIGHT: f32 = 24.0;
const DEBOUNCE: Duration = Duration::from_millis(350);

impl SearchView {
    pub fn focus_input(&mut self) {
        self.focus = true;
    }

    pub fn reset(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.results = None;
        self.running = false;
        self.receiver = None;
    }

    fn start(&mut self, root: &Path) {
        self.cancel.store(true, Ordering::Relaxed);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = cancel.clone();
        self.searched_query = self.query.clone();
        self.searched_mode = self.mode;
        // What a pending confirmation was about is no longer what is shown.
        self.confirming = false;
        if self.query.is_empty() {
            self.results = None;
            self.running = false;
            self.receiver = None;
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let (root, query, mode) = (root.to_path_buf(), self.query.clone(), self.mode);
        std::thread::spawn(move || {
            let _ = sender.send(search::search(&root, &query, mode, &cancel));
        });
        self.receiver = Some(receiver);
        self.running = true;
    }

    /// Replace every match of the query that was searched in every file, except the `protected`
    /// ones (open with unsaved changes). It runs on a worker thread; `take_replaced` reports.
    fn start_replace(&mut self, root: &Path, protected: Vec<PathBuf>) {
        let (root, query, mode, replacement) = (
            root.to_path_buf(),
            self.searched_query.clone(),
            self.searched_mode,
            self.replacement.clone(),
        );
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let never = AtomicBool::new(false);
            let _ = sender.send(search::replace_in_files(
                &root,
                &query,
                mode,
                &replacement,
                &protected,
                &never,
            ));
        });
        self.replacing = Some(receiver);
        self.confirming = false;
    }

    /// A finished Replace All, once; the results are searched again afterwards.
    pub fn take_replaced(&mut self) -> Option<search::Replaced> {
        if let Some(receiver) = &self.replacing {
            let done = match receiver.try_recv() {
                Ok(done) => Some(done),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(search::Replaced {
                    error: Some("the replace stopped unexpectedly".into()),
                    ..search::Replaced::default()
                }),
            };
            if let Some(done) = done {
                self.replacing = None;
                self.dirty_since = Some(Instant::now() - DEBOUNCE);
                self.replaced = Some(done);
            }
        }
        self.replaced.take()
    }

    /// Pick up a finished search. True while work is still pending (caller should repaint soon).
    pub fn poll(&mut self) -> bool {
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(results) => {
                    if !results.cancelled {
                        self.results = Some(results);
                    }
                    self.running = false;
                    self.receiver = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    // The search thread died: stop showing a spinner for it.
                    self.running = false;
                    self.receiver = None;
                }
            }
        }
        // A pending search only needs frames while the sidebar can start it; when the sidebar is not
        // shown the wish is kept (see `show`) and no repaint loop is run for it.
        if self
            .dirty_since
            .is_some_and(|since| since.elapsed() > DEBOUNCE * 4)
        {
            self.dirty_since = None;
        }
        self.running || self.replacing.is_some() || self.dirty_since.is_some()
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        root: Option<&Path>,
        protected: &[PathBuf],
    ) -> Action {
        let mut action = Action::None;
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            ui.label(
                RichText::new("SEARCH")
                    .size(11.0)
                    .strong()
                    .color(theme::text_dim()),
            );
        });
        ui.add_space(8.0);
        let Some(root) = root else {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(
                    RichText::new("Open a folder to search its files.").color(theme::text_dim()),
                );
            });
            return action;
        };

        // Text typed while the sidebar was away (or a search that never started) is searched now.
        if self.dirty_since.is_none()
            && !self.running
            && self.query != self.searched_query
            && self.receiver.is_none()
        {
            self.dirty_since = Some(Instant::now() - DEBOUNCE);
        }
        if std::mem::take(&mut self.replace_requested) {
            self.start_replace(root, protected.to_vec());
        }
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(12, 0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Search")
                            .desired_width(ui.available_width() - 40.0)
                            .margin(egui::Margin::symmetric(10, 7)),
                    );
                    if self.focus {
                        input.request_focus();
                        self.focus = false;
                    }
                    if input.changed() {
                        self.dirty_since = Some(Instant::now());
                    }
                    if input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.dirty_since = None;
                        self.start(root);
                    }
                    for (flag, label, hint) in [
                        (&mut self.mode.case_sensitive, "Aa", "Match case"),
                        (&mut self.mode.whole_word, "W", "Match whole word"),
                        (&mut self.mode.regex, ".*", "Use regular expression"),
                    ] {
                        if ui.toggle_value(flag, label).on_hover_text(hint).changed() {
                            self.dirty_since = Some(Instant::now());
                        }
                    }
                });
            });
        if !self.query.is_empty() {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(12, 0))
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    let replacement = ui.add(
                        egui::TextEdit::singleline(&mut self.replacement)
                            .hint_text("Replace with")
                            .desired_width(ui.available_width())
                            .margin(egui::Margin::symmetric(10, 7)),
                    );
                    if replacement.changed() {
                        self.confirming = false;
                    }
                    if self.mode.regex {
                        ui.label(
                            RichText::new("The replacement is plain text: $1 is not a group.")
                                .size(12.0)
                                .color(theme::text_dim()),
                        );
                    }
                });
        }
        if let Some(since) = self.dirty_since
            && since.elapsed() >= DEBOUNCE
        {
            self.dirty_since = None;
            self.start(root);
        }
        ui.add_space(8.0);

        if self.replacing.is_some() {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.spinner();
                ui.label(RichText::new("Replacing…").color(theme::text_dim()));
            });
            return action;
        }
        if self.running {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.spinner();
                ui.label(RichText::new("Searching…").color(theme::text_dim()));
            });
            return action;
        }
        let Some(results) = &self.results else {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(
                    RichText::new("Type to search every text file in this folder.")
                        .color(theme::text_dim()),
                );
            });
            return action;
        };

        ui.horizontal(|ui| {
            ui.add_space(14.0);
            let files = {
                let mut seen: Vec<&PathBuf> = results.hits.iter().map(|hit| &hit.path).collect();
                seen.dedup();
                seen.len()
            };
            let text = if results.files_cut && results.hits.is_empty() && results.error.is_none() {
                format!(
                    "No results in {} files (only the first {} files of this folder are searched)",
                    results.files_scanned,
                    search::MAX_LISTED
                )
            } else if let Some(error) = &results.error {
                format!("Invalid pattern: {}", error.lines().next().unwrap_or(""))
            } else if results.hits.is_empty() {
                format!("No results in {} files", results.files_scanned)
            } else if results.truncated {
                format!(
                    "{}+ results in {} files (showing the first {})",
                    results.hits.len(),
                    files,
                    search::MAX_HITS
                )
            } else {
                format!("{} results in {} files", results.hits.len(), files)
            };
            ui.label(RichText::new(text).size(12.0).color(theme::text_dim()));
        });
        let replaceable = results.error.is_none() && !results.hits.is_empty();
        let (hits, truncated) = (results.hits.len(), results.truncated);
        let files = {
            let mut seen: Vec<&PathBuf> = results.hits.iter().map(|hit| &hit.path).collect();
            seen.dedup();
            seen.len()
        };
        if replaceable {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                if self.confirming {
                    let text = if truncated {
                        "Replace every match on disk, including those not listed? Files with \
                         unsaved changes are skipped. Undo with Git."
                            .to_string()
                    } else {
                        format!(
                            "Replace {hits} matches in {files} files on disk? Files with unsaved \
                             changes are skipped. Undo with Git."
                        )
                    };
                    ui.label(RichText::new(text).size(12.0).color(theme::text()));
                } else if ui.button("Replace All").clicked() {
                    self.confirming = true;
                }
            });
            if self.confirming {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    if ui.button("Replace").clicked() {
                        self.replace_requested = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirming = false;
                    }
                });
            }
        } else {
            self.confirming = false;
        }
        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let mut last: Option<&PathBuf> = None;
                for hit in &results.hits {
                    if last != Some(&hit.path) {
                        last = Some(&hit.path);
                        file_header(ui, root, &hit.path);
                    }
                    if hit_row(ui, hit) {
                        action = Action::Open {
                            path: hit.path.clone(),
                            line: hit.line,
                            column: hit.column,
                            length: hit.length,
                        };
                    }
                }
            });
        action
    }
}

fn file_header(ui: &mut egui::Ui, root: &Path, path: &Path) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT + 2.0),
        Sense::hover(),
    );
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let (directory, name) = match relative.rsplit_once('/') {
        Some((directory, name)) => (directory.to_string(), name.to_string()),
        None => (String::new(), relative.clone()),
    };
    let y = rect.center().y;
    let painter = ui.painter();
    crate::icons::file(
        painter,
        Rect::from_center_size(egui::pos2(rect.left() + 22.0, y), egui::vec2(10.0, 13.0)),
        explorer::file_color(&name),
    );
    let galley = painter.layout_no_wrap(name, FontId::proportional(13.0), theme::text());
    let width = galley.size().x;
    painter.galley(
        egui::pos2(rect.left() + 38.0, y - galley.size().y / 2.0),
        galley,
        theme::text(),
    );
    painter.text(
        egui::pos2(rect.left() + 46.0 + width, y),
        Align2::LEFT_CENTER,
        directory,
        FontId::proportional(11.5),
        theme::text_dim(),
    );
}

/// One result line with the matched text highlighted. True when clicked.
fn hit_row(ui: &mut egui::Ui, hit: &search::Hit) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::overlay(10));
    }
    let font = FontId::monospace(12.5);
    let mut job = egui::text::LayoutJob::default();
    let normal = egui::TextFormat::simple(font.clone(), theme::text_dim());
    let mut marked = egui::TextFormat::simple(font, theme::text());
    marked.background = Color32::from_rgba_unmultiplied(
        theme::accent().r(),
        theme::accent().g(),
        theme::accent().b(),
        90,
    );
    let chars: Vec<char> = hit.preview.chars().collect();
    let mut cursor = 0;
    for range in hit.marks.iter().cloned() {
        job.append(
            &chars[cursor..range.start].iter().collect::<String>(),
            0.0,
            normal.clone(),
        );
        job.append(
            &chars[range.start..range.end].iter().collect::<String>(),
            0.0,
            marked.clone(),
        );
        cursor = range.end;
    }
    job.append(&chars[cursor..].iter().collect::<String>(), 0.0, normal);
    let galley = ui.painter().layout_job(job);
    let painter = ui.painter().with_clip_rect(Rect::from_min_max(
        egui::pos2(rect.left() + 38.0, rect.top()),
        rect.right_bottom(),
    ));
    painter.galley(
        egui::pos2(rect.left() + 38.0, rect.center().y - galley.size().y / 2.0),
        galley,
        theme::text(),
    );
    painter_text_dim(ui, rect, hit.line);
    response.clicked()
}

fn painter_text_dim(ui: &egui::Ui, rect: Rect, line: usize) {
    ui.painter().text(
        egui::pos2(rect.left() + 34.0, rect.center().y),
        Align2::RIGHT_CENTER,
        line,
        FontId::proportional(11.0),
        theme::mix(theme::bg_sidebar(), theme::text(), 0.4),
    );
}
