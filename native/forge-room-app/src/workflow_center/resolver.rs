//! Local review seals. Checks paths and inputs; no process is started here.
use crate::controlled_build::validation::{Plan, Recipe};
use forge_room_core::workflows::{self, Expanded, StepKind};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSeal {
    pub path: PathBuf,
    pub bytes: u64,
    pub digest: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub alias: String,
    pub label: String,
    pub root: PathBuf,
    pub source: PathBuf,
    #[serde(default)]
    pub worktree: Option<crate::agent_workspaces::Workspace>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSeal {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub root: PathBuf,
    pub recipe: Recipe,
    pub binary: FileSeal,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewedStep {
    pub id: String,
    pub context: Context,
    pub cwd: PathBuf,
    pub command: Option<CommandSeal>,
    pub inputs: Vec<FileSeal>,
    pub artifacts: Vec<String>,
    pub room_task: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub version: u32,
    pub source: PathBuf,
    pub expanded: Expanded,
    pub steps: BTreeMap<String, ReviewedStep>,
    pub source_file: Option<FileSeal>,
    pub seal: String,
    pub reviewed_at: u64,
    pub seal_tracked: bool,
}

pub use crate::local_records::linked;
pub fn canonical(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))
}
pub fn path(root: &Path, relative: &str, must_exist: bool) -> Result<PathBuf, String> {
    workflows::relative(relative)?;
    let root = canonical(root)?;
    let mut candidate = root.clone();
    if relative != "." {
        for part in relative.split('/') {
            candidate.push(part);
            match fs::symlink_metadata(&candidate) {
                Ok(meta) => {
                    if linked(&meta) {
                        return Err("Linked workflow paths are not supported".into());
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound && !must_exist => (),
                Err(error) => return Err(error.to_string()),
            }
        }
    }
    if must_exist {
        let resolved = canonical(&candidate)?;
        if !resolved.starts_with(&root) {
            return Err("Workflow path escaped its reviewed context".into());
        }
        Ok(resolved)
    } else {
        Ok(candidate)
    }
}
pub fn file_seal(path: &Path, max: u64) -> Result<FileSeal, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if linked(&metadata) || !metadata.is_file() || metadata.len() > max {
        return Err("Workflow input/tool is linked, non-regular or too large".into());
    }
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("Workflow input grew beyond its limit".into());
    }
    Ok(FileSeal {
        path: canonical(path)?,
        bytes: bytes.len() as u64,
        digest: workflows::digest(&bytes),
    })
}
pub fn unchanged(seal: &FileSeal, max: u64) -> bool {
    file_seal(&seal.path, max).is_ok_and(|current| current == *seal)
}
fn manifests(cwd: &Path, recipe: Recipe) -> Vec<PathBuf> {
    let names: &[&str] = match recipe {
        Recipe::CargoCheck
        | Recipe::CargoTest
        | Recipe::CargoBuild
        | Recipe::CargoFmtCheck
        | Recipe::CargoClippy
        | Recipe::CargoDoc => &["Cargo.toml", "Cargo.lock", "build.rs"],
        Recipe::PythonCompile | Recipe::Pytest | Recipe::PythonUnittest => &[
            "pyproject.toml",
            "pytest.ini",
            "setup.cfg",
            "requirements.txt",
        ],
        Recipe::NodeTest => &["package.json", "package-lock.json"],
        Recipe::GoTest | Recipe::GoVet => &["go.mod", "go.sum"],
        Recipe::DotnetBuild | Recipe::DotnetTest => &[
            "global.json",
            "Directory.Build.props",
            "Directory.Build.targets",
            "NuGet.Config",
        ],
    };
    names
        .iter()
        .map(|name| cwd.join(name))
        .filter(|path| path.is_file())
        .collect()
}
pub fn review(
    source: &Path,
    expanded: Expanded,
    contexts: &BTreeMap<String, Context>,
    source_file: Option<PathBuf>,
    dirty: &[PathBuf],
    seal_tracked: bool,
) -> Result<Review, String> {
    let source = canonical(source)?;
    expanded.validate()?;
    if !dirty.is_empty() {
        return Err("Save or close dirty buffers before reviewing an executable workflow".into());
    }
    let source_file = source_file
        .map(|path| file_seal(&path, workflows::MAX_SPEC_BYTES as u64))
        .transpose()?;
    let mut steps = BTreeMap::new();
    let mut programs = BTreeMap::<PathBuf, FileSeal>::new();
    let mut total = 0u64;
    for definition in expanded
        .workflow
        .steps
        .iter()
        .filter(|step| expanded.included.contains(&step.id))
    {
        let context = contexts
            .get(&definition.context)
            .ok_or_else(|| format!("Map execution context {}", definition.context))?
            .clone();
        if canonical(&context.source)? != source || canonical(&context.root)? != context.root {
            return Err("Execution context changed or belongs to another project".into());
        }
        if let Some(worktree) = &context.worktree
            && worktree.validate(&source)? != context.root
        {
            return Err("Registered worktree context changed".into());
        }
        let cwd = path(&context.root, &definition.cwd, true)?;
        if !cwd.is_dir() {
            return Err("Step cwd must be a directory".into());
        }
        let input_paths = definition
            .inputs
            .iter()
            .map(|relative| path(&cwd, relative, true))
            .collect::<Result<Vec<_>, _>>()?;
        let command = if let StepKind::Check { recipe } = definition.action {
            let plan = Plan::resolve(&cwd, recipe, &input_paths)?;
            let program = canonical(&plan.program)?;
            let binary = if let Some(binary) = programs.get(&program) {
                binary.clone()
            } else {
                let binary = file_seal(&program, 256 * 1024 * 1024)?;
                programs.insert(program.clone(), binary.clone());
                binary
            };
            Some(CommandSeal {
                program,
                args: plan.args,
                root: plan.root,
                recipe,
                binary,
            })
        } else {
            None
        };
        let mut inputs = BTreeMap::<PathBuf, FileSeal>::new();
        let mut inferred = command
            .as_ref()
            .map(|command| manifests(&cwd, command.recipe))
            .unwrap_or_default();
        if let Some(command) = &command {
            if let Some(index) = command.args.iter().position(|arg| arg == "--manifest-path")
                && let Some(file) = command.args.get(index + 1)
            {
                inferred.push(PathBuf::from(file));
            }
            if matches!(command.recipe, Recipe::DotnetBuild | Recipe::DotnetTest)
                && let Some(file) = command.args.get(1)
            {
                inferred.push(PathBuf::from(file));
            }
        }
        for input in input_paths.into_iter().chain(inferred) {
            let input = canonical(&input)?;
            let relative = input
                .strip_prefix(&context.root)
                .map_err(|_| "Input is outside context")?
                .to_string_lossy()
                .replace('\\', "/");
            let checked = path(&context.root, &relative, true)?;
            let seal = file_seal(&checked, 8 * 1024 * 1024)?;
            inputs.insert(checked, seal);
        }
        if seal_tracked {
            for input in tracked(&cwd)? {
                let relative = input
                    .strip_prefix(&context.root)
                    .map_err(|_| "Tracked file is outside context")?
                    .to_string_lossy()
                    .replace('\\', "/");
                let checked = path(&context.root, &relative, true)?;
                let seal = file_seal(&checked, 8 * 1024 * 1024)?;
                inputs.insert(checked, seal);
            }
        }
        total = total.saturating_add(inputs.values().map(|input| input.bytes).sum::<u64>());
        if total > 32 * 1024 * 1024 || inputs.len() > 512 {
            return Err("Reviewed input seal exceeds 32 MiB / 512 files per step".into());
        }
        for artifact in &definition.artifacts {
            path(&cwd, artifact, false)?;
        }
        steps.insert(
            definition.id.clone(),
            ReviewedStep {
                id: definition.id.clone(),
                context,
                cwd,
                command,
                inputs: inputs.into_values().collect(),
                artifacts: definition.artifacts.clone(),
                room_task: definition.room_task.clone(),
            },
        );
    }
    let seal = workflows::seal(&(&source, &expanded, &steps, &source_file, seal_tracked))?;
    let review = Review {
        version: 1,
        source,
        expanded,
        steps,
        source_file,
        seal,
        reviewed_at: forge_room_core::now_epoch_seconds(),
        seal_tracked,
    };
    review.validate()?;
    Ok(review)
}
impl Review {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.steps.len() != self.expanded.included.len()
            || self.steps.len() > workflows::MAX_STEPS
            || self.seal
                != workflows::seal(&(
                    &self.source,
                    &self.expanded,
                    &self.steps,
                    &self.source_file,
                    self.seal_tracked,
                ))?
        {
            return Err("Reviewed workflow record changed".into());
        }
        self.expanded.validate()?;
        if self
            .steps
            .values()
            .map(|step| step.inputs.len())
            .sum::<usize>()
            > 2048
            || serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 2 * 1024 * 1024
        {
            return Err("Review metadata exceeds 2048 sealed entries / 2 MiB".into());
        }
        for (id, step) in &self.steps {
            if id != &step.id
                || !self.expanded.included.contains(id)
                || step.inputs.len() > 512
                || step.artifacts.len() > 16
            {
                return Err("Invalid reviewed step".into());
            }
        }
        Ok(())
    }
    pub fn check_source(&self) -> Result<(), String> {
        self.validate()?;
        if let Some(source) = &self.source_file
            && !unchanged(source, workflows::MAX_SPEC_BYTES as u64)
        {
            return Err("Workflow source file changed; review a fresh plan".into());
        }
        Ok(())
    }
    pub fn verify_step(&self, id: &str) -> Result<Option<Plan>, String> {
        self.check_source()?;
        let step = self.steps.get(id).ok_or("Step was not reviewed")?;
        if let Some(worktree) = &step.context.worktree {
            if worktree.validate(&self.source)? != step.context.root {
                return Err("Registered worktree changed since review".into());
            }
        } else if step.context.root != self.source || step.context.alias != "primary" {
            return Err("Unregistered execution context".into());
        }
        let definition = self
            .expanded
            .workflow
            .steps
            .iter()
            .find(|definition| definition.id == id)
            .ok_or("Missing definition")?;
        if path(&step.context.root, &definition.cwd, true)? != step.cwd {
            return Err("Step working directory changed".into());
        }
        if canonical(&step.context.root)? != step.context.root || canonical(&step.cwd)? != step.cwd
        {
            return Err("Execution root changed".into());
        }
        for input in &step.inputs {
            let relative = input
                .path
                .strip_prefix(&step.context.root)
                .map_err(|_| "Input escaped context")?
                .to_string_lossy()
                .replace('\\', "/");
            if path(&step.context.root, &relative, true)? != input.path
                || !unchanged(input, 8 * 1024 * 1024)
            {
                return Err(format!("Reviewed input changed: {}", input.path.display()));
            }
        }
        if let Some(command) = &step.command {
            let StepKind::Check { recipe } = definition.action else {
                return Err("Unexpected saved command".into());
            };
            let inputs = definition
                .inputs
                .iter()
                .map(|input| path(&step.cwd, input, true))
                .collect::<Result<Vec<_>, _>>()?;
            let resolved = Plan::resolve(&step.cwd, recipe, &inputs)?;
            if canonical(&resolved.program)? != command.program
                || resolved.args != command.args
                || resolved.root != command.root
                || resolved.recipe != command.recipe
            {
                return Err("Closed recipe/tool resolution changed".into());
            }
            if !unchanged(&command.binary, 256 * 1024 * 1024) {
                return Err("Reviewed executable changed".into());
            }
            return Ok(Some(Plan {
                program: command.program.clone(),
                args: command.args.clone(),
                root: command.root.clone(),
                recipe: command.recipe,
            }));
        }
        if matches!(definition.action, StepKind::Check { .. }) {
            return Err("Check is missing its reviewed command".into());
        }
        Ok(None)
    }
    pub fn inputs_unchanged(&self, id: &str) -> bool {
        self.steps.get(id).is_some_and(|step| {
            if let Some(worktree) = &step.context.worktree
                && worktree.validate(&self.source).ok().as_ref() != Some(&step.context.root)
            {
                return false;
            }
            step.inputs.iter().all(|input| {
                let relative = input
                    .path
                    .strip_prefix(&step.context.root)
                    .ok()
                    .map(|path| path.to_string_lossy().replace('\\', "/"));
                relative.is_some_and(|relative| {
                    path(&step.context.root, &relative, true).is_ok_and(|path| path == input.path)
                }) && unchanged(input, 8 * 1024 * 1024)
            })
        })
    }
}
fn tracked(cwd: &Path) -> Result<Vec<PathBuf>, String> {
    let git = crate::controlled_build::validation::find("git")
        .ok_or("Git is unavailable for tracked-file sealing")?;
    let mut command = std::process::Command::new(git);
    command
        .args(["-C", &cwd.to_string_lossy(), "ls-files", "-z", "--cached"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let output = child.stdout.take().ok_or("Git stdout unavailable")?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output
            .take(512 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = tx.send(result);
    });
    let result = rx.recv_timeout(std::time::Duration::from_secs(5));
    let bytes = match result {
        Ok(Ok(bytes)) if bytes.len() <= 512 * 1024 => bytes,
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Tracked file listing timed out, failed or exceeded its limit".into());
        }
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Git listing did not exit".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    if !status.success() {
        return Err("Git could not list tracked files".into());
    }
    let mut result = Vec::new();
    for name in bytes
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
    {
        let name =
            std::str::from_utf8(name).map_err(|_| "Non-UTF8 tracked paths are unsupported")?;
        result.push(path(cwd, name, true)?);
        if result.len() > 512 {
            return Err("Tracked-file sealing is limited to 512 files".into());
        }
    }
    Ok(result)
}
