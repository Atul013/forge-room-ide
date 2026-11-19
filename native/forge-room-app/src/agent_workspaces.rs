//! Explicit Git workspaces for agent tasks. Never resets, stashes, merges or deletes user work.
use crate::agent::Provider;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub source: PathBuf,
    pub folder: PathBuf,
    #[serde(default)]
    pub project: PathBuf,
    pub branch: String,
    pub common: PathBuf,
}
#[derive(Default)]
pub struct State {
    pub enabled: bool,
    root: Option<PathBuf>,
    pub records: Vec<Workspace>,
    selected: Option<usize>,
    name: String,
    pending: Option<Receiver<Result<Workspace, String>>>,
    sessions: Vec<(PathBuf, Option<String>, Option<String>)>,
    pub open_requested: Option<PathBuf>,
    summary: Option<String>,
    summary_rx: Option<Receiver<Result<String, String>>>,
}
fn base() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Forge Room Early Preview")
        .join("agent-workspaces")
}
fn key(root: &Path) -> String {
    let text = root
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let hash = text.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}
fn store(root: &Path) -> PathBuf {
    base().join(format!("{}.json", key(root)))
}
fn read_small(path: &Path) -> Result<String, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 4096 {
        return Err("Git metadata is too large".into());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}
fn common(root: &Path) -> Result<PathBuf, String> {
    for folder in root.ancestors() {
        let marker = folder.join(".git");
        if marker.is_dir() {
            return marker.canonicalize().map_err(|e| e.to_string());
        }
        if marker.is_file() {
            let line = read_small(&marker)?;
            let value = line
                .trim()
                .strip_prefix("gitdir: ")
                .ok_or("Invalid Git worktree metadata")?;
            let dir = folder
                .join(value)
                .canonicalize()
                .map_err(|e| e.to_string())?;
            let pointer = dir.join("commondir");
            return if pointer.is_file() {
                dir.join(read_small(&pointer)?.trim())
                    .canonicalize()
                    .map_err(|e| e.to_string())
            } else {
                Ok(dir)
            };
        }
    }
    Err("Open a Git repository before creating an agent workspace".into())
}
fn reparse(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_type().is_symlink() || meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}
impl Workspace {
    fn project_path(&self) -> &Path {
        if self.project.as_os_str().is_empty() {
            &self.folder
        } else {
            &self.project
        }
    }
    pub fn validate(&self, root: &Path) -> Result<PathBuf, String> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        if root != self.source || common(&root)? != self.common {
            return Err("Workspace belongs to another source repository".into());
        }
        let owned = base().canonicalize().map_err(|e| e.to_string())?;
        let folder = self.folder.canonicalize().map_err(|e| e.to_string())?;
        if !folder.starts_with(&owned) || folder == root {
            return Err("Agent workspace is outside its owned folder".into());
        }
        let mut target = owned.clone();
        for part in folder
            .strip_prefix(&owned)
            .map_err(|e| e.to_string())?
            .components()
        {
            target.push(part);
            if reparse(&fs::symlink_metadata(&target).map_err(|e| e.to_string())?) {
                return Err("Linked agent workspace is unsupported".into());
            }
        }
        let marker = folder.join(".git");
        if reparse(&fs::symlink_metadata(&marker).map_err(|e| e.to_string())?) {
            return Err("Linked Git metadata is unsupported".into());
        }
        let pointer = read_small(&marker)?;
        let gitdir = folder
            .join(
                pointer
                    .trim()
                    .strip_prefix("gitdir: ")
                    .ok_or("Invalid worktree marker")?,
            )
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if gitdir.parent() != Some(self.common.join("worktrees").as_path())
            || gitdir
                .join("..")
                .canonicalize()
                .map_err(|e| e.to_string())?
                != self
                    .common
                    .join("worktrees")
                    .canonicalize()
                    .map_err(|e| e.to_string())?
        {
            return Err("Git worktree registration changed".into());
        }
        let backlink = PathBuf::from(read_small(&gitdir.join("gitdir"))?.trim())
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if backlink != marker.canonicalize().map_err(|e| e.to_string())?
            || read_small(&gitdir.join("HEAD"))?.trim()
                != format!("ref: refs/heads/{}", self.branch)
        {
            return Err("Worktree path or branch changed since creation".into());
        }
        if common(&folder)? != self.common {
            return Err("Worktree repository changed".into());
        }
        let project = self.project_path();
        let relative = project
            .strip_prefix(&folder)
            .map_err(|_| "Project folder is outside its worktree")?;
        let mut current = folder.clone();
        for part in relative.components() {
            if !matches!(part, std::path::Component::Normal(_)) {
                return Err("Invalid project folder".into());
            }
            current.push(part);
            if reparse(&fs::symlink_metadata(&current).map_err(|e| e.to_string())?) {
                return Err("Linked project folder is unsupported".into());
            }
        }
        let project = current.canonicalize().map_err(|e| e.to_string())?;
        if !project.is_dir() {
            return Err("Task project folder is missing".into());
        }
        Ok(project)
    }
}
pub(crate) fn create(root: &Path, name: &str) -> Result<Workspace, String> {
    let source = root.canonicalize().map_err(|e| e.to_string())?;
    let common = common(&source)?;
    let top = crate::git::run(&source, &["rev-parse", "--show-toplevel"])?;
    let top = PathBuf::from(top.trim())
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let relative = source
        .strip_prefix(&top)
        .map_err(|_| "Project is outside its Git root")?
        .to_path_buf();
    if !relative.as_os_str().is_empty() {
        let tree = format!("HEAD:{}", relative.to_string_lossy().replace('\\', "/"));
        crate::git::run(&source, &["cat-file", "-e", &tree]).map_err(
            |_| "Project folder is not present in HEAD; commit or open the repository root first",
        )?;
    }
    if name.trim().is_empty() || name.chars().count() > 80 {
        return Err("Task name must contain 1 to 80 characters".into());
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let id = format!("{}-{stamp}", std::process::id());
    let branch = format!("forge/agent-{id}");
    let parent = base().join(key(&source));
    fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    let owned = base().canonicalize().map_err(|e| e.to_string())?;
    if reparse(&fs::symlink_metadata(&parent).map_err(|e| e.to_string())?) {
        return Err("Linked workspace storage is unsupported".into());
    }
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    if !parent.starts_with(&owned) {
        return Err("Workspace storage is outside its owned folder".into());
    }
    let folder = parent.join(&id);
    // Git for Windows does not accept a verbatim \\?\ path for worktree add.
    let git_folder = crate::tabs::normalize(&folder);
    if folder.exists() {
        return Err("Agent workspace already exists".into());
    }
    crate::git::run(
        &source,
        &[
            "worktree",
            "add",
            "-b",
            &branch,
            "--",
            &git_folder.to_string_lossy(),
            "HEAD",
        ],
    )?;
    let workspace = Workspace {
        name: name.trim().into(),
        source,
        folder: folder.canonicalize().map_err(|e| e.to_string())?,
        project: folder
            .join(relative)
            .canonicalize()
            .map_err(|e| e.to_string())?,
        branch,
        common,
    };
    workspace.validate(&workspace.source)?;
    Ok(workspace)
}
impl State {
    pub fn creating(&self) -> bool {
        self.pending.is_some()
    }
    pub fn adopt(&mut self, workspace: Workspace) -> Result<(), String> {
        if self.root.as_ref() != Some(&workspace.source) {
            return Err("Source folder changed while creating workspace".into());
        }
        if let Some(index) = self
            .records
            .iter()
            .position(|w| w.folder == workspace.folder)
        {
            self.selected = Some(index);
        } else {
            self.records.push(workspace);
            self.selected = Some(self.records.len() - 1);
        }
        self.enabled = true;
        self.save()
    }
    pub fn register_workflow(&mut self, workspace: Workspace) -> Result<(), String> {
        let root = self.root.as_ref().ok_or("Open the source project first")?;
        workspace.validate(root)?;
        if !self
            .records
            .iter()
            .any(|record| record.folder == workspace.folder)
        {
            self.records.push(workspace);
        }
        self.save()
    }
    pub fn load(&mut self, root: &Path) {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        if self.root.as_ref() == Some(&root) {
            return;
        }
        *self = Self::default();
        self.root = Some(root.clone());
        let path = store(&root);
        if fs::metadata(&path).is_ok_and(|m| m.len() <= 128 * 1024)
            && let Ok(text) = fs::read_to_string(path)
            && let Ok(records) = serde_json::from_str::<Vec<Workspace>>(&text)
        {
            self.records = records
                .into_iter()
                .filter(|w| w.source == root && w.name.len() <= 320 && w.branch.len() <= 100)
                .take(32)
                .collect();
        }
    }
    fn save(&self) -> Result<(), String> {
        let root = self.root.as_deref().ok_or("No source folder")?;
        fs::create_dir_all(base()).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec(&self.records).map_err(|e| e.to_string())?;
        fs::write(store(root), bytes).map_err(|e| e.to_string())
    }
    pub fn target(&self, root: &Path) -> Result<Option<PathBuf>, String> {
        if !self.enabled {
            return Ok(None);
        }
        let record = self
            .selected
            .and_then(|i| self.records.get(i))
            .ok_or("Create or choose an isolated agent workspace first")?;
        record.validate(root).map(Some)
    }
    pub fn session(&self, path: &Path, provider: Provider) -> Option<String> {
        self.sessions
            .iter()
            .find(|s| s.0 == path)
            .and_then(|(_, claude, codex)| {
                if provider == Provider::Claude {
                    claude.clone()
                } else if provider == Provider::Codex {
                    codex.clone()
                } else {
                    None
                }
            })
    }
    pub fn record_session(&mut self, path: PathBuf, provider: Provider, id: Option<String>) {
        if !self.sessions.iter().any(|s| s.0 == path) {
            self.sessions.push((path.clone(), None, None));
        }
        if let Some((_, claude, codex)) = self.sessions.iter_mut().find(|s| s.0 == path) {
            match provider {
                Provider::Claude => *claude = id,
                Provider::Codex => *codex = id,
                _ => {}
            }
        }
    }
    pub fn new_chat(&mut self) {
        if let Some(folder) = self
            .selected
            .and_then(|i| self.records.get(i))
            .map(|r| r.project_path().to_path_buf())
        {
            self.sessions.retain(|s| s.0 != folder);
        }
    }
    pub fn poll(&mut self) -> Vec<String> {
        let mut notes = Vec::new();
        if let Some(receiver) = &self.pending
            && let Ok(result) = receiver.try_recv()
        {
            match result {
                Ok(workspace) => {
                    self.name.clear();
                    notes.push("Isolated workspace created from HEAD; primary uncommitted work is unchanged".into());
                    if let Err(error) = self.adopt(workspace) {
                        notes.push(format!("Workspace created but list was not saved: {error}"));
                    }
                }
                Err(error) => notes.push(format!("Workspace not created: {error}")),
            }
            self.pending = None;
        }
        if let Some(receiver) = &self.summary_rx
            && let Ok(result) = receiver.try_recv()
        {
            match result {
                Ok(summary) => self.summary = Some(summary),
                Err(error) => notes.push(error),
            }
            self.summary_rx = None;
        }
        notes
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        root: Option<&Path>,
        trusted: bool,
        busy: bool,
    ) -> Vec<String> {
        let mut notes = self.poll();
        egui::CollapsingHeader::new("Isolated agent workspaces").show(ui,|ui|{
            ui.label("Create a separate Git branch and checkout from HEAD. Primary uncommitted files and unsaved buffers are not copied or changed. Repository hooks/filters may run during checkout; only create from trusted repositories.");
            ui.label("Direct-agent runs can target this checkout using the CLI permissions selected above. This is Git isolation, not an OS security sandbox. Ask/Plan/Reviewed edits keep their original folder scope.");
            ui.add_enabled_ui(!busy,|ui|{ui.checkbox(&mut self.enabled,"Run Direct agent in selected isolated workspace");});
            ui.label("Task name");ui.add(egui::TextEdit::singleline(&mut self.name).char_limit(80).interactive(!busy));
            if ui.add_enabled(trusted&&!busy&&root.is_some()&&!self.name.trim().is_empty()&&self.records.len()<32&&self.pending.is_none(),egui::Button::new("Create isolated workspace from HEAD")).clicked()&&let Some(root)=root{let root=root.to_path_buf();let name=self.name.clone();let (sender,receiver)=mpsc::sync_channel(1);self.pending=Some(receiver);self.selected=None;self.enabled=true;let context=ui.ctx().clone();std::thread::spawn(move||{let _=sender.send(create(&root,&name));context.request_repaint();});}
            if self.pending.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));}
            for (index,record) in self.records.iter().enumerate(){ui.add_enabled_ui(!busy,|ui|{ui.selectable_value(&mut self.selected,Some(index),format!("{} | {}",record.name,record.branch));});}
            if let Some(record)=self.selected.and_then(|i|self.records.get(i)){
                ui.label(record.project_path().display().to_string());
                if ui.add_enabled(trusted,egui::Button::new("Open isolated folder in another window")).clicked()&&let Some(root)=root{match record.validate(root){Ok(path)=>self.open_requested=Some(path),Err(error)=>notes.push(error)}}
                if ui.add_enabled(trusted&&self.summary_rx.is_none(),egui::Button::new("Summarize isolated changes")).clicked()&&let Some(root)=root{match record.validate(root){Ok(path)=>{let (sender,receiver)=mpsc::sync_channel(1);self.summary_rx=Some(receiver);let context=ui.ctx().clone();std::thread::spawn(move||{let result=crate::git::run(&path,&["status","--short"]).and_then(|status|crate::git::run(&path,&["diff","--no-ext-diff","--no-textconv","--stat","HEAD"]).map(|diff|format!("{status}\n{diff}").chars().take(50_000).collect()));let _=sender.send(result);context.request_repaint();});},Err(error)=>notes.push(error)}}
            }
            if self.summary_rx.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));}
            if let Some(summary)=&self.summary{egui::ScrollArea::vertical().id_salt("isolated-change-summary").max_height(180.0).show(ui,|ui|{ui.label(summary);});}
            ui.small("Agent Stop ends the owned process; workspace files and branch remain for review. No automatic merge, commit, cleanup or deletion. Open the isolated folder to review diffs, run tests and decide how to integrate.");
        });
        notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn isolated_checkout_keeps_dirty_source_and_revalidates_registration() {
        let root =
            std::env::temp_dir().join(format!("forge-worktree-policy-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        crate::git::run(&root, &["init"]).unwrap();
        crate::git::run(&root, &["config", "core.autocrlf", "false"]).unwrap();
        crate::git::run(&root, &["config", "user.email", "test@example.invalid"]).unwrap();
        crate::git::run(&root, &["config", "user.name", "Test"]).unwrap();
        fs::write(root.join("main.txt"), "committed\n").unwrap();
        fs::create_dir_all(root.join("component")).unwrap();
        fs::write(root.join("component/data.txt"), "component\n").unwrap();
        crate::git::run(&root, &["add", "main.txt", "component/data.txt"]).unwrap();
        crate::git::run(&root, &["commit", "-m", "base"]).unwrap();
        fs::write(root.join("main.txt"), "primary unsaved-on-disk change\n").unwrap();
        let workspace = create(&root, "Test task").unwrap();
        assert_ne!(workspace.folder, root);
        assert_eq!(
            fs::read_to_string(workspace.folder.join("main.txt")).unwrap(),
            "committed\n"
        );
        fs::write(workspace.folder.join("main.txt"), "agent task edit\n").unwrap();
        assert_eq!(
            fs::read_to_string(root.join("main.txt")).unwrap(),
            "primary unsaved-on-disk change\n"
        );
        workspace.validate(&root).unwrap();
        let mut state = State::default();
        state.load(&root);
        state.adopt(workspace.clone()).unwrap();
        state.record_session(
            workspace.project.clone(),
            Provider::Codex,
            Some("scoped-session".into()),
        );
        let mut loaded = State::default();
        loaded.load(&root);
        assert_eq!(loaded.records.len(), 1);
        assert!(
            loaded
                .session(&workspace.project, Provider::Codex)
                .is_none()
        );
        loaded.enabled = true;
        loaded.selected = Some(0);
        assert_eq!(
            loaded.target(&root).unwrap(),
            Some(workspace.project.clone())
        );
        let component = root.join("component");
        let nested = create(&component, "Nested project").unwrap();
        assert_eq!(
            nested.validate(&component).unwrap(),
            nested.folder.join("component")
        );
        assert_eq!(
            fs::read_to_string(nested.project.join("data.txt")).unwrap(),
            "component\n"
        );
        crate::git::run(
            &component,
            &[
                "worktree",
                "remove",
                "--force",
                &crate::tabs::normalize(&nested.folder).to_string_lossy(),
            ],
        )
        .unwrap();
        let mut forged = workspace.clone();
        forged.folder = root.clone();
        assert!(forged.validate(&root).is_err());
        // Test-only removal of this owned fixture is explicit; the product never removes worktrees.
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
        assert!(workspace.validate(&root).is_err());
        let _ = fs::remove_file(store(&root.canonicalize().unwrap()));
        fs::remove_dir_all(root).unwrap();
    }
}
