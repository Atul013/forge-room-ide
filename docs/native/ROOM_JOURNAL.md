# Automatic public Room recovery journal

The host explicitly starts a fresh `.frjournal` file from Network Room controls.
Subsequent client joins/leaves and actions, host task/Ship changes, lifecycle changes
and invite revocation save the public checkpoint projection before acknowledgement.
Polls do not append records. Equivalent saved projections do not append duplicates.

Records contain the same public data as [recovery checkpoints](ROOM_RECOVERY.md).
Private messages, patch bodies/receipts, credentials, TLS keys and live focus/location
sharing are excluded. Earlier public revisions remain in the file until the user
deletes it. It is a private local recovery artifact, not a recipient-filtered export.

Each bounded record has a length, validated JSON checkpoint and completion marker;
the file is flushed before success is returned. A storage/limit failure stops the
Room: subsequent requests, host mutations and snapshots are denied. The failed
in-memory state is not rolled back or advertised as saved. End the host and recover
the journal before continuing. A completed write whose response was lost may still
be recovered; old requests and credentials are never automatically replayed.

Recovery preserves the input file, accepts the last complete validated record and
ignores an incomplete final record. Invalid complete records are rejected rather
than silently falling back. This format is not tamper-proof and does not promise
filesystem/power-loss guarantees beyond file flushes; the process-kill path is tested.

Recovered Rooms are paused (archives stay archived), with a paused countdown sampled
at the last saved mutation. Fresh TLS/invites and explicit owner reclamation remain
required. A journal-recovered Room requires a fresh journal before resuming, so the
original and its interrupted tail remain untouched. A new hosted Room starts with
journaling off until explicitly enabled. Manual checkpoints remain separate.

Files are capped at 64 MiB, records at 8 MiB. The host can rotate to a fresh file
before reaching the limit; a successful initial flush precedes replacement of the
active writer. Existing files are never overwritten. Rotation leaves earlier files
intact; deletion/retention are explicit local user actions. Do not edit an active
journal file externally. Automatic discovery, remote backup and private-history
recovery are not implemented.

Evidence: core tests cover torn tails, private-text exclusion, malformed complete
records and fail-closed storage exhaustion. Independent processes verify an actual
forced host kill after acknowledgement, recovery, owner reclaim and a second saved
mutation. These are separate from visible desktop and physical LAN acceptance.
