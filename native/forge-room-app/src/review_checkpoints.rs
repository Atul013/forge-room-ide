//! Durable, workspace-scoped recovery copies for app-reviewed file replacements.
//! Nothing is restored automatically, and unknown/dirty contents are never overwritten.
use crate::lsp_rename::{FileChange, Preview};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::{SystemTime, UNIX_EPOCH},
};
const MAX_BYTES: u64 = 8 * 1024 * 1024;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SavedFile {
    path: String,
    original: String,
    applied: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Journal {
    version: u32,
    id: String,
    workspace: PathBuf,
    created: u64,
    files: Vec<SavedFile>,
}
struct Entry {
    id: String,
    created: u64,
    files: usize,
}
#[derive(Default)]
pub struct State {
    root: Option<PathBuf>,
    entries: Vec<Entry>,
    loading: Option<Receiver<Result<Vec<Entry>, String>>>,
    selected: Option<Journal>,
    notice: Option<String>,
    confirm_delete: bool,
    pub restore_requested: bool,
}
fn base() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Forge Room Early Preview")
        .join("review-checkpoints")
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
fn directory(root: &Path, create: bool) -> Result<PathBuf, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let text = root
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let hash = text.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    let dir = base().join(format!("{hash:016x}"));
    if create {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    if dir.exists() {
        if reparse(&fs::symlink_metadata(&dir).map_err(|e| e.to_string())?) {
            return Err("Linked checkpoint storage is unsupported".into());
        }
        let owned = base().canonicalize().map_err(|e| e.to_string())?;
        let resolved = dir.canonicalize().map_err(|e| e.to_string())?;
        if !resolved.starts_with(owned) {
            return Err("Checkpoint storage is outside its owned folder".into());
        }
        return Ok(resolved);
    }
    Ok(dir)
}
fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
fn target(root: &Path, relative: &str, allow_missing: bool) -> Result<PathBuf, String> {
    // Reuse the portable credential/path boundary, not its content scanner.
    forge_room_core::transport::validate_patch(relative, "old", "new")?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let parts: Vec<_> = Path::new(relative).components().collect();
    let mut path = root;
    for (index, part) in parts.iter().enumerate() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err("Invalid checkpoint target".into());
        }
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                if reparse(&meta) {
                    return Err("Linked checkpoint targets are unsupported".into());
                }
                if index + 1 == parts.len() && !meta.is_file() {
                    return Err("Checkpoint target is not a file".into());
                }
                if index + 1 < parts.len() && !meta.is_dir() {
                    return Err("Checkpoint parent is not a directory".into());
                }
            }
            Err(error)
                if allow_missing
                    && index + 1 == parts.len()
                    && error.kind() == std::io::ErrorKind::NotFound =>
            {
                return Ok(path);
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(path)
}
fn read(root: &Path, id: &str) -> Result<Journal, String> {
    if !safe_id(id) {
        return Err("Invalid checkpoint ID".into());
    }
    let path = directory(root, false)?.join(format!("{id}.json"));
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if reparse(&meta) || meta.len() > MAX_BYTES {
        return Err("Checkpoint is linked or oversized".into());
    }
    let saved: Journal = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|_| "Checkpoint could not be read")?;
    if saved.version != 1
        || saved.id != id
        || saved.workspace != root.canonicalize().map_err(|e| e.to_string())?
        || saved.files.is_empty()
        || saved.files.len() > 8
    {
        return Err("Checkpoint does not match this workspace".into());
    }
    let mut paths = std::collections::HashSet::new();
    for file in &saved.files {
        forge_room_core::transport::validate_patch(&file.path, "old", "new")?;
        if !paths.insert(&file.path)
            || file.original.len() > 128 * 1024
            || file.applied.len() > 128 * 1024
            || file.original.contains('\0')
            || file.applied.contains('\0')
        {
            return Err("Invalid checkpoint file data".into());
        }
    }
    Ok(saved)
}
pub fn save(root: &Path, preview: &Preview) -> Result<String, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if preview.files.is_empty() || preview.files.len() > 8 {
        return Err("Checkpoint requires one to eight files".into());
    }
    let dir = directory(&root, true)?;
    if fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|e| e == "json"))
        .count()
        >= 32
    {
        return Err("Recovery storage has 32 checkpoints; review and delete an unneeded copy before applying more edits".into());
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    let id = format!("{}-{}", stamp.as_nanos(), std::process::id());
    let mut files = Vec::new();
    for file in &preview.files {
        let relative = file
            .path
            .strip_prefix(&root)
            .map_err(|_| "Checkpoint target is outside the workspace")?
            .to_string_lossy()
            .replace('\\', "/");
        target(&root, &relative, false)?;
        if file.before.len() > 128 * 1024
            || file.after.len() > 128 * 1024
            || file.before.contains('\0')
            || file.after.contains('\0')
        {
            return Err("Checkpoint file exceeds its text limit".into());
        }
        files.push(SavedFile {
            path: relative,
            original: file.before.clone(),
            applied: file.after.clone(),
        });
    }
    let saved = Journal {
        version: 1,
        id: id.clone(),
        workspace: root,
        created: stamp.as_secs(),
        files,
    };
    let bytes = serde_json::to_vec(&saved).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Checkpoint exceeds 8 MiB".into());
    }
    let path = dir.join(format!("{id}.json"));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            format!("No files applied: recovery copy could not be created: {error}")
        })?;
    let result = (|| {
        output.write_all(&bytes)?;
        output.sync_all()
    })();
    drop(output);
    if let Err(error) = result {
        let _ = fs::remove_file(path);
        return Err(format!(
            "No files applied: recovery copy could not be saved: {error}"
        ));
    }
    Ok(id)
}
fn list(root: &Path) -> Result<Vec<Entry>, String> {
    let dir = directory(root, false)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for file in fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .take(128)
        .filter_map(Result::ok)
    {
        if file.path().extension().is_some_and(|e| e == "json")
            && let Some(id) = file.path().file_stem().and_then(|s| s.to_str())
            && let Ok(saved) = read(root, id)
        {
            entries.push(Entry {
                id: saved.id,
                created: saved.created,
                files: saved.files.len(),
            });
        }
    }
    entries.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id)));
    Ok(entries)
}
fn restore(root: &Path, saved: &Journal, dirty: &[PathBuf]) -> Result<usize, String> {
    if saved.workspace != root.canonicalize().map_err(|e| e.to_string())? {
        return Err("Workspace changed; select its recovery copy again".into());
    }
    let mut changes = Vec::new();
    let mut missing = Vec::new();
    for file in &saved.files {
        let path = target(root, &file.path, true)?;
        if dirty
            .iter()
            .any(|p| crate::tabs::normalize(p) == crate::tabs::normalize(&path))
        {
            return Err("Save edited buffers before recovery".into());
        }
        if !path.exists() {
            missing.push((path, file.original.clone()));
            continue;
        }
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 128 * 1024 {
            return Err("A target changed after the checkpoint; no files were restored".into());
        }
        let current = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if current == file.original {
            continue;
        }
        if current != file.applied {
            return Err(format!(
                "{} changed after this checkpoint; inspect the saved original instead of overwriting it",
                file.path
            ));
        }
        changes.push(FileChange {
            path,
            before: current,
            after: file.original.clone(),
            lines: Vec::new(),
        });
    }
    let mut count = changes.len();
    if !changes.is_empty() {
        crate::lsp_rename::apply(&Preview {
            edit_count: changes.len(),
            files: changes,
        })?;
    }
    for (index, (path, text)) in missing.into_iter().enumerate() {
        let relative = path
            .strip_prefix(&saved.workspace)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        target(root, &relative, true)?;
        let staged = path.with_extension(format!("forge-recovery-{}-{index}.tmp", saved.id));
        let mut owned_stage = false;
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&staged)?;
            owned_stage = true;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::hard_link(&staged, &path)
        })();
        if owned_stage {
            let _ = fs::remove_file(staged);
        }
        if let Err(error) = result {
            return Err(format!(
                "Restored {count} file(s); a missing file could not be recreated safely. Recovery copy retained: {error}"
            ));
        }
        count += 1;
    }
    Ok(count)
}
impl State {
    fn load_list(&mut self, root: &Path, context: &egui::Context) {
        let root = root.to_path_buf();
        let (sender, receiver) = mpsc::sync_channel(1);
        self.loading = Some(receiver);
        let context = context.clone();
        std::thread::spawn(move || {
            let _ = sender.send(list(&root));
            context.request_repaint();
        });
    }
    pub fn restore_selected(&mut self, root: &Path, dirty: &[PathBuf]) -> Result<usize, String> {
        let selected = self
            .selected
            .as_ref()
            .ok_or("Select a recovery checkpoint first")?;
        let current = read(root, &selected.id)?;
        if &current != selected {
            return Err("Recovery copy changed after review; reload it before restoring".into());
        }
        let result = restore(root, &current, dirty);
        self.notice = Some(match &result {
            Ok(count) => format!("Recovered {count} file(s); checkpoint retained"),
            Err(error) => error.clone(),
        });
        result
    }
    pub fn show(&mut self, ui: &mut egui::Ui, root: Option<&Path>, trusted: bool, busy: bool) {
        let canonical = root.and_then(|p| p.canonicalize().ok());
        if canonical != self.root {
            *self = Self::default();
            self.root = canonical;
        }
        if let Some(receiver) = &self.loading
            && let Ok(result) = receiver.try_recv()
        {
            match result {
                Ok(entries) => self.entries = entries,
                Err(error) => self.notice = Some(error),
            }
            self.loading = None;
        }
        egui::CollapsingHeader::new("Recovery checkpoints").show(ui,|ui|{
            ui.label("Before reviewed AI or Room patch edits are applied or undone, a local recovery copy is saved. It survives New Chat and app restart. Direct CLI edits are not journaled here.");
            if ui.add_enabled(root.is_some()&&self.loading.is_none(),egui::Button::new("Load / refresh checkpoints")).clicked()&&let Some(root)=root{self.load_list(root,ui.ctx());}
            if self.loading.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
            if let Some(notice)=&self.notice{ui.label(notice);}
            for entry in &self.entries{if ui.button(format!("{} file(s), {}",entry.files,crate::room_state::ago(forge_room_core::now_epoch_seconds(),entry.created))).on_hover_text(&entry.id).clicked()&&let Some(root)=root{match read(root,&entry.id){Ok(saved)=>{self.selected=Some(saved);self.confirm_delete=false;},Err(error)=>self.notice=Some(error)}}}
            let mut delete=false;
            if let Some(saved)=&self.selected{
                ui.label(format!("Recovery copy {}",saved.id));
                for file in &saved.files{egui::CollapsingHeader::new(&file.path).show(ui,|ui|{ui.label("Saved original");let mut text=file.original.as_str();egui::ScrollArea::vertical().id_salt((&file.path,"checkpoint-original")).max_height(160.0).show(ui,|ui|{ui.add(egui::TextEdit::multiline(&mut text).code_editor().interactive(false).desired_width(f32::INFINITY));});if ui.button("Copy saved original").clicked(){ui.ctx().copy_text(file.original.clone());}});}
                if ui.add_enabled(trusted&&!busy,egui::Button::new("Restore unchanged checkpoint targets")).clicked(){self.restore_requested=true;}
                ui.checkbox(&mut self.confirm_delete,"I no longer need this recovery copy");delete=ui.add_enabled(self.confirm_delete&&!busy,egui::Button::new("Delete selected recovery copy")).clicked();
            }
            if delete&&let Some(root)=root&&let Some(saved)=&self.selected{let result=read(root,&saved.id).and_then(|current|{if &current!=saved{return Err("Recovery copy changed after review; reload it before deletion".into());}fs::remove_file(directory(root,false)?.join(format!("{}.json",current.id))).map_err(|e|e.to_string())});match result{Ok(())=>{self.selected=None;self.restore_requested=false;self.confirm_delete=false;self.load_list(root,ui.ctx());},Err(error)=>self.notice=Some(error)}}
            ui.small("Copies contain source text in local app storage, never Room transport. Recovery skips files already restored, recreates missing files without replacing new work, and refuses changed or dirty targets. At most 32 copies are retained; deletion is explicit.");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durable_checkpoint_recovers_partial_apply_and_preserves_later_work() {
        let root =
            std::env::temp_dir().join(format!("forge-durable-checkpoint-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let files = vec![
            FileChange {
                path: root.join("one.txt"),
                before: "one".into(),
                after: "changed one".into(),
                lines: vec![],
            },
            FileChange {
                path: root.join("two.txt"),
                before: "two".into(),
                after: "changed two".into(),
                lines: vec![],
            },
        ];
        fs::write(root.join("one.txt"), "one").unwrap();
        fs::write(root.join("two.txt"), "two").unwrap();
        let id = save(
            &root,
            &Preview {
                files,
                edit_count: 2,
            },
        )
        .unwrap();
        let saved = read(&root, &id).unwrap();
        let manifest = directory(&root, false).unwrap().join(format!("{id}.json"));
        let mut manager = State {
            selected: Some(saved.clone()),
            ..Default::default()
        };
        let mut modified = saved.clone();
        modified.files[0].original = "unreviewed replacement".into();
        fs::write(&manifest, serde_json::to_vec(&modified).unwrap()).unwrap();
        assert!(
            manager
                .restore_selected(&root, &[])
                .unwrap_err()
                .contains("changed after review")
        );
        fs::write(&manifest, serde_json::to_vec(&saved).unwrap()).unwrap();
        assert_eq!(list(&root).unwrap().len(), 1);
        fs::write(root.join("one.txt"), "changed one").unwrap();
        fs::remove_file(root.join("two.txt")).unwrap();
        assert!(restore(&root, &saved, &[root.join("one.txt")]).is_err());
        assert_eq!(restore(&root, &saved, &[]).unwrap(), 2);
        assert_eq!(fs::read_to_string(root.join("two.txt")).unwrap(), "two");
        fs::write(root.join("one.txt"), "user's later edit").unwrap();
        assert!(restore(&root, &saved, &[]).is_err());
        assert_eq!(
            fs::read_to_string(root.join("one.txt")).unwrap(),
            "user's later edit"
        );
        fs::write(root.join("one.txt"), "changed one").unwrap();
        fs::remove_file(root.join("two.txt")).unwrap();
        let preexisting = root
            .join("two.txt")
            .with_extension(format!("forge-recovery-{id}-0.tmp"));
        fs::write(&preexisting, "preexisting data").unwrap();
        assert!(restore(&root, &saved, &[]).is_err());
        assert_eq!(
            fs::read_to_string(&preexisting).unwrap(),
            "preexisting data"
        );
        fs::remove_file(preexisting).unwrap();
        assert_eq!(restore(&root, &saved, &[]).unwrap(), 1);
        fs::remove_file(directory(&root, false).unwrap().join(format!("{id}.json"))).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
