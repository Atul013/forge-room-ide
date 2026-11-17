# Native Room transport v3 - connected Room workflows and saved patches

This is an ephemeral self-hosted native transport, implemented in `forge-room-core::transport`
and exposed by **Team / Chat → Network Room → host / join**. It is independent of the
existing saved local Room. This contract covers invite identities, presence, visibility-filtered messages, and shared
tasks/handoffs/evidence/activity/countdown. The server creates a separate shared domain RoomState;
it never imports the saved local Room or its participant selector.

## User journey and scope

1. Open a folder and expand Network Room. Inspect the sharing preview and display name.
2. Host binds `127.0.0.1:0` by default (same computer). For a LAN trial, enter the host's
   specific LAN IP and a port, e.g. `192.168.1.20:45678`. Binding all interfaces is refused.
   Hosting requires explicit action and does not change Windows Firewall settings.
3. Copy a single-use five-minute invite and send it privately. It contains the reachable
   endpoint, room name, host certificate and a 256-bit capability. Treat clipboard contents
   as private. The app does not save or log invites, session credentials or certificates.
4. Another native app pastes the invite, inspects room/endpoint and sharing scope, then
   chooses Join. A name, assigned identity, roster and items explicitly created in the network panel
   are shared. No project file, path, saved local message/task, agent, terminal or debug data
   is uploaded by host/join. Network tasks, handoffs, evidence, activity and the countdown
   are shared with all members. Direct messages are restricted to sender and recipient.
5. The host can revoke each issued invite, ejecting its joined session as well. Leave removes
   a session; it cannot reuse its consumed invite. End, workspace change and app exit end
   the hosted session. Clients need a fresh invite after a host restart or session expiry.
6. The host can pause/resume the Room or archive it as final read-only state. Existing members
   can still poll while paused/archived, but cannot mutate shared state; new joins and invites
   are refused. Archive cannot be resumed. The host can explicitly copy a public JSON archive
   for local records; it includes bounded public events and shared workflow state, and excludes
   direct messages and recipient-only patch bodies.

The host is pinned by the certificate carried in the invite. Membership authentication is
possession of an invite/session capability, not a verified human account. Display names
are labels; immutable random actor IDs distinguish identical names. Public network chat is visible to every authorized member, including recent history on join.
Direct messages and replies are filtered on the server before snapshots/history are serialized;
public host snapshots exclude DMs too. The trusted host process owns the in-memory authority,
so this is not end-to-end encryption against the host operator. There are no account roles,
agents, join approval queue, relay, discovery or hosted service here.

## Wire and authority contract

- TLS uses rustls with its ring provider; rcgen creates a fresh self-signed certificate for
  each Host. The client trusts only its invite certificate and verifies the fixed TLS name
  `forge-room.local`. No insecure verifier or operating-system trust-store bypass is used.
- Frames are four-byte big-endian lengths followed by UTF-8 serde JSON, bounded to 8 MiB
  before allocation. Each connection handles one request and one response. Up to 16
  connections run concurrently, with two-second socket read/write timeouts; clients use
  a two-second connect timeout. This preview has no public-service abuse protection.
- Request variants are `Join { version, invite, name }`, `Poll { session, cursor }`,
  `Post { session, operation, text, cursor }` and `Leave { session }`. Version is 3. Version-1/2 invites are rejected; sessions are ephemeral and require a fresh invite after upgrade.
  A new `Execute { session, operation, action, cursor }` request carries typed workflow actions.
  Responses contain an optional error or snapshot; only a successful Join returns a
  session credential and assigned identity. Authentication/transport errors return no snapshot or credential. Domain rejections
  return an acknowledged `rejection` with a filtered snapshot; this consumes the operation
  without applying it, so a rejected action does not wedge the next operation.
- The server alone issues 256-bit invite, session and actor IDs using OS randomness.
  A client supplies no actor ID to Post; the server resolves attribution from its session.
  No network request can mint invites, revoke members, select a host actor, approve a handoff,
  reopen completed work or control the countdown. Host-only actions use a local Host handle.
- Authority mutations, join-use accounting, revocation, session checks and snapshots share
  one mutex. Two concurrent uses of the same invite cannot both join. A post committed
  before revocation may be visible; requests ordered after revocation are denied.
- Invites are single-use, expire after at most one hour (UI: five minutes), and are bounded
  to 128 active grants. Invite expiry affects new joins; it does not expire a joined session.
  The UI retains at most 32 issued invites for individual revocation.
- Sessions expire after one hour without an authenticated request; active clients renew that idle deadline. Successful authenticated requests refresh the
  ten-second presence lease, and extend the idle session deadline. Offline roster entries
  remain until session expiry, Leave or revocation. The room permits at most 32 sessions,
  including the host's normal authenticated client.
- Reconnect uses the same in-memory session and cursor on a fresh TCP/TLS connection.
  Transport failures do not create another identity. The UI polls every 500 ms and queues
  a bounded number of outgoing messages. Closing the workspace destroys these credentials;
  this is reconnect within a running client, not identity recovery after app restart.
- Room lifecycle is host-owned. Pause keeps authenticated polling/presence alive while denying
  new invites, joins, host edits and member mutations. Resume re-enables those operations.
  Archive is terminal: polls remain readable for joined sessions, while joins, invites and
  mutations are denied. Invite revocation remains available for privacy after pause/archive.
  The JSON archive is the public host snapshot (latest bounded event window and current shared
  state); the server filters DMs and recipient-scoped patches before it can be copied. It is an
  explicit clipboard export, not automatic disk persistence or restart recovery. Session
  credentials and the TLS private key remain ephemeral and are never written to the archive.
- Mutation operation IDs increase by one per session, with the last action/outcome retained. Retrying a committed operation does not
  add a second event. An uncertain client post retains its operation and text for retry;
  a different action is refused until that operation is acknowledged. A changed retry of the
  same operation is rejected. This applies to both tasks and messages. The UI retains queued messages
  in memory while reconnecting. Leaving discards the queue.
- Events have an authoritative monotonic sequence. Poll returns only events after the
  cursor. The host retains 128 events; a cursor outside that window returns `reset: true`
  and the replacement history. Client history is bounded and replaces on reset. Names
  are 1–64 characters, room names 1–80, messages 1–2000, with control characters refused
  except newline/tab. Clients also validate snapshot bounds before advancing their cursor.

## Shared workflow contract

- Shared tasks reuse forge-room-core ownership, dependency, checklist, priority, due-time,
  handoff and evidence policy. Any member may capture a task for a joined member. Only the
  current owner or host changes it; only the host approves handoffs or reopens completed work.
  A handoff request must originate from the current owner. Revocation immediately removes
  the session from shared participant policy. A disconnected lease retains task ownership
  until session expiry; the host must approve a new handoff while both participants are joined.
- Limits: 100 tasks; 8 criteria of 200 characters each; 16 existing shared dependencies;
  128 evidence records of 500 characters each; latest 128 activity entries. Due offsets are
  bounded to thirty days. Snapshots include host epoch time so relative due labels use the
  host clock rather than the client clock. Local task IDs are not valid shared task links.
- Messages support public/DM channels, decision markers, task links, replies and local search
  of authorized retained history. Reply targets must exist in the same conversation; a third
  participant cannot quote a DM into public chat. Private message bodies and recipient metadata
  are never placed in shared activity. The global replay cursor is opaque and can reveal total
  message sequence progress; this is not traffic-analysis protection.
- The host sets, pauses, resumes and resets a countdown up to seven days. It advances from
  a monotonic host clock; clients display the latest host sample with local interpolation.
  It ends with the hosting session. Up to twenty host-owned milestones have shared titles,
  optional host-clock deadlines and open/completed state; clients see overdue status. Only
  the host creates or completes them. Reminders, persistent room recovery and read-only
  follow remain separate unfinished features; saved patch transport is described below.

No persisted RoomState field or schema changes are required. Existing local rooms load
unchanged. Local Chat, Tasks and acting-as controls retain their local meaning. R03/R05 have the bounded shared workflow above. R06 adds the recipient-scoped saved patch
contract below; no saved local state is serialized implicitly.

## Evidence and remaining gates

Core tests exercise expired/revoked/used/unknown invites, incompatible versions, forged
sessions, certificate mismatch, concurrent invite use, revocation, lease expiry, session
expiry, failed-connection reconnect, exactly-once post retry, replay gaps, oversized frames
and invalid content. Each integration trial starts **two separate `forge-room-peer` OS
processes**, supplies invites only on stdin, checks distinct identities and bidirectional
delivery, denies reuse and revoked requests, verifies cursor replay and clean Leave.
A third multi-process trial checks dependency/completion denial, task ownership, host-approved
handoff, evidence, host-only countdown, acknowledged rejection recovery, DM/reply isolation
from a third client and late joins, and revocation of task and DM access.
A second trial inserts a TCP fault proxy, drops a real client connection, then verifies
recovery with the same identity and one copy of the missed event. The diagnostic client accepts bounded JSON commands on stdin and outputs JSON responses;
credentials are not written to stdout or command-line arguments.

Run `cargo test --locked --manifest-path native/forge-room-core/Cargo.toml` for protocol
and independent-process tests. Windows CI also checks both crates, runs app tests, formatting
and warning-free Clippy. Automated process trials are same-machine loopback transport proof.
They do not establish desktop rendering, keyboard/clipboard behavior, firewall routing or
two-machine acceptance.

Hands-on gate (open): launch two native desktops, host and join, verify the sharing preview,
displayed identities and public messages, revoke a joined invite, interrupt/recover a LAN
connection without duplicate members/messages, wait for offline presence, then leave/end.
Check shared task ownership and host-approved handoff, countdown pause/resume, and a third
client that cannot read another pair's DM or private thread. Repeat on two independent machines with the explicit LAN bind. Check that local files,
local chat/tasks and terminal/debug data remain private. Neither R01 nor R04 is marked fully
accepted until those checks are recorded. Account identity, persistent/host-offline recovery,
public-internet deployment and public-service security review are separate future work.


## R06 saved patch proposal fallback (protocol v3)

A trusted client explicitly selects a saved tracked UTF-8 file, prepares its Git HEAD base
and current saved version, inspects both versions and chooses one joined recipient before
sending. Relative path and full before/after text are shared; unsaved buffers, other files,
terminal output and directory snapshots are not included. Git reads run on a worker.
The server accepts at most eight active patches, with 64 KiB bases and 128 KiB replacements.
Only sender and recipient receive a patch in snapshots; public host snapshots exclude it.
No patch path/body is logged in public activity. The sender (or host-local authority) can
revoke it; either participant leaving, expiring or being revoked removes further delivery.
Copies already received cannot be recalled; this is not end-to-end encryption against the
host process/operator. The client drops a revoked pending review on its next snapshot.

Absolute/traversal/link/alternate-stream paths, .env/.git/.ssh and common credential files
are blocked. Private-key markers and common quoted credential assignments are rejected
on both client preparation and server submission. This is best-effort filtering, not a
complete secret detector, and the sender must inspect both versions before sharing.

Receiving never writes a file. The recipient explicitly reviews against an existing local
workspace file, which must match the exact supplied base. Local apply and guarded undo reuse
the same engine as AI reviewed edits and reject dirty buffers, changed files and links.
There is no automatic merge, co-editing, file creation, folder grant or unsaved-buffer sharing.
Windows line endings in a HEAD base are aligned to a CRLF saved working file for this preview.

Independent process tests check recipient/third-client isolation, secret/traversal rejection,
unauthorized revocation, sender revocation and sender-session revocation. Local review tests
check revision mismatch, no write on receipt, dirty-buffer rejection, apply and undo. Native
desktop and two-machine privacy/revocation acceptance remain open.

Rejected message/task drafts are retained without overwriting newer input. Evidence drafts stay
scoped to their task, and duplicate acceptance criteria are rejected before creating a task.
A hidden panel does not discard acknowledged rejection records; they drain when it is reopened.
