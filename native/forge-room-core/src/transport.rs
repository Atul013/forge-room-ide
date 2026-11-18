//! Ephemeral, invite-only TLS Room. Local RoomState is never uploaded; saved patches require explicit recipient grants.
use rustls::{
    pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName},
    ClientConfig, ServerConfig,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

mod readiness;
mod recovery;
mod shipping;
pub use shipping::{
    ship_blockers, ship_handoff, ShipCheck, ShipPlan, ShipReport, ShipResult, ShipState,
};
mod workflow;
pub use readiness::{
    assess, Assessment, Declaration, HelpRequest, Impact, ReadinessProfile, ReadinessState,
    TaskReadiness,
};
pub use workflow::{
    preview_plan, Action, CodeLocation, CodeReference, Countdown, Milestone, PlannedTask,
    SharedSnapshot, TaskFocus, TaskPlan,
};

const VERSION: u32 = 3;
// Includes escaped text for bounded shared tasks, messages and eight private patches.
const MAX_FRAME: usize = 8 * 1024 * 1024;
const MAX_MEMBERS: usize = 32;
const MAX_EVENTS: usize = 128;
const MAX_REACTIONS_PER_EVENT: usize = 32;
const MAX_MENTIONS: usize = 8;
const REACTION_EMOJI: [&str; 5] = ["👍", "✅", "👀", "🎉", "❤️"];
const LEASE: Duration = Duration::from_secs(10);
const SESSION: Duration = Duration::from_secs(3600);
type Result<T> = std::result::Result<T, String>;

// Secrets intentionally do not implement Debug and are never persisted.
#[derive(Clone, Serialize, Deserialize)]
pub struct Invite {
    version: u32,
    address: SocketAddr,
    certificate: Vec<u8>,
    room: String,
    token: String,
}
impl Invite {
    pub fn encode(&self) -> String {
        serde_json::to_string(self).expect("invite serialization")
    }
    pub fn decode(text: &str) -> Result<Self> {
        if text.len() > 16_384 {
            return Err("Invite is too large".into());
        }
        let invite: Self = serde_json::from_str(text).map_err(|_| "Invalid invite")?;
        bounded(&invite.room, 80)?;
        if invite.version != VERSION
            || invite.certificate.len() > 8192
            || invite.token.len() != 64
            || invite.address.port() == 0
            || invite.address.ip().is_unspecified()
            || invite.address.ip().is_multicast()
        {
            return Err("Unsupported invite".into());
        }
        Ok(invite)
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn room_name(&self) -> &str {
        &self.room
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub id: String,
    pub name: String,
    pub online: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub sequence: u64,
    pub actor: String,
    pub text: String,
    pub recipient: Option<String>,
    pub task: Option<String>,
    pub decision: bool,
    pub parent: Option<u64>,
    #[serde(default)]
    pub mentions: Vec<String>,
    /// Optional user-attached workspace-relative code location; message visibility still applies.
    #[serde(default)]
    pub code_reference: Option<CodeReference>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageReaction {
    pub sequence: u64,
    pub actor: String,
    pub emoji: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Patch {
    pub id: u64,
    pub actor: String,
    pub recipient: String,
    pub path: String,
    pub before: String,
    pub after: String,
    #[serde(default)]
    pub review: PatchReview,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchReview {
    #[default]
    Pending,
    Reviewed,
    Declined,
}
/// Best-effort credential/path filter; sharing still requires explicit human review.
pub fn validate_patch(path: &str, before: &str, after: &str) -> Result<()> {
    if path.len() > 512
        || path.contains(['\\', ':'])
        || path.chars().any(char::is_control)
        || path.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || [".git", ".ssh", "id_rsa", "id_ed25519"]
                    .contains(&part.to_ascii_lowercase().as_str())
                || part.to_ascii_lowercase().starts_with(".env")
                || [".key", ".pem", ".pfx", ".p12"]
                    .iter()
                    .any(|ext| part.to_ascii_lowercase().ends_with(ext))
        })
        || before.len() > 64 * 1024
        || after.len() > 128 * 1024
        || before == after
        || before.contains('\0')
        || after.contains('\0')
    {
        return Err(
            "Patch path or content is unsupported, sensitive, oversized or unchanged".into(),
        );
    }
    for text in [before, after] {
        if text
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
        {
            return Err("Patch contains unsupported control characters".into());
        }
        if text.contains("PRIVATE KEY-----") {
            return Err("Private key material cannot be shared".into());
        }
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=').or_else(|| line.split_once(':')) else {
                continue;
            };
            let key = key.to_ascii_lowercase();
            let value = value.trim().trim_end_matches([',', ';']);
            if [
                "api_key",
                "apikey",
                "api-key",
                "password",
                "secret",
                "access_token",
                "private_key",
            ]
            .iter()
            .any(|field| key.contains(field))
                && value.len() > 10
                && value.starts_with(['\"', '\''])
            {
                return Err("Possible credential assignment; remove it before sharing".into());
            }
        }
    }
    Ok(())
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub room: String,
    pub you: String,
    pub shared: SharedSnapshot,
    pub patches: Vec<Patch>,
    pub server_time_epoch_seconds: u64,
    pub members: Vec<Member>,
    pub events: Vec<Event>,
    pub cursor: u64,
    /// The requested cursor predates retained history. Events are a replacement window.
    pub reset: bool,
    /// Host controlled lifecycle; defaults to Active for older protocol-v3 snapshots.
    #[serde(default)]
    pub lifecycle: RoomLifecycle,
    /// Reactions are scoped to visible retained events; absent on older v3 peers.
    #[serde(default)]
    pub reactions: Vec<MessageReaction>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomLifecycle {
    #[default]
    Active,
    Paused,
    Archived,
}
#[derive(Serialize, Deserialize)]
enum Request {
    Join {
        version: u32,
        invite: String,
        name: String,
    },
    Poll {
        session: String,
        cursor: u64,
    },
    Post {
        session: String,
        operation: u64,
        text: String,
        cursor: u64,
    },
    Execute {
        session: String,
        operation: u64,
        action: Action,
        cursor: u64,
    },
    Leave {
        session: String,
    },
}
#[derive(Serialize, Deserialize)]
struct Response {
    rejection: Option<String>,
    error: Option<String>,
    session: Option<String>,
    identity: Option<String>,
    snapshot: Option<Snapshot>,
}
struct Grant {
    expires: Instant,
    used: bool,
    revoked: bool,
    reclaim: Option<Member>,
}
struct Session {
    id: String,
    name: String,
    invite: String,
    seen: Instant,
    expires: Instant,
    operation: u64,
    last_action: Option<Action>,
    last_rejection: Option<String>,
}
struct MessageDraft {
    text: String,
    recipient: Option<String>,
    task: Option<String>,
    decision: bool,
    parent: Option<u64>,
    mentions: Vec<String>,
    code_reference: Option<CodeReference>,
}
struct Authority {
    room: String,
    lifecycle: RoomLifecycle,
    grants: HashMap<String, Grant>,
    sessions: HashMap<String, Session>,
    events: VecDeque<Event>,
    sequence: u64,
    shared: workflow::SharedRoom,
    patches: Vec<Patch>,
    reactions: Vec<MessageReaction>,
    patch_sequence: u64,
    recovered: HashMap<String, Member>,
    journal: Option<recovery::Journal>,
    journal_required: bool,
    storage_error: Option<String>,
}
impl Authority {
    fn snapshot(&self, cursor: u64, viewer: Option<&str>) -> Snapshot {
        let now = Instant::now();
        let mut members: Vec<_> = self
            .sessions
            .values()
            .filter(|s| s.expires > now)
            .map(|s| Member {
                id: s.id.clone(),
                name: s.name.clone(),
                online: now.duration_since(s.seen) < LEASE,
            })
            .chain(self.recovered.values().cloned())
            .collect();
        members.sort_by(|a, b| a.id.cmp(&b.id));
        let reset = cursor > self.sequence
            || self
                .events
                .front()
                .is_some_and(|e| cursor.saturating_add(1) < e.sequence);
        let visible_event = |event: &&Event| {
            event.recipient.is_none()
                || viewer
                    .is_some_and(|id| event.actor == id || event.recipient.as_deref() == Some(id))
        };
        Snapshot {
            room: self.room.clone(),
            you: viewer.unwrap_or_default().into(),
            shared: self.shared.snapshot(),
            patches: self
                .patches
                .iter()
                .filter(|patch| viewer.is_some_and(|id| patch.actor == id || patch.recipient == id))
                .cloned()
                .collect(),
            server_time_epoch_seconds: crate::now_epoch_seconds(),
            members,
            events: self
                .events
                .iter()
                .filter(|e| {
                    (reset || e.sequence > cursor)
                        && (e.recipient.is_none()
                            || viewer.is_some_and(|id| {
                                e.actor == id || e.recipient.as_deref() == Some(id)
                            }))
                })
                .cloned()
                .collect(),
            cursor: self.sequence,
            reset,
            lifecycle: self.lifecycle,
            reactions: self
                .reactions
                .iter()
                .filter(|reaction| {
                    self.events
                        .iter()
                        .find(|event| event.sequence == reaction.sequence)
                        .is_some_and(|event| visible_event(&event))
                })
                .cloned()
                .collect(),
        }
    }
    fn handle(&mut self, request: Request) -> Result<Response> {
        self.check_storage()?;
        if !matches!(&request, Request::Poll { .. }) {
            self.check_mutation()?;
        }
        let now = Instant::now();
        self.sessions.retain(|_, s| s.expires > now);
        self.sync_members();
        let mut credential = None;
        let mut identity = None;
        let mut rejection = None;
        let viewer;
        let cursor = match request {
            Request::Join {
                version,
                invite,
                name,
            } => {
                if self.lifecycle != RoomLifecycle::Active {
                    return Err(match self.lifecycle {
                        RoomLifecycle::Paused => "Room is paused; ask the host to resume it",
                        RoomLifecycle::Archived => "Room is archived and read-only",
                        RoomLifecycle::Active => unreachable!(),
                    }
                    .into());
                }
                if version != VERSION {
                    return Err("Protocol version mismatch".into());
                }
                let name = bounded(&name, 64)?;
                let grant = self.grants.get_mut(&invite).ok_or("Invite denied")?;
                if grant.revoked || grant.used || grant.expires <= now {
                    return Err("Invite expired, revoked or already used".into());
                }
                if self.sessions.len() + self.recovered.len() >= MAX_MEMBERS
                    && grant.reclaim.is_none()
                {
                    return Err("Room is full".into());
                }
                let token = secret()?;
                let reclaimed = grant.reclaim.clone();
                let id = match &reclaimed {
                    Some(member) => member.id.clone(),
                    None => secret()?,
                };
                if self.sessions.values().any(|session| session.id == id) {
                    return Err("Member already rejoined".into());
                }
                let name = reclaimed
                    .as_ref()
                    .map_or(name, |member| member.name.clone());
                self.recovered.remove(&id);
                grant.used = true;
                identity = Some(id.clone());
                viewer = id.clone();
                credential = Some(token.clone());
                self.sessions.insert(
                    token,
                    Session {
                        id,
                        name,
                        invite,
                        seen: now,
                        expires: now + SESSION,
                        operation: 0,
                        last_action: None,
                        last_rejection: None,
                    },
                );
                self.sync_members();
                self.shared.audit(&viewer, "joined the network Room");
                0
            }
            Request::Poll { session, cursor } => {
                viewer = self.authenticate(&session, now)?.id.clone();
                cursor
            }
            Request::Post {
                session,
                operation,
                text,
                cursor,
            } => {
                let action = Action::message(&text);
                viewer = self.authenticate(&session, now)?.id.clone();
                rejection = self.execute(&session, operation, action)?;
                cursor
            }
            Request::Execute {
                session,
                operation,
                action,
                cursor,
            } => {
                viewer = self.authenticate(&session, now)?.id.clone();
                rejection = self.execute(&session, operation, action)?;
                cursor
            }
            Request::Leave { session } => {
                self.authenticate(&session, now)?;
                let actor = self
                    .sessions
                    .remove(&session)
                    .expect("authenticated session")
                    .id;
                self.sync_members();
                self.shared.audit(&actor, "left the network Room");
                return Ok(Response {
                    rejection: None,
                    error: None,
                    session: None,
                    identity: None,
                    snapshot: None,
                });
            }
        };
        Ok(Response {
            rejection,
            error: None,
            session: credential,
            identity,
            snapshot: Some(self.snapshot(cursor, Some(&viewer))),
        })
    }
    fn sync_members(&mut self) {
        self.patches.retain(|patch| {
            self.sessions.values().any(|s| s.id == patch.actor)
                && self.sessions.values().any(|s| s.id == patch.recipient)
        });
        self.shared.sync_members(
            self.sessions
                .values()
                .map(|s| crate::Participant {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    role: crate::Role::Human,
                    owner_id: None,
                })
                .chain(self.recovered.values().map(|member| crate::Participant {
                    id: member.id.clone(),
                    name: member.name.clone(),
                    role: crate::Role::Human,
                    owner_id: None,
                }))
                .collect(),
        );
        let joined: HashSet<_> = self
            .sessions
            .values()
            .map(|session| session.id.as_str())
            .collect();
        self.reactions
            .retain(|reaction| joined.contains(reaction.actor.as_str()));
    }
    fn execute(&mut self, token: &str, operation: u64, action: Action) -> Result<Option<String>> {
        let session = self.sessions.get(token).ok_or("Session denied")?;
        if operation == session.operation && session.last_action.as_ref() == Some(&action) {
            return Ok(session.last_rejection.clone());
        }
        if operation == 0 || operation != session.operation.saturating_add(1) {
            return Err("Invalid operation sequence or changed retry".into());
        }
        let actor = session.id.clone();
        let rejection = if self.lifecycle != RoomLifecycle::Active {
            Some(match self.lifecycle {
                RoomLifecycle::Paused => "Room is paused by the host".to_owned(),
                RoomLifecycle::Archived => "Room is archived and read-only".to_owned(),
                RoomLifecycle::Active => unreachable!(),
            })
        } else {
            match &action {
                Action::SharePatch {
                    recipient,
                    path,
                    before,
                    after,
                } => self
                    .share_patch(&actor, recipient, path, before, after)
                    .err(),
                Action::RevokePatch { id } => self.revoke_patch(&actor, *id, false).err(),
                Action::ReviewPatch { id, decision } => {
                    self.review_patch(&actor, *id, *decision).err()
                }
                Action::Message {
                    text,
                    recipient,
                    task,
                    decision,
                    parent,
                } => self
                    .message(
                        &actor,
                        MessageDraft {
                            text: text.clone(),
                            recipient: recipient.clone(),
                            task: task.clone(),
                            decision: *decision,
                            parent: *parent,
                            mentions: Vec::new(),
                            code_reference: None,
                        },
                    )
                    .err(),
                Action::ContextMessage {
                    text,
                    recipient,
                    task,
                    decision,
                    parent,
                    path,
                    line,
                } => match workflow::validate_code_location(path, *line) {
                    Ok(path) => self
                        .message(
                            &actor,
                            MessageDraft {
                                text: text.clone(),
                                recipient: recipient.clone(),
                                task: task.clone(),
                                decision: *decision,
                                parent: *parent,
                                mentions: Vec::new(),
                                code_reference: Some(CodeReference { path, line: *line }),
                            },
                        )
                        .err(),
                    Err(error) => Some(error),
                },
                Action::MentionMessage {
                    text,
                    mentions,
                    task,
                    decision,
                    parent,
                } => self
                    .message(
                        &actor,
                        MessageDraft {
                            text: text.clone(),
                            recipient: None,
                            task: task.clone(),
                            decision: *decision,
                            parent: *parent,
                            mentions: mentions.clone(),
                            code_reference: None,
                        },
                    )
                    .err(),
                Action::React {
                    sequence,
                    emoji,
                    active,
                } => self.react(&actor, *sequence, emoji, *active).err(),
                _ => self.shared.apply(&actor, action.clone()).err(),
            }
        };
        let session = self
            .sessions
            .get_mut(token)
            .expect("session held under authority lock");
        session.operation = operation;
        session.last_action = Some(action);
        session.last_rejection = rejection.clone();
        Ok(rejection)
    }
    fn message(&mut self, actor: &str, draft: MessageDraft) -> Result<()> {
        let MessageDraft {
            text,
            recipient,
            task,
            decision,
            parent,
            mentions,
            code_reference,
        } = draft;
        let text = bounded(&text, 2000)?;
        if recipient
            .as_ref()
            .is_some_and(|id| !self.sessions.values().any(|s| &s.id == id))
        {
            return Err("Recipient is no longer joined".into());
        }
        if task
            .as_ref()
            .is_some_and(|id| !self.shared.state.tasks.iter().any(|t| &t.id == id))
        {
            return Err("Task is not a shared Room task".into());
        }
        if mentions.len() > MAX_MENTIONS || (recipient.is_some() && !mentions.is_empty()) {
            return Err("Mentions are limited to eight members in public messages".into());
        }
        let unique: HashSet<_> = mentions.iter().collect();
        if unique.len() != mentions.len()
            || mentions
                .iter()
                .any(|id| id == actor || !self.sessions.values().any(|session| &session.id == id))
        {
            return Err("Choose distinct, currently joined members other than yourself".into());
        }
        if let Some(parent) = parent {
            let event = self
                .events
                .iter()
                .find(|e| e.sequence == parent)
                .ok_or("Reply target is unavailable")?;
            let same_channel = match (&event.recipient, &recipient) {
                (None, None) => true,
                (Some(old), Some(new)) => {
                    (event.actor == actor && old == new) || (event.actor == *new && old == actor)
                }
                _ => false,
            };
            if !same_channel {
                return Err("Reply target is unavailable".into());
            }
        }
        self.sequence += 1;
        let public = recipient.is_none();
        self.events.push_back(Event {
            sequence: self.sequence,
            actor: actor.into(),
            text,
            recipient,
            task,
            decision,
            parent,
            mentions,
            code_reference,
        });
        if self.events.len() > MAX_EVENTS {
            if let Some(expired) = self.events.pop_front() {
                self.reactions
                    .retain(|reaction| reaction.sequence != expired.sequence);
            }
        }
        if public {
            self.shared.audit(
                actor,
                if decision {
                    "recorded a public Room decision"
                } else {
                    "posted a public Room message"
                },
            );
        }
        Ok(())
    }
    fn react(&mut self, actor: &str, sequence: u64, emoji: &str, active: bool) -> Result<()> {
        if !REACTION_EMOJI.contains(&emoji) {
            return Err("Choose one of the supported Room reactions".into());
        }
        let event = self
            .events
            .iter()
            .find(|event| event.sequence == sequence)
            .ok_or("Message is no longer retained")?;
        if event
            .recipient
            .as_ref()
            .is_some_and(|recipient| event.actor != actor && recipient != actor)
        {
            return Err("Message is not visible to this member".into());
        }
        let existing = self.reactions.iter().position(|reaction| {
            reaction.sequence == sequence && reaction.actor == actor && reaction.emoji == emoji
        });
        if active && existing.is_none() {
            if self
                .reactions
                .iter()
                .filter(|reaction| reaction.sequence == sequence)
                .count()
                >= MAX_REACTIONS_PER_EVENT
            {
                return Err("Message reaction limit reached".into());
            }
            self.reactions.push(MessageReaction {
                sequence,
                actor: actor.into(),
                emoji: emoji.into(),
            });
        } else if !active {
            if let Some(index) = existing {
                self.reactions.remove(index);
            }
        }
        Ok(())
    }
    fn share_patch(
        &mut self,
        actor: &str,
        recipient: &str,
        path: &str,
        before: &str,
        after: &str,
    ) -> Result<()> {
        validate_patch(path, before, after)?;
        if self.patches.len() >= 8 {
            return Err("Room patch limit reached; revoke an old patch first".into());
        }
        if recipient == actor
            || !self
                .sessions
                .values()
                .any(|session| session.id == recipient)
        {
            return Err("Choose another joined recipient".into());
        }
        self.patch_sequence += 1;
        self.patches.push(Patch {
            id: self.patch_sequence,
            actor: actor.into(),
            recipient: recipient.into(),
            path: path.into(),
            before: before.into(),
            after: after.into(),
            review: PatchReview::Pending,
        });
        Ok(())
    }
    fn review_patch(&mut self, actor: &str, id: u64, decision: PatchReview) -> Result<()> {
        if decision == PatchReview::Pending {
            return Err("A review receipt must be Reviewed or Declined".into());
        }
        let patch = self
            .patches
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or("Patch is unavailable")?;
        if patch.recipient != actor {
            return Err("Only the selected recipient can record a patch review".into());
        }
        patch.review = decision;
        Ok(())
    }
    fn revoke_patch(&mut self, actor: &str, id: u64, host: bool) -> Result<()> {
        let index = self
            .patches
            .iter()
            .position(|p| p.id == id)
            .ok_or("Patch is unavailable")?;
        if !host && self.patches[index].actor != actor {
            return Err("Only the sender or host can revoke this patch".into());
        }
        self.patches.remove(index);
        Ok(())
    }
    fn authenticate(&mut self, token: &str, now: Instant) -> Result<&mut Session> {
        let session = self
            .sessions
            .get_mut(token)
            .ok_or("Session denied; request a new invite")?;
        let grant = self.grants.get(&session.invite).ok_or("Session revoked")?;
        if grant.revoked || session.expires <= now {
            return Err("Session expired or revoked".into());
        }
        session.seen = now;
        session.expires = now + SESSION;
        Ok(session)
    }
}

pub struct Host {
    address: SocketAddr,
    certificate: Vec<u8>,
    authority: Arc<Mutex<Authority>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Host {
    /// `bind` is explicit; use 127.0.0.1:0 for same-machine trials or a specific LAN IP.
    pub fn start(bind: SocketAddr, room: &str) -> Result<Self> {
        let room = bounded(room, 80)?;
        if bind.ip().is_unspecified() || bind.ip().is_multicast() {
            return Err("Choose a specific loopback or LAN address".into());
        }
        let certified = rcgen::generate_simple_self_signed(vec!["forge-room.local".into()])
            .map_err(|e| e.to_string())?;
        let certificate = certified.cert.der().to_vec();
        let config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|e| e.to_string())?
                .with_no_client_auth()
                .with_single_cert(
                    vec![CertificateDer::from(certificate.clone())],
                    PrivatePkcs8KeyDer::from(certified.key_pair.serialize_der()).into(),
                )
                .map_err(|e| e.to_string())?;
        let config = Arc::new(config);
        let listener = TcpListener::bind(bind).map_err(|e| e.to_string())?;
        let address = listener.local_addr().map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let authority = Arc::new(Mutex::new(Authority {
            room,
            lifecycle: RoomLifecycle::Active,
            grants: HashMap::new(),
            sessions: HashMap::new(),
            events: VecDeque::new(),
            sequence: 0,
            shared: workflow::SharedRoom::new()?,
            patches: Vec::new(),
            reactions: Vec::new(),
            journal: None,
            journal_required: false,
            storage_error: None,
            recovered: HashMap::new(),
            patch_sequence: 0,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (state, stopping) = (authority.clone(), stop.clone());
        let worker = thread::spawn(move || {
            let active = Arc::new(AtomicUsize::new(0));
            let mut workers = Vec::new();
            while !stopping.load(Ordering::Relaxed) {
                workers.retain(|w: &thread::JoinHandle<()>| !w.is_finished());
                match listener.accept() {
                    Ok((socket, _)) => {
                        if active.load(Ordering::Relaxed) >= 16 {
                            continue;
                        }
                        active.fetch_add(1, Ordering::Relaxed);
                        let (state, config, active) =
                            (state.clone(), config.clone(), active.clone());
                        workers.push(thread::spawn(move || {
                            let _ = serve(socket, config, state);
                            active.fetch_sub(1, Ordering::Relaxed);
                        }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
            for worker in workers {
                let _ = worker.join();
            }
        });
        Ok(Self {
            address,
            certificate,
            authority,
            stop,
            worker: Some(worker),
        })
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn invite(&self, lifetime: Duration) -> Result<Invite> {
        if lifetime.is_zero() || lifetime > Duration::from_secs(3600) {
            return Err("Invite lifetime must be 1 second to 1 hour".into());
        }
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_storage()?;
        match state.lifecycle {
            RoomLifecycle::Active => {}
            RoomLifecycle::Paused => {
                return Err("Room is paused; resume it before issuing invites".into())
            }
            RoomLifecycle::Archived => {
                return Err("Room is archived; no new invites can be issued".into())
            }
        }
        let now = Instant::now();
        let active: Vec<_> = state
            .sessions
            .values()
            .filter(|s| s.expires > now)
            .map(|s| s.invite.clone())
            .collect();
        state
            .grants
            .retain(|token, g| !g.revoked && (g.expires > now || active.contains(token)));
        if state.grants.len() >= 128 {
            return Err("Invite limit reached".into());
        }
        let token = secret()?;
        state.grants.insert(
            token.clone(),
            Grant {
                expires: now + lifetime,
                used: false,
                revoked: false,
                reclaim: None,
            },
        );
        Ok(Invite {
            version: VERSION,
            address: self.address,
            certificate: self.certificate.clone(),
            room: state.room.clone(),
            token,
        })
    }
    /// Revocation also ejects sessions created with this invite, including offline clients.
    pub fn revoke(&self, invite: &Invite) -> Result<()> {
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_mutation()?;
        if let Some(grant) = state.grants.get_mut(&invite.token) {
            grant.revoked = true;
        }
        state.sessions.retain(|_, s| s.invite != invite.token);
        state.sync_members();
        let host = state.shared.state.host_id.clone();
        state
            .shared
            .audit(&host, "revoked an invite and its joined session");
        state.persist()
    }
    pub fn lifecycle(&self) -> Result<RoomLifecycle> {
        Ok(self
            .authority
            .lock()
            .map_err(|_| "Room unavailable")?
            .lifecycle)
    }
    pub fn pause(&self) -> Result<()> {
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_mutation()?;
        if state.lifecycle != RoomLifecycle::Active {
            return Err("Only an active Room can be paused".into());
        }
        state.lifecycle = RoomLifecycle::Paused;
        let host = state.shared.state.host_id.clone();
        state.shared.audit(&host, "paused the network Room");
        state.persist()
    }
    pub fn resume(&self) -> Result<()> {
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_mutation()?;
        if state.journal_required && state.journal.is_none() {
            return Err("Start a fresh public journal before resuming a recovered journal".into());
        }
        if state.lifecycle != RoomLifecycle::Paused {
            return Err("Only a paused Room can be resumed".into());
        }
        state.lifecycle = RoomLifecycle::Active;
        let host = state.shared.state.host_id.clone();
        state.shared.audit(&host, "resumed the network Room");
        state.persist()
    }
    pub fn archive(&self) -> Result<()> {
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_mutation()?;
        if state.lifecycle == RoomLifecycle::Archived {
            return Err("Room is already archived".into());
        }
        state.lifecycle = RoomLifecycle::Archived;
        let host = state.shared.state.host_id.clone();
        state
            .shared
            .audit(&host, "archived the network Room as read-only");
        state.persist()
    }
    /// Host-local authority only. No serialized request can select the host actor.
    pub fn control(&self, action: Action) -> Result<()> {
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_mutation()?;
        let now = Instant::now();
        state.sessions.retain(|_, s| s.expires > now);
        state.sync_members();
        if state.lifecycle != RoomLifecycle::Active {
            return Err("Host edits are unavailable while the Room is paused or archived".into());
        }
        let actor = state.shared.state.host_id.clone();
        if let Action::RevokePatch { id } = action {
            return state.revoke_patch(&actor, id, true);
        }
        if let Action::ReviewPatch { id, decision } = action {
            return state.review_patch(&actor, id, decision);
        }
        state.shared.apply(&actor, action)?;
        state.persist()
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_storage()?;
        Ok(state.snapshot(0, None))
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub struct Outcome {
    pub snapshot: Snapshot,
    pub rejection: Option<String>,
}

pub struct Client {
    invite: Invite,
    config: Arc<ClientConfig>,
    session: String,
    pub identity: String,
    cursor: u64,
    operation: u64,
    pending: Option<Action>,
}
impl Client {
    pub fn join(invite: Invite, name: &str) -> Result<(Self, Snapshot)> {
        if invite.version != VERSION {
            return Err("Protocol version mismatch".into());
        }
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(CertificateDer::from(invite.certificate.clone()))
            .map_err(|_| "Invalid host certificate")?;
        let config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|e| e.to_string())?
                .with_root_certificates(roots)
                .with_no_client_auth();
        let mut client = Self {
            invite,
            config: Arc::new(config),
            session: String::new(),
            identity: String::new(),
            cursor: 0,
            operation: 0,
            pending: None,
        };
        let response = client.exchange(Request::Join {
            version: VERSION,
            invite: client.invite.token.clone(),
            name: bounded(name, 64)?,
        })?;
        client.session = response.session.ok_or("Missing session")?;
        client.identity = response.identity.ok_or("Missing identity")?;
        let snapshot = client.accept(response.snapshot)?;
        Ok((client, snapshot))
    }
    /// Each request opens a fresh TLS connection. A failed connection retains identity/cursor.
    pub fn poll(&mut self) -> Result<Snapshot> {
        let response = self.exchange(Request::Poll {
            session: self.session.clone(),
            cursor: self.cursor,
        })?;
        self.accept(response.snapshot)
    }
    /// Acknowledged rejections consume an operation; interrupted delivery retains it for retry.
    pub fn execute(&mut self, action: Action) -> Result<Outcome> {
        if self.pending.as_ref().is_some_and(|p| p != &action) {
            return Err("Retry the pending action before sending another".into());
        }
        self.pending = Some(action.clone());
        let response = self.exchange(Request::Execute {
            session: self.session.clone(),
            operation: self.operation + 1,
            action,
            cursor: self.cursor,
        })?;
        let snapshot = self.accept(response.snapshot)?;
        self.operation += 1;
        self.pending = None;
        Ok(Outcome {
            snapshot,
            rejection: response.rejection,
        })
    }
    pub fn post(&mut self, text: &str) -> Result<Snapshot> {
        let text = bounded(text, 2000)?;
        let outcome = self.execute(Action::message(&text))?;
        match outcome.rejection {
            Some(error) => Err(error),
            None => Ok(outcome.snapshot),
        }
    }
    pub fn leave(&mut self) -> Result<()> {
        self.exchange(Request::Leave {
            session: self.session.clone(),
        })?;
        self.session.clear();
        Ok(())
    }
    fn accept(&mut self, snapshot: Option<Snapshot>) -> Result<Snapshot> {
        let snapshot = snapshot.ok_or("Missing room state")?;
        if snapshot.members.len() > MAX_MEMBERS
            || snapshot.events.len() > MAX_EVENTS
            || bounded(&snapshot.room, 80).is_err()
            || snapshot
                .members
                .iter()
                .any(|m| m.id.len() != 64 || bounded(&m.name, 64).is_err())
            || snapshot.events.iter().any(|e| {
                e.actor.len() != 64
                    || bounded(&e.text, 2000).is_err()
                    || e.sequence > snapshot.cursor
                    || e.mentions.len() > MAX_MENTIONS
                    || (e.recipient.is_some() && !e.mentions.is_empty())
                    || e.mentions.iter().any(|id| id.len() != 64)
                    || e.mentions.iter().collect::<HashSet<_>>().len() != e.mentions.len()
                    || e.code_reference.as_ref().is_some_and(|reference| {
                        workflow::validate_code_location(&reference.path, reference.line).is_err()
                    })
                    || e.recipient.as_ref().is_some_and(|recipient| {
                        recipient.len() != 64
                            || (e.actor != self.identity && recipient != &self.identity)
                    })
                    || e.task.as_ref().is_some_and(|task| task.len() > 16)
                    || e.parent.is_some_and(|parent| parent >= e.sequence)
            })
            || snapshot
                .events
                .windows(2)
                .any(|pair| pair[0].sequence >= pair[1].sequence)
            || snapshot.reactions.len() > MAX_EVENTS * MAX_REACTIONS_PER_EVENT
            || {
                let mut seen = HashSet::new();
                let mut counts = HashMap::new();
                snapshot.reactions.iter().any(|reaction| {
                    let count = counts.entry(reaction.sequence).or_insert(0usize);
                    *count += 1;
                    reaction.sequence == 0
                        || reaction.sequence > snapshot.cursor
                        || reaction.actor.len() != 64
                        || !snapshot
                            .members
                            .iter()
                            .any(|member| member.id == reaction.actor)
                        || !REACTION_EMOJI.contains(&reaction.emoji.as_str())
                        || *count > MAX_REACTIONS_PER_EVENT
                        || !seen.insert((
                            reaction.sequence,
                            reaction.actor.as_str(),
                            reaction.emoji.as_str(),
                        ))
                })
            }
        {
            return Err("Invalid room snapshot".into());
        }
        workflow::validate(&snapshot.shared)?;
        let focus_actors: std::collections::HashSet<_> = snapshot
            .shared
            .task_focus
            .iter()
            .map(|focus| focus.actor.as_str())
            .collect();
        if focus_actors.len() != snapshot.shared.task_focus.len()
            || snapshot.shared.task_focus.iter().any(|focus| {
                !snapshot
                    .members
                    .iter()
                    .any(|member| member.id == focus.actor)
            })
        {
            return Err("Task focus belongs to a member who is no longer in the Room".into());
        }
        let location_actors: std::collections::HashSet<_> = snapshot
            .shared
            .code_locations
            .iter()
            .map(|location| location.actor.as_str())
            .collect();
        if location_actors.len() != snapshot.shared.code_locations.len()
            || snapshot.shared.code_locations.iter().any(|location| {
                !snapshot
                    .members
                    .iter()
                    .any(|member| member.id == location.actor)
            })
        {
            return Err("Code location belongs to a member who is no longer in the Room".into());
        }
        if snapshot.patches.len() > 8
            || snapshot.patches.iter().any(|p| {
                p.actor.len() != 64
                    || p.recipient.len() != 64
                    || (p.actor != self.identity && p.recipient != self.identity)
                    || validate_patch(&p.path, &p.before, &p.after).is_err()
            })
        {
            return Err("Invalid shared patch state".into());
        }
        if snapshot.you.len() != 64 || snapshot.you != self.identity {
            return Err("Invalid session identity".into());
        }
        self.cursor = snapshot.cursor;
        Ok(snapshot)
    }
    fn exchange(&self, request: Request) -> Result<Response> {
        let socket = TcpStream::connect_timeout(&self.invite.address, Duration::from_secs(2))
            .map_err(|_| "Host unreachable; reconnecting")?;
        timeouts(&socket)?;
        let connection = rustls::ClientConnection::new(
            self.config.clone(),
            ServerName::try_from("forge-room.local").expect("fixed server name"),
        )
        .map_err(|e| e.to_string())?;
        let mut stream = rustls::StreamOwned::new(connection, socket);
        write_frame(&mut stream, &request)?;
        let response: Response = read_frame(&mut stream)?;
        if let Some(error) = response.error {
            return Err(error);
        }
        Ok(response)
    }
}
fn timeouts(socket: &TcpStream) -> Result<()> {
    // Winsock accepts can inherit the listener's nonblocking mode.
    socket.set_nonblocking(false).map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())
}
fn serve(socket: TcpStream, config: Arc<ServerConfig>, state: Arc<Mutex<Authority>>) -> Result<()> {
    timeouts(&socket)?;
    let connection = rustls::ServerConnection::new(config).map_err(|e| e.to_string())?;
    let mut stream = rustls::StreamOwned::new(connection, socket);
    let request: Request = read_frame(&mut stream)?;
    let mutating = !matches!(&request, Request::Poll { .. });
    let response = (|| {
        let mut authority = state.lock().map_err(|_| "Room unavailable")?;
        let response = authority.handle(request)?;
        if mutating {
            authority.persist()?;
        }
        Ok::<_, String>(response)
    })()
    .unwrap_or_else(|error| Response {
        rejection: None,
        error: Some(error),
        session: None,
        identity: None,
        snapshot: None,
    });
    write_frame(&mut stream, &response)
}
fn write_frame(stream: &mut impl Write, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_FRAME {
        return Err("Frame too large".into());
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .and_then(|_| stream.write_all(&bytes))
        .and_then(|_| stream.flush())
        .map_err(|e| format!("Connection interrupted: {e}"))
}
fn read_frame<T: for<'de> Deserialize<'de>>(stream: &mut impl Read) -> Result<T> {
    let mut size = [0; 4];
    stream
        .read_exact(&mut size)
        .map_err(|_| "Connection interrupted; retry")?;
    let size = u32::from_be_bytes(size) as usize;
    if size == 0 || size > MAX_FRAME {
        return Err("Frame too large".into());
    }
    let mut bytes = vec![0; size];
    stream
        .read_exact(&mut bytes)
        .map_err(|_| "Connection interrupted; retry")?;
    serde_json::from_slice(&bytes).map_err(|_| "Invalid protocol frame".into())
}
fn bounded(text: &str, max: usize) -> Result<String> {
    let text = text.trim();
    if text.is_empty()
        || text.chars().count() > max
        || text
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(format!("Enter 1–{max} characters"));
    }
    Ok(text.into())
}
fn secret() -> Result<String> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| "Secure randomness unavailable")?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_patch_snapshots_default_to_pending_review() {
        let patch: Patch = serde_json::from_value(serde_json::json!({
            "id": 1,
            "actor": "a",
            "recipient": "b",
            "path": "src/lib.rs",
            "before": "old",
            "after": "new"
        }))
        .unwrap();
        assert_eq!(patch.review, PatchReview::Pending);
    }
    fn host() -> Host {
        Host::start("127.0.0.1:0".parse().unwrap(), "Private trial").unwrap()
    }
    fn invite(host: &Host) -> Invite {
        host.invite(Duration::from_secs(60)).unwrap()
    }

    #[test]
    fn task_focus_field_is_additive_and_malformed_focus_is_rejected() {
        let mut old = serde_json::to_value(SharedSnapshot::default()).unwrap();
        old.as_object_mut().unwrap().remove("task_focus");
        let decoded: SharedSnapshot = serde_json::from_value(old).unwrap();
        assert!(decoded.task_focus.is_empty());

        let mut invalid = SharedSnapshot::default();
        invalid.task_focus.push(TaskFocus {
            actor: "a".repeat(64),
            task: "T404".into(),
        });
        assert!(workflow::validate(&invalid).is_err());
        invalid.task_focus.clear();
        invalid.task_focus.extend((0..33).map(|index| TaskFocus {
            actor: format!("{index:064x}"),
            task: "T1".into(),
        }));
        assert!(workflow::validate(&invalid).is_err());
    }

    #[test]
    fn code_location_field_is_backward_compatible_and_rejects_invalid_or_oversized_state() {
        let mut old = serde_json::to_value(SharedSnapshot::default()).unwrap();
        old.as_object_mut().unwrap().remove("code_locations");
        let decoded: SharedSnapshot = serde_json::from_value(old).unwrap();
        assert!(decoded.code_locations.is_empty());

        let mut invalid = SharedSnapshot::default();
        invalid.code_locations.push(CodeLocation {
            actor: "a".repeat(64),
            path: "../outside.rs".into(),
            line: 10,
        });
        assert!(workflow::validate(&invalid).is_err());
        invalid.code_locations.clear();
        invalid
            .code_locations
            .extend((0..33).map(|index| CodeLocation {
                actor: format!("{index:064x}"),
                path: "src/main.rs".into(),
                line: 1,
            }));
        assert!(workflow::validate(&invalid).is_err());
    }

    #[test]
    fn lifecycle_snapshot_field_defaults_for_older_protocol_three_peers() {
        let mut value = serde_json::to_value(Snapshot::default()).unwrap();
        value.as_object_mut().unwrap().remove("lifecycle");
        value.as_object_mut().unwrap().remove("reactions");
        let decoded: Snapshot = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.lifecycle, RoomLifecycle::Active);
        assert!(decoded.reactions.is_empty());
    }

    #[test]
    fn older_message_events_default_to_no_mentions_and_reactions_are_bounded() {
        let event = Event {
            sequence: 1,
            actor: "a".repeat(64),
            text: "hello".into(),
            recipient: None,
            task: None,
            decision: false,
            parent: None,
            mentions: Vec::new(),
            code_reference: None,
        };
        let mut value = serde_json::to_value(event).unwrap();
        value.as_object_mut().unwrap().remove("mentions");
        let decoded: Event = serde_json::from_value(value).unwrap();
        assert!(decoded.mentions.is_empty());
        assert!(decoded.code_reference.is_none());

        let mut authority = Authority {
            room: "trial".into(),
            lifecycle: RoomLifecycle::Active,
            grants: HashMap::new(),
            sessions: HashMap::new(),
            events: VecDeque::from([decoded]),
            sequence: 1,
            shared: workflow::SharedRoom::new().unwrap(),
            patches: Vec::new(),
            reactions: (0..MAX_REACTIONS_PER_EVENT)
                .map(|index| MessageReaction {
                    sequence: 1,
                    actor: format!("{index:064x}"),
                    emoji: REACTION_EMOJI[index % REACTION_EMOJI.len()].into(),
                })
                .collect(),
            journal: None,
            journal_required: false,
            storage_error: None,
            recovered: HashMap::new(),
            patch_sequence: 0,
        };
        assert!(authority
            .react(&"f".repeat(64), 1, "👍", true)
            .unwrap_err()
            .contains("limit"));
    }

    #[test]
    fn bounded_shared_payload_fits_the_transport_envelope() {
        let id = "a".repeat(64);
        let mut snapshot = Snapshot {
            room: "room".into(),
            you: id.clone(),
            ..Snapshot::default()
        };
        for index in 0..100 {
            snapshot.shared.readiness.profiles.push(TaskReadiness {
                task: format!("T{index}"),
                revision: 1,
                owner: id.clone(),
                updated: 0,
                profile: ReadinessProfile {
                    estimate_minutes: Some(10080),
                    blocker: "x".repeat(500),
                    files: (0..8).map(|i| format!("{i}{}", "x".repeat(500))).collect(),
                    ..Default::default()
                },
            });
            snapshot.shared.tasks.push(crate::Task {
                id: format!("T{index}"),
                title: "😀".repeat(200),
                owner_id: id.clone(),
                owner_name: "😀".repeat(64),
                status: crate::TaskStatus::Open,
                priority: crate::TaskPriority::Normal,
                dependencies: vec!["T1".into(); 16],
                acceptance_criteria: vec!["😀".repeat(200); 8],
                completed_criteria: vec![],
                worktree_branch: None,
                due_at_epoch_seconds: None,
                updated_at_epoch_seconds: 0,
            });
        }
        for index in 0..128 {
            snapshot.events.push(Event {
                sequence: index + 1,
                actor: id.clone(),
                text: "😀".repeat(2000),
                recipient: None,
                task: None,
                decision: false,
                parent: None,
                mentions: Vec::new(),
                code_reference: None,
            });
            snapshot.shared.evidence.push(crate::Evidence {
                id: index.to_string(),
                task_id: "T1".into(),
                actor_id: id.clone(),
                kind: crate::EvidenceKind::Check,
                summary: "😀".repeat(500),
                recorded_at_epoch_seconds: 0,
            });
        }
        for index in 0..8 {
            snapshot.patches.push(Patch {
                id: index,
                actor: id.clone(),
                recipient: "b".repeat(64),
                path: format!("file{index}.txt"),
                before: "\"".repeat(64 * 1024),
                after: "\\".repeat(128 * 1024),
                review: PatchReview::Pending,
            });
        }
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &snapshot).unwrap();
        let decoded: Snapshot = read_frame(&mut bytes.as_slice()).unwrap();
        assert_eq!(decoded.patches.len(), 8);
        assert_eq!(decoded.shared.tasks.len(), 100);
    }

    #[test]
    fn expiry_revocation_version_and_unknown_credentials_are_denied() {
        let host = host();
        let expired = invite(&host);
        host.authority
            .lock()
            .unwrap()
            .grants
            .get_mut(&expired.token)
            .unwrap()
            .expires = Instant::now();
        assert!(Client::join(expired, "Ada").is_err());
        let revoked = invite(&host);
        host.revoke(&revoked).unwrap();
        assert!(Client::join(revoked, "Ada").is_err());
        let mut invalid = invite(&host);
        invalid.token = "0".repeat(64);
        assert!(Client::join(invalid, "Ada").is_err());
        let mut version = invite(&host);
        version.version += 1;
        assert!(Invite::decode(&version.encode()).is_err());
        assert!(Client::join(version, "Ada").is_err());
        let valid = invite(&host);
        let (mut client, _) = Client::join(valid.clone(), "Ada").unwrap();
        client.session = "forged identity".into();
        assert!(client.poll().is_err());
        assert!(client.post("Denied").is_err());
        assert!(Client::join(valid, "Duplicate").is_err());
        assert!(host.snapshot().unwrap().events.is_empty());
    }

    #[test]
    fn certificate_pin_prevents_joining_a_different_host() {
        let first = host();
        let second = host();
        let mut wrong_host = invite(&first);
        wrong_host.address = second.address();
        assert!(Client::join(wrong_host, "Ada").is_err());
        assert!(second.snapshot().unwrap().members.is_empty());
    }

    #[test]
    fn reconnect_replays_without_duplicate_identity_and_lease_expires() {
        let host = host();
        let (mut a, _) = Client::join(invite(&host), "Ada").unwrap();
        let (mut b, _) = Client::join(invite(&host), "Grace").unwrap();
        let identity = a.identity.clone();
        let address = a.invite.address;
        // Reserve then close a port to force a real disconnected request.
        let unused = TcpListener::bind("127.0.0.1:0").unwrap();
        a.invite.address = unused.local_addr().unwrap();
        drop(unused);
        assert!(a.poll().is_err());
        b.post("while Ada was disconnected").unwrap();
        {
            let mut state = host.authority.lock().unwrap();
            state.sessions.get_mut(&a.session).unwrap().seen = Instant::now() - LEASE;
        }
        assert!(
            !host
                .snapshot()
                .unwrap()
                .members
                .iter()
                .find(|m| m.id == identity)
                .unwrap()
                .online
        );
        a.invite.address = address;
        let replay = a.poll().unwrap();
        assert_eq!(a.identity, identity);
        assert_eq!(replay.members.len(), 2);
        assert!(
            replay
                .members
                .iter()
                .find(|m| m.id == identity)
                .unwrap()
                .online
        );
        assert_eq!(replay.events.len(), 1);
        assert!(a.poll().unwrap().events.is_empty());
    }

    #[test]
    fn uncertain_delivery_is_idempotent_and_replay_gap_is_explicit() {
        let host = host();
        let (mut a, _) = Client::join(invite(&host), "Ada").unwrap();
        let (mut b, _) = Client::join(invite(&host), "Grace").unwrap();
        let request = || Request::Post {
            session: a.session.clone(),
            operation: 1,
            text: "once".into(),
            cursor: 0,
        };
        // Commit a request, deliberately discard its response, then retry it on a fresh socket.
        a.exchange(request()).unwrap();
        a.exchange(request()).unwrap();
        assert_eq!(host.snapshot().unwrap().events.len(), 1);
        a.operation = 1;
        for index in 0..MAX_EVENTS {
            a.post(&format!("event {index}")).unwrap();
        }
        let replay = b.poll().unwrap();
        assert!(replay.reset);
        assert_eq!(replay.events.len(), MAX_EVENTS);
        assert_eq!(replay.cursor, (MAX_EVENTS + 1) as u64);
        assert!(b.poll().unwrap().events.is_empty());
    }

    #[test]
    fn invite_and_revoke_races_have_one_authoritative_result() {
        let host = host();
        let shared = invite(&host);
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let mut workers = Vec::new();
        for name in ["Ada", "Grace"] {
            let (invite, barrier) = (shared.clone(), barrier.clone());
            workers.push(thread::spawn(move || {
                barrier.wait();
                Client::join(invite, name)
            }));
        }
        barrier.wait();
        let mut winners: Vec<_> = workers
            .into_iter()
            .filter_map(|w| w.join().unwrap().ok())
            .collect();
        assert_eq!(winners.len(), 1);
        let (mut client, _) = winners.pop().unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let posting = barrier.clone();
        let post = thread::spawn(move || {
            posting.wait();
            let committed = client.post("racing revocation").is_ok();
            (client, committed)
        });
        barrier.wait();
        host.revoke(&shared).unwrap();
        let (mut client, committed) = post.join().unwrap();
        assert_eq!(
            host.snapshot().unwrap().events.len(),
            usize::from(committed)
        );
        assert!(client.poll().is_err());
        assert!(client.post("revoked").is_err());
        assert!(host.snapshot().unwrap().members.is_empty());
    }

    #[test]
    fn invite_expiry_does_not_expire_joined_session_but_session_deadline_does() {
        let host = host();
        let grant = invite(&host);
        let (mut client, _) = Client::join(grant.clone(), "Ada").unwrap();
        host.authority
            .lock()
            .unwrap()
            .grants
            .get_mut(&grant.token)
            .unwrap()
            .expires = Instant::now();
        // Issuing another invite must not prune the grant backing an existing session.
        let _ = invite(&host);
        assert!(client.poll().is_ok());
        host.authority
            .lock()
            .unwrap()
            .sessions
            .get_mut(&client.session)
            .unwrap()
            .expires = Instant::now();
        assert!(client.poll().is_err());
        assert!(host.snapshot().unwrap().members.is_empty());
    }

    #[test]
    fn malformed_oversized_and_unauthorized_frames_never_reveal_state() {
        let oversized = ((MAX_FRAME + 1) as u32).to_be_bytes();
        assert!(read_frame::<Request>(&mut &oversized[..]).is_err());
        let mut malformed = vec![0, 0, 0, 1];
        malformed.push(b'{');
        assert!(read_frame::<Request>(&mut &malformed[..]).is_err());
        assert!(Invite::decode(&"x".repeat(16_385)).is_err());
        let host = host();
        let (mut client, _) = Client::join(invite(&host), "Ada").unwrap();
        assert!(client.post(&"x".repeat(2001)).is_err());
        assert!(client.post("  ").is_err());
        let denied = client.exchange(Request::Poll {
            session: "unknown".into(),
            cursor: 0,
        });
        assert!(denied.is_err());
        assert!(Host::start("0.0.0.0:0".parse().unwrap(), "bad bind").is_err());
    }
    #[test]
    fn active_requests_renew_the_idle_session_deadline() {
        let host = host();
        let (mut client, _) = Client::join(invite(&host), "Ada").unwrap();
        host.authority
            .lock()
            .unwrap()
            .sessions
            .get_mut(&client.session)
            .unwrap()
            .expires = Instant::now() + Duration::from_secs(30);
        client.poll().unwrap();
        assert!(
            host.authority.lock().unwrap().sessions[&client.session].expires
                > Instant::now() + Duration::from_secs(3500)
        );
    }
}
