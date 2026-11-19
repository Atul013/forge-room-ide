# Room Recovery Center - implementation contract

In progress on feat/room-recovery-center; this document is a target contract until final evidence is recorded. One complete native bundle extends existing public checkpoints/journals rather than adding another transport or execution authority.

## Interfaces and invariants

- Core inspection opens no host or network listener. It reads one regular, non-linked bounded file, validates every complete checkpoint/journal record, identifies incomplete tails, enforces source continuity and returns an immutable private preview with a full source digest, kind, identity, retained-public-state summary and sanitized checkpoint payload.
- Restore requires explicit user review and an unchanged source. The caller supplies the bind address. Core restores paused (archived stays archived), fresh TLS and no sessions/invites. Journal recovery still requires a fresh journal before mutations. No old bearer credentials or private histories are retained.
- Comparison uses inspected public task/coordination summaries; differing Room identities remain explicit and cannot masquerade as a continuous history. Export creates a new sanitized checkpoint, never overwrites or edits original source, and does not invent work after the last complete frame.
- The native private catalog records explicitly registered paths/managed files and bounded metadata, never credentials, private messages or source bodies. A checked create-only local record ledger, short writer lock, stale seals and finite retention keep competing windows from overwriting each other.
- Managed files live only under the catalog vault. Discovery is bounded to that vault; no broad disk scan. External sources are referenced, never moved/deleted. Quarantine preserves only reviewed inactive owned files; active journals are protected. Forget changes catalog metadata only.
- The center presents recents, source health, incomplete-tail loss, public-state inspection/comparison, exact restore review, optional explicit fresh managed journal, checkpoint export and inactive-file management. Background inspection/catalog work must not block egui. Existing save/rotate controls register successful sources, reporting catalog failures separately from journal/checkpoint durability.
- Recovery does not verify a workspace association, build result, manual check declaration, provider result or original person's identity. Existing explicit owner reclaim remains necessary. Workspace/source/terminal/AI contents and live focus are not restored.

## Bounds and migration

Existing checkpoint version 1 and FRJ1 journal framing remain readable (8 MiB frame, 64 MiB journal). New integrity/continuity errors preserve bytes. The private catalog is additive and versioned; old users can still inspect a selected file without an existing catalog. No source format rewrite or destructive reset is required.

## Acceptance to implement

Core validation/export/staleness/continuity/private-field tests; real-host paused restore and fresh-claim/journal tests; independent-process forced-host restart with inspected recovery; native catalog concurrency/torn/corrupt/managed-file bounds and active-file protection; complete UI frames and action paths. Final full Windows verification/package/download/install receipts remain separate from visible desktop and physical LAN acceptance.
