//! Native evidence dossiers. Verification is local source/file integrity, reports stay manual.
use crate::{background::Latest, local_records};
use eframe::egui;
use forge_room_core::{
    delivery::{self, Dossier, Outcome, SourceAnchor},
    workflows,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    dossier: Dossier,
    root: Option<PathBuf>,
    files: BTreeMap<String, PathBuf>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    version: u32,
    dossier: Dossier,
    dossier_sha256: String,
    source_and_files_rechecked_at: Option<u64>,
}
impl Report {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > delivery::MAX_DOSSIER {
            return Err("Report exceeds 256 KiB".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|e| {
            format!(
                "Invalid release report schema at line {}, column {}",
                e.line(),
                e.column()
            )
        })?;
        if value.version != 1 || value.dossier.seal()? != value.dossier_sha256 {
            return Err("Release report version/digest mismatch".into());
        }
        Ok(value)
    }
}
#[derive(Clone)]
struct Saved {
    path: PathBuf,
    title: String,
}
fn directory() -> Result<PathBuf, String> {
    Ok(
        PathBuf::from(
            std::env::var_os("LOCALAPPDATA").ok_or("Local application data unavailable")?,
        )
        .join("Forge Room Early Preview")
        .join("release-evidence"),
    )
}
fn library(root: &Path) -> Result<Vec<Saved>, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    local_records::directory(root)?;
    let mut saved = vec![];
    for (i, entry) in fs::read_dir(root).map_err(|e| e.to_string())?.enumerate() {
        if i >= 256 {
            return Err("Release library scan exceeds 256 entries; preserve and manage older versions externally".into());
        }
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_none_or(|e| e != "frelease") {
            continue;
        }
        let title = match local_records::read::<Draft>(&path, 1024 * 1024).and_then(|d| {
            d.dossier.validate()?;
            Ok(d.dossier.title)
        }) {
            Ok(title) => title,
            Err(_) => "Unreadable saved dossier (preserved)".into(),
        };
        saved.push(Saved { path, title });
    }
    saved.sort_by(|a, b| a.title.cmp(&b.title).then_with(|| a.path.cmp(&b.path)));
    Ok(saved)
}
fn save(root: &Path, draft: &Draft) -> Result<Vec<Saved>, String> {
    draft.dossier.validate()?;
    if library(root)?.len() >= 128 {
        return Err("Release evidence library limit reached (128 versions)".into());
    }
    local_records::write_new(
        &root.join(format!("{}.frelease", workflows::new_id()?)),
        1024 * 1024,
        draft,
    )?;
    library(root)
}
fn source(root: &Path) -> Result<SourceAnchor, String> {
    let ids = crate::git::run(root, &["rev-parse", "HEAD", "HEAD^{tree}"])?;
    let status = crate::git::run(
        root,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    )?;
    if status.len() > 2 * 1024 * 1024 {
        return Err("Git status exceeds the evidence capture limit".into());
    }
    if ids != crate::git::run(root, &["rev-parse", "HEAD", "HEAD^{tree}"])?
        || status
            != crate::git::run(
                root,
                &["status", "--porcelain=v1", "--untracked-files=normal"],
            )?
    {
        return Err("Git source changed during capture; retry".into());
    }
    let mut lines = ids.lines();
    let commit = lines.next().ok_or("Commit unavailable")?.into();
    let tree = lines.next().ok_or("Tree unavailable")?.into();
    let value = SourceAnchor {
        commit,
        tree,
        status_digest: workflows::digest(status.as_bytes()),
        dirty_paths: status.lines().count() as u64,
        observed_at: forge_room_core::now_epoch_seconds(),
    };
    let mut check = Dossier::new("Source validation".into())?;
    check.source = Some(value.clone());
    check.validate()?;
    Ok(value)
}
fn recheck(draft: &Draft) -> Result<(), String> {
    draft.dossier.validate()?;
    let expected = draft
        .dossier
        .source
        .as_ref()
        .ok_or("Capture source before checking a handoff")?;
    if expected.dirty_paths > 0 {
        return Err(
            "Checked handoff requires clean committed source; dirty/untracked files are present"
                .into(),
        );
    }
    let current=source(draft.root.as_deref().ok_or("Imported source has no local project binding; explicitly bind and capture the active project")?)?;
    if !expected.same_source(&current) {
        return Err(
            "Source identity/status changed; capture it again and re-report acceptance".into(),
        );
    }
    if draft.dossier.artifacts.is_empty() {
        return Err("Attach at least one artifact before checking a handoff".into());
    }
    for artifact in &draft.dossier.artifacts {
        let path = draft
            .files
            .get(&artifact.id)
            .ok_or_else(|| format!("No local file binding for {}", artifact.name))?;
        delivery::recheck_artifact(path, artifact)?;
    }
    if !expected.same_source(&source(
        draft.root.as_deref().ok_or("Source binding unavailable")?,
    )?) {
        return Err("Source changed while artifact files were being checked; retry after recapturing source".into());
    }
    Ok(())
}
fn export(path: &Path, draft: &Draft, checked: bool) -> Result<(), String> {
    if checked {
        recheck(draft)?;
    }
    let report = Report {
        version: 1,
        dossier: draft.dossier.clone(),
        dossier_sha256: draft.dossier.seal()?,
        source_and_files_rechecked_at: checked.then(forge_room_core::now_epoch_seconds),
    };
    let bytes = serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?;
    local_records::create_file(path, &bytes, delivery::MAX_DOSSIER)
}
fn read_report(path: &Path) -> Result<Report, String> {
    local_records::file(path)?;
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(delivery::MAX_DOSSIER as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Report::parse(&bytes)
}
enum Reply {
    Library(Result<Vec<Saved>, String>),
    Loaded(Result<Box<Draft>, String>),
    Source(Result<(PathBuf, SourceAnchor), String>),
    Artifact(Result<delivery::ArtifactInspection, String>),
    Compared(Result<Vec<String>, String>),
    Done(Result<String, String>),
}
#[derive(Clone)]
struct Review {
    draft: Draft,
    seal: String,
    checked: bool,
}
pub struct State {
    pub open: bool,
    draft: Draft,
    worker: Latest<Reply>,
    expected: Option<String>,
    loaded: bool,
    saved: Vec<Saved>,
    notice: String,
    comparison: Vec<String>,
    review: Option<Review>,
    task: String,
    pub stage: Option<(String, String, String)>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            draft: Draft {
                dossier: Dossier::new("Release evidence".into()).expect("OS random identity"),
                root: None,
                files: Default::default(),
            },
            worker: Default::default(),
            expected: None,
            loaded: false,
            saved: vec![],
            notice: String::new(),
            comparison: vec![],
            review: None,
            task: String::new(),
            stage: None,
        }
    }
}
impl State {
    fn request(&mut self, ctx: &egui::Context, work: impl FnOnce() -> Reply + Send + 'static) {
        if self.worker.busy() {
            return;
        }
        self.expected = Some(workflows::seal(&self.draft.dossier).unwrap_or_default());
        self.worker.request(ctx, work);
    }
    fn tick(&mut self) {
        if let Some(reply) = self.worker.poll() {
            if self.expected.take().is_some_and(|seal| {
                workflows::seal(&self.draft.dossier).unwrap_or_default() != seal
            }) {
                self.notice =
                    "Draft changed before the operation completed; result was not adopted".into();
                return;
            }
            match reply {
                Reply::Library(r) => match r {
                    Ok(v) => {
                        self.saved = v;
                        self.notice = "Saved dossier versions refreshed".into();
                    }
                    Err(e) => self.notice = e,
                },
                Reply::Loaded(r) => match r {
                    Ok(d) => {
                        self.draft = *d;
                        self.review = None;
                        self.comparison.clear();
                        self.notice="Dossier loaded; reports are manual claims and local files/source need rechecking".into();
                    }
                    Err(e) => self.notice = e,
                },
                Reply::Source(r) => {
                    match r {
                        Ok((root, anchor)) => {
                            self.draft.dossier.anchor(anchor);
                            self.draft.root = Some(root);
                            self.notice="Source captured; changed source resets all five manual gate reports".into();
                        }
                        Err(e) => self.notice = e,
                    }
                }
                Reply::Artifact(r) => match r {
                    Ok(i) => match self.draft.dossier.attach_artifact(i.receipt.clone()) {
                        Ok(old) => {
                            if let Some(id) = old {
                                self.draft.files.remove(&id);
                            }
                            self.draft.files.insert(i.receipt.id, i.path);
                            self.notice="Artifact fingerprint observed locally. Changed artifact sets reset manual acceptance reports; package contents and product behavior remain separate checks".into();
                        }
                        Err(e) => self.notice = e,
                    },
                    Err(e) => self.notice = e,
                },
                Reply::Compared(r) => match r {
                    Ok(v) => {
                        self.comparison = v;
                        self.notice="Portable evidence compared; imported claims were not promoted to verified acceptance".into();
                    }
                    Err(e) => self.notice = e,
                },
                Reply::Done(r) => self.notice = r.unwrap_or_else(|e| e),
            }
        }
        if self.worker.take_failed() {
            self.notice =
                "Evidence worker stopped without a result; retry after inspecting the saved files"
                    .into();
            self.expected = None;
        }
    }
    pub fn frame(
        &mut self,
        ctx: &egui::Context,
        active_root: Option<&Path>,
        trusted: bool,
        room: &str,
        tasks: &[forge_room_core::Task],
    ) {
        self.tick();
        if !self.open {
            return;
        }
        if !self.loaded && !self.worker.busy() {
            self.loaded = true;
            self.request(ctx, || {
                Reply::Library(directory().and_then(|p| library(&p)))
            });
        }
        let mut open = self.open;
        egui::Window::new("Release Evidence Desk").open(&mut open).default_size([820.0,700.0]).resizable(true).show(ctx,|ui|{
            ui.heading("Record each acceptance layer separately");ui.label("Human reports remain attributed claims. Git checks establish source identity; file fingerprints establish bytes. Neither proves visible desktop, real-provider inference or a physical two-client trial.");ui.label(&self.notice);
            if self.worker.busy(){ui.spinner();}
            egui::ScrollArea::vertical().id_salt("release-evidence").show(ui,|ui|{
                ui.add_enabled_ui(!self.worker.busy() && self.review.is_none(),|ui|self.editor(ui,active_root,trusted,room,tasks));
                if let Some(review)=self.review.clone(){ui.separator();ui.heading("Frozen export review");ui.monospace(format!("Dossier SHA-256: {}",review.seal));ui.label(review.draft.dossier.brief());
                    ui.label("Portable export includes observer names, notes and references you entered. Review that text before sharing.");
                    ui.collapsing("Exact portable dossier",|ui|{ui.monospace(serde_json::to_string_pretty(&review.draft.dossier).unwrap_or_default());});
                    if let Some(root)=&review.draft.root{ui.label(format!("Source binding: {}",root.display()));}
                    for artifact in &review.draft.dossier.artifacts {ui.label(format!("{} / {}",artifact.name,review.draft.files.get(&artifact.id).map(|p|p.display().to_string()).unwrap_or_else(||"unbound imported metadata".into())));ui.monospace(&artifact.sha256);}ui.label(if review.checked{"Confirm rechecks clean source and every bound artifact on a worker immediately before writing. It does not independently verify manual gate reports."}else{"Draft export contains reported metadata; local source/files are not rechecked."});
                    if ui.add_enabled(!self.worker.busy(),egui::Button::new("Export reviewed new report...")).clicked() && let Some(path)=rfd::FileDialog::new().set_file_name("release-evidence.json").save_file(){self.request(ctx,move||Reply::Done(export(&path,&review.draft,review.checked).map(|_|"New portable report exported; local paths omitted, original files preserved".into())));self.review=None;}
                    if !self.worker.busy() && ui.button("Back to editing").clicked(){self.review=None;}
                }
            });
        });
        self.open = open;
    }
    fn editor(
        &mut self,
        ui: &mut egui::Ui,
        active_root: Option<&Path>,
        trusted: bool,
        room: &str,
        tasks: &[forge_room_core::Task],
    ) {
        let ctx = ui.ctx().clone();
        ui.label("Release title");
        ui.add(egui::TextEdit::singleline(&mut self.draft.dossier.title).char_limit(160));
        ui.horizontal_wrapped(|ui| {
            if ui.button("New dossier").clicked() {
                self.draft = Draft {
                    dossier: Dossier::new("Release evidence".into()).expect("OS random identity"),
                    root: None,
                    files: Default::default(),
                };
                self.comparison.clear();
            }
            if ui.button("Save new private version").clicked() {
                let draft = self.draft.clone();
                self.request(&ctx, move || {
                    Reply::Library(directory().and_then(|p| save(&p, &draft)))
                });
            }
            if ui.button("Import portable report...").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Release evidence", &["json"])
                    .pick_file()
            {
                self.request(&ctx, move || {
                    Reply::Loaded(read_report(&path).map(|r| {
                        Box::new(Draft {
                            dossier: r.dossier,
                            root: None,
                            files: Default::default(),
                        })
                    }))
                });
            }
        });
        ui.collapsing("Saved private versions", |ui| {
            for saved in self.saved.clone() {
                if ui
                    .button(format!(
                        "{} / {}",
                        saved.title,
                        saved.path.file_stem().unwrap_or_default().to_string_lossy()
                    ))
                    .clicked()
                {
                    self.request(&ctx, move || {
                        Reply::Loaded(
                            local_records::read::<Draft>(&saved.path, 1024 * 1024).and_then(|d| {
                                d.dossier.validate()?;
                                Ok(Box::new(d))
                            }),
                        )
                    });
                }
            }
            if ui.button("Refresh saved versions").clicked() {
                self.request(&ctx, || {
                    Reply::Library(directory().and_then(|p| library(&p)))
                });
            }
        });
        ui.separator();
        ui.heading("Source identity");
        if let Some(s) = &self.draft.dossier.source {
            ui.monospace(format!(
                "Commit {}\nTree {}\nDirty/untracked paths: {}",
                s.commit, s.tree, s.dirty_paths
            ));
        }
        if let Some(root) = &self.draft.root {
            ui.label(format!("Private local binding: {}", root.display()));
        } else {
            ui.label("No local project binding");
        }
        if ui
            .add_enabled(
                trusted && active_root.is_some(),
                egui::Button::new("Bind and capture active project source"),
            )
            .clicked()
            && let Some(root) = active_root
        {
            let root = root.to_path_buf();
            self.request(&ctx, move || {
                Reply::Source(source(&root).map(|anchor| (root, anchor)))
            });
        }
        ui.label("Source changes reset all manual acceptance reports. A checked handoff requires clean committed source. Ignored files are outside Git source identity.");
        ui.separator();
        ui.heading("Independent manual acceptance reports");
        for report in &mut self.draft.dossier.gates {
            ui.push_id(format!("{:?}", report.gate), |ui| {
                egui::CollapsingHeader::new(format!(
                    "{} / {:?}",
                    report.gate.label(),
                    report.outcome
                ))
                .default_open(true)
                .show(ui, |ui| {
                    let mut changed = false;
                    ui.horizontal_wrapped(|ui| {
                        for outcome in [
                            Outcome::Unknown,
                            Outcome::ReportedPass,
                            Outcome::ReportedFail,
                        ] {
                            changed |= ui
                                .selectable_value(
                                    &mut report.outcome,
                                    outcome,
                                    format!("{outcome:?}"),
                                )
                                .changed();
                        }
                    });
                    ui.label("Observer");
                    changed |= ui
                        .add(egui::TextEdit::singleline(&mut report.observer).char_limit(80))
                        .changed();
                    ui.label("Observed behavior / limitation");
                    changed |= ui
                        .add(
                            egui::TextEdit::multiline(&mut report.note)
                                .desired_rows(2)
                                .char_limit(4000),
                        )
                        .changed();
                    ui.label("Evidence reference (text only; never fetched or executed)");
                    changed |= ui
                        .add(egui::TextEdit::singleline(&mut report.reference).char_limit(1000))
                        .changed();
                    if changed {
                        report.recorded_at = forge_room_core::now_epoch_seconds();
                    }
                });
            });
        }
        ui.separator();
        ui.heading("Artifact integrity");
        ui.label("Attach artifacts before recording acceptance. Adding/removing receipts or replacing changed bytes resets all five manual reports.");
        ui.label("Up to 64 files, 512 MiB each, 2 GiB total. Hashing does not inspect archive contents or execute installers.");
        let mut remove = None;
        for (i, a) in self.draft.dossier.artifacts.iter().enumerate() {
            ui.push_id(&a.id, |ui| {
                ui.label(format!(
                    "{} / {} bytes / {}",
                    a.name,
                    a.bytes,
                    if self.draft.files.contains_key(&a.id) {
                        "bound locally; recheck required"
                    } else {
                        "imported metadata; no local binding"
                    }
                ));
                ui.monospace(&a.sha256);
                if let Some(path) = self.draft.files.get(&a.id) {
                    ui.label(format!("Private local file: {}", path.display()));
                }
                if ui.small_button("Remove receipt from dossier").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            let a = self
                .draft
                .dossier
                .remove_artifact(i)
                .expect("selected artifact index");
            self.draft.files.remove(&a.id);
        }
        if ui
            .add_enabled(
                self.draft.dossier.artifacts.len() < 64,
                egui::Button::new("Attach / replace artifact fingerprint..."),
            )
            .clicked()
            && let Some(path) = rfd::FileDialog::new().pick_file()
        {
            self.request(&ctx, move || {
                Reply::Artifact(delivery::inspect_artifact(&path))
            });
        }
        ui.label("Release notes");
        ui.add(
            egui::TextEdit::multiline(&mut self.draft.dossier.notes)
                .desired_rows(3)
                .char_limit(4000),
        );
        ui.collapsing("Outstanding gates / limitations", |ui| {
            for issue in self.draft.dossier.blockers() {
                ui.label(issue);
            }
        });
        ui.horizontal_wrapped(|ui|{
            if ui.button("Recheck local source and artifact bytes").clicked(){let draft=self.draft.clone();self.request(&ctx,move||Reply::Done(recheck(&draft).map(|_|"Source and artifact bytes match now; manual acceptance remains separately reported".into())));}
            if ui.button("Compare portable report...").clicked() && let Some(path)=rfd::FileDialog::new().add_filter("Release evidence",&["json"]).pick_file(){let dossier=self.draft.dossier.clone();self.request(&ctx,move||Reply::Compared(read_report(&path).map(|r|dossier.compare(&r.dossier))));}
            for checked in [false,true]{if ui.button(if checked{"Review checked handoff export"}else{"Review draft export"}).clicked(){match self.draft.dossier.seal(){Ok(seal)=>self.review=Some(Review{draft:self.draft.clone(),seal,checked}),Err(e)=>self.notice=e}}}
        });
        for change in &self.comparison {
            ui.label(change);
        }
        ui.collapsing("Stage public status into Room evidence",|ui|{
            ui.label("Only the title, source commit, gate outcomes and artifact count are staged. Notes, observer names, references, local paths and artifact hashes are omitted. Review the existing Room evidence editor before publishing.");
            for task in tasks{ui.selectable_value(&mut self.task,task.id.clone(),format!("{} / {}",task.id,task.title));}
            if ui.add_enabled(!room.is_empty() && tasks.iter().any(|t|t.id==self.task),egui::Button::new("Stage public release status")).clicked(){match self.draft.dossier.validate(){Ok(())=>self.stage=Some((room.into(),self.task.clone(),self.draft.dossier.brief())),Err(e)=>self.notice=e}}
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_handoff_exercises_real_git_and_files_without_promoting_manual_acceptance() {
        let root = std::env::temp_dir().join(format!(
            "forge-release-verified-{}",
            workflows::new_id().unwrap()
        ));
        let repo = root.join("repo");
        fs::create_dir_all(&repo).unwrap();
        let hooks = root.join("empty-hooks");
        fs::create_dir(&hooks).unwrap();
        crate::git::run(&repo, &["-c", "init.templateDir=", "init", "-q"]).unwrap();
        crate::git::run(
            &repo,
            &["config", "core.hooksPath", hooks.to_str().unwrap()],
        )
        .unwrap();
        crate::git::run(&repo, &["config", "user.name", "Fixture"]).unwrap();
        crate::git::run(&repo, &["config", "user.email", "fixture@example.invalid"]).unwrap();
        fs::write(repo.join("source.txt"), b"source").unwrap();
        crate::git::run(&repo, &["add", "source.txt"]).unwrap();
        crate::git::run(
            &repo,
            &["-c", "commit.gpgsign=false", "commit", "-qm", "fixture"],
        )
        .unwrap();
        let package = root.join("package.zip");
        fs::write(&package, b"package").unwrap();
        let observed = delivery::inspect_artifact(&package).unwrap();
        let mut dossier = Dossier::new("Release".into()).unwrap();
        dossier.anchor(source(&repo).unwrap());
        dossier.attach_artifact(observed.receipt.clone()).unwrap();
        let draft = Draft {
            dossier,
            root: Some(repo.clone()),
            files: BTreeMap::from([(observed.receipt.id, observed.path)]),
        };
        let report = root.join("checked.json");
        export(&report, &draft, true).unwrap();
        let parsed = read_report(&report).unwrap();
        assert!(parsed.source_and_files_rechecked_at.is_some());
        assert!(
            parsed
                .dossier
                .gates
                .iter()
                .all(|r| r.outcome == Outcome::Unknown)
        );
        fs::write(&package, b"changed").unwrap();
        assert!(export(&root.join("stale.json"), &draft, true).is_err());
        assert!(!root.join("stale.json").exists());
        fs::write(&package, b"package").unwrap();
        fs::write(repo.join("source.txt"), b"dirty").unwrap();
        assert!(recheck(&draft).is_err());
        let canonical = root.canonicalize().unwrap();
        assert!(canonical.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("forge-release-verified-")
        );
        fs::remove_dir_all(canonical).unwrap();
    }
    #[test]
    fn private_versions_and_portable_reports_preserve_files_and_exclude_local_paths() {
        let root = std::env::temp_dir().join(format!(
            "forge-release-desk-{}",
            workflows::new_id().unwrap()
        ));
        fs::create_dir(&root).unwrap();
        let artifact = root.join("package.zip");
        fs::write(&artifact, b"package").unwrap();
        let inspected = delivery::inspect_artifact(&artifact).unwrap();
        let mut dossier = Dossier::new("Release".into()).unwrap();
        dossier.artifacts.push(inspected.receipt.clone());
        let mut draft = Draft {
            dossier,
            root: Some(root.clone()),
            files: BTreeMap::from([(inspected.receipt.id, inspected.path)]),
        };
        assert_eq!(save(&root.join("library"), &draft).unwrap().len(), 1);
        assert_eq!(save(&root.join("library"), &draft).unwrap().len(), 2);
        let report = root.join("report.json");
        export(&report, &draft, false).unwrap();
        let text = fs::read_to_string(&report).unwrap();
        assert!(!text.contains(&root.to_string_lossy().to_string()));
        let parsed = read_report(&report).unwrap();
        assert_eq!(parsed.dossier, draft.dossier);
        assert!(parsed.source_and_files_rechecked_at.is_none());
        assert!(export(&report, &draft, false).is_err());
        draft.files.clear();
        assert!(recheck(&draft).is_err());
        for file in fs::read_dir(root.join("library")).unwrap() {
            fs::remove_file(file.unwrap().path()).unwrap();
        }
        fs::remove_dir(root.join("library")).unwrap();
        for file in fs::read_dir(&root).unwrap() {
            fs::remove_file(file.unwrap().path()).unwrap();
        }
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn all_acceptance_sections_and_export_review_render_without_claiming_acceptance() {
        let ctx = egui::Context::default();
        let mut state = State {
            open: true,
            loaded: true,
            ..Default::default()
        };
        for width in [1280.0, 650.0] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 900.0),
                    )),
                    ..Default::default()
                },
                |ui| state.frame(ui.ctx(), None, false, "", &[]),
            );
            output.textures_delta.clear();
            assert!(!output.shapes.is_empty());
        }
        state.review = Some(Review {
            draft: state.draft.clone(),
            seal: state.draft.dossier.seal().unwrap(),
            checked: true,
        });
        let mut output = ctx.run_ui(Default::default(), |ui| {
            state.frame(ui.ctx(), None, false, "", &[])
        });
        output.textures_delta.clear();
        assert_eq!(
            state
                .draft
                .dossier
                .gates
                .iter()
                .filter(|r| r.outcome == Outcome::Unknown)
                .count(),
            5
        );
    }
}
