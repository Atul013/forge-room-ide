# Native Room Recovery Center

This bundle extends public checkpoints/journals with one native recovery journey. Source implementation is complete; final exact-head Windows/package evidence is recorded in progress.md when available. It adds no transport protocol version or execution authority.

## User journey

Open Room Recovery Center from Network Room controls. Existing checkpoint/journal recovery buttons now inspect a selected source instead of immediately starting a host. The modeless center remains available when the Room dock is hidden.

- **Recent sources:** search, pin and filter cached inspections, issues and quarantine entries. Entries identify kind, counts, checked time and optional local project association. These are cached observations, not proof that the current file is intact. Select a source to inspect its current bytes. Newly successful saves/rotations register separately from their durability result.
- **Inspect:** view the full source digest, saved capture clock, declared lifecycle, public message cursor, task/owner/criteria data, retained-public-state counts, complete records and incomplete-tail bytes. No listener, session or invite is created by inspection.
- **Compare:** inspect a second source and compare frozen public snapshots. Different Room identities are explicit. Task additions/absence/changes and lifecycle, countdown, readiness/help/impact, Ship, milestone, member, public-message, handoff, evidence and activity snapshot differences are shown. A message cursor does not order every task mutation. Capture-clock-only differences are excluded from the public-state digest.
- **Copy/export:** create a new sanitized checkpoint externally or in the managed vault without starting a host. Export a bounded inspection report, with the local path omitted by default. Exports refuse replacement; unknown imported JSON fields are not copied. No state after the last complete record is invented.
- **Restore:** leave/end the current Network Room first, review exact source identity/digest, bind address and saved state, acknowledge any incomplete tail, and explicitly choose whether to create a fresh managed journal. Source bytes are rechecked immediately before restoration. Active/paused sources recover paused; archived sources remain archived. New TLS, invitations and host-approved owner reclaim are required. No automatic resume, join, owner claim, file edit or agent action occurs.
- **Manage:** forget metadata while preserving sources; discover unindexed files only in the private vault/quarantine; quarantine reviewed inactive owned files; permanently remove only explicitly confirmed, fingerprint-matching inactive owned quarantine bytes. External sources are never moved/deleted. Active managed journals and restore/file operations hold an OS file lease shared across native windows.

Cancel during restore cancels adoption when the worker completes; the temporary host is stopped and original files/any new managed copy are retained. A later Network Room change also prevents adopting a recovered host over the current connection. Host shutdown retains its journal lease until the actual host has closed. Recovery cannot establish whether an original host or copied host is still running on another machine; stop it before restoration.

## Core inspection and compatibility

Inspection reads one regular non-linked source, bounded to 64 MiB, and validates every complete record. Checkpoints remain version 1 (8 MiB maximum); journals remain FRJ1 with 8 MiB records and a 64 MiB total limit. Incomplete final frames report their byte count and use only the last complete validated record. Complete corruption is rejected and preserved.

Journal records must retain the same host/Room identity and a nonregressing public message cursor; an archived history cannot return to active/paused in a later frame. The previous per-record-only reader is replaced by this same inspector for both legacy recovery entry points. Core immutable previews retain typed validated fields, not raw input; export drops unknown input fields. Schema/state error categories do not echo untrusted JSON values into persistent catalog diagnostics.

The public state includes tasks, ownership, retained public messages/activity, handoffs, evidence, readiness/impacts/help, milestones, Ship declarations and a paused countdown. It excludes DMs, patches/receipts, reactions, TLS material, invitations/session tokens, source files, live focus/locations, terminals and AI state. Saved manual reports remain declarations. Digests/structure checks do not establish source authenticity, original-person identity or a matching workspace.

## Private catalog and managed storage

Storage is `%LOCALAPPDATA%/Forge Room Early Preview/room-recovery`. Versioned checksummed FRC1 index records use create-only synced writes, short exclusive writer locks and exact revision seals. Sixteen index snapshots and sixty-four management events are retained, with at most 128 source references and a 1 MiB index record limit. Incomplete index tails can recover the last complete record; complete corruption closes the catalog while preserving sources. Selected-file inspection/export and checkpoint restoration still work without an intact catalog. Managed journal creation needs an intact catalog; external journaling remains available.

The managed vault reserves each journal's full 64 MiB allowance and each checkpoint's 8 MiB allowance, including pending creations, against a 512 MiB quota. Up to 128 owned files are supported. Reservations are durable before creation; interrupted/failed creation stays visible rather than auto-resuming. Quarantine remains charged until explicitly removed. Manual export destinations cannot enter the private recovery namespace and bypass its quota/index rules.

The catalog contains registered paths, optional user-chosen project associations, public summary metadata, bounded error categories and management history. It stores no credentials or source bodies. Source/project metadata is local; recovery does not open or verify that project. Existing associations survive ordinary refresh unless explicitly reassociated.

Rendering performs no source-directory probing. Inspection, comparison, exports and catalog operations run on one worker slot, with at most sixteen queued successful-save registrations. Cancellation/adoption and completion are separate. Existing host save/rotate controls retain the core's bounded host-write path; registration failure is reported separately from a successful flushed checkpoint/journal.

File management records an intent before moving/removing data and a separate completion afterward. A crash or filesystem error leaves either a preserved original/unindexed owned source or its committed quarantine reference, with discovery/reinspection available. There is no destructive reset or automatic source deletion. These integrity/coordination controls are not an OS sandbox or a cryptographically signed backup system.

## Verification and open acceptance

Core tests cover exact-source staleness, sanitized exports/unknown-field stripping, schema diagnostic privacy, incomplete tails, complete corruption, mixed/regressing/terminal-archive histories, comparisons, fresh TLS/claims and fresh-journal requirements. Independent processes force-kill a real host, inspect/export its saved public state, recover it and reclaim owners with continued durability.

Native tests cover catalog locks/staleness/torn/corrupt states, worst-case reservations, protected leases, damaged-source quarantine/removal, source preservation, private namespace guards, non-hosting copies, asynchronous inspection/paused restore, cancelled adoption, closed-catalog fallback, host/lease lifetime and prevention of connection replacement. Offscreen frames cover all three pages and restore/file reviews at regular/narrow input sizes. These are source/process/frame checks, not visible desktop acceptance.

Visible desktop/installer and physical two-client restart/reclaim remain open. The current desktop probe still returns native pipe unavailable / os error 2, and the user has no second physical device available. This bundle introduces no provider calls; existing real native-provider acceptance remains separate and open. Final Windows verification/package/download/install results and exact source identity belong in progress.md.
