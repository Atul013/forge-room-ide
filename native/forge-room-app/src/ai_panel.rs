//! The AI Assistant tab of the right dock: transcript, quick actions and the composer.
//!
//! The first functional path runs Codex CLI in read-only Ask mode. Authentication is managed by
//! the CLI in the integrated terminal; this panel never handles credentials.

use crate::{ai_backend::{self, Provider}, brand, fonts, icons, theme};
use eframe::egui::{self, Align2, Color32, FontId, Key, Modifiers, RichText, Sense, Stroke};
use std::{path::Path, sync::mpsc};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Author {
    User,
    Assistant,
}

struct Message {
    author: Author,
    text: String,
}

#[derive(Default)]
pub struct State {
    messages: Vec<Message>,
    input: String,
    focus_input: bool,
    provider: Provider,
    model: String,
    pending: Option<mpsc::Receiver<Result<String, String>>>,
}

const QUICK_ACTIONS: [(&str, &str); 4] = [
    ("Implement a feature", "Implement a feature: "),
    ("Explain this code", "Explain this code"),
    ("Write tests", "Write tests for "),
    ("Find and fix issues", "Find and fix issues in "),
];

impl State {
    fn send(&mut self, context: &egui::Context, root: Option<&Path>) {
        if self.pending.is_some() { return; }
        let text = self.input.trim().to_string();
        if text.is_empty() || root.is_none() {
            return;
        }
        self.messages.push(Message {
            author: Author::User,
            text: text.clone(),
        });
        let root = root.unwrap().to_path_buf();
        let provider = self.provider;
        let model = self.model.clone();
        let (sender, receiver) = mpsc::channel();
        let context = context.clone();
        std::thread::spawn(move || {
            let _ = sender.send(ai_backend::ask(&root, provider, &model, &text));
            context.request_repaint();
        });
        self.pending = Some(receiver);
        self.input.clear();
        self.focus_input = true;
    }

    fn poll(&mut self) {
        let Some(receiver) = self.pending.as_ref() else { return; };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err("Codex worker stopped without a reply.".into()),
        };
        self.pending = None;
        self.messages.push(Message { author: Author::Assistant, text: result.unwrap_or_else(|error| error) });
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

pub fn show(ui: &mut egui::Ui, state: &mut State, active_file: Option<&str>, root: Option<&Path>) {
    state.poll();
    egui::Panel::bottom("ai-composer")
        .resizable(false)
        .frame(egui::Frame::new().inner_margin(egui::Margin::same(12)))
        .show_separator_line(false)
        .show(ui, |ui| composer(ui, state, active_file, root));

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.add_space(14.0);
            header(ui);
            ui.add_space(14.0);
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

fn composer(ui: &mut egui::Ui, state: &mut State, active_file: Option<&str>, root: Option<&Path>) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("ai-provider")
            .selected_text(state.provider.label())
            .show_ui(ui, |ui| {
                for provider in [Provider::Codex, Provider::Ollama, Provider::LmStudio] {
                    ui.selectable_value(&mut state.provider, provider, provider.label());
                }
            });
        ui.add(egui::TextEdit::singleline(&mut state.model).desired_width(120.0).hint_text("Default model"));
    });
    ui.label(RichText::new("Ask only · read-only Codex CLI. Sign in with `codex login` in Terminal; local providers use Codex CLI too.").size(11.0).color(theme::text_dim()));
    if root.is_none() { ui.label("Open and trust a project folder before asking."); }
    if state.pending.is_some() { ui.spinner(); ui.label("Codex is working…"); }
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
                .hint_text("Ask anything, or use @ to reference files, agents, or Room context…");
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
                    let ready = !state.input.trim().is_empty() && root.is_some() && state.pending.is_none();
                    let fill = if ready {
                        theme::button()
                    } else {
                        theme::mix(theme::bg_sidebar(), theme::button(), 0.35)
                    };
                    ui.painter().rect_filled(rect, 8.0, fill);
                    icons::draw(
                        ui.painter(),
                        "arrow-up",
                        rect.center(),
                        18.0,
                        Color32::WHITE,
                    );
                    if (response.on_hover_text("Send (Enter)").clicked() || send_requested) && ready
                    {
                        state.send(ui.ctx(), root);
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Ask")
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
    fn sending_without_a_project_does_not_start_a_provider() {
        let mut state = State {
            input: "  explain main.rs  ".into(),
            ..State::default()
        };
        state.send(&egui::Context::default(), None);
        assert!(state.messages.is_empty());
    }

    #[test]
    fn blank_input_is_not_sent() {
        let mut state = State {
            input: "   \n".into(),
            ..State::default()
        };
        state.send(&egui::Context::default(), Some(Path::new(".")));
        assert!(state.messages.is_empty());
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
