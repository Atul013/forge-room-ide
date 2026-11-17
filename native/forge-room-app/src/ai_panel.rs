//! The AI Assistant tab of the right dock: transcript, quick actions and the composer, driving a
//! coding agent (Claude Code or Codex) that runs in the open folder. The message goes to the agent
//! program installed on this computer, or an explicitly reviewed native HTTP model request.

use crate::agent::{self, Mode, Provider};
use crate::{brand, fonts, icons, local_models, theme};
use eframe::egui::{self, Align2, Color32, FontId, Key, Modifiers, RichText, Sense, Stroke};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(windows)]
fn clipboard_image() -> Result<Option<PathBuf>, String> {
    use std::os::windows::process::CommandExt;
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Forge Room Early Preview")
        .join("agent-images");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let path = dir.join(format!("paste-{}-{stamp}.png", std::process::id()));
    let program = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    let command = "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $image=[System.Windows.Forms.Clipboard]::GetImage(); if ($null -eq $image) { exit 2 }; try { $image.Save($env:FORGE_ROOM_IMAGE_PATH, [System.Drawing.Imaging.ImageFormat]::Png) } finally { $image.Dispose() }";
    let status = std::process::Command::new(program)
        .args(["-NoProfile", "-Sta", "-Command", command])
        .env("FORGE_ROOM_IMAGE_PATH", &path)
        .creation_flags(0x0800_0000)
        .status()
        .map_err(|e| e.to_string())?;
    if status.code() == Some(2) {
        return Ok(None);
    }
    if !status.success() {
        let _ = std::fs::remove_file(&path);
        return Err("Could not read the clipboard image".into());
    }
    Ok(Some(path))
}

#[cfg(not(windows))]
fn clipboard_image() -> Result<Option<PathBuf>, String> {
    Ok(None)
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
struct Message {
    author: Author,
    text: String,
}

pub struct State {
    pub context: crate::ai_context::State,
    sent_context: String,
    controlled: crate::controlled_build::State,
    native: crate::native_models::panel::State,
    workflow: crate::ai_workflow::Workflow,
    isolated: crate::agent_workspaces::State,
    run_workspace: Option<PathBuf>,
    recovery: crate::review_checkpoints::State,
    messages: Vec<Message>,
    input: String,
    focus_input: bool,
    provider: Provider,
    mode: Mode,
    /// The model to use; empty is the provider's default.
    model: String,
    /// The model box is a free text field (a name that is not in the list).
    model_custom: bool,
    local_models: Vec<String>,
    local_model_status: String,
    local_models_rx: Option<std::sync::mpsc::Receiver<Result<Vec<String>, String>>>,
    /// How hard to think; empty is the provider's default.
    effort: String,
    /// The choices as last written to the settings file.
    saved: (String, String, String, String),
    /// The conversation the next message continues (Claude Code only).
    session: Option<String>,
    codex_session: Option<String>,
    run: Option<agent::Run>,
    run_provider: Option<Provider>,
    /// When the running agent was started.
    started: Option<std::time::Instant>,
    /// Where each program was found (Claude Code's, Codex's), looked up once.
    programs: Option<[Option<PathBuf>; 2]>,
    /// "Sign in" was clicked: the command for the terminal to run.
    login_requested: Option<&'static str>,
    terminal_requested: Option<String>,
    /// Full access was picked and waits for the user's confirmation.
    confirm_full: bool,
    image: Option<PathBuf>,
    image_is_temp: bool,
    workspace: Option<PathBuf>,
    chat_dirty: bool,
    chat_saved: std::time::Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            context: Default::default(),
            sent_context: String::new(),
            controlled: Default::default(),
            native: Default::default(),
            workflow: Default::default(),
            isolated: Default::default(),
            run_workspace: None,
            recovery: Default::default(),
            messages: Vec::new(),
            input: String::new(),
            focus_input: false,
            provider: Provider::Claude,
            mode: Mode::Edit,
            model: String::new(),
            model_custom: false,
            local_models: Vec::new(),
            local_model_status: String::new(),
            local_models_rx: None,
            effort: String::new(),
            saved: Default::default(),
            session: None,
            codex_session: None,
            run: None,
            run_provider: None,
            started: None,
            programs: None,
            login_requested: None,
            terminal_requested: None,
            confirm_full: false,
            image: None,
            image_is_temp: false,
            workspace: None,
            chat_dirty: false,
            chat_saved: std::time::Instant::now(),
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

#[derive(Serialize, Deserialize)]
struct StoredChat {
    #[serde(default)]
    plan: String,
    messages: Vec<Message>,
    session: Option<String>,
    #[serde(default)]
    codex_session: Option<String>,
}

fn chat_path(workspace: &Path) -> PathBuf {
    let name = workspace
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    let hash = name.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Forge Room Early Preview")
        .join("agent-chats")
        .join(format!("{hash:016x}.json"))
}

impl State {
    pub fn set_workspace(&mut self, workspace: &Path) {
        if self.workspace.as_deref() == Some(workspace) {
            return;
        }
        self.run = None;
        self.native.reset();
        self.controlled.reset();
        self.context.clear();
        self.sent_context.clear();
        self.run_provider = None;
        self.started = None;
        self.finish();
        self.messages.clear();
        self.isolated.new_chat();
        self.run_workspace = None;
        let selected_flow = self.workflow.flow;
        self.workflow = Default::default();
        self.workflow.flow = selected_flow;
        self.session = None;
        self.codex_session = None;
        self.workspace = Some(workspace.to_path_buf());
        self.isolated.load(workspace);
        self.run_workspace = None;
        let path = chat_path(workspace);
        if std::fs::metadata(&path).is_ok_and(|meta| meta.len() <= 2 * 1024 * 1024)
            && let Ok(bytes) = std::fs::read(path)
            && let Ok(saved) = serde_json::from_slice::<StoredChat>(&bytes)
        {
            self.workflow.plan = saved.plan.chars().take(64 * 1024).collect();
            self.messages = saved
                .messages
                .into_iter()
                .take(200)
                .filter(|m| m.text.len() <= 20_000)
                .collect();
            self.session = saved.session;
            self.codex_session = saved.codex_session;
        }
        self.chat_dirty = false;
        self.chat_saved = std::time::Instant::now();
    }

    fn save_chat(&mut self) {
        if !self.chat_dirty {
            return;
        }
        let Some(workspace) = &self.workspace else {
            return;
        };
        let path = chat_path(workspace);
        let messages = self
            .messages
            .iter()
            .rev()
            .take(200)
            .rev()
            .map(|m| Message {
                author: m.author,
                text: m.text.chars().take(20_000).collect(),
            })
            .collect();
        let mut saved = StoredChat {
            plan: self.workflow.plan.clone(),
            messages,
            session: self.session.clone(),
            codex_session: self.codex_session.clone(),
        };
        let bytes = loop {
            let Ok(bytes) = serde_json::to_vec(&saved) else {
                return;
            };
            if bytes.len() <= 2 * 1024 * 1024 {
                break bytes;
            }
            if saved.messages.is_empty() {
                return;
            }
            saved.messages.remove(0);
        };
        if let Some(parent) = path.parent()
            && std::fs::create_dir_all(parent).is_ok()
            && std::fs::write(path, bytes).is_ok()
        {
            self.chat_dirty = false;
            self.chat_saved = std::time::Instant::now();
        }
    }

    pub fn finish(&mut self) {
        self.save_chat();
        self.clear_image();
    }
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
        if let Some(config) = &settings.native_model
            && config.validate().is_ok()
        {
            state.native.config = config.clone();
        }
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
            if agent::valid_model(&self.model) {
                self.model.clone()
            } else {
                String::new()
            },
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
    pub fn set_active_file(&mut self, path: Option<PathBuf>) {
        self.workflow.active_file = path;
    }
    pub fn take_review_request(&mut self) -> Option<bool> {
        let apply = std::mem::take(&mut self.workflow.apply_requested);
        let undo = std::mem::take(&mut self.workflow.undo_requested);
        if apply {
            Some(false)
        } else if undo {
            Some(true)
        } else {
            None
        }
    }
    pub fn take_checkpoint_request(&mut self) -> bool {
        std::mem::take(&mut self.recovery.restore_requested)
    }
    pub fn restore_checkpoint(&mut self, root: &Path, dirty: &[PathBuf]) -> Result<usize, String> {
        if self.busy() {
            return Err("Stop the agent before recovery".into());
        }
        self.recovery.restore_selected(root, dirty)
    }
    pub fn apply_review(
        &mut self,
        root: &Path,
        dirty: &[PathBuf],
        undo: bool,
    ) -> Result<(), String> {
        if self.busy() {
            return Err("Stop the running agent before applying edits".into());
        }
        if self.workflow.flow == crate::ai_workflow::Flow::Controlled && !undo {
            self.controlled.guard_apply(root, &self.native.config)?;
        }
        let approved_files = self
            .workflow
            .preview
            .as_ref()
            .map(|p| p.files.clone())
            .unwrap_or_default();
        self.workflow.apply(root, dirty, undo)?;
        if self.controlled.active() {
            self.controlled.after_apply(root, &approved_files, undo);
        }
        self.note(if undo {
            "Last reviewed proposal was undone"
        } else {
            "Reviewed proposal applied; guarded undo is available in this chat"
        });
        Ok(())
    }
    pub fn take_login_request(&mut self) -> Option<&'static str> {
        self.login_requested.take()
    }

    pub fn take_workspace_open_request(&mut self) -> Option<PathBuf> {
        self.isolated.open_requested.take()
    }
    pub fn take_terminal_request(&mut self) -> Option<String> {
        let command = self.terminal_requested.take()?;
        if self.isolated.creating() {
            self.fail("Wait for the isolated workspace to finish creating");
            return None;
        }
        if self.isolated.enabled && self.workflow.flow == crate::ai_workflow::Flow::Direct {
            let root = self.workspace.as_deref()?;
            match self.isolated.target(root) {
                Ok(Some(path)) => {
                    #[cfg(windows)]
                    {
                        let path = path.to_string_lossy().replace('\'', "''");
                        Some(format!(
                            "try {{ Set-Location -LiteralPath '{path}' -ErrorAction Stop; {command} }} catch {{ Write-Error $_ }}"
                        ))
                    }
                    #[cfg(not(windows))]
                    {
                        let path = path.to_string_lossy().replace('\'', "'\\''");
                        Some(format!("cd -- '{path}' && {command}"))
                    }
                }
                Ok(None) => Some(command),
                Err(error) => {
                    self.fail(error);
                    None
                }
            }
        } else {
            Some(command)
        }
    }

    pub fn context_locked(&self) -> bool {
        self.controlled.active()
    }
    pub fn busy(&self) -> bool {
        self.run.is_some() || self.native.busy() || self.controlled.busy()
    }
    fn provider_name(&self) -> &str {
        if self.native.enabled {
            "Native model"
        } else {
            self.provider.name()
        }
    }
    fn review_native(&mut self, context: &egui::Context, cwd: Option<&Path>, trusted: bool) {
        let had_review = self.native.review.is_some();
        let Some(review) = self.native.decision(context) else {
            if had_review && self.native.review.is_none() {
                self.workflow.run_flow = None;
                self.controlled.cancel_review();
            }
            return;
        };
        self.accept_native(review, context, cwd, trusted);
    }
    fn accept_native(
        &mut self,
        review: crate::native_models::panel::Review,
        context: &egui::Context,
        cwd: Option<&Path>,
        trusted: bool,
    ) {
        if !trusted
            || cwd.and_then(|p| p.canonicalize().ok()).as_ref() != Some(&review.root)
            || self.workflow.flow != review.flow
            || self.native.config != review.prepared.config
        {
            self.workflow.run_flow = None;
            self.fail("Workspace, trust or flow changed; review a fresh request");
            self.controlled.cancel_review();
            return;
        }
        let controlled = review.flow == crate::ai_workflow::Flow::Controlled;
        let current = if controlled {
            self.controlled
                .current_prompt(&review.root, &self.native.config)
        } else {
            self.context
                .augment(&review.root, &review.instruction)
                .and_then(|instruction| self.workflow.prepare(&review.root, &instruction))
        };
        if current.as_ref().ok() != Some(&review.prepared.prompt) {
            self.workflow.run_flow = None;
            self.fail("Attached files, selected context or plan changed; review a fresh request before sending");
            self.controlled.cancel_review();
            return;
        }
        let wake = context.clone();
        let prompt = review.prepared.prompt.clone();
        let reservation = if controlled {
            match self.controlled.reserve(
                &review.root,
                &self.native.config,
                serde_json::to_vec(&review.prepared.body).map_or(usize::MAX, |b| b.len()),
            ) {
                Ok(r) => Some(r),
                Err(error) => {
                    self.workflow.run_flow = None;
                    self.fail(error);
                    return;
                }
            }
        } else {
            None
        };
        match crate::native_models::start(review.prepared, move || wake.request_repaint()) {
            Ok(run) => {
                if let Some(reservation) = reservation {
                    self.controlled.commit(reservation);
                }
                self.native.run = Some(run);
                self.native.inflight = Some(prompt);
                if self.input.trim() == review.instruction {
                    self.input.clear();
                }
                self.messages.push(Message {
                    author: Author::User,
                    text: review.instruction,
                });
                self.chat_dirty = true;
                self.started = Some(std::time::Instant::now());
                self.focus_input = true;
                self.run_provider = None;
                self.run_workspace = None;
            }
            Err(error) => {
                if controlled {
                    self.controlled.cancel_review();
                }
                self.workflow.run_flow = None;
                self.fail(error);
            }
        }
    }

    fn refresh_local_models(&mut self, context: &egui::Context) {
        if self.local_models_rx.is_some() {
            return;
        }
        let provider = self.provider;
        if !matches!(provider, Provider::Ollama | Provider::LmStudio) {
            return;
        }
        self.local_model_status = format!("Checking {} models…", provider.name());
        let (sender, receiver) = std::sync::mpsc::channel();
        self.local_models_rx = Some(receiver);
        let context = context.clone();
        std::thread::spawn(move || {
            let _ = sender.send(local_models::discover(provider));
            context.request_repaint();
        });
    }

    fn poll_local_models(&mut self) {
        let Some(receiver) = &self.local_models_rx else {
            return;
        };
        let Ok(result) = receiver.try_recv() else {
            return;
        };
        self.local_models_rx = None;
        match result {
            Ok(models) => {
                self.local_model_status = if models.is_empty() {
                    "No local models found; download or load one in the provider".into()
                } else {
                    format!("{} local model(s) available", models.len())
                };
                if models.iter().any(|model| model == &self.model) {
                    self.model_custom = false;
                }
                self.local_models = models;
            }
            Err(error) => {
                self.local_models.clear();
                self.local_model_status = error;
            }
        }
    }

    fn fail(&mut self, text: impl Into<String>) {
        self.messages.push(Message {
            author: Author::Error,
            text: text.into(),
        });
        self.chat_dirty = true;
    }

    fn note(&mut self, text: impl Into<String>) {
        self.messages.push(Message {
            author: Author::Note,
            text: text.into(),
        });
        self.chat_dirty = true;
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
        if self.isolated.creating() {
            self.fail(
                "Wait for the isolated workspace to finish creating before starting an agent",
            );
            return;
        }
        if (text.is_empty() && self.image.is_none()) || self.busy() {
            return;
        }
        if self.image.is_some() && self.provider != Provider::Codex {
            self.fail("Image input currently requires Codex. Choose Codex or remove the image.");
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
        let context_identity = match self.context.augment(cwd, "") {
            Ok(identity) => identity,
            Err(error) => {
                self.fail(error);
                return;
            }
        };
        if context_identity != self.sent_context {
            self.native.history.clear();
            self.session = None;
            self.codex_session = None;
            self.sent_context = context_identity;
            self.note("Selected context changed; provider conversation history starts fresh. Previously sent data may remain with the provider.");
        }
        let instruction = match self.context.augment(cwd, &text) {
            Ok(instruction) => instruction,
            Err(error) => {
                self.fail(error);
                return;
            }
        };
        if self.native.enabled {
            if self.image.is_some() {
                self.fail("Native model connections currently accept text only; remove the image");
                return;
            }
            let controlled = self.workflow.flow == crate::ai_workflow::Flow::Controlled;
            let controlled_instruction = if self.controlled.active() {
                text.as_str()
            } else {
                instruction.as_str()
            };
            let prompt = if controlled {
                self.native.history.clear();
                crate::native_models::prepare(&self.native.config, String::new(), &[]).and_then(
                    |_| {
                        self.controlled.request(
                            cwd,
                            &self.native.config,
                            &self.workflow.sources,
                            controlled_instruction,
                        )
                    },
                )
            } else {
                self.workflow.prepare(cwd, &instruction)
            };
            match prompt
                .and_then(|prompt| self.native.prepare(cwd, self.workflow.flow, prompt, text))
            {
                Ok(()) => {
                    if controlled {
                        self.workflow.run_flow = Some(crate::ai_workflow::Flow::Controlled);
                        self.workflow.output.clear();
                        if let Some(review) = &mut self.native.review {
                            match self.controlled.reserve(
                                cwd,
                                &self.native.config,
                                serde_json::to_vec(&review.prepared.body)
                                    .map_or(usize::MAX, |b| b.len()),
                            ) {
                                Ok(r) => {
                                    review.budget =
                                        Some(crate::controlled_build::State::reservation_note(&r))
                                }
                                Err(error) => {
                                    self.native.review = None;
                                    self.workflow.run_flow = None;
                                    self.fail(error);
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    self.workflow.run_flow = None;
                    self.fail(error);
                }
            }
            return;
        }
        if self.workflow.flow == crate::ai_workflow::Flow::Controlled {
            self.fail("Controlled build requires the native model connection");
            return;
        }
        let provider = self.provider;
        // A model name that is not a plain word is not passed on: say so instead of quietly using
        // the default.
        if !self.model.is_empty() && !agent::valid_model(&self.model) {
            self.note(format!(
                "\"{}\" is not a valid model name (letters, digits and . - _ : / [ ]), so the default                  model is used.",
                self.model
            ));
        }
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
        let instruction = if text.is_empty() {
            "Describe this image."
        } else {
            text.as_str()
        };
        let instruction = match self.context.augment(cwd, instruction) {
            Ok(value) => value,
            Err(error) => {
                self.fail(error);
                return;
            }
        };
        let prompt = match self.workflow.prepare(cwd, &instruction) {
            Ok(prompt) => prompt,
            Err(error) => {
                self.fail(error);
                return;
            }
        };
        let isolated = if self.workflow.flow == crate::ai_workflow::Flow::Direct {
            match self.isolated.target(cwd) {
                Ok(target) => target,
                Err(error) => {
                    self.fail(error);
                    return;
                }
            }
        } else {
            None
        };
        let request = agent::Request {
            provider,
            model: self.model.clone(),
            effort: self.effort.clone(),
            mode: self.workflow.flow.mode(self.mode),
            prompt,
            cwd: isolated.clone().unwrap_or_else(|| cwd.to_path_buf()),
            session: if self.workflow.flow != crate::ai_workflow::Flow::Direct {
                None
            } else if let Some(path) = &isolated {
                self.isolated.session(path, provider)
            } else {
                match provider {
                    Provider::Claude => self.session.clone(),
                    Provider::Codex => self.codex_session.clone(),
                    _ => None,
                }
            },
            image: self.image.clone(),
        };
        let wake = context.clone();
        match agent::start(&program, &request, move || wake.request_repaint()) {
            Ok(run) => {
                self.run = Some(run);
                self.run_workspace = isolated;
                if let Some(path) = &self.run_workspace {
                    self.note(format!(
                        "Agent execution folder: {} (isolated Git checkout)",
                        path.display()
                    ));
                }
                self.run_provider = Some(provider);
                self.started = Some(std::time::Instant::now());
                self.messages.push(Message {
                    author: Author::User,
                    text: if self.image.is_some() {
                        format!("{text}\n[image attached]")
                    } else {
                        text
                    },
                });
                self.chat_dirty = true;
                self.input.clear();
                self.focus_input = true;
            }
            Err(error) => self.fail(error),
        }
    }

    /// Take what the running agent has said since the last frame. True when the run just ended
    /// (files may have changed, so the caller refreshes what shows them).
    pub fn poll(&mut self) -> bool {
        let validation_finished = if let Some(note) = self.controlled.poll() {
            self.note(note);
            true
        } else {
            false
        };
        for note in self.isolated.poll() {
            self.note(note);
        }
        if self.chat_dirty && self.chat_saved.elapsed() >= std::time::Duration::from_secs(2) {
            self.save_chat();
        }
        let native = self.native.run.is_some();
        let mut finished = false;
        let mut succeeded = false;
        let events = if let Some(run) = &mut self.run {
            run.poll()
        } else if let Some(run) = &mut self.native.run {
            run.poll()
        } else {
            return validation_finished;
        };
        if !events.is_empty() {
            self.chat_dirty = true;
        }
        for event in events {
            if let agent::Event::Text(text) = &event {
                if native {
                    self.workflow.output.push_str(text);
                } else {
                    self.workflow.output = if text.len() <= 1024 * 1024 {
                        text.clone()
                    } else {
                        String::new()
                    };
                }
            }
            match event {
                agent::Event::Text(text) => match self.messages.last_mut() {
                    Some(last) if last.author == Author::Assistant => {
                        if !native {
                            last.text.push_str("\n\n");
                        }
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
                    if self.workflow.run_flow == Some(crate::ai_workflow::Flow::Direct)
                        && let Some(path) = &self.run_workspace
                        && let Some(provider) = self.run_provider
                    {
                        self.isolated
                            .record_session(path.clone(), provider, Some(id));
                    } else if self.workflow.run_flow == Some(crate::ai_workflow::Flow::Direct) {
                        match self.run_provider {
                            Some(Provider::Claude) => self.session = Some(id),
                            Some(Provider::Codex) => self.codex_session = Some(id),
                            _ => {}
                        }
                    }
                }
                agent::Event::Done { cost, error } => {
                    succeeded = error.is_none();
                    finished = true;
                    match error {
                        Some(error) if error == "Stopped" => self.note("Stopped"),
                        Some(error) => {
                            self.fail(error);
                            // A conversation that failed (an expired or unknown session) is not
                            // continued by the next message: that would fail the same way for ever.
                            if let Some(path) = &self.run_workspace
                                && let Some(provider) = self.run_provider
                            {
                                self.isolated.record_session(path.clone(), provider, None);
                            }
                            let old = if self.run_workspace.is_none()
                                && self.workflow.run_flow == Some(crate::ai_workflow::Flow::Direct)
                            {
                                match self.run_provider {
                                    Some(Provider::Claude) => self.session.take(),
                                    Some(Provider::Codex) => self.codex_session.take(),
                                    _ => None,
                                }
                            } else {
                                None
                            };
                            if old.is_some() {
                                self.note(
                                    "The conversation could not be continued; your next message                                      starts a new one.",
                                );
                            }
                        }
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
            let controlled = self.workflow.run_flow == Some(crate::ai_workflow::Flow::Controlled);
            if native {
                if !controlled {
                    self.native.complete(succeeded, &self.workflow.output);
                } else {
                    self.native.inflight = None;
                }
                self.native.run = None;
            }
            if succeeded && controlled {
                let controlled_output = self.workflow.output.clone();
                let result = self
                    .workspace
                    .clone()
                    .ok_or_else(|| "Workspace changed".to_owned())
                    .and_then(|root| {
                        self.controlled.receive(
                            &root,
                            &self.native.config,
                            &controlled_output,
                            &mut self.workflow,
                        )
                    });
                self.workflow.run_flow = None;
                match result {
                    Ok(note) => self.note(note),
                    Err(error) => {
                        self.controlled.record_error(&error);
                        self.fail(error);
                    }
                }
            } else if succeeded {
                match self.workflow.complete() {
                    Ok(Some(note)) => self.note(note),
                    Err(error) => self.fail(error),
                    _ => {}
                }
            } else {
                if controlled {
                    self.controlled
                        .record_error("Model request failed or was stopped");
                }
                self.workflow.run_flow = None;
            }
            self.run = None;
            self.run_provider = None;
            self.run_workspace = None;
            self.started = None;
            self.clear_image();
            self.save_chat();
        }
        finished || validation_finished
    }

    /// Stop the agent that is running.
    fn stop(&mut self) {
        if self.workflow.flow == crate::ai_workflow::Flow::Controlled {
            self.controlled.reset();
            self.workflow.preview = None;
            self.workflow.run_flow = None;
        }
        if let Some(run) = &self.run {
            run.cancel();
        }
        if let Some(run) = &mut self.native.run {
            run.cancel();
        }
        if self.native.review.take().is_some() {
            self.workflow.run_flow = None;
        }
    }
    pub fn restrict(&mut self) {
        self.context.clear();
        self.sent_context.clear();
        self.controlled.reset();
        self.stop();
        self.run = None;
        self.native.reset();
        self.started = None;
        self.workflow.run_flow = None;
        self.note("AI requests stopped after folder trust was removed");
    }

    /// Forget the conversation (and stop the agent): the next message starts fresh.
    pub fn new_chat(&mut self) {
        self.context.clear();
        self.sent_context.clear();
        self.controlled.reset();
        // Dropping the run stops the agent and its events no longer reach the new chat.
        self.run = None;
        self.native.reset();
        self.run_provider = None;
        self.started = None;
        self.messages.clear();
        self.isolated.new_chat();
        self.run_workspace = None;
        let selected_flow = self.workflow.flow;
        self.workflow = Default::default();
        self.workflow.flow = selected_flow;
        self.session = None;
        self.codex_session = None;
        self.confirm_full = false;
        self.clear_image();
        self.chat_dirty = true;
        self.save_chat();
    }

    fn clear_image(&mut self) {
        if self.image_is_temp
            && let Some(path) = &self.image
        {
            let _ = std::fs::remove_file(path);
        }
        self.image = None;
        self.image_is_temp = false;
    }

    fn attach_image(&mut self, path: PathBuf, temporary: bool) {
        let valid = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            ["png", "jpg", "jpeg", "webp"].contains(&e.to_ascii_lowercase().as_str())
        }) && std::fs::metadata(&path)
            .is_ok_and(|m| m.is_file() && m.len() <= 10 * 1024 * 1024);
        if !valid {
            if temporary {
                let _ = std::fs::remove_file(path);
            }
            self.fail("Choose a PNG, JPEG, or WebP image under 10 MB.");
            return;
        }
        self.clear_image();
        self.image = Some(path);
        self.image_is_temp = temporary;
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
            let busy = state.busy();
            for note in if !state.native.enabled {
                state.isolated.show(ui, cwd, trusted, busy)
            } else {
                Vec::new()
            } {
                state.note(note);
            }
            state.recovery.show(ui, cwd, trusted, busy);
            let busy = state.busy();
            state
                .context
                .show(ui, cwd, trusted && !busy && !state.controlled.active());
            let old_plan = state.workflow.plan.clone();
            let old_flow = state.workflow.flow;
            for note in state.workflow.show(
                ui,
                cwd,
                busy,
                trusted,
                !state.native.enabled,
                state.controlled.active(),
            ) {
                state.note(note);
            }
            if state.workflow.flow != old_flow
                && (old_flow == crate::ai_workflow::Flow::Controlled
                    || state.workflow.flow == crate::ai_workflow::Flow::Controlled)
            {
                state.controlled.reset();
                state.workflow.preview = None;
            }
            state.controlled.reconcile_preview(&mut state.workflow);
            if state.native.enabled && state.workflow.flow == crate::ai_workflow::Flow::Controlled {
                state.controlled.show(
                    ui,
                    cwd,
                    &state.native.config,
                    trusted,
                    state.busy(),
                    &mut state.workflow,
                );
                if std::mem::take(&mut state.controlled.continue_requested) {
                    if state.input.trim().is_empty() {
                        state.input = "Continue controlled task.".into();
                    }
                    state.send(ui.ctx(), cwd, trusted);
                }
            }
            if old_plan != state.workflow.plan {
                state.chat_dirty = true;
            }
            if std::mem::take(&mut state.workflow.generate_requested) {
                state.input = "Implement the reviewed plan in the attached files.".into();
                state.send(ui.ctx(), cwd, trusted);
            }
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
                            None if state.native.review.is_some() => {
                                "Awaiting request review; nothing has been sent".into()
                            }
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
    state.review_native(ui.ctx(), cwd, trusted);
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
    let busy = state.busy();
    let before = state.native.enabled;
    ui.add_enabled_ui(!busy, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut state.native.enabled, false, "Installed CLI agent");
            ui.selectable_value(&mut state.native.enabled, true, "Native model connection");
        });
    });
    if state.native.enabled != before {
        state.controlled.reset();
        state.workflow.output.clear();
        state.workflow.preview = None;
        if state.native.enabled && state.workflow.flow == crate::ai_workflow::Flow::Direct {
            state.workflow.flow = crate::ai_workflow::Flow::Ask;
        }
        if !state.native.enabled && state.workflow.flow == crate::ai_workflow::Flow::Controlled {
            state.workflow.flow = crate::ai_workflow::Flow::Ask;
        }
    }
    if state.native.enabled {
        state.native.show(ui, busy);
        if ui
            .add_enabled(!busy, egui::Button::new("New chat"))
            .clicked()
        {
            state.new_chat();
        }
        return;
    }
    state.poll_local_models();
    if matches!(state.provider, Provider::Ollama | Provider::LmStudio)
        && state.local_model_status.is_empty()
    {
        state.refresh_local_models(ui.ctx());
    }
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
                state.local_models.clear();
                state.local_model_status.clear();
                state.local_models_rx = None;
                if !agent::efforts(state.provider).contains(&state.effort.as_str()) {
                    state.effort.clear();
                }
                state.refresh_local_models(ui.ctx());
            }
            let shown = if state.model_custom {
                "Custom model".to_string()
            } else if state.local_models.iter().any(|model| model == &state.model) {
                state.model.clone()
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
                    for model in &state.local_models {
                        if ui.selectable_label(state.model == *model, model).clicked() {
                            state.model = model.clone();
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
            if matches!(state.provider, Provider::Ollama | Provider::LmStudio) {
                if ui.small_button("Refresh models").clicked() {
                    state.refresh_local_models(ui.ctx());
                }
                ui.small(&state.local_model_status);
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
            if ui
                .add_enabled(!state.busy(), egui::Button::new("Interactive direct agent"))
                .on_hover_text(
                    "Opens the installed agent in a terminal so you can answer permission prompts",
                )
                .clicked()
            {
                if let Some(path) = state.program(state.provider) {
                    let escaped = path.to_string_lossy().replace('\'', "''");
                    let mut command = format!("& '{escaped}'");
                    match state.provider {
                        Provider::Ollama => command.push_str(" --oss --local-provider ollama"),
                        Provider::LmStudio => command.push_str(" --oss --local-provider lmstudio"),
                        _ => {}
                    }
                    state.terminal_requested = Some(command);
                } else {
                    state.fail("Install this agent before opening it in a terminal.");
                }
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
    // The length of the opening fence and the lines inside it.
    let mut code: Option<(usize, Vec<&str>)> = None;
    let flush = |paragraph: &mut Vec<&str>, result: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            result.push(Block::Paragraph(paragraph.join("\n")));
            paragraph.clear();
        }
    };
    for line in text.lines() {
        if let Some((length, lines)) = &mut code {
            // Only a line of nothing but backticks, at least as many as opened it, closes the block.
            let closing = line.trim();
            if closing.len() >= *length && closing.chars().all(|c| c == '`') {
                result.push(Block::Code(lines.join("\n")));
                code = None;
            } else {
                lines.push(line);
            }
            continue;
        }
        let trimmed = line.trim_start();
        // A tab counts as two spaces; a level is two spaces.
        let depth = line[..line.len() - trimmed.len()]
            .chars()
            .map(|c| if c == '\t' { 2 } else { 1 })
            .sum::<usize>()
            / 2;
        if let Some(length) = fence_length(trimmed) {
            flush(&mut paragraph, &mut result);
            code = Some((length, Vec::new()));
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
            result.push(Block::Item(depth, "•".into(), item.trim().to_string()));
        } else if let Some((number, rest)) = trimmed
            .split_once(". ")
            .or_else(|| trimmed.split_once(") "))
            && !number.is_empty()
            && number.len() <= 3
            && number.chars().all(|c| c.is_ascii_digit())
        {
            flush(&mut paragraph, &mut result);
            result.push(Block::Item(
                depth,
                format!("{number}."),
                rest.trim().to_string(),
            ));
        } else {
            paragraph.push(line);
        }
    }
    // An unclosed fence is still shown as code.
    if let Some((_, lines)) = code {
        result.push(Block::Code(lines.join("\n")));
    }
    flush(&mut paragraph, &mut result);
    result
}

/// The length of the backtick fence that opens a code block on this line, when it is one: three or
/// more backticks and an info string without backticks (so "```npm install```" is not a fence).
fn fence_length(trimmed: &str) -> Option<usize> {
    let length = trimmed.chars().take_while(|c| *c == '`').count();
    (length >= 3 && !trimmed[length..].contains('`')).then_some(length)
}

/// How a span of inline text is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Span {
    bold: bool,
    code: bool,
}

/// Where the `**` that closes a bold span in `after` is: the first one that follows a character that
/// is not a space (and is not the very start).
fn closing_stars(after: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = after[from..].find("**") {
        let at = from + found;
        if at > 0
            && after[..at]
                .chars()
                .next_back()
                .is_some_and(|c| !c.is_whitespace())
        {
            return Some(at);
        }
        from = at + 2;
    }
    None
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
    // The character before `rest`, to tell an opening `**` from a stray one (`src/**/*.rs`).
    let mut previous: Option<char> = None;
    while !rest.is_empty() {
        let may_open = previous.is_none_or(|c| c.is_whitespace() || "([{\"'".contains(c));
        if may_open
            && let Some(after) = rest.strip_prefix("**")
            && after.chars().next().is_some_and(|c| !c.is_whitespace())
            && let Some(end) = closing_stars(after)
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
            previous = None;
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
            previous = None;
        } else {
            let mut chars = rest.chars();
            if let Some(c) = chars.next() {
                plain.push(c);
                previous = Some(c);
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
    if !state.native.enabled
        && focused
        && ui.input(|input| input.modifiers.command && input.key_pressed(Key::V))
    {
        match clipboard_image() {
            Ok(Some(path)) => state.attach_image(path, true),
            Ok(None) => {}
            Err(error) => state.fail(error),
        }
    }
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
            let hint = if state.native.review.is_some() {
                "Review the request before sending (Esc cancels)".into()
            } else if state.busy() {
                format!("{} is working… (Esc stops it)", state.provider_name())
            } else if state.native.enabled {
                "Ask the native model using selected context (Enter reviews the request)".into()
            } else {
                format!(
                    "Ask {} to work in this folder…  (Enter sends, Shift+Enter adds a line)",
                    state.provider_name()
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
            if let Some(path) = &state.image {
                let name = path
                    .file_name()
                    .map_or("Image", |name| name.to_str().unwrap_or("Image"))
                    .to_owned();
                ui.horizontal(|ui| {
                    ui.label(format!("Image: {name}"));
                    if ui.small_button("Remove").clicked() {
                        state.clear_image();
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !state.native.enabled,
                        egui::Button::new("Attach image…").small(),
                    )
                    .clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                        .pick_file()
                {
                    state.attach_image(path, false);
                }
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
                    let ready = (!state.input.trim().is_empty() || state.image.is_some())
                        && !busy
                        && !state.isolated.creating();
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
                        RichText::new(state.provider_name())
                            .size(13.0)
                            .color(theme::text_dim()),
                    );
                    if !state.native.enabled && state.workflow.flow.mode(state.mode) == Mode::Full {
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
    fn chat_returns_for_its_folder_and_new_chat_clears_it() {
        let folder = std::env::temp_dir().join(format!("forge-chat-{}", std::process::id()));
        let mut first = State::default();
        first.set_workspace(&folder);
        first.note("saved for this folder");
        first.workflow.plan = "Reviewed implementation plan".into();
        first.finish();
        let mut second = State::default();
        second.set_workspace(&folder);
        assert_eq!(second.messages.len(), 1);
        assert_eq!(second.messages[0].text, "saved for this folder");
        assert_eq!(second.workflow.plan, "Reviewed implementation plan");
        second.new_chat();
        let mut third = State::default();
        third.set_workspace(&folder);
        assert!(third.messages.is_empty());
        assert!(third.workflow.plan.is_empty());
        let _ = std::fs::remove_file(chat_path(&folder));
    }

    #[test]
    fn older_chat_files_load_without_a_plan() {
        let chat: StoredChat = serde_json::from_str(r#"{"messages":[],"session":null}"#).unwrap();
        assert!(chat.plan.is_empty() && chat.codex_session.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn direct_agent_runs_in_its_worktree_without_reusing_primary_conversation() {
        let root =
            std::env::temp_dir().join(format!("forge-isolated-agent-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        crate::git::run(&root, &["init"]).unwrap();
        crate::git::run(&root, &["config", "core.autocrlf", "false"]).unwrap();
        crate::git::run(&root, &["config", "user.name", "Test"]).unwrap();
        crate::git::run(&root, &["config", "user.email", "test@example.invalid"]).unwrap();
        std::fs::write(root.join("main.txt"), "committed\n").unwrap();
        crate::git::run(&root, &["add", "main.txt"]).unwrap();
        crate::git::run(&root, &["commit", "-m", "base"]).unwrap();
        std::fs::write(root.join("main.txt"), "primary dirty\n").unwrap();
        let workspace = crate::agent_workspaces::create(&root, "Direct isolated task").unwrap();
        let program = root.join("isolated-mock.cmd");
        std::fs::write(&program,"@echo off\r\necho %* > isolated-args.txt\r\necho agent isolated edit> main.txt\r\necho {\"type\":\"thread.started\",\"thread_id\":\"isolated-thread\"}\r\necho {\"type\":\"turn.completed\"}\r\n").unwrap();
        let mut state = State::default();
        state.set_workspace(&root);
        state.provider = Provider::Codex;
        state.mode = Mode::Edit;
        state.programs = Some([None, Some(program)]);
        state.codex_session = Some("primary-thread".into());
        state.isolated.adopt(workspace.clone()).unwrap();
        state.input = "Work only in the task checkout".into();
        state.send(&egui::Context::default(), Some(&root), true);
        let start = std::time::Instant::now();
        while state.busy() && start.elapsed() < std::time::Duration::from_secs(8) {
            state.poll();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(!state.busy());
        assert_eq!(
            std::fs::read_to_string(root.join("main.txt")).unwrap(),
            "primary dirty\n"
        );
        assert_eq!(
            std::fs::read_to_string(workspace.folder.join("main.txt"))
                .unwrap()
                .trim(),
            "agent isolated edit"
        );
        assert_eq!(state.codex_session.as_deref(), Some("primary-thread"));
        assert_eq!(
            state
                .isolated
                .session(&workspace.folder, Provider::Codex)
                .as_deref(),
            Some("isolated-thread")
        );
        assert!(
            !std::fs::read_to_string(workspace.folder.join("isolated-args.txt"))
                .unwrap()
                .contains("resume")
        );
        state.new_chat();
        assert!(
            state
                .isolated
                .session(&workspace.folder, Provider::Codex)
                .is_none()
        );
        assert!(workspace.folder.exists());
        workspace.validate(&root).unwrap();
        crate::git::run(
            &root,
            &[
                "worktree",
                "remove",
                "--force",
                &workspace.folder.to_string_lossy(),
            ],
        )
        .unwrap();
        let _ = std::fs::remove_file(chat_path(&root));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_request_waits_for_review_then_requires_apply_and_keeps_cli_history_private() {
        use crate::ai_workflow::Flow;
        let root =
            std::env::temp_dir().join(format!("forge-native-reviewed-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("main.txt");
        std::fs::write(&file, "before\n").unwrap();
        let proposal =
            serde_json::json!({"summary":"change","edits":[{"path":"main.txt","after":"after\n"}]})
                .to_string();
        let response=serde_json::json!({"choices":[{"message":{"content":proposal},"finish_reason":"stop"}]}).to_string();
        let (config, server) = crate::native_models::tests::mock(
            crate::native_models::Protocol::Chat,
            response,
            false,
            "200 OK",
        );
        let mut state = State::default();
        state.set_workspace(&root);
        state.native.enabled = true;
        state.native.config = config;
        state.programs = Some([None, None]);
        state.codex_session = Some("private-cli-session".into());
        state.messages.push(Message {
            author: Author::User,
            text: "PRIVATE CLI HISTORY".into(),
        });
        state.workflow.flow = Flow::Propose;
        state.workflow.plan = "Update the attached file".into();
        state.workflow.attach(&root, &file).unwrap();
        state.input = "Implement reviewed plan".into();
        let context = egui::Context::default();
        state.send(&context, Some(&root), true);
        assert!(state.native.run.is_none());
        assert!(state.native.review.is_some());
        assert!(!state.input.is_empty());
        let review = state.native.review.take().unwrap();
        assert!(
            !review
                .prepared
                .body
                .to_string()
                .contains("PRIVATE CLI HISTORY")
        );
        state.input = "Newer draft typed while reviewing".into();
        state.accept_native(review, &context, Some(&root), true);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while state.busy() {
            assert!(std::time::Instant::now() < deadline);
            state.poll();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(state.workflow.preview.is_some());
        assert_eq!(state.input, "Newer draft typed while reviewing");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "before\n");
        assert_eq!(state.codex_session.as_deref(), Some("private-cli-session"));
        state.apply_review(&root, &[], false).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "after\n");
        state.apply_review(&root, &[], true).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "before\n");
        let request = server.join().unwrap();
        assert!(
            request.body["messages"][0]["content"]
                .as_str()
                .unwrap()
                .contains("before\\n")
        );
        state.new_chat();
        assert!(state.native.history.is_empty());
    }
    #[test]
    fn native_review_rejects_changed_attachments_and_restricted_workspaces_before_network() {
        use crate::ai_workflow::Flow;
        let root = std::env::temp_dir().join(format!("forge-native-stale-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("main.txt");
        std::fs::write(&file, "before").unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut state = State::default();
        state.set_workspace(&root);
        state.native.enabled = true;
        state.native.config.base_url = format!(
            "http://127.0.0.1:{}/v1",
            listener.local_addr().unwrap().port()
        );
        state.native.config.model = "test".into();
        state.workflow.flow = Flow::Ask;
        state.workflow.attach(&root, &file).unwrap();
        state.input = "Explain".into();
        let context = egui::Context::default();
        state.send(&context, Some(&root), false);
        assert!(state.native.review.is_none());
        state.send(&context, Some(&root), true);
        let review = state.native.review.take().unwrap();
        std::fs::write(&file, "newer").unwrap();
        state.accept_native(review, &context, Some(&root), true);
        assert!(!state.busy());
        assert!(state.messages.last().unwrap().text.contains("changed"));
        assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
        assert_eq!(state.input, "Explain");
        state.send(&context, Some(&root), true);
        assert!(state.native.review.is_some());
        state.restrict();
        assert!(!state.busy());
        assert!(state.native.history.is_empty());
    }
    #[cfg(windows)]
    #[test]
    fn read_only_provider_draft_waits_for_apply_and_keeps_direct_session_separate() {
        use crate::ai_workflow::Flow;
        let root = std::env::temp_dir().join(format!("forge-ai-pipeline-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("main.txt");
        std::fs::write(&file, "before\n").unwrap();
        let program = root.join("mock-codex.cmd");
        let proposal=serde_json::json!({"summary":"reviewed change","edits":[{"path":"main.txt","after":"after\n"}]}).to_string();
        let event=serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":proposal}}).to_string();
        std::fs::write(&program,format!("@echo off\r\necho %* > arguments.txt\r\necho {{\"type\":\"thread.started\",\"thread_id\":\"draft-thread\"}}\r\necho {event}\r\necho {{\"type\":\"turn.completed\"}}\r\n")).unwrap();
        let mut state = State::default();
        state.set_workspace(&root);
        state.provider = Provider::Codex;
        state.mode = Mode::Full;
        state.programs = Some([None, Some(program)]);
        state.codex_session = Some("direct-thread".into());
        state.workflow.flow = Flow::Propose;
        state.workflow.plan = "Update the attached file".into();
        state.workflow.attach(&root, &file).unwrap();
        state.input = "Implement the plan".into();
        state.send(&egui::Context::default(), Some(&root), true);
        let started = std::time::Instant::now();
        while state.busy() && started.elapsed() < std::time::Duration::from_secs(8) {
            state.poll();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(!state.busy(), "mock provider finished within its deadline");
        assert!(
            state.workflow.preview.is_some(),
            "draft produced a reviewable proposal"
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "before\n");
        let args = std::fs::read_to_string(root.join("arguments.txt")).unwrap();
        assert!(args.contains("--sandbox read-only"));
        assert!(!args.contains("danger-full-access") && !args.contains("resume"));
        assert_eq!(state.codex_session.as_deref(), Some("direct-thread"));
        state.apply_review(&root, &[], false).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "after\n");
        state.apply_review(&root, &[], true).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "before\n");
        let _ = std::fs::remove_file(chat_path(&root));
        std::fs::remove_dir_all(root).unwrap();
    }

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
        state.mode = Mode::Full;
        state.workflow.flow = crate::ai_workflow::Flow::Plan;
        state.new_chat();
        assert!(state.messages.is_empty() && state.session.is_none());
        assert!(state.workflow.flow == crate::ai_workflow::Flow::Plan);
        assert_eq!(state.workflow.flow.mode(state.mode), Mode::ReadOnly);
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

    #[test]
    fn fences_follow_their_length_and_stray_markers_stay_as_written() {
        // A longer fence holds a shorter one; a line that only mentions a fence does not close it.
        assert_eq!(
            blocks("Example:\n````\n```\n# Title\n```\nmore\n````\nafter"),
            vec![
                Block::Paragraph("Example:".into()),
                Block::Code("```\n# Title\n```\nmore".into()),
                Block::Paragraph("after".into()),
            ]
        );
        assert_eq!(
            blocks("```\nline one\n```rust is mentioned here\nline three\n```\nafter"),
            vec![
                Block::Code("line one\n```rust is mentioned here\nline three".into()),
                Block::Paragraph("after".into()),
            ]
        );
        // A one-line triple-backtick span is text, not a fence that swallows the rest.
        assert_eq!(
            blocks("```npm install```\n\nThen run it.\n\n- one"),
            vec![
                Block::Paragraph("```npm install```".into()),
                Block::Paragraph("Then run it.".into()),
                Block::Item(0, "•".into(), "one".into()),
            ]
        );
        // `**` needs text right inside it and a word break before it.
        let plain = |text: &str| spans(text).iter().all(|(_, span)| !span.bold);
        assert!(plain("compute 2 ** 3 and 4 ** 5"));
        assert!(plain("src/**/*.rs and tests/**/*.rs"));
        assert!(!plain("a **bold** word"));
        assert!(!plain("(**bold**)"));
        // Tabs nest lists, and `1)` is a numbered list too.
        assert_eq!(
            blocks("- a\n\t- b\n\t\t- c\n1) step"),
            vec![
                Block::Item(0, "•".into(), "a".into()),
                Block::Item(1, "•".into(), "b".into()),
                Block::Item(2, "•".into(), "c".into()),
                Block::Item(0, "1.".into(), "step".into()),
            ]
        );
    }
    #[test]
    fn selected_context_is_reviewed_revoked_and_never_restored_from_native_history() {
        let root =
            std::env::temp_dir().join(format!("forge-selected-context-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let response=serde_json::json!({"choices":[{"message":{"content":"answer"},"finish_reason":"stop"}]}).to_string();
        let (config, server) = crate::native_models::tests::mock(
            crate::native_models::Protocol::Chat,
            response,
            false,
            "200 OK",
        );
        let mut state = State::default();
        state.set_workspace(&root);
        state.native.enabled = true;
        state.native.config = config;
        state.native.config.history = true;
        state.workflow.flow = crate::ai_workflow::Flow::Ask;
        state
            .context
            .capture(
                &root,
                crate::ai_context::Kind::Selection,
                "selected draft".into(),
                "SELECTED_PRIVATE_DRAFT".into(),
                false,
            )
            .unwrap();
        state
            .context
            .capture(
                &root,
                crate::ai_context::Kind::Terminal,
                "unselected terminal".into(),
                "UNSELECTED_TERMINAL_SECRET".into(),
                true,
            )
            .unwrap();
        state.context.include(0, true).unwrap();
        state
            .native
            .history
            .push(serde_json::json!({"role":"user","content":"OLDER_CONTEXT"}));
        state.codex_session = Some("old-cli-conversation".into());
        state.input = "Explain context".into();
        let context = egui::Context::default();
        state.send(&context, Some(&root), true);
        let review = state.native.review.take().unwrap();
        let body = review.prepared.body.to_string();
        assert!(body.contains("SELECTED_PRIVATE_DRAFT"));
        assert!(!body.contains("UNSELECTED_TERMINAL_SECRET"));
        assert!(!body.contains("OLDER_CONTEXT"));
        assert!(state.codex_session.is_none());
        state.context.include(0, false).unwrap();
        state.accept_native(review, &context, Some(&root), true);
        assert!(state.native.run.is_none());
        state.context.include(0, true).unwrap();
        state.send(&context, Some(&root), true);
        let review = state.native.review.take().unwrap();
        state.accept_native(review, &context, Some(&root), true);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while state.busy() {
            assert!(std::time::Instant::now() < deadline);
            state.poll();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let captured = server.join().unwrap().body.to_string();
        assert!(captured.contains("SELECTED_PRIVATE_DRAFT"));
        assert!(!captured.contains("UNSELECTED_TERMINAL_SECRET"));
        assert!(!state.native.history.is_empty());
        state.context.include(0, false).unwrap();
        state.input = "Continue without snapshot".into();
        state.send(&context, Some(&root), true);
        assert!(
            !state
                .native
                .review
                .as_ref()
                .unwrap()
                .prepared
                .body
                .to_string()
                .contains("SELECTED_PRIVATE_DRAFT")
        );
        assert!(state.native.history.is_empty());
        state.new_chat();
        assert_eq!(state.context.augment(&root, "task").unwrap(), "task");
        let _ = std::fs::remove_file(chat_path(&root));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn cli_context_is_explicit_and_changing_it_starts_a_fresh_conversation() {
        let root = std::env::temp_dir().join(format!("forge-cli-context-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let program = root.join("mock-codex.cmd");
        std::fs::write(&program,"@echo off\r\npowershell.exe -NoProfile -NonInteractive -Command \"[IO.File]::WriteAllText('prompt.txt',[Console]::In.ReadToEnd())\"\r\necho {\"type\":\"turn.completed\"}\r\n").unwrap();
        let mut state = State::default();
        state.set_workspace(&root);
        state.provider = Provider::Codex;
        state.programs = Some([None, Some(program)]);
        state.workflow.flow = crate::ai_workflow::Flow::Ask;
        state
            .context
            .capture(
                &root,
                crate::ai_context::Kind::Skills,
                "skill".into(),
                "SELECTED_SKILL_INSTRUCTIONS".into(),
                false,
            )
            .unwrap();
        state
            .context
            .capture(
                &root,
                crate::ai_context::Kind::Terminal,
                "private terminal".into(),
                "UNSELECTED_PRIVATE_TERMINAL".into(),
                true,
            )
            .unwrap();
        state.context.include(0, true).unwrap();
        state.codex_session = Some("old-session".into());
        state.input = "Explain".into();
        state.send(&egui::Context::default(), Some(&root), true);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while state.busy() {
            assert!(std::time::Instant::now() < deadline);
            state.poll();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let prompt = std::fs::read_to_string(root.join("prompt.txt")).unwrap();
        assert!(prompt.contains("SELECTED_SKILL_INSTRUCTIONS"));
        assert!(!prompt.contains("UNSELECTED_PRIVATE_TERMINAL"));
        assert!(state.codex_session.is_none());
        state.new_chat();
        let _ = std::fs::remove_file(chat_path(&root));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn controlled_continuations_keep_context_sealed_without_repeated_attachments() {
        let root =
            std::env::temp_dir().join(format!("forge-controlled-context-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("main.txt");
        std::fs::write(&file, "saved baseline").unwrap();
        // A malformed action leaves the bounded session able to accept a corrected next turn.
        let response=serde_json::json!({"choices":[{"message":{"content":"{\"action\":\"unknown\"}"},"finish_reason":"stop"}]}).to_string();
        let (config, server) = crate::native_models::tests::mock(
            crate::native_models::Protocol::Chat,
            response,
            false,
            "200 OK",
        );
        let mut state = State::default();
        state.set_workspace(&root);
        state.native.enabled = true;
        state.native.config = config;
        state.workflow.flow = crate::ai_workflow::Flow::Controlled;
        state.workflow.attach(&root, &file).unwrap();
        state
            .context
            .capture(
                &root,
                crate::ai_context::Kind::Selection,
                "selection".into(),
                "SEALED_CONTEXT_MARKER".into(),
                false,
            )
            .unwrap();
        state.context.include(0, true).unwrap();
        state.input = "Plan a fix".into();
        let context = egui::Context::default();
        state.send(&context, Some(&root), true);
        assert!(state.context_locked());
        let review = state.native.review.take().unwrap();
        assert!(!review.prepared.prompt.contains("saved baseline"));
        state.accept_native(review, &context, Some(&root), true);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while state.busy() {
            assert!(std::time::Instant::now() < deadline);
            state.poll();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        server.join().unwrap();
        state.input = "Continue controlled task.".into();
        state.send(&context, Some(&root), true);
        let review = state.native.review.as_ref().unwrap();
        assert_eq!(
            review
                .prepared
                .prompt
                .matches("SEALED_CONTEXT_MARKER")
                .count(),
            1
        );
        state.restrict();
        assert!(!state.context_locked());
        assert_eq!(state.context.augment(&root, "task").unwrap(), "task");
        let _ = std::fs::remove_file(chat_path(&root));
        std::fs::remove_dir_all(root).unwrap();
    }
}
