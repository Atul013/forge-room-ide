# Native Studio and resource packs

Studio is one local declarative resource system, entered from Extensions, Settings or Accessibility. The existing theme gallery, loose grammar/snippet imports, workflow editor and explicit AI context controls remain available. This bundle does not add executable plugins or a remote marketplace.

## Complete user flow

1. Open the Pack workshop to author a manifest, use the eight-resource local starter or copy an installed version. Edit metadata, dependencies, individual bodies or complete JSON; import bounded UTF-8 bodies, recompute digests, capture appearance and export a new pack file. Installation does not occur while editing.
2. Inspect a local pack file or workshop draft. Review publisher/source/license claims, readme, resource bodies and content hashes. Install the exact inspected immutable version **disabled**. Same-version different bytes are rejected.
3. Select one version per pack and individual resources, then validate dependencies and review the activation. Preparation compiles the complete grammar set before committing. Stale registry/selection reviews fail; rollback is a new reviewed revision.
4. Enabled themes enter the native theme picker, grammars rebuild highlighting/cache and snippets enter the editor picker. Workflow libraries and Skills remain data until explicit staging. Profiles and widget permissions remain separate choices.
5. Stage workflows in the trusted active project's existing editor; imported identities are disambiguated, Room links are cleared by default, and saving/execution still require their usual explicit actions. Capture a Skill only in an idle trusted active project; Include and Send remain explicit. No pack script/assets are read or executed.

## Appearance and Morph

Profiles are strict typed JSON: theme reference, editor text size (60-250%), interface size (Automatic or 80-200%), compact/comfortable/spacious control spacing, control animation preference and existing native panel visibility/sizes. Theme references must be available. Profiles cannot grant trust, configure a provider, edit shortcuts or write source.

Capture, edit or select a profile, then Preview. The modal displays changes and offers Keep or Restore. Session persistence is suspended during preview; a normal exit restores the prior appearance. If the workspace/layout changes meanwhile, restore can preserve the newer layout. Keep saves appearance through existing Settings/session paths and appends private appearance history (eight retained entries); a history failure is reported separately from a successful Settings save. History selection only stages a candidate. Saved malformed appearance payloads do not discard unrelated trust/provider settings. Native monitor fitting remains the interface-scaling contract; no automatic multi-monitor relocation claim is added.

Morph optionally uses the enabled Native HTTP configuration. Prepare freezes a request containing only current appearance, available theme references and the user's appearance instruction. Review endpoint/model/token limit and exact body, then Send explicitly. History is disabled, tools are absent, each send reserves one of eight requests per explicitly reset Morph session, output is bounded to 64 KiB and parsed as the same strict profile schema. A valid response is only a draft; Preview and Keep remain separate. Failed, oversized or executable/unknown-field output cannot change appearance. Manual editing needs no model/server. This is not arbitrary generated UI/code.

## Widgets and privacy

Widgets use bounded native Metric/List/Table bindings with one or two columns, 1-60 second refresh and at most twelve bindings/32 rows each. Sources are a closed enum: Workspace, cached Git, cached Problems, Workflow status counts and visible public network Room task counts. The bridge captures only requested, explicitly granted sources, with no file paths, messages, private conversations, logs, raw terminal output, identities or arbitrary APIs. Workspace counts cover application buffers; Git counts are repository-wide; diagnostics are filtered to the active root; workflow successes may include manual/model steps and are not CI acceptance.

Permissions start empty and are scoped to the trusted active workspace/session. Scope/trust/resource changes revoke grants and clear cached data. No grants are serialized. Each binding checks its grant again at render time. Simulated previews have their own labeled sample snapshot; they are rejected by the live capture path. Up to four widget windows can be pinned. Cached snapshots are labeled and refresh wakes idle egui frames; refresh never runs Git, diagnostics, commands or network requests. Widget exports are definitions only.

## Storage, recovery and boundaries

Packs use `%LOCALAPPDATA%/Forge Room Early Preview/packs`. Version data and monotonic ledger snapshots are create-only, checksummed FRC1 records synced before success. Short exclusive writer locks and expected-ledger checks reject competing/stale mutations. Sixteen ledger snapshots and sixty-four audit events are retained; activation allows at most 32 enabled packs/128 resources/8 MiB; the catalog allows 64 versions. Individual JSON packs are at most 4 MiB, resources total 2 MiB with per-kind limits. Registry data/states/quarantine have a 512 MiB/1024-file cap. These are integrity/retention boundaries, not an OS sandbox or verified publisher signature.

Incomplete final ledger frames fall back to the last complete validated record. A complete framed checksum/schema failure closes storage rather than silently substituting an older state. Damaged active version data disables contributions at load while retaining the ledger for repair. If an optional loose grammar cannot rebuild during error recovery, clearing contributions restores built-in syntax without depending on that failed grammar. Explicit disable-all retains bytes; exact-source repair accepts only the original immutable version digest and archives damaged bytes. Inactive versions can be quarantined (preserved) or explicitly removed. Cleanup warnings are reported. Complete ledger corruption requires inspection/restoration of intact ledger data; there is no destructive reset button. Quarantined bytes are not automatically deleted.

Author drafts are explicitly saved private records, including invalid/unfinished identity, version and JSON edits. An exclusive draft writer uses monotonic names; eight 8 MiB-bounded records are retained (separate from the registry's cap). Incomplete tails can recover the previous draft; complete corruption remains visible and preserved. Reloading the registry does not overwrite an in-progress workshop draft. Kept appearance history separately retains eight 64 KiB records. Exports create new files and refuse replacement. Existing workflow FRC1 framing is unchanged and covered by the golden compatibility regression.

## Verification and open acceptance

Local source verification: strict schemas/dependencies/version upgrades/selective rollback, immutable install/export, competing writers, torn/corrupt restart, exact repair/quarantine, all eight native resource parsers, pre-publication grammar/profile conflicts, unfinished draft/history restart, profile scope-aware rollback, widget privacy/grant revocation and exact mock HTTP Morph sends. Offscreen egui frames cover all five Studio pages at regular/narrow input sizes and the install review. These checks do not establish visible Windows behavior.

Final Windows CI and package receipts are recorded in progress.md after completion. Visible desktop interaction (including preview/restore, real editor resource behavior, scaled/narrow layouts and installer UI), a configured real native model's Morph response, and physical two-client Room acceptance remain independent gates. The desktop bridge probe on 19 November still returned native pipe unavailable / os error 2. No usual local model listener/executable or second physical device is currently available. No hands-on/provider/physical acceptance is claimed.

## Room blueprint extension

The Room Launch Center adds a ninth `room_blueprint` resource kind, validated by the strict public setup schema. Workshop authors it, selective activation exposes it, and an explicit Stage action opens the Launch Center draft. A separate frozen launch review is required to create a fresh host. Existing resource kinds and saved packs remain supported; older binaries reject this new kind. See [ROOM_LAUNCH_CENTER.md](ROOM_LAUNCH_CENTER.md).
