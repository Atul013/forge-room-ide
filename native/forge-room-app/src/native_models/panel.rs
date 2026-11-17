use super::*;
use crate::ai_workflow::Flow;
use eframe::egui;
use std::path::{Path, PathBuf};
type Catalog = (
    Config,
    std::sync::mpsc::Receiver<Result<Vec<String>, String>>,
);
pub struct Review {
    pub prepared: Prepared,
    pub root: PathBuf,
    pub instruction: String,
    pub flow: Flow,
}
#[derive(Default)]
pub struct State {
    pub enabled: bool,
    pub config: Config,
    pub run: Option<Run>,
    pub review: Option<Review>,
    pub history: Vec<Value>,
    identity: String,
    pub inflight: Option<String>,
    status: String,
    models: Vec<String>,
    catalog: Option<Catalog>,
}
impl State {
    pub fn reset(&mut self) {
        self.run = None;
        self.review = None;
        self.history.clear();
        self.identity.clear();
        self.inflight = None;
    }
    pub fn busy(&self) -> bool {
        self.run.is_some() || self.review.is_some()
    }
    pub fn prepare(
        &mut self,
        root: &Path,
        flow: Flow,
        prompt: String,
        instruction: String,
    ) -> Result<(), String> {
        if flow == Flow::Direct {
            return Err("Native HTTP supports Ask, Plan and Reviewed edits; it does not run a direct CLI agent".into());
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let identity = serde_json::to_string(&(
            &root,
            &self.config.base_url,
            &self.config.model,
            self.config.protocol,
            flow.label(),
            &self.config.key_env,
        ))
        .map_err(|_| "Workspace path cannot be represented in a native request")?;
        if identity != self.identity {
            self.history.clear();
            self.identity = identity;
        }
        let prepared = super::prepare(&self.config, prompt, &self.history)?;
        self.review = Some(Review {
            prepared,
            root,
            instruction,
            flow,
        });
        Ok(())
    }
    pub fn complete(&mut self, success: bool, output: &str) {
        if let Some(prompt) = self.inflight.take()
            && success
        {
            self.history.push(json!({"role":"user","content":prompt}));
            self.history
                .push(json!({"role":"assistant","content":output}));
            // Never silently remove turns; the next request explains when a new conversation is needed.
            if self.history.len() > 18
                || self
                    .history
                    .iter()
                    .map(|m| m["content"].as_str().unwrap_or("").len())
                    .sum::<usize>()
                    > MAX_INPUT
            {
                self.history.clear();
                self.status =
                    "Native conversation exceeded its retained history limit and was cleared"
                        .into();
            }
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, busy: bool) {
        if let Some((config, rx)) = &self.catalog
            && let Ok(result) = rx.try_recv()
        {
            if config == &self.config {
                match result {
                    Ok(models) => {
                        self.status = format!("{} model(s) reported by endpoint", models.len());
                        self.models = models;
                    }
                    Err(error) => self.status = error,
                }
            }
            self.catalog = None;
        }
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (label, base, protocol, key) in [
                    ("Ollama", "http://127.0.0.1:11434/v1", Protocol::Chat, ""),
                    ("LM Studio", "http://127.0.0.1:1234/v1", Protocol::Chat, ""),
                    (
                        "OpenAI-compatible",
                        "https://api.openai.com/v1",
                        Protocol::Chat,
                        "OPENAI_API_KEY",
                    ),
                    (
                        "Anthropic",
                        "https://api.anthropic.com/v1",
                        Protocol::Messages,
                        "ANTHROPIC_API_KEY",
                    ),
                ] {
                    if ui.button(label).clicked() {
                        self.config.base_url = base.into();
                        self.config.protocol = protocol;
                        self.config.key_env = key.into();
                        self.config.modern_limit = label == "OpenAI-compatible";
                        self.config.model.clear();
                        self.models.clear();
                        self.history.clear();
                    }
                }
            });
            ui.label("API base URL");
            ui.add(egui::TextEdit::singleline(&mut self.config.base_url).char_limit(2048));
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(
                    &mut self.config.protocol,
                    Protocol::Chat,
                    "Chat Completions",
                );
                ui.selectable_value(
                    &mut self.config.protocol,
                    Protocol::Messages,
                    "Anthropic Messages",
                );
            });
            ui.label("API key environment variable (optional; no key is saved)");
            ui.add(egui::TextEdit::singleline(&mut self.config.key_env).char_limit(100));
            ui.label("Model ID");
            ui.add(egui::TextEdit::singleline(&mut self.config.model).char_limit(200));
            if ui.button("Fetch models from this endpoint").clicked() && self.catalog.is_none() {
                let config = self.config.clone();
                let work = config.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                let context = ui.ctx().clone();
                self.catalog = Some((config, rx));
                self.status = "Fetching model IDs; no chat or files are sent".into();
                std::thread::spawn(move || {
                    let _ = tx.send(super::discover(work));
                    context.request_repaint();
                });
            }
            egui::ComboBox::from_id_salt("native-model-catalog")
                .selected_text("Choose discovered model")
                .show_ui(ui, |ui| {
                    for model in &self.models {
                        ui.selectable_value(&mut self.config.model, model.clone(), model);
                    }
                });
            ui.horizontal_wrapped(|ui| {
                ui.label("Max output tokens");
                ui.add(egui::DragValue::new(&mut self.config.max_tokens).range(128..=32768));
                ui.label("Time limit (seconds)");
                ui.add(egui::DragValue::new(&mut self.config.seconds).range(10..=600));
            });
            if self.config.protocol == Protocol::Chat {
                ui.checkbox(
                    &mut self.config.modern_limit,
                    "Use max_completion_tokens (newer model APIs)",
                );
            }
            ui.checkbox(
                &mut self.config.history,
                "Include this native conversation on subsequent sends",
            );
            ui.small(format!(
                "Retained native history: {} message(s). CLI history is never forwarded.",
                self.history.len()
            ));
            if ui.button("Clear native conversation").clicked() {
                self.history.clear();
            }
            if ui.button("Save connection settings").clicked() {
                self.status = match self.config.validate() {
                    Err(e) => e,
                    Ok(()) => match crate::settings::update(|s| {
                        s.native_model = Some(self.config.clone())
                    }) {
                        Ok(()) => {
                            "Connection saved; no API key or execution choice was stored".into()
                        }
                        Err(_) => "Could not save connection settings".into(),
                    },
                };
            }
        });
        ui.small("Text only. No tools, commands or automatic file access. Ask/Plan/Reviewed edits use explicit attached context. Output limits are not a dollar budget; stopping the client may not stop provider billing.");
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
    }
    /// Returns a reviewed request only after the user presses Send this request.
    pub fn decision(&mut self, context: &egui::Context) -> Option<Review> {
        let review = self.review.as_ref()?;
        let mut send = false;
        let mut cancel = false;
        let mut open = true;
        egui::Window::new("Review native model request").open(&mut open).resizable(true).default_width(650.0).show(context,|ui|{
            let suffix=if review.prepared.config.protocol==Protocol::Chat {"chat/completions"}else{"messages"};
            ui.label(format!("Destination: {}",review.prepared.config.url(suffix).map(|u|u.to_string()).unwrap_or_else(|_|"Invalid endpoint".into())));ui.label(format!("Model: {}",review.prepared.config.model));
            ui.label(format!("{} bytes; at most {} output tokens / {} seconds",serde_json::to_vec(&review.prepared.body).map_or(0,|b|b.len()),review.prepared.config.max_tokens,review.prepared.config.seconds));
            ui.small("The complete request below leaves this device when you send. Authentication uses the named environment variable and is not shown. No native tools are enabled.");
            egui::ScrollArea::vertical().max_height(350.0).show(ui,|ui|{if let Some(messages)=review.prepared.body["messages"].as_array(){for message in messages{ui.label(message["role"].as_str().unwrap_or("message"));ui.monospace(message["content"].as_str().unwrap_or(""));ui.separator();}}});
            ui.horizontal(|ui|{send=ui.button("Send this request").clicked();cancel=ui.button("Cancel").clicked();});
        });
        if send {
            return self.review.take();
        }
        if cancel || !open {
            self.review = None;
        }
        None
    }
}
