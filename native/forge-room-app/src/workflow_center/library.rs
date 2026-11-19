//! Versioned project definitions; no imported command becomes an unrestricted process.
use super::resolver;
use forge_room_core::workflows::{self, CheckRecipe, Policy, Step, StepKind, When, Workflow};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub version: u32,
    pub workflows: Vec<Workflow>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            workflows: workflows::starter_workflows(),
        }
    }
}
impl Library {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.workflows.len() > 32 {
            return Err("Workflow library supports version 1 and at most 32 workflows".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for workflow in &self.workflows {
            workflow.validate()?;
            if !ids.insert(&workflow.id) {
                return Err("Workflow identities must be unique".into());
            }
        }
        Ok(())
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > workflows::MAX_SPEC_BYTES {
            return Err("Workflow import exceeds 1 MiB".into());
        }
        let library: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        library.validate()?;
        Ok(library)
    }
    pub fn append(&mut self, mut imported: Self) -> Result<(), String> {
        imported.validate()?;
        if self.workflows.len() + imported.workflows.len() > 32 {
            return Err("Library limit is 32 workflows".into());
        }
        for workflow in &mut imported.workflows {
            let base = workflow.id.chars().take(35).collect::<String>();
            let mut suffix = 1;
            while self
                .workflows
                .iter()
                .any(|existing| existing.id == workflow.id)
            {
                workflow.id = format!("{base}-import-{suffix}");
                suffix += 1;
            }
            self.workflows.push(workflow.clone());
        }
        Ok(())
    }
}
pub fn location(root: &Path) -> PathBuf {
    root.join(".forge-room/workflows.json")
}
pub fn read(path: &Path) -> Result<Vec<u8>, String> {
    let seal = resolver::file_seal(path, workflows::MAX_SPEC_BYTES as u64)?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(workflows::MAX_SPEC_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if workflows::digest(&bytes) != seal.digest {
        return Err("Definition changed while reading".into());
    }
    Ok(bytes)
}
pub fn load(root: &Path) -> Result<(Library, Option<String>), String> {
    let file = location(root);
    if !file.exists() {
        return Ok((Library::default(), None));
    }
    resolver::path(root, ".forge-room/workflows.json", true)?;
    let bytes = read(&file)?;
    Ok((Library::parse(&bytes)?, Some(workflows::digest(&bytes))))
}
pub fn save(
    root: &Path,
    library: &Library,
    expected: Option<&str>,
    backup: &Path,
) -> Result<String, String> {
    library.validate()?;
    let file = resolver::path(root, ".forge-room/workflows.json", false)?;
    let bytes = serde_json::to_vec_pretty(library).map_err(|e| e.to_string())?;
    if bytes.len() > workflows::MAX_SPEC_BYTES {
        return Err("Library exceeds 1 MiB".into());
    }
    let old = if file.exists() {
        let old = read(&file)?;
        if Some(workflows::digest(&old).as_str()) != expected {
            return Err(
                "Project definitions changed outside this editor; import/reload before saving"
                    .into(),
            );
        }
        Some(old)
    } else {
        if expected.is_some() {
            return Err("Project definitions were removed outside this editor".into());
        }
        None
    };
    if let Some(old) = &old {
        let count = fs::read_dir(backup)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("library-"))
            .count();
        if count >= 32 {
            return Err("32 private definition backups retained; archive/remove obsolete library backups before saving again".into());
        }
        let target = backup.join(format!("library-{}.json", workflows::new_id()?));
        create_file(&target, old)?;
    }
    let parent = file.parent().ok_or("No definition parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    resolver::path(root, ".forge-room", true)?;
    if old.is_none() {
        create_file(&file, &bytes)?;
    } else {
        let temp = parent.join(format!(".workflow-{}.tmp", workflows::new_id()?));
        create_file(&temp, &bytes)?;
        // Recheck immediately before atomic replacement; never fall back to in-place writing.
        if workflows::digest(&read(&file)?) != expected.unwrap_or_default() {
            let _ = fs::remove_file(&temp);
            return Err("Definitions changed during save".into());
        }
        if let Err(error) = fs::rename(&temp, &file) {
            let _ = fs::remove_file(&temp);
            return Err(format!("Atomic definition save failed: {error}"));
        }
    }
    Ok(workflows::digest(&bytes))
}
pub fn create_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
pub fn vscode(bytes: &[u8]) -> Result<Library, String> {
    if bytes.len() > workflows::MAX_SPEC_BYTES {
        return Err("Task import exceeds 1 MiB".into());
    }
    let normalized = crate::project_tools::normalize_jsonc(bytes).ok_or("Invalid task JSONC")?;
    let source: serde_json::Value =
        serde_json::from_slice(&normalized).map_err(|e| e.to_string())?;
    let tasks = source
        .get("tasks")
        .and_then(|v| v.as_array())
        .ok_or("No VS Code tasks array")?;
    if tasks.is_empty() || tasks.len() > 128 {
        return Err("Import needs 1â€“128 tasks".into());
    }
    let mut steps = Vec::new();
    let mut labels = std::collections::BTreeMap::new();
    for (index, task) in tasks.iter().enumerate() {
        let label = task
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported task");
        if labels
            .insert(label.to_owned(), format!("task-{}", index + 1))
            .is_some()
        {
            return Err("Imported task labels must be unique".into());
        }
    }
    for (index, task) in tasks.iter().enumerate() {
        let command = task.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let args = task
            .get("args")
            .and_then(|v| v.as_array())
            .and_then(|v| v.iter().map(|v| v.as_str()).collect::<Option<Vec<_>>>());
        let recipe = match (command, args.as_deref()) {
            ("cargo", Some(["check"])) => Some(CheckRecipe::CargoCheck),
            ("cargo", Some(["test"])) => Some(CheckRecipe::CargoTest),
            ("cargo", Some(["build"])) => Some(CheckRecipe::CargoBuild),
            ("node", Some(["--test"])) => Some(CheckRecipe::NodeTest),
            ("go", Some(["test", "./..."])) => Some(CheckRecipe::GoTest),
            ("go", Some(["vet", "./..."])) => Some(CheckRecipe::GoVet),
            ("python" | "python3", Some(["-m", "pytest", "-q"])) => Some(CheckRecipe::Pytest),
            _ => None,
        };
        let safe = task.get("type").and_then(|v| v.as_str()) == Some("process")
            && !task
                .get("isBackground")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            && task.get("options").is_none()
            && task.get("windows").is_none()
            && task.get("linux").is_none()
            && task.get("osx").is_none();
        let action = if let Some(recipe) = recipe.filter(|_| safe) {
            StepKind::Check { recipe }
        } else {
            let original = serde_json::to_string_pretty(task).map_err(|e| e.to_string())?;
            if original.len() > 3500 {
                return Err("Unsupported task definition is too large for a complete manual gate; simplify it before importing".into());
            }
            StepKind::Manual {
                instructions: format!(
                    "Imported task requires manual execution and review. Unsupported shell, arguments, background mode, environment, cwd or platform override. Original task:\n{original}"
                ),
            }
        };
        let depends = task.get("dependsOn");
        let dependencies: Vec<&str> = match depends {
            None => vec![],
            Some(serde_json::Value::String(value)) => vec![value],
            Some(serde_json::Value::Array(values)) => values
                .iter()
                .map(|v| v.as_str().ok_or("Non-string dependency"))
                .collect::<Result<_, _>>()?,
            _ => return Err("Unsupported task dependency".into()),
        };
        let needs = dependencies
            .into_iter()
            .map(|label| {
                labels
                    .get(label)
                    .cloned()
                    .ok_or_else(|| format!("Unknown imported dependency {label}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        steps.push(Step {
            id: format!("task-{}", index + 1),
            title: task
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("Imported task")
                .to_owned(),
            action,
            needs,
            when: When::Success,
            context: "primary".into(),
            cwd: ".".into(),
            inputs: vec![],
            artifacts: vec![],
            resources: vec![],
            timeout_seconds: 120,
            retries: 0,
            optional: false,
            review_each_attempt: true,
            room_task: None,
            room_id: None,
        });
    }
    let library = Library {
        version: 1,
        workflows: vec![Workflow {
            version: 1,
            id: "imported-tasks".into(),
            title: "Imported project tasks".into(),
            description: "Exact supported process recipes; other commands require manual gates"
                .into(),
            parameters: vec![],
            steps,
            policy: Policy::default(),
        }],
    };
    library.validate()?;
    Ok(library)
}
