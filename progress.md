# Forge Room — progress and complete working plan

**Updated:** 2 November 2026. **Active product:** native Rust IDE, Developer Preview `0.110.0-alpha.1`, Windows 11 x64. **Current phase:** Phase 1, visible Windows baseline. **Owners:** @ZayedBH and @Atul013.

This is the current project status, assignment, and next-action document. Historical documents remain available for requirements and evidence, but their older Code OSS status statements do not describe the native app. **Done** means code or an action is complete; **CI passed** means automated checks passed; **Needs hands-on check** means actual product behavior remains unverified; **Planned** means it is not implemented.

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

1. **@ZayedBH:** perform the Phase 1 room, theme and accessibility trial in a disposable project; record defects and fix reproducible issues in your areas.
2. **@Atul013:** perform the editor, Git, terminal and installer trial; run the separate Windows package job, inspect artifact evidence and fix reproducible issues in his areas.
3. **Together:** review failures, choose the first bounded fix, and close the Phase 1 gate before Phase 2.
