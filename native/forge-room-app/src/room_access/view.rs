use super::*;
use eframe::egui;
use forge_room_core::transport::RoomLifecycle;
impl State {
    pub(super) fn view(
        &mut self,
        ctx: &egui::Context,
        offline: bool,
        host: Option<&Host>,
        project: Option<&Path>,
    ) {
        if host.is_none() {
            self.fresh_invite = None;
            self.revocation = None;
            self.grant_selection.clear();
        }
        if !self.open {
            return;
        }
        if !self.loaded && !self.worker.busy() {
            self.loaded = true;
            let root = self.catalog_root.clone();
            self.worker.request(ctx, move || Job::Catalog(open(root)));
        }
        let mut opened = self.open;
        egui::Window::new("Room Access Center").open(&mut opened).default_size([800.0,650.0]).resizable(true).show(ctx,|ui|{
            ui.heading("Review access before connecting");ui.label("Private invites authenticate a TLS host. Bookmarks store public host metadata only; they cannot sign in, prove a host is online or open a project.");ui.label(&self.notice);
            if !offline && ui.button(if host.is_some(){"Detach local member connection; keep host running"}else{"Leave current joined Room"}).clicked(){self.detach_member=true;}
            ui.horizontal_wrapped(|ui|{for (page,title) in ["Join review","Connection bookmarks","Host invitations"].iter().enumerate(){ui.selectable_value(&mut self.page,page,*title);}});
            if self.worker.busy(){ui.spinner();if self.joining && ui.button("Cancel pending join adoption").clicked(){self.cancelled=true;self.notice="Cancellation requested; a completed join will be closed with leave".into();}}
            egui::ScrollArea::vertical().id_salt("room-access").show(ui,|ui|{
                ui.add_enabled_ui(!self.worker.busy() && self.pending.is_none(),|ui|match self.page {0=>self.join_view(ui,offline,project),1=>self.bookmarks(ui),_=>self.host_view(ui,host)});
                if let Some(joined)=&self.pending {
                    ui.separator();ui.heading("Review joined Room and member identity");ui.label("The host accepted the invite. Its Room/member identity differs from the review, or this identity already owns tasks. Review before adoption. Different Room IDs create separate bookmarks. Cancelling sends leave; the single-use invite is consumed and reclaimed owners return offline.");
                    ui.label(format!("{} / {} / {} members / {} tasks",joined.profile.room,joined.profile.address,joined.snapshot.members.len(),joined.snapshot.shared.tasks.len()));ui.monospace(format!("Actual Room ID: {}",if joined.profile.room_id.is_empty(){"not supplied"}else{&joined.profile.room_id}));
                    ui.label(format!("Actual member: {} / {} / {} owned tasks",joined.snapshot.members.iter().find(|m|m.id==joined.snapshot.you).map(|m|m.name.as_str()).unwrap_or("Unavailable"),joined.snapshot.you,joined.snapshot.shared.tasks.iter().filter(|t|t.owner_id==joined.snapshot.you).count()));
                    if let Some(expected)=&joined.expected{ui.monospace(format!("Bookmarked Room ID: {}",expected.room_id));}
                    if ui.add_enabled(offline,egui::Button::new("Recheck membership and adopt this reviewed connection")).clicked(){self.adopt_pending(ui.ctx());}
                    if ui.button("Close this temporary joined connection").clicked(){self.pending.take();self.invite.clear();self.notice="Temporary connection closed with leave; saved bookmark unchanged".into();}
                    if !offline{ui.label("Leave/end the current Network Room before adopting this connection.");}
                }
            });
        });
        if self.open && !opened {
            self.cancelled |= self.joining;
            self.pending.take();
            self.review = None;
            self.fresh_invite = None;
            self.invite.clear();
            self.show_invite = false;
        }
        self.open = opened;
    }
    fn join_view(&mut self, ui: &mut egui::Ui, offline: bool, project: Option<&Path>) {
        if self.review.is_none() {
            ui.label("Private invite (kept in memory only)");
            ui.add(
                egui::TextEdit::multiline(&mut self.invite)
                    .password(!self.show_invite)
                    .desired_rows(3)
                    .char_limit(16384),
            );
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.show_invite, "Show private invite text");
                if ui.button("Clear private invite").clicked() {
                    self.invite.clear();
                }
            });
            ui.label("Your display name");
            ui.add(egui::TextEdit::singleline(&mut self.name).char_limit(64));
            if let Some(profile) = self.expected() {
                ui.label(format!(
                    "Comparing with bookmark: {} / {}",
                    profile.room, profile.address
                ));
                if ui.button("Join without a selected bookmark").clicked() {
                    self.selected.clear();
                }
            }
            if !self.selected.is_empty()
                && ui.button("Clear selected bookmark comparison").clicked()
            {
                self.selected.clear();
                self.review = None;
            }
            ui.label("Joining shares your display name and new Room activity. Local files, terminals, local saved Rooms, model credentials and agents are not shared. Expiry/revocation/single-use validity is checked by the host; the invite text cannot prove it is still valid.");
            if ui
                .add_enabled(
                    offline && !self.busy() && !self.invite.is_empty(),
                    egui::Button::new("Inspect private invite for review"),
                )
                .clicked()
                && let Err(e) = self.freeze(project)
            {
                self.notice = e;
            }
            if !offline {
                ui.label("Leave or end the current Network Room before joining another.");
            }
        } else {
            let review = self.review.as_ref().expect("review");
            let fingerprint = review.invite.certificate_fingerprint();
            ui.heading("Frozen private join review");
            ui.label(format!(
                "Room: {} / host: {} / member: {}",
                review.invite.room_name(),
                review.invite.address(),
                review.name
            ));
            ui.monospace(format!("TLS certificate SHA-256: {fingerprint}"));
            ui.label("This fingerprint comes from the invite. Check it privately with the sender; it is not verified account or publisher identity.");
            ui.label(if review.invite.address().ip().is_loopback(){"Loopback address: this invite points at this machine, not another LAN device."}else{"Specific network address: reachability and firewall configuration are not established by this preview."});
            let changed = review.expected.as_ref().is_some_and(|p| {
                !p.fingerprint.eq_ignore_ascii_case(&fingerprint)
                    || p.address != review.invite.address()
                    || p.room != review.invite.room_name()
            });
            if let Some(expected) = &review.expected {
                ui.label(format!(
                    "Saved host: {} / {}",
                    expected.room, expected.address
                ));
                ui.monospace(format!("Saved TLS fingerprint: {}", expected.fingerprint));
            }
            if changed {
                ui.label("Host identity/address/name differs from the bookmark. Recovery uses fresh TLS too; verify the new invite privately with the host before proceeding.");
                ui.checkbox(
                    &mut self.allow_changed,
                    "I checked the changed host identity with the host",
                );
            } else if review.expected.is_some() {
                ui.label("Certificate, address and Room name match the selected bookmark. This is not account/publisher authentication.");
            }
            ui.label("After connection the public Room ID is checked separately. Cancelling can consume the single-use invite if the host has already accepted it; completed temporary connections send leave. A saved project association is metadata only.");
            if ui
                .add_enabled(
                    offline && (!changed || self.allow_changed),
                    egui::Button::new("Join this reviewed host"),
                )
                .clicked()
            {
                self.connect(ui.ctx());
            }
            if ui.button("Back and discard review").clicked() {
                self.review = None;
                self.allow_changed = false;
            }
        }
    }
    fn bookmarks(&mut self, ui: &mut egui::Ui) {
        if let Some((profile, _, _)) = &self.last_joined {
            ui.label(format!(
                "Last locally adopted connection: {} / {}",
                profile.room, profile.address
            ));
            if ui
                .button("Save / retry this connection metadata without reconnecting")
                .clicked()
            {
                self.save_last_joined(ui.ctx());
            }
        }
        ui.label("Search saved name, Room or address");
        ui.text_edit_singleline(&mut self.query);
        let query = self.query.to_lowercase();
        let mut entries = self
            .store
            .as_ref()
            .map(|s| s.index.entries.clone())
            .unwrap_or_default();
        entries.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| b.joined_at.cmp(&a.joined_at))
                .then_with(|| a.profile.room.cmp(&b.profile.room))
        });
        for entry in entries.into_iter().filter(|e| {
            format!(
                "{} {} {}",
                e.profile.label, e.profile.room, e.profile.address
            )
            .to_lowercase()
            .contains(&query)
        }) {
            let id = entry.profile.id().unwrap_or_default();
            ui.push_id(&id, |ui| {
                let label = if entry.profile.label.is_empty() {
                    &entry.profile.room
                } else {
                    &entry.profile.label
                };
                if ui
                    .selectable_label(
                        self.selected == id,
                        format!(
                            "{}{} / {}",
                            if entry.pinned { "Pinned / " } else { "" },
                            label,
                            entry.profile.address
                        ),
                    )
                    .clicked()
                {
                    self.selected = id.clone();
                    self.label = entry.profile.label.clone();
                    self.review = None;
                }
                ui.label(match entry.joined_at {
                    Some(at) => format!(
                        "Latest saved local join observation {} minutes ago (cached)",
                        forge_room_core::now_epoch_seconds().saturating_sub(at) / 60
                    ),
                    None => "Imported public metadata; never joined locally".into(),
                });
                if self.selected == id {
                    ui.monospace(format!(
                        "Room ID: {}\nTLS fingerprint: {}",
                        if entry.profile.room_id.is_empty() {
                            "not supplied"
                        } else {
                            &entry.profile.room_id
                        },
                        entry.profile.fingerprint
                    ));
                    if let Some(project) = &entry.project {
                        ui.label(format!(
                            "Private project association (not opened/shared): {project}"
                        ));
                    }
                    ui.add(egui::TextEdit::singleline(&mut self.label).char_limit(160));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Save label").clicked() {
                            self.catalog_edit(
                                ui.ctx(),
                                Edit::Label(id.clone(), self.label.clone()),
                            );
                        }
                        if ui
                            .button(if entry.pinned { "Unpin" } else { "Pin" })
                            .clicked()
                        {
                            self.catalog_edit(ui.ctx(), Edit::Pin(id.clone(), !entry.pinned));
                        }
                        if ui.button("Use for fresh invite review").clicked() {
                            self.page = 0;
                            self.review = None;
                            self.invite.clear();
                        }
                        if ui.button("Remove bookmark...").clicked()
                            && let Some(seal) =
                                self.store.as_ref().and_then(|s| s.index.seal().ok())
                        {
                            self.forget = Some((id.clone(), seal));
                        }
                    });
                }
            });
        }
        ui.separator();
        ui.label("Up to 64 entries; eight private catalog versions retained. Removing a bookmark changes the current catalog only; retained versions can still contain metadata, including private project associations. It never revokes a host invite, deletes a project or erases Room history.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Refresh metadata").clicked() {
                let root = self.catalog_root.clone();
                self.worker
                    .request(ui.ctx(), move || Job::Catalog(open(root)));
            }
            if ui.button("Inspect bookmark import...").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Connection profiles", &["json"])
                    .pick_file()
            {
                self.worker
                    .request(ui.ctx(), move || Job::Profiles(profiles(&path)));
            }
            if ui
                .add_enabled(
                    self.store.is_some(),
                    egui::Button::new("Export public bookmark metadata..."),
                )
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_file_name("room-connections.json")
                    .save_file()
            {
                let value = self.store.as_ref().expect("catalog").index.public();
                self.worker.request(ui.ctx(), move || {
                    Job::Export(value.encode().and_then(|raw| {
                        crate::local_records::create_file(&path, &raw, storage::MAX_PROFILE)
                    }))
                });
            }
        });
        if let Some((value, seal)) = &self.import {
            ui.heading("Frozen bookmark import review");
            ui.label("Adds new public profiles only; existing entries keep their labels, pins and local history. Imported timestamps/project associations/authentication are unsupported. No connection starts.");
            ui.monospace(format!(
                "Profile SHA-256: {}",
                forge_room_core::workflows::seal(value).unwrap_or_default()
            ));
            for entry in &value.entries {
                ui.label(format!(
                    "{} / {} / {}",
                    entry.label, entry.room, entry.address
                ));
                ui.monospace(&entry.fingerprint);
            }
            if ui
                .add_enabled(
                    seal.is_some(),
                    egui::Button::new("Add these reviewed profiles"),
                )
                .clicked()
            {
                let (value, seal) = self.import.take().expect("import");
                let root = self.catalog_root.clone();
                self.worker
                    .request(ui.ctx(), move || update(root, seal, Edit::Import(value)));
            }
            if ui.button("Discard inspected import").clicked() {
                self.import = None;
            }
        }
        if let Some((id, seal)) = self.forget.clone() {
            ui.separator();
            ui.label("Remove this reference from the current catalog? Older retained metadata, including project associations, remains. No files or host grants are removed.");
            ui.monospace(&id);
            if ui.button("Confirm remove bookmark reference").clicked() {
                let root = self.catalog_root.clone();
                self.worker
                    .request(ui.ctx(), move || update(root, Some(seal), Edit::Forget(id)));
                self.forget = None;
                self.selected.clear();
            }
            if ui.button("Keep bookmark").clicked() {
                self.forget = None;
            }
        }
    }
    fn host_view(&mut self, ui: &mut egui::Ui, host: Option<&Host>) {
        let Some(host) = host else {
            self.fresh_invite = None;
            self.revocation = None;
            self.grant_selection.clear();
            ui.label("Host a Network Room to inspect or revoke its invitations. Client bookmarks confer no host authority.");
            return;
        };
        ui.label("Host-local inventory: public invitation IDs are not sign-in credentials. An invite is single-use; its expiry does not end an already joined session. Revocation removes its session access too. Task data remains; reclaimed owners return offline for a fresh private reclaim.");
        let rows = match host.invitation_summaries() {
            Ok(rows) => rows,
            Err(e) => {
                ui.label(e);
                return;
            }
        };
        self.grant_selection
            .retain(|id| rows.iter().any(|g| &g.id == id));
        if self.revocation.is_none() {
            if ui.button("Clear invitation selection").clicked() {
                self.grant_selection.clear();
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("Fresh private invite lifetime (seconds)");
                ui.add(egui::DragValue::new(&mut self.lifetime).range(1..=3600));
                if ui
                    .add_enabled(
                        host.lifecycle().ok() == Some(RoomLifecycle::Active),
                        egui::Button::new("Create fresh single-use invite"),
                    )
                    .clicked()
                {
                    match host.invite(std::time::Duration::from_secs(self.lifetime)) {
                        Ok(invite) => {
                            self.fresh_invite = Some(invite);
                            self.notice =
                                "Fresh invite created in memory; copy and send it privately".into();
                        }
                        Err(e) => self.notice = e,
                    }
                }
            });
            ui.collapsing("Private reclaim for retained offline owners",|ui|{
                for member in host.recovered_members().unwrap_or_default(){
                    ui.label(format!("Original owner: {} / {}",member.name,member.id));
                    if ui.add_enabled(host.lifecycle().ok()==Some(RoomLifecycle::Active),egui::Button::new(format!("Create private reclaim for {}",member.name))).clicked(){match host.reclaim_invite(&member.id,std::time::Duration::from_secs(self.lifetime)){Ok(invite)=>{self.fresh_invite=Some(invite);self.notice="Owner reclaim created. Give it privately only to this original owner, not a replacement person.".into();},Err(e)=>self.notice=e}}
                }
            });
            if let Some(invite) = &self.fresh_invite {
                let row = rows.iter().find(|g| g.id == invite.public_id());
                let usable = row.is_some_and(|g| !g.revoked && !g.used && !g.expired);
                if ui
                    .add_enabled(
                        usable,
                        egui::Button::new("Copy freshly created private invite"),
                    )
                    .clicked()
                {
                    ui.ctx().copy_text(invite.encode());
                }
                if ui.small_button("Discard private copy").clicked() {
                    self.fresh_invite = None;
                }
                ui.label("Discarding this local copy does not revoke the grant. Use reviewed revocation below.");
            }
            for row in &rows {
                ui.push_id(&row.id, |ui| {
                    let mut chosen = self.grant_selection.contains(&row.id);
                    if ui
                        .checkbox(
                            &mut chosen,
                            format!(
                                "{} / {} retained sessions / {}",
                                if row.revoked {
                                    "revoked"
                                } else if row.used {
                                    "used"
                                } else if row.expired {
                                    "expired"
                                } else {
                                    "unused"
                                },
                                row.sessions.len(),
                                if row.expired {
                                    "expired".into()
                                } else {
                                    format!("expires in {} s", row.remaining_seconds)
                                }
                            ),
                        )
                        .changed()
                    {
                        if chosen {
                            self.grant_selection.insert(row.id.clone());
                        } else {
                            self.grant_selection.remove(&row.id);
                        }
                    }
                    ui.monospace(format!("Public invite ID: {}", row.id));
                    if let Some(member) = &row.reclaim {
                        ui.label(format!("Owner reclaim: {} / {}", member.name, member.id));
                    }
                    for member in &row.sessions {
                        ui.label(format!("Affected member: {} / {}", member.name, member.id));
                    }
                });
            }
            if ui
                .add_enabled(
                    !self.grant_selection.is_empty(),
                    egui::Button::new("Review selected invitation revocation"),
                )
                .clicked()
            {
                match host.review_revocation(self.grant_selection.iter().cloned().collect()) {
                    Ok(review) => self.revocation = Some(review),
                    Err(e) => self.notice = e,
                }
            }
        } else {
            let review = self.revocation.as_ref().expect("revocation");
            ui.heading("Exact revocation review");
            ui.label("This can disconnect your own joined member too. All selected grants and affected identities must still match at confirmation; a newly joined member invalidates the entire review.");
            ui.monospace(review.digest());
            for grant in review.grants() {
                ui.monospace(&grant.id);
                for member in &grant.sessions {
                    ui.label(format!("Eject: {} / {}", member.name, member.id));
                }
            }
            if ui
                .button("Revoke reviewed grants and eject their sessions")
                .clicked()
            {
                let result = host.revoke_reviewed(self.revocation.as_ref().expect("review"));
                self.notice = match result {
                    Ok(count) => {
                        format!("Reviewed invitations revoked; {count} retained sessions removed")
                    }
                    Err(e) => e,
                };
                self.revocation = None;
                self.grant_selection.clear();
            }
            if ui.button("Keep these invitations").clicked() {
                self.revocation = None;
            }
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(250));
    }
}
