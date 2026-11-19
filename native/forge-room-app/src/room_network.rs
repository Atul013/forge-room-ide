//! Native controls for the isolated network Room. Network work never blocks egui polling.
use eframe::egui;
use forge_room_core::transport::{Action, Client, Host, Invite, RoomLifecycle, Snapshot};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::Duration,
};

enum Command {
    Action(Action),
    Stop,
}

fn member_action(action: &Action) -> bool {
    matches!(
        action,
        Action::Message { .. }
            | Action::ContextMessage { .. }
            | Action::MentionMessage { .. }
            | Action::React { .. }
            | Action::RequestHandoff { .. }
            | Action::SharePatch { .. }
            | Action::ReviewPatch { .. }
            | Action::ShareFocus { .. }
            | Action::ShareLocation { .. }
            | Action::ReportShipCheck { .. }
            | Action::UpdateReadiness { .. }
            | Action::RequestHelp { .. }
            | Action::CloseHelp { .. }
            | Action::DeclareImpact { .. }
            | Action::AcknowledgeImpact { .. }
    )
}

type Update = Result<(Snapshot, Option<(String, Action)>), String>;
struct Connection {
    commands: SyncSender<Command>,
    updates: Receiver<Update>,
}
impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.commands.try_send(Command::Stop);
    }
}

pub struct NetworkRoom {
    host: Option<Host>,
    connection: Option<Connection>,
    invites: Vec<(Invite, bool)>,
    bind: String,
    name: String,
    invite_text: String,
    workflow: crate::room_network_workflow::Workflow,
    changes: crate::room_changes::Changes,
    notice: Option<String>,
    snapshot: Snapshot,
    status: String,
    confirm_archive: bool,
    recovery: crate::room_recovery::State,
    launch: crate::room_launch::State,
    journal_lease: Option<crate::room_recovery::Lease>,
    journal_path: Option<PathBuf>,
}
impl Default for NetworkRoom {
    fn default() -> Self {
        Self {
            host: None,
            connection: None,
            invites: Vec::new(),
            bind: "127.0.0.1:0".into(),
            name: crate::room_state::local_user_name(),
            invite_text: String::new(),
            workflow: Default::default(),
            changes: Default::default(),
            notice: None,
            snapshot: Snapshot::default(),
            status: "Offline".into(),
            confirm_archive: false,
            recovery: Default::default(),
            launch: Default::default(),
            journal_lease: None,
            journal_path: None,
        }
    }
}
impl Drop for NetworkRoom {
    fn drop(&mut self) {
        self.connection.take();
        let lease = self.journal_lease.take();
        if let Some(host) = self.host.take() {
            thread::spawn(move || {
                drop(host);
                drop(lease);
            });
        }
    }
}
impl NetworkRoom {
    pub fn recovery_frame(&mut self, ctx: &egui::Context, root: Option<&Path>) {
        self.launch.tick();
        if let Some(mut prepared) = self.launch.ready.take() {
            if self.host.is_none() && self.connection.is_none() {
                if let (Some(host), Some(client)) = (prepared.host.take(), prepared.client.take()) {
                    self.snapshot = prepared.snapshot.clone();
                    self.host = Some(host);
                    let snapshot = prepared.snapshot.clone();
                    self.attach(move || Ok((client, snapshot)));
                    self.invites.clear();
                }
            } else {
                self.launch.operation_notice("Network Room changed before launch adoption; temporary host closed, existing Room preserved".into());
            }
        }
        self.launch
            .show(ctx, self.host.is_none() && self.connection.is_none());
        self.recovery.tick(ctx, root);
        if let Some(creation) = self.recovery.create.take() {
            let creation = *creation;
            let matches = self
                .host
                .as_ref()
                .and_then(|host| host.snapshot().ok())
                .is_some_and(|snapshot| snapshot.shared.room_id == creation.room);
            if matches {
                let reservation = creation.reservation;
                let host = self.host.as_ref().expect("matching recovery host");
                let result = match reservation.kind {
                    forge_room_core::transport::RecoveryKind::Checkpoint => {
                        host.save_checkpoint(&reservation.path)
                    }
                    forge_room_core::transport::RecoveryKind::Journal => {
                        host.enable_journal(&reservation.path)
                    }
                };
                self.status = match result {
                    Ok(()) => {
                        let path = reservation.path;
                        let kind = reservation.kind;
                        if kind == forge_room_core::transport::RecoveryKind::Journal {
                            self.journal_path = Some(reservation.lease.path.clone());
                            self.journal_lease = Some(reservation.lease);
                        } else {
                            drop(reservation.lease);
                        }
                        self.recovery.remember(
                            ctx,
                            path,
                            creation.project.as_deref().map(Path::new),
                        );
                        format!(
                            "Managed {} saved and flushed; catalog registration follows separately",
                            kind.label()
                        )
                    }
                    Err(error) => {
                        self.recovery.remember(
                            ctx,
                            reservation.path,
                            creation.project.as_deref().map(Path::new),
                        );
                        error
                    }
                };
            } else {
                self.status="Managed path was reserved but not written because the hosted Room changed; inspect or forget its private reservation".into();
            }
            self.recovery.operation_notice(self.status.clone());
        }
        if let Some(mut restored) = self.recovery.ready.take() {
            if self.host.is_none() && self.connection.is_none() {
                match restored
                    .host
                    .as_ref()
                    .ok_or("Recovered host unavailable".to_string())
                    .and_then(|host| host.snapshot())
                {
                    Ok(snapshot) => {
                        self.snapshot = snapshot;
                        self.host = restored.host.take();
                        self.journal_lease = restored.lease.take();
                        self.journal_path = restored.journal.take();
                        self.status = restored.notice.clone();
                        self.invites.clear();
                        self.recovery.operation_notice(self.status.clone());
                    }
                    Err(error) => {
                        self.status = error;
                        self.recovery.operation_notice(self.status.clone());
                    }
                }
            } else {
                self.recovery.operation_notice("Network Room changed before restore adoption; the temporary paused host was closed and source files preserved".into());
            }
        }
        self.recovery.show(
            ctx,
            self.host.is_none() && self.connection.is_none(),
            &self.bind,
            self.journal_path.as_deref(),
        );
    }
    pub fn stage_blueprint(&mut self, blueprint: forge_room_core::transport::RoomBlueprint) {
        self.launch.replace(blueprint);
        self.launch.open = true;
    }
    pub fn workflow_tasks(&self) -> Vec<forge_room_core::Task> {
        self.snapshot.shared.tasks.clone()
    }
    pub fn workflow_room_id(&self) -> String {
        let id = &self.snapshot.shared.room_id;
        if id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            id.clone()
        } else {
            String::new()
        }
    }
    pub fn stage_workflow_receipt(
        &mut self,
        room: &str,
        task: &str,
        text: String,
    ) -> Result<(), String> {
        if room.is_empty() || room != self.workflow_room_id() {
            return Err("Linked Room identity changed; receipt was not staged".into());
        }
        if self.connection.is_none() && self.host.is_none() {
            return Err("Connect to the matching Network Room before staging evidence".into());
        }
        self.workflow
            .stage_workflow_receipt(&self.snapshot, task, text)
    }
    pub fn cancel_local_checks(&mut self) {
        self.workflow.readiness.preflight.reset();
    }
    pub fn take_ai_brief(&mut self) -> Option<String> {
        self.workflow.readiness.brief.take()
    }
    pub fn take_files_changed(&mut self) -> bool {
        std::mem::take(&mut self.changes.changed)
    }
    pub fn take_open_location(&mut self) -> Option<forge_room_core::transport::CodeReference> {
        self.workflow.take_open_location()
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        workspace: &str,
        root: Option<&Path>,
        trusted: bool,
        dirty: &[PathBuf],
    ) {
        if let Some(connection) = &self.connection {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
            while let Ok(update) = connection.updates.try_recv() {
                match update {
                    Ok((snapshot, notice)) => {
                        if let Some((error, action)) = notice {
                            self.workflow.restore(action);
                            self.notice = Some(error);
                        }
                        self.snapshot = snapshot;
                        self.status = match self.snapshot.lifecycle {
                            RoomLifecycle::Active => "Connected · TLS · invite identity".into(),
                            RoomLifecycle::Paused => "Connected · Room paused by host".into(),
                            RoomLifecycle::Archived => "Connected · Room archived read-only".into(),
                        };
                    }
                    Err(error) => self.status = error,
                }
            }
        }
        if self.host.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
        if self.connection.is_none()
            && let Some(host) = &self.host
        {
            match host.snapshot() {
                Ok(snapshot) => self.snapshot = snapshot,
                Err(error) => self.status = error,
            }
        }
        let mut local_rejoin = None;
        let panel_height = (ui.available_height() * 0.65).max(120.0);
        self.workflow
            .readiness
            .preflight
            .reconcile(root.filter(|_| trusted), &self.snapshot);
        let unread_activity = self.workflow.observe_activity(&self.snapshot);
        if ui.button("Open Room Launch Center").clicked() {
            self.launch.open = true;
        }
        if self.host.is_some() && ui.button("Capture public setup as blueprint").clicked() {
            self.launch.capture(&self.snapshot);
        }
        if ui.button("Open Room Recovery Center").clicked() {
            self.recovery.open = true;
        }
        let header = if unread_activity > 0 {
            format!("Network Room · {unread_activity} new activity")
        } else {
            "Network Room · host / join".to_owned()
        };
        egui::CollapsingHeader::new(header).default_open(false).show(ui, |ui| {
          egui::ScrollArea::vertical().id_salt("network-room-controls").max_height(panel_height).show(ui, |ui| {
            ui.label(&self.status);
            if let Some(notice)=&self.notice { ui.label(notice); }
            if self.notice.is_some() && ui.small_button("Dismiss notice").clicked() {self.notice=None;}
            ui.label("Shares your name and items created in this network Room: roster, tasks, handoffs, evidence, activity and public messages. Direct messages go only to sender and recipient. Saved local Room data, files, terminals and agents are not shared by joining. Saved patches require a separate reviewed recipient grant.");
            if self.connection.is_none() && self.host.is_none() {
                ui.label("Your display name");
                ui.add(egui::TextEdit::singleline(&mut self.name).char_limit(64));
                ui.label("Host address (specific loopback or LAN IP:port)");
                ui.text_edit_singleline(&mut self.bind);
                if ui.button("Host network Room").clicked() {
                    let result = self.bind.parse().map_err(|_| "Enter an IP:port".to_string()).and_then(|bind| Host::start(bind, workspace));
                    match result {
                        Ok(host) => {
                            match host.invite(Duration::from_secs(60)) {
                                Ok(invite) => { self.connect(invite); self.host = Some(host); }
                                Err(error) => self.status = error,
                            }
                        }
                        Err(error) => self.status = error,
                    }
                }
                if ui.button("Inspect checkpoint for recovery...").clicked()
                    && let Some(path) = rfd::FileDialog::new().add_filter("Room checkpoint", &["json"]).pick_file() {
                    self.recovery.inspect_selected(ui.ctx(),path);
                }
                if ui.button("Inspect public journal for recovery...").clicked()
                    && let Some(path) = rfd::FileDialog::new().add_filter("Room journal", &["frjournal"]).pick_file() {
                    self.recovery.inspect_selected(ui.ctx(),path);
                }
                ui.separator();
                ui.label("Paste a private invite from the host");
                ui.add(egui::TextEdit::singleline(&mut self.invite_text).password(true).char_limit(16_384));
                let preview = Invite::decode(&self.invite_text);
                if let Ok(invite) = &preview { ui.label(format!("Join {} at {}. The invite pins this host's TLS certificate.", invite.room_name(), invite.address())); }
                if ui.add_enabled(preview.is_ok(), egui::Button::new("Join network Room")).clicked()
                    && let Ok(invite) = preview { self.connect(invite); self.invite_text.clear();
                }
                ui.label("Self-hosted preview: send invites privately. No discovery, relay or account identity. LAN requires a reachable host and firewall configuration.");
            } else {
                if let Some(host) = &self.host {
                    ui.label(format!("Hosting at {} · {}", host.address(), match self.snapshot.lifecycle { RoomLifecycle::Active => "active", RoomLifecycle::Paused => "paused and read-only", RoomLifecycle::Archived => "archived and read-only" }));
                    match self.snapshot.lifecycle {
                        RoomLifecycle::Active => {
                            if ui.button("Pause Room").clicked() {
                                match host.pause() { Ok(()) => { self.snapshot.lifecycle = RoomLifecycle::Paused; self.status = "Room paused".into(); }, Err(error) => self.status = error }
                                self.confirm_archive = false;
                            }
                        }
                        RoomLifecycle::Paused => {
                            if ui.button("Resume Room").clicked() {
                                match host.resume() { Ok(()) => { self.snapshot.lifecycle = RoomLifecycle::Active; self.status = "Room resumed".into(); }, Err(error) => self.status = error }
                                self.confirm_archive = false;
                            }
                        }
                        RoomLifecycle::Archived => { ui.label("Archived Room remains connected for read-only review until the host ends it."); }
                    }
                    if self.snapshot.lifecycle != RoomLifecycle::Archived {
                        if !self.confirm_archive {
                            if ui.button("Archive Room…").clicked() { self.confirm_archive = true; }
                        } else {
                            ui.label("Archive is final for this session. Members can review it but cannot join or change it.");
                            if ui.button("Confirm archive as read-only").clicked() {
                                match host.archive() { Ok(()) => { self.snapshot.lifecycle = RoomLifecycle::Archived; self.status = "Room archived read-only".into(); self.confirm_archive = false; }, Err(error) => self.status = error }
                            }
                            if ui.small_button("Cancel archive").clicked() { self.confirm_archive = false; }
                        }
                    }
                    ui.horizontal_wrapped(|ui|{
                        if ui.add_enabled(!self.recovery.busy(),egui::Button::new("Save checkpoint in managed vault")).clicked(){self.recovery.reserve_managed(ui.ctx(),forge_room_core::transport::RecoveryKind::Checkpoint,self.snapshot.shared.room_id.clone(),self.snapshot.room.clone(),root);}
                        if ui.add_enabled(!self.recovery.busy(),egui::Button::new("Start / rotate managed journal")).clicked(){self.recovery.reserve_managed(ui.ctx(),forge_room_core::transport::RecoveryKind::Journal,self.snapshot.shared.room_id.clone(),self.snapshot.room.clone(),root);}
                    });
                    match host.journal_bytes() {
                        Ok(Some(bytes)) => { ui.label(format!("Automatic public recovery journal: {bytes} / 67108864 bytes. Writes are flushed before acknowledgement.")); },
                        Ok(None) => { ui.label("Automatic recovery journal is off. Enable it to preserve subsequent public mutations."); },
                        Err(error) => { ui.colored_label(egui::Color32::RED, error); }
                    }
                    if ui.add_enabled(!self.recovery.busy(),egui::Button::new("Start / rotate public recovery journal...")).clicked()
                        && let Some(path) = rfd::FileDialog::new().set_file_name("room.frjournal").save_file() {
                        self.status = match self.recovery.external_destination(&path).and_then(|_|host.enable_journal(&path)) { Ok(()) => {self.journal_path=Some(path.canonicalize().unwrap_or_else(|_|path.clone()));self.journal_lease=None;self.recovery.remember(ui.ctx(),path,root);"Public journal active; private histories and credentials excluded".into()}, Err(error) => error };
                    }
                    ui.label("Recovery checkpoints save public Room work only. DMs, patches, credentials and live focus are excluded. Changes after the last save are not recovered.");
                    if ui.add_enabled(!self.recovery.busy(),egui::Button::new("Save new recovery checkpoint...")).clicked()
                        && let Some(path) = rfd::FileDialog::new().set_file_name("room-checkpoint.json").save_file() {
                        self.status = match self.recovery.external_destination(&path).and_then(|_|host.save_checkpoint(&path)) { Ok(()) => {self.recovery.remember(ui.ctx(),path,root);"Checkpoint saved and flushed; save again after further work".into()}, Err(error) => error };
                    }
                    if self.connection.is_none() && self.snapshot.lifecycle == RoomLifecycle::Active
                        && ui.button("Join recovered Room as a new member").clicked() {
                        match host.invite(Duration::from_secs(60)) { Ok(invite) => local_rejoin = Some(invite), Err(error) => self.status = error }
                    }
                    for member in host.recovered_members().unwrap_or_default() {
                        ui.label(format!("Recovered owner: {} ({}) - offline", member.name, member.id.chars().take(8).collect::<String>()));
                        ui.horizontal_wrapped(|ui| {
                            let active = self.snapshot.lifecycle == RoomLifecycle::Active;
                            if ui.add_enabled(active, egui::Button::new("Copy owner reclaim invite")).on_hover_text("Give privately only to this original owner; it restores their task authority").clicked() {
                                match host.reclaim_invite(&member.id, Duration::from_secs(300)) {
                                    Ok(invite) => { ui.ctx().copy_text(invite.encode()); self.invites.push((invite, false)); },
                                    Err(error) => self.status = error,
                                }
                            }
                            if self.connection.is_none() && ui.add_enabled(active, egui::Button::new("Rejoin locally as this owner")).clicked() {
                                match host.reclaim_invite(&member.id, Duration::from_secs(60)) { Ok(invite) => local_rejoin = Some(invite), Err(error) => self.status = error }
                            }
                        });
                    }
                    if ui.button("Copy public Room archive JSON").clicked() {
                        match host.snapshot().and_then(|snapshot| serde_json::to_string_pretty(&snapshot).map_err(|error| error.to_string())) {
                            Ok(text) => { ui.ctx().copy_text(text); self.status = "Public archive copied; private messages and patches are excluded".into(); }
                            Err(error) => self.status = format!("Archive export failed: {error}"),
                        }
                    }
                    if ui.add_enabled(self.snapshot.lifecycle == RoomLifecycle::Active && self.invites.len() < 32, egui::Button::new("Copy single-use invite · expires in 5 minutes")).clicked() {
                        match host.invite(Duration::from_secs(300)) {
                            Ok(invite) => { ui.ctx().copy_text(invite.encode()); self.invites.push((invite, false)); }
                            Err(error) => self.status = error,
                        }
                    }
                    for (index, (invite, revoked)) in self.invites.iter_mut().enumerate() {
                        if ui.add_enabled(!*revoked, egui::Button::new(format!("Revoke invite {} and its joined member{}", index + 1, if *revoked { " · revoked" } else { "" }))).clicked() {
                            match host.revoke(invite) { Ok(()) => *revoked = true, Err(error) => self.status = error }
                        }
                    }
                }
                if ui.button(if self.host.is_some() { "End hosted Room" } else { "Leave network Room" }).clicked() {
                    let recovery=std::mem::take(&mut self.recovery);
                    let launch=std::mem::take(&mut self.launch);
                    let mut next=Self::default();
                    next.recovery=recovery;
                    next.launch=launch;
                    *self=next;
                    return;
                }
                ui.separator();
                for member in &self.snapshot.members {
                    ui.label(format!("{} ({}) · {}", member.name, member.id.chars().take(8).collect::<String>(), if member.online { "online" } else { "offline" }));
                }
                let mut actions=self.workflow.show(ui,&self.snapshot,self.host.is_some(),root.filter(|_|trusted));
                actions.extend(self.changes.show(ui,&self.snapshot,root,trusted,dirty,self.status.starts_with("Connected")));
                for action in actions {
                    if !member_action(&action) && let Some(host)=&self.host {
                        if let Err(error)=host.control(action.clone()) {self.workflow.restore(action);self.notice=Some(error);}
                    } else if let Some(connection)=&self.connection {
                        if let Err(error)=connection.commands.try_send(Command::Action(action)) {
                            if let Command::Action(action)=match error {mpsc::TrySendError::Full(command)|mpsc::TrySendError::Disconnected(command)=>command} {self.workflow.restore(action);}
                            self.notice=Some("Action queue busy or disconnected; try again".into());
                        }
                    } else {
                        self.workflow.restore(action);
                        self.notice=Some("Join the recovered Room before sending member actions".into());
                    }
                }
                ui.label("Presence expires after 10 seconds offline. Active sessions renew; reconnect retains identity for one idle hour while the host runs. Local Room tasks and files are not imported.");
            }
          });
        });
        if let Some(invite) = local_rejoin {
            self.connect(invite);
        }
        ui.separator();
    }
    fn connect(&mut self, invite: Invite) {
        let name = self.name.clone();
        self.attach(move || Client::join(invite, &name));
    }
    fn attach(
        &mut self,
        join: impl FnOnce() -> Result<(Client, Snapshot), String> + Send + 'static,
    ) {
        let (commands, receiver) = mpsc::sync_channel(4);
        let (sender, updates) = mpsc::sync_channel(4);
        self.status = "Connecting…".into();
        self.connection = Some(Connection { commands, updates });
        thread::spawn(move || {
            let (mut client, mut snapshot) = match join() {
                Ok(joined) => joined,
                Err(error) => {
                    let _ = sender.send(Err(format!(
                        "Join failed: {error}. Leave to try another invite."
                    )));
                    return;
                }
            };
            let mut pending = std::collections::VecDeque::new();
            let mut rejections = std::collections::VecDeque::new();
            let _ = sender.try_send(Ok((snapshot.clone(), None)));
            loop {
                let command = if pending.len() < 4 {
                    receiver.recv_timeout(Duration::from_millis(500))
                } else {
                    thread::sleep(Duration::from_millis(500));
                    Err(mpsc::RecvTimeoutError::Timeout)
                };
                match command {
                    Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let _ = client.leave();
                        break;
                    }
                    Ok(Command::Action(text)) => {
                        pending.push_back(text);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let result = if let Some(text) = pending.front() {
                    client
                        .execute(text.clone())
                        .map(|outcome| (outcome.snapshot, outcome.rejection))
                } else {
                    client.poll().map(|snapshot| (snapshot, None))
                };
                match result {
                    Ok((mut update, rejection)) => {
                        if let Some(error) = rejection
                            && let Some(action) = pending.front()
                        {
                            rejections.push_back((error, action.clone()));
                        }
                        pending.pop_front();
                        if update.reset {
                            snapshot.events.clear();
                        }
                        snapshot.events.append(&mut update.events);
                        if snapshot.events.len() > 128 {
                            snapshot.events.drain(..snapshot.events.len() - 128);
                        }
                        snapshot.members = update.members;
                        snapshot.cursor = update.cursor;
                        snapshot.shared = update.shared;
                        snapshot.patches = update.patches;
                        snapshot.reactions = update.reactions;
                        snapshot.server_time_epoch_seconds = update.server_time_epoch_seconds;
                        snapshot.lifecycle = update.lifecycle;
                        match sender.try_send(Ok((snapshot.clone(), rejections.front().cloned()))) {
                            Ok(()) => {
                                rejections.pop_front();
                            }
                            Err(mpsc::TrySendError::Disconnected(_)) => break,
                            Err(mpsc::TrySendError::Full(_)) => {}
                        }
                    }
                    Err(error) => {
                        if matches!(
                            sender.try_send(Err(format!("Disconnected: {error}. Retrying…"))),
                            Err(mpsc::TrySendError::Disconnected(_))
                        ) {
                            break;
                        }
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;
    use forge_room_core::transport::{ShipCheck, ShipPlan, ShipResult};

    #[test]
    fn hosting_members_report_checks_and_readiness_as_the_joined_identity() {
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "routing").unwrap();
        let (mut client, _) = Client::join(
            host.invite(Duration::from_secs(60)).unwrap(),
            "Local member",
        )
        .unwrap();
        let plan = Action::SetShipPlan {
            expected_revision: 0,
            plan: ShipPlan {
                title: "Release".into(),
                checks: vec![ShipCheck {
                    title: "Smoke".into(),
                    owner: client.identity.clone(),
                }],
                ..Default::default()
            },
        };
        assert!(!member_action(&plan));
        host.control(plan).unwrap();
        let report = Action::ReportShipCheck {
            expected_revision: 1,
            check: 0,
            result: ShipResult::ClaimedPass,
            note: "manual smoke".into(),
        };
        assert!(member_action(&report));
        assert!(host.control(report.clone()).is_err());
        assert!(client.execute(report).unwrap().rejection.is_none());
        host.control(Action::CreateTask {
            title: "Owner task".into(),
            owner: client.identity.clone(),
            criteria: vec![],
            dependencies: vec![],
            priority: forge_room_core::TaskPriority::Normal,
            due_hours: None,
        })
        .unwrap();
        let readiness = Action::UpdateReadiness {
            task: "T1".into(),
            expected_revision: 0,
            profile: Default::default(),
        };
        assert!(member_action(&readiness));
        assert!(host.control(readiness.clone()).is_err());
        let outcome = client.execute(readiness).unwrap();
        assert!(outcome.rejection.is_none());
        assert_eq!(
            outcome.snapshot.shared.shipping.reports[0].actor,
            client.identity
        );
        assert_eq!(
            outcome.snapshot.shared.readiness.profiles[0].owner,
            client.identity
        );
    }
}

#[cfg(test)]
mod recovery_integration_tests {
    use super::*;
    use std::{fs, time::Instant};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "forge-network-recovery-{}",
                forge_room_core::workflows::new_id().unwrap()
            ));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
        fn catalog(&self) -> PathBuf {
            self.0.join("catalog")
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
                    .starts_with("forge-network-recovery-")
            );
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn indexed(catalog: &Path, kind: &str) -> bool {
        let Ok(entries) = fs::read_dir(catalog.join("states")) else {
            return false;
        };
        let mut paths = entries
            .flatten()
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .last()
            .and_then(|path| {
                crate::local_records::read::<serde_json::Value>(path, 1024 * 1024).ok()
            })
            .and_then(|value| value["entries"].as_object().cloned())
            .is_some_and(|entries| {
                entries
                    .values()
                    .any(|entry| entry["summary"]["kind"] == kind)
            })
    }
    #[test]
    fn managed_save_rotation_catalog_registration_and_host_close_keep_writer_lease_owned_together()
    {
        let fixture = Fixture::new();
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "Network managed").unwrap();
        let mut room = NetworkRoom::default();
        room.snapshot = host.snapshot().unwrap();
        room.host = Some(host);
        room.recovery = crate::room_recovery::State::at(&fixture.catalog());
        let context = egui::Context::default();
        room.recovery.reserve_managed(
            &context,
            forge_room_core::transport::RecoveryKind::Checkpoint,
            room.snapshot.shared.room_id.clone(),
            room.snapshot.room.clone(),
            None,
        );
        let deadline = Instant::now() + Duration::from_secs(15);
        while !indexed(&fixture.catalog(), "checkpoint") {
            room.recovery_frame(&context, None);
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        while room.recovery.busy() {
            room.recovery_frame(&context, None);
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        room.recovery.reserve_managed(
            &context,
            forge_room_core::transport::RecoveryKind::Journal,
            room.snapshot.shared.room_id.clone(),
            room.snapshot.room.clone(),
            None,
        );
        let deadline = Instant::now() + Duration::from_secs(15);
        while room.journal_path.is_none() || !indexed(&fixture.catalog(), "journal") {
            room.recovery_frame(&context, None);
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        let journal = room.journal_path.clone().unwrap();
        assert!(crate::room_recovery::lease_for_test(Some(&fixture.catalog()), &journal).is_err());
        drop(room);
        let deadline = Instant::now() + Duration::from_secs(10);
        while crate::room_recovery::lease_for_test(Some(&fixture.catalog()), &journal).is_err() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert!(journal.exists());
        assert!(forge_room_core::transport::inspect_recovery(&journal).is_ok());
    }
    #[test]
    fn blueprint_launch_adopts_the_prepared_member_and_never_replaces_an_existing_room() {
        let fixture = Fixture::new();
        let ctx = egui::Context::default();
        let mut room = NetworkRoom::default();
        room.recovery = crate::room_recovery::State::at(&fixture.catalog());
        let blueprint = forge_room_core::transport::RoomBlueprint::starters().remove(1);
        let prepare = || {
            let (host, client, snapshot) =
                Host::from_blueprint("127.0.0.1:0".parse().unwrap(), &blueprint, "Owner", false)
                    .unwrap();
            Box::new(crate::room_launch::Prepared {
                host: Some(host),
                client: Some(client),
                snapshot,
            })
        };
        room.launch.ready = Some(prepare());
        room.recovery_frame(&ctx, None);
        assert!(room.connection.is_some());
        assert_eq!(room.snapshot.shared.tasks.len(), 4);
        assert!(room.invites.is_empty());
        let first = room.host.as_ref().unwrap().address();
        let identity = room.snapshot.shared.tasks[0].owner_id.clone();
        room.launch.ready = Some(prepare());
        room.recovery_frame(&ctx, None);
        assert_eq!(room.host.as_ref().unwrap().address(), first);
        assert_eq!(room.snapshot.shared.tasks[0].owner_id, identity);
    }
    #[test]
    fn reviewed_host_adopts_paused_and_never_replaces_an_existing_host() {
        let fixture = Fixture::new();
        let source = fixture.0.join("source.json");
        let original = Host::start("127.0.0.1:0".parse().unwrap(), "Restored network").unwrap();
        original.save_checkpoint(&source).unwrap();
        drop(original);
        let preview = forge_room_core::transport::inspect_recovery(&source).unwrap();
        let context = egui::Context::default();
        let mut room = NetworkRoom::default();
        room.recovery = crate::room_recovery::State::at(&fixture.catalog());
        room.recovery
            .request_restore(&context, preview.clone(), "127.0.0.1:0".into(), true);
        let deadline = Instant::now() + Duration::from_secs(15);
        while room.host.is_none() {
            room.recovery_frame(&context, None);
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(room.snapshot.lifecycle, RoomLifecycle::Paused);
        assert!(room.journal_lease.is_some());
        assert!(room.invites.is_empty());
        let first = room.host.as_ref().unwrap().address();
        while room.recovery.busy() {
            room.recovery_frame(&context, None);
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        room.recovery
            .request_restore(&context, preview, "127.0.0.1:0".into(), false);
        let deadline = Instant::now() + Duration::from_secs(15);
        while room.recovery.busy() {
            room.recovery_frame(&context, None);
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(room.host.as_ref().unwrap().address(), first);
        assert_eq!(room.snapshot.room, "Restored network");
    }
}
