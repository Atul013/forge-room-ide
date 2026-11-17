//! Local-only syntactic hints from saved tracked diffs and declared task ownership.
use eframe::egui;
use forge_room_core::{
    TaskStatus,
    transport::{SharedSnapshot, Snapshot},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc,
};
#[derive(Clone)]
pub struct Suggestion {
    pub source: String,
    pub target: String,
    pub file: String,
    pub base: String,
    pub summary: String,
}
#[derive(Default)]
struct ResultSet {
    items: Vec<Suggestion>,
    warnings: Vec<String>,
    limited: bool,
}
struct Job {
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    root: PathBuf,
    actor: String,
    room: String,
    rx: mpsc::Receiver<Result<ResultSet, String>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Release);
    }
}
#[derive(Default)]
pub struct State {
    scope: Option<(PathBuf, String, String)>,
    job: Option<Job>,
    result: Option<ResultSet>,
    notice: String,
}
fn scan(
    root: &Path,
    shared: &SharedSnapshot,
    actor: &str,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<ResultSet, String> {
    let started = std::time::Instant::now();
    if cancel.load(std::sync::atomic::Ordering::Acquire) {
        return Err("Scan canceled".into());
    }
    let base = crate::git::run(root, &["rev-parse", "--verify", "HEAD"])?
        .trim()
        .to_owned();
    if ![40, 64].contains(&base.len()) || !base.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("No full Git base commit is available".into());
    }
    let mut result = ResultSet::default();
    let mut files = HashMap::<String, bool>::new();
    for profile in shared
        .readiness
        .profiles
        .iter()
        .filter(|p| p.owner == actor)
    {
        let Some(source) = shared
            .tasks
            .iter()
            .find(|t| t.id == profile.task && t.owner_id == actor && t.status != TaskStatus::Done)
        else {
            continue;
        };
        for file in &profile.profile.files {
            if cancel.load(std::sync::atomic::Ordering::Acquire)
                || started.elapsed() > std::time::Duration::from_secs(60)
            {
                return Err(
                    "Scan canceled or reached its time window; existing suggestions are discarded"
                        .into(),
                );
            }
            let changed = if let Some(changed) = files.get(file) {
                *changed
            } else {
                if files.len() >= 16 {
                    result.limited = true;
                    continue;
                }
                let checked = crate::ai_workflow::source(root, &root.join(file)).and_then(|_| {
                    let working = crate::git::diff(root, file, false)?;
                    if cancel.load(std::sync::atomic::Ordering::Acquire) {
                        return Err("Scan canceled".into());
                    }
                    let staged = crate::git::diff(root, file, true)?;
                    Ok(!working.trim().is_empty() || !staged.trim().is_empty())
                });
                let changed = match checked {
                    Ok(value) => value,
                    Err(error) => {
                        result.warnings.push(format!("{file}: {error}"));
                        false
                    }
                };
                files.insert(file.clone(), changed);
                changed
            };
            if !changed {
                continue;
            }
            for target in shared
                .tasks
                .iter()
                .filter(|t| t.id != source.id && t.status != TaskStatus::Done)
            {
                let dependency = target.dependencies.contains(&source.id);
                let overlap = shared
                    .readiness
                    .profile(&target.id)
                    .filter(|p| p.owner == target.owner_id)
                    .is_some_and(|p| p.profile.files.contains(file));
                if !dependency && !overlap {
                    continue;
                }
                if result.items.len() >= 64 {
                    result.limited = true;
                    return Ok(result);
                }
                result.items.push(Suggestion{source:source.id.clone(),target:target.id.clone(),file:file.clone(),base:base.clone(),summary:format!("Local saved staged/working change observed in the linked file; target {}. Base commit {base}; uncommitted diff is not shared. Confirm semantic impact before publishing.",if dependency{"depends on source task"}else{"links the same file"})});
            }
        }
    }
    Ok(result)
}
impl State {
    fn start(&mut self, root: &Path, snapshot: &Snapshot) -> Result<(), String> {
        if self.job.is_some() {
            return Err("Impact scan is already running".into());
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let workspace = root.clone();
        let actor = snapshot.you.clone();
        let worker_actor = actor.clone();
        let shared = snapshot.shared.clone();
        let (tx, rx) = mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        std::thread::spawn(move || {
            let _ = tx.send(scan(&workspace, &shared, &worker_actor, &worker_cancel));
        });
        self.scope = Some((root.clone(), actor.clone(), snapshot.room.clone()));
        self.job = Some(Job {
            cancel,
            root,
            actor,
            room: snapshot.room.clone(),
            rx,
        });
        self.result = None;
        self.notice =
            "Scanning up to 16 linked existing files locally; no network action is queued".into();
        Ok(())
    }
    fn poll(&mut self, root: Option<&Path>, snapshot: &Snapshot) {
        let Some(job) = &self.job else { return };
        let result = match job.rx.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(_) => Err("Impact worker stopped".into()),
        };
        let valid = root.and_then(|r| r.canonicalize().ok()).as_ref() == Some(&job.root)
            && job.actor == snapshot.you
            && job.room == snapshot.room;
        self.job = None;
        if !valid {
            self.result = None;
            self.notice = "Discarded scan after workspace/identity/trust changed".into();
            return;
        }
        match result {
            Ok(result) => {
                self.notice = format!(
                    "{} local suggestion(s); no impact has been published",
                    result.items.len()
                );
                self.result = Some(result)
            }
            Err(e) => self.notice = e,
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        root: Option<&Path>,
        snapshot: &Snapshot,
    ) -> Option<Suggestion> {
        let current = root
            .and_then(|r| r.canonicalize().ok())
            .map(|r| (r, snapshot.you.clone(), snapshot.room.clone()));
        if self.scope.is_some() && self.scope != current {
            *self = Self::default();
        }
        self.poll(root, snapshot);
        let mut selected = None;
        egui::CollapsingHeader::new("Local Git impact suggestions").show(ui,|ui|{
            ui.small("Explicit scan of saved tracked diffs for current-owner linked files. Inference uses declared dependencies/file overlap, not semantic analysis or authorship detection. Base commit is a reference, not the uncommitted diff digest. Deleted, unsafe, untracked and oversized files are outside this scan.");
            if ui.add_enabled(root.is_some()&&self.job.is_none(),egui::Button::new("Scan linked saved changes locally")).clicked()&&let Some(root)=root&&let Err(e)=self.start(root,snapshot){self.notice=e;}
            if let Some(result)=&self.result{if result.limited{ui.label("Scan limited to 16 files / 64 suggestions; absence of a notice does not prove no impact.");}for warning in &result.warnings{ui.label(warning);}for suggestion in &result.items{ui.label(format!("{} -> {}: {}",suggestion.source,suggestion.target,suggestion.file));if ui.button(format!("Review suggestion {} -> {} / {} (replaces impact form)",suggestion.source,suggestion.target,suggestion.file)).clicked(){selected=Some(suggestion.clone());}}}
            if self.job.is_some()&&ui.button("Cancel scan / discard (after current Git call)").clicked(){self.job=None;self.result=None;self.notice="Scan canceled; current Git call may finish before the worker exits".into();}
            if self.job.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}ui.label(&self.notice);
        });
        selected
    }
}
#[cfg(test)]
mod tests;
