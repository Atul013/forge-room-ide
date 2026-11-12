//! Running a coding agent (Claude Code or Codex) from the AI panel: the prompt goes in on standard
//! input, the agent's own JSON event stream comes back on standard output, and the panel shows the
//! answer and the tools it used as they happen. The agent runs in the open folder with the same rights
//! as a terminal would give it, so it only runs in a folder the user trusts and its permission mode is
//! always visible and chosen by the user.

use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provider {
    Claude,
    Codex,
    /// Codex running a model on this computer through Ollama.
    Ollama,
    /// Codex running a model on this computer through LM Studio.
    LmStudio,
}

impl Provider {
    pub const ALL: [Provider; 4] = [
        Provider::Claude,
        Provider::Codex,
        Provider::Ollama,
        Provider::LmStudio,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Provider::Claude => "Claude Code",
            Provider::Codex => "Codex",
            Provider::Ollama => "Ollama (local)",
            Provider::LmStudio => "LM Studio (local)",
        }
    }

    /// Whether the Codex program runs it (and its JSON events are read).
    pub fn uses_codex(self) -> bool {
        self != Provider::Claude
    }

    fn program(self) -> &'static str {
        if self.uses_codex() { "codex" } else { "claude" }
    }

    /// The command that signs in to the provider, typed into a terminal by the panel's button.
    pub fn login_command(self) -> Option<&'static str> {
        match self {
            Provider::Claude => Some("claude auth login"),
            Provider::Codex => Some("codex login"),
            Provider::Ollama | Provider::LmStudio => None,
        }
    }

    /// The program on this computer: a `.exe` or `.cmd` named for the provider on PATH or in the
    /// folder npm installs command-line tools to.
    pub fn find(self) -> Option<PathBuf> {
        let mut folders: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect())
            .unwrap_or_default();
        // Where the installers put command-line tools, which a program started from the Start menu
        // may not have on its PATH: npm's folder, the native installer's, WinGet's links.
        if let Some(appdata) = std::env::var_os("APPDATA") {
            folders.push(PathBuf::from(appdata).join("npm"));
        }
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            let home = PathBuf::from(home);
            folders.push(home.join(".local").join("bin"));
            folders.push(home.join(".claude").join("local"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let local = PathBuf::from(local);
            folders.push(local.join("Microsoft").join("WinGet").join("Links"));
            folders.push(local.join("Programs").join("claude"));
        }
        let names: &[&str] = if cfg!(windows) {
            &["exe", "cmd"]
        } else {
            &[""]
        };
        for folder in folders {
            for extension in names {
                let file = if extension.is_empty() {
                    self.program().to_string()
                } else {
                    format!("{}.{extension}", self.program())
                };
                let candidate = folder.join(file);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }
}

/// What the agent may do without asking (a headless run cannot ask).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    /// Look and answer: no edits, no commands.
    ReadOnly,
    /// Edit files in the folder.
    Edit,
    /// Everything, including commands and the network, without asking.
    Full,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::ReadOnly, Mode::Edit, Mode::Full];

    pub fn label(self) -> &'static str {
        match self {
            Mode::ReadOnly => "Read only",
            Mode::Edit => "Edit files",
            Mode::Full => "Full access",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Mode::ReadOnly => "The agent can read and answer but cannot change anything",
            Mode::Edit => "The agent can edit files in this folder",
            Mode::Full => "The agent can run any command without asking: only for code you trust",
        }
    }
}

/// The effort levels each provider accepts.
pub fn efforts(provider: Provider) -> &'static [&'static str] {
    if provider.uses_codex() {
        &["minimal", "low", "medium", "high"]
    } else {
        &["low", "medium", "high", "xhigh", "max"]
    }
}

/// A model name is a plain token (`opus`, `claude-opus-4-1`, `gpt-5-codex`): anything else is not
/// passed on, so a pasted sentence can never become extra arguments.
pub fn valid_model(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_alphanumeric())
        && name.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '/' | '[' | ']')
        })
}

pub struct Request {
    pub provider: Provider,
    /// The model to use; empty is the provider's own default.
    pub model: String,
    /// How hard to think; empty is the provider's own default.
    pub effort: String,
    pub mode: Mode,
    pub prompt: String,
    pub cwd: PathBuf,
    /// The conversation to continue, when the provider can.
    pub session: Option<String>,
}

/// What a run reports, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A piece of the agent's answer.
    Text(String),
    /// The agent used a tool (edited a file, ran a command): one short line.
    Tool(String),
    /// The id to pass back to continue this conversation.
    Session(String),
    /// Something the agent reported that does not end the run (a retry, a warning).
    Notice(String),
    /// The run ended. `error` is why it failed, if it did.
    Done {
        cost: Option<f64>,
        error: Option<String>,
    },
}

/// The command-line arguments for a run (the prompt travels on standard input, so nothing the user
/// types ever reaches a command line).
pub fn arguments(request: &Request) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    match request.provider {
        Provider::Claude => {
            args.extend(
                ["-p", "--output-format", "stream-json", "--verbose"]
                    .iter()
                    .map(|a| a.to_string()),
            );
            match request.mode {
                Mode::ReadOnly => {
                    // "Cannot change anything" has to hold even when the repository ships its own
                    // Claude settings (hooks, allow rules): only the user's own settings apply.
                    args.extend(["--setting-sources".into(), "user".into()]);
                    args.extend(["--permission-mode".into(), "plan".into()]);
                }
                Mode::Edit => args.extend(["--permission-mode".into(), "acceptEdits".into()]),
                Mode::Full => args.push("--dangerously-skip-permissions".into()),
            }
            if valid_model(&request.model) {
                args.extend(["--model".into(), request.model.clone()]);
            }
            if efforts(Provider::Claude).contains(&request.effort.as_str()) {
                args.extend(["--effort".into(), request.effort.clone()]);
            }
            // A session id is a plain token; anything else is not passed on.
            if let Some(id) = request.session.as_deref().filter(|id| clean_id(id)) {
                args.extend(["--resume".into(), id.to_string()]);
            }
        }
        Provider::Codex | Provider::Ollama | Provider::LmStudio => {
            args.extend([
                "exec".into(),
                "--json".into(),
                "--skip-git-repo-check".into(),
            ]);
            // A model that runs on this computer, through Ollama or LM Studio.
            let local = match request.provider {
                Provider::Ollama => Some("ollama"),
                Provider::LmStudio => Some("lmstudio"),
                _ => None,
            };
            if let Some(local) = local {
                args.extend(["--oss".into(), "--local-provider".into(), local.into()]);
            }
            let sandbox = match request.mode {
                Mode::ReadOnly => "read-only",
                Mode::Edit => "workspace-write",
                Mode::Full => "danger-full-access",
            };
            args.extend(["--sandbox".into(), sandbox.into()]);
            if valid_model(&request.model) {
                args.extend(["--model".into(), request.model.clone()]);
            }
            if efforts(request.provider).contains(&request.effort.as_str()) {
                args.extend([
                    "-c".into(),
                    format!("model_reasoning_effort={}", request.effort),
                ]);
            }
            // The prompt is read from standard input.
            args.push("-".into());
        }
    }
    args
}

fn clean_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The events in one line of Claude Code's `stream-json` output.
pub fn parse_claude(line: &str) -> Vec<Event> {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let mut events = Vec::new();
    match value.get("type").and_then(Value::as_str) {
        Some("system") => {
            if let Some(id) = value.get("session_id").and_then(Value::as_str) {
                events.push(Event::Session(id.to_string()));
            }
        }
        Some("assistant") => {
            let blocks = value
                .pointer("/message/content")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(text) = block.get("text").and_then(Value::as_str)
                            && !text.trim().is_empty()
                        {
                            events.push(Event::Text(text.to_string()));
                        }
                    }
                    Some("tool_use") => {
                        let name = block.get("name").and_then(Value::as_str).unwrap_or("tool");
                        events.push(Event::Tool(describe_tool(name, block.get("input"))));
                    }
                    _ => {}
                }
            }
        }
        Some("result") => {
            if let Some(id) = value.get("session_id").and_then(Value::as_str) {
                events.push(Event::Session(id.to_string()));
            }
            if let Some(denials) = value.get("permission_denials").and_then(Value::as_array)
                && !denials.is_empty()
            {
                let mut names: Vec<String> = denials
                    .iter()
                    .filter_map(|d| d.get("tool_name").and_then(Value::as_str))
                    .map(String::from)
                    .collect();
                names.sort();
                names.dedup();
                events.push(Event::Notice(format!(
                    "The agent was not allowed to use: {}. A run from here cannot ask for \
                     permission: choose Full access to let it run commands.",
                    names.join(", ")
                )));
            }
            let failed = value.get("is_error").and_then(Value::as_bool) == Some(true)
                || value
                    .get("subtype")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| kind.starts_with("error"));
            events.push(Event::Done {
                cost: value.get("total_cost_usd").and_then(Value::as_f64),
                error: failed.then(|| {
                    value
                        .get("result")
                        .and_then(Value::as_str)
                        .filter(|text| !text.is_empty())
                        .unwrap_or("The agent reported an error")
                        .to_string()
                }),
            });
        }
        _ => {}
    }
    events
}

/// One line about a tool call: its name and the most telling input (a path, a command).
fn describe_tool(name: &str, input: Option<&Value>) -> String {
    let detail = input.and_then(|input| {
        [
            "file_path",
            "path",
            "command",
            "pattern",
            "url",
            "description",
        ]
        .iter()
        .find_map(|key| input.get(*key).and_then(Value::as_str))
    });
    match detail {
        Some(detail) => {
            // Every line of a command is shown (joined), so nothing a multi-line command does after
            // its first line is hidden; only an extremely long one is cut.
            let lines: Vec<&str> = detail
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect();
            let mut detail = lines.join("  ;  ");
            if detail.chars().count() > 400 {
                detail = detail.chars().take(400).collect::<String>() + "…";
            }
            format!("{name} {detail}")
        }
        None => name.to_string(),
    }
}

/// The events in one line of `codex exec --json` output. Codex reports finished work as items
/// (`agent_message`, `command_execution`, `file_change`), so only the kinds known to matter are read.
pub fn parse_codex(line: &str) -> Vec<Event> {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let mut events = Vec::new();
    match value.get("type").and_then(Value::as_str) {
        Some("thread.started") => {
            if let Some(id) = value.get("thread_id").and_then(Value::as_str) {
                events.push(Event::Session(id.to_string()));
            }
        }
        Some("item.completed") => {
            let item = value.get("item").cloned().unwrap_or(Value::Null);
            match item.get("type").and_then(Value::as_str) {
                Some("agent_message") => {
                    if let Some(text) = item.get("text").and_then(Value::as_str)
                        && !text.trim().is_empty()
                    {
                        events.push(Event::Text(text.to_string()));
                    }
                }
                Some("command_execution") => {
                    let command = item.get("command").and_then(Value::as_str).unwrap_or("");
                    events.push(Event::Tool(describe_tool(
                        "Run",
                        Some(&serde_json::json!({ "command": command })),
                    )));
                }
                Some("file_change") => events.push(Event::Tool("Edit files".into())),
                _ => {}
            }
        }
        Some("turn.completed") => events.push(Event::Done {
            cost: None,
            error: None,
        }),
        Some("turn.failed") => {
            let message = value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("The agent reported an error");
            events.push(Event::Done {
                cost: None,
                error: Some(message.to_string()),
            });
        }
        // An error in the middle of a turn can be a retry: the run goes on until the turn ends or
        // the process does.
        Some("error") => {
            let message = value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("The agent reported an error");
            events.push(Event::Notice(message.to_string()));
        }
        _ => {}
    }
    events
}

/// A run in progress.
pub struct Run {
    receiver: mpsc::Receiver<Event>,
    /// The agent's process until it has been waited for. Once it is gone from here its process
    /// number may be reused, so nothing is ever killed by number after that.
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl Run {
    /// Events that arrived since the last call.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }

    /// Stop the agent and everything it started. An npm-installed agent is a `.cmd` file that runs
    /// `node`, so killing the first process alone would leave the real agent running.
    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut child) = self.child.lock()
            && let Some(child) = child.as_mut()
        {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                // The whole tree (/T), without asking (/F). The process handle is still open, so
                // the number cannot belong to anything else yet.
                let _ = Command::new("taskkill")
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .creation_flags(0x0800_0000)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = child.kill();
        }
    }
}

impl Drop for Run {
    /// Leaving the panel (or closing the app) never leaves an agent running unseen.
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Start `request` with `program`; `wake` is called whenever there is something new to show.
pub fn start(
    program: &Path,
    request: &Request,
    wake: impl Fn() + Send + 'static,
) -> Result<Run, String> {
    let mut command = Command::new(program);
    if request.mode != Mode::Full {
        // Without full access the agent has no business with the user's tokens for other services.
        for name in [
            "GITHUB_TOKEN",
            "GH_TOKEN",
            "NPM_TOKEN",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "AWS_SESSION_TOKEN",
        ] {
            command.env_remove(name);
        }
    }
    command
        .args(arguments(request))
        .current_dir(&request.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start {}: {error}", request.provider.name()))?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let child = Arc::new(Mutex::new(Some(child)));
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel();
    let provider = request.provider;
    // The message is written on its own thread: a long one cannot fill the pipe while this one
    // waits (the window must never freeze on it), and the output is already being read.
    let prompt = request.prompt.clone().into_bytes();
    std::thread::spawn(move || {
        if let Some(mut stdin) = stdin {
            // A failure shows up as the agent's own complaint; closing the pipe ends the message.
            let _ = stdin.write_all(&prompt);
        }
    });
    {
        let (child, cancelled) = (child.clone(), cancelled.clone());
        std::thread::spawn(move || {
            // The error output is read on its own thread so a chatty agent cannot fill the pipe.
            let errors = std::thread::spawn(move || {
                let mut bytes = Vec::new();
                if let Some(pipe) = &mut stderr {
                    let _ = pipe.read_to_end(&mut bytes);
                }
                String::from_utf8_lossy(&bytes).into_owned()
            });
            let mut finished = false;
            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout);
                let mut line = Vec::new();
                loop {
                    line.clear();
                    // Bytes, not text: one line that is not valid UTF-8 must not end the reading
                    // (the agent would then block on a full pipe for ever).
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                    let text = String::from_utf8_lossy(&line);
                    let text = text.trim_end();
                    let events = if provider.uses_codex() {
                        parse_codex(text)
                    } else {
                        parse_claude(text)
                    };
                    for event in events {
                        finished |= matches!(event, Event::Done { .. });
                        let _ = sender.send(event);
                    }
                    wake();
                }
            }
            let status = child
                .lock()
                .ok()
                .and_then(|mut child| child.take())
                .and_then(|mut child| child.wait().ok());
            let errors = errors.join().unwrap_or_default();
            if !finished {
                let error = if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                    "Stopped".to_string()
                } else {
                    let reason = errors
                        .lines()
                        .rev()
                        .find(|line| !line.trim().is_empty())
                        .unwrap_or("");
                    match status {
                        Some(status) if !status.success() && !reason.is_empty() => {
                            format!("{} stopped: {reason}", provider.name())
                        }
                        Some(status) if !status.success() => {
                            format!("{} stopped without an answer", provider.name())
                        }
                        _ => format!("{} ended without an answer", provider.name()),
                    }
                };
                let _ = sender.send(Event::Done {
                    cost: None,
                    error: Some(error),
                });
            }
            wake();
        });
    }
    Ok(Run {
        receiver,
        child,
        cancelled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(provider: Provider, mode: Mode, session: Option<&str>) -> Request {
        Request {
            provider,
            model: String::new(),
            effort: String::new(),
            mode,
            prompt: "hello".into(),
            cwd: PathBuf::from("."),
            session: session.map(String::from),
        }
    }

    #[test]
    fn claude_arguments_follow_the_mode_and_resume_only_a_clean_session() {
        let read = arguments(&request(Provider::Claude, Mode::ReadOnly, None));
        assert!(read.windows(2).any(|w| w == ["--permission-mode", "plan"]));
        assert!(read.contains(&"stream-json".to_string()));
        let edit = arguments(&request(Provider::Claude, Mode::Edit, Some("abc-123_X")));
        assert!(
            edit.windows(2)
                .any(|w| w == ["--permission-mode", "acceptEdits"])
        );
        assert!(edit.windows(2).any(|w| w == ["--resume", "abc-123_X"]));
        let full = arguments(&request(Provider::Claude, Mode::Full, None));
        assert!(full.contains(&"--dangerously-skip-permissions".to_string()));
        // A session id that is not a plain token never reaches the command line.
        let odd = arguments(&request(Provider::Claude, Mode::Edit, Some("x; rm -rf /")));
        assert!(!odd.iter().any(|a| a == "--resume"));
        // The prompt is never an argument.
        assert!(!edit.iter().any(|a| a == "hello"));
        // Read only leaves repository settings out; edit mode keeps the project's own.
        assert!(read.windows(2).any(|w| w == ["--setting-sources", "user"]));
        assert!(!edit.iter().any(|a| a == "--setting-sources"));
        // A model name that looks like an option is never passed on.
        assert!(!valid_model("--dangerously-skip-permissions"));
        assert!(!valid_model("-x"));
        assert!(valid_model("claude-opus-4-1"));
    }

    #[test]
    fn model_and_effort_are_passed_only_when_they_are_plain_known_values() {
        let mut claude = request(Provider::Claude, Mode::Edit, None);
        claude.model = "opus".into();
        claude.effort = "high".into();
        let args = arguments(&claude);
        assert!(args.windows(2).any(|w| w == ["--model", "opus"]));
        assert!(args.windows(2).any(|w| w == ["--effort", "high"]));
        // Defaults add nothing; odd values are dropped, never passed on.
        let plain = arguments(&request(Provider::Claude, Mode::Edit, None));
        assert!(!plain.iter().any(|a| a == "--model" || a == "--effort"));
        claude.model = "opus --dangerously-skip-permissions".into();
        claude.effort = "maximum-overdrive".into();
        let odd = arguments(&claude);
        assert!(!odd.iter().any(|a| a == "--model" || a == "--effort"));
        assert!(!odd.iter().any(|a| a == "--dangerously-skip-permissions"));
        let mut codex = request(Provider::Codex, Mode::Edit, None);
        codex.model = "gpt-5-codex".into();
        codex.effort = "minimal".into();
        let args = arguments(&codex);
        assert!(args.windows(2).any(|w| w == ["--model", "gpt-5-codex"]));
        assert!(
            args.windows(2)
                .any(|w| w == ["-c", "model_reasoning_effort=minimal"])
        );
        assert_eq!(args.last().map(String::as_str), Some("-"));
        // Claude's top levels are not Codex's.
        codex.effort = "xhigh".into();
        assert!(!arguments(&codex).iter().any(|a| a == "-c"));
    }

    #[test]
    fn codex_arguments_pick_the_sandbox_and_read_the_prompt_from_stdin() {
        let args = arguments(&request(Provider::Codex, Mode::Edit, None));
        assert_eq!(args.first().map(String::as_str), Some("exec"));
        assert!(
            args.windows(2)
                .any(|w| w == ["--sandbox", "workspace-write"])
        );
        assert_eq!(args.last().map(String::as_str), Some("-"));
        let read = arguments(&request(Provider::Codex, Mode::ReadOnly, None));
        assert!(read.windows(2).any(|w| w == ["--sandbox", "read-only"]));
    }

    #[test]
    fn claude_events_become_session_text_tools_and_an_end() {
        let init = r#"{"type":"system","subtype":"init","session_id":"s-1"}"#;
        assert_eq!(parse_claude(init), vec![Event::Session("s-1".into())]);
        let message = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Looking"},{"type":"tool_use","name":"Edit","input":{"file_path":"src/a.rs","old_string":"x"}},{"type":"tool_use","name":"Bash","input":{"command":"cargo test\nmore"}}]}}"#;
        assert_eq!(
            parse_claude(message),
            vec![
                Event::Text("Looking".into()),
                Event::Tool("Edit src/a.rs".into()),
                Event::Tool("Bash cargo test  ;  more".into()),
            ]
        );
        let done = r#"{"type":"result","subtype":"success","is_error":false,"result":"ok","session_id":"s-1","total_cost_usd":0.0123}"#;
        assert_eq!(
            parse_claude(done),
            vec![
                Event::Session("s-1".into()),
                Event::Done {
                    cost: Some(0.0123),
                    error: None
                }
            ]
        );
        let failed = r#"{"type":"result","subtype":"error_max_turns","is_error":true,"result":"Too many turns"}"#;
        assert_eq!(
            parse_claude(failed),
            vec![Event::Done {
                cost: None,
                error: Some("Too many turns".into())
            }]
        );
        // Tool results, blank text and junk are not shown.
        assert!(parse_claude(r#"{"type":"user","message":{"content":[]}}"#).is_empty());
        assert!(
            parse_claude(
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"  "}]}}"#
            )
            .is_empty()
        );
        assert!(parse_claude("not json").is_empty());
    }

    #[test]
    fn codex_events_become_text_commands_and_an_end() {
        assert_eq!(
            parse_codex(r#"{"type":"thread.started","thread_id":"t1"}"#),
            vec![Event::Session("t1".into())]
        );
        assert_eq!(
            parse_codex(
                r#"{"type":"item.completed","item":{"type":"agent_message","text":"Done."}}"#
            ),
            vec![Event::Text("Done.".into())]
        );
        assert_eq!(
            parse_codex(
                r#"{"type":"item.completed","item":{"type":"command_execution","command":"cargo build"}}"#
            ),
            vec![Event::Tool("Run cargo build".into())]
        );
        assert_eq!(
            parse_codex(r#"{"type":"turn.completed"}"#),
            vec![Event::Done {
                cost: None,
                error: None
            }]
        );
        assert_eq!(
            parse_codex(r#"{"type":"error","message":"no auth"}"#),
            vec![Event::Notice("no auth".into())]
        );
        // Only the end of the turn ends the run.
        assert_eq!(
            parse_codex(r#"{"type":"turn.failed","error":{"message":"quota"}}"#),
            vec![Event::Done {
                cost: None,
                error: Some("quota".into())
            }]
        );
    }

    /// A stand-in agent (a script that prints Claude-style events) shows that the whole path works:
    /// the prompt arrives on stdin, events come back in order, and the run ends with a result.
    #[cfg(windows)]
    #[test]
    fn a_run_streams_events_from_a_process_and_ends_with_a_result() {
        let dir = std::env::temp_dir().join(format!("forge-agent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("fake-agent.cmd");
        std::fs::write(
            &script,
            "@echo off\r\n\
             echo {\"type\":\"system\",\"session_id\":\"fake-1\"}\r\n\
             echo {\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hi there\"}]}}\r\n\
             echo {\"type\":\"result\",\"is_error\":false,\"result\":\"ok\",\"session_id\":\"fake-1\",\"total_cost_usd\":0.5}\r\n",
        )
        .unwrap();
        let request = Request {
            provider: Provider::Claude,
            model: String::new(),
            effort: String::new(),
            mode: Mode::ReadOnly,
            prompt: "say hi".into(),
            cwd: dir.clone(),
            session: None,
        };
        let mut run = start(&script, &request, || {}).unwrap();
        let mut seen = Vec::new();
        for _ in 0..100 {
            seen.extend(run.poll());
            if seen.iter().any(|e| matches!(e, Event::Done { .. })) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(seen.contains(&Event::Text("hi there".into())), "{seen:?}");
        assert!(seen.contains(&Event::Session("fake-1".into())));
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::Done { error: None, cost: Some(c) } if *c == 0.5))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_program_that_cannot_start_is_reported() {
        let request = request(Provider::Claude, Mode::ReadOnly, None);
        let error = start(Path::new("definitely-not-a-program-xyz"), &request, || {})
            .err()
            .expect("it cannot start");
        assert!(error.contains("Could not start"));
    }
}
