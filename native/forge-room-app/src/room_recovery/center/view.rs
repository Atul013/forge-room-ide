use super::*;
impl State {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        can_restore: bool,
        bind: &str,
        active_journal: Option<&Path>,
    ) {
        let mut open = self.open;
        if open {
            egui::Window::new("Room Recovery Center").id(egui::Id::unique("room-recovery-center")).open(&mut open).default_size([1040.0,760.0]).show(ctx,|ui|{
            ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Wrap);
            ui.heading("Recover public Room work");
            ui.weak("Inspection starts no host. Restore uses exactly reviewed saved state, paused with fresh TLS and new owner claims.");
            ui.horizontal_wrapped(|ui|{for (page,title)in ["Recent sources","Inspect / compare / restore","Managed vault / history"].iter().enumerate(){ui.selectable_value(&mut self.page,page,*title);}});
            ui.separator();ui.label(&self.notice);if self.busy(){ui.horizontal(|ui|{ui.spinner();ui.label("Working on the reviewed recovery operation…");});}
            ui.horizontal_wrapped(|ui|{
                if ui.add_enabled(!self.busy(),egui::Button::new("Inspect selected recovery file…")).clicked()&& let Some(path)=rfd::FileDialog::new().add_filter("Public checkpoint / journal",&["json","frjournal"]).pick_file(){self.request_inspect(ui.ctx(),path);}
                if ui.add_enabled(!self.busy(),egui::Button::new("Reload private catalog")).clicked(){self.request_reload(ui.ctx());}
                ui.checkbox(&mut self.associate_project,"Associate explicitly inspected files with the current project");
            });
            if self.store.is_none(){ui.weak("The private catalog is unavailable. Selected-file inspection/export and checkpoint recovery remain available; a managed journal requires an intact catalog.");}
            egui::ScrollArea::vertical().id_salt("recovery-center-page").show(ui,|ui|match self.page{0=>self.recents(ui,active_journal),1=>self.inspector(ui,can_restore,bind,active_journal),_=>self.vault(ui,active_journal)});
        });
        }
        self.open = open;
        self.dialogs(ctx, can_restore, active_journal);
    }
    fn recents(&mut self, ui: &mut egui::Ui, active: Option<&Path>) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Find source");
            ui.text_edit_singleline(&mut self.query);
            egui::ComboBox::from_id_salt("recovery-filter")
                .selected_text(
                    ["All sources", "Pinned", "Inspection issue", "Quarantined"][self.filter],
                )
                .show_ui(ui, |ui| {
                    for (index, label) in
                        ["All sources", "Pinned", "Inspection issue", "Quarantined"]
                            .into_iter()
                            .enumerate()
                    {
                        ui.selectable_value(&mut self.filter, index, label);
                    }
                });
        });
        let Some(store) = &self.store else {
            return;
        };
        let quarantine = store.root.join("quarantine");
        let mut entries = store.index.entries.values().cloned().collect::<Vec<_>>();
        entries.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.checked.cmp(&a.checked))
                .then(a.id.cmp(&b.id))
        });
        let mut inspect = None;
        for entry in entries {
            if !format!("{} {}", entry.label, entry.path)
                .to_lowercase()
                .contains(&self.query.to_lowercase())
            {
                continue;
            }
            let quarantined = Path::new(&entry.path).parent() == Some(quarantine.as_path());
            if (self.filter == 1 && !entry.pinned)
                || (self.filter == 2 && entry.issue.is_none())
                || (self.filter == 3 && !quarantined)
            {
                continue;
            }
            ui.push_id(&entry.id,|ui|{egui::Frame::group(ui.style()).show(ui,|ui|{
                ui.horizontal_wrapped(|ui|{if ui.selectable_label(self.selected==entry.id,&entry.label).clicked()&&!self.busy(){inspect=Some(PathBuf::from(&entry.path));}
if entry.pinned{ui.weak("Pinned");}
if active.is_some_and(|path|Store::id(path)==entry.id){ui.strong("Active local journal");}
if quarantined{ui.weak("Private quarantine");}});
                ui.label(&entry.path);ui.weak(format!("Cached inspection checked {}",age(entry.checked)));
                if let Some(summary)=&entry.summary{ui.label(format!("{} · {} tasks · {} saved public messages · {} complete records · {} trailing bytes",summary.kind.label(),summary.tasks,summary.messages,summary.records,summary.tail_bytes));}
                if let Some(issue)=&entry.issue{ui.label(format!("Inspection issue: {issue}"));}
                if let Some(project)=&entry.project{ui.weak(format!("Recorded local project association: {project} (not restored or verified)"));}
                if ui.add_enabled(!self.busy(),egui::Button::new("Inspect current bytes")).clicked(){inspect=Some(PathBuf::from(&entry.path));}
            });});
        }
        if let Some(path) = inspect {
            self.request_inspect(ui.ctx(), path);
        }
        if self
            .store
            .as_ref()
            .is_some_and(|store| store.index.entries.is_empty())
        {
            ui.label("No sources registered. Existing checkpoints/journals can be inspected with the file picker. New successful saves are registered automatically.");
        }
    }
    fn inspector(
        &mut self,
        ui: &mut egui::Ui,
        can_restore: bool,
        bind: &str,
        active: Option<&Path>,
    ) {
        if !self.source_error.is_empty() {
            ui.heading("Recovery source needs attention");
            ui.label(&self.source_error);
            ui.weak("No partial host was started. A malformed complete record is rejected; original bytes remain untouched.");
        }
        if let Some(stamp) = &self.stamp {
            ui.label(stamp.source().display().to_string());
            ui.monospace(format!(
                "{} bytes / SHA-256 {}",
                stamp.bytes(),
                stamp.digest()
            ));
        } else if let Some(path) = &self.source_path {
            ui.label(path.display().to_string());
        }
        let Some(preview) = self.preview.clone() else {
            ui.label("Select and inspect a source before restoring. Damaged owned files can still be managed by their exact fingerprint.");
            self.source_actions(ui, active);
            return;
        };
        let summary = preview.summary();
        ui.heading(&summary.room);
        ui.weak(format!(
            "Stored capture clock: {} / {}",
            summary.captured,
            age(summary.captured)
        ));
        ui.label(format!(
            "{} / declared lifecycle {:?} / message cursor {}",
            summary.kind.label(),
            summary.lifecycle,
            summary.cursor
        ));
        egui::Grid::new("recovery-summary-counts")
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                for (label, value) in [
                    ("Tasks", summary.tasks),
                    ("Done declarations", summary.done),
                    ("Offline saved members", summary.members),
                    ("Retained public messages", summary.messages),
                    ("Evidence entries", summary.evidence),
                    ("Milestones", summary.milestones),
                    ("Readiness profiles", summary.readiness),
                    ("Impacts without acknowledgement", summary.pending_impacts),
                ] {
                    ui.label(label);
                    ui.strong(value.to_string());
                    if matches!(
                        label,
                        "Done declarations"
                            | "Retained public messages"
                            | "Milestones"
                            | "Impacts without acknowledgement"
                    ) {
                        ui.end_row();
                    }
                }
            });
        ui.label(format!(
            "{} complete records / {} complete bytes / {} trailing incomplete bytes",
            summary.records, summary.complete_bytes, summary.tail_bytes
        ));
        ui.weak(format!("Saved countdown: {} seconds; recovery pauses it. Ship revision {} is a saved declaration, not fresh verification.",summary.countdown_seconds,summary.ship_revision));
        ui.collapsing("Saved tasks and owners", |ui| {
            for task in preview.tasks() {
                ui.collapsing(
                    format!("{} / {:?} / {}", task.id, task.status, task.title),
                    |ui| {
                        ui.label(format!(
                            "Saved owner: {} ({})",
                            task.owner_name, task.owner_id
                        ));
                        for criterion in &task.acceptance_criteria {
                            ui.label(criterion);
                        }
                        if !task.dependencies.is_empty() {
                            ui.label(format!("Dependencies: {}", task.dependencies.join(", ")));
                        }
                    },
                );
            }
            for member in preview.members() {
                ui.label(format!(
                    "Offline saved member: {} / {}",
                    member.name, member.id
                ));
            }
        });
        ui.weak("Recovery restores public coordination and retained public messages. Private histories, patches, credentials, reactions, live focus, source files, terminals and AI state are excluded. Saved manual reports remain declarations.");
        if let Some(project) = &self.project {
            ui.label(format!("Current local project stays unchanged: {project}"));
        } else {
            ui.label(
                "No project is currently open; recovery does not select or verify a workspace.",
            );
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.busy() && can_restore,
                    egui::Button::new("Review paused restore…"),
                )
                .clicked()
            {
                self.restore = true;
                self.restore_bind = bind.into();
                self.fresh = summary.lifecycle != RoomLifecycle::Archived;
                self.acknowledge_tail = summary.tail_bytes == 0;
            }
            if ui
                .add_enabled(
                    !self.busy(),
                    egui::Button::new("Export sanitized checkpoint…"),
                )
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_file_name("public-room-checkpoint.json")
                    .save_file()
            {
                let root=self.catalog_root.clone();
                let preview = preview.clone();
                self.job.request(ui.ctx(), move || {
                    Job::Exported(external_destination_at(root.as_deref(),&path).and_then(|_|preview.export_checkpoint(&path)).map(|_| {
                        "Sanitized checkpoint saved to a new file; original source unchanged".into()
                    }))
                });
            }
            if ui
                .add_enabled(!self.busy(), egui::Button::new("Inspect source again"))
                .clicked()
            {
                self.request_inspect(ui.ctx(), preview.source().into());
            }
        });
        if ui
            .add_enabled(
                !self.busy() && self.store.is_some(),
                egui::Button::new("Save sanitized copy in managed vault"),
            )
            .clicked()
        {
            self.managed_copy(ui.ctx(), preview.clone());
        }
        if !can_restore {
            ui.label("Leave or end the current Network Room before restoring another host. Inspection and export remain available.");
        }
        self.source_actions(ui, active);
        ui.separator();
        ui.heading("Compare inspected public snapshots");
        if let Some(store) = &self.store {
            egui::ComboBox::from_id_salt("recovery-comparison-source")
                .selected_text(if self.compare_path.is_empty() {
                    "Choose another registered source"
                } else {
                    &self.compare_path
                })
                .show_ui(ui, |ui| {
                    for entry in store.index.entries.values() {
                        if Path::new(&entry.path) != preview.source() {
                            ui.selectable_value(
                                &mut self.compare_path,
                                entry.path.clone(),
                                &entry.label,
                            );
                        }
                    }
                });
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.busy(), egui::Button::new("Choose comparison file…"))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Public checkpoint / journal", &["json", "frjournal"])
                    .pick_file()
            {
                self.compare_path = path.to_string_lossy().into_owned();
            }
            if ui
                .add_enabled(
                    !self.busy() && !self.compare_path.is_empty(),
                    egui::Button::new("Compare saved snapshots"),
                )
                .clicked()
            {
                let before = preview.clone();
                let path = PathBuf::from(&self.compare_path);
                self.job.request(ui.ctx(), move || {
                    Job::Compared((|| {
                        let after = inspect_recovery(&path)?;
                        let comparison = before.compare(&after)?;
                        Ok(Box::new((after, comparison)))
                    })())
                });
            }
        });
        if let Some(value) = &self.comparison {
            let (after, comparison) = value.as_ref();
            ui.label(format!(
                "Comparison source: {} / {}",
                after.summary().room,
                after.source().display()
            ));
            if !comparison.same_room {
                ui.strong("Different Room identities. Task histories are not merged or treated as continuous.");
            } else {
                ui.label(format!(
                    "Declared lifecycle: {:?} -> {:?}",
                    comparison.before_lifecycle, comparison.after_lifecycle
                ));
                for change in &comparison.coordination {
                    ui.label(format!("Different saved snapshot: {change}"));
                }
                ui.label(format!("Same Room identity / identical public state: {} / public message cursor {} -> {}",comparison.same_public_state,comparison.before_cursor,comparison.after_cursor));
                ui.weak("The message cursor alone does not order every task change. These are frozen inspected snapshots.");
                for change in &comparison.tasks {
                    ui.label(format!(
                        "{} / {} / {}",
                        change.id, change.change, change.title
                    ));
                }
            }
        }
        ui.separator();
        ui.checkbox(
            &mut self.include_path,
            "Include local file path in exported inspection report",
        );
        if ui
            .add_enabled(!self.busy(), egui::Button::new("Export inspection report…"))
            .clicked()
            && let Some(path) = rfd::FileDialog::new()
                .set_file_name("room-recovery-inspection.json")
                .save_file()
        {
            let root = self.catalog_root.clone();
            let value = serde_json::json!({"format":1,"summary":summary,"local_path":self.include_path.then(||preview.source().to_string_lossy().into_owned()),"comparison":self.comparison.as_ref().map(|value|&value.1),"restores_private_history":false,"restores_old_credentials":false,"manual_results_independently_verified":false});
            self.job.request(ui.ctx(), move || {
                Job::Exported(
                    external_destination_at(root.as_deref(), &path)
                        .and_then(|_| serde_json::to_vec_pretty(&value).map_err(|e| e.to_string()))
                        .and_then(|bytes| {
                            crate::local_records::create_file(&path, &bytes, 1024 * 1024)
                        })
                        .map(|_| "Inspection report exported to a new file".into()),
                )
            });
        }
    }
    fn source_actions(&mut self, ui: &mut egui::Ui, active: Option<&Path>) {
        let Some(store) = &self.store else {
            return;
        };
        let Some(entry) = store.index.entries.get(&self.selected).cloned() else {
            return;
        };
        let seal = store.index.seal().unwrap_or_default();
        let managed = store.managed_candidate(Path::new(&entry.path));
        let current = active.is_some_and(|path| Store::id(path) == entry.id);
        let matching = self
            .stamp
            .as_ref()
            .filter(|stamp| Store::id(stamp.source()) == entry.id);
        let digest = matching
            .map(|stamp| stamp.digest().to_string())
            .unwrap_or_default();
        let quarantined =
            Path::new(&entry.path).parent() == Some(store.root.join("quarantine").as_path());
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.busy(),
                    egui::Button::new(if entry.pinned {
                        "Unpin source"
                    } else {
                        "Pin source"
                    }),
                )
                .clicked()
            {
                action = Some((
                    if entry.pinned { "unpin" } else { "pin" }.to_string(),
                    entry.id.clone(),
                    seal.clone(),
                    String::new(),
                ));
            }
            if ui
                .add_enabled(!self.busy(), egui::Button::new("Forget catalog entry…"))
                .clicked()
            {
                self.confirm = Some((
                    "forget".into(),
                    entry.id.clone(),
                    seal.clone(),
                    String::new(),
                ));
                self.confirm_delete = false;
            }
            if ui
                .add_enabled(
                    !self.busy() && managed && !quarantined && !current && !digest.is_empty(),
                    egui::Button::new("Quarantine inactive managed file…"),
                )
                .clicked()
            {
                self.confirm = Some((
                    "quarantine".into(),
                    entry.id.clone(),
                    seal.clone(),
                    digest.clone(),
                ));
                self.confirm_delete = false;
            }
            if ui
                .add_enabled(
                    !self.busy() && managed && quarantined && !current && !digest.is_empty(),
                    egui::Button::new("Remove owned quarantine file…"),
                )
                .clicked()
            {
                self.confirm = Some((
                    "purge".into(),
                    entry.id.clone(),
                    seal.clone(),
                    digest.clone(),
                ));
                self.confirm_delete = false;
            }
        });
        if !managed {
            ui.weak("External source: the center can forget its reference, but cannot move or delete the original file.");
        } else if current {
            ui.weak("Active local journal is protected. Other native windows are also checked through a held file lease.");
        } else if digest.is_empty() {
            ui.weak("Inspect the current file fingerprint before moving or deleting owned recovery data.");
        }
        if let Some((kind, id, seal, digest)) = action {
            self.update(ui.ctx(), kind, id, seal, digest);
        }
    }
    fn vault(&mut self, ui: &mut egui::Ui, active: Option<&Path>) {
        let Some(store) = &self.store else {
            return;
        };
        ui.heading("Managed recovery vault");
        ui.label(store.root.display().to_string());
        ui.weak("Managed journals reserve their full 64 MiB allowance; checkpoints reserve 8 MiB. Pending creations also count. The vault is capped at 512 MiB reserved capacity; quarantine remains charged until explicitly removed.");
        ui.label(format!(
            "Cached index: {} entries / revision {}",
            store.index.entries.len(),
            store.index.revision
        ));
        if ui
            .add_enabled(!self.busy(), egui::Button::new("Measure vault usage"))
            .clicked()
        {
            let root = self.catalog_root.clone();
            self.job.request(ui.ctx(),move||Job::Exported(catalog_at(root.as_deref()).and_then(|store|store.usage()).map(|(bytes,reserved,files)|format!("{files} managed files / {bytes} actual bytes / {reserved} reserved bytes"))));
        }
        if ui
            .add_enabled(
                !self.busy(),
                egui::Button::new("Discover unindexed owned vault/quarantine files"),
            )
            .clicked()
        {
            let root = self.catalog_root.clone();
            self.job.request(ui.ctx(), move || {
                Job::Discovered(catalog_at(root.as_deref()).and_then(|store| store.discover()))
            });
        }
        let mut inspect = None;
        for path in &self.discovered {
            ui.horizontal_wrapped(|ui| {
                ui.label(path.display().to_string());
                if ui
                    .add_enabled(!self.busy(), egui::Button::new("Inspect / register"))
                    .clicked()
                {
                    inspect = Some(path.clone());
                }
            });
        }
        if let Some(path) = inspect {
            self.request_inspect(ui.ctx(), path);
        }
        self.source_actions(ui, active);
        ui.separator();
        ui.heading("Private management history");
        if let Some(store) = &self.store {
            for event in store.index.history.iter().rev() {
                ui.label(format!(
                    "{} / {} / {}",
                    age(event.at),
                    event.kind,
                    event.detail
                ));
            }
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context, can_restore: bool, active: Option<&Path>) {
        if self.restore
            && let Some(preview) = self.preview.clone()
        {
            let mut restore = false;
            let mut cancel = false;
            let summary = preview.summary();
            egui::Modal::new(egui::Id::unique("room-recovery-restore-review")).show(ctx,|ui|{
                ui.set_max_width(700.0);ui.heading("Restore exactly reviewed public Room state?");ui.label(format!("{} / {} / {} tasks / {} complete records",summary.room,summary.kind.label(),summary.tasks,summary.records));ui.label(preview.source().display().to_string());ui.monospace(&summary.digest);
                ui.label("The original file is rechecked and preserved. The restored host is paused; archived Rooms stay archived. Old TLS, invitations and sessions do not return. Original owners need fresh host-approved identity claims.");
                ui.label("Stop the original host before restoring. File checks cannot prove that another machine or copied host is offline.");
                ui.label("Bind address (specific loopback or LAN IP:port)");ui.text_edit_singleline(&mut self.restore_bind);
                if summary.lifecycle!=RoomLifecycle::Archived{ui.checkbox(&mut self.fresh,"Create a fresh managed public journal before adopting the host");}else{self.fresh=false;ui.label("Archived state remains read-only; no new journal is needed.");}
                if summary.kind==RecoveryKind::Journal&&!self.fresh&&summary.lifecycle!=RoomLifecycle::Archived{ui.label("Without a fresh managed journal, start an external journal in Network Room controls before resuming.");}
                if summary.tail_bytes>0{ui.label(format!("{} trailing bytes belong to an incomplete record. They are not restored and remain in the original file.",summary.tail_bytes));ui.checkbox(&mut self.acknowledge_tail,"Use only the last complete validated record");}
                let source_active=active.is_some_and(|path|Store::id(path)==Store::id(preview.source()));
                ui.horizontal_wrapped(|ui|{restore=ui.add_enabled(can_restore&&!source_active&&!self.busy()&&self.acknowledge_tail,egui::Button::new("Restore reviewed state, paused")).clicked();cancel=ui.button("Cancel").clicked();});
            });
            if restore {
                self.request_restore(ctx, preview, self.restore_bind.clone(), self.fresh);
            }
            if cancel {
                self.discard_restore = true;
                self.restore = false;
            }
        }
        if let Some((kind, id, seal, digest)) = self.confirm.clone() {
            let mut apply = false;
            let mut cancel = false;
            let entry = self
                .store
                .as_ref()
                .and_then(|store| store.index.entries.get(&id))
                .cloned();
            egui::Modal::new(egui::Id::unique("room-recovery-file-action")).show(ctx,|ui|{
                ui.set_max_width(680.0);ui.heading("Review recovery file management");ui.label(format!("Action: {kind}"));if let Some(entry)=&entry{ui.label(&entry.label);ui.label(&entry.path);}
if !digest.is_empty(){ui.monospace(&digest);}
                match kind.as_str(){"forget"=>{ui.label("Forget local catalog metadata only. Original source files remain available and managed files remain discoverable.");},"quarantine"=>{ui.label("Move this exact inactive owned vault file into private quarantine. The bytes are preserved; external and active files are protected.");},"purge"=>{ui.label("Permanently remove only this exact inactive owned quarantine file. The file digest and catalog revision must still match.");ui.checkbox(&mut self.confirm_delete,"Permanently delete these reviewed owned bytes");},_=>()}
                let active=entry.as_ref().is_some_and(|entry|active.is_some_and(|path|Store::id(path)==entry.id));
                ui.horizontal_wrapped(|ui|{apply=ui.add_enabled(!self.busy()&&!active&&(kind!="purge"||self.confirm_delete),egui::Button::new("Apply this exact action")).clicked();cancel=ui.button("Cancel").clicked();});
            });
            if apply {
                self.update(ctx, kind, id, seal, digest);
                self.confirm = None;
            }
            if cancel {
                self.confirm = None;
            }
        }
    }
}
fn age(at: u64) -> String {
    let now = forge_room_core::now_epoch_seconds();
    if at > now {
        return "with a future clock value".into();
    }
    let seconds = now - at;
    match seconds {
        0..=59 => format!("{seconds}s ago"),
        60..=3599 => format!("{}m ago", seconds / 60),
        3600..=86399 => format!("{}h ago", seconds / 3600),
        _ => format!("{}d ago", seconds / 86400),
    }
}
