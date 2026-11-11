//! The AI Assistant tab of the right dock: transcript, quick actions and the composer, driving a
//! coding agent (Claude Code or Codex) that runs in the open folder. The message goes to the agent
//! program installed on this computer (see `agent.rs`); Forge Room itself sends nothing anywhere.

use crate::agent::{self, Mode, Provider};
use crate::{brand, fonts, icons, theme};
use eframe::egui::{self, Align2, Color32, FontId, Key, Modifiers, RichText, Sense, Stroke};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Author {
    User,
    Assistant,
    /// A tool the agent used (an edit, a command): a small dim line.
    Tool,
    /// Something Forge Room says (an error, how a run ended).
    Note,
}

struct Message {
    author: Author,
    text: String,
}

pub struct State {
    messages: Vec<Message>,
    input: String,
    focus_input: bool,
    provider: Provider,
    mode: Mode,
    /// The conversation the next message continues (Claude Code only).
    session: Option<String>,
    run: Option<agent::Run>,
    /// Where each provider's program was found, looked up once.
    programs: Option<[Option<PathBuf>; 2]>,
    /// Full access was picked and waits for the user's confirmation.
    confirm_full: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            focus_input: false,
            provider: Provider::Claude,
            mode: Mode::Edit,
            session: None,
            run: None,
            programs: None,
            confirm_full: false,
        }
    }
}

const QUICK_ACTIONS: [(&str, &str); 4] = [
    ("Implement a feature", "Implement a feature: "),
    ("Explain this code", "Explain this code"),
    ("Write tests", "Write tests for "),
    ("Find and fix issues", "Find and fix issues in "),
];

impl State {
    pub fn busy(&self) -> bool {
        self.run.is_some()
    }

    fn note(&mut self, text: impl Into<String>) {
        self.messages.push(Message {
            author: Author::Note,
            text: text.into(),
        });
    }

    fn program(&mut self, provider: Provider) -> Option<PathBuf> {
        let found = self
            .programs
            .get_or_insert_with(|| [Provider::Claude.find(), Provider::Codex.find()]);
        found[usize::from(provider == Provider::Codex)].clone()
    }

    /// Send the typed message to the agent, in `cwd`, when that is allowed.
    fn send(&mut self, context: &egui::Context, cwd: Option<&Path>, trusted: bool) {
        let text = self.input.trim().to_string();
        if text.is_empty() || self.run.is_some() {
            return;
        }
        self.messages.push(Message {
            author: Author::User,
            text: text.clone(),
        });
        self.input.clear();
        self.focus_input = true;
        let Some(cwd) = cwd else {
            self.note("Open a folder first: the agent works in the open folder.");
            return;
        };
        if !trusted {
            self.note(
                "This folder is not trusted yet. An agent runs programs with your rights, so use \
                 Folder: Trust This Folder first.",
            );
            return;
        }
        let provider = self.provider;
        let Some(program) = self.program(provider) else {
            let how = match provider {
                Provider::Claude => "Install it with: npm install -g @anthropic-ai/claude-code",
                Provider::Codex => "Install it with: npm install -g @openai/codex",
            };
            self.note(format!(
                "{} was not found on this computer (looked on PATH and in the npm folder). {how}",
                provider.name()
            ));
            return;
        };
        let request = agent::Request {
            provider,
            mode: self.mode,
            prompt: text,
            cwd: cwd.to_path_buf(),
            session: self.session.clone(),
        };
        let wake = context.clone();
        match agent::start(&program, &request, move || wake.request_repaint()) {
            Ok(run) => self.run = Some(run),
            Err(error) => self.note(error),
        }
    }

    /// Take what the running agent has said since the last frame. True when the run just ended
    /// (files may have changed, so the caller refreshes what shows them).
    pub fn poll(&mut self) -> bool {
        let Some(run) = &mut self.run else {
            return false;
        };
        let mut finished = false;
        for event in run.poll() {
            match event {
                agent::Event::Text(text) => match self.messages.last_mut() {
                    Some(last) if last.author == Author::Assistant => {
                        last.text.push_str("\n\n");
                        last.text.push_str(&text);
                    }
                    _ => self.messages.push(Message {
                        author: Author::Assistant,
                        text,
                    }),
                },
                agent::Event::Tool(line) => self.messages.push(Message {
                    author: Author::Tool,
                    text: line,
                }),
                agent::Event::Session(id) => {
                    // Only Claude Code can continue a conversation this way.
                    if self.provider == Provider::Claude {
                        self.session = Some(id);
                    }
                }
                agent::Event::Done { cost, error } => {
                    finished = true;
                    match error {
                        Some(error) => self.note(error),
                        None => {
                            if let Some(cost) = cost {
                                self.note(format!("Done (cost ${cost:.3})"));
                            }
                        }
                    }
                }
            }
        }
        if finished {
            self.run = None;
        }
        finished
    }

    /// Stop the agent that is running.
    fn stop(&mut self) {
        if let Some(run) = &self.run {
            run.cancel();
        }
    }

    /// Forget the conversation (and stop the agent): the next message starts fresh.
    fn new_chat(&mut self) {
        self.stop();
        self.messages.clear();
        self.session = None;
        self.confirm_full = false;
    }

    /// Add `@name ` to the input, the way the design references files and Room context.
    fn mention(&mut self, name: &str) {
        if !self.input.is_empty() && !self.input.ends_with(' ') {
            self.input.push(' ');
        }
        self.input.push('@');
        self.input.push_str(name);
        self.input.push(' ');
        self.focus_input = true;
    }
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut State,
    active_file: Option<&str>,
    cwd: Option<&Path>,
    trusted: bool,
) {
    egui::Panel::bottom("ai-composer")
        .resizable(false)
        .frame(egui::Frame::new().inner_margin(egui::Margin::same(12)))
        .show_separator_line(false)
        .show(ui, |ui| composer(ui, state, active_file, cwd, trusted));

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.add_space(14.0);
            header(ui);
            ui.add_space(8.0);
            controls(ui, state);
            ui.add_space(10.0);
            if state.messages.is_empty() {
                quick_actions(ui, state);
            }
            for message in &state.messages {
                ui.add_space(12.0);
                bubble(ui, message);
            }
            ui.add_space(12.0);
        });
}

fn header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let (logo, _) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), Sense::hover());
        brand::logo(ui.painter(), logo.center(), 16.0);
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(
                RichText::new("Forge Room AI")
                    .size(15.0)
                    .family(fonts::semibold())
                    .color(theme::text()),
            );
            ui.label(
                RichText::new("Agentic coding in your workspace")
                    .size(12.5)
                    .color(theme::text_dim()),
            );
        });
    });
}

/// Which agent, what it may do, and a fresh conversation.
fn controls(ui: &mut egui::Ui, state: &mut State) {
    ui.horizontal_wrapped(|ui| {
        ui.add_space(14.0);
        let busy = state.busy();
        ui.add_enabled_ui(!busy, |ui| {
            let installed: Vec<bool> = Provider::ALL
                .iter()
                .map(|provider| state.program(*provider).is_some())
                .collect();
            egui::ComboBox::from_id_salt("ai-provider")
                .selected_text(state.provider.name())
                .show_ui(ui, |ui| {
                    for (provider, found) in Provider::ALL.iter().zip(&installed) {
                        let label = if *found {
                            provider.name().to_string()
                        } else {
                            format!("{} (not installed)", provider.name())
                        };
                        ui.selectable_value(&mut state.provider, *provider, label);
                    }
                });
            let before = state.mode;
            egui::ComboBox::from_id_salt("ai-mode")
                .selected_text(state.mode.label())
                .show_ui(ui, |ui| {
                    for mode in Mode::ALL {
                        ui.selectable_value(&mut state.mode, mode, mode.label())
                            .on_hover_text(mode.hint());
                    }
                });
            if state.mode == Mode::Full && before != Mode::Full {
                // Not allowed until the user has read what it means.
                state.mode = before;
                state.confirm_full = true;
            }
            if ui.button("New chat").clicked() {
                state.new_chat();
            }
        });
        if busy {
            ui.spinner();
            if ui.button("Stop").clicked() {
                state.stop();
            }
        }
    });
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(
            RichText::new(state.mode.hint())
                .size(12.0)
                .color(theme::text_dim()),
        );
    });
    if state.confirm_full {
        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(240, 113, 120, 28))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(240, 113, 120, 120),
            ))
            .corner_radius(8.0)
            .outer_margin(egui::Margin::symmetric(14, 6))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    "Full access lets the agent run any command, install software and use the \
                     network without asking. Only use it in code you trust.",
                );
                ui.horizontal(|ui| {
                    if ui.button("Allow full access").clicked() {
                        state.mode = Mode::Full;
                        state.confirm_full = false;
                    }
                    if ui.button("Cancel").clicked() {
                        state.confirm_full = false;
                    }
                });
            });
    }
}

fn quick_actions(ui: &mut egui::Ui, state: &mut State) {
    egui::Frame::new()
        .fill(theme::mix(theme::bg_sidebar(), theme::text(), 0.03))
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(10.0)
        .outer_margin(egui::Margin::symmetric(14, 0))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("How can I help you today?")
                    .size(16.0)
                    .family(fonts::semibold())
                    .color(theme::text()),
            );
            ui.add_space(10.0);
            let width = (ui.available_width() - 8.0) / 2.0;
            for pair in QUICK_ACTIONS.chunks(2) {
                ui.horizontal(|ui| {
                    for (label, prompt) in pair {
                        let button = egui::Button::new(*label)
                            .corner_radius(8.0)
                            .min_size(egui::vec2(width, 34.0));
                        if ui.add(button).clicked() {
                            state.input = (*prompt).to_string();
                            state.focus_input = true;
                        }
                    }
                });
            }
        });
}

fn bubble(ui: &mut egui::Ui, message: &Message) {
    match message.author {
        Author::Tool => {
            ui.horizontal(|ui| {
                ui.add_space(52.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("› {}", message.text))
                            .monospace()
                            .size(12.0)
                            .color(theme::text_dim()),
                    )
                    .truncate(),
                );
            });
            return;
        }
        Author::Note => {
            ui.horizontal_top(|ui| {
                ui.add_space(52.0);
                ui.set_max_width((ui.available_width() - 14.0).max(120.0));
                ui.label(
                    RichText::new(&message.text)
                        .size(12.5)
                        .color(theme::text_dim()),
                );
            });
            return;
        }
        _ => {}
    }
    let user = message.author == Author::User;
    ui.horizontal_top(|ui| {
        ui.add_space(14.0);
        let (avatar, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
        let painter = ui.painter();
        if user {
            painter.circle_filled(avatar.center(), 14.0, theme::overlay(18));
            icons::room(
                painter,
                egui::Rect::from_center_size(avatar.center(), egui::vec2(16.0, 16.0)),
                theme::text_dim(),
            );
        } else {
            brand::logo(painter, avatar.center(), 13.0);
        }
        ui.add_space(6.0);
        let fill = if user {
            theme::overlay(14)
        } else {
            Color32::TRANSPARENT
        };
        egui::Frame::new()
            .fill(fill)
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.set_max_width((ui.available_width() - 14.0).max(120.0));
                ui.label(RichText::new(&message.text).size(14.0).color(theme::text()));
            });
    });
}

fn composer(
    ui: &mut egui::Ui,
    state: &mut State,
    active_file: Option<&str>,
    cwd: Option<&Path>,
    trusted: bool,
) {
    let id = ui.scope_id().with("ai-input");
    // Enter sends, Shift+Enter adds a line. Consumed before the text box sees it.
    let focused = ui.memory(|memory| memory.has_focus(id));
    let send_requested =
        focused && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));

    egui::Frame::new()
        .stroke(Stroke::new(1.0, theme::border()))
        .fill(theme::mix(theme::bg_sidebar(), theme::bg_editor(), 0.5))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let editor = egui::TextEdit::multiline(&mut state.input)
                .id(id)
                .frame(egui::Frame::NONE)
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .hint_text(format!(
                    "Ask {} to work in this folder…  (Enter sends, Shift+Enter adds a line)",
                    state.provider.name()
                ));
            ui.add(editor);
            if std::mem::take(&mut state.focus_input) {
                ui.memory_mut(|memory| memory.request_focus(id));
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let attach = active_file.map(str::to_string);
                if icons::icon_button(ui, "Reference the open file", icons::plus).clicked()
                    && let Some(name) = &attach
                {
                    state.mention(name);
                }
                let at = ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::click());
                let hot = at.1.hovered();
                ui.painter().text(
                    at.0.center(),
                    Align2::CENTER_CENTER,
                    "@",
                    FontId::proportional(15.0),
                    if hot {
                        theme::text()
                    } else {
                        theme::text_dim()
                    },
                );
                if at.1.on_hover_text("Reference a file").clicked() {
                    state.mention(attach.as_deref().unwrap_or("workspace"));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(34.0, 30.0), Sense::click());
                    let busy = state.busy();
                    let ready = !state.input.trim().is_empty() && !busy;
                    let fill = if ready {
                        theme::button()
                    } else {
                        theme::mix(theme::bg_sidebar(), theme::button(), 0.35)
                    };
                    ui.painter().rect_filled(rect, 8.0, fill);
                    icons::draw(
                        ui.painter(),
                        if busy { "close" } else { "arrow-up" },
                        rect.center(),
                        18.0,
                        Color32::WHITE,
                    );
                    let clicked = response
                        .on_hover_text(if busy { "Stop" } else { "Send (Enter)" })
                        .clicked();
                    if busy && clicked {
                        state.stop();
                    } else if (clicked || send_requested) && ready {
                        let context = ui.ctx().clone();
                        state.send(&context, cwd, trusted);
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(state.provider.name())
                            .size(13.0)
                            .color(theme::text_dim()),
                    );
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sending_needs_a_folder_and_trust_and_says_why_not() {
        let context = egui::Context::default();
        let mut state = State {
            input: "  explain main.rs  ".into(),
            ..State::default()
        };
        state.send(&context, None, true);
        assert_eq!(state.messages[0].text, "explain main.rs");
        assert!(state.messages[1].text.contains("Open a folder"));
        assert!(state.input.is_empty() && !state.busy());
        state.input = "again".into();
        state.send(&context, Some(Path::new(".")), false);
        assert!(state.messages.last().unwrap().text.contains("not trusted"));
        assert!(!state.busy());
    }

    #[test]
    fn blank_input_is_not_sent_and_a_new_chat_forgets_everything() {
        let context = egui::Context::default();
        let mut state = State {
            input: "   \n".into(),
            ..State::default()
        };
        state.send(&context, Some(Path::new(".")), true);
        assert!(state.messages.is_empty());
        state.messages.push(Message {
            author: Author::User,
            text: "x".into(),
        });
        state.session = Some("s".into());
        state.new_chat();
        assert!(state.messages.is_empty() && state.session.is_none());
    }

    #[test]
    fn mention_appends_a_spaced_reference() {
        let mut state = State {
            input: "fix".into(),
            ..State::default()
        };
        state.mention("main.rs");
        assert_eq!(state.input, "fix @main.rs ");
    }
}
