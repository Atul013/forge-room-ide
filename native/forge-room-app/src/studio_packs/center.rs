use super::appearance_history;
use super::{author, profiles, runtime, storage, widgets};
use crate::{background, local_records, native_models};
use eframe::egui;
use forge_room_core::packs::{self, Pack, Profile, Selection};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

enum Job {
    Loaded(Result<(storage::Store, Result<runtime::Prepared, String>), String>),
    Inspected(Result<(Pack, Vec<String>), String>),
    Prepared(storage::ActivationReview, Result<runtime::Prepared, String>),
}
#[derive(Default)]
pub struct State {
    pub open: bool,
    pub profiles: profiles::Panel,
    pub widgets: widgets::Panel,
    store: Option<storage::Store>,
    started: bool,
    job: background::Latest<Job>,
    page: usize,
    notice: String,
    author: author::Author,
    selected: String,
    selection: BTreeMap<String, Selection>,
    inspected: Option<(Pack, Vec<String>)>,
    activation: Option<(storage::ActivationReview, runtime::Prepared)>,
    confirm: Option<(String, String, String)>,
    query: String,
    pub changed: bool,
    pub stage_workflows: Option<crate::workflow_center::WorkflowLibrary>,
    pub stage_skill: Option<(String, String)>,
    pub stage_blueprint: Option<forge_room_core::transport::RoomBlueprint>,
    resource: String,
    clear_room_links: bool,
    appearance_history: Vec<appearance_history::Saved>,
}
fn load() -> Job {
    Job::Loaded(storage::Store::open().and_then(|mut store| {
        store.reload()?;
        let prepared = store
            .catalog()
            .and_then(|catalog| runtime::prepare(&store.ledger.activation, &catalog));
        Ok((store, prepared))
    }))
}
fn read_pack(path: &Path) -> Result<Pack, String> {
    local_records::file(path)?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(packs::MAX_PACK as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Pack::parse(&bytes)
}
impl State {
    pub fn widgets_visible(&self) -> bool {
        self.open && self.page == 3
    }
    pub fn tick(&mut self, ctx: &egui::Context) {
        if !self.started {
            self.started = true;
            self.clear_room_links = true;
            self.job.request(ctx, load);
        }
        self.profiles.poll(ctx);
        if let Some(result) = self.job.poll() {
            match result {
                Job::Loaded(result) => match result {
                    Ok((store, prepared)) => {
                        self.selection = store.ledger.activation.enabled.clone();
                        let draft_notice = match if self.store.is_none() {
                            read_draft(&store.root)
                        } else {
                            Ok(None)
                        } {
                            Ok(Some(draft)) => self.author.restore(draft).err(),
                            Ok(None) => None,
                            Err(error) => {
                                Some(format!("Author draft preserved but not restored: {error}"))
                            }
                        };
                        self.store = Some(store);
                        match appearance_history::load(&self.store.as_ref().unwrap().root) {
                            Ok(history) => self.appearance_history = history,
                            Err(error) => {
                                self.profiles.notice = format!(
                                    "Kept appearance history preserved but not loaded: {error}"
                                )
                            }
                        };
                        match prepared {
                            Ok(prepared) => {
                                runtime::apply(prepared);
                                self.changed = true;
                                self.notice = "Pack registry loaded; no workflow, Skill or profile was automatically executed".into();
                            }
                            Err(error) => {
                                runtime::apply(runtime::disabled());
                                self.changed = true;
                                self.notice = format!(
                                    "Contributions disabled: {error}. Inspect storage and use explicit repair."
                                )
                            }
                        }
                        if let Some(error) = draft_notice {
                            self.author.notice = error;
                        }
                    }
                    Err(error) => {
                        self.store = None;
                        runtime::apply(runtime::disabled());
                        self.changed = true;
                        self.notice = format!(
                            "Pack storage could not be opened; existing files preserved: {error}"
                        )
                    }
                },
                Job::Inspected(result) => match result {
                    Ok(value) => self.inspected = Some(value),
                    Err(error) => self.notice = error,
                },
                Job::Prepared(review, result) => match result {
                    Ok(prepared) => self.activation = Some((review, prepared)),
                    Err(error) => self.notice = error,
                },
            }
        }
        if self.job.take_failed() {
            self.notice = "Pack worker ended without a result; no activation committed".into();
        }
    }
    fn inspect(&mut self, ctx: &egui::Context, pack: Pack) {
        self.job.request(ctx, move || {
            Job::Inspected(runtime::validate_pack(&pack).map(|notes| (pack, notes)))
        });
    }
    fn prepare(
        &mut self,
        ctx: &egui::Context,
        review: storage::ActivationReview,
    ) -> Result<(), String> {
        let catalog = self
            .store
            .as_ref()
            .ok_or("Open pack registry first")?
            .catalog()?;
        self.job.request(ctx, move || {
            let prepared = runtime::prepare(&review.review.next, &catalog);
            Job::Prepared(review, prepared)
        });
        Ok(())
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        current: &Profile,
        workspace: &str,
        config: Option<&native_models::Config>,
    ) {
        self.profiles.dialogs(ctx, current, workspace);
        self.widgets.windows(ctx);
        let mut open = self.open;
        if open {
            egui::Window::new("Studio and resource packs")
                .id(egui::Id::unique("studio-packs"))
                .open(&mut open)
                .default_size([1000.0, 720.0])
                .show(ctx, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    ui.horizontal_wrapped(|ui| {
                        for (index, title) in [
                            "Installed packs",
                            "Pack workshop",
                            "Appearance / Morph",
                            "Widgets",
                            "History / repair",
                        ]
                        .iter()
                        .enumerate()
                        {
                            ui.selectable_value(&mut self.page, index, *title);
                        }
                    });
                    ui.separator();
                    ui.label(&self.notice);
                    if self.job.busy() {
                        ui.spinner();
                        ui.label("Validating native resources on a worker…");
                    }
                    egui::ScrollArea::vertical().show(ui, |ui| match self.page {
                        0 => self.library(ui),
                        1 => self.author.show(ui, current),
                        2 => {
                            ui.collapsing("Previously kept appearances (stage only)", |ui| {
                                for saved in &self.appearance_history {
                                    if ui
                                        .button(format!("{} / {}", saved.at, saved.profile.name))
                                        .clicked()
                                    {
                                        self.profiles.notice = self
                                            .profiles
                                            .stage(
                                                saved.profile.clone(),
                                                "Private kept appearance history".into(),
                                            )
                                            .err()
                                            .unwrap_or_else(|| {
                                                "Prior appearance staged; explicitly Preview/Keep"
                                                    .into()
                                            });
                                    }
                                }
                            });
                            self.profiles.show(ui, current, config)
                        }
                        3 => self.widgets.show(ui),
                        _ => self.history(ui),
                    });
                });
        }
        self.open = open;
        if let Some(pack) = self.author.inspect.take() {
            self.inspect(ctx, pack);
        }
        if self.author.save_draft {
            self.author.save_draft = false;
            self.author.notice = self.save_draft().err().unwrap_or_else(|| {
                "Private author draft saved; no pack installed or activated".into()
            });
        }
        self.dialogs(ctx);
    }
    fn library(&mut self, ui: &mut egui::Ui) {
        ui.heading("Local resource packs");
        ui.weak("Install text resources, then review the exact resources to enable. No executable extensions or automatic commands.");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.job.busy(), egui::Button::new("Inspect pack file…"))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Forge pack JSON", &["json"])
                    .pick_file()
            {
                self.job.request(ui.ctx(), move || {
                    Job::Inspected(
                        read_pack(&path).and_then(|pack| {
                            runtime::validate_pack(&pack).map(|notes| (pack, notes))
                        }),
                    )
                });
            }
            if ui
                .add_enabled(
                    !self.job.busy() && !self.profiles.active(),
                    egui::Button::new("Reload registry"),
                )
                .clicked()
            {
                self.job.request(ui.ctx(), load);
            }
            ui.text_edit_singleline(&mut self.query);
        });
        let Some(store) = &self.store else {
            ui.label(
                "Registry unavailable. Retry opening it after inspecting the preserved storage.",
            );
            if ui
                .add_enabled(!self.job.busy(), egui::Button::new("Retry opening storage"))
                .clicked()
            {
                self.started = false;
            }
            return;
        };
        ui.weak(format!(
            "{} versions / {} enabled packs / private storage {}",
            store.ledger.versions.len(),
            store.ledger.activation.enabled.len(),
            store.root.display()
        ));
        let keys = store
            .ledger
            .versions
            .keys()
            .filter(|key| key.to_lowercase().contains(&self.query.to_lowercase()))
            .cloned()
            .collect::<Vec<_>>();
        ui.horizontal_wrapped(|ui| {
            for key in keys {
                ui.selectable_value(&mut self.selected, key.clone(), key);
            }
        });
        let Some(store) = &self.store else {
            return;
        };
        let Ok(pack) = store.pack(&self.selected) else {
            if !self.selected.is_empty() {
                ui.label("This version is damaged. Use History / repair to inspect its issue.");
            }
            return;
        };
        ui.heading(&pack.name);
        ui.label(&pack.summary);
        ui.label(format!(
            "{} · publisher claim: {} · license claim: {}",
            pack.key(),
            pack.publisher,
            pack.license
        ));
        ui.monospace(pack.digest().unwrap_or_default());
        ui.collapsing("Readme, license and dependencies", |ui| {
            ui.label(&pack.readme);
            ui.label(&pack.license_text);
            for dependency in &pack.dependencies {
                ui.label(format!(
                    "{} >= {} before {:?}",
                    dependency.id, dependency.minimum, dependency.before
                ));
            }
        });
        let mut enabled = self
            .selection
            .get(&pack.id)
            .is_some_and(|selection| selection.version == pack.version);
        if ui
            .checkbox(&mut enabled, "Select this version for activation review")
            .changed()
        {
            if enabled {
                self.selection.insert(
                    pack.id.clone(),
                    Selection {
                        version: pack.version,
                        resources: pack.resources.iter().map(|r| r.id.clone()).collect(),
                    },
                );
            } else {
                self.selection.remove(&pack.id);
            }
        }
        for resource in &pack.resources {
            ui.push_id(&resource.id, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let mut selected = enabled
                        && self
                            .selection
                            .get(&pack.id)
                            .is_some_and(|s| s.resources.contains(&resource.id));
                    if ui
                        .add_enabled(enabled, egui::Checkbox::new(&mut selected, &resource.label))
                        .changed()
                        && let Some(selection) = self.selection.get_mut(&pack.id)
                    {
                        if selected {
                            selection.resources.insert(resource.id.clone());
                        } else {
                            selection.resources.remove(&resource.id);
                        }
                    }
                    ui.weak(format!(
                        "{} · {} bytes",
                        resource.kind.label(),
                        resource.content.len()
                    ));
                    if ui.button("Inspect body").clicked() {
                        self.resource = resource.content.clone();
                    }
                });
                ui.weak(resource.kind.permission());
            });
        }
        let mut review = None;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.job.busy() && !self.profiles.active(),
                    egui::Button::new("Validate and review activation"),
                )
                .clicked()
            {
                review = Some(store.review(self.selection.clone()));
            }
            if ui.button("Reset selection to active registry").clicked() {
                self.selection = store.ledger.activation.enabled.clone();
            }
            if ui.button("Copy version into workshop").clicked() {
                self.author.set(pack.clone());
                self.page = 1;
            }
            if ui.button("Export installed version…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_file_name(format!("{}.forgepack.json", pack.key()))
                    .save_file()
            {
                self.notice = store
                    .export(&pack.key(), &path)
                    .err()
                    .unwrap_or_else(|| "Pack exported to a new file".into());
            }
        });
        if let Some(review) = review {
            self.notice = review
                .and_then(|review| self.prepare(ui.ctx(), review))
                .err()
                .unwrap_or_else(|| "Preparing exact activation review".into());
        }
        ui.separator();
        ui.heading("Enabled resources");
        let resources = runtime::current();
        for (key, asset) in &resources.assets {
            ui.push_id(key, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(&asset.label);
                    ui.weak(format!(
                        "{} · {} · {}",
                        asset.pack,
                        asset.resource,
                        asset.kind.label()
                    ));
                    match &asset.value {
                        runtime::Value::Theme(actual) => {
                            ui.label(format!("Available in theme picker: {actual}"));
                        }
                        runtime::Value::Grammar(name) => {
                            ui.label(format!("Active syntax: {name}"));
                        }
                        runtime::Value::Snippets(count) => {
                            ui.label(format!("{count} snippets in editor picker"));
                        }
                        runtime::Value::Profile(profile) => {
                            if ui.button("Stage appearance profile").clicked() {
                                self.notice = self
                                    .profiles
                                    .stage(profile.clone(), asset.pack.clone())
                                    .err()
                                    .unwrap_or_else(|| {
                                        "Profile staged; open Appearance to preview".into()
                                    });
                                self.page = 2;
                            }
                        }
                        runtime::Value::Workflow(library) => {
                            ui.checkbox(&mut self.clear_room_links, "Clear imported Room links");
                            if ui
                                .button("Stage definitions in active project editor")
                                .clicked()
                            {
                                let mut library = library.clone();
                                if self.clear_room_links {
                                    for workflow in &mut library.workflows {
                                        for step in &mut workflow.steps {
                                            step.room_task = None;
                                            step.room_id = None;
                                        }
                                    }
                                }
                                self.stage_workflows = Some(library);
                            }
                        }
                        runtime::Value::RoomBlueprint(blueprint) => {
                            ui.label(
                                "Stages setup only. No host starts and no existing Room changes.",
                            );
                            if ui.button("Stage blueprint in Room Launch Center").clicked() {
                                self.stage_blueprint = Some(blueprint.clone());
                            }
                        }
                        runtime::Value::Skill(text) => {
                            if ui.button("Capture Skill for explicit AI Include").clicked() {
                                self.stage_skill = Some((
                                    format!(
                                        "{} / {} / SHA-256 {}",
                                        asset.pack, asset.resource, asset.digest
                                    ),
                                    text.clone(),
                                ));
                            }
                        }
                        runtime::Value::Widget(_) => {
                            if ui.button("Open widget permissions").clicked() {
                                self.page = 3;
                            }
                        }
                        runtime::Value::Readme(text) => {
                            if ui.button("Read text").clicked() {
                                self.resource = text.clone();
                            }
                        }
                    }
                });
            });
        }
        for warning in &resources.warnings {
            ui.label(warning);
        }
    }
    fn history(&mut self, ui: &mut egui::Ui) {
        let Some(store) = &self.store else {
            ui.label("Storage remains closed; no original files modified. Inspect the reported error and restore an intact registry backup before retrying.");
            return;
        };
        ui.heading("Registry history and repair");
        ui.weak("Rollback is a new reviewed activation. Quarantine and removal apply only to inactive versions.");
        ui.label(format!(
            "Private data usage: {:?} bytes / 512 MiB",
            store.usage()
        ));
        for (key, error) in store.issues() {
            ui.label(format!("{key}: {error}"));
        }
        let mut rollback = None;
        for (index, event) in store.ledger.history.iter().enumerate().rev() {
            ui.push_id(index, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("{} / {} / {}", event.at, event.kind, event.detail));
                    if ui
                        .add_enabled(
                            !self.job.busy() && !self.profiles.active(),
                            egui::Button::new("Review prior activation"),
                        )
                        .clicked()
                    {
                        rollback = Some(store.rollback_review(index));
                    }
                });
            });
        }
        ui.separator();
        for (key, version) in &store.ledger.versions {
            ui.push_id(key, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(key);
                    let active = store
                        .ledger
                        .activation
                        .enabled
                        .get(&version.id)
                        .is_some_and(|s| s.version == version.version);
                    if ui
                        .add_enabled(!active, egui::Button::new("Quarantine version…"))
                        .clicked()
                    {
                        self.confirm = Some((
                            "quarantine".into(),
                            key.clone(),
                            store.ledger.seal().unwrap_or_default(),
                        ));
                    }
                    if ui
                        .add_enabled(!active, egui::Button::new("Remove owned version bytes…"))
                        .clicked()
                    {
                        self.confirm = Some((
                            "purge".into(),
                            key.clone(),
                            store.ledger.seal().unwrap_or_default(),
                        ));
                    }
                });
            });
        }
        if ui
            .button("Review disabling all contributions for repair…")
            .clicked()
        {
            self.confirm = Some((
                "disable".into(),
                String::new(),
                store.ledger.seal().unwrap_or_default(),
            ));
        }
        if let Some(review) = rollback {
            self.notice = review
                .and_then(|review| self.prepare(ui.ctx(), review))
                .err()
                .unwrap_or_else(|| "Preparing rollback review".into());
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some((pack, notes)) = &self.inspected {
            let mut install = false;
            let mut repair = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::unique("pack-inspect-install")).show(ctx, |ui| {
                ui.set_max_width(720.0);
                ui.heading("Inspect pack before local installation");
                ui.label(format!("{} / {}", pack.key(), pack.name));
                ui.label(format!("Publisher claim: {} / License claim: {}", pack.publisher, pack.license));
                ui.label(&pack.summary);
                ui.monospace(pack.digest().unwrap_or_default());
                egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                    ui.label(&pack.readme);
                    ui.label(&pack.license_text);
                    for note in notes { ui.label(note); }
                    for resource in &pack.resources {
                        ui.collapsing(format!("{} / {}", resource.id, resource.kind.permission()), |ui| { ui.monospace(&resource.content); });
                    }
                });
                ui.weak("Install stores this exact version disabled. Same-version replacement is rejected.");
                ui.weak("Checksums establish integrity; publisher fields are unverified claims.");
                ui.horizontal_wrapped(|ui| {
                    install = ui.add_enabled(self.store.is_some(), egui::Button::new("Install exact version, disabled")).clicked();
                    let installed = self.store.as_ref().is_some_and(|store| store.ledger.versions.contains_key(&pack.key()));
                    repair = ui.add_enabled(installed, egui::Button::new("Restore exact original damaged bytes")).clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
            if install || repair {
                let result = (|| {
                    let store = self.store.as_mut().ok_or("Open registry first")?;
                    let digest = pack.digest()?;
                    if repair {
                        store.repair_data(pack, &digest)
                    } else {
                        store.install(pack, &digest)
                    }
                })();
                self.notice = result
                    .err()
                    .unwrap_or_else(|| "Exact version stored; review activation separately".into());
                self.inspected = None;
            }
            if cancel {
                self.inspected = None;
            }
        }
        if let Some((review, prepared)) = &self.activation {
            let mut apply = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::unique("pack-activate")).show(ctx, |ui| {
                ui.set_max_width(680.0);
                ui.heading("Apply exactly reviewed resource activation?");
                ui.label(format!("Activation revision {} / {} resources", review.review.next.revision, prepared.resources.assets.len()));
                egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                    ui.strong("Exact next pack versions and manifest digests");
                    for (key,digest) in &review.review.packs {ui.label(key);ui.monospace(digest);}
                    for change in &review.review.changes {
                        let before = if change.before.is_some() { "present" } else { "absent" };
                        let after = if change.after.is_some() { "present" } else { "absent" };
                        ui.label(format!("{}: {before} -> {after}", change.resource));
                        ui.monospace(format!("Resource seals: {} -> {}",change.before.as_deref().unwrap_or("none"),change.after.as_deref().unwrap_or("none")));
                    }
                    for warning in &prepared.resources.warnings { ui.label(warning); }
                });
                ui.weak("Themes, grammar and snippets become available. Other resources require separate explicit choices.");
                ui.weak("Widget grants are revoked. Nothing runs or sends automatically.");
                ui.horizontal_wrapped(|ui| {
                    apply = ui.add_enabled(!self.profiles.active(), egui::Button::new("Apply this activation")).clicked();
                    cancel = ui.button("Discard review").clicked();
                });
            });
            if apply {
                let (review, prepared) = self.activation.take().unwrap();
                let result = self
                    .store
                    .as_mut()
                    .ok_or("Registry closed".into())
                    .and_then(|store| store.activate(&review));
                match result {
                    Ok(()) => {
                        runtime::apply(prepared);
                        self.widgets.resources_changed();
                        self.changed = true;
                        self.notice = "Reviewed activation committed; workflow/Skill/profile actions remain explicit".into();
                    }
                    Err(error) => self.notice = error,
                }
            }
            if cancel {
                self.activation = None;
            }
        }
        if let Some((kind, key, seal)) = &self.confirm {
            let mut apply = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::unique("pack-storage-action")).show(ctx, |ui| {
                ui.heading("Confirm owned pack storage action");
                ui.label(format!("{kind}: {key}"));
                ui.label("The registry revision must still match. Quarantine retains original bytes.");
                ui.label("Removal deletes only the selected owned inactive data file. Disabling preserves version bytes.");
                apply = ui.add_enabled(!self.profiles.active(), egui::Button::new("Apply this storage action")).clicked();
                cancel = ui.button("Cancel").clicked();
            });
            if apply {
                let result = (|| {
                    let store = self.store.as_mut().ok_or("Registry unavailable")?;
                    if store.ledger.seal()? != *seal {
                        return Err("Storage review is stale".into());
                    }
                    match kind.as_str() {
                        "quarantine" => store.quarantine(key),
                        "purge" => store.purge(key),
                        "disable" => {
                            let prepared = runtime::disabled();
                            store.repair_disable(seal)?;
                            runtime::apply(prepared);
                            self.widgets.resources_changed();
                            self.changed = true;
                            self.selection.clear();
                            Ok(None)
                        }
                        _ => Err("Unknown storage action".into()),
                    }
                })();
                self.notice = match result {
                    Ok(warning) => {
                        warning.unwrap_or_else(|| "Owned storage action completed".into())
                    }
                    Err(error) => error,
                };
                self.confirm = None;
            }
            if cancel {
                self.confirm = None;
            }
        }
        if !self.resource.is_empty() {
            let mut open = true;
            egui::Window::new("Inspect declarative resource text")
                .id(egui::Id::unique("pack-resource-text"))
                .open(&mut open)
                .default_size([780.0, 550.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.monospace(&self.resource);
                    });
                });
            if !open {
                self.resource.clear();
            }
        }
    }
    fn save_draft(&self) -> Result<(), String> {
        let store = self
            .store
            .as_ref()
            .ok_or("Open registry before saving a draft")?;
        save_draft(&store.root, &self.author.draft())
    }
    pub fn finish(&mut self) {
        self.profiles.finish();
    }
    pub fn record_appearance(&mut self, profile: &Profile) -> Result<(), String> {
        let root = &self
            .store
            .as_ref()
            .ok_or("Pack storage unavailable; appearance kept in Settings only")?
            .root;
        appearance_history::save(root, profile)?;
        self.appearance_history = appearance_history::load(root)?;
        Ok(())
    }
}
fn draft_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let directory = root.join("drafts");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    local_records::directory(&directory)?;
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        local_records::file(&path)?;
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or("Invalid author draft filename")?;
        if path.extension().is_none_or(|e| e != "draft")
            || name.len() != 20
            || !name.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("Unexpected author draft file; original bytes preserved".into());
        }
        paths.push(path);
        if paths.len() > 32 {
            return Err("Too many author draft records".into());
        }
    }
    paths.sort();
    Ok(paths)
}
fn save_draft(root: &Path, draft: &author::Draft) -> Result<(), String> {
    local_records::directory(root)?;
    let lock_path = root.join(".draft-lock");
    if lock_path.exists() {
        local_records::file(&lock_path)?;
    }
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|e| format!("Another window is saving an author draft: {e}"))?;
    let paths = draft_paths(root)?;
    let sequence = paths
        .last()
        .and_then(|path| path.file_stem()?.to_str()?.parse::<u64>().ok())
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("Author draft sequence exhausted")?;
    local_records::write_new(
        &root.join("drafts").join(format!("{sequence:020}.draft")),
        8 * 1024 * 1024,
        draft,
    )?;
    for path in paths.iter().take(paths.len().saturating_sub(7)) {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn read_draft(root: &Path) -> Result<Option<author::Draft>, String> {
    for path in draft_paths(root)?.iter().rev() {
        match local_records::read::<author::Draft>(path, 8 * 1024 * 1024) {
            Ok(draft) => {
                return Ok(Some(draft));
            }
            Err(error) if error == local_records::INCOMPLETE => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "forge-studio-ui-{}",
                forge_room_core::workflows::new_id().unwrap()
            ));
            std::fs::create_dir(&root).unwrap();
            Self(root.canonicalize().unwrap())
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            assert!(
                self.0
                    .starts_with(std::env::temp_dir().canonicalize().unwrap())
            );
            assert!(
                self.0
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("forge-studio-ui-")
            );
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn private_drafts_restore_unfinished_edits_in_monotonic_order_and_retain_previous_records() {
        let fixture = Fixture::new();
        let author = author::Author::default();
        let mut draft = author.draft();
        draft.pack.id = "unfinished identity".into();
        draft.version = "unfinished version".into();
        draft.json = "{ unfinished JSON".into();
        for index in 0..12 {
            draft.pack.summary = format!("Draft {index}");
            save_draft(&fixture.0, &draft).unwrap();
        }
        assert_eq!(draft_paths(&fixture.0).unwrap().len(), 8);
        let latest = read_draft(&fixture.0).unwrap().unwrap();
        assert_eq!(latest.pack.summary, "Draft 11");
        assert_eq!(latest.version, "unfinished version");
        let mut restored = author::Author::default();
        restored.restore(latest).unwrap();
        assert_eq!(restored.pack.id, "unfinished identity");
        assert!(restored.pack.validate().is_err());
        let path = fixture.0.join("drafts/00000000000000000013.draft");
        std::fs::write(&path, b"FRC1\ninterrupted").unwrap();
        assert_eq!(
            read_draft(&fixture.0).unwrap().unwrap().pack.summary,
            "Draft 11"
        );
        std::fs::write(&path, b"FRC1\n{}\nDONE\n").unwrap();
        assert!(read_draft(&fixture.0).is_err());
    }
    #[test]
    fn every_studio_page_and_review_build_at_regular_and_narrow_sizes_without_desktop_acceptance() {
        let fixture = Fixture::new();
        let mut store = storage::Store::open_at(&fixture.0).unwrap();
        let pack = author::starter();
        store.install(&pack, &pack.digest().unwrap()).unwrap();
        let mut state = State {
            open: true,
            started: true,
            selected: pack.key(),
            store: Some(store),
            ..State::default()
        };
        let context = egui::Context::default();
        for width in [1280.0, 700.0] {
            for page in 0..5 {
                state.page = page;
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 900.0),
                        )),
                        ..Default::default()
                    },
                    |ui| state.show(ui.ctx(), &Profile::default(), "test", None),
                );
                output.textures_delta.clear();
                assert!(!output.shapes.is_empty());
            }
        }
        state.inspected = Some((pack.clone(), runtime::validate_pack(&pack).unwrap()));
        let mut output = context.run_ui(Default::default(), |ui| {
            state.show(ui.ctx(), &Profile::default(), "test", None)
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty());
        assert!(
            state
                .store
                .as_ref()
                .unwrap()
                .ledger
                .activation
                .enabled
                .is_empty()
        );
    }
}
