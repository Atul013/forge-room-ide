use eframe::egui;
use forge_room_core::{
    packs::{self, Binding, DataSource, Kind, Pack, Profile, Resource, View, Widget},
    workflows,
};
use std::io::Read;
pub fn body(kind: Kind) -> String {
    match kind{
 Kind::Theme=>serde_json::json!({"name":"Calm Pack","type":"dark","colors":{"editor.background":"#14191d","editor.foreground":"#dce3e9","focusBorder":"#cf8b5f"}}).to_string(),
 Kind::Grammar=>"%YAML 1.2\n---\nname: Forge Notes\nfile_extensions: [fgnote]\nscope: text.forge.notes\ncontexts:\n  main:\n    - match: '\\b(TODO|NOTE)\\b'\n      scope: keyword.other.forge\n".into(),
 Kind::Snippets=>serde_json::json!({"Task note":{"prefix":"note","body":["// Task:","// Evidence:","// Remaining:"]}}).to_string(),
 Kind::Workflow=>serde_json::to_string_pretty(&packs::WorkflowSet{version:1,workflows:vec![workflows::starter_workflows().remove(0)]}).unwrap_or_default(),
 Kind::Skill=>"---\nname: evidence-review\ndescription: Review observed results and remaining acceptance gates\n---\n\nDistinguish process results, manual declarations and model suggestions. Inspect the approved scope before proposing changes. Do not infer desktop or physical-client acceptance from CI.\n".into(),
 Kind::Profile=>serde_json::to_string_pretty(&Profile::default()).unwrap_or_default(),
 Kind::Widget=>serde_json::to_string_pretty(&Widget{
version:1,id:"workspace-summary".into(),title:"Workspace summary".into(),description:"Read-only cached summaries; grant sources for the active workspace".into(),columns:1,refresh_seconds:2,bindings:vec![Binding{
id:"project".into(),label:"Project".into(),source:DataSource::Workspace,view:View::List,limit:4}
,Binding{
id:"runs".into(),label:"Workflow records".into(),source:DataSource::Workflows,view:View::List,limit:4}
]}
).unwrap_or_default(),
 Kind::RoomBlueprint=>serde_json::to_string_pretty(&forge_room_core::transport::RoomBlueprint::starters().remove(1)).unwrap_or_default(),
 Kind::Readme=>"# Local Studio pack\n\nInspect the publisher/license claims and every resource before installing. Appearance and widget data grants are separate choices. Workflow definitions and Skills are staged explicitly; nothing runs or sends automatically.\n".into(),
}
}
pub fn starter() -> Pack {
    Pack {
        format: 1,
        api: 1,
        id: "local.studio".into(),
        version: "1.0.0".parse().unwrap(),
        name: "Local Studio pack".into(),
        publisher: "Local author claim".into(),
        summary: "Editable native resource starter".into(),
        license: "UNSPECIFIED".into(),
        license_text: String::new(),
        source: String::new(),
        readme: body(Kind::Readme),
        dependencies: vec![],
        resources: Kind::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| {
                Resource::new(
                    format!("resource-{}", index + 1),
                    kind.label().into(),
                    kind,
                    body(kind),
                )
                .unwrap()
            })
            .collect(),
    }
}
pub struct Author {
    pub pack: Pack,
    version: String,
    json: String,
    selected: usize,
    new_kind: Kind,
    pub notice: String,
    pub inspect: Option<Pack>,
    pub save_draft: bool,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub pack: Pack,
    pub version: String,
    pub json: String,
}
impl Default for Author {
    fn default() -> Self {
        let pack = starter();
        Self {
            version: pack.version.to_string(),
            json: serde_json::to_string_pretty(&pack).unwrap_or_default(),
            pack,
            selected: 0,
            new_kind: Kind::Readme,
            notice: String::new(),
            inspect: None,
            save_draft: false,
        }
    }
}
impl Author {
    pub fn draft(&self) -> Draft {
        Draft {
            pack: self.pack.clone(),
            version: self.version.clone(),
            json: self.json.clone(),
        }
    }
    pub fn restore(&mut self, draft: Draft) -> Result<(), String> {
        if draft.version.len() > 80
            || draft.json.len() > packs::MAX_PACK
            || serde_json::to_vec(&draft.pack)
                .map_err(|e| e.to_string())?
                .len()
                > packs::MAX_PACK
        {
            return Err("Author draft exceeds its supported bounds".into());
        }
        self.pack = draft.pack;
        self.version = draft.version;
        self.json = draft.json;
        self.selected = 0;
        self.notice = "Private author draft restored, including unfinished edits; validate before installation".into();
        Ok(())
    }
    pub fn set(&mut self, pack: Pack) {
        self.version = pack.version.to_string();
        self.json = serde_json::to_string_pretty(&pack).unwrap_or_default();
        self.pack = pack;
        self.selected = 0;
    }
    fn candidate(&self) -> Result<Pack, String> {
        let mut pack = self.pack.clone();
        pack.version = self.version.parse()?;
        pack.validate()?;
        Ok(pack)
    }
    pub fn show(&mut self, ui: &mut egui::Ui, current: &Profile) {
        ui.heading("Pack workshop");
        ui.weak("Versioned declarative resources only. Publisher/license fields are claims, and checksums establish integrity rather than a verified signer.");
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Identity");
            ui.text_edit_singleline(&mut self.pack.id);
            ui.label("Version");
            ui.text_edit_singleline(&mut self.version);
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.pack.name);
            ui.label("Publisher claim");
            ui.text_edit_singleline(&mut self.pack.publisher);
        });
        ui.label("Summary");
        ui.add(
            egui::TextEdit::multiline(&mut self.pack.summary)
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Declared license");
            ui.text_edit_singleline(&mut self.pack.license);
            ui.label("Source claim (text only)");
            ui.text_edit_singleline(&mut self.pack.source);
        });
        ui.collapsing("License text and pack readme", |ui| {
            ui.label("License text");
            ui.add(
                egui::TextEdit::multiline(&mut self.pack.license_text)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            ui.label("Readme");
            ui.add(
                egui::TextEdit::multiline(&mut self.pack.readme)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
        });
        ui.collapsing("Dependencies", |ui| {
            let mut remove = None;
            for (index, dependency) in self.pack.dependencies.iter_mut().enumerate() {
                ui.push_id(index, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.text_edit_singleline(&mut dependency.id);
                        ui.label(format!(
                            "at least {}{}",
                            dependency.minimum,
                            dependency
                                .before
                                .map(|version| format!(", before {version}"))
                                .unwrap_or_default()
                        ));
                        if ui.button("Remove dependency").clicked() {
                            remove = Some(index);
                        }
                    });
                });
            }
            if let Some(index) = remove {
                self.pack.dependencies.remove(index);
            }
            if ui
                .add_enabled(
                    self.pack.dependencies.len() < 16,
                    egui::Button::new("Add dependency (edit ranges in manifest JSON)"),
                )
                .clicked()
            {
                self.pack.dependencies.push(packs::Dependency {
                    id: "local.dependency".into(),
                    minimum: "1.0.0".parse().unwrap(),
                    before: None,
                });
            }
        });
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("pack-add-kind")
                .selected_text(self.new_kind.label())
                .show_ui(ui, |ui| {
                    for kind in Kind::ALL {
                        ui.selectable_value(&mut self.new_kind, kind, kind.label());
                    }
                });
            if ui
                .add_enabled(
                    self.pack.resources.len() < 32,
                    egui::Button::new("Add resource"),
                )
                .clicked()
            {
                let id = format!(
                    "asset-{}",
                    workflows::new_id()
                        .unwrap_or_else(|_| "random-unavailable".into())
                        .chars()
                        .take(12)
                        .collect::<String>()
                );
                match Resource::new(
                    id,
                    self.new_kind.label().into(),
                    self.new_kind,
                    body(self.new_kind),
                ) {
                    Ok(resource) => {
                        self.pack.resources.push(resource);
                        self.selected = self.pack.resources.len() - 1;
                    }
                    Err(error) => self.notice = error,
                }
            }
            if ui
                .add_enabled(
                    self.pack.resources.len() < 32,
                    egui::Button::new("Capture current appearance as resource"),
                )
                .clicked()
            {
                let content = serde_json::to_string_pretty(current).unwrap_or_default();
                match Resource::new(
                    format!(
                        "profile-{}",
                        workflows::new_id()
                            .unwrap_or_else(|_| "random-unavailable".into())
                            .chars()
                            .take(12)
                            .collect::<String>()
                    ),
                    "Captured appearance".into(),
                    Kind::Profile,
                    content,
                ) {
                    Ok(resource) => {
                        self.pack.resources.push(resource);
                        self.selected = self.pack.resources.len() - 1;
                    }
                    Err(error) => self.notice = error,
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (index, resource) in self.pack.resources.iter().enumerate() {
                ui.selectable_value(&mut self.selected, index, &resource.label);
            }
        });
        let mut remove = false;
        let mut movement = None;
        if let Some(resource) = self.pack.resources.get_mut(self.selected) {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label("Resource key");
                ui.text_edit_singleline(&mut resource.id);
                ui.label("Label");
                ui.text_edit_singleline(&mut resource.label);
            });
            ui.label(format!(
                "{} / {}",
                resource.kind.label(),
                resource.kind.permission()
            ));
            ui.horizontal_wrapped(|ui| {
                if ui.button("Read body from file…").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    let result = (|| {
                        crate::local_records::file(&path)?;
                        let mut bytes = Vec::new();
                        std::fs::File::open(path)
                            .map_err(|e| e.to_string())?
                            .take(resource.kind.max_bytes() as u64 + 1)
                            .read_to_end(&mut bytes)
                            .map_err(|e| e.to_string())?;
                        if bytes.len() > resource.kind.max_bytes() {
                            return Err("Resource file exceeds its type limit".into());
                        }
                        String::from_utf8(bytes).map_err(|e| e.to_string())
                    })();
                    match result {
                        Ok(text) => {
                            resource.sha256 = workflows::digest(text.as_bytes());
                            resource.content = text;
                        }
                        Err(error) => self.notice = error,
                    }
                }
                if ui.button("Remove this resource").clicked() {
                    remove = true;
                }
                if ui
                    .add_enabled(self.selected > 0, egui::Button::new("Move up"))
                    .clicked()
                {
                    movement = Some((self.selected, self.selected - 1));
                }
            });
            if ui
                .add(
                    egui::TextEdit::multiline(&mut resource.content)
                        .code_editor()
                        .desired_rows(12)
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                resource.sha256 = workflows::digest(resource.content.as_bytes());
            }
            ui.monospace(format!(
                "SHA-256 {} · {} bytes",
                resource.sha256,
                resource.content.len()
            ));
        }
        if remove {
            self.pack.resources.remove(self.selected);
            self.selected = self
                .selected
                .min(self.pack.resources.len().saturating_sub(1));
        }
        if let Some((from, to)) = movement {
            self.pack.resources.swap(from, to);
            self.selected = to;
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Inspect complete draft for install").clicked() {
                match self.candidate() {
                    Ok(pack) => self.inspect = Some(pack),
                    Err(error) => self.notice = error,
                }
            }
            if ui.button("Save private author draft").clicked() {
                self.save_draft = true;
            }
            if ui.button("Refresh complete manifest JSON").clicked() {
                let mut pack = self.pack.clone();
                if let Ok(version) = self.version.parse() {
                    pack.version = version;
                }
                self.json = serde_json::to_string_pretty(&pack).unwrap_or_default();
            }
            if ui.button("Export new pack file…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_file_name("studio.forgepack.json")
                    .save_file()
            {
                self.notice = self
                    .candidate()
                    .and_then(|pack| serde_json::to_vec_pretty(&pack).map_err(|e| e.to_string()))
                    .and_then(|bytes| {
                        crate::local_records::create_file(&path, &bytes, packs::MAX_PACK)
                    })
                    .err()
                    .unwrap_or_else(|| "Pack exported to a new file".into());
            }
        });
        ui.collapsing("Complete manifest JSON", |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.json)
                    .code_editor()
                    .desired_rows(14)
                    .desired_width(f32::INFINITY),
            );
            if ui.button("Validate JSON into workshop").clicked() {
                match Pack::parse(self.json.as_bytes()) {
                    Ok(pack) => self.set(pack),
                    Err(error) => self.notice = error,
                }
            }
            if ui
                .button("Recompute resource digests in JSON draft")
                .clicked()
            {
                let result = (|| {
                    if self.json.len() > packs::MAX_PACK {
                        return Err("Draft exceeds 4 MiB".into());
                    }
                    let mut pack: Pack =
                        serde_json::from_str(&self.json).map_err(|e| e.to_string())?;
                    for resource in &mut pack.resources {
                        resource.sha256 = workflows::digest(resource.content.as_bytes());
                    }
                    pack.validate()?;
                    Ok(pack)
                })();
                match result {
                    Ok(pack) => self.set(pack),
                    Err(error) => self.notice = error,
                }
            }
        });
    }
}
