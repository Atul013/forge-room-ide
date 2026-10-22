# Forge Room service control plane

A self-hostable service-side slice for PRODUCT_SPEC.md Phase 6. It provides TLS-only account provisioning, account key rotation, owned room metadata, expiring/revocable bearer invites, scoped audit views, single-process durable control state, and a WSS relay. Team Lobby connects using an HTTPS origin and an account key stored in VS Code SecretStorage. The service relays live protocol-16 traffic but does not persist room messages, tasks, notes, or shared documents.

## Run

Requires Node.js 22 or newer and the `ws` runtime dependency. Set `FORGE_ROOM_TLS_CERT`, `FORGE_ROOM_TLS_KEY`, `FORGE_ROOM_ADMIN_KEY` (at least 32 random characters), and optionally `FORGE_ROOM_DATA`, `FORGE_ROOM_PORT` (default 4780), and `FORGE_ROOM_BIND` (default `127.0.0.1`). Terminate TLS here and expose the listener only behind an appropriately managed network boundary.

The admin key is a bootstrap credential and must be stored outside the workspace. Account keys and invite tokens are returned only at provisioning/rotation and invite creation; the database stores SHA-256 hashes. Rotation invalidates the old key immediately, writes an account audit event, and closes rooms hosted by the account. On POSIX, the database directory and file use restrictive `0700`/`0600` modes, writes use a synced temporary file and atomic rename, and corrupt state fails closed. Run one process against one database path; multi-process locking and replicated storage are not implemented.

## API

All endpoints except `/health` require a bearer credential over HTTPS.

| Method and path | Credential | Result |
|---|---|---|
| `GET /health` | None | Service liveness |
| `POST /v1/admin/accounts` | Admin key | Provision account; return its key once |
| `GET /v1/admin/accounts` | Admin key | List account metadata, never keys |
| `POST /v1/admin/accounts/{id}/rotate-key` | Admin key | Recover an account; return one replacement, invalidate the old key and close hosted sessions. A disabled account is reactivated and its outstanding invites are revoked. |
| `POST /v1/admin/accounts/{id}/disable` | Admin key | Disable account and its relay sessions |
| `POST /v1/account/rotate-key` | Current account key | Return a replacement once; invalidate old key and hosted sessions |
| `GET /v1/account/audit` | Account key | Page through account, owned-room, and invite lifecycle events |
| `POST /v1/rooms` | Account key | Create an owned room |
| `GET /v1/rooms` | Account key | List only the caller's rooms |
| `POST /v1/rooms/{id}/invites` | Owning account key | Create one-time-visible invite (60 seconds to 30 days) |
| `GET /v1/rooms/{id}/invites` | Owning account key | List invite status and expiry |
| `POST /v1/rooms/{id}/invites/{inviteId}/revoke` | Owning account key | Revoke invite and disconnect active guests |
| `GET /v1/rooms/{id}/audit` | Owning account key | Page through room and invite lifecycle events |

Audit endpoints accept `limit` from 1 to 200 (default 100) and a `cursor` from the previous response. Pages are bounded and returned in chronological order; `nextCursor: null` means there are no older records. Cursors that have expired because the bounded audit log rolled over are rejected so the client can restart from the newest page. Team Lobby exposes the account-wide view through **Team Lobby: View Room Service Audit History**. Entries contain action, actor, resource ID, and timestamp; they do not contain room content or credentials.

The relay endpoint is `wss://<service>/v1/relay/{roomId}?mode=host|participant`. The host authenticates in the HTTP upgrade request. Participants send the normal room `hello` with a service-issued invite; the service validates the invite before forwarding to the host. The extension bridges guest sockets to its loopback-only local room authority.

## Verify

Run `npm.cmd test`. Tests use the test-only certificate in `test/fixtures/server.pfx`; do not deploy it.

## Current limits

Account owners can rotate a known key, and the local service operator can recover a lost key. Admin recovery reactivates a disabled account only while replacing its key, and revokes its outstanding invites so disabled access is not silently restored. There is no self-signup, user-driven recovery, MFA, organization hierarchy, database encryption at rest, multi-node deployment, admin web console, or operational monitoring. Separate IDE and separate-machine acceptance remain open. These are required before calling this a managed production service. See [THREAT_MODEL.md](../../docs/architecture/THREAT_MODEL.md).
