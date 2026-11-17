//! Ephemeral, invite-only TLS room. No workspace paths or local RoomState cross this boundary.
use rustls::{
    pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName},
    ClientConfig, ServerConfig,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const VERSION: u32 = 1;
// 128 messages × 2000 Unicode characters plus roster and JSON framing fit below 2 MiB.
const MAX_FRAME: usize = 2 * 1024 * 1024;
const MAX_MEMBERS: usize = 32;
const MAX_EVENTS: usize = 128;
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
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub room: String,
    pub members: Vec<Member>,
    pub events: Vec<Event>,
    pub cursor: u64,
    /// The requested cursor predates retained history. Events are a replacement window.
    pub reset: bool,
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
    Leave {
        session: String,
    },
}
#[derive(Serialize, Deserialize)]
struct Response {
    error: Option<String>,
    session: Option<String>,
    identity: Option<String>,
    snapshot: Option<Snapshot>,
}
struct Grant {
    expires: Instant,
    used: bool,
    revoked: bool,
}
struct Session {
    id: String,
    name: String,
    invite: String,
    seen: Instant,
    expires: Instant,
    operation: u64,
}
struct Authority {
    room: String,
    grants: HashMap<String, Grant>,
    sessions: HashMap<String, Session>,
    events: VecDeque<Event>,
    sequence: u64,
}
impl Authority {
    fn snapshot(&self, cursor: u64) -> Snapshot {
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
            .collect();
        members.sort_by(|a, b| a.id.cmp(&b.id));
        let reset = cursor > self.sequence
            || self
                .events
                .front()
                .is_some_and(|e| cursor.saturating_add(1) < e.sequence);
        Snapshot {
            room: self.room.clone(),
            members,
            events: self
                .events
                .iter()
                .filter(|e| reset || e.sequence > cursor)
                .cloned()
                .collect(),
            cursor: self.sequence,
            reset,
        }
    }
    fn handle(&mut self, request: Request) -> Result<Response> {
        let now = Instant::now();
        self.sessions.retain(|_, s| s.expires > now);
        let mut credential = None;
        let mut identity = None;
        let cursor = match request {
            Request::Join {
                version,
                invite,
                name,
            } => {
                if version != VERSION {
                    return Err("Protocol version mismatch".into());
                }
                let name = bounded(&name, 64)?;
                if self.sessions.len() >= MAX_MEMBERS {
                    return Err("Room is full".into());
                }
                let grant = self.grants.get_mut(&invite).ok_or("Invite denied")?;
                if grant.revoked || grant.used || grant.expires <= now {
                    return Err("Invite expired, revoked or already used".into());
                }
                let token = secret()?;
                let id = secret()?;
                grant.used = true;
                identity = Some(id.clone());
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
                    },
                );
                0
            }
            Request::Poll { session, cursor } => {
                self.authenticate(&session, now)?;
                cursor
            }
            Request::Post {
                session,
                operation,
                text,
                cursor,
            } => {
                let text = bounded(&text, 2000)?;
                let session = self.authenticate(&session, now)?;
                if operation == 0 || operation > session.operation.saturating_add(1) {
                    return Err("Invalid operation sequence".into());
                }
                if operation == session.operation + 1 {
                    session.operation = operation;
                    let actor = session.id.clone();
                    self.sequence += 1;
                    self.events.push_back(Event {
                        sequence: self.sequence,
                        actor,
                        text,
                    });
                    if self.events.len() > MAX_EVENTS {
                        self.events.pop_front();
                    }
                }
                cursor
            }
            Request::Leave { session } => {
                self.authenticate(&session, now)?;
                self.sessions.remove(&session);
                return Ok(Response {
                    error: None,
                    session: None,
                    identity: None,
                    snapshot: None,
                });
            }
        };
        Ok(Response {
            error: None,
            session: credential,
            identity,
            snapshot: Some(self.snapshot(cursor)),
        })
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
            grants: HashMap::new(),
            sessions: HashMap::new(),
            events: VecDeque::new(),
            sequence: 0,
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
        if let Some(grant) = state.grants.get_mut(&invite.token) {
            grant.revoked = true;
        }
        state.sessions.retain(|_, s| s.invite != invite.token);
        Ok(())
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        Ok(self
            .authority
            .lock()
            .map_err(|_| "Room unavailable")?
            .snapshot(0))
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

pub struct Client {
    invite: Invite,
    config: Arc<ClientConfig>,
    session: String,
    pub identity: String,
    cursor: u64,
    operation: u64,
    pending: Option<String>,
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
    /// On uncertain delivery only the same text may be retried, with the same operation ID.
    pub fn post(&mut self, text: &str) -> Result<Snapshot> {
        let text = bounded(text, 2000)?;
        if self.pending.as_ref().is_some_and(|p| p != &text) {
            return Err("Retry the pending message before sending another".into());
        }
        self.pending = Some(text.clone());
        let response = self.exchange(Request::Post {
            session: self.session.clone(),
            operation: self.operation + 1,
            text,
            cursor: self.cursor,
        })?;
        let snapshot = self.accept(response.snapshot)?;
        self.operation += 1;
        self.pending = None;
        Ok(snapshot)
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
            })
            || snapshot
                .events
                .windows(2)
                .any(|pair| pair[0].sequence >= pair[1].sequence)
        {
            return Err("Invalid room snapshot".into());
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
    let request = read_frame(&mut stream)?;
    let response = state
        .lock()
        .map_err(|_| "Room unavailable")?
        .handle(request)
        .unwrap_or_else(|error| Response {
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
    fn host() -> Host {
        Host::start("127.0.0.1:0".parse().unwrap(), "Private trial").unwrap()
    }
    fn invite(host: &Host) -> Invite {
        host.invite(Duration::from_secs(60)).unwrap()
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
}
