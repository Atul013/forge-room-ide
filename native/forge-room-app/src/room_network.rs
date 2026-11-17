//! Native controls for the isolated network Room. Network work never blocks egui polling.
use eframe::egui;
use forge_room_core::transport::{Client, Host, Invite, Snapshot};
use std::{
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::Duration,
};

enum Command {
    Post(String),
    Stop,
}
struct Connection {
    commands: SyncSender<Command>,
    updates: Receiver<Result<Snapshot, String>>,
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
    draft: String,
    snapshot: Snapshot,
    status: String,
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
            draft: String::new(),
            snapshot: Snapshot::default(),
            status: "Offline".into(),
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
    pub fn show(&mut self, ui: &mut egui::Ui, workspace: &str) {
        if let Some(connection) = &self.connection {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
            while let Ok(update) = connection.updates.try_recv() {
                match update {
                    Ok(snapshot) => {
                        self.snapshot = snapshot;
                        self.status = "Connected · TLS · invite identity".into();
                    }
                    Err(error) => self.status = error,
                }
            }
        }
        let panel_height = (ui.available_height() * 0.65).max(120.0);
        egui::CollapsingHeader::new("Network Room · host / join").default_open(false).show(ui, |ui| {
          egui::ScrollArea::vertical().id_salt("network-room-controls").max_height(panel_height).show(ui, |ui| {
            ui.label(&self.status);
            ui.label("Shares only your display name, network roster and messages typed below. Local tasks, chat, files, terminals and agents stay private.");
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
                    ui.label(format!("Hosting at {} · ends when this workspace closes", host.address()));
                    if ui.add_enabled(self.invites.len() < 32, egui::Button::new("Copy single-use invite · expires in 5 minutes")).clicked() {
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
                egui::ScrollArea::vertical().id_salt("network-room-chat").max_height(180.0).stick_to_bottom(true).show(ui, |ui| {
                    for event in &self.snapshot.events {
                        let name = self.snapshot.members.iter().find(|m| m.id == event.actor).map(|m| m.name.as_str()).unwrap_or("Former member");
                        ui.label(format!("{name}: {}", event.text));
                    }
                });
                ui.label("Public network message");
                ui.add(egui::TextEdit::multiline(&mut self.draft).desired_rows(2).char_limit(2000));
                if ui.add_enabled(!self.draft.trim().is_empty(), egui::Button::new("Send to network Room")).clicked()
                    && let Some(connection) = &self.connection {
                        match connection.commands.try_send(Command::Post(self.draft.clone())) {
                            Ok(()) => self.draft.clear(),
                            Err(_) => self.status = "Message queue busy; try again".into(),
                        }
                }
                ui.label("Presence expires after 10 seconds offline. Reconnect retains identity for up to 1 hour while the host runs. Recent history is bounded to 128 messages. DMs and local task sync are unavailable.");
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
            let _ = sender.try_send(Ok(snapshot.clone()));
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
                    Ok(Command::Post(text)) => {
                        pending.push_back(text);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let result = if let Some(text) = pending.front() {
                    client.post(text)
                } else {
                    client.poll()
                };
                match result {
                    Ok(mut update) => {
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
                        if matches!(
                            sender.try_send(Ok(snapshot.clone())),
                            Err(mpsc::TrySendError::Disconnected(_))
                        ) {
                            break;
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
