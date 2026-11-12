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
    /// Something Forge Room says (how a run ended, what the agent was not allowed).
    Note,
    /// Something went wrong: shown in red.
    Error,
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
    /// The model to use; empty is the provider's default.
    model: String,
    /// The model box is a free text field (a name that is not in the list).
    model_custom: bool,
    /// How hard to think; empty is the provider's default.
    effort: String,
    /// The choices as last written to the settings file.
    saved: (String, String, String, String),
    /// The conversation the next message continues (Claude Code only).
    session: Option<String>,
    run: Option<agent::Run>,
    /// When the running agent was started.
    started: Option<std::time::Instant>,
    /// Where each program was found (Claude Code's, Codex's), looked up once.
    programs: Option<[Option<PathBuf>; 2]>,
    /// "Sign in" was clicked: the command for the terminal to run.
    login_requested: Option<&'static str>,
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
            model: String::new(),
            model_custom: false,
            effort: String::new(),
            saved: Default::default(),
            session: None,
            run: None,
            started: None,
            programs: None,
            login_requested: None,
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

const CLAUDE_MODELS: [(&str, &str); 5] = [
    ("Default model", ""),
    ("Fable", "fable"),
    ("Opus", "opus"),
    ("Sonnet", "sonnet"),
    ("Haiku", "haiku"),
];
const CODEX_MODELS: [(&str, &str); 1] = [("Default model", "")];

impl State {
    /// The agent panel as the user left it (read from the settings file).
    pub fn from_settings(settings: &crate::settings::Settings) -> Self {
        let mut state = State {
            provider: match settings.ai_provider.as_deref() {
                Some("codex") => Provider::Codex,
                Some("ollama") => Provider::Ollama,
                Some("lmstudio") => Provider::LmStudio,
                _ => Provider::Claude,
            },
            ..State::default()
        };
        let model = settings.ai_model.clone().unwrap_or_default();
        if agent::valid_model(&model) {
            state.model_custom = !presets(state.provider)
                .iter()
                .any(|(_, value)| *value == model);
            state.model = model;
        }
        let effort = settings.ai_effort.clone().unwrap_or_default();
        if agent::efforts(state.provider).contains(&effort.as_str()) {
            state.effort = effort;
        }
        // Full access is never carried over to the next start.
        state.mode = if settings.ai_mode.as_deref() == Some("read") {
            Mode::ReadOnly
        } else {
            Mode::Edit
        };
        state.saved = state.choices();
        state
    }

    fn choices(&self) -> (String, String, String, String) {
        (
            match self.provider {
                Provider::Claude => "claude",
                Provider::Codex => "codex",
                Provider::Ollama => "ollama",
                Provider::LmStudio => "lmstudio",
            }
            .into(),
            self.model.clone(),
            self.effort.clone(),
            if self.mode == Mode::ReadOnly {
                "read"
            } else {
                "edit"
            }
            .into(),
        )
    }

    /// The sign-in command the user asked for with the panel's button, once.
    pub fn take_login_request(&mut self) -> Option<&'static str> {
        self.login_requested.take()
    }

    pub fn busy(&self) -> bool {
        self.run.is_some()
    }

    fn fail(&mut self, text: impl Into<String>) {
        self.messages.push(Message {
            author: Author::Error,
            text: text.into(),
        });
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
        found[usize::from(provider.uses_codex())].clone()
    }

    /// Send the typed message to the agent, in `cwd`, when that is allowed. A message that is
    /// refused stays in the box, and the reason is shown.
    fn send(&mut self, context: &egui::Context, cwd: Option<&Path>, trusted: bool) {
        let text = self.input.trim().to_string();
        if text.is_empty() || self.run.is_some() {
            return;
        }
        let Some(cwd) = cwd else {
            self.fail("Open a folder first: the agent works in the open folder.");
            return;
        };
        if !trusted {
            self.fail(
                "This folder is not trusted yet. An agent runs programs with your rights, so use \
                 Folder: Trust This Folder first.",
            );
            return;
        }
        let provider = self.provider;
        // Installed since the panel last looked? Look again before saying it is missing.
        if self.program(provider).is_none() {
            self.programs = None;
        }
        let Some(program) = self.program(provider) else {
            let how = match provider {
                Provider::Claude => {
                    "Install it with: npm install -g @anthropic-ai/claude-code (or the native installer)"
                }
                Provider::Codex | Provider::Ollama | Provider::LmStudio => {
                    "Install it with: npm install -g @openai/codex (it also runs local models)"
                }
            };
            self.fail(format!(
                "{} was not found on this computer (looked on PATH and where installers put it). {how}",
                provider.name()
            ));
            return;
        };
        let request = agent::Request {
            provider,
            model: self.model.clone(),
            effort: self.effort.clone(),
            mode: self.mode,
            prompt: text.clone(),
            cwd: cwd.to_path_buf(),
            session: self.session.clone(),
        };
        let wake = context.clone();
        match agent::start(&program, &request, move || wake.request_repaint()) {
            Ok(run) => {
                self.run = Some(run);
                self.started = Some(std::time::Instant::now());
                self.messages.push(Message {
                    author: Author::User,
                    text,
                });
                self.input.clear();
                self.focus_input = true;
            }
            Err(error) => self.fail(error),
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
                agent::Event::Notice(text) => self.note(text),
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
                        Some(error) if error == "Stopped" => self.note("Stopped"),
                        Some(error) => self.fail(error),
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
            self.started = None;
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
    pub fn new_chat(&mut self) {
        // Dropping the run stops the agent and its events no longer reach the new chat.
        self.run = None;
        self.started = None;
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
    // A label in a horizontal row does not wrap unless told to: a long answer would then widen the
    // whole dock instead of breaking into lines.
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
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
            if state.busy() {
                ui.add_space(10.0);
                let seconds = state.started.map_or(0, |at| at.elapsed().as_secs());
                let last_tool = state
                    .messages
                    .iter()
                    .rev()
                    .take_while(|m| m.author != Author::User)
                    .find(|m| m.author == Author::Tool)
                    .map(|m| m.text.clone());
                ui.horizontal(|ui| {
                    ui.add_space(52.0);
                    ui.spinner();
                    ui.label(
                        RichText::new(match last_tool {
                            Some(tool) => format!("Working {seconds}s · {tool}"),
                            None => format!("Working {seconds}s"),
                        })
                        .size(13.0)
                        .color(theme::text_dim()),
                    );
                    if ui.small_button("Stop").clicked() {
                        state.stop();
                    }
                });
                // The seconds count up even when nothing arrives.
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs(1));
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
            let provider_before = state.provider;
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
            // Another provider has other models and effort levels.
            if state.provider != provider_before {
                state.model.clear();
                state.model_custom = false;
                state.session = None;
                if !agent::efforts(state.provider).contains(&state.effort.as_str()) {
                    state.effort.clear();
                }
            }
            let shown = if state.model_custom {
                "Custom model".to_string()
            } else {
                presets(state.provider)
                    .iter()
                    .find(|(_, value)| *value == state.model)
                    .map_or("Default model".to_string(), |(label, _)| {
                        (*label).to_string()
                    })
            };
            egui::ComboBox::from_id_salt("ai-model")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for (label, value) in presets(state.provider) {
                        let chosen = !state.model_custom && state.model == *value;
                        if ui.selectable_label(chosen, *label).clicked() {
                            state.model = (*value).to_string();
                            state.model_custom = false;
                        }
                    }
                    if ui.selectable_label(state.model_custom, "Custom…").clicked() {
                        state.model_custom = true;
                        state.model.clear();
                    }
                });
            if state.model_custom {
                ui.add(
                    egui::TextEdit::singleline(&mut state.model)
                        .hint_text("model name")
                        .desired_width(130.0),
                )
                .on_hover_text("For example claude-opus-4-1 or gpt-5-codex");
            }
            egui::ComboBox::from_id_salt("ai-effort")
                .selected_text(if state.effort.is_empty() {
                    "Default effort".to_string()
                } else {
                    format!("{} effort", state.effort)
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(state.effort.is_empty(), "Default effort")
                        .clicked()
                    {
                        state.effort.clear();
                    }
                    for level in agent::efforts(state.provider) {
                        if ui
                            .selectable_label(state.effort == *level, *level)
                            .clicked()
                        {
                            state.effort = (*level).to_string();
                        }
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
            if let Some(command) = state.provider.login_command()
                && ui
                    .button("Sign in")
                    .on_hover_text(format!("Opens a terminal and runs `{command}`"))
                    .clicked()
            {
                state.login_requested = Some(command);
            }
        });
    });
    // The choices are remembered for the next start.
    let now = state.choices();
    if now != state.saved {
        let (provider, model, effort, mode) = now.clone();
        let _ = crate::settings::update(|settings| {
            settings.ai_provider = Some(provider);
            settings.ai_model = Some(model);
            settings.ai_effort = Some(effort);
            settings.ai_mode = Some(mode);
        });
        state.saved = now;
    }
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

fn presets(provider: Provider) -> &'static [(&'static str, &'static str)] {
    match provider {
        Provider::Claude => &CLAUDE_MODELS,
        Provider::Codex | Provider::Ollama | Provider::LmStudio => &CODEX_MODELS,
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

/// One block of an answer written in Markdown.
#[derive(Debug, PartialEq)]
enum Block {
    Heading(usize, String),
    /// A list item: its depth (0, 1, ...), the marker (`•` or `3.`) and its text.
    Item(usize, String, String),
    Code(String),
    Paragraph(String),
}

/// The blocks of `text`: headings (`#`), list items (`-`, `*`, `1.`), fenced code and paragraphs
/// (consecutive lines stay together). Only what agents actually write; no tables or links.
fn blocks(text: &str) -> Vec<Block> {
    let mut result = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut code: Option<Vec<&str>> = None;
    let flush = |paragraph: &mut Vec<&str>, result: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            result.push(Block::Paragraph(paragraph.join("\n")));
            paragraph.clear();
        }
    };
    for line in text.lines() {
        if let Some(lines) = &mut code {
            if line.trim_start().starts_with("```") {
                result.push(Block::Code(lines.join("\n")));
                code = None;
            } else {
                lines.push(line);
            }
            continue;
        }
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed.starts_with("```") {
            flush(&mut paragraph, &mut result);
            code = Some(Vec::new());
        } else if trimmed.is_empty() {
            flush(&mut paragraph, &mut result);
        } else if let Some(heading) = trimmed
            .strip_prefix('#')
            .filter(|rest| rest.trim_start_matches('#').starts_with(' '))
        {
            flush(&mut paragraph, &mut result);
            let level = 1 + heading.chars().take_while(|c| *c == '#').count();
            let title = heading.trim_start_matches('#').trim();
            if title.is_empty() || level > 6 {
                paragraph.push(line);
            } else {
                result.push(Block::Heading(level, title.to_string()));
            }
        } else if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            flush(&mut paragraph, &mut result);
            result.push(Block::Item(indent / 2, "•".into(), item.trim().to_string()));
        } else if let Some((number, rest)) = trimmed.split_once(". ")
            && !number.is_empty()
            && number.len() <= 3
            && number.chars().all(|c| c.is_ascii_digit())
        {
            flush(&mut paragraph, &mut result);
            result.push(Block::Item(
                indent / 2,
                format!("{number}."),
                rest.trim().to_string(),
            ));
        } else {
            paragraph.push(line);
        }
    }
    // An unclosed fence is still shown as code.
    if let Some(lines) = code {
        result.push(Block::Code(lines.join("\n")));
    }
    flush(&mut paragraph, &mut result);
    result
}

/// How a span of inline text is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Span {
    bold: bool,
    code: bool,
}

/// `text` split at `**bold**` and `` `code` `` markers. A marker that is never closed stays as
/// written.
fn spans(text: &str) -> Vec<(String, Span)> {
    let mut result: Vec<(String, Span)> = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    let push = |result: &mut Vec<(String, Span)>, plain: &mut String| {
        if !plain.is_empty() {
            result.push((std::mem::take(plain), Span::default()));
        }
    };
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("**")
            && let Some(end) = after.find("**")
            && end > 0
        {
            push(&mut result, &mut plain);
            result.push((
                after[..end].to_string(),
                Span {
                    bold: true,
                    code: false,
                },
            ));
            rest = &after[end + 2..];
        } else if let Some(after) = rest.strip_prefix('`')
            && let Some(end) = after.find('`')
            && end > 0
        {
            push(&mut result, &mut plain);
            result.push((
                after[..end].to_string(),
                Span {
                    bold: false,
                    code: true,
                },
            ));
            rest = &after[end + 1..];
        } else {
            let mut chars = rest.chars();
            if let Some(c) = chars.next() {
                plain.push(c);
            }
            rest = chars.as_str();
        }
    }
    push(&mut result, &mut plain);
    result
}

/// An answer drawn as Markdown: headings, bold, inline code, lists and code blocks.
fn markdown(ui: &mut egui::Ui, text: &str) {
    for block in blocks(text) {
        match block {
            Block::Code(code) => {
                egui::Frame::new()
                    .fill(theme::overlay(14))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let mut job = egui::text::LayoutJob::single_section(
                            code,
                            egui::TextFormat::simple(FontId::monospace(12.5), theme::text()),
                        );
                        job.wrap.max_width = ui.available_width();
                        job.wrap.break_anywhere = true;
                        ui.label(job);
                    });
                ui.add_space(6.0);
            }
            Block::Heading(level, title) => {
                let size = match level {
                    1 => 17.0,
                    2 => 16.0,
                    _ => 15.0,
                };
                ui.add_space(4.0);
                inline(ui, &title, size, true, "");
                ui.add_space(2.0);
            }
            Block::Item(depth, marker, item) => {
                let lead = format!("{}{marker} ", "    ".repeat(depth));
                inline(ui, &item, 14.0, false, &lead);
            }
            Block::Paragraph(paragraph) => {
                inline(ui, &paragraph, 14.0, false, "");
                ui.add_space(6.0);
            }
        }
    }
}

/// One run of text with bold and code spans, after an optional `lead` (a list marker).
fn inline(ui: &mut egui::Ui, text: &str, size: f32, heading: bool, lead: &str) {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    let normal = |bold: bool| {
        let family = if bold || heading {
            fonts::semibold()
        } else {
            egui::FontFamily::Proportional
        };
        egui::TextFormat::simple(
            FontId::new(size, family),
            if bold || heading {
                theme::text()
            } else {
                theme::mix(theme::text(), theme::bg_sidebar(), 0.08)
            },
        )
    };
    if !lead.is_empty() {
        job.append(lead, 0.0, normal(false));
    }
    for (piece, span) in spans(text) {
        if span.code {
            let mut format =
                egui::TextFormat::simple(FontId::monospace(size * 0.92), theme::text());
            format.background = theme::overlay(26);
            job.append(&piece, 0.0, format);
        } else {
            job.append(&piece, 0.0, normal(span.bold));
        }
    }
    ui.label(job);
}

/// Text that breaks into lines at the width it is given, even inside one long word (a path, a hash):
/// it never makes the panel around it wider.
fn wrapped(ui: &mut egui::Ui, text: &str, size: f32, color: Color32) {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_string(),
        egui::TextFormat::simple(FontId::proportional(size), color),
    );
    job.wrap.max_width = ui.available_width();
    ui.label(job);
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
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 52,
                    right: 14,
                    top: 0,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    wrapped(ui, &message.text, 13.0, theme::text_dim());
                });
            return;
        }
        Author::Error => {
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 52,
                    right: 14,
                    top: 0,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(240, 113, 120, 26))
                        .stroke(Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(240, 113, 120, 110),
                        ))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            wrapped(ui, &message.text, 13.0, Color32::from_rgb(244, 150, 156));
                        });
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
                // Top to bottom: the frame inherits the row's left-to-right layout, which would put
                // the Copy button beside the text and widen the whole panel.
                ui.vertical(|ui| {
                    if user {
                        wrapped(ui, &message.text, 14.0, theme::text());
                    } else {
                        markdown(ui, &message.text);
                    }
                    if !user
                        && ui
                            .small_button("Copy")
                            .on_hover_text("Copy this answer")
                            .clicked()
                    {
                        ui.ctx().copy_text(message.text.clone());
                    }
                });
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
    // `consume_key` ignores a held Shift, so Shift+Enter has to be told apart first.
    let enter =
        focused && ui.input(|input| input.key_pressed(Key::Enter) && !input.modifiers.shift);
    let send_requested =
        enter && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
    if focused && state.busy() && ui.input(|input| input.key_pressed(Key::Escape)) {
        state.stop();
    }

    egui::Frame::new()
        .stroke(Stroke::new(1.0, theme::border()))
        .fill(theme::mix(theme::bg_sidebar(), theme::bg_editor(), 0.5))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let hint = if state.busy() {
                format!("{} is working… (Esc stops it)", state.provider.name())
            } else {
                format!(
                    "Ask {} to work in this folder…  (Enter sends, Shift+Enter adds a line)",
                    state.provider.name()
                )
            };
            let editor = egui::TextEdit::multiline(&mut state.input)
                .id(id)
                .frame(egui::Frame::NONE)
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .hint_text(hint);
            // A long paste scrolls inside the box instead of pushing the answers out of view.
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .auto_shrink([false, true])
                .show(ui, |ui| ui.add(editor));
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
                    if state.mode == Mode::Full {
                        ui.label(
                            RichText::new("Full access")
                                .size(12.0)
                                .strong()
                                .color(Color32::from_rgb(244, 150, 156)),
                        );
                    }
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
        assert!(state.messages[0].text.contains("Open a folder"));
        // A refused message stays in the box, to be sent again.
        assert_eq!(state.input.trim(), "explain main.rs");
        assert!(!state.busy());
        state.send(&context, Some(Path::new(".")), false);
        assert!(state.messages.last().unwrap().text.contains("not trusted"));
        assert_eq!(state.input.trim(), "explain main.rs");
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

    /// Frames of the dock (a resizable panel with a size range) showing `messages`: the widths it had.
    fn dock_widths(messages: Vec<(Author, String)>) -> Vec<f32> {
        let context = egui::Context::default();
        let mut state = State::default();
        for (author, text) in messages {
            state.messages.push(Message { author, text });
        }
        let mut widths = Vec::new();
        for frame in 0..6 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                time: Some(f64::from(frame) * 0.1),
                ..egui::RawInput::default()
            };
            let mut output = context.run_ui(input, |ui| {
                let drawn = egui::Panel::right("test-dock")
                    .default_size(364.0)
                    .size_range(280.0..=640.0)
                    .resizable(true)
                    .show(ui, |ui| {
                        show(ui, &mut state, Some("main.rs"), Some(Path::new(".")), true)
                    });
                widths.push(drawn.response.rect.width());
            });
            output.textures_delta.clear();
        }
        widths
    }

    /// The dock must keep its width whatever the transcript holds: ordinary answers, errors and tool
    /// lines never widen it (it once changed size after a second message).
    #[test]
    fn the_dock_keeps_its_width_with_long_answers_errors_and_tool_lines() {
        let long = "word ".repeat(400);
        let widths = dock_widths(vec![
            (Author::User, "hi".to_string()),
            (Author::Assistant, "Hello! How can I help?".to_string()),
            (
                Author::Tool,
                "Bash cargo test  ;  cargo clippy  ;  git status".to_string(),
            ),
            (Author::User, long.clone()),
            (
                Author::Assistant,
                format!(
                    "{long}

{long}"
                ),
            ),
            (Author::Error, format!("Claude Code stopped: {long}")),
            (
                Author::Note,
                format!("The agent was not allowed to use: Bash. {long}"),
            ),
        ]);
        eprintln!("ordinary widths {widths:?}");
        assert!(
            widths.iter().all(|w| (*w - 364.0).abs() < 2.0),
            "{widths:?}"
        );
    }

    /// A single word with no break (a long path or hash) must not widen the dock either.
    #[test]
    fn an_unbroken_word_does_not_widen_the_dock() {
        let unbroken = "x".repeat(300);
        let widths = dock_widths(vec![
            (Author::Assistant, format!("see {unbroken}")),
            (Author::Tool, format!("Bash {unbroken}")),
            (Author::Error, unbroken.clone()),
            (Author::Note, unbroken),
        ]);
        eprintln!("unbroken widths {widths:?}");
        assert!(
            widths.iter().all(|w| (*w - 364.0).abs() < 2.0),
            "{widths:?}"
        );
    }

    #[test]
    fn markdown_blocks_and_spans_are_read_as_agents_write_them() {
        let text = "## Done today\n\nI fixed **three** bugs in `main.rs`.\nSecond line.\n\n- first\n  - nested\n1. step\n```rust\nfn main() {}\n```\nafter";
        assert_eq!(
            blocks(text),
            vec![
                Block::Heading(2, "Done today".into()),
                Block::Paragraph("I fixed **three** bugs in `main.rs`.\nSecond line.".into()),
                Block::Item(0, "•".into(), "first".into()),
                Block::Item(1, "•".into(), "nested".into()),
                Block::Item(0, "1.".into(), "step".into()),
                Block::Code("fn main() {}".into()),
                Block::Paragraph("after".into()),
            ]
        );
        let found = spans("I fixed **three** bugs in `main.rs`.");
        assert_eq!(found.len(), 5);
        assert!(found[1].1.bold && found[3].1.code);
        assert_eq!(found[1].0, "three");
        // Markers that are never closed, and a lone hash, stay as written.
        assert_eq!(spans("a **b").len(), 1);
        assert_eq!(spans("a ` b")[0].0, "a ` b");
        assert_eq!(blocks("#"), vec![Block::Paragraph("#".into())]);
        // An unclosed fence still shows its code; text with multibyte characters is not split.
        assert_eq!(blocks("```\nx"), vec![Block::Code("x".into())]);
        assert_eq!(spans("héllo **wörld**")[1].0, "wörld");
    }

    /// Answers drawn as Markdown (headings, code blocks, long lists) keep the dock's width too.
    #[test]
    fn a_markdown_answer_keeps_the_dock_width() {
        let long = "word ".repeat(150);
        let answer = format!(
            "## Heading\n\n**bold** and `code` {long}\n\n- item {long}\n\n```\n{}\n```",
            "x".repeat(200)
        );
        let widths = dock_widths(vec![(Author::Assistant, answer)]);
        assert!(
            widths.iter().all(|w| (*w - 364.0).abs() < 2.0),
            "{widths:?}"
        );
    }
}
