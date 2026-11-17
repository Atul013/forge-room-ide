//! Reviewable sequential VS Code task plans, executed only by an explicit terminal action.
use crate::project_tools::{Page, normalize_jsonc};
use eframe::egui;
use serde_json::Value;
use std::io::Read;
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

struct Step {
    label: String,
    command: String,
    cwd: PathBuf,
    env: BTreeMap<String, Option<String>>,
}
struct Plan {
    steps: Vec<Step>,
    script: String,
}

struct Card {
    label: String,
    group: Option<String>,
    plan: Result<Plan, String>,
}
#[derive(Default)]
pub struct State {
    context: Option<(PathBuf, Option<PathBuf>)>,
    stamp: Option<(u64, Option<std::time::SystemTime>)>,
    checked: Option<std::time::Instant>,
    cards: Vec<Card>,
    error: Option<String>,
}
impl State {
    fn refresh(&mut self, root: &Path, active: Option<&Path>) {
        let context = (root.to_path_buf(), active.map(Path::to_path_buf));
        let changed = self.context.as_ref() != Some(&context);
        if !changed
            && self
                .checked
                .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(1))
        {
            return;
        }
        self.checked = Some(std::time::Instant::now());
        let stamp = fs::metadata(root.join(".vscode/tasks.json"))
            .ok()
            .map(|m| (m.len(), m.modified().ok()));
        if !changed && stamp == self.stamp {
            return;
        }
        self.context = Some(context);
        self.stamp = stamp;
        self.cards.clear();
        self.error = None;
        match read(root) {
            Ok(Some(tasks)) => {
                for task in &tasks {
                    let label = task
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("<missing label>");
                    let group = task
                        .get("group")
                        .and_then(|v| v.as_str().or_else(|| v.get("kind")?.as_str()))
                        .map(str::to_owned);
                    self.cards.push(Card {
                        label: label.chars().take(100).collect(),
                        group,
                        plan: plan(label.trim(), &tasks, root, active),
                    });
                }
            }
            Ok(None) => {}
            Err(e) => self.error = Some(e),
        }
    }
}
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
fn bounded(s: &str, max: usize) -> Result<&str, String> {
    if s.len() > max || s.chars().any(char::is_control) {
        Err("Text is too long or contains control characters".into())
    } else {
        Ok(s)
    }
}
fn expand(text: &str, root: &Path, active: Option<&Path>) -> Result<String, String> {
    bounded(text, 2048)?;
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        let end = rest.find('}').ok_or("Unclosed task variable")?;
        let name = &rest[..end];
        let root_text = crate::tabs::normalize(root);
        let value = match name {
            "workspaceFolder" => root_text.to_string_lossy().into_owned(),
            "workspaceFolderBasename" => root
                .file_name()
                .ok_or("Workspace has no basename")?
                .to_string_lossy()
                .into_owned(),
            "pathSeparator" | "/" => std::path::MAIN_SEPARATOR.to_string(),
            "file"
            | "fileDirname"
            | "fileBasename"
            | "fileBasenameNoExtension"
            | "relativeFile"
            | "relativeFileDirname" => {
                let file = active
                    .ok_or("This task requires an active saved file")?
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                if !file.starts_with(root) || !file.is_file() {
                    return Err("Active file must be inside this workspace".into());
                }
                let value = match name {
                    "file" => crate::tabs::normalize(&file),
                    "fileDirname" => crate::tabs::normalize(file.parent().unwrap()),
                    "fileBasename" => PathBuf::from(file.file_name().unwrap()),
                    "fileBasenameNoExtension" => PathBuf::from(file.file_stem().unwrap()),
                    "relativeFile" => file.strip_prefix(root).unwrap().to_path_buf(),
                    _ => file
                        .strip_prefix(root)
                        .unwrap()
                        .parent()
                        .unwrap_or(Path::new(""))
                        .to_path_buf(),
                };
                value.to_string_lossy().into_owned()
            }
            _ => return Err(format!("Unsupported variable: ${{{name}}}")),
        };
        bounded(&value, 4096)?;
        out.push_str(&value);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    bounded(&out, 4096)?;
    Ok(out)
}
fn merge(base: &mut Value, overlay: &Value) {
    if let (Some(base), Some(overlay)) = (base.as_object_mut(), overlay.as_object()) {
        for (key, value) in overlay {
            if let Some(old) = base.get_mut(key) {
                merge(old, value);
            } else {
                base.insert(key.clone(), value.clone());
            }
        }
    } else {
        *base = overlay.clone();
    }
}
fn effective(task: &Value) -> Value {
    let mut out = task.clone();
    if cfg!(windows)
        && let Some(overlay) = task.get("windows")
    {
        merge(&mut out, overlay);
    }
    out
}
fn step(
    task: &Value,
    label: &str,
    root: &Path,
    active: Option<&Path>,
) -> Result<Option<Step>, String> {
    if task
        .get("isBackground")
        .is_some_and(|v| v.as_bool() != Some(false))
    {
        return Err("Background readiness is unsupported; use a foreground task".into());
    }
    if task.get("options").is_some_and(|o| !o.is_object()) {
        return Err("Task options must be an object".into());
    }
    if task["options"].get("shell").is_some() {
        return Err("Custom task shells are unsupported; this runner uses PowerShell".into());
    }
    let kind = match task.get("type") {
        None => "shell",
        Some(v) => v.as_str().ok_or("Task type must be a string")?,
    };
    if !matches!(kind, "shell" | "process") {
        return Err(format!("Unsupported task type: {kind}"));
    }
    let Some(command) = task.get("command") else {
        if task.get("dependsOn").is_some() {
            return Ok(None);
        }
        return Err("Task needs a command or dependencies".into());
    };
    let command = command.as_str().ok_or("Task command must be a string")?;
    if command.trim().is_empty() {
        return Err("Task command is empty".into());
    }
    bounded(command, 2048)?;
    let mut rendered = match kind {
        "process" => format!("& {}", quote(&expand(command, root, active)?)),
        "shell" => {
            if command.contains("${") {
                return Err("Put variables in args, cwd or env; raw shell command variables are unsupported".into());
            }
            command.into()
        }
        _ => return Err(format!("Unsupported task type: {kind}")),
    };
    if let Some(args) = task.get("args") {
        let args = args.as_array().ok_or("Task args must be a string array")?;
        if args.len() > 64 {
            return Err("Task has more than 64 arguments".into());
        }
        for arg in args {
            rendered.push(' ');
            rendered.push_str(&quote(&expand(
                arg.as_str().ok_or("Task argument must be a string")?,
                root,
                active,
            )?));
        }
    }
    let cwd = match task["options"].get("cwd") {
        None => root.to_path_buf(),
        Some(v) => {
            let value = expand(v.as_str().ok_or("cwd must be a string")?, root, active)?;
            let path = Path::new(&value);
            let path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            };
            path.canonicalize()
                .map_err(|_| "Task cwd must be an existing directory")?
        }
    };
    if !cwd.is_dir() || !cwd.starts_with(root) {
        return Err("Task cwd must resolve inside the workspace".into());
    }
    let mut env = BTreeMap::new();
    if let Some(values) = task["options"].get("env") {
        let values = values.as_object().ok_or("Task env must be an object")?;
        if values.len() > 32 {
            return Err("Task env exceeds 32 entries".into());
        }
        for (key, value) in values {
            if key.is_empty()
                || key.len() > 100
                || !key.bytes().enumerate().all(|(i, c)| {
                    c.is_ascii_alphabetic() || c == b'_' || (i > 0 && c.is_ascii_digit())
                })
            {
                return Err("Invalid task environment name".into());
            }
            if env.keys().any(|k: &String| k.eq_ignore_ascii_case(key)) {
                return Err("Duplicate environment name ignoring case".into());
            }
            env.insert(
                key.clone(),
                if value.is_null() {
                    None
                } else {
                    Some(expand(
                        value
                            .as_str()
                            .ok_or("Environment values must be strings or null")?,
                        root,
                        active,
                    )?)
                },
            );
        }
    }
    Ok(Some(Step {
        label: label.into(),
        command: rendered,
        cwd: crate::tabs::normalize(&cwd),
        env,
    }))
}
fn visit(
    label: &str,
    tasks: &[Value],
    root: &Path,
    active: Option<&Path>,
    visiting: &mut HashSet<String>,
    done: &mut HashSet<String>,
    steps: &mut Vec<Step>,
) -> Result<(), String> {
    if done.contains(label) {
        return Ok(());
    }
    if visiting.len() >= 16 || steps.len() >= 32 {
        return Err("Task graph exceeds depth 16 or 32 steps".into());
    }
    if !visiting.insert(label.into()) {
        return Err(format!("Dependency cycle at {label}"));
    }
    let matches: Vec<_> = tasks
        .iter()
        .filter(|t| t.get("label").and_then(Value::as_str).map(str::trim) == Some(label))
        .collect();
    if matches.len() != 1 {
        return Err(format!("Missing or duplicate task label: {label}"));
    }
    bounded(label, 100)?;
    if label.is_empty() {
        return Err("Task label is empty".into());
    }
    let task = effective(matches[0]);
    let deps = match task.get("dependsOn") {
        None => vec![],
        Some(Value::String(s)) => vec![s.as_str()],
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| v.as_str().ok_or("Dependency must be a label string"))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("Invalid dependsOn".into()),
    };
    if deps.len() > 16 {
        return Err("Task has more than 16 dependencies".into());
    }
    if deps.len() > 1 && task.get("dependsOrder").and_then(Value::as_str) != Some("sequence") {
        return Err("Multiple dependencies require dependsOrder: sequence; parallel execution is unsupported".into());
    }
    if task
        .get("dependsOrder")
        .is_some_and(|v| v.as_str() != Some("sequence") && v.as_str() != Some("parallel"))
    {
        return Err("Unknown dependsOrder".into());
    }
    // Validate this node before walking dependencies, so an unsupported node never runs prerequisites.
    let own = step(&task, label, root, active).map_err(|e| format!("{label}: {e}"))?;
    for dependency in deps {
        visit(
            dependency.trim(),
            tasks,
            root,
            active,
            visiting,
            done,
            steps,
        )?;
    }
    if let Some(own) = own {
        if steps.len() >= 32 {
            return Err("Task graph exceeds 32 steps".into());
        }
        steps.push(own);
    }
    visiting.remove(label);
    done.insert(label.into());
    Ok(())
}
fn plan(label: &str, tasks: &[Value], root: &Path, active: Option<&Path>) -> Result<Plan, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut steps = Vec::new();
    visit(
        label,
        tasks,
        &root,
        active,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut steps,
    )?;
    if steps.is_empty() {
        return Err("Task graph contains no commands".into());
    }
    let mut script = "& { $ErrorActionPreference = 'Stop'; ".to_owned();
    for (i, step) in steps.iter().enumerate() {
        script.push_str(&format!(
            "Write-Host {}; $forgeTaskEnv = @{{}}; ",
            quote(&format!(
                "[Forge task {}/{}] {}",
                i + 1,
                steps.len(),
                step.label
            ))
        ));
        for key in step.env.keys() {
            script.push_str(&format!(
                "$forgeTaskEnv[{}] = [Environment]::GetEnvironmentVariable({}, 'Process'); ",
                quote(key),
                quote(key)
            ));
        }
        script.push_str("try { ");
        for (key, value) in &step.env {
            script.push_str(&format!(
                "[Environment]::SetEnvironmentVariable({}, {}, 'Process'); ",
                quote(key),
                value
                    .as_deref()
                    .map(quote)
                    .unwrap_or_else(|| "$null".into())
            ));
        }
        script.push_str(&format!("Push-Location -LiteralPath {}; try {{ $LASTEXITCODE = 0; {}; $forgeTaskOk = $?; $forgeTaskExit = $LASTEXITCODE; if (-not $forgeTaskOk -or $forgeTaskExit -ne 0) {{ throw {} }} }} finally {{ Pop-Location }} ",quote(&step.cwd.to_string_lossy()),step.command,quote(&format!("Task failed: {}",step.label))));
        script.push_str("} finally { foreach ($forgeTaskKey in $forgeTaskEnv.Keys) { [Environment]::SetEnvironmentVariable($forgeTaskKey, $forgeTaskEnv[$forgeTaskKey], 'Process') } }; ");
    }
    script.push_str(&format!(
        "Write-Host {} }}",
        quote(&format!(
            "[Forge tasks] All {} steps succeeded.",
            steps.len()
        ))
    ));
    if script.len() > 8 * 1024 {
        return Err("Expanded task script exceeds the 8 KiB terminal limit".into());
    }
    Ok(Plan { steps, script })
}
fn read(root: &Path) -> Result<Option<Vec<Value>>, String> {
    let path = root.join(".vscode/tasks.json");
    let meta = match fs::metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    if meta.len() > 128 * 1024 {
        return Err("VS Code task file exceeds 128 KiB".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(128 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 128 * 1024 {
        return Err("VS Code task file exceeds 128 KiB".into());
    }
    let bytes = normalize_jsonc(&bytes).ok_or("Invalid task JSONC")?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if value["version"].as_str() != Some("2.0.0") {
        return Err("VS Code tasks require version 2.0.0".into());
    }
    if ["command", "args", "options", "windows", "linux", "osx"]
        .iter()
        .any(|k| value.get(*k).is_some())
    {
        return Err(
            "Top-level task defaults/overrides are unsupported; put them on individual tasks"
                .into(),
        );
    }
    let tasks = value["tasks"]
        .as_array()
        .ok_or("Task file needs a tasks array")?;
    if tasks.len() > 50 {
        return Err("VS Code task file exceeds 50 tasks".into());
    }
    Ok(Some(tasks.clone()))
}
pub fn show(
    ui: &mut egui::Ui,
    root: &Path,
    active: Option<&Path>,
    page: Page,
    state: &mut State,
) -> Option<String> {
    state.refresh(root, active);
    if (state.stamp.is_some() || state.error.is_some())
        && ui.small_button("Refresh task plans").clicked()
    {
        state.context = None;
        state.refresh(root, active);
    }
    if let Some(error) = &state.error {
        ui.colored_label(egui::Color32::LIGHT_RED, format!("VS Code tasks: {error}"));
        return None;
    }
    if state.cards.is_empty() {
        return None;
    }
    ui.label("VS Code task pipelines");
    ui.small("Review every step. Runs in a new PowerShell terminal, stops on failure; problem matchers and automatic/background tasks are not active.");
    for card in &state.cards {
        let group = card.group.as_deref();
        let shown = matches!(
            (page, group),
            (Page::Workflows, Some("build") | None)
                | (Page::Tests, Some("test"))
                | (Page::RunDebug, Some("run"))
        );
        if !shown {
            continue;
        }
        let label = &card.label;
        match &card.plan {
            Ok(plan) => {
                ui.group(|ui| {
                    ui.label(format!("{label} — {} sequential step(s)", plan.steps.len()));
                    for (i, step) in plan.steps.iter().enumerate() {
                        ui.label(format!("{}. {}", i + 1, step.label));
                        ui.monospace(&step.command);
                        ui.small(format!("Folder: {}", step.cwd.display()));
                        for (key, value) in &step.env {
                            ui.small(format!(
                                "Environment: {key} = {}",
                                value.as_deref().unwrap_or("<unset>")
                            ));
                        }
                    }
                });
                if ui
                    .add_enabled(cfg!(windows), egui::Button::new(format!("Run {label}")))
                    .clicked()
                {
                    return Some(plan.script.clone());
                }
                if !cfg!(windows) {
                    ui.small("Imported pipelines currently require Windows PowerShell.");
                }
            }
            Err(error) => {
                ui.group(|ui| {
                    ui.label(label);
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                });
            }
        }
        ui.add_space(6.0);
    }
    None
}
