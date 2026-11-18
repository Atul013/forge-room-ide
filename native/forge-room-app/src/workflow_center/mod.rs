//! Native workflow library, frozen reviews, owned execution and private run evidence.
mod editor;
mod executor;
mod library;
mod reports;
mod resolver;
mod storage;
#[cfg(test)]
mod tests;
use eframe::egui;
use forge_room_core::workflows::{self, Workflow};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::mpsc,
};
type ReviewJob = mpsc::Receiver<Result<resolver::Review, String>>;

#[derive(Default)]
pub struct State {
    pub open: bool,
    root: Option<PathBuf>,
    engine: Option<executor::Engine>,
    library: library::Library,
    baseline: Option<String>,
    selected: usize,
    page: usize,
    query: String,
    selected_run: String,
    selected_step: String,
    note: String,
    notice: String,
    review: Option<ReviewJob>,
    json: Option<String>,
    parameters: BTreeMap<String, String>,
    targets: BTreeSet<String>,
    tracked: bool,
    delete_confirm: bool,
    discard_library: bool,
    compare: String,
    room_preview: Option<(String, String, String)>,
    pub stage_ai: Option<String>,
    pub stage_room: Option<(String, String, String)>,
    pub import_room: bool,
    create_name: String,
    create_job: Option<mpsc::Receiver<Result<crate::agent_workspaces::Workspace, String>>>,
    pub created: Option<crate::agent_workspaces::Workspace>,
    drafts: BTreeMap<PathBuf, storage::Draft>,
    draft_seal: Option<String>,
    draft_due: Option<std::time::Instant>,
}

impl State {
    pub fn finish(&mut self) {
        self.preserve_draft(true);
        if let Some(engine) = &mut self.engine {
            engine.stop_all("Application exited; no owned work resumes automatically");
        }
    }

    fn preserve_draft(&mut self, force: bool) {
        if let Some(root) = &self.root {
            let draft = storage::Draft {
                library: self.library.clone(),
                baseline: self.baseline.clone(),
                json: self.json.clone(),
            };
            let seal = workflows::seal(&draft).ok();
            self.drafts.insert(root.clone(), draft.clone());
            if seal != self.draft_seal {
                let due = self.draft_due.get_or_insert_with(|| {
                    std::time::Instant::now() + std::time::Duration::from_secs(2)
                });
                if !force && std::time::Instant::now() < *due {
                    return;
                }
                if let Some(engine) = &self.engine {
                    match engine.store.save_draft(&draft) {
                        Ok(()) => {
                            self.draft_seal = seal;
                            self.draft_due = None;
                        }
                        Err(error) => {
                            self.notice = format!(
                                "Private draft save failed; in-memory draft retained: {error}"
                            )
                        }
                    }
                }
            }
        }
    }

    pub fn tick(
        &mut self,
        ctx: &egui::Context,
        root: Option<&Path>,
        trusted: bool,
        dirty: &[PathBuf],
        records: &[crate::agent_workspaces::Workspace],
        config: Option<&crate::native_models::Config>,
    ) {
        let canonical = root.and_then(|root| resolver::canonical(root).ok());
        if self.root != canonical {
            self.preserve_draft(true);
            if let Some(engine) = &mut self.engine {
                engine.stop_all("Active project changed");
            }
            self.engine = None;
            self.review = None;
            self.room_preview = None;
            self.stage_ai = None;
            self.stage_room = None;
            self.parameters.clear();
            self.targets.clear();
            self.json = None;
            self.selected_run.clear();
            self.root = canonical;
            self.notice.clear();
            self.selected = 0;
            self.baseline = None;
            self.draft_seal = None;
            self.draft_due = None;
            self.library = library::Library::default();
            if let Some(root) = &self.root {
                match library::load(root) {
                    Ok((library, baseline)) => {
                        self.library = library;
                        self.baseline = baseline;
                    }
                    Err(error) => self.notice = format!("Definitions unavailable: {error}"),
                };
                match executor::Engine::open(root) {
                    Ok(engine) => {
                        let draft = self
                            .drafts
                            .get(root)
                            .cloned()
                            .map(Some)
                            .map(Ok)
                            .unwrap_or_else(|| engine.store.load_draft());
                        match draft {
                            Ok(Some(draft)) => {
                                self.draft_seal = workflows::seal(&draft).ok();
                                self.library = draft.library;
                                self.baseline = draft.baseline;
                                self.json = draft.json;
                                self.notice = "Private workflow editor draft restored; project file still requires explicit guarded save.".into();
                            }
                            Ok(None) => (),
                            Err(error) => {
                                self.notice =
                                    format!("Private draft could not be restored: {error}")
                            }
                        };
                        self.engine = Some(engine)
                    }
                    Err(error) => self.notice = format!("History writer unavailable: {error}"),
                };
            }
        }
        let contexts = self.contexts(records, false).unwrap_or_default();
        self.preserve_draft(false);
        if let Some(engine) = &mut self.engine {
            engine.tick(ctx, root, trusted, dirty, &contexts, config);
        }
        if let Some(job) = &self.review {
            match job.try_recv() {
                Ok(result) => {
                    self.review = None;
                    match result {
                        Ok(review) => {
                            if trusted
                                && dirty.is_empty()
                                && self.root.as_ref() == Some(&review.source)
                            {
                                let result = self
                                    .engine
                                    .as_mut()
                                    .ok_or_else(|| "History writer is unavailable".to_owned())
                                    .and_then(|engine| engine.add(review));
                                match result {
                                    Ok(id) => {
                                        self.selected_run = id;
                                        self.page = 1;
                                        self.notice = "Frozen draft saved. Inspect the plan before queue approval.".into();
                                    }
                                    Err(error) => self.notice = error,
                                }
                            } else {
                                self.notice =
                                    "Review discarded because workspace/trust/buffers changed"
                                        .into();
                            }
                        }
                        Err(error) => self.notice = error,
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.review = None;
                    self.notice = "Review worker stopped unexpectedly".into();
                }
                Err(_) => (),
            }
        }
        if let Some(job) = &self.create_job {
            match job.try_recv() {
                Ok(result) => {
                    self.create_job = None;
                    match result {
                        Ok(workspace) => {
                            self.created = Some(workspace);
                            self.notice = "Isolated worktree created; it can be assigned to a step after registration.".into();
                        }
                        Err(error) => self.notice = error,
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.create_job = None;
                    self.notice = "Worktree worker stopped unexpectedly".into();
                }
                Err(_) => (),
            }
        }
        if self.review.is_some() || self.create_job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn contexts(
        &self,
        records: &[crate::agent_workspaces::Workspace],
        validate: bool,
    ) -> Result<BTreeMap<String, resolver::Context>, String> {
        let root = self.root.as_ref().ok_or("Open a project folder")?;
        let mut contexts = BTreeMap::new();
        contexts.insert(
            "primary".into(),
            resolver::Context {
                alias: "primary".into(),
                label: "Current project".into(),
                root: root.clone(),
                source: root.clone(),
                worktree: None,
            },
        );
        for record in records {
            if resolver::canonical(&record.source).ok().as_ref() != Some(root) {
                continue;
            }
            let context_root = if validate {
                record.validate(root)?
            } else {
                let project = if record.project.as_os_str().is_empty() {
                    &record.folder
                } else {
                    &record.project
                };
                match resolver::canonical(project) {
                    Ok(root) => root,
                    Err(_) => continue,
                }
            };
            let alias = format!(
                "checkout-{}",
                &workflows::digest(record.folder.to_string_lossy().as_bytes())[..12]
            );
            contexts.insert(
                alias.clone(),
                resolver::Context {
                    alias,
                    label: format!("{} · {}", record.name, record.branch),
                    root: context_root,
                    source: root.clone(),
                    worktree: Some(record.clone()),
                },
            );
        }
        Ok(contexts)
    }

    pub fn import_tasks(
        &mut self,
        room: &str,
        tasks: &[forge_room_core::Task],
    ) -> Result<(), String> {
        if room.is_empty() {
            return Err("Connect to a current Network Room with a unique receipt scope; older/legacy recovered Rooms cannot link workflow receipts".into());
        }
        let mut workflow = workflows::from_room_tasks("room-plan", "Room task acceptance", tasks)?;
        for step in &mut workflow.steps {
            step.room_id = Some(room.into());
        }
        self.library.append(library::Library {
            version: 1,
            workflows: vec![workflow],
        })?;
        self.selected = self.library.workflows.len() - 1;
        self.parameters.clear();
        self.targets.clear();
        self.open = true;
        self.page = 0;
        Ok(())
    }

    pub fn brief(&self) -> Result<String, String> {
        let engine = self.engine.as_ref().ok_or("No workflow history is open")?;
        let record = engine
            .records
            .iter()
            .find(|record| record.saved.run.id == self.selected_run)
            .ok_or("Select a run in the Workflow center")?;
        Ok(reports::brief(record))
    }

    fn outcome(&mut self, result: Result<(), String>, success: &str) {
        self.notice = result.err().unwrap_or_else(|| success.into());
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        trusted: bool,
        dirty: &[PathBuf],
        records: &[crate::agent_workspaces::Workspace],
    ) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        egui::Window::new("Workflow execution center")
            .id(egui::Id::unique("workflow-center"))
            .open(&mut open)
            .default_size([1060.0, 760.0])
            .min_size([640.0, 420.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Workflows");
                    ui.selectable_value(&mut self.page, 0, "Library and editor");
                    ui.selectable_value(&mut self.page, 1, "Queue and history");
                    ui.selectable_value(&mut self.page, 2, "Contexts");
                    if let Some(engine) = &self.engine {
                        ui.label(format!("{} owned attempts", engine.active_count()));
                    }
                });
                ui.weak("Native checks, manual acceptance and model suggestions retain separate receipts. Restart recovery never starts work automatically.");
                if !self.notice.is_empty() {
                    ui.label(&self.notice);
                }
                if let Some(fault) = self.engine.as_ref().and_then(|engine| engine.fault.as_ref()) {
                    ui.colored_label(crate::theme::danger(), format!("Execution stopped: {fault}"));
                }
                if !trusted {
                    ui.colored_label(crate::theme::warn(), "Trust the active project before executable review or launch.");
                }
                if !dirty.is_empty() {
                    ui.label("Save or close dirty buffers before reviewing/running workflows.");
                }
                ui.separator();
                egui::ScrollArea::both().id_salt("workflow-center-scroll").show(ui, |ui| match self.page {
                    0 => self.library_ui(ui, ctx, trusted, dirty, records),
                    1 => self.history_ui(ui),
                    _ => self.context_ui(ui, ctx, trusted, records),
                });
            });
        self.open = open;
        self.preserve_draft(!open);
        if let Some(engine) = &mut self.engine
            && let Some((run, step, prepared)) = engine.model_review()
        {
            let run = run.to_owned();
            let step = step.to_owned();
            let mut send = false;
            let mut cancel = false;
            egui::Window::new("Review exact workflow AI request").id(egui::Id::unique("workflow-model-send")).default_size([780.0,600.0]).show(ctx,|ui|{
ui.label(format!("Run {run} · step {step}"));
ui.label(format!("{} · {} · up to {} output tokens",prepared.config.base_url,prepared.config.model,prepared.config.max_tokens));
ui.weak("Selected input contents below leave this computer when sent to a remote endpoint. Response is a suggestion; no tools or automatic edits.");
egui::ScrollArea::vertical().max_height(400.0).show(ui,|ui|{
ui.monospace(serde_json::to_string_pretty(&prepared.body).unwrap_or_default());
}
);
ui.horizontal(|ui|{
send=ui.add_enabled(trusted&&dirty.is_empty(),egui::Button::new("Send this exact native request")).clicked();
cancel=ui.button("Cancel this run").clicked();
}
);
}
);
            if send {
                self.notice = engine
                    .send_model(&run, &step, ctx)
                    .err()
                    .unwrap_or_else(|| "Native request sent".into());
            }
            if cancel {
                let result = engine.change(&run, |run| {
                    run.stop("AI send declined", forge_room_core::now_epoch_seconds());
                    Ok(())
                });
                self.notice = result
                    .err()
                    .unwrap_or_else(|| "Run cancellation requested".into());
            }
        }
        if let Some((room, task, text)) = self.room_preview.clone() {
            let mut close = false;
            egui::Window::new("Review Room status receipt")
                .id(egui::Id::unique("workflow-room-stage"))
                .show(ctx, |ui| {
                    ui.label(format!("Room {room} ? task {task}"));
                    ui.label(&text);
                    ui.weak("Staging fills a local evidence draft. Attach in Room is a separate action.");
                    if ui.button("Stage this status-only receipt").clicked() {
                        self.stage_room = Some((room, task, text));
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            if close {
                self.room_preview = None;
            }
        }
    }

    fn library_ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        trusted: bool,
        dirty: &[PathBuf],
        records: &[crate::agent_workspaces::Workspace],
    ) {
        let root = self.root.clone();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.json.is_none(),
                    egui::Button::new("Save project library"),
                )
                .clicked()
            {
                let result = (|| {
                    let root = root.as_ref().ok_or("Open a project")?;
                    let engine = self
                        .engine
                        .as_ref()
                        .ok_or("Private backup storage unavailable")?;
                    self.baseline = Some(library::save(
                        root,
                        &self.library,
                        self.baseline.as_deref(),
                        &engine.store.root,
                    )?);
                    Ok(())
                })();
                self.outcome(
                    result,
                    "Library saved; previous file retained in private backup storage",
                );
            }
            if ui
                .add_enabled(self.json.is_none(), egui::Button::new("Import library…"))
                .clicked()
                && let Some(file) = rfd::FileDialog::new()
                    .add_filter("Workflow JSON", &["json"])
                    .pick_file()
            {
                let result = library::read(&file)
                    .and_then(|bytes| library::Library::parse(&bytes))
                    .and_then(|library| self.library.append(library));
                self.outcome(
                    result,
                    "Imported workflows appended; save the project library to persist",
                );
            }
            if ui
                .add_enabled(
                    self.json.is_none(),
                    egui::Button::new("Import VS Code tasks…"),
                )
                .clicked()
                && let Some(file) = rfd::FileDialog::new()
                    .add_filter("Task JSON/JSONC", &["json", "jsonc"])
                    .pick_file()
            {
                let result = library::read(&file)
                    .and_then(|bytes| library::vscode(&bytes))
                    .and_then(|library| self.library.append(library));
                self.outcome(
                    result,
                    "Task graph appended; unsupported commands are explicit manual gates",
                );
            }
            if ui
                .add_enabled(
                    self.json.is_none(),
                    egui::Button::new("Import Room task graph"),
                )
                .clicked()
            {
                self.import_room = true;
            }
            if ui.button("Export library…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_file_name("workflows.json")
                    .save_file()
            {
                let result = self
                    .library
                    .validate()
                    .and_then(|_| {
                        serde_json::to_vec_pretty(&self.library).map_err(|e| e.to_string())
                    })
                    .and_then(|bytes| library::create_file(&path, &bytes));
                self.outcome(result, "Library exported to a new file");
            }
        });
        ui.horizontal(|ui| {
            ui.label("Find");
            ui.text_edit_singleline(&mut self.query);
            egui::ComboBox::from_id_salt("workflow-select")
                .selected_text(
                    self.library
                        .workflows
                        .get(self.selected)
                        .map(|workflow| workflow.title.as_str())
                        .unwrap_or("No workflow"),
                )
                .show_ui(ui, |ui| {
                    for (index, workflow) in self.library.workflows.iter().enumerate() {
                        if !workflow
                            .title
                            .to_lowercase()
                            .contains(&self.query.to_lowercase())
                        {
                            continue;
                        }
                        if ui
                            .add_enabled(
                                self.json.is_none(),
                                egui::Button::selectable(self.selected == index, &workflow.title),
                            )
                            .clicked()
                        {
                            self.selected = index;
                            self.parameters.clear();
                            self.targets.clear();
                        }
                    }
                });
        });
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("workflow-starters")
                .selected_text("Add starter workflow")
                .show_ui(ui, |ui| {
                    for workflow in workflows::starter_workflows() {
                        if ui
                            .add_enabled(self.json.is_none(), egui::Button::new(&workflow.title))
                            .clicked()
                        {
                            let result = self.library.append(library::Library {
                                version: 1,
                                workflows: vec![workflow],
                            });
                            if result.is_ok() {
                                self.selected = self.library.workflows.len() - 1;
                                self.parameters.clear();
                                self.targets.clear();
                            }
                            self.outcome(result, "Starter appended");
                        }
                    }
                });
            if ui
                .add_enabled(
                    self.json.is_none() && self.selected < self.library.workflows.len(),
                    egui::Button::new("Duplicate workflow"),
                )
                .clicked()
            {
                let workflow = self.library.workflows[self.selected].clone();
                let result = self.library.append(library::Library {
                    version: 1,
                    workflows: vec![workflow],
                });
                self.outcome(result, "Workflow copy appended");
            }
        });
        ui.collapsing("Remove or reload definitions",|ui|{
            ui.checkbox(&mut self.discard_library,"Discard the selected definition or replace this editor draft");
            ui.horizontal(|ui|{
                if ui.add_enabled(self.discard_library&&self.json.is_none()&&self.selected<self.library.workflows.len(),egui::Button::new("Remove selected workflow")).clicked(){self.library.workflows.remove(self.selected);self.selected=self.selected.min(self.library.workflows.len().saturating_sub(1));self.discard_library=false;self.parameters.clear();self.targets.clear();self.notice="Definition removed from this editor. Save the project library to persist; frozen run history remains.".into();}
                if ui.add_enabled(self.discard_library,egui::Button::new("Reload saved project library")).clicked(){let result=root.as_ref().ok_or_else(||"Open a project".to_owned()).and_then(|root|library::load(root));match result{Ok((library,baseline))=>{self.library=library;self.baseline=baseline;self.json=None;self.selected=0;self.parameters.clear();self.targets.clear();self.discard_library=false;self.notice="Project library reloaded explicitly".into();},Err(error)=>self.notice=error}}
            });
        });
        let contexts = match self.contexts(records, true) {
            Ok(contexts) => contexts,
            Err(error) => {
                ui.colored_label(crate::theme::danger(), error);
                BTreeMap::new()
            }
        };
        let Some(workflow) = self.library.workflows.get_mut(self.selected) else {
            return;
        };
        ui.separator();
        if let Some(json) = &mut self.json {
            ui.label("Advanced definition JSON; Apply validates the complete graph. Editor selection and imports stay locked while this draft is open.");
            ui.add(
                egui::TextEdit::multiline(json)
                    .code_editor()
                    .desired_rows(22)
                    .desired_width(f32::INFINITY),
            );
            let mut apply = false;
            let mut discard = false;
            ui.horizontal(|ui| {
                apply = ui.button("Apply JSON draft").clicked();
                discard = ui.button("Discard JSON draft").clicked();
            });
            if apply {
                let result = (|| {
                    if json.len() > workflows::MAX_SPEC_BYTES {
                        return Err("Definition exceeds 1 MiB".into());
                    }
                    let parsed: Workflow = serde_json::from_str(json).map_err(|e| e.to_string())?;
                    parsed.validate()?;
                    Ok(parsed)
                })();
                match result {
                    Ok(parsed) => {
                        *workflow = parsed;
                        self.json = None;
                        self.parameters.clear();
                        self.targets.clear();
                    }
                    Err(error) => self.notice = error,
                }
            } else if discard {
                self.json = None;
            }
            return;
        }
        if ui.button("Edit complete definition JSON").clicked() {
            self.json = Some(serde_json::to_string_pretty(workflow).unwrap_or_default());
            return;
        }
        editor::show(ui, workflow, &contexts.keys().cloned().collect::<Vec<_>>());
        ui.separator();
        ui.heading("Review a new run");
        for parameter in &workflow.parameters {
            let value = self
                .parameters
                .entry(parameter.name.clone())
                .or_insert_with(|| parameter.default.clone().unwrap_or_default());
            ui.horizontal(|ui| {
                ui.label(&parameter.label);
                if parameter.kind == workflows::ParameterKind::Choice {
                    egui::ComboBox::from_id_salt(("workflow-value", &parameter.name))
                        .selected_text(value.as_str())
                        .show_ui(ui, |ui| {
                            for choice in &parameter.choices {
                                ui.selectable_value(value, choice.clone(), choice);
                            }
                        });
                } else {
                    ui.text_edit_singleline(value);
                }
            });
        }
        self.parameters.retain(|name, _| {
            workflow
                .parameters
                .iter()
                .any(|parameter| &parameter.name == name)
        });
        self.targets
            .retain(|id| workflow.steps.iter().any(|step| &step.id == id));
        ui.label("Targets (none selected means the whole graph; prerequisites are included automatically)");
        ui.horizontal_wrapped(|ui| {
            for step in &workflow.steps {
                let mut target = self.targets.contains(&step.id);
                if ui.checkbox(&mut target, &step.id).changed() {
                    if target {
                        self.targets.insert(step.id.clone());
                    } else {
                        self.targets.remove(&step.id);
                    }
                }
            }
        });
        ui.checkbox(
            &mut self.tracked,
            "Seal Git-tracked input files as well as explicit inputs/manifests (512 files per step, 32 MiB total)",
        );
        let expanded = workflow.expand(
            &self.parameters,
            &self.targets.iter().cloned().collect::<Vec<_>>(),
        );
        match &expanded {
            Ok(plan) => {
                ui.label(format!(
                    "{} selected steps including prerequisites · plan {}",
                    plan.included.len(),
                    &plan.seal[..16]
                ));
            }
            Err(error) => {
                ui.colored_label(crate::theme::danger(), error);
            }
        }
        if ui
            .add_enabled(
                trusted
                    && dirty.is_empty()
                    && self.engine.is_some()
                    && self.review.is_none()
                    && expanded.is_ok(),
                egui::Button::new("Resolve tools and freeze review draft"),
            )
            .clicked()
        {
            let expanded = expanded.unwrap();
            let root = root.unwrap();
            let dirty = dirty.to_vec();
            let tracked = self.tracked;
            let (tx, receiver) = mpsc::channel();
            let wake = ctx.clone();
            std::thread::spawn(move || {
                let result = resolver::review(&root, expanded, &contexts, None, &dirty, tracked);
                let _ = tx.send(result);
                wake.request_repaint();
            });
            self.review = Some(receiver);
            self.notice =
                "Resolving exact commands, tool hashes, execution contexts and input seals…".into();
        }
    }

    fn history_ui(&mut self, ui: &mut egui::Ui) {
        let Some(engine) = &mut self.engine else {
            ui.label("Workflow history is unavailable for this project.");
            return;
        };
        if self.selected_run.is_empty()
            && let Some(record) = engine.records.last()
        {
            self.selected_run = record.saved.run.id.clone();
        }
        ui.horizontal_wrapped(|ui| {
            for record in engine.records.iter().rev() {
                if ui
                    .selectable_label(
                        self.selected_run == record.saved.run.id,
                        format!(
                            "{} · {:?} · {}",
                            &record.saved.run.id[..8],
                            record.saved.run.status,
                            record.saved.run.plan.workflow.title
                        ),
                    )
                    .clicked()
                {
                    self.selected_run = record.saved.run.id.clone();
                    self.selected_step.clear();
                    self.note.clear();
                    self.delete_confirm = false;
                }
            }
        });
        let Some(record) = engine
            .records
            .iter()
            .find(|record| record.saved.run.id == self.selected_run)
        else {
            return;
        };
        ui.collapsing("Compare with another run", |ui| {
            egui::ComboBox::from_id_salt("workflow-compare")
                .selected_text(if self.compare.is_empty() {
                    "Choose prior run"
                } else {
                    &self.compare
                })
                .show_ui(ui, |ui| {
                    for other in &engine.records {
                        if other.saved.run.id != record.saved.run.id {
                            ui.selectable_value(
                                &mut self.compare,
                                other.saved.run.id.clone(),
                                format!(
                                    "{} · {}",
                                    &other.saved.run.id[..8],
                                    other.saved.run.plan.workflow.title
                                ),
                            );
                        }
                    }
                });
            if let Some(other) = engine
                .records
                .iter()
                .find(|other| other.saved.run.id == self.compare)
            {
                ui.monospace(reports::comparison(other, record));
            }
        });
        let action = reports::show(
            ui,
            record,
            &engine.store,
            &mut self.selected_step,
            &mut self.note,
            &mut self.delete_confirm,
        );
        let id = record.saved.run.id.clone();
        let seal = record.saved.run.plan.seal.clone();
        let now = forge_room_core::now_epoch_seconds();
        let result = match action {
            None => return,
            Some(reports::Action::Approve) => engine.change(&id, |run| run.approve(&seal, now)),
            Some(reports::Action::Pause) => engine.change(&id, |run| run.pause(now)),
            Some(reports::Action::Resume) => engine.change(&id, |run| run.resume(now)),
            Some(reports::Action::Stop) => engine.change(&id, |run| {
                run.stop("Canceled by user", now);
                Ok(())
            }),
            Some(reports::Action::ApproveAttempt(step)) => {
                engine.change(&id, |run| run.approve_attempt(&step, now))
            }
            Some(reports::Action::Decision(step, passed, note)) => {
                engine.change(&id, |run| run.decide(&step, passed, note, now))
            }
            Some(reports::Action::Skip(step, note)) => {
                engine.change(&id, |run| run.skip_optional(&step, &note, now))
            }
            Some(reports::Action::Retry(step)) => {
                engine.change(&id, |run| run.retry_from(&step, now))
            }
            Some(reports::Action::Delete) => engine.delete(&id),
            Some(reports::Action::StageAi) => {
                self.stage_ai = Some(reports::brief(record));
                Ok(())
            }
            Some(reports::Action::StageRoom(step)) => {
                let task = record
                    .saved
                    .run
                    .plan
                    .workflow
                    .steps
                    .iter()
                    .find(|definition| definition.id == step)
                    .and_then(|definition| definition.room_task.clone());
                match task {
                    Some(task) => reports::receipt(record, &step).map(|text| {
                        self.room_preview = Some((
                            record
                                .saved
                                .run
                                .plan
                                .workflow
                                .steps
                                .iter()
                                .find(|definition| definition.id == step)
                                .and_then(|definition| definition.room_id.clone())
                                .unwrap_or_default(),
                            task,
                            text,
                        ))
                    }),
                    None => Err("No linked Room task".into()),
                }
            }
            Some(reports::Action::Fork) => {
                let workflow = record.saved.run.plan.workflow.clone();
                let result = self.library.append(library::Library {
                    version: 1,
                    workflows: vec![workflow],
                });
                if result.is_ok() {
                    self.selected = self.library.workflows.len() - 1;
                    self.page = 0;
                    self.parameters.clear();
                    self.targets.clear();
                }
                result
            }
        };
        self.outcome(result, "Run action saved");
    }

    fn context_ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        trusted: bool,
        records: &[crate::agent_workspaces::Workspace],
    ) {
        ui.heading("Execution contexts");
        match self.contexts(records, true) {
            Ok(contexts) => {
                for context in contexts.values() {
                    ui.strong(format!("{} · {}", context.alias, context.label));
                    ui.monospace(context.root.display().to_string());
                }
            }
            Err(error) => {
                ui.label(error);
            }
        }
        ui.weak("Contexts come from the active project and registered isolated Git workspaces. Overlapping canonical roots serialize execution; named resources serialize across runs too.");
        ui.horizontal(|ui| {
            ui.label("New isolated branch/worktree name");
            ui.text_edit_singleline(&mut self.create_name);
            if ui
                .add_enabled(
                    trusted && self.root.is_some() && self.create_job.is_none(),
                    egui::Button::new("Create isolated worktree"),
                )
                .clicked()
            {
                let root = self.root.clone().unwrap();
                let name = self.create_name.clone();
                let (tx, receiver) = mpsc::channel();
                let wake = ctx.clone();
                std::thread::spawn(move || {
                    let result = crate::agent_workspaces::create(&root, &name);
                    let _ = tx.send(result);
                    wake.request_repaint();
                });
                self.create_job = Some(receiver);
            }
        });
    }
}
