//! Native reusable setup, isolated from Room credentials and live authority.
use crate::{background::Latest, local_records};
use eframe::egui;
use forge_room_core::{
    TaskPriority,
    transport::{
        BlueprintMilestone, BlueprintTask, Client, Host, MAX_BLUEPRINT, RoomBlueprint, Snapshot,
    },
    workflows,
};
use std::{
    fs,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
};

pub struct Prepared {
    pub host: Option<Host>,
    pub client: Option<Client>,
    pub snapshot: Snapshot,
}
impl Drop for Prepared {
    fn drop(&mut self) {
        let host = self.host.take();
        let client = self.client.take();
        if host.is_some() {
            std::thread::spawn(move || {
                drop(client);
                drop(host);
            });
        }
    }
}
#[derive(Clone)]
struct Saved {
    path: PathBuf,
    blueprint: RoomBlueprint,
}
struct Library {
    saved: Vec<Saved>,
    issues: Vec<String>,
}
fn directory() -> Result<PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA").ok_or("Local application data unavailable")?;
    Ok(PathBuf::from(root)
        .join("Forge Room Early Preview")
        .join("room-blueprints"))
}
fn load(root: &Path) -> Result<Library, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    local_records::directory(root)?;
    let mut saved = vec![];
    let mut issues = vec![];
    for (index, entry) in fs::read_dir(root).map_err(|e| e.to_string())?.enumerate() {
        if index >= 256 {
            return Err("Library exceeds the 256-entry scan limit; preserve and manage older files externally".into());
        }
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_none_or(|e| e != "frblueprint") {
            continue;
        }
        match local_records::read::<RoomBlueprint>(&path, MAX_BLUEPRINT as u64).and_then(|b| {
            b.validate()?;
            Ok(b)
        }) {
            Ok(blueprint) => saved.push(Saved { path, blueprint }),
            Err(_) => issues.push(format!(
                "Invalid saved blueprint: {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            )),
        }
    }
    saved.sort_by(|a, b| {
        a.blueprint
            .title
            .cmp(&b.blueprint.title)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(Library { saved, issues })
}
fn save(root: &Path, b: &RoomBlueprint) -> Result<Library, String> {
    b.encode()?;
    let current = load(root)?;
    if current.saved.len() + current.issues.len() >= 128 {
        return Err(
            "Saved blueprint limit reached (128); export or manage old versions externally".into(),
        );
    }
    local_records::write_new(
        &root.join(format!("{}.frblueprint", workflows::new_id()?)),
        MAX_BLUEPRINT as u64,
        b,
    )?;
    load(root)
}
fn import(path: &Path) -> Result<RoomBlueprint, String> {
    local_records::file(path)?;
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_BLUEPRINT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    RoomBlueprint::parse(&bytes)
}
enum Reply {
    Library(Result<Library, String>),
    Imported(Result<RoomBlueprint, String>),
    Exported(Result<(), String>),
    Launched(Result<Box<Prepared>, String>),
}
#[derive(Clone)]
struct Review {
    blueprint: RoomBlueprint,
    name: String,
    bind: SocketAddr,
    running: bool,
    seal: String,
}
pub struct State {
    pub open: bool,
    pub ready: Option<Box<Prepared>>,
    draft: RoomBlueprint,
    json: String,
    json_mode: bool,
    name: String,
    bind: String,
    running: bool,
    library: Library,
    loaded: bool,
    query: String,
    worker: Latest<Reply>,
    review: Option<Review>,
    cancelled: bool,
    notice: String,
}
impl Default for State {
    fn default() -> Self {
        let draft = RoomBlueprint::starters().remove(1);
        Self {
            open: false,
            ready: None,
            json: serde_json::to_string_pretty(&draft).unwrap(),
            draft,
            json_mode: false,
            name: crate::room_state::local_user_name(),
            bind: "127.0.0.1:0".into(),
            running: false,
            library: Library {
                saved: vec![],
                issues: vec![],
            },
            loaded: false,
            query: String::new(),
            worker: Latest::default(),
            review: None,
            cancelled: false,
            notice: String::new(),
        }
    }
}
impl State {
    pub fn capture(&mut self, snapshot: &Snapshot) {
        match RoomBlueprint::capture(snapshot) {
            Ok(b) => self.replace(b),
            Err(e) => self.notice = e,
        };
        self.open = true;
    }
    pub fn operation_notice(&mut self, text: String) {
        self.notice = text;
    }
    pub fn replace(&mut self, b: RoomBlueprint) {
        if self.worker.busy() {
            self.notice = "Setup operation in progress; retry staging after it finishes".into();
            return;
        }
        self.json = serde_json::to_string_pretty(&b).unwrap_or_default();
        self.draft = b;
        self.review = None;
        self.notice = "Draft staged; nothing was launched or saved".into();
    }
    fn selected(&self) -> Result<RoomBlueprint, String> {
        if self.json_mode {
            RoomBlueprint::parse(self.json.as_bytes())
        } else {
            self.draft.validate()?;
            Ok(self.draft.clone())
        }
    }
    pub fn tick(&mut self) {
        if let Some(reply) = self.worker.poll() {
            match reply {
                Reply::Library(result) => match result {
                    Ok(l) => {
                        self.library = l;
                        self.notice = "Saved versions refreshed".into();
                    }
                    Err(e) => self.notice = e,
                },
                Reply::Imported(result) => match result {
                    Ok(b) => self.replace(b),
                    Err(e) => self.notice = e,
                },
                Reply::Exported(result) => {
                    self.notice = match result {
                        Ok(()) => "New blueprint exported; existing files were not replaced".into(),
                        Err(e) => e,
                    }
                }
                Reply::Launched(result) => {
                    self.review = None;
                    match result {
                        Ok(prepared) => {
                            if self.cancelled {
                                drop(prepared);
                                self.notice = "Launch cancelled; temporary host closed".into();
                            } else {
                                self.ready = Some(prepared);
                                self.notice = "Fresh Room prepared; adopting connection".into();
                            }
                        }
                        Err(e) => self.notice = e,
                    };
                    self.cancelled = false;
                }
            }
        }
        if self.worker.take_failed() {
            self.notice = "Setup worker stopped without a result; no Room was adopted".into();
            self.review = None;
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, offline: bool) {
        if !self.open {
            return;
        }
        if !self.loaded && !self.worker.busy() {
            self.loaded = true;
            self.worker
                .request(ctx, || Reply::Library(directory().and_then(|p| load(&p))));
        }
        let mut open = self.open;
        egui::Window::new("Room Launch Center").open(&mut open).default_size([760.0,650.0]).resizable(true).show(ctx,|ui|{
            ui.heading("Reusable setup, fresh identity");
            ui.label("Blueprints contain public task and timing intent. Every task and release check starts with the launching member; use reviewed handoffs after teammates join. Messages, files, credentials and previous completion are excluded.");
            ui.label(&self.notice);
            if self.worker.busy(){ui.spinner();if self.review.is_some() && ui.button("Cancel pending launch").clicked(){self.cancelled=true;self.notice="Cancelling adoption; waiting for worker cleanup".into();}}
            egui::ScrollArea::vertical().id_salt("room-launch").show(ui,|ui|{
                ui.add_enabled_ui(!self.worker.busy() && self.review.is_none(),|ui|{
                    ui.horizontal_wrapped(|ui|{for b in RoomBlueprint::starters(){if ui.button(&b.title).clicked(){self.replace(b);}}});
                    ui.collapsing("Saved blueprint versions",|ui|{
                        ui.text_edit_singleline(&mut self.query);
                        let query=self.query.to_lowercase();
                        for entry in self.library.saved.clone().into_iter().filter(|s|s.blueprint.title.to_lowercase().contains(&query)) {if ui.button(format!("{} / {} tasks / {}",entry.blueprint.title,entry.blueprint.tasks.len(),entry.path.file_stem().unwrap_or_default().to_string_lossy())).clicked(){self.replace(entry.blueprint);}}
                        for issue in &self.library.issues {ui.label(issue);}
                        if ui.button("Refresh saved versions").clicked(){self.worker.request(ctx,||Reply::Library(directory().and_then(|p|load(&p))));}
                    });
                    ui.horizontal_wrapped(|ui|{
                        if ui.selectable_label(!self.json_mode,"Structured editor").clicked() && self.json_mode {match self.selected(){Ok(b)=>{self.draft=b;self.json_mode=false;},Err(e)=>self.notice=e}}
                        if ui.selectable_label(self.json_mode,"Manifest JSON").clicked() && !self.json_mode {self.json=serde_json::to_string_pretty(&self.draft).unwrap_or_default();self.json_mode=true;}
                    });
                    if self.json_mode {ui.add(egui::TextEdit::multiline(&mut self.json).code_editor().desired_rows(18).char_limit(MAX_BLUEPRINT));}else{self.edit(ui);}
                    ui.horizontal_wrapped(|ui|{
                        if ui.button("Import blueprint...").clicked() && let Some(path)=rfd::FileDialog::new().add_filter("Room blueprint",&["json"]).pick_file(){self.worker.request(ctx,move||Reply::Imported(import(&path)));}
                        if ui.button("Save new library version").clicked(){match self.selected(){Ok(b)=>{self.worker.request(ctx,move||Reply::Library(directory().and_then(|p|save(&p,&b))));},Err(e)=>self.notice=e}}
                        if ui.button("Export new JSON...").clicked(){match self.selected(){Ok(b)=>if let Some(path)=rfd::FileDialog::new().set_file_name("room-blueprint.json").save_file(){self.worker.request(ctx,move||Reply::Exported(b.encode().and_then(|bytes|local_records::create_file(&path,&bytes,MAX_BLUEPRINT))));},Err(e)=>self.notice=e}}
                    });
                    ui.separator();ui.label("Launching member");ui.add(egui::TextEdit::singleline(&mut self.name).char_limit(64));
                    ui.label("Explicit host IP:port (loopback or a specific reachable LAN IP)");ui.add(egui::TextEdit::singleline(&mut self.bind).char_limit(100));
                    ui.checkbox(&mut self.running,"Start countdown when the new Room is created");
                    ui.label("No discovery, hosted relay or automatic project sharing. Initial recovery journaling is off; enable it explicitly in Network Room after launch. Starting a Room does not verify physical reachability.");
                    if ui.add_enabled(offline,egui::Button::new("Review fresh Room launch")).clicked(){self.review=match self.selected().and_then(|blueprint|{
                        let bind:SocketAddr=self.bind.parse().map_err(|_|"Enter a valid IP:port")?;
                        if bind.ip().is_unspecified() || bind.ip().is_multicast(){return Err("Choose a specific loopback or LAN IP".into());}
                        if self.name.trim().is_empty() || self.name.len()>64 || self.name.chars().any(char::is_control){return Err("Enter a valid display name (64 bytes maximum)".into());}
                        let seal=workflows::seal(&blueprint)?;Ok(Review{blueprint,name:self.name.clone(),bind,running:self.running,seal})
                    }){Ok(r)=>Some(r),Err(e)=>{self.notice=e;None}};}
                    if !offline{ui.label("Leave or end the current Network Room before launching another.");}
                });
                if let Some(review)=self.review.clone(){
                    ui.separator();ui.heading("Frozen launch review");ui.label(format!("{} at {} / owner {} / countdown {}",review.blueprint.title,review.bind,review.name,if review.running{"starts"}else{"paused"}));
                    ui.monospace(format!("Blueprint SHA-256: {}",review.seal));
                    ui.label(format!("{} tasks, {} milestones, {} required tasks, {} manual release checks",review.blueprint.tasks.len(),review.blueprint.milestones.len(),review.blueprint.required_tasks.len(),review.blueprint.release_checks.len()));
                    ui.collapsing("Exact reviewed setup",|ui|{ui.monospace(serde_json::to_string_pretty(&review.blueprint).unwrap_or_default());});
                    if ui.add_enabled(offline && !self.worker.busy(),egui::Button::new("Create reviewed fresh Room")).clicked(){self.cancelled=false;self.worker.request(ctx,move||Reply::Launched(Host::from_blueprint(review.bind,&review.blueprint,&review.name,review.running).map(|(host,client,snapshot)|Box::new(Prepared{host:Some(host),client:Some(client),snapshot}))));}
                    if !self.worker.busy() && ui.button("Back to editing").clicked(){self.review=None;}
                }
            });
        });
        if self.open && !open && self.worker.busy() && self.review.is_some() {
            self.cancelled = true;
        }
        self.open = open;
    }
    fn edit(&mut self, ui: &mut egui::Ui) {
        ui.label("Room title");
        ui.add(egui::TextEdit::singleline(&mut self.draft.title).char_limit(80));
        ui.label("Blueprint description");
        ui.add(
            egui::TextEdit::multiline(&mut self.draft.description)
                .desired_rows(2)
                .char_limit(2000),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Countdown seconds (maximum 604800)");
            ui.add(egui::DragValue::new(&mut self.draft.countdown_seconds).range(0..=604800));
        });
        let mut remove = None;
        for (i, t) in self.draft.tasks.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                egui::CollapsingHeader::new(format!("{} / {}", t.key, t.title)).show(ui, |ui| {
                    ui.label("Stable task key");
                    ui.add(egui::TextEdit::singleline(&mut t.key).char_limit(32));
                    ui.label("Title");
                    ui.add(egui::TextEdit::singleline(&mut t.title).char_limit(200));
                    ui.horizontal_wrapped(|ui| {
                        for p in [
                            TaskPriority::Normal,
                            TaskPriority::High,
                            TaskPriority::Critical,
                        ] {
                            ui.selectable_value(&mut t.priority, p, format!("{p:?}"));
                        }
                    });
                    ui.label("Acceptance criteria (one per line)");
                    let mut criteria = t.criteria.join("\n");
                    if ui
                        .add(
                            egui::TextEdit::multiline(&mut criteria)
                                .desired_rows(2)
                                .char_limit(1800),
                        )
                        .changed()
                    {
                        t.criteria = criteria.lines().map(str::to_string).collect();
                    }
                    ui.label("Dependency keys (comma separated)");
                    let mut dependencies = t.dependencies.join(", ");
                    if ui.text_edit_singleline(&mut dependencies).changed() {
                        t.dependencies = dependencies
                            .split(',')
                            .map(str::trim)
                            .filter(|v| !v.is_empty())
                            .map(str::to_string)
                            .collect();
                    }
                    deadline(ui, &mut t.due_hours);
                    if ui.button("Remove task from draft").clicked() {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove {
            let key = self.draft.tasks.remove(i).key;
            self.draft.required_tasks.retain(|k| k != &key);
            for t in &mut self.draft.tasks {
                t.dependencies.retain(|k| k != &key);
            }
        }
        if ui
            .add_enabled(self.draft.tasks.len() < 100, egui::Button::new("Add task"))
            .clicked()
        {
            let mut i = self.draft.tasks.len() + 1;
            while self
                .draft
                .tasks
                .iter()
                .any(|t| t.key == format!("task_{i}"))
            {
                i += 1;
            }
            self.draft.tasks.push(BlueprintTask {
                key: format!("task_{i}"),
                title: "New task".into(),
                criteria: vec![],
                dependencies: vec![],
                priority: TaskPriority::Normal,
                due_hours: None,
            });
        }
        ui.collapsing("Milestones and release scope", |ui| {
            let mut remove = None;
            for (i, m) in self.draft.milestones.iter_mut().enumerate() {
                ui.push_id(("milestone", i), |ui| {
                    ui.text_edit_singleline(&mut m.title);
                    deadline(ui, &mut m.due_hours);
                    if ui.small_button("Remove milestone").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                self.draft.milestones.remove(i);
            }
            if ui
                .add_enabled(
                    self.draft.milestones.len() < 20,
                    egui::Button::new("Add milestone"),
                )
                .clicked()
            {
                self.draft.milestones.push(BlueprintMilestone {
                    title: "Milestone".into(),
                    due_hours: None,
                });
            }
            for t in &self.draft.tasks {
                let mut required = self.draft.required_tasks.contains(&t.key);
                if ui
                    .checkbox(&mut required, format!("Required: {}", t.title))
                    .changed()
                {
                    self.draft.required_tasks.retain(|k| k != &t.key);
                    if required {
                        self.draft.required_tasks.push(t.key.clone());
                    }
                }
            }
            ui.label("Manual release check titles (one per line, all initially Unknown)");
            let mut checks = self.draft.release_checks.join("\n");
            if ui
                .add(
                    egui::TextEdit::multiline(&mut checks)
                        .desired_rows(3)
                        .char_limit(4000),
                )
                .changed()
            {
                self.draft.release_checks = checks.lines().map(str::to_string).collect();
            }
        });
    }
}
fn deadline(ui: &mut egui::Ui, due: &mut Option<u64>) {
    ui.horizontal_wrapped(|ui| {
        let mut enabled = due.is_some();
        if ui
            .checkbox(&mut enabled, "Deadline hours after launch")
            .changed()
        {
            *due = enabled.then_some(24);
        }
        if let Some(hours) = due {
            ui.add(egui::DragValue::new(hours).range(0..=720));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_versions_are_create_only_and_corruption_does_not_execute_or_hide_valid_versions() {
        let root =
            std::env::temp_dir().join(format!("forge-launch-{}", workflows::new_id().unwrap()));
        let b = RoomBlueprint::starters().remove(1);
        let first = save(&root, &b).unwrap();
        let second = save(&root, &b).unwrap();
        assert_eq!(first.saved.len(), 1);
        assert_eq!(second.saved.len(), 2);
        assert_ne!(second.saved[0].path, second.saved[1].path);
        fs::write(root.join("damaged.frblueprint"), b"FRC1\ntorn").unwrap();
        let loaded = load(&root).unwrap();
        assert_eq!(loaded.saved.len(), 2);
        assert_eq!(loaded.issues.len(), 1);
        let exported = root.join("template.json");
        local_records::create_file(&exported, &b.encode().unwrap(), MAX_BLUEPRINT).unwrap();
        assert_eq!(import(&exported).unwrap(), b);
        assert!(local_records::create_file(&exported, b"replacement", MAX_BLUEPRINT).is_err());
        for file in fs::read_dir(&root).unwrap() {
            fs::remove_file(file.unwrap().path()).unwrap();
        }
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn native_editor_and_frozen_review_frames_render_at_regular_and_narrow_sizes() {
        let ctx = egui::Context::default();
        let mut state = State {
            open: true,
            loaded: true,
            ..Default::default()
        };
        for width in [1200.0, 650.0] {
            for json in [false, true] {
                state.json_mode = json;
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 800.0),
                        )),
                        ..Default::default()
                    },
                    |ui| state.show(ui.ctx(), true),
                );
                let mut output = output;
                output.textures_delta.clear();
            }
        }
        state.review = Some(Review {
            blueprint: state.draft.clone(),
            name: "Owner".into(),
            bind: "127.0.0.1:0".parse().unwrap(),
            running: false,
            seal: "a".repeat(64),
        });
        let mut output = ctx.run_ui(Default::default(), |ui| state.show(ui.ctx(), false));
        output.textures_delta.clear();
    }
}
