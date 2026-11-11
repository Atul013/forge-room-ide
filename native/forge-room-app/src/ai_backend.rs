//! A deliberately read-only first provider path. Codex owns authentication and model access;
//! Forge Room passes each request as argv/stdin, never through a shell.

use serde_json::Value;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Provider {
    #[default]
    Codex,
    Ollama,
    LmStudio,
}

impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex (OpenAI)",
            Self::Ollama => "Ollama (local)",
            Self::LmStudio => "LM Studio (local)",
        }
    }
}

pub fn ask(root: &Path, provider: Provider, model: &str, prompt: &str) -> Result<String, String> {
    let mut command = Command::new("codex");
    command.args(["exec", "--json", "--sandbox", "read-only", "--cd"]);
    command.arg(root);
    if provider != Provider::Codex {
        command.args(["--oss", "--local-provider"]);
        command.arg(match provider {
            Provider::Ollama => "ollama",
            Provider::LmStudio => "lmstudio",
            Provider::Codex => unreachable!(),
        });
    }
    let model = model.trim();
    if !model.is_empty() {
        command.args(["--model", model]);
    }
    command.arg("-");
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| format!(
        "Could not start Codex CLI: {error}. Install Codex and sign in with `codex login` in the terminal."
    ))?;
    child
        .stdin
        .take()
        .ok_or("Codex stdin was unavailable")?
        .write_all(prompt.as_bytes())
        .map_err(|error| format!("Could not send prompt: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("Codex failed: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let answer = last_agent_message(&stdout);
    if output.status.success() {
        answer.ok_or_else(|| "Codex finished without an assistant reply. Check its terminal login and model access.".into())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr
            .lines()
            .last()
            .unwrap_or("No error detail was returned.");
        Err(format!(
            "Codex exited with {}: {}",
            output.status,
            truncate(detail, 500)
        ))
    }
}

fn last_agent_message(events: &str) -> Option<String> {
    events
        .lines()
        .filter_map(|line| {
            let value: Value = serde_json::from_str(line).ok()?;
            if value.get("type")?.as_str()? != "item.completed" {
                return None;
            }
            let item = value.get("item")?;
            if item.get("type")?.as_str()? != "agent_message" {
                return None;
            }
            item.get("text")?
                .as_str()
                .map(|text| truncate(text, 20_000))
        })
        .last()
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let short: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{short}\n… (truncated)")
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_last_completed_agent_message() {
        let events = "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"first\"}}\n{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"final\"}}";
        assert_eq!(last_agent_message(events).as_deref(), Some("final"));
    }
}
