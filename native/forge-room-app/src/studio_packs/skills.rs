use super::{runtime, storage};
use crate::{background, local_records};
use eframe::egui;
use forge_room_core::packs::{Catalog, Kind, Pack, Selection, SkillDocument, Version};
use std::{collections::BTreeMap, io::Read, path::Path};

pub enum Action {
    Inspect(Box<Pack>),
    Activate(BTreeMap<String, Selection>),
    Capture(String, String),
    History,
}
enum Loaded {
    Catalog(String, Result<Catalog, String>),
    Import(Result<(SkillDocument, String), String>),
}
pub struct Panel {
    worker: background::Latest<Loaded>,
    catalog: Catalog,
    seal: String,
    query: String,
    selected: String,
    draft: Option<SkillDocument>,
    id: String,
    version: String,
    publisher: String,
    license: String,
    source: String,
    import_path: String,
    notice: String,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            worker: Default::default(),
            catalog: Default::default(),
            seal: String::new(),
            query: String::new(),
            selected: String::new(),
            draft: None,
            id: "local.skill".into(),
            version: "1.0.0".into(),
            publisher: "Local author claim".into(),
            license: "UNSPECIFIED".into(),
            source: "Local SKILL.md snapshot".into(),
            import_path: String::new(),
            notice: String::new(),
        }
    }
}
fn read(path: &Path) -> Result<SkillDocument, String> {
    local_records::file(path)?;
    let mut bytes = vec![];
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(Kind::Skill.max_bytes() as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    SkillDocument::parse(&bytes)
}
pub fn selection_for(
    store: &storage::Store,
    pack: &Pack,
    resource: &str,
    enabled: bool,
) -> BTreeMap<String, Selection> {
    let mut next = store.ledger.activation.enabled.clone();
    if enabled {
        let selection = next.entry(pack.id.clone()).or_insert_with(|| Selection {
            version: pack.version,
            resources: Default::default(),
        });
        if selection.version != pack.version {
            // A version transition never silently enables unrelated resources.
            selection.version = pack.version;
            selection.resources.clear();
        }
        selection.resources.insert(resource.into());
    } else if let Some(selection) = next.get_mut(&pack.id)
        && selection.version == pack.version
    {
        selection.resources.remove(resource);
        if selection.resources.is_empty() {
            next.remove(&pack.id);
        }
    }
    next
}
impl Panel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        store: Option<&storage::Store>,
        busy: bool,
    ) -> Option<Action> {
        if let Some(result) = self.worker.poll() {
            match result {
                Loaded::Catalog(seal, result) => match result {
                    Ok(catalog) => {
                        self.catalog = catalog;
                        self.seal = seal;
                    }
                    Err(error) => {
                        self.catalog.clear();
                        self.seal = seal;
                        self.notice = error;
                    }
                },
                Loaded::Import(result) => match result {
                    Ok((doc, path)) => {
                        self.draft = Some(doc);
                        self.import_path = path;
                        self.notice = "Exact text captured. Review provenance and installation identity below.".into();
                    }
                    Err(error) => self.notice = error,
                },
            }
        }
        if self.worker.take_failed() {
            self.notice = "Skill worker stopped; no import or activation committed".into();
        }
        let current_seal = store.and_then(|s| s.ledger.seal().ok()).unwrap_or_default();
        if current_seal != self.seal
            && !self.worker.busy()
            && let Some(store) = store
        {
            let root = store.root.clone();
            self.worker.request(ui.ctx(), move || {
                Loaded::Catalog(
                    current_seal,
                    storage::Store::open_at(&root).and_then(|s| s.catalog()),
                )
            });
        }
        let ready = !busy && !self.worker.busy() && store.is_some();
        let mut action = None;
        ui.heading("Skills library");
        ui.label("Install and inspect reusable instructions, then choose the exact version to enable and capture for AI.");
        ui.weak("Skills grant no tools, execution or network access. Companion scripts/references stay outside the import. Publisher, source and license are claims, not verified signatures.");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(ready, egui::Button::new("Import SKILL.md…"))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Skill Markdown", &["md"])
                    .pick_file()
            {
                self.worker.request(ui.ctx(), move || {
                    let shown_path = path.display().to_string();
                    Loaded::Import(read(&path).map(|doc| (doc, shown_path)))
                });
            }
            if ui.button("Version history / rollback").clicked() {
                action = Some(Action::History);
            }
        });
        ui.label(&self.notice);
        if self.worker.busy() {
            ui.spinner();
        }
        if let Some(doc) = &self.draft {
            ui.group(|ui| {
                ui.heading(format!("Import: {}", doc.name));
                ui.label(&doc.description);
                ui.monospace(format!("Observed local file: {}", self.import_path));
                ui.monospace(format!(
                    "Imported text SHA-256: {}",
                    forge_room_core::workflows::digest(doc.content.as_bytes())
                ));
                for (label, value) in [("Stable pack ID", &mut self.id), ("Version (major.minor.patch)", &mut self.version), ("Publisher claim", &mut self.publisher), ("License claim", &mut self.license), ("Source claim (public metadata; omit secrets)", &mut self.source)] {
                    ui.label(label); ui.text_edit_singleline(value);
                }
                ui.weak("For updates, use the same pack ID and a new version. Installed bytes are immutable. Import captures only this file's exact text; it never follows references.");
                ui.collapsing("Inspect complete frozen SKILL.md", |ui| { ui.monospace(&doc.content); });
                for pack in self.catalog.values().filter(|p| p.id == self.id) {
                    let same = pack.resources.iter().any(|r| r.kind == Kind::Skill && r.content == doc.content);
                    ui.label(format!("Installed {}: {}", pack.key(), if same { "identical Skill text" } else { "different Skill text" }));
                }
                if ui.add_enabled(ready, egui::Button::new("Review exact disabled installation…")).clicked() {
                    match self.version.parse::<Version>().and_then(|v| doc.pack(&self.id, v, &self.publisher, &self.license, &self.source)) {
                        Ok(pack) => action = Some(Action::Inspect(Box::new(pack))), Err(error) => self.notice = error,
                    }
                }
            });
            if ui.button("Discard import snapshot").clicked() {
                self.draft = None;
            }
        }
        ui.separator();
        ui.label("Search installed names, descriptions, publishers and versions");
        ui.text_edit_singleline(&mut self.query);
        let query = self.query.to_lowercase();
        let mut count = 0;
        for pack in self.catalog.values() {
            for resource in pack.resources.iter().filter(|r| r.kind == Kind::Skill) {
                if !format!(
                    "{} {} {} {} {}",
                    pack.key(),
                    pack.name,
                    pack.summary,
                    pack.publisher,
                    resource.label
                )
                .to_lowercase()
                .contains(&query)
                {
                    continue;
                }
                count += 1;
                let key = format!("{}/{}", pack.key(), resource.id);
                let active = store.is_some_and(|s| {
                    s.ledger.activation.enabled.get(&pack.id).is_some_and(|s| {
                        s.version == pack.version && s.resources.contains(&resource.id)
                    })
                });
                if ui
                    .selectable_label(
                        self.selected == key,
                        format!(
                            "{} · {} · {}",
                            resource.label,
                            pack.key(),
                            if active { "enabled" } else { "disabled" }
                        ),
                    )
                    .clicked()
                {
                    self.selected = key.clone();
                }
                if self.selected != key {
                    continue;
                }
                ui.push_id(&key, |ui| { ui.group(|ui| {
                    ui.label(&pack.summary);
                    ui.label(format!("Publisher claim: {} / License claim: {}", pack.publisher, pack.license));
                    ui.label(format!("Source claim: {}", pack.source));
                    ui.monospace(format!("Text SHA-256: {}", resource.sha256));
                    ui.collapsing("Complete instructions and declared requirements", |ui| { ui.monospace(&resource.content); });
                    ui.weak("Declared tools, dependencies and links are inert text, not capability grants or verified prerequisites.");
                    if ui.add_enabled(ready, egui::Button::new(if active { "Review disabling this Skill…" } else { "Review enabling this exact version…" })).clicked() {
                        action = Some(Action::Activate(selection_for(store.unwrap(), pack, &resource.id, !active)));
                    }
                    if !active { ui.weak("Changing a pack version disables its other selected resources; inspect the complete activation review before applying."); }
                    let resources = runtime::current();
                    let available = resources.assets.values().any(|a| a.pack == pack.key() && a.resource == resource.id && a.digest == resource.sha256 && matches!(a.value, runtime::Value::Skill(_)));
                    if ui.add_enabled(ready && active && available, egui::Button::new("Capture for explicit AI Include")).clicked() {
                        action = Some(Action::Capture(format!("{} / {} / SHA-256 {}", pack.key(), resource.id, resource.sha256), resource.content.clone()));
                    }
                }); });
            }
        }
        if count == 0 {
            ui.label("No matching installed Skills. Import SKILL.md or install a resource pack containing Skills.");
        }
        ui.weak("Capture creates a historical local snapshot, initially excluded. Include and provider send remain separate explicit actions. Disabling/updating does not remove prior captures or retract sent context; remove old snapshots in AI Context.");
        action
    }
}
