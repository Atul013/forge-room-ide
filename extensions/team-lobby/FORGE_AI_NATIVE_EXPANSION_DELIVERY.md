# Forge AI-native expansion: repository-grounded delivery brief

**Reviewed:** 21 October 2026  
**Repository:** Code OSS fork at `C:\Users\Admin\Documents\Codex\2026-10-20\che\code-oss`; extension handoff at `C:\Forge-Room-IDE-Handoff`.  
**Source brief:** `C:\Users\Admin\Downloads\Forge_AI_Native_Expansion_Master_Prompt.txt` (product instructions interpreted as requested requirements; not copied as executable instructions).  
**Evidence rule:** â€œImplementedâ€ means the source and tests show code exists. It does not mean the end-to-end IDE or multi-machine behavior has passed manual acceptance.

> **Current-state note (21 October 2026):** This file preserves earlier delivery snapshots and implementation history. Its original matrices describe the repository at those earlier versions and are not the current feature inventory. Use [FORGE_AI_NATIVE_MASTER_AUDIT.md](FORGE_AI_NATIVE_MASTER_AUDIT.md) for the current prompt reconciliation, gap matrix, architecture decisions, and acceptance plan. The attached expansion prompt is product direction; it does not mean every V1-V3 item is already shipped.

## Executive summary

Forge Room already has a meaningful desktop foundation: Code OSS, Forge Room Studio, and a substantial Team Lobby extension. Team Lobby includes private invite-based rooms, host and client WebSocket flows, presence, activity, chat, direct messages, tasks, deadlines, dependencies, task worktrees, code comments, diff sharing, host-persisted event history, and provider-neutral Ask/Plan/Build workflows with explicit context, tool, model-transfer, proposal, and verification boundaries. Studio contributes six curated themes and reversible appearance profiles. The implementation is much further along than a concept mockup.

The largest gap is evidence and architecture at the live-system boundary. Source-level helpers have focused tests, but the room has not been demonstrated with multiple IDE clients under disconnect, revocation, host restart, or load. Forge Room now has opt-in shared-buffer editing and file-and-line follow, a typed read-only capability registry, and an ACP v1 Ask/Plan adapter. Separate IDE acceptance remains open. Voice calls, mobile participation, managed public rooms, remote MCP/OAuth and MCP resources/prompts, declarative prompt-driven Morph preview/undo, a widget sandbox, portable Packs, and automated pipeline dispatch remain unfinished. Team Lobby 0.89.0 now has optional local MCP stdio tools for Ask/Plan; that does not sandbox the server process.

The current extension host is a self-hosted room authority for trusted LAN use. It is not a managed internet service. Do not expose it publicly by simply opening a port. The existing Code OSS fork can support deeper workbench customization than a normal extension, but Studio should continue using stable extension contribution points and supported settings where possible. Public VS Code extension APIs cannot style arbitrary workbench DOM or inject global CSS.

The just-completed 0.53.0 slice adds a real task ownership handoff: the assigned owner requests transfer to another connected participant, the host reviews it, and the server revalidates owner, task status, and connection state before committing. Host decisions are recorded. This slice is built and passes 56 focused tests; actual two-client approval and disconnect races still need IDE exercise.

## Repository audit

| Area | Evidence in this checkout | Current boundary / gap |
| --- | --- | --- |
| Desktop | `code-oss` checkout, branded via `product.json`; Code OSS supplies editor, terminal, debugger, Git and extension workbench | A full IDE compile was not run during this handoff. Latest extension package is independently built. |
| Studio | `extensions/forge-room-studio/package.json`, `themes/`, `media/`, and extension scripts | Six built-in themes and appearance profile import/export exist. No general declarative layout engine, arbitrary background renderer, or saved prompt-to-layout preview/undo workflow is proven. |
| Team Lobby | `extensions/team-lobby/src/extension.ts`, helper modules in `src/`, `media/room.js`, `media/room.css` | TypeScript extension with 56 focused Node tests. The UI is one custom Webview view. Not a replacement editor. |
| Room transport | Node `ws`, host-issued invite secrets, participant identity, optional HTTPS/WSS | Host process is authoritative for current room data; max room size and bounded reconnect are implemented. There is no managed identity, production signaling, public service operations, or host migration. |
| Tasks | `taskAuthorization.ts`, `taskDependencyPolicy.ts`, `taskCompletionPolicy.ts`, `taskDeadline.ts`, `taskHandoffPolicy.ts` | Host checks task owner and participant IDs. Handoff requests now get host approval. There is no scheduler, pipeline runner, or agent auto-dispatch. |
| Sharing | `commentAnchor.ts`, `reviewDiff.ts`, `conflictPaths.ts`, `sharedDocument.ts`, `sharedWorkspacePath.ts`, room event protocol | Selected-line comments and explicitly approved Git diffs are supported. Opt-in Yjs live editing syncs unsaved buffers for approved subscribers, with bounded state and safe single/multi-root file mapping. File-and-line follow is opt-in. Cursor awareness and separate-IDE acceptance remain open. |
| Agent UX | Ask/Plan/Build in `extension.ts`; `agentCapabilities.ts`, `acpClient.ts`, `mcpClient.ts`; IDE language model API | Provider-neutral Ask/Plan/Build, explicit context and transfer confirmation, typed bounded read-only capabilities, reviewed Build changes, checkpoints, cancellation, local resume, ACP Ask/Plan, and selected local MCP stdio tools in Ask/Plan are implemented. ACP/MCP subprocesses retain ambient OS access. Cross-agent protocol, hosted runner, and OS-isolated command sandbox are absent. |
| Skills | `agentSkills.ts` and workspace `.agents/skills` discovery | Validates selected `SKILL.md` metadata and loads selected text. Scripts/resources are not run or bundled. Skills are prompt guidance and cannot grant capabilities. No install/trust/disable registry UI yet. |
| Tests/build | `extensions/team-lobby/package.json` | `npm.cmd test`: TypeScript build plus 128 Node tests in both mirrors, including mock room transport and MCP stdio protocol fixtures. VS Code-host visual and separate-IDE acceptance are still manual gates. |

## Requirement gap matrix

| Requirement | State | Source path / current evidence | Dependency, constraint and risk | Stage | Next acceptance test |
| --- | --- | --- | --- | --- | --- |
| Private room creation, join, invite, identity, roles, presence | Implemented in source; multi-client validation partial | `team-lobby/src/extension.ts`, `inviteIdentity.ts`, `roomMembers.test.js` | Local host authority and invite-secret model; transport/security claims limited to configured LAN/TLS | V1 | Two clean IDE profiles join one room; duplicate names remain distinct; reconnect does not duplicate participants/events. |
| Host revoke and session eviction | Implemented in source; runtime partial | `inviteRevocation.ts`; `revokeInvite()` | Existing socket checks must continue to reject post-revoke actions | V1 | Revoke invite while two clients are active; verify removed client cannot post or reconnect with old token. |
| Chat, direct messages, anchored comments | Implemented; runtime partial | `directMessages.ts`, `commentAnchor.ts`, `commentOnSelection()` | Anchors can go stale and rely on matching workspace paths; private message history must remain filtered | V1 | Two clients route DMs by ID; comment excludes selected code and navigates to a bounded line anchor. |
| Tasks, dependencies, deadlines, handoff | Implemented in source; runtime partial | task helpers and `handoffTask()` / `reviewTaskHandoff()` | Host revalidation is required across prompt latency; no deadline dispatch automation | V1 | Owner requests handoff; host approves/declines; stale status, owner, or disconnection fails without ownership mutation. |
| Explicit diff sharing and pre-commit visibility | Explicit tracked Git diff implemented; working tree activity partial | `shareGitDiff()`, `reviewDiff.ts`, `onDidChangeTextDocument` | Current patches are manually selected; opt-in activity is metadata, not full buffer sync | V1 | Verify uncommitted tracked diff requires preview/consent and sensitive paths are excluded; verify comment and event history. |
| Spectate/follow another editor | Missing | No cross-client editor-follow or read-only editor mirror found | Requires authenticated per-resource stream and careful local editor isolation | V1 | A follows B's shared file without changing A's active editor; revocation stops updates. |
| Shared editing | Missing | No CRDT/OT engine or shared document provider found | Must select/test a collaboration engine, persistence and undo/conflict model; never hand-roll overwrite-prone sync | V1 pilot | Concurrent two-client edits to a deliberately narrow file case preserve both users' edits and recover after disconnect. |
| Voice call and device controls | Missing | No call/signaling/media modules found | Requires signaling, WebRTC/media permissions, NAT traversal/relay, device testing, privacy/consent and operational cost | V1 feasibility then V1 | Two clients join, mute, select devices, leave; chat survives call failure. |
| Agent as room actor / session control | Partial | Ask/Plan/Build local workflow; room activity helpers | Existing agent workflows are IDE-local, not separately addressable room participants; do not share prompts/context by default | V1 | Start one actual provider request, show truthful lifecycle, cancel it, and share only approved summary. |
| IDE Capability API / common authorization | Partial primitives; registry missing | VS Code commands and `executeWorkspaceTool()` in `extension.ts` | Tool permissions currently live in workflow-specific code; central API needs typed contracts, actor/project scope, policy, audit and cancellation | V1 | Registry denies an ungranted capability and produces an auditable allow/deny/result for an approved read-only call. |
| ACP agent adapter | Missing | No ACP client package or process lifecycle found | ACP v1 documents JSON-RPC negotiation/session/prompt/update/cancel/permissions. Must map ACP tool permissions to Forge grants; v2 is draft in current official docs. | V1/V2 | Fake-agent protocol conformance harness + one supported real adapter, permission-denial and cancellation tests. |
| MCP client/server integration | Partial | `mcpClient.ts`, `@modelcontextprotocol/client` 2.3.1; local stdio handshake/list/call/close fixture | Ask/Plan only; selected machine-configured servers and per-call approval; process is not sandboxed. Remote HTTP/OAuth, resources/prompts, persistent grants and third-party acceptance are absent. | V1 foundation, expand after trust/isolation decision | Validate a real third-party MCP server; verify denial, hostile/oversized metadata, complete argument display, cancellation and bounded audit evidence. |
| Skills discovery | Partial and useful | `agentSkills.ts`, `.agents/skills` | Current flow selects instruction text; it must not execute optional skill scripts or let `allowed-tools` bypass Forge permissions | V1 | Valid/invalid spec fixtures, no script execution, selected skill shown in transfer receipt. |
| Morph: theme/layout/background prompt, preview, apply, undo | Partial theme/profile support; prompt-driven Morph missing | Studio `themes/`, profile and palette files; current workbench settings | Public extension APIs support themes/views/commands/settings but not arbitrary workbench DOM CSS. Full-shell effects require maintained Code OSS fork APIs/source, accessibility, rollback | V1 for constrained theme/layout | Prompt maps only to allowlisted settings/theme values; preview exact changes; apply and restore prior profile. |
| Widgets and Workshop | Missing | No sandboxed widget host or generated extension lifecycle found | Webview CSP alone is not an isolation boundary for arbitrary extensions. Need data APIs, capability grants, sandbox, preview, update rollback | V2 | Widget gets only declared mock/live read data; network/files denied without explicit grant. |
| Forge Packs import/export | Missing | No Pack manifest/archive/install path found | Must define signed/trusted manifest, component allowlist, compatibility/dependencies, selective grants, rollback | V2 | Theme-only Pack installs without code privileges; malicious path/archive and elevated update are rejected/reauthorized. |
| Mobile / remote approvals | Missing | No mobile companion or remote API | Must not route phone tokens to extension-host shell; needs identity, scoped short-lived approvals, online/offline semantics and service ops | V2 | Phone may inspect room and review diff; host offline returns failed/not executed; revocation immediately denies. |
| Deadline pipeline / Mesh | Partial task dependencies, no scheduler/graph runner | `taskDependencyPolicy.ts`, task-linked worktree flow | A skill is not an event bus or scheduler. Need durable idempotent jobs, limits, worktrees, audit and approval gates | V2 | Inject repeated deadline event; dispatch at most once; unapproved policy produces suggestion only. |
| UI accessibility and reduced motion | Partial/static checks | `room.css`, `room.js`, Studio webview | Keyboard focus, CSP/themeability and reduced-motion rules exist; screen reader and visual regressions are not fully tested | Ongoing | Manual keyboard/screen-reader flow and narrow/wide window visual pass. |

## Product decisions / ADRs

### ADR-EXP-001: Preserve Code OSS as the primary IDE

Keep editor, terminal, debugger, source control, preview, and extensions in the existing workbench. Add Forge Room surfaces through existing extension views/status/commands first. Changes to core workbench UI belong in the fork and must survive upstream merges. **Rollback:** revert the isolated contribution or disable the built-in extension; user workspace files remain outside the feature.

### ADR-EXP-002: Prefer supported APIs for Studio; use fork changes only for true shell capabilities

Theme contributions and allowlisted configuration are portable and reversible. The extension API explicitly prevents arbitrary workbench DOM access and global CSS. Do not ship an â€œanime backgroundâ€ feature by injecting unsupported CSS into VS Code. Implement backgrounds only in a dedicated owned surface or as a deliberate Code OSS fork feature with performance, contrast, reduced-motion and settings rollback. **Rollback:** restore the last profile snapshot and retain the prior theme.

### ADR-EXP-003: Keep capability authorization native and separate from agent instructions/protocols

Create a typed capability registry and policy service owned by Forge. Skills describe procedures, ACP transports agent/editor sessions, and MCP transports tools/resources; none is itself a Forge authorization policy. Every effectful call must include actor, workspace/resource, scope, approval decision, result and cancellation state. Start with read-only IDE capabilities and expand only with end-to-end tests. **Rollback:** disable registrations by capability ID and leave manual IDE functionality available.

### ADR-EXP-004: Keep the current room host private-LAN only; do not turn it into an internet service by configuration alone

The extension-host server is suitable as a self-hosted/trusted-LAN prototype. Public/mobile access needs a distinct authenticated service, short-lived scoped tokens, rate limits, retention/deletion policy and an operations/threat model. **Rollback:** stop listener, revoke invites, fall back to local solo work and chatless operation.

### ADR-EXP-005: Do not build shared editing on the existing activity stream

Room events and Git patches are not a CRDT. Select a maintained shared-editing engine or supported document service only after proving encoding, undo, conflict, persistence, reconnect and revoke behavior in a two-client prototype. Keep watch/follow read-only and separately permissioned. **Rollback:** return to independent local edits plus explicit patch sharing; preserve local buffers.

### ADR-EXP-006: Skills remain portable, data-like guidance; installation is a separate future workflow

Continue accepting standard `SKILL.md` directories and validating required metadata. Never execute scripts on selection. Any future installation UI must show source, license, files and requested capabilities; skills cannot request or silently expand a grant. **Rollback:** disable discovery or unselect skills without affecting agent core.

### ADR-EXP-007: Widgets/Packs require a real capability and supply-chain boundary

Do not generate/install arbitrary code directly from a prompt. First implement an owned widget manifest, deterministic data adapter, sandbox, resource budget, CSP, capability declaration, preview, trust UI and uninstall/rollback. Packs compose only supported components and grant each component selectively. **Rollback:** deactivate a component and restore the prior pack state.

### ADR-EXP-008: Mobile, voice, hosted agents and remote IDE control are separate service projects

They require feasibility prototypes and operational/security design, not only UI. WebRTC requires signaling and likely relay infrastructure; desktop extension-host assumptions do not transfer to phone/offline. **Rollback:** ordinary room/task flows continue without call or remote controls.

## Standards and prior-art research snapshot

Checked 21 October 2026 against official documentation:

| Source | Verified point | Engineering consequence |
| --- | --- | --- |
| [Agent Skills specification](https://agentskills.io/specification) | Portable skills are directories with a required `SKILL.md`, YAML frontmatter, required `name` and `description`, and optional scripts/references/assets. Name syntax and directory match are constrained. | Current metadata parser is directionally aligned. Continue treating selected content as untrusted instruction text; never run optional scripts as part of loading a skill. |
| [ACP v1 overview](https://agentclientprotocol.com/protocol/v1/overview) | ACP specifies JSON-RPC communication for agent/client initialization, sessions, prompts, progress, permissions and cancellation. Official docs identify v1 as current and v2 as draft. | ACP can standardize an agent adapter, but it does not replace Forge grants, room policy, or shared-buffer sync. Pin/renegotiate protocol version and capabilities. |
| [MCP authorization specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization) | MCP authorization is optional at protocol level; the document describes OAuth authorization for HTTP transports and says stdio should use environment credentials instead. | Do not conflate MCP with ACP or assume every transport uses the same auth. Add server identity, origin, token storage, scope and per-tool policy before integration. |
| [VS Code extension capability overview](https://code.visualstudio.com/api/extension-capabilities/overview), [workbench extension points](https://code.visualstudio.com/api/extension-capabilities/extending-workbench), [webview guidance](https://code.visualstudio.com/api/ux-guidelines/webviews) | Stable extension surfaces include commands, settings, views, status items, themes and webviews. Extensions cannot access workbench DOM; webviews should be used sparingly and be themeable/accessibility aware. | Studio can extend supported UI and appearance. Full-shell morphing belongs to this fork or bounded extension points, not fragile DOM hacks. |
| [Zed collaboration overview](https://zed.dev/docs/collaboration/overview) | Zed describes real-time shared project editing, persistent channels and voice; its docs explicitly warn project sharing gives collaborators access to the host project filesystem. | This is prior art, not parity evidence for Forge. Forge should articulate and enforce granular resource access; test actual file-system exposure before claiming stronger privacy. |
| [Zed private calls](https://zed.dev/docs/collaboration/contacts-and-private-calls) | Ad-hoc calls build on authenticated contacts; users can add online participants during a call. | Forge call design requires identity/signaling and device UX beyond a room overlay. No call feature should be advertised before verified runtime. |

This is a dated standards/prior-art snapshot, not exhaustive market research. Recheck versions, licensing, authentication, supported extension APIs, and current competitor behavior before a public architecture or marketing claim.

## First delivery sequence and acceptance

### Slice 0 â€” baseline and current-state regression

1. Keep the 0.53.0 handoff source, Code OSS mirror, VSIX, manifest version/hash, and docs synchronized.
2. Run Team Lobby unit/build suite and validate each packaged/mirrored source hash.
3. Run a full Code OSS compile only when the extension integration has changed core workbench source or built-in registration.

### Slice 1 â€” room reliability E2E (next mandatory V1 proof)

1. Add a repeatable two-client integration harness using separate identities and actual WebSocket sessions.
2. Exercise invite join, duplicate-name identity, presence, DM isolation, code-anchor metadata, approved diff, disconnect/reconnect, invite revocation and post-revoke denial.
3. Assert history is bounded and contains no invite token, unselected source, prompt, terminal output, or unapproved patch.
4. Exercise task ownership handoff approval/decline/disconnect with both client and host sessions.
5. Run manual IDE acceptance on two isolated profiles; keep unresolved transport/runtime limitations visible.

**Exit:** repeatable automated transport tests plus a recorded manual two-IDE pass for the acceptance cases above. No shared-edit or public-internet claim is implied.

### Slice 2 â€” read-only IDE capability registry

1. Define versioned capability IDs and typed JSON inputs/results for workspace list, selected file read, diagnostics, Git status, task read, room presence and agent status.
2. Centralize registration, capability negotiation, actor/workspace scoping, approval, cancellation, structured audit and response-size ceilings.
3. Adapt one existing read-only tool through the registry without changing the user-visible approval contract.
4. Add tests for unknown capability, invalid input, wrong workspace, denial, cancellation, oversized output and audit redaction.

**Exit:** one real provider tool call goes through the registry, displays provenance, cannot exceed its grant, and records allow/deny/result without sensitive content.

### Slice 3 â€” agent protocol feasibility

1. Implement a fake ACP v1 agent conformance fixture; negotiate capabilities and exercise session create/prompt/update/cancel/permission denial.
2. Decide whether ACP adapter adds value alongside the IDE language-model provider. Do not duplicate provider integrations without a clear need.
3. Keep MCP as an independent tools/resources adapter with separately tested auth transports; defer remote OAuth until the service architecture is chosen.

### Slice 4 â€” Studio Morph v1

1. Build a structured, allowlisted appearance/layout profile type with schema version, before/after preview, apply, undo and export.
2. Support settings and built-in theme contributions first; label fork-only capabilities explicitly.
3. Add WCAG contrast checks, reduced-motion support, persistent rollback snapshot and safe fallback when a profile references unavailable settings.
4. Prompt-to-profile generation may propose values but cannot write arbitrary settings or source code. Require review before apply.

### Slice 5 â€” Sessions, widgets and Packs feasibility

Prototype separate small spikes: WebRTC voice signaling/relay; a bounded widget host with fake data and no ambient file/network access; and an allowlisted theme-only Pack archive. Each spike needs its own abuse test before roadmap promotion.

## Current implementation update: 0.53.0 task ownership handoff

Implemented in Team Lobby:

- Added a task action and `Team Lobby: Hand Off Task` command.
- The assigned owner can request an unfinished task transfer to another connected participant and add bounded reason/context.
- A client request is not applied until the host approves. The host re-reads the task and connected participants after the approval dialog and checks owner/status/connection again.
- Host-initiated transfers use the same task and target checks.
- Approved, declined and stale/disconnected decisions are recorded in room activity with the task and reason; a denied/expired transfer leaves ownership unchanged.
- Added policy tests for assigned owner, target connectivity, self-target, completed/missing tasks.

**Verification:** `npm.cmd test` in `extensions/team-lobby` builds TypeScript and passes 56/56 tests. `npx.cmd --yes @vscode/vsce package --no-dependencies` produced Team Lobby 0.53.0; SHA-256 is `27329e860856cefb3aec71492477fc092bda10256e15d88338f6204b9953f6a7`. The built-in Code OSS manifest references the same version, VSIX path and hash. IDE two-client approval/disconnect runtime remains unverified. A full Code OSS `npm.cmd run compile` was attempted, but repository-wide typechecking currently fails with duplicate VS Code declarations between `src/vscode-dts/vscode.d.ts` and `node_modules/@types/vscode/index.d.ts`; the extension build itself passes.

## Not claimed / not yet delivered

No claim is made that Forge supports voice calls, mobile control, a managed hosted room service, MCP integration, prompt-driven full-workbench Morph, widget sandbox, Workshop, Forge Packs, cross-agent Mesh, automated deadline dispatch, or production external collaboration. Shared editing, file-and-line follow, read-only capabilities and ACP Ask/Plan have source and test coverage, but separate IDE/provider acceptance remains open.

## Current implementation update: 0.54.0 workspace file link containment

The current release validates room file links and comment anchors using resolved filesystem paths inside the selected workspace. It rejects parent traversal, absolute paths, missing files, and symlinks that escape the workspace. No file contents are shared by opening a link.

**Verification:** The extension build and 58/58 focused tests pass in the handoff and built-in Code OSS copies. The full Code OSS compile has now passed with zero errors. Its shared `extensions/tsconfig.base.json` maps `vscode` to this checkout's `src/vscode-dts/vscode.d.ts`, avoiding accidental resolution to a neighboring `node_modules/@types/vscode`. VSIX SHA-256: `55691c6c25775c77a2ee7b90027a72359a15d0d4df762e61e1c1ea67eceb0430`. The earlier 0.53.0 compile-failure note above is historical and superseded by this successful compile. Manual IDE runtime link-opening remains unverified.


## Current implementation update: 0.55.0 validated path reuse

The 0.54 workspace containment check now passes its canonical real path directly to the IDE open APIs, removing a symlink swap window between validation and use. 
pm.cmd test passes 58/58 in both handoff and Code OSS copies, webview syntax is valid, and the VSIX is synchronized with product.json. Runtime IDE validation remains open.


## Current implementation update: 0.56.0 LAN transport integration

Moved wire framing, version gating, handshake timeout, authenticated-session checks, and stale-socket-safe disconnect handling into `roomTransport.ts` and wired the extension host through it. Added a real local WebSocket integration suite for two clients: join snapshots, presence/post updates, task-owner policy, disconnect, stable-identity reconnect, and task history. Negative invite and protocol-version cases are covered. 60/60 tests pass in both extension copies. VSIX SHA-256: `95f3b2c4387f579524a19bbf4c94b65e2020747c0bd13b403901ed752d1147b9`. This gives repeatable process-level transport evidence; it does not replace two-machine IDE/TLS runtime acceptance.


## Current implementation update: 0.57.0 trusted-LAN boundary

Unencrypted hosting no longer binds to every network interface. It binds to a private IPv4 interface, or to loopback if there is no private LAN interface. A configured public invite address now requires both trusted TLS certificate and private-key paths. A WSS host still depends on valid certificate-chain checks at each client. Build and 63/63 focused tests pass in both copies. Package SHA-256: `cd8bafd311763ae5906ac9cdeb3aa6e5806ddd96a9915ac0689db38f051cc33d`. This reduces accidental public plaintext exposure; router/firewall configuration and multi-machine deployment remain external acceptance checks.


## Current implementation update: 0.58.0 canonical AI context paths

Workspace file selection, project guidance, selected Agent Skills, and provider read/search tools now resolve targets through filesystem realpath and read using the canonical workspace URI. This prevents outside-workspace symlink reads and catches sensitive files hidden behind ordinary-looking aliases. File picker, direct model reads, and project guidance retain explicit confirmation for likely sensitive targets; sensitive skill files are omitted. Build and 65/65 tests pass in both copies. Package SHA-256: `1dd732c13e64b3c3e33382f6f6ab9ec8f5526258d408024b264a10806e1497b3`. IDE/provider runtime and non-local URI schemes remain unverified.

## Team Lobby 0.59.0 — verification command checkpoint evidence

When a user runs the configured workspace verification task after applying a Build proposal, the bounded single-line command is recorded in its local rollback checkpoint in redacted form, with exit code and timestamp. Review bundles label whether that evidence is local-only or was explicitly shared with the room. Room activity receives command text only after the user chooses **Run and share command**; **Run privately** remains local. Overlong or multiline commands are omitted. Credential masking is heuristic and cannot guarantee removal of every secret.

**Verification:** TypeScript build and 66/66 focused tests pass in the handoff and Code OSS extension copies; webview JavaScript syntax check passes. The 0.59.0 VSIX is synchronized with the Code OSS built-in manifest. SHA-256: `a29a1f42e8675fa29d9e7082a842ed3ae99b41bda14c71043c2ce277ed725c7c`. IDE runtime approval and room-sharing flows remain unverified.

## Team Lobby 0.60.0 — multi-check verification evidence

After applying Build, users can select up to ten configured verification tasks and run them sequentially. Each check keeps its existing per-task execution confirmation; failures, exit codes, missing process exit codes, timestamps, and sanitized commands are added to the local checkpoint. The checkpoint retains the most recent twenty results, and linked room activity receives each result separately; command text remains gated by the existing workspace setting and explicit per-run share choice. A task launch error is recorded as exit code 1. TypeScript build and 66/66 focused tests pass in both copies; webview syntax check passes. VSIX SHA-256: 13385465b032d93740816d91b4f18543c9a5c90c17d92041fc9c779eb973f577.
## Team Lobby 0.61.0 — Build workspace tool provenance

Read-only workspace tool calls in Build now leave bounded local evidence containing tool name, returned workspace-relative source paths, returned character count, outcome, and timestamp. Search queries and returned file/search text are never added to the evidence. The most recent twenty entries survive opt-in Build restart recovery and are carried into the applied checkpoint and Markdown review bundle; export confirmation discloses the included path metadata. The new helper tests verify source extraction, query/content exclusion, outcome labels, and restored-state bounds. TypeScript build and 70/70 focused tests pass in both copies; package SHA-256: `d973184abf3d10c18310076f13cfd17ad139ffe510467907c8c91f67e2201532`.

**Current package state (21 October 2026):** Team Lobby 0.102.0 adds approved active-debugger stack inspection. See `FORGE_AI_NATIVE_MASTER_AUDIT.md` and `FORGE_AI_NATIVE_EXPANSION_RECONCILIATION.md` for the refreshed current requirements map and gaps; this file's earlier version-by-version matrices are historical snapshots.

**Current package state (21 October 2026):** Team Lobby 0.103.0 adds consented ACP session continuation when the configured agent advertises v1 `loadSession`. The current audit and acceptance gates are summarized in `FORGE_AI_NATIVE_MASTER_AUDIT.md`; historical matrices above are not the current inventory.

**Current package state (21 October 2026):** Team Lobby 0.104.0 adds exact-consent access to captured terminal output, consistent with the expansion brief's explicit-share requirement. The current prompt reconciliation remains in `FORGE_AI_NATIVE_MASTER_AUDIT.md`; prior audit matrices are historical.


**Current package state (21 October 2026):** Team Lobby 0.105.0 adds a Build-only exact-match text edit with full diff preview, mandatory per-call approval, stale-document rejection, unsaved/undoable application and content-free audit metadata. Both extension copies pass TypeScript checks and 174/174 tests. VSIX SHA-256: `838583216E620109596DEF720D218B87E551DF6D360A6751EF1D6E6AED346E41`. Full provider, UI interaction and broader staged roadmap remain open.

**Current package state (21 October 2026):** Team Lobby 0.106.0 adds checkpoint-backed rollback to exact approved Build edits. It reuses the standard conflict guard to protect post-edit changes. Both extension copies pass TypeScript checks and 175/175 tests. VSIX SHA-256: `D7896744609ED694E7E9A199C6D1E2EAF5B7C3BB900D00C6DA0B2CC38E630AF2`, synchronized with product.json and both handoff package copies. Live IDE interaction and provider acceptance remain open.

## Current implementation update: Team Lobby 0.107.0 workspace inventory

Added the native workspace_list capability for Ask, Plan, and Build. It enumerates up to 250 non-sensitive relative paths in the selected local workspace root, omits common generated folders, validates canonical containment, reads no file contents, supports cancellation, and stays within 12,000 result characters. Tests cover the typed capability boundary and formatter; real-provider runtime remains unverified.

**Verification:** both extension copies pass 
pm.cmd run check and 178/178 tests. The 17-file VSIX SHA-256 is 6EF2EC09E51E25884577C159AA19382A947C682C6F1487D080C9B6ACF8F0FBFA; it matches the built-in product manifest and all handoff package copies.


## Current implementation update: Team Lobby 0.108.0 editor navigation

Added the native editor_open action to the shared capability registry. It resolves the selected-root file through canonical containment checks, rejects sensitive paths, validates optional line numbers against the opened document, and requires a dedicated confirmation before changing the visible editor. The tool result carries no file contents.

**Verification:** both extension copies pass TypeScript checks and 179/179 tests. The 17-file VSIX SHA-256 is 6EF2EC09E51E25884577C159AA19382A947C682C6F1487D080C9B6ACF8F0FBFA; the manifest and package copies match. Live provider interaction remains open.
