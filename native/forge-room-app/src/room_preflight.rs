//! Owner-approved local checks. Only an explicit status summary can enter Room evidence.
use crate::{
    ai_workflow::{self, Source},
    controlled_build::validation::{Evidence, Plan, Recipe, Run},
};
use eframe::egui;
use forge_room_core::{
    EvidenceKind, TaskStatus,
    transport::{Action, Snapshot},
};
use std::path::{Path, PathBuf};
struct Scope {
    root: PathBuf,
    actor: String,
    task: String,
    revision: u64,
    files: Vec<Source>,
}
impl Scope {
    fn new(root: &Path, snapshot: &Snapshot, task: &str) -> Result<Self, String> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let t = snapshot
            .shared
            .tasks
            .iter()
            .find(|t| t.id == task && t.owner_id == snapshot.you && t.status != TaskStatus::Done)
            .ok_or("Select your own unfinished task")?;
        let profile = snapshot
            .shared
            .readiness
            .profile(&t.id)
            .filter(|p| p.owner == snapshot.you)
            .ok_or("Publish current-owner readiness first")?;
        let files = profile
            .profile
            .files
            .iter()
            .map(|file| ai_workflow::source(&root, &root.join(file)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            root,
            actor: snapshot.you.clone(),
            task: task.into(),
            revision: profile.revision,
            files,
        })
    }
    fn valid(&self, root: Option<&Path>, snapshot: &Snapshot, task: &str) -> bool {
        root.and_then(|r| r.canonicalize().ok()).as_ref() == Some(&self.root)
            && task == self.task
            && snapshot.you == self.actor
            && snapshot.shared.tasks.iter().any(|t| {
                t.id == self.task && t.owner_id == self.actor && t.status != TaskStatus::Done
            })
            && snapshot
                .shared
                .readiness
                .profile(&self.task)
                .is_some_and(|p| p.owner == self.actor && p.revision == self.revision)
    }
    fn unchanged(&self) -> bool {
        self.files.iter().all(|f| {
            ai_workflow::source(&self.root, &f.path)
                .is_ok_and(|s| s.path == f.path && s.before == f.before)
        })
    }
}
struct Receipt {
    command: String,
    limit_seconds: u64,
    started_epoch_seconds: u64,
    evidence: Evidence,
    unchanged: bool,
}
fn summary(task: &str, revision: u64, receipt: &Receipt) -> String {
    let e = &receipt.evidence;
    format!(
        "Owner-attested local {:?} for {task}, readiness rev {revision}: exit {:?}, stopped {}, timeout {}, complete capture {}, truncated {}; selected saved links unchanged at finish {}. Other inputs/unsaved buffers are not sealed; this is not host-verified test-case acceptance.",
        e.recipe,
        e.exit_code,
        e.stopped,
        e.timed_out,
        e.capture_complete,
        e.truncated,
        receipt.unchanged
    )
}
pub struct State {
    scope: Option<Scope>,
    plan: Option<Plan>,
    run: Option<Run>,
    receipt: Option<Receipt>,
    approved: Option<(String, u64, u64)>,
    recipe: Recipe,
    seconds: u64,
    submitted: bool,
    notice: String,
}
impl Default for State {
    fn default() -> Self {
        Self {
            scope: None,
            plan: None,
            run: None,
            receipt: None,
            approved: None,
            recipe: Recipe::CargoCheck,
            seconds: 60,
            submitted: false,
            notice: String::new(),
        }
    }
}
impl State {
    pub fn reset(&mut self) {
        self.run = None;
        self.scope = None;
        self.plan = None;
        self.receipt = None;
        self.approved = None;
        self.submitted = false;
        self.notice = "Local preflight canceled and its publish grant revoked".into();
    }
    pub fn reconcile(&mut self, root: Option<&Path>, snapshot: &Snapshot) {
        if self
            .scope
            .as_ref()
            .is_some_and(|s| !s.valid(root, snapshot, &s.task))
        {
            self.reset();
        }
        self.poll();
    }
    fn prepare(&mut self, root: &Path, snapshot: &Snapshot, task: &str) -> Result<(), String> {
        let scope = Scope::new(root, snapshot, task)?;
        let files = scope
            .files
            .iter()
            .map(|f| f.path.clone())
            .collect::<Vec<_>>();
        let plan = Plan::resolve(root, self.recipe, &files)?;
        self.scope = Some(scope);
        self.plan = Some(plan);
        self.receipt = None;
        self.approved = None;
        self.submitted = false;
        Ok(())
    }
    fn start(
        &mut self,
        root: &Path,
        snapshot: &Snapshot,
        task: &str,
        context: &egui::Context,
    ) -> Result<(), String> {
        if !(1..=120).contains(&self.seconds) {
            return Err("Preflight time allowance is invalid".into());
        }
        let scope = self
            .scope
            .as_ref()
            .ok_or("Prepare a command review first")?;
        if !scope.valid(Some(root), snapshot, task) || !scope.unchanged() {
            return Err(
                "Scope, revision or selected saved files changed; prepare a fresh review".into(),
            );
        }
        let reviewed = self.plan.as_ref().ok_or("Command review expired")?;
        let files = scope
            .files
            .iter()
            .map(|f| f.path.clone())
            .collect::<Vec<_>>();
        let current = Plan::resolve(root, reviewed.recipe, &files)?;
        if current.display() != reviewed.display() {
            return Err("Executable or command scope changed; prepare a fresh review".into());
        }
        let context = context.clone();
        self.approved = Some((
            current.display(),
            self.seconds,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        ));
        self.run = Some(Run::start(current, self.seconds, move || {
            context.request_repaint()
        }));
        self.plan = None;
        self.receipt = None;
        Ok(())
    }
    fn poll(&mut self) {
        let Some(result) = self.run.as_mut().and_then(Run::poll) else {
            return;
        };
        self.run = None;
        match result {
            Ok(evidence) => {
                let Some((command, limit_seconds, started_epoch_seconds)) = self.approved.take()
                else {
                    self.notice = "Missing executed-command provenance; receipt discarded".into();
                    return;
                };
                let unchanged = self.scope.as_ref().is_some_and(Scope::unchanged);
                self.receipt = Some(Receipt {
                    command,
                    limit_seconds,
                    started_epoch_seconds,
                    evidence,
                    unchanged,
                });
                self.notice="Check finished locally. Inspect its actual status and output before sharing a status-only receipt.".into()
            }
            Err(error) => self.notice = error,
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        root: Option<&Path>,
        snapshot: &Snapshot,
        task: &str,
        actions: &mut Vec<Action>,
    ) {
        if self
            .scope
            .as_ref()
            .is_some_and(|s| !s.valid(root, snapshot, task))
        {
            self.reset();
        }
        self.poll();
        let mut prepare = false;
        let mut start = false;
        let mut cancel = false;
        let mut publish = false;
        egui::CollapsingHeader::new("Approved local preflight and execution receipt").default_open(true).show(ui,|ui|{
            ui.small("Runs trusted saved project code with ordinary local rights, not an OS sandbox. Scripts can write files, spawn programs or use the network. Open unsaved buffers and unselected inputs are not sealed. Switching task/root/trust/owner/readiness revision revokes this run and its publish grant.");
            let owner=snapshot.shared.tasks.iter().any(|t|t.id==task&&t.owner_id==snapshot.you&&t.status!=TaskStatus::Done);
            ui.add_enabled_ui(owner&&root.is_some()&&self.run.is_none(),|ui|{
                egui::ComboBox::from_id_salt("room-preflight-recipe").selected_text(self.recipe.label()).show_ui(ui,|ui|{for recipe in Recipe::ALL{if ui.selectable_value(&mut self.recipe,recipe,recipe.label()).changed(){self.plan=None;}}});
                ui.add(egui::DragValue::new(&mut self.seconds).range(1..=120).suffix(" sec"));
                prepare=ui.button("Prepare exact command for review").clicked();
                if let Some(plan)=&self.plan{ui.monospace(plan.display());start=ui.button("Approve and run this command").clicked();}
            });
            if self.run.is_some(){ui.spinner();ui.label("Approved preflight is running");ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
            cancel=ui.button("Stop / discard local preflight").clicked();
            if let (Some(receipt),Some(scope))=(&self.receipt,&self.scope){
                let text=summary(&scope.task,scope.revision,receipt);ui.label(&text);
                egui::ScrollArea::vertical().id_salt("preflight-output").max_height(180.0).show(ui,|ui|{ui.monospace(&receipt.evidence.stdout);ui.monospace(&receipt.evidence.stderr);});
                ui.small("Output remains local and may contain secrets. Publishing sends the status summary above only; the Room cannot independently verify another machine's execution.");
                publish=ui.add_enabled(!self.submitted&&snapshot.shared.evidence.len()<128,egui::Button::new("Publish status-only receipt as Check evidence")).clicked();
                if ui.button("Copy full receipt locally (includes output)").clicked(){ui.ctx().copy_text(serde_json::json!({"task":scope.task,"readiness_revision":scope.revision,"command_scope":scope.root,"executed_command":receipt.command,"allowance_seconds":receipt.limit_seconds,"started_epoch_seconds":receipt.started_epoch_seconds,"evidence":receipt.evidence,"selected_saved_links_unchanged":receipt.unchanged,"verification":"owner-attested; inputs not fully sealed"}).to_string());}
                if self.submitted{ui.label("Receipt queued. Rejected Room evidence is recoverable in Rejected drafts; task status and manual check declarations are unchanged.");}
            }
            if !self.notice.is_empty(){ui.label(&self.notice);}
        });
        let result = if cancel {
            self.reset();
            Ok(())
        } else if let Some(root) = root {
            if prepare {
                self.prepare(root, snapshot, task)
            } else if start {
                self.start(root, snapshot, task, ui.ctx())
            } else if publish {
                if let (Some(scope), Some(receipt)) = (&self.scope, &self.receipt)
                    && scope.valid(Some(root), snapshot, task)
                {
                    actions.push(Action::Evidence {
                        task: scope.task.clone(),
                        kind: EvidenceKind::Check,
                        text: summary(&scope.task, scope.revision, receipt),
                    });
                    self.submitted = true;
                }
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        };
        if let Err(error) = result {
            self.notice = error;
        }
    }
}
#[cfg(test)]
mod tests;
