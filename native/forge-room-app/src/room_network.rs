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
        }
    }
}
impl Drop for NetworkRoom {
    fn drop(&mut self) {
        self.connection.take();
        if let Some(host) = self.host.take() {
            thread::spawn(move || drop(host));
        }
    }
}
impl NetworkRoom {
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
        let panel_height = (ui.available_height() * 0.65).max(120.0);
        self.workflow
            .readiness
            .preflight
            .reconcile(root.filter(|_| trusted), &self.snapshot);
        let unread_activity = self.workflow.observe_activity(&self.snapshot);
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
            if self.connection.is_none() {
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
                    *self = Self::default();
                    return;
                }
                ui.separator();
                for member in &self.snapshot.members {
                    ui.label(format!("{} ({}) · {}", member.name, member.id.chars().take(8).collect::<String>(), if member.online { "online" } else { "offline" }));
                }
                let mut actions=self.workflow.show(ui,&self.snapshot,self.host.is_some(),root.filter(|_|trusted));
                actions.extend(self.changes.show(ui,&self.snapshot,root,trusted,dirty,self.status.starts_with("Connected")));
                for action in actions {
                    if !matches!(action,Action::Message {..}|Action::ContextMessage {..}|Action::MentionMessage {..}|Action::React {..}|Action::RequestHandoff {..}|Action::SharePatch {..}|Action::ShareFocus {..}|Action::ShareLocation {..}) && let Some(host)=&self.host {
                        if let Err(error)=host.control(action.clone()) {self.workflow.restore(action);self.notice=Some(error);}
                    } else if let Some(connection)=&self.connection
                        && let Err(error)=connection.commands.try_send(Command::Action(action)) {
                            if let Command::Action(action)=match error {mpsc::TrySendError::Full(command)|mpsc::TrySendError::Disconnected(command)=>command} {self.workflow.restore(action);}
                            self.notice=Some("Action queue busy or disconnected; try again".into());
                    }
                }
                ui.label("Presence expires after 10 seconds offline. Active sessions renew; reconnect retains identity for one idle hour while the host runs. Local Room tasks and files are not imported.");
            }
          });
        });
        ui.separator();
    }
    fn connect(&mut self, invite: Invite) {
        let (commands, receiver) = mpsc::sync_channel(4);
        let (sender, updates) = mpsc::sync_channel(4);
        let name = self.name.clone();
        self.status = "Connecting…".into();
        self.connection = Some(Connection { commands, updates });
        thread::spawn(move || {
            let (mut client, mut snapshot) = match Client::join(invite, &name) {
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
