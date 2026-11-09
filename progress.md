# Forge Room — progress and complete working plan

**Updated:** 6 November 2026. **Active product:** native Rust IDE, Developer Preview `0.110.0-alpha.2`, Windows 11 x64. **Current phase:** Phase 1, visible Windows baseline. **Owners:** @ZayedBH and @Atul013.

**9 November local feature slice:** The native left rail now opens Workflows, Models, Run & Debug, Tests, Extensions, Settings, and Accessibility panels. Rust, Node, and Python marker files expose a small set of explicit project commands; pressing a command sends it to the integrated terminal. Debugger adapters, AI model providers, and extension execution remain unavailable and are labelled as such. Source inspection complete; this shell has no `cargo` command, so Rust compilation and desktop acceptance are pending.

**9 November follow-on slice:** Workflows, Tests, and Run now discover safe-name npm scripts from `package.json` and fixed Go/Maven commands in addition to Rust/Python commands. The explorer has a New Folder action and Copy Path context actions for files and folders. These are source changes awaiting compilation and desktop acceptance.

**9 November integration:** PR #23 merged the project panels and explorer actions. Its last Windows CI run was still running when the merge completed; source checks, formatting, and graphical compilation had passed in the prior run, while a parallel Git test temp-directory collision was fixed in the final commit. Desktop acceptance remains open. A follow-on Room snapshot export slice is under review.

**9 November CI and follow-on:** The final PR #23 Windows verification passed. PR #24 merged local Room snapshot export after Windows verification passed. A further bottom-panel slice connects Output to the current terminal screen and Problems to visible error/warning lines; these are terminal-derived summaries, not language-server diagnostics. Debug Console and Ports remain disabled until adapters exist.

**9 November explorer follow-on:** PR #25 merged the bottom Output/Problems views after Windows verification passed. PR #26 merged New file here from Explorer folder menus after Windows verification passed. It opens a save dialog in that folder and creates the chosen file only if it does not already exist. Desktop interaction remains unverified.

This is the current project status, assignment, and next-action document. Historical documents remain available for requirements and evidence, but their older Code OSS status statements do not describe the native app. **Done** means code or an action is complete; **CI passed** means automated checks passed; **Needs hands-on check** means actual product behavior remains unverified; **Planned** means it is not implemented.

### Navigate this document

1. [Repository and current native state](#repository-and-migration-state)
2. [Feature inventory and original detailed specification](#complete-product-feature-inventory-and-migration-status)
3. [Delivery contracts for each product system](#delivery-contracts-for-the-major-systems)
4. [The division between @ZayedBH and @Atul013](#one-ordered-workflow-split-equally)
5. [Later phases after the first release](#beyond-the-first-release-candidate)
6. [Slice order, boundaries and risks](#slice-by-slice-execution-order)
7. [End-to-end acceptance scripts](#end-to-end-acceptance-scripts)
8. [Working rules, verification and next actions](#working-rules)

## Repository and migration state

- Original repository: `ZayedBH/forge-room-ide`. Its `main` now contains the native Rust replacement through PR #3. The Code OSS handoff at `b669705` is history.
- Supplied fork: `Atul013/forge-room-ide`. The active Rust source imported here is `native-rust-ide` at `5fadb8f40bf42325e8a9a4e17ec3a59eaa1adea7` (Developer Preview `0.110.0-alpha.1`). The fork also has an open early-preview PR; that branch is older than the imported native branch.
- Imported from `atul/native-rust-ide`: `native/`, `packaging/`, `design-assets/`, `.github/`, `COLLAB.md`, `docs/native/`, and `docs/design/`. The native README is now the entry point. The previous README is preserved at `docs/legacy/README_CODE_OSS.md`; existing documents, `PROJECT_PROGRESS.md`, and planning files remain in the checkout.
- The former Code OSS `extensions/`, `services/`, `releases/`, and `.build/` directories have been restored at their original workspace paths, including local edits, new modules, packages, and embedded documents. They are retained as migration references; the active application is `native/forge-room-app`. The source, tests, and documents in the first three folders are restored to Git. Generated `.build/` output and ignored VSIX packages remain available locally, outside Git.
- The native replacement was merged into `ZayedBH/forge-room-ide` `main` through [PR #3](https://github.com/ZayedBH/forge-room-ide/pull/3) on 2 November 2026 (merge commit `f6f0a7d`; follow-up ledger commit `9c382af`). The fork is remote `atul`; `origin` remains the original repository. Windows PR CI passed Rust formatting, locked dependency resolution, checks, tests, and clippy for both core and graphical app. The PR packaging job was skipped by workflow design. Installer packaging and hands-on Windows acceptance remain open. Earlier local edits to historical documents remain uncommitted.

## Product goal and hard constraints

Build a fast, native, customizable IDE with a complete daily coding loop and Forge Room collaboration. The current app uses Rust, egui/eframe, wgpu, and a separate `forge-room-core` policy crate. No Electron, webview, or bundled Node runtime. Windows 11 x64 and per-user installation are the first supported target. Keep solo/offline use, private sharing defaults, owner-approved agent actions, recoverable changes, Git as durable source of truth, accessibility, and honest feature/verification labels. The egui editor is an early implementation choice; benchmark it against other Rust editor components before committing to it for large files.

## Implemented in the imported native branch

| Area | Current implementation | Evidence boundary |
|---|---|---|
| Editor shell | Folder explorer, multiple tabs, native dialogs, UTF-8 open/edit/save, dirty prompts, shortcuts, status bar | Prior branch progress reports real executable checks; recheck in this checkout. |
| Editing/navigation | Syntax colors, line numbers, current line, breadcrumbs, command palette, quick open, go to line, find/replace, file search | Large-file performance, external edits, bracket matching, indent guides, and minimap remain open. |
| Git | Status, diff, stage, commit, branch, sync/pull/push/fetch, stash, commit graph, file history, inline blame | Conflict resolution, line history, rebase/cherry-pick, and richer remote/tag views remain open. |
| Terminal | One integrated PowerShell session through ConPTY | Scrollback UI, selection/copy, tabs/splits, shell picker remain open. |
| Room | Local per-project people, agents, tasks, ownership, dependencies, handoffs, evidence, activity; policy in `forge-room-core` | No native network relay, invites, presence, chat, shared documents, or agent provider clients yet. |
| Appearance | Six Studio themes with live preview and saved choice; newer Forge default visual shell | Profile import/export, custom palettes, accessibility and display-scale acceptance remain open. |
| Installer | Per-user Windows installer, upgrade/reinstall/downgrade rules, themed uninstaller | CI packaging, actual install/upgrade/uninstall, DPI and high-contrast checks need fresh evidence. |
| AI panel | UI layout and composer | No functional provider backend. Do not present it as working AI. |

The older Code OSS work has a broader collaboration and AI feature inventory. Those features are **not native ports** simply because their sources are present. The full intent is retained below; `PROJECT_PROGRESS.md` and `docs/product/PRODUCT_SPEC.md` carry the detailed requirements and old implementation evidence.

## Complete product feature inventory and migration status

This inventory restores the product scope that the short roadmap hid. **Legacy** means the Code OSS implementation or prototype is available in the restored folders. **Native** means behavior is integrated into the Rust app or core. **Planned** means the native product still needs it. The order column maps each area to the roadmap phase where it should be handled; it is not a claim that all items in that phase are already scheduled as one slice.

| Product area | Intended experience | Current native state | Legacy reference | Roadmap |
|---|---|---|---|---|
| Core IDE | Project and multi-root workspaces, fast editing, search/navigation, language tools, terminal, tasks/tests, debugger, Git, accessibility, offline solo work | Editor, search, terminal and broad Git foundation; LSP, DAP, task runner and deeper editing absent | Code OSS product and Team Lobby workspace tools | 1, 3, 4 |
| First-run room flow | Create/join/recent rooms, project/repo binding, privacy and sharing preview, LAN/service selection, duration and milestones | Local project room only | Team Lobby host/join and invite flows | 5 |
| Room membership and lifecycle | Invite-only identity and roles, participant/agent ownership, moderation, pause/archive/export, revocation, reconnect | Local identities, tasks, handoff and room persistence | Team Lobby protocol, room-service account/invite controls | 5 |
| Room UI | Lobby, members, tasks, chat, agents, activity, build/demo, connection and error states, keyboard access | Local Room view; several panels are placeholders | Team Lobby webview | 5 |
| Communication | Public chat, direct messages, mentions, reactions, threads, decisions, search, anchored comments with privacy | Not ported | Team Lobby chat, DMs, threads, decision and search modules | 5 |
| Tasks and hackathon timing | Ownership, status, priority, estimates, dependencies, deadlines, milestones, countdown, critical path, reminders, handoff and readiness | Basic tasks, dependencies, acceptance, handoff and evidence; advanced timing not ported | Team Lobby task and countdown modules | 5 |
| Activity and awareness | Attributed event feed with live/delayed/offline labels, evidence links, since-last-seen and retention rules | Local activity | Team Lobby room history and activity modules | 5 |
| Work visibility | Consent-based paths, saved diffs, optional unsaved buffers, terminal/debug output, previews and artifact links | Local Git diffs only | Team Lobby opt-in diff and sharing features | 5 |
| Spectate and shared edit | Read-only follow, scoped edit access, revocation, tested concurrent editing or reviewed patch fallback | Not ported | Team Lobby shared-document prototype | 5, later |
| Agent participants | Human-owned agents with provider, runtime, permissions, task, location, activity, approvals and stop controls | Local agent identities only | Team Lobby agent roster and ACP/MCP integration | 6 |
| AI context and modes | Ask, editable Plan, approved Build; selected files, symbols, diagnostics, terminal/debug results, room context and skills | Assistant UI layout only | Team Lobby Ask/Plan/Build and context tools | 6 |
| AI tools and evidence | Typed native capabilities, bounded outputs, exact approved edits, test results, checkpoints and rollback | Not ported | Team Lobby capability, review and rollback modules | 6 |
| Autonomy and isolation | Per-task worktrees, distinct action grants, stop/reclaim, budgets and execution isolation | Not ported | Team Lobby worktree and approval controls; no reliable OS sandbox | 6 |
| Studio appearance | Curated themes, readable profiles, preview, import/export, contrast and reduced motion | Six themes and preview; profiles and deeper controls absent | Forge Room Studio source and theme bundles | 3, 6 |
| Forge Morph | Prompt-to-typed appearance/layout proposal with preview and undo | Not implemented | Theme/profile primitives only | Later |
| Forge Widgets | Project-aware generated panels with typed data bindings, sandbox and rollback | Not implemented | Product concept only | Later |
| Forge Workshop | Reviewed authoring of commands, tools, widgets and extensions | Not implemented | Product concept only | Later |
| Forge Packs | Portable versioned bundles of appearance, tools, workflows and room templates | Not implemented | Product concept only | Later |
| Skills-first intelligence | Discover, inspect, select and safely use procedural `SKILL.md` knowledge | Not ported | Team Lobby local skill reader | 6, later |
| Forge Sessions | Voice, optional screen share, follow, and controlled shared session actions | Not implemented | Product concept only | Later |
| Forge Pocket | Mobile companion for rooms, approvals and read-only project awareness | Not implemented | Product concept only | Later |
| Forge Mesh / Pipeline | Task/event graph, agent handoffs, bounded context, gated test/deploy pipeline | Not implemented | Task dependency foundation | Later |
| Deadline rescue | Recommendations, owner opt-out, optional narrow preauthorized dispatch | Not ported | Team Lobby local triage/reminders; no production scheduler | Later |
| Conflict and contract awareness | Detect overlapping files/API changes and alert affected owners | Not ported | Limited legacy diff/path awareness | Later |
| Demo and shipping | Authorized preview, readiness evidence, last-hour triage, submission, archive/export | Not ported | Team Lobby readiness and export modules | Later |
| Cross-IDE ecosystem | Capability-tier adapters, CLI/guest clients, truthful sync support | Not implemented | Code OSS extension target only | Later |
| LAN/cloud service | Authenticated encrypted transport, invites, persistence, operations and retention | Not ported | `services/room-service/` self-hosted prototype | 5, later |
| Replay and deferred ideas | Optional activity replay after core; other speculative concepts remain low priority | Not implemented | Product ideas ledger | Later |

**Migration rule:** preserve a legacy behavior only when its user outcome is still wanted. Port its policy or data model into Rust, build the native interface/transport, and test it end to end. Do not move a webview or Node extension host into the Rust runtime merely to claim parity.

## Detailed feature specification from the original plan

The table below carries forward the original feature inventory without dropping individual ideas. Its right column is the **25 October 2026 Code OSS snapshot**, not the current Rust status. Use the native-status table above when deciding what is implemented today. Product requirements in the middle column remain in scope unless @ZayedBH and @Atul013 explicitly change them.

| Area | Full intended feature set | Legacy Code OSS snapshot (25 October 2026) |
|---|---|---|
| Core IDE | A professional Code OSS desktop product; project and multi-root workspaces; editing, search, navigation, language tooling, run/debug, tasks/tests, terminal, Git, extension ecosystem, keyboard/accessibility, offline and solo use. | Code OSS fork and packaged Forge extensions exist. Continue repeatable product builds and test the real end-user loop. |
| First-run room flow | Rooms/Live entry and commands; create/join/recent/return; minimal progressive setup; bind project/repo/branch optionally; room name, privacy, icon and mode; choose LAN or service availability; choose duration/timezone, milestones and submission details; preview sharing before start. | Room and invite foundations exist. Full product-flow and manual UX acceptance remain. |
| Room membership and lifecycle | Invite-only default; high-entropy codes/links with expiry, use limits, rotation and approval; identity and roles; human/agent ownership; member moderation; create/join/leave/pause/archive/restore/export; host-authoritative countdown and controlled pause/extension; reconnect and presence leases. | Significant host/client room, identity, invite, presence, timer and history foundation exists. Hosted-service operations and full multi-client acceptance remain. |
| Room UI | Room/project selector; connectivity state; member count, time, sharing indicator and alerts; lobby/team, chat, tasks, agents, activity and build/demo surfaces; dock/collapse; loading, empty, offline, error and reconnect states; accessible focus and reduced motion; do-not-disturb and notification controls. | Team Lobby views and Studio exist. A broad UI overhaul and all interaction states are not complete. |
| People communication | Room chat, direct messages, mentions, reactions, threads and searchable decisions; contextual messages anchored to code/diff/task/preview/test; messages to an agent owner stay distinct from direct agent commands; privacy for DMs. | Chat, direct messages, anchored comments, local room pulse search, optional task-linked messages, toggleable reactions, parent-linked public/DM threads, participant-local retained message search, and explicit attributed decision records exist. New guests start with recent room history; richer contextual integration remains future work. |
| Tasks and hackathon timing | Fast task capture; owner, priority, effort estimate, due time, status, blocker, dependency, linked artifact and handoff; 12/24/36/48/custom countdown; milestones and warnings; audit-controlled pause; critical-path and time-aware triage; submission/demo URL and readiness checklist. | Tasks, ownership/handoff, dependencies, deadlines, worktrees, host-controlled countdown and up to 20 shared milestones exist in slices. Priority, optional effort estimates, estimated critical path, countdown and milestone feasibility, milestone urgency warnings, opt-in countdown alerts and evidence-bounded readiness report are implemented; external readiness integrations remain open. |
| Live awareness and activity | Attributed timeline for membership, task, agent, file, tests, terminal/build, Git, decisions, contracts, conflicts, preview, milestones and handoff; immutable IDs, timestamps, sequence, schema, visibility, source and idempotency; “since last seen” with evidence links; real live/delayed/offline/unsupported labels; rate limits and no keystroke surveillance. | Room event history, Git/activity, diff snapshots and collaboration events have a substantial base. Complete event coverage and cross-client acceptance remain. |
| Work visibility | Permissioned active-file metadata, saved diffs, optional unsaved buffer, cursor/selection where supported, conflict awareness, preview and artifact links before Git commit; sensitive-path filtering; explicit consent, bounded transfer, revision, reconnect and deletion rules. | Opt-in diff sharing, comments, shared-document prototype and capability levels exist. Two-IDE and two-machine validation, broader conflict handling and clear tier reporting remain. |
| Spectate and shared edit | Read-only follow without stealing local navigation; ask for edit access; owner grants file/session/time scope; revocation stops writes; tested CRDT/OT, cursor awareness, reconnect, conflict semantics and collaborative undo; safe patch fallback for unsupported clients. | Shared-document capability foundation exists. Safe multi-client IDE accept/revoke/reconnect and general concurrent editing are not verified. |
| Agent participants | Owned agent roster, provider/runtime, local/remote location, capabilities and permissions, session/task/worktree, availability/busy/blocked, activity/progress, logs and artifacts, pending approvals, abort/retry, review and handoff. Keep private chain-of-thought private. | Provider-neutral Ask/Plan/Build, ACP Ask/Plan and agent identity/activity foundations exist. Broad agent fleet orchestration and external provider acceptance remain. |
| AI context and modes | Distinct Ask (understand only), Plan (editable proposal), Build (approved implementation); choose files, folders, symbols, selection, diffs, room tasks/decisions, diagnostics, terminal results, debugger stack and skills; show what will go to which provider; selected-project-root scope and bounded outputs. | Significant Ask/Plan/Build and context tooling exist. The 0.107 file inventory, 0.108 editor navigation, 0.109 opt-in bounded room-task snapshot, and 0.110 local recommendation-only task triage close four specific gaps. Manual provider/IDE acceptance remains. |
| AI tools and evidence | Typed IDE capability registry; provider/model choice; approval levels; search/list/read/navigation; diagnostics; terminal inspection; debugger stack; task/test discovery and explicit test execution; exact edits with diff; checkpoints/rollback; session continuation; plan export/handoff; verification records and review bundle. | Many of these are implemented as individually tested capabilities through 0.108. Automated tests exist; live provider and IDE operation still require acceptance. |
| Autonomy and work isolation | Worktree per task; separate “where changes go” and “what actions are allowed”; pause/redirect/stop/reclaim; budgets/time/resource limits where available; retries and idempotency; no implied sandbox; subprocess isolation before untrusted autonomous Build. | Worktree management and approval controls exist. ACP/MCP/local extension processes may still have ambient OS permissions; a reliable execution sandbox is not delivered. |
| Forge Room Studio / appearance | Curated theme moods including anime-inspired, retro terminal, cozy, minimal and high contrast; coordinated workbench/editor/icon/type/atmosphere; live preview, keep/restore, personal and project profiles; import/export; contrast checks; optional non-code artwork and opt-in motion; reduced-motion support; no hidden code or permission changes. | Six curated themes, profile import/export, reversible preview, contrast checks and opt-in motion exist. Broad generated/deep workspace morphing and artwork system remain future ideas. |
| Forge Morph | Natural-language request to proposed appearance/layout changes; typed manifest; preview, apply, undo/restore; color, typography, density, layout, docks, backgrounds, optional motion; original designs inspired by moods rather than copied trade dress; supported Code OSS hooks for workbench shell changes. | Constrained theme/profile primitives exist; prompt-to-layout morph and arbitrary workbench mutation are not implemented. Extension API limits are documented; deeper changes need deliberate fork-owned work. |
| Forge Widgets | Generate useful project-aware panels/dashboards from approved data; typed UI and data bindings; capability requests; schema/API/CPU/memory/network validation; clearly labelled sample-data preview; install, version/configuration, crash recovery, disable/uninstall and rollback; sandbox without ambient APIs. | Not implemented. Requires a restricted runtime, capability grants and resource limits before generated code can execute. |
| Forge Workshop | Create reusable commands, mini-tools, widgets and extension contributions with AI assistance; inspect source/manifest, dependency and permission scope; isolated build/test, preview, approval, install/update/uninstall and audit. | Not implemented. Keep this one authoring surface rather than a branded folder for every small feature. |
| Forge Packs | Portable, versioned bundles of appearance/layout, licensed assets, widget configuration, tools/extensions, Skills references, room templates and workflows; manifest, dependencies, signatures/trust, selective install, compatibility checks, update/uninstall and rollback. | Not implemented. Start local and declarative; do not bundle executable code without sandbox and provenance. |
| Skills-first intelligence | Discover local and trusted catalog Skills using `SKILL.md`; validate name/description/location/size; license, maintainer, recency and script/network review; select per request; map required native tools; synthetic sandbox evaluation; enable/disable, authoring and portable references. Examples: code review, test planning, context summary, debugging, docs, handoffs, budget guidance and demo prep. | Local standards-oriented reader and selected-skill flow exist. It does not execute scripts or authorize tools. Trusted catalog, sandbox testing and audited install are future work. |
| Forge Sessions | Room voice with device permission, mute/deafen/push-to-talk, speaker state, reconnect/QoS and optional screen share/recording only with consent; tightly integrated workspace follow, agent progress, permitted debugger/terminal output and requested shared control. | Voice calls and mature shared session control are not implemented. Do not use screen share to imply IDE-level shared state. |
| Forge Pocket | Phone-friendly companion for rooms and calls, notifications, approvals, task/agent coordination, read-only diffs/diagnostics, limited session access; clear whether actions require the desktop to remain running or use an explicitly hosted runtime. | Not implemented. Requires its own mobile and authenticated service architecture. |
| Forge Mesh / Pipeline | Native versioned task/event graph; agent-to-agent and team handoffs with bounded relevant context; ownership/dependency/contract validation; stale-context and impact alerts; gated CI/test/deploy workflows; retries, idempotency, result evidence and deadlines. | Room tasks/dependencies provide a base. Autonomous cross-agent routing, durable scheduler and pipeline execution are not implemented. |
| Deadline dispatch and rescue | Recommend by default when work is blocked or due; eligible agent selection by skill/capability/language/cost/runtime/resources; optional preauthorized queue/execute with narrow scope, budget/time, stop/escalation and duplicate prevention; owner can opt out/reclaim; no forced takeover. | No production scheduler or automatic agent dispatch. Keep recommendation separate from execution approval. |
| Conflict and contract awareness | Detect same-file/worktree overlap, interface/schema/API contract changes, impacted owners/tasks, stale agent context and integration risk; show source/confidence and ask affected people to acknowledge; offer preflight integration checks. | Limited file/diff awareness exists; impact radar and contract board are future work. |
| Demo, preview and shipping | Shared authorized preview; comments/reproduction artifacts; test/build/deploy signals; demo-readiness checklist; last-hour Ship/Panic mode for P0 blockers, must-have vs nice-to-have and merge risk; smoke tests and actual submission evidence; handoff bundle and archive/export. | Task timing, test capability, room exports and some verification evidence exist. Shared preview theater, deployment integration, endgame checklist and Panic Mode are not complete. |
| Cross-IDE ecosystem | Adapter SDK and supported IDE/CLI clients; negotiate capability tiers A full collaboration, B saved-buffer/diff, C task/Git/activity; browser guest reviewer; accurate sync/support and revocation behavior. | Current extension targets Code OSS/VS Code APIs; cross-IDE SDK and client acceptance are not implemented. |
| Service / LAN / cloud | Private LAN/self-hosted room first; authenticated encrypted online relay as separate tier; room authority, discovery, NAT/relay decision, durable identities, expiring/revocable invites, persistence, account/key recovery, tenant policy, retention/deletion, observability and independent security review. | Self-hosted HTTPS/WSS service includes account/invite controls, per-address API rate limiting, per-connection relay byte/frame limits, per-address HTTP and WebSocket upgrade limits, and durable control metadata. It is not a production managed internet service; separate-machine acceptance, global bandwidth controls, operations and independent review remain. |
| Replay and deferred concepts | Optional activity replay may be explored after the core flow. Keep Replay lower priority. Do not split low-value concepts such as Forge Twin, Ghost Developer, Forge Universe or Forge What-If into separate flagship products unless the user revisits that direction. | No Replay implementation. Deprioritized concepts are deliberately out of the active roadmap. |

## Delivery contracts for the major systems

The following sections translate the original briefs into concrete native-product behavior. They are **requirements and acceptance targets**, not claims that the Rust app already implements them. The status tables above identify what is present today. Each group should be delivered in smaller reviewable slices, and every slice needs automated evidence plus a visible or multi-client trial when relevant.

### A. Everyday IDE foundation

**Experience to deliver**

1. Open a single file, folder, project and multi-root workspace; restore the last session without silently opening or sharing private content. Explorer operations include create, rename, delete, refresh and reveal, with clear confirmation for destructive actions.
2. Edit with reliable undo/redo, selection, multi-cursor, indentation, bracket matching, find/replace, go to line, symbols, syntax colors, diagnostics and accessibility. Non-UTF-8, binary, very large and externally changed files need explicit behavior and safe failure messages.
3. Run a normal language workflow through native LSP: completion, hover, definitions, references, rename and diagnostics. Track which language server produced each result and show when it is unavailable.
4. Provide terminal, tasks, test discovery/execution, Problems and Debug views backed by real sources. DAP should support breakpoints, stepping, stack and variables only after a supported adapter is connected.
5. Keep Git authoritative: status, stage, diff, commit, branch, fetch/pull/push, conflict resolution, history, blame and review. Never auto-commit merely because the room observed a change.
6. Define extension tiers truthfully: declarative themes/grammars/snippets first, native language/debug servers next, optional constrained WASM extensions later. A VSIX filename is not proof its JavaScript code can run.

**Acceptance:** @ZayedBH and @Atul013 can complete the same edit–diagnose–test–review–commit path on two representative projects. A failed save, external edit, disconnected remote or missing language tool cannot silently lose work. Startup, idle memory, large-file interaction and keyboard access are measured.

### B. First-run room and membership

**Create:** offer Create Room, Join Room, Recent Rooms and Return to Room from a visible but dismissible entry. A room may bind to a project/repo/branch without requiring Git. The wizard asks only for room name, privacy, availability mode, optional icon, template and duration at first; advanced settings include milestones, timezone, submission URL, roles and a preview of what will be shared. Invite-only is the default. Room creation alone uploads no code.

**Join:** accept a high-entropy invite code/link or supported LAN discovery. Show room owner, project, network mode, role, local project binding and the client's actual capability tier. A participant can join read-only without a checkout. Joining never resets, pulls, switches branches or runs scripts without a separate action and consent.

**Lifecycle:** invite expiry, maximum uses, rotation, revocation and optional join approval need server-side enforcement. A host can leave, pause, end, archive, restore or export according to role policy. Presence expires by lease; disconnect does not delete files or artifacts. The countdown uses authoritative start/end times and keeps advancing across app closure unless an authorized pause or extension records an audit event. Make host-offline behavior explicit rather than inventing continuity.

**Identity:** humans, human-owned agents and service/test runners are distinct actor types. Multiple clients and agents under one human must not inflate human headcount. Roster details may show owner, provider/runtime, location, capability tier, task and last seen only to authorized viewers. Moderation includes revoke invite, remove role, mute/kick and deny rejoin where supported.

**Acceptance:** two independent machines create/join/leave/rejoin a private room; expired or revoked codes fail; a disconnected participant returns without duplicate members or events; no project file is uploaded merely by joining.

### C. Room UI and communication

The editor remains the primary surface. A compact room chip shows connection state, participant count, timer, sharing state and actionable alerts. Dockable or collapsible areas cover Team, Chat, Tasks, Agents, Live Activity and Build/Demo. Each has loading, empty, offline, reconnecting, permission-denied and error states. Keep focus order, reduced motion, contrast, keyboard shortcuts and a do-not-disturb control.

Communication includes public room messages, direct messages, mentions, reactions, threads, decisions and search. A message can link to a permitted task, file, diff hunk, test, preview or agent run. Private messages must be filtered before delivery and before local indexing; search does not expose content the viewer never received. Talking to an agent's owner is distinct from issuing an agent command. Retention, export and deletion policies need visible rules.

**Acceptance:** two participants exchange public and private messages, link a decision to a task, reconnect and search only authorized history. A guest cannot retrieve another pair's DMs through history, search, export or a crafted protocol request.

### D. Tasks, timer, milestones and deadline rescue

Tasks need fast capture, owner, priority, optional estimate, due time, status, blocker, dependencies, acceptance criteria, linked files/artifacts and explicit handoff. A task can be reopened only under policy; completion requires its stated evidence rather than a status click. The host controls timer start/pause/extension; milestone changes are attributed and propagated. Presets may include 12, 24, 36 and 48 hours plus custom time. Timezone display must not change the authoritative deadline.

Readiness compares remaining estimated work, critical dependencies, time left, test/build evidence, preview and actual submission status. Unknown or manual results remain labelled unknown/manual. Alerts can recommend help for blocked or due tasks. A later policy may queue or execute an eligible agent only within preapproved task, file, time, cost and tool scope. Owner opt-out, pause and reclaim always remain available; expiry never auto-marks a task done.

**Acceptance:** a due-soon blocked task produces an explainable recommendation; unauthorized takeover or execution is rejected; a paused timer and changed milestone stay consistent after reconnect; the readiness view separates verified results from manual claims.

### E. Live activity, visibility and spectate

Events need stable IDs, actor, source, timestamp, sequence, schema version, visibility, task/file link, evidence link and idempotency key. Show real live/delayed/offline/unsupported labels. Activity may include membership, tasks, agent runs, file/diff changes, tests, terminal/build summaries, Git, decisions, contracts, conflicts, previews and handoffs. Never infer authorship from code style or claim private agent reasoning. No keystroke surveillance.

Sharing scopes are separate: presence, active-file metadata, path, saved diff, saved content, unsaved buffer, cursor/selection, terminal output, debugger data, preview and agent context. Default to minimal sharing; show recipient and destination. Apply sensitive-path filters, size bounds, revision tracking and retention. Authorized teammates can inspect useful uncommitted work before Git commit, but Git remains the merge/release record.

Read-only follow must not seize the viewer's own navigation. Request-edit is a separate flow with file/session/time limits. If safe shared-buffer editing is unavailable, use reviewed patch proposals and label the capability tier. True co-edit requires a tested consistency model, remote cursors, reconnect reconciliation, revocation that stops writes and collaborative undo semantics.

**Acceptance:** one person shares only a saved diff while keeping unsaved content private; the viewer follows and stops following; an edit request is granted for one file, then revoked; unauthorized content and stale writes are blocked. Two-client network tests verify event visibility and ordering.

### F. Human-owned agents and AI work

Every agent has a unique session ID, human/service owner, provider/runtime, task, project/worktree, execution location, capabilities, permissions and observed state. The roster distinguishes available, busy, blocked, offline and stopped. Show planned action, files changed, commands run, tests, logs/artifacts and pending approvals where the provider exposes them. Users can pause, redirect, stop, reclaim and review; provider limitations must be displayed honestly.

Ask reads approved context without edits. Plan creates an editable proposal and can export or hand off a structured plan. Build executes only an approved scope and yields a reviewable diff, checks, failures and remaining risk. The context picker can include selected files, symbols, selection, Git diff, diagnostics, bounded terminal output, debugger stack, room tasks/decisions and chosen Skills. In multi-root workspaces, every request and continuation stays pinned to the chosen root. Show what leaves the device and to which provider before transfer.

The native capability registry should separate read/search/navigation from terminal, tests, edits, network, secret access, delegation, merge and deploy. A grant is narrower than a request. Exact approved edits should be checked against revision and path. Worktrees isolate parallel task changes, but a worktree is not an execution sandbox; untrusted autonomous commands require a real process boundary and resource limits.

**Acceptance:** a real provider completes one Ask, Plan and Build path. An unapproved tool call is denied. Approved edits show an exact diff; tests show real results; rollback restores only that agent's change without deleting unrelated work. A disconnected or failed provider leaves an attributable, recoverable state.

### G. Forge Studio, Morph, Widgets, Workshop and Packs

Studio themes must coordinate workbench, editor syntax, icons, typography and optional non-code atmosphere. Users can preview, Keep or Restore, save personal/project profiles, import/export, and inspect contrast/reduced-motion behavior. Themes cannot silently alter permissions, keybindings, installed code or network access. Preserve the existing moods—Neon Ronin, Starship Deck, Moss Shrine, Ink & Ember, Paper Studio and Signal Guard—while allowing original future designs.

Morph turns a natural-language request into a typed appearance/layout manifest. Preview the exact changes to colors, density, fonts, docks, background and motion; validate support, contrast and performance; apply only after confirmation; provide undo/restore. A prompt is not permission to mutate arbitrary workbench internals.

Widgets are project-aware panels with typed UI/data bindings. The generator must identify requested sources, request only allowed capabilities, validate schema and forbidden APIs, bound CPU/memory/network, mark sample data as simulated, preview, install, monitor crashes, disable/uninstall and roll back. Workshop authors reusable commands, tools, widgets or extension contributions through source/manifest inspection, dependency review, isolated build/test and explicit install. Packs bundle versioned appearance, licensed assets, widget configuration, tools, Skill references, room templates and workflows with compatibility, dependency, provenance, selective install and rollback.

**Acceptance:** a theme/profile can be previewed and restored without changing trust settings; a Morph proposal can be rejected with no side effects; a generated widget cannot access data or APIs outside its grant; a Pack can be installed and cleanly rolled back.

### H. Skills, Mesh and Pipeline

Skills are portable procedural instructions, not security policy, memory authority, a scheduler, debugger or sandbox. Discover local or trusted catalog `SKILL.md` entries; inspect name, description, location, license, maintainer, scripts and network needs; select for a request; map required native tools; test with synthetic data before enabling. Keep install/update/disable and provenance visible.

Mesh is a versioned task/event graph that connects human and agent owners, dependencies, contracts and evidence. Pipeline orchestrates bounded handoffs and gated tests/build/deploy actions with retries, idempotency, cancellation, deadlines and verified results. Detect overlapping files and interface/schema/API changes; notify affected owners with source and confidence, ask for acknowledgement and offer preflight checks. Automation starts as recommendation. Queue/execute requires explicit narrow policy and must never force takeover, silently merge or deploy.

**Acceptance:** a backend contract change alerts the owning frontend task with an exact source link; a failed test yields an artifact and blocks the next gated action; an agent handoff carries only authorized context; duplicate events cannot trigger duplicate execution.

### I. Sessions, Pocket, cross-IDE and service operations

Sessions adds room voice with device permission, mute/deafen/push-to-talk, speaking state, reconnect and quality indicators. Optional screen share/recording/transcription requires separate consent, retention and revocation. Integrated follow, permitted debugger/terminal view and requested shared control have their own scopes; screen share alone is not IDE-level shared state.

Pocket is a phone-friendly companion for room presence, chat/calls, alerts, approvals, tasks, agent progress and read-only diffs/diagnostics. It must say whether an action needs the desktop online or an explicitly hosted runtime. Remote control needs fresh identity, scope and audit checks.

Cross-IDE participation uses negotiated tiers: A for tested full collaboration, B for saved-buffer/diff participation, C for task/Git/activity only. A CLI or browser guest may join only within its tested tier. Do not promise universal unsaved-buffer streaming from third-party editors.

The service starts with private LAN/self-hosted capability and expands to authenticated encrypted internet relay only with operational readiness. Define authority, discovery/NAT/relay choice, durable identities, account/key recovery, expiring invites, tenant policy, retention/deletion, quotas, observability, incident handling and independent security review. A responsive process or TLS connection is not multi-machine product acceptance.

**Acceptance:** room voice reconnects without unconsented capture; a Pocket approval is attributed to the correct person and scope; a tier-C client cannot request tier-A content; a separate-machine room survives disconnect and revocation under its stated network tier.

### J. Demo, shipping and deferred exploration

Build/Demo should show real build/test/deploy health, authorized preview, comments and reproduction artifacts, a readiness checklist and actual submission evidence. Last-hour Ship/Panic mode surfaces P0 blockers, must-have versus optional work, merge risk and approved rescue suggestions; it must not silently broaden permissions. Export a handoff bundle with decisions, current diffs, checks, unresolved problems and agent context, filtered by recipient visibility. Activity Replay is optional after the core workflow; speculative concepts such as Twin, Ghost Developer, Universe and What-If remain parked until user evidence gives them a concrete acceptance question.

**Acceptance:** the team can perform a final smoke test, inspect deployment/preview evidence, record the submitted artifact and export a privacy-filtered handoff without converting guesses into verified status.

## One ordered workflow, split equally

This is the **proposed division between @ZayedBH and @Atul013**. Atul has not confirmed these assignments in this document. Both tracks include implementation and verification; each person owns a distinct code area, reviews the other's integration points, and records evidence. Rebalance after the first two phases if the measured effort differs. Neither person closes a row from a build alone.

| Order | @ZayedBH | @Atul013 | Exit evidence |
|---|---|---|---|
| 1. Establish the native baseline | Own room, appearance and accessibility trials; fix reproducible defects in those areas. | Own editor, Git, terminal and installer trials; run Rust/CI checks and fix defects there. | Baseline log with build, pass/fail, defects and measured gaps for both tracks. |
| 2. Resolve migration boundary | Map Team Lobby, Studio and product requirements to native room, appearance and AI slices. | Map editor, Git, terminal, installer and extension infrastructure to native slices. | One prioritized feature matrix with owner, dependencies and acceptance for every kept item. |
| 3. Make the editor dependable | Own room persistence/recovery, theme readability and accessibility checks and fixes. | Own file change/save safety, editor performance and component benchmark. | No known silent data loss; repeatable startup/memory/large-file evidence. |
| 4. Daily IDE loop | Implement settings/keybindings, task/test UI and project workflow integration; test sample projects. | Implement LSP/DAP adapters, Git conflict flow and terminal improvements; test sample projects. | Both sample projects complete the agreed coding path. |
| 5. Native collaboration | Implement room UI, task/chat flows, consent display and cross-client UX. | Implement authenticated transport, invites, presence, revocation, persistence and reconnect. | Two machines pass authorization, reconnect and privacy scenarios. |
| 6. AI and extension model | Implement Ask/Plan/Build experience, review/rollback UI and declarative assets; test real provider UX. | Implement provider adapter, scoped capabilities, command/edit execution and extension runtime boundaries. | Real provider task with scoped context, approval, attributable edits and rollback. |
| 7. Release candidate | Own hands-on Windows acceptance, accessibility, product-facing fixes and release notes. | Own CI packaging, installer, license/security checks, hashes and release measurements. | Tested Windows artifacts and support claims backed by observed behavior. |

### Phase 1 — establish the native baseline (current)

**Why:** Source-branch claims and a green CI job do not prove the merged app works on your Windows desktop. This phase establishes a reproducible starting point before more code changes.

1. **@ZayedBH prepare a disposable trial.** Note the app commit/version, Windows build, display scale, and project path. Use a sample project without private credentials.
2. **@ZayedBH exercise the editor.** Open a folder and multiple files; edit and save UTF-8 text; close a dirty tab using Save, Discard, and Cancel in separate trials; reopen the file and confirm the right content. Try quick open, command palette, find/replace, file search, and theme preview.
3. **@ZayedBH own the room and Studio baseline.** Create a local room with a person, agent, task, dependency, evidence and handoff. Test theme preview, readability, keyboard operation and display scale. Reproduce and fix defects in those areas on your branch.
4. **@Atul013 owns the editor and platform baseline.** On a disposable Git repo, try status, stage, commit, diff, branch and sync; exercise the terminal and editor save paths. Install, upgrade, launch and uninstall in a disposable Windows user or VM. Check settings and external-project preservation, installer UI and shortcuts; fix defects in his areas.
5. **Both verify their slices.** Run Rust formatting, checks, tests and clippy for affected crates. Atul triggers the package workflow and records hashes/build metrics/install evidence; you inspect room/appearance outcomes and cross-check the release notes. Each records pass, fail or not tested.
6. **Together close the phase.** Record actual desktop behavior and outstanding defects. The exit condition is a visibly working edit/save loop and fresh installer result, with failures assigned by severity. Packaging smoke alone does not close interactive acceptance.

### Phase 2 — decide what migrates from Code OSS

**Why:** Old Team Lobby and Studio source remains useful as a behavior reference, but the Rust app cannot run those VS Code extensions as-is.

1. **@ZayedBH map collaboration and product behavior.** Read `PROJECT_PROGRESS.md` and the old Team Lobby/Studio sources. For each room, appearance and AI feature, record the wanted outcome, native equivalent, missing rule, priority and test scenario. Pick one real project and a two-person room scenario.
2. **@Atul013 maps editor and platform behavior.** For editor, Git, terminal, language tooling, extensions and installer, record the current native equivalent, missing adapter, priority and test. Separate reusable Rust code from Code OSS/Node assumptions.
3. **Both account for legacy sources.** Keep restored `extensions/`, `services/` and `releases/` as references until needed files and documents are mapped. Agree which specific modules to port and which to defer; avoid an undifferentiated "port everything" task.
4. **Together order the backlog.** Select the first three bounded native slices, each with a user outcome and a checkable exit condition. This phase ends with an explicit feature matrix, not a claim of parity.

### Phase 3 — protect files and measure editor performance

**Why:** Data safety and responsiveness must be established before expanding the editor.

1. **@ZayedBH own room and appearance safety.** Exercise room save/reload/corrupt-state recovery and theme readability at normal and high display scale. Implement recovery or accessibility fixes in the room and appearance paths, with regression tests.
2. **@Atul013 owns document safety.** Try large text, many tabs, external file edits, Save As, crash/relaunch, non-UTF-8 input and permission errors. Add external-change detection and a conflict choice before overwriting changed content; cover dirty-buffer and save-failure paths.
3. **Both measure.** Atul records startup, memory and large-file behavior and compares Rust editor components if egui misses thresholds. You record keyboard, contrast, screen-reader and room-persistence behavior. Treat silent data loss as a blocker in either track.
5. **Together make the component decision.** Exit only after there is no known silent edit loss, measurements can be repeated, and the editor-core choice is evidence-based.

### Phase 4 — complete the daily IDE loop

**Why:** The native product should support normal coding work before collaboration adds more moving parts.

1. **@ZayedBH own the workflow surface.** Choose two sample projects and implement settings/keybinding persistence, task/test discovery and run controls, and clear Problems/Run states. Test missing tools, failed tests and invalid projects.
2. **@Atul013 owns the engine adapters.** Add native LSP diagnostics/completion/hover/definition, DAP support, Git conflict handling and terminal scrollback/copy/shell selection in separate slices. Test disconnected remotes and failed processes.
3. **Both integrate and test.** Walk the same daily path—open, search, diagnose, edit, test, inspect Git, resolve conflict and commit—on both sample projects. Keep placeholders labelled until backed by live results.
4. **Together close against real projects.** Both sample projects must complete the agreed path without returning to Code OSS. A unit test or UI placeholder does not satisfy this gate.

### Phase 5 — bring Forge Room collaboration online

**Why:** The local room model is not yet a secure, recoverable multi-machine room.

1. **@ZayedBH own room behavior and UI.** Define the two-machine trial and default sharing choices. Implement participant/task/chat views, consent and revocation controls, shared-diff review, and clear offline/reconnect states.
2. **@Atul013 owns transport and identity.** Bind authenticated identities to `forge-room-core`; implement invite expiry/revocation, presence, event delivery, persistence and reconnect. Enforce authorization and recipient visibility before data reaches the UI.
3. **Both test both clients.** Try expired/revoked invites, unauthorized task changes, stale state after reconnect and project mismatch. Fix failures in each owner's layer and repeat the complete scenario.
4. **Together record the supported tier.** Exit only with two independent clients completing the scenario and preserving privacy. State whether the service is LAN/self-hosted or production-ready; a local protocol test cannot decide that.

### Phase 6 — connect AI and define extensions

**Why:** The AI panel is layout only, and arbitrary VS Code JavaScript extensions do not run in this Rust process.

1. **@ZayedBH own the interaction and asset layer.** Select one provider/task. Implement Ask/Plan/Build panels, context preview, approval and proposed-diff review, checkpoints and rollback controls. Port declarative themes, grammars and snippets with compatibility labels.
2. **@Atul013 owns execution boundaries.** Implement the provider adapter and native capability layer for context read, command execution, file edit, network use and delegation. Enforce scoped grants, attribution, cancellation and recoverability; evaluate WASM only after sandbox limits are designed.
3. **Both run the provider trial.** Test success, denied permission, rejected edit and rollback with the real provider. Compare visible UI state with execution logs and actual file changes before closing the slice.

### Phase 7 — prepare a release candidate

**Why:** Artifact creation, CI smoke, and a usable release are different milestones.

1. **@Atul013 owns release engineering.** Run Windows CI for build, license report, portable archive, installer, checksums, silent install/upgrade/uninstall and launch smoke. Compare binary, installer, notes and tag versions; review signing, permissions, package size and memory metrics.
2. **@ZayedBH own product acceptance and release presentation.** Perform hands-on install/upgrade/launch/uninstall on Windows 11 x64, test default/high-DPI and high-contrast displays, keyboard access and a real project. Fix product-facing defects, write accurate release notes and check settings/external-file preservation.
3. **Both cross-review.** You inspect artifacts and metrics; Atul reproduces critical product defects. Agree on known limits and a recovery path to the prior release.
4. **Together sign off narrowly.** Release notes must distinguish working features, placeholders, unsupported extension tiers, unsigned status, and platforms not tested. Exit when artifacts and observed Windows behavior support every published claim.

## Beyond the first release candidate

Phase 7 is an **early native release gate**, not the end of the Forge Room product plan. These follow-on phases keep the larger ideas visible. Start each only after its prerequisite is proven; split it into small slices with the same evidence loop used above.

| Phase | @ZayedBH owns | @Atul013 owns | Prerequisite and exit |
|---|---|---|---|
| 8. Advanced room collaboration | Spectate/follow UX, shared edit grants, chat context, voice/session controls, preview and Ship/Panic experience | Proven concurrent-edit protocol or reviewed patch fallback, session transport, revocation, reconnect, bounded media/service integration | Phase 5 two-machine privacy gate; multi-user edit/revoke/reconnect and consent checks pass |
| 9. Studio and creator ecosystem | Morph typed proposals and preview, Widgets/Workshop authoring flow, Packs UX, theme/profile controls and accessibility | Restricted widget/WASM runtime, package manifests, capability grants, provenance, install/update/uninstall and rollback | Phase 6 permission model; generated or installed content cannot obtain ungranted APIs |
| 10. Mesh and controlled automation | Task graph, deadlines, agent handoff, conflict/contract alerts, readiness and human escalation UX | Durable scheduler, idempotency, retry/cancel, budgets, gated test/deploy pipeline and evidence/event schema | Phase 6 real provider gate; no forced takeover or unapproved execution |
| 11. Wider access and operations | Pocket companion scope, cross-IDE/browser/CLI capability tiers, user-facing account/recovery/retention controls | Hosted room operations, authenticated relay, account/key recovery, cross-client protocol, observability and security review | Phase 5 transport gate; independent clients and operations tested before public service claims |
| 12. Deferred exploration | Decide whether Replay or other speculative concepts solve a real observed need | Prototype only after a bounded acceptance question exists | No standalone flagship commitment without user evidence |

Specific features, privacy conditions, and legacy evidence for these phases are in the detailed inventory above. Moving an item to a future phase is a scheduling decision, not deletion from scope. The original briefs in `docs/source-prompts/` remain the detailed requirement source when implementing a slice.

## Slice-by-slice execution order

The phases describe outcomes; this list is the proposed implementation order within them. A slice should end in a usable user action, a failure path and evidence. Owners are proposed and should be confirmed together before either person relies on the assignment. A dependency must be accepted before a dependent slice is called complete.

| ID | Owner | Slice and user-visible result | Depends on | Proof to record |
|---|---|---|---|---|
| B01 | @ZayedBH | Room/theme baseline: local task, handoff, theme preview and restore work visibly | Current native app | Screen observations, persistence check, defects |
| B02 | @Atul013 | Editor/Git/terminal baseline: open/edit/save, diff/commit, terminal and installer run | Current native app | Real project and disposable install observations |
| B03 | Both | Legacy migration map: every kept feature has target crate/UI, priority, owner and test | B01, B02 | Reviewed matrix with explicit deferred items |
| E01 | @Atul013 | External file change warning and safe save conflict choice | B02 | Two-process edit and no-overwrite test |
| E02 | @ZayedBH | Room state recovery plus theme/keyboard accessibility fixes | B01 | Corrupt-state recovery and accessibility trial |
| E03 | @Atul013 | Large-file measurement and editor component decision | E01 | Repeatable time/memory figures and decision note |
| I01 | @ZayedBH | Persistent settings/keybindings and clear task/test controls | B03 | Restart persistence and real test run |
| I02 | @Atul013 | Native LSP: diagnostics, completion, hover, definition | E01, B03 | Two-language project trial and server failure path |
| I03 | @Atul013 | Git conflict resolution and usable terminal scrollback/copy | B02 | Conflict and terminal replay trial |
| I04 | @Atul013 | Native debugger adapter path | I02 | Breakpoint/step/stack in supported project |
| R01 | @Atul013 | Authenticated two-client room transport and identity | B03, E02 | Join/deny/reconnect protocol tests |
| R02 | @ZayedBH | Create/join/invite/roster UX with explicit sharing preview | R01 | Two-machine wizard and privacy trial |
| R03 | @ZayedBH | Chat, DMs, task links and visibility-filtered history | R01, R02 | Cross-client DM isolation and search test |
| R04 | @Atul013 | Invite expiry/revocation, presence leases and event replay | R01 | Revocation and reconnect race tests |
| R05 | @ZayedBH | Task/milestone/countdown, activity and read-only follow UX | R02, R04 | Two-client timing, authorization and follow trial |
| R06 | @Atul013 | Permissioned saved diff transport and patch-proposal fallback | R04 | Revision/recipient/secret-filter tests |
| A01 | @Atul013 | Provider-neutral scoped capability and adapter boundary | R01 or local-only scope defined | Denied-tool, cancel, audit and path tests |
| A02 | @ZayedBH | Ask/Plan/Build context, approval, review and rollback UI | A01 | Real provider trial, rejection and restore |
| A03 | @ZayedBH | Declarative themes/grammar/snippet import and compatibility UI | I01, B03 | Safe import and invalid package tests |
| A04 | @Atul013 | Worktree isolation, exact edits, test evidence and recovery | A01, A02 | Unrelated work preserved after rollback |
| L01 | @Atul013 | Reproducible installer, portable archive, hashes, license and upgrade checks | Accepted B/I/R/A slices | CI artifacts and install records |
| L02 | @ZayedBH | Hands-on release acceptance, accessibility and honest notes | L01 | Windows trial and signed-off support matrix |
| X01 | Both | Advanced co-edit/follow and Sessions | R05, R06 | Two-client revoke/reconnect/media consent |
| X02 | Both | Morph, Widgets, Workshop and Packs | A01, A03 | Sandbox, permission and rollback trials |
| X03 | Both | Mesh/Pipeline, impact alerts and deadline rescue | R05, A04 | Idempotent gated execution and human override |
| X04 | Both | Pocket, cross-IDE tiers and hosted operations | R04, L02 | Independent client and operational security review |

### Handoff between the two owners

For a slice touching both tracks, the first owner opens an interface contract before writing dependent UI or transport code. The contract names actor IDs, message/version schema, error states, visibility rules, limits and migration behavior. The second owner builds against a fixture or mock of that contract. Integrate the two branches, run contract tests, then run the real user journey. Changes to a shared contract require both owners' review. A task handoff includes the diff, unresolved decisions, tests run, known failures, and the next exact action.

### System boundaries to maintain

| Boundary | Responsibility | Failure that must be prevented |
|---|---|---|
| `native/forge-room-core` | Room/task policy, actor IDs, state transitions, validation, persistence model | UI bypassing owner/host rules |
| `native/forge-room-app` | Native editor and UI, local state, platform dialogs, user approval surfaces | Placeholder shown as live data; silent file overwrite |
| Native adapters | Git, LSP/DAP, terminal, provider, transport and extension import | Ambient access exceeding the selected project or grant |
| Room service | Authenticated membership, invite enforcement, bounded relay and operational storage | DM/content leakage, replay, unbounded traffic |
| `packaging/windows` and CI | Reproducible artifacts, installer, license report, hashes and smoke evidence | Untested artifact or upgrade deleting user data |
| Legacy folders | Behavior and test references for migration | Accidentally treating Code OSS extension code as the native runtime |

No single event stream doubles as a collaborative editing algorithm. Event feed, saved-diff sharing, read-only follow and live co-edit have different consistency and permission requirements. No worktree doubles as a sandbox. No Skill text authorizes a tool. No screen share implies shared IDE state. These distinctions must survive implementation and release copy.

## Risk register and decision points

| Risk or open decision | Why it matters | Immediate owner action | Gate |
|---|---|---|---|
| Rust editor scaling | egui text layout may struggle with large files and many tabs | @Atul013 measures real workloads; @ZayedBH supplies representative projects | E03 before committing to a long-term widget |
| External edits and recovery | Silent overwrite would destroy user work | @Atul013 adds conflict handling; @ZayedBH tests crash and room recovery | E01/E02 before wider release |
| Legacy feature loss | A source folder exists, but behavior may not be native | Both maintain B03 matrix and this feature contract | Every ported feature has a native acceptance result |
| Room privacy and authorization | Search, history, relay or export could reveal content across roles | @Atul013 enforces server/core visibility; @ZayedBH tests guest and DM UX | R01–R06 two-client denial tests |
| Network scope | LAN, self-hosted and public internet are different support tiers | Both choose authority and deployment tier before inviting users | R01 design note; X04 operations review |
| Shared edit consistency | Concurrent edits/revocation can corrupt buffers | Both choose proven CRDT/OT or reviewed patch fallback | X01 multi-client no-loss trial |
| Agent process isolation | ACP/MCP/worktrees do not remove ambient OS permissions | @Atul013 defines execution boundary; @ZayedBH defines approval UX | A01/A04 denial and rollback tests |
| Provider cost and reliability | Budget, cancellation and stale results vary by provider | Both specify limits and actual supported provider behavior | A02 real-provider trial |
| Extension compatibility | Users may expect JavaScript VS Code extensions to run | @ZayedBH labels tiers; @Atul013 bounds native/WASM runtime | A03 package and permission tests |
| Installer and release claims | CI smoke can pass without usable desktop behavior | @Atul013 owns artifacts; @ZayedBH owns visible acceptance | L01/L02 before release |

## End-to-end acceptance scripts

These are future checks. Keep the result **not run** until an actual person or automation performs them on a named commit and environment. Each run records pass/fail, screenshots or logs where useful, and any unverified part.

### T01 — native daily coding loop

1. Open a representative project and multiple files. Edit, navigate, search and save; restart and confirm content and layout.
2. Run its language server, a failing then passing test, and a debugger session if that language is supported.
3. Inspect the exact Git diff, resolve a controlled conflict, commit and verify the resulting history.
4. Repeat after an external process changes an open file. **Pass:** the app warns and offers a safe choice; no buffer or external change disappears silently.

### T02 — private room from two machines

1. Host creates an invite-only room bound to a disposable project and shows the sharing preview.
2. A second machine joins from an expiring code and binds its own local project without automatic pull/reset or upload.
3. Disconnect and reconnect both clients; compare member count, task state, event sequence and timer.
4. Revoke the invite and try to join again. **Pass:** the old code fails, authorized members retain correct state, and private files remain private.

### T03 — room communication and visibility

1. Three participants exchange public chat; two exchange a DM and link it to a task.
2. The third participant searches, requests history and exports permitted activity.
3. Directly call the protocol with the third identity for the DM ID. **Pass:** the third participant receives no DM content, result metadata or search hit; authorized participants can still recover their own conversation after reconnect.

### T04 — task, handoff and countdown

1. Create dependent tasks with owners, criteria, estimates, due times and a room milestone.
2. Attempt to complete a blocked task without its criteria; request handoff to another owner; approve as host.
3. Pause and extend the countdown as host; try the same operation as a contributor; reconnect both clients.
4. **Pass:** unauthorized transitions fail, approved events are attributed once, timer/deadlines agree across clients, and readiness distinguishes verified checks from manual statements.

### T05 — working diff, follow and edit grant

1. Participant A opts to share a saved diff for one file while leaving an unsaved buffer and terminal output private.
2. Participant B follows the permitted file, requests edit access for that file/session/time, then submits a patch or joins a supported shared buffer.
3. A revokes access during activity; disconnect and reconnect both clients.
4. **Pass:** B never sees the private buffer/terminal data, cannot write after revocation, and both versions reconcile without losing either person's work.

### T06 — agent Ask, Plan, Build and rollback

1. Bind a real provider and a disposable project. In Ask, allow selected context but no writes; in Plan, review an editable proposal.
2. In Build, approve only named files and one test task. Try a forbidden file read, an unapproved command and a network request outside the grant.
3. Inspect exact changed files, provider/tool evidence, test result and task attribution. Reject one proposed edit; accept another; roll back the accepted edit.
4. **Pass:** denied actions do not execute, approvals match actual operations, rollback preserves unrelated work, and failed/unknown checks are not called verified.

### T07 — deadline rescue without forced takeover

1. Make a critical task blocked and due soon. Compare recommendations against eligibility, dependencies, capability, budget and owner preference.
2. Choose recommend-only, then optionally authorize a narrow queue/execute policy with a fixed time and cost ceiling.
3. Opt out, pause or reclaim while work is pending. Retry a duplicate event.
4. **Pass:** no agent starts without applicable authority, owner control wins, duplicate events do not duplicate work, and merge/deploy still require their own gates.

### T08 — Studio and creator safety

1. Preview each theme at normal, high-contrast and reduced-motion settings; Keep one and Restore another. Import/export a profile.
2. Request a Morph layout change and reject it after preview. Generate a widget with a denied data source and one permitted source; mark sample data visibly.
3. Package approved appearance/widget configuration into a Pack, install, disable and uninstall it.
4. **Pass:** editor readability and trust settings survive every appearance operation; denied data cannot be read; widget crashes are contained; rollback removes only the installed change.

### T09 — Sessions, Pocket and client tiers

1. Join a room voice call with device permission, mute, push-to-talk and reconnect. Attempt optional recording without consent.
2. Use Pocket to view a task, inspect a permitted diff and approve a bounded action; try it with a revoked identity.
3. Join through a tier-C CLI or guest client and request a tier-A live buffer.
4. **Pass:** media capture follows consent, actions are attributed to the correct identity, revocation takes effect, and capability tiers reject unsupported data/action requests.

### T10 — release, upgrade and recovery

1. Build the release from a named tag; record source commit, version, license report, binary/installer/portable sizes and SHA-256 hashes.
2. On Windows 11 x64, install as a normal user, open a real project, save settings, upgrade, reopen, then uninstall.
3. Check external project files, settings, shortcuts and data retention against the documented policy. Repeat at high DPI/high contrast and inspect startup/idle/editor/room memory.
4. **Pass:** hashes and versions agree, supported workflows visibly work, user projects survive, removal behavior matches release notes, and limitations are plainly stated.

## Working rules

1. **Define the outcome.** Write what the person should be able to do, what must never happen, and the observable success condition. Link the requirement or bug.
2. **Record the starting point.** Note branch, commit, existing behavior, affected files, and related uncommitted work. Preserve historical local edits and the restored legacy reference folders.
3. **Change one bounded slice.** Branch from `main`. Put reusable room policy in `forge-room-core` and UI/platform adapters in `forge-room-app`. Do not mix feature work with historical-document cleanup.
4. **Run focused checks.** Format, check, test, and clippy the affected crate; then use Windows CI. Add tests for policy, persistence, permission boundaries, and real regressions.
5. **Try the actual workflow.** You test a representative Windows project. For network or provider behavior, test two independent clients or the real provider. Capture observed behavior, not only a passing process.
6. **Review and integrate.** Atul records code and test evidence; you review the product result. Merge only when the slice's exit gate is met or clearly label an experimental gap.
7. **Update this ledger.** Record date, owner, branch/commit, commands, Windows observation, limitations, and next owner action. Keep source integrated, automated pass, interactive pass, and multi-client/provider pass distinct.

Keep room and AI execution permissions explicit. Local policy checks do not establish secure multi-machine behavior. Do not claim compatibility with VS Code JavaScript extensions. Keep credentials, tokens, private project files, and room data out of commits and screenshots.

### Verification commands

Run these from the repository root in PowerShell with a Rust toolchain installed. They do not install or uninstall the app. `cargo` was unavailable in the local environment during the repository switch, so use a Rust-capable machine or CI until that changes.

```powershell
cargo fmt --manifest-path native/forge-room-core/Cargo.toml -- --check
cargo check --locked --manifest-path native/forge-room-core/Cargo.toml
cargo test --locked --manifest-path native/forge-room-core/Cargo.toml
cargo clippy --locked --manifest-path native/forge-room-core/Cargo.toml --all-targets -- -D warnings

cargo fmt --manifest-path native/forge-room-app/Cargo.toml -- --check
cargo check --locked --manifest-path native/forge-room-app/Cargo.toml
cargo test --locked --manifest-path native/forge-room-app/Cargo.toml
cargo clippy --locked --manifest-path native/forge-room-app/Cargo.toml --all-targets -- -D warnings

cargo run --release --manifest-path native/forge-room-app/Cargo.toml -- C:\path\to\sample-project
```

The workflow is `.github/workflows/native-rust.yml`. Pull requests run `verify`; `package-windows` runs on its configured push branches, tags, or manual dispatch. Inspect its artifacts before claiming an installer was built. `packaging/windows/verify.ps1` performs silent install/upgrade/uninstall and a non-interactive launch probe; it changes the test user's Forge Room installation, so run it in a disposable test environment after building its expected artifacts. A visible desktop trial remains necessary.

### Evidence entry template

Append one short entry per completed slice or failed gate:

```text
Date / owner / branch / commit:
Outcome and test project:
Commands and CI link:
Observed Windows behavior:
Status: source integrated | automated pass | interactive pass | multi-client/provider pass
Open defects or limits:
Next action and owner:
```

## Source index and reconciliation

- `PROJECT_PROGRESS.md`: historical Code OSS product inventory, delivery evidence and outstanding acceptance gates. Its Code OSS release numbers are **legacy history**, not native version numbers.
- `docs/product/PRODUCT_SPEC.md` and `docs/source-prompts/`: full feature requirements and original briefs.
- `docs/architecture/IMPLEMENTATION_MAP.md`, `THREAT_MODEL.md`: legacy component map and security assumptions; revisit for Rust architecture.
- `docs/planning/CURRENT_STATE.md`, `BUILD_STATUS.md`, `FORGE_AI_NATIVE_EXPANSION_DELIVERY.md`, `FORGE_AI_NATIVE_EXPANSION_RECONCILIATION.md`, `BUILD_PROMPT.md`, `PROTOTYPE_README.md`: prior build and AI delivery details; their current-status claims predate the Rust switch.
- `docs/research/`: audit and competitor research that informs later slices.
- The supplied fork's `project_rust_progress.md` informed this consolidated ledger. Its `docs/native/MIGRATION_COMPATIBILITY.md`, `docs/native/EARLY_PREVIEW_NOTES.md`, and `docs/design/installer-progress.md` are available in this checkout under their original paths.
- `docs/native/` and `docs/design/`: native release, migration, and installer source documents copied from the fork without replacing existing planning documents.
- `COLLAB.md`: imported source-branch agent handoff; this file supersedes its project status and assignments.

## Next checkpoint

### UI concept direction — 2 November 2026

The supplied Forge Room mockups are beta design references for layout and workflow, not proof that their sample agents, tests, calls, sharing, deployments, or live collaboration exist. Keep the current Forge Room color system. For native slices, use the mockups' clear left navigation, Room Pulse, project Lobby summary, task/agent context beside code, review and conflict status, and explicit permission states where the underlying behavior is real. Treat Workflows, Ship Mode, Studio Morph/Packs, and the logo variants as later concepts until their contracts and assets are accepted. Keep controls readable at the supported minimum window size and keyboard accessible.

Seven original concept PNGs from Downloads are preserved in [`docs/design/concept-images/2026-11-02/`](docs/design/concept-images/2026-11-02/README.md). Both shared chats contain ten image references in total; three original files were not in Downloads when checked. Asset collection remains open until those files are supplied.

### Phase 1 evidence — 2 November 2026

- **6 November concept review and dock controls:** [PR #21](https://github.com/ZayedBH/forge-room-ide/pull/21) adds a compact Room-dock row for real Git changes, prioritizing merge conflicts and opening Source Control. [PR #22](https://github.com/ZayedBH/forge-room-ide/pull/22) replaces custom dock/terminal tab hit areas with native egui buttons and keeps the Room tabs scrollable at narrow widths without hiding the close control. [Windows CI #21](https://github.com/ZayedBH/forge-room-ide/actions/runs/42214090593) and [#22](https://github.com/ZayedBH/forge-room-ide/actions/runs/42215267170) passed core and graphical formatting, checks, tests, and Clippy. Desktop narrow-window and keyboard interaction remain open; the Git row shows only repository status actually reported by Git.

- **6 November concept-based native UI:** [PR #19](https://github.com/ZayedBH/forge-room-ide/pull/19) and [PR #20](https://github.com/ZayedBH/forge-room-ide/pull/20) brought the coding workspace and Project Lobby Room Pulse closer to the saved Aether concept images: bounded, individually scannable local activity segments open the Activity dock; the coding workspace opens with the Team dock; the Lobby setup Review is a native keyboard-focusable button. [Windows verify #19](https://github.com/ZayedBH/forge-room-ide/actions/runs/42211620065) and [#20](https://github.com/ZayedBH/forge-room-ide/actions/runs/42212887443) passed core and graphical formatting, checks, tests, and Clippy. A [portable package run](https://github.com/ZayedBH/forge-room-ide/actions/runs/42211282195) succeeded and its archive SHA-256 matched `SHA256SUMS.txt` (`5D1ABA9CEE2FCC0A94E47D55B31693DECB02497CE16BD3AF627EE176038B903D`). The isolated app launched, but the computer-use surface reported its window minimized and did not provide a screenshot. Keep visual desktop layout and keyboard activation acceptance open; no remote presence or live agent state is implemented by these UI changes.

- **6 November task handoff copy:** [PR #18](https://github.com/ZayedBH/forge-room-ide/pull/18) merged into `main` (`b60ecb2`). Expanded local tasks can copy a summary with title, owner, status, unfinished dependencies, acceptance checklist, and evidence belonging to that task. A focused test excludes another task's evidence. [Windows CI run 42210098263](https://github.com/ZayedBH/forge-room-ide/actions/runs/42210098263) passed core and graphical formatting, locked checks, tests, and Clippy. Desktop clipboard interaction remains open.

- **6 November blocker triage:** [PR #16](https://github.com/ZayedBH/forge-room-ide/pull/16) merged into `main` (`6b0c48e`). The local Tasks dock can show blocked tasks only, open unfinished dependencies from a task card, and label a missing dependency. Following a dependency clears task filters so the target is visible. A focused dependency test and the core/graphical Windows checks passed in [run 42208366118](https://github.com/ZayedBH/forge-room-ide/actions/runs/42208366118). Desktop interaction remains open.
- **6 November activity copy:** [PR #17](https://github.com/ZayedBH/forge-room-ide/pull/17) merged into `main` (`b3f6fd9`). An explicit button copies only the currently visible local activity after the person/detail query and 10/50 limit, including actor, detail, and relative time. Core and graphical Windows checks passed in [run 42209195088](https://github.com/ZayedBH/forge-room-ide/actions/runs/42209195088). Clipboard interaction remains to be checked on the desktop. These changes do not add cross-client history.

- **6 November Room task owner filter:** [PR #15](https://github.com/ZayedBH/forge-room-ide/pull/15) merged into `main` (`d58e2e0`). The local Tasks dock can select an assigned owner and combine that choice with title/name and status filters; Clear filters resets all three. Choices include owners on saved tasks even after a participant leaves. [Windows CI run 42206725868](https://github.com/ZayedBH/forge-room-ide/actions/runs/42206725868) passed core and graphical formatting, locked checks, tests, and Clippy. The first run failed only on rustfmt layout and was corrected. The owner-filter desktop trial remains open; this is local UI filtering, not shared synchronization.

- **6 November integration:** PR #14 merged the Room keyboard, task, and activity stack (#10-#13) with four newer commits from `Atul013/native-rust-ide` into `main` (`45e934b`). [Windows CI run 42198613453](https://github.com/ZayedBH/forge-room-ide/actions/runs/42198613453) passed core and graphical formatting, locked checks, tests, and Clippy. PR packaging was skipped by workflow design. The Phase 1 hands-on checklist is in `docs/native/PHASE1_TRIAL_CHECKLIST.md`; desktop, installer, and multi-client acceptance remain open.

- **Room activity filter, source changed 3 November 2026:** Activity dock can filter local events by person or detail, ignoring case and surrounding query spaces. Clear filter restores the list; no-match state is explicit. One focused match test was added. Windows CI passed in run 42198613453; desktop interaction is pending. This remains local history, not cross-client sync.
- **Room activity history, source changed 3 November 2026:** The native Activity dock shows an explicit empty state and can switch between the latest 10 and up to 50 events. The limit stays bounded for the current sidebar renderer. Windows CI and desktop interaction are pending; this does not implement cross-client history. B01 remains open.
- **Room task usability, source changed 3 November 2026:** The native Tasks dock now filters existing tasks by title or owner and by status. Clear filters restores the full list; a no-match message distinguishes filtered results from an empty room. Opening the new-task form focuses its title field. Expanded tasks show an explicit Add evidence button that requires nonempty text; Enter remains available. One focused query-match test was added. Windows CI passed in run 42198613453; desktop interaction is pending. B01 remains open.
- **E02 keyboard accessibility, automated pass:** Lobby now focuses `Open Room details` when an unreadable room first appears. Room task titles, Lobby tabs, primary actions, and icon actions use native buttons; checklist, permission, and sharing switches use native checkboxes. Icon actions and checklist switches have accessible names. [PR #10 Windows verify](https://github.com/ZayedBH/forge-room-ide/actions/runs/41503226799) passed formatting, locked checks, tests, and clippy for both Rust crates. Keyboard activation and state semantics still need a desktop trial. E02 remains open.
- **Latest source integration:** Atul's `native-rust-ide` branch at `ade42a2` was integrated through [PR #8](https://github.com/ZayedBH/forge-room-ide/pull/8), then the Lobby room recovery change through [PR #9](https://github.com/ZayedBH/forge-room-ide/pull/9). Git reported no text conflicts in either merge. The combined PR passed Windows formatting, locked checks, tests, and clippy for both Rust crates. PR #7's commits are also marked merged through PR #9. The [current-main Windows run](https://github.com/ZayedBH/forge-room-ide/actions/runs/41489152314) passed verify, release packaging, and noninteractive install/uninstall checks. Desktop acceptance remains open.
- **Lobby recovery slice, source done:** When room files cannot be read, Lobby shows an unavailable warning, an `Open Room details` action, and a recovery status instead of offering `Create Room`. The action opens the Room dock. An isolated Windows trial of the first patch confirmed the warning, button, status, and unchanged invalid file hashes. The final dock-opening change passed CI but has not yet had a fresh desktop interaction trial. Keep E02 open until that trial and keyboard/theme checks pass.
- **PR check:** `ZayedBH/forge-room-ide` had no open pull requests when checked.
- **Integration update:** [PR #4](https://github.com/ZayedBH/forge-room-ide/pull/4) was subsequently merged into `main` at `c495aafa260694150c86908e6e93461b4027ecae`. Its editor safety, terminal, conflict and lobby changes are source-integrated; the PR's verify and branch package jobs passed. Interactive acceptance remains separate. [PR #6](https://github.com/ZayedBH/forge-room-ide/pull/6) restored the 149 `docs/`, `extensions/`, `services/`, and `releases/` reference files removed by PR #4; every restored file matched its pre-PR Git blob hash. The separate `archive/legacy-team-lobby/` duplicate remains removed.
- **Merge conflict audit:** On 2 November 2026, `main` contained all commits from `atul/native-rust-ide` and PR #5. `git ls-files -u` and `git status --short` were empty after PR #6. PR #4 and PR #5 touched `native/forge-room-app/src/main.rs` and `rooms.rs`, but PR #5 was based on the merged PR #4 and passed Rust verification and a current-main Windows package run. No unresolved text or code merge conflict was found.
- **E02 recovery guard, source integrated:** [PR #5](https://github.com/ZayedBH/forge-room-ide/pull/5) merged at `1d6626d6a6b8a94dd33b3ac5ef50d2a799fff4d7`. If room primary and backup files are unreadable, Create Room cannot replace them. A valid backup is loaded even when the primary file is missing. The Lobby routes failed creation to the Room error panel. The [Windows verify run](https://github.com/ZayedBH/forge-room-ide/actions/runs/41445528112) passed formatting, checks, tests and clippy for both Rust crates, including two new recovery tests. The PR package job was skipped by workflow design. Accessibility trial remains open, so E02 is not marked complete.
- **E02 recovery desktop trial:** The [current-main Windows package run](https://github.com/ZayedBH/forge-room-ide/actions/runs/41446717033) passed verify, package, and noninteractive install checks. Its portable archive SHA-256 `CD12AA60B606537E40C7E73534AC2436347E57C17173B7989BB40E5D6268D229` matched `SHA256SUMS.txt`. With isolated `%LOCALAPPDATA%` and a disposable project, the running app displayed a read error and disabled Create Room when both state files were invalid; both file hashes remained unchanged. In a second disposable project with only a valid backup, the running app loaded its participant and displayed the recovery message; backup hash remained unchanged. Theme, task, keyboard accessibility, and interactive installer checks remain open.
- **B01 partial trial, @ZayedBH track:** Downloaded the repository's `v0.110.0-alpha.1` portable archive into a disposable temporary folder. SHA-256 `51CDCC7E1E5CFF47CFFE35721DCB22344AED01749417985BCA2D8C14EDF2D966` matched the release asset digest. Launched `forge-room-early-preview.exe` against a temporary sample project. The app showed version `0.110.0-alpha.1`; keyboard shortcut `Ctrl+Shift+R` opened Room. Created a local room as Admin, exited, reopened the same project, and observed `Room: sample` and Admin as host. The state file retained `schemaVersion: 1`, one participant, and private sharing defaults.
- **B01 status: needs hands-on check.** The screenshot channel timed out, so theme preview/restore, readability, display scale, room task/dependency/evidence/handoff, and full keyboard accessibility were not verified. No B01 completion claim. Repeated the launch and Room persistence observation using the current-`main` CI portable artifact.
- **Build gate:** `cargo` and a Visual Studio C++ toolchain were unavailable on this Windows host. The [current-main Windows verify/package run](https://github.com/ZayedBH/forge-room-ide/actions/runs/41438036066) passed formatting, locked checks, tests and clippy for both Rust crates; it built and uploaded the portable archive and installer. Downloaded archive SHA-256 `44C97EC8C9D7FEBAF48403C5FDF7F98C4A66FF682C71016F90ED5A955C7F70D8` matched `SHA256SUMS.txt`. CI's noninteractive installer check passed upgrade, preservation, shortcuts and uninstall; `interactiveInstallerUiVerified` was `false`. The current-main portable app launched on this desktop and displayed the persisted local room. B02 hands-on installer trial and Phase 1 exit remain open.
- **Next action:** Complete B01 theme/task/accessibility checks, E02 accessibility trial, and B02 desktop/interactive installer trials. Mark slices done only after their visible exit conditions pass.

1. **@ZayedBH:** perform the Phase 1 room, theme and accessibility trial in a disposable project; record defects and fix reproducible issues in your areas.
2. **@Atul013:** inspect the completed Windows package artifact evidence, perform the editor, Git, terminal and interactive installer trial, and fix reproducible issues in his areas.
3. **Together:** review failures, choose the first bounded fix, and close the Phase 1 gate before Phase 2.
