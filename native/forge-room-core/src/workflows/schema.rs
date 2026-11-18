use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_STEPS: usize = 128;
pub const MAX_SPEC_BYTES: usize = 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckRecipe {
    CargoCheck,
    CargoTest,
    PythonCompile,
    Pytest,
    CargoBuild,
    CargoFmtCheck,
    CargoClippy,
    CargoDoc,
    PythonUnittest,
    NodeTest,
    GoTest,
    GoVet,
    DotnetBuild,
    DotnetTest,
}
impl CheckRecipe {
    pub const ALL: [Self; 14] = [
        Self::CargoCheck,
        Self::CargoTest,
        Self::PythonCompile,
        Self::Pytest,
        Self::CargoBuild,
        Self::CargoFmtCheck,
        Self::CargoClippy,
        Self::CargoDoc,
        Self::PythonUnittest,
        Self::NodeTest,
        Self::GoTest,
        Self::GoVet,
        Self::DotnetBuild,
        Self::DotnetTest,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::CargoCheck => "cargo check --all-targets",
            Self::CargoTest => "cargo test",
            Self::PythonCompile => "Python compile selected .py files",
            Self::Pytest => "python -m pytest -q",
            Self::CargoBuild => "cargo build",
            Self::CargoFmtCheck => "cargo fmt --all -- --check",
            Self::CargoClippy => "cargo clippy --all-targets -- -D warnings",
            Self::CargoDoc => "cargo doc --no-deps",
            Self::PythonUnittest => "python -m unittest discover -v",
            Self::NodeTest => "node --test",
            Self::GoTest => "go test ./...",
            Self::GoVet => "go vet ./...",
            Self::DotnetBuild => "dotnet build",
            Self::DotnetTest => "dotnet test",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StepKind {
    AiReview {
        prompt: String,
        #[serde(default)]
        include_statuses: bool,
        #[serde(default)]
        include_inputs: bool,
        max_tokens: u32,
    },
    Check {
        recipe: CheckRecipe,
    },
    Manual {
        instructions: String,
    },
    Approval {
        prompt: String,
    },
    Checkpoint {
        label: String,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum When {
    #[default]
    Success,
    Failure,
    Always,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    #[default]
    Text,
    Choice,
    Boolean,
    RelativePath,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub kind: ParameterKind,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub choices: Vec<String>,
}
fn primary() -> String {
    "primary".into()
}
fn dot() -> String {
    ".".into()
}
fn seconds() -> u64 {
    120
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub action: StepKind,
    #[serde(default)]
    pub needs: Vec<String>,
    #[serde(default)]
    pub when: When,
    #[serde(default = "primary")]
    pub context: String,
    #[serde(default = "dot")]
    pub cwd: String,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default = "seconds")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub retries: u32,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub review_each_attempt: bool,
    #[serde(default)]
    pub room_task: Option<String>,
    #[serde(default)]
    pub room_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub parallelism: usize,
    pub max_attempts: u32,
    pub wall_seconds: u64,
    pub output_bytes: u64,
    pub artifact_bytes: u64,
    pub continue_independent: bool,
    pub allow_retry: bool,
    pub model_calls: u32,
    pub model_output_tokens: u64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            parallelism: 1,
            max_attempts: 32,
            wall_seconds: 1800,
            output_bytes: 4 * 1024 * 1024,
            artifact_bytes: 8 * 1024 * 1024,
            continue_independent: false,
            allow_retry: false,
            model_calls: 0,
            model_output_tokens: 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    pub version: u32,
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub policy: Policy,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expanded {
    pub workflow: Workflow,
    pub parameters: BTreeMap<String, String>,
    pub targets: Vec<String>,
    pub included: BTreeSet<String>,
    pub seal: String,
}
impl Expanded {
    pub fn validate(&self) -> Result<(), String> {
        self.workflow.validate()?;
        if !self.workflow.parameters.is_empty()
            || self.parameters.len() > 32
            || self.targets.is_empty()
            || self.targets.len() > MAX_STEPS
        {
            return Err("Invalid expanded parameter/target set".into());
        }
        for (name, value) in &self.parameters {
            if !identifier(name) {
                return Err("Invalid expanded parameter".into());
            }
            text(value, 512, false)?;
        }
        let mut closure: BTreeSet<String> = self.targets.iter().cloned().collect();
        if closure.len() != self.targets.len()
            || closure
                .iter()
                .any(|id| !self.workflow.steps.iter().any(|step| &step.id == id))
        {
            return Err("Invalid expanded targets".into());
        }
        loop {
            let before = closure.len();
            for step in &self.workflow.steps {
                if closure.contains(&step.id) {
                    closure.extend(step.needs.iter().cloned());
                }
            }
            if before == closure.len() {
                break;
            }
        }
        if closure != self.included
            || super::seal(&(
                &self.workflow,
                &self.parameters,
                &self.targets,
                &self.included,
            ))? != self.seal
        {
            return Err("Expanded dependency closure or seal changed".into());
        }
        Ok(())
    }
}

pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
pub fn text(value: &str, max: usize, required: bool) -> Result<(), String> {
    if value.len() > max || value.contains('\0') || (required && value.trim().is_empty()) {
        Err("Text is empty, contains NUL or exceeds its limit".into())
    } else {
        Ok(())
    }
}
pub fn relative(value: &str) -> Result<(), String> {
    if value == "." {
        return Ok(());
    }
    if value.is_empty()
        || value.len() > 1024
        || value.contains(['\\', ':'])
        || value.chars().any(char::is_control)
        || value.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || [".git", ".ssh", "id_rsa", "id_ed25519"]
                    .contains(&part.to_ascii_lowercase().as_str())
                || part.to_ascii_lowercase().starts_with(".env")
                || [".key", ".pem", ".pfx", ".p12"]
                    .iter()
                    .any(|suffix| part.to_ascii_lowercase().ends_with(suffix))
        })
    {
        return Err(
            "Use a bounded relative path without traversal or credential/repository files".into(),
        );
    }
    Ok(())
}
impl Policy {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=4).contains(&self.parallelism)
            || !(1..=512).contains(&self.max_attempts)
            || !(1..=86400).contains(&self.wall_seconds)
            || !(1024..=32 * 1024 * 1024).contains(&self.output_bytes)
            || self.artifact_bytes > 64 * 1024 * 1024
            || self.model_calls > 8
            || self.model_output_tokens > 262144
        {
            return Err("Workflow run limits are invalid".into());
        }
        Ok(())
    }
}
impl Workflow {
    pub fn validate(&self) -> Result<Vec<String>, String> {
        if self.version != 1
            || !identifier(&self.id)
            || self.steps.is_empty()
            || self.steps.len() > MAX_STEPS
            || self.parameters.len() > 32
        {
            return Err("Invalid workflow version, ID or size".into());
        }
        text(&self.title, 200, true)?;
        text(&self.description, 4000, false)?;
        self.policy.validate()?;
        let mut parameter_names = BTreeSet::new();
        for parameter in &self.parameters {
            if !identifier(&parameter.name)
                || !parameter_names.insert(&parameter.name)
                || parameter.choices.len() > 32
            {
                return Err("Invalid or repeated parameter".into());
            }
            text(&parameter.label, 200, true)?;
            for choice in &parameter.choices {
                text(choice, 512, true)?;
            }
            if parameter.kind == ParameterKind::Choice && parameter.choices.is_empty() {
                return Err("Choice parameter needs choices".into());
            }
            if let Some(value) = &parameter.default {
                parameter.validate_value(value)?;
            }
        }
        let mut ids = BTreeSet::new();
        for step in &self.steps {
            if !identifier(&step.id)
                || !ids.insert(step.id.clone())
                || !identifier(&step.context)
                || step.needs.len() > 32
                || step.inputs.len() > 64
                || step.artifacts.len() > 16
                || step.resources.len() > 8
                || step.retries > 4
                || !(1..=3600).contains(&step.timeout_seconds)
            {
                return Err("Invalid step identifiers or limits".into());
            }
            text(&step.title, 200, true)?;
            text(&step.cwd, 1024, true)?;
            if step.needs.iter().collect::<BTreeSet<_>>().len() != step.needs.len()
                || step.needs.contains(&step.id)
            {
                return Err("Repeated or self dependency".into());
            }
            for resource in &step.resources {
                if !identifier(resource) {
                    return Err("Invalid resource lock name".into());
                }
            }
            if step
                .room_task
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > 16)
            {
                return Err("Invalid Room task reference".into());
            }
            match &step.action {
                _ if step.room_id.as_ref().is_some_and(|id| {
                    id.is_empty() || id.len() > 128 || id.chars().any(char::is_control)
                }) =>
                {
                    return Err("Invalid Room scope".into())
                }
                StepKind::AiReview {
                    prompt, max_tokens, ..
                } => {
                    text(prompt, 16384, true)?;
                    if !(128..=32768).contains(max_tokens) {
                        return Err("AI review token limit is invalid".into());
                    }
                }
                StepKind::Manual { instructions } => text(instructions, 4000, true)?,
                StepKind::Approval { prompt } => text(prompt, 4000, true)?,
                StepKind::Checkpoint { label } => text(label, 200, true)?,
                StepKind::Check { .. } => (),
            }
            for path in std::iter::once(&step.cwd)
                .chain(&step.inputs)
                .chain(&step.artifacts)
            {
                validate_tokens(path, &parameter_names)?;
                if !path.contains("${") {
                    relative(path)?;
                }
            }
        }
        let mut ordered = Vec::new();
        let mut done = BTreeSet::new();
        while done.len() < ids.len() {
            let before = done.len();
            for step in &self.steps {
                if !done.contains(&step.id) && step.needs.iter().all(|id| done.contains(id)) {
                    done.insert(step.id.clone());
                    ordered.push(step.id.clone());
                }
            }
            if before == done.len() {
                return Err("Workflow contains a dependency cycle or missing step".into());
            }
        }
        Ok(ordered)
    }
    pub fn expand(
        &self,
        supplied: &BTreeMap<String, String>,
        targets: &[String],
    ) -> Result<Expanded, String> {
        self.validate()?;
        let mut parameters = BTreeMap::new();
        if supplied.keys().any(|key| {
            !self
                .parameters
                .iter()
                .any(|parameter| &parameter.name == key)
        }) {
            return Err("Unknown workflow parameter".into());
        }
        for parameter in &self.parameters {
            let value = supplied
                .get(&parameter.name)
                .cloned()
                .or_else(|| parameter.default.clone())
                .unwrap_or_default();
            parameter.validate_value(&value)?;
            parameters.insert(parameter.name.clone(), value);
        }
        let mut workflow = self.clone();
        for step in &mut workflow.steps {
            step.cwd = substitute(&step.cwd, &parameters)?;
            relative(&step.cwd)?;
            for path in step.inputs.iter_mut().chain(&mut step.artifacts) {
                *path = substitute(path, &parameters)?;
                relative(path)?;
            }
            match &mut step.action {
                StepKind::AiReview { prompt, .. } => *prompt = substitute(prompt, &parameters)?,
                StepKind::Manual { instructions } => {
                    *instructions = substitute(instructions, &parameters)?
                }
                StepKind::Approval { prompt } => *prompt = substitute(prompt, &parameters)?,
                StepKind::Checkpoint { label } => *label = substitute(label, &parameters)?,
                _ => (),
            }
        }
        workflow.parameters.clear();
        workflow.validate()?;
        let targets = if targets.is_empty() {
            workflow.steps.iter().map(|step| step.id.clone()).collect()
        } else {
            targets.to_vec()
        };
        if targets.iter().collect::<BTreeSet<_>>().len() != targets.len()
            || targets
                .iter()
                .any(|id| !workflow.steps.iter().any(|step| &step.id == id))
        {
            return Err("Unknown or repeated target".into());
        }
        let mut included: BTreeSet<String> = targets.iter().cloned().collect();
        loop {
            let before = included.len();
            for step in &workflow.steps {
                if included.contains(&step.id) {
                    included.extend(step.needs.iter().cloned());
                }
            }
            if included.len() == before {
                break;
            }
        }
        let seal = super::seal(&(&workflow, &parameters, &targets, &included))?;
        Ok(Expanded {
            workflow,
            parameters,
            targets,
            included,
            seal,
        })
    }
}
impl Parameter {
    pub fn validate_value(&self, value: &str) -> Result<(), String> {
        text(value, 512, self.required)?;
        if value.is_empty() && !self.required {
            return Ok(());
        }
        match self.kind {
            ParameterKind::Boolean if !["true", "false"].contains(&value) => {
                Err("Boolean parameters use true or false".into())
            }
            ParameterKind::Choice if !self.choices.iter().any(|choice| choice == value) => {
                Err("Choose a declared parameter value".into())
            }
            ParameterKind::RelativePath => relative(value),
            _ => Ok(()),
        }
    }
}
fn validate_tokens(value: &str, names: &BTreeSet<&String>) -> Result<(), String> {
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let tail = &rest[start + 2..];
        let end = tail.find('}').ok_or("Unclosed parameter token")?;
        if !names.iter().any(|name| name.as_str() == &tail[..end]) {
            return Err("Unknown parameter token".into());
        }
        rest = &tail[end + 1..];
    }
    Ok(())
}
pub fn substitute(value: &str, parameters: &BTreeMap<String, String>) -> Result<String, String> {
    let mut rest = value;
    let mut result = String::new();
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let tail = &rest[start + 2..];
        let end = tail.find('}').ok_or("Unclosed parameter token")?;
        result.push_str(
            parameters
                .get(&tail[..end])
                .ok_or("Unknown parameter token")?,
        );
        rest = &tail[end + 1..];
    }
    result.push_str(rest);
    text(&result, 8192, false)?;
    Ok(result)
}
