//! The AI Assistant tab of the right dock: transcript, quick actions and the composer.
//!
//! There is no model behind it yet. Sending a message records it and answers honestly that no
//! provider is connected, so nothing a user types leaves the machine. The shape (messages,
//! composer, context chips) is what a real provider will plug into.

use crate::{brand, fonts, icons, theme};
use eframe::egui::{self, Align2, Color32, FontId, Key, Modifiers, RichText, Sense, Stroke};

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
}

const QUICK_ACTIONS: [(&str, &str); 4] = [
    ("Implement a feature", "Implement a feature: "),
    ("Explain this code", "Explain this code"),
    ("Write tests", "Write tests for "),
    ("Find and fix issues", "Find and fix issues in "),
];

const NOT_CONNECTED: &str = "No AI provider is connected yet, so I can't answer. \
Your message stayed on this computer. Rooms already track agents and the tasks they own; \
running agents from this panel is on the roadmap.";

impl State {
    fn send(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }
        self.messages.push(Message {
            author: Author::User,
            text,
        });
        self.messages.push(Message {
            author: Author::Assistant,
            text: NOT_CONNECTED.to_string(),
        });
        self.input.clear();
        self.focus_input = true;
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

pub fn show(ui: &mut egui::Ui, state: &mut State, active_file: Option<&str>) {
    egui::Panel::bottom("ai-composer")
        .resizable(false)
        .frame(egui::Frame::new().inner_margin(egui::Margin::same(12)))
        .show_separator_line(false)
        .show(ui, |ui| composer(ui, state, active_file));

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

fn composer(ui: &mut egui::Ui, state: &mut State, active_file: Option<&str>) {
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
                    let ready = !state.input.trim().is_empty();
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
                        state.send();
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Forge AI")
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
    fn sending_records_the_message_and_an_honest_reply() {
        let mut state = State {
            input: "  explain main.rs  ".into(),
            ..State::default()
        };
        state.send();
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].text, "explain main.rs");
        assert!(state.messages[1].text.contains("No AI provider"));
        assert!(state.input.is_empty());
    }

    #[test]
    fn blank_input_is_not_sent() {
        let mut state = State {
            input: "   \n".into(),
            ..State::default()
        };
        state.send();
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
