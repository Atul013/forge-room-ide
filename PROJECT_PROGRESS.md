# Forge Room — project progress and complete ideas ledger

**Snapshot:** 22 October 2026
**Active implementation:** paused at the user's request while the repository and documentation are organized.
**Evidence rule:** A feature is “implemented” only when source is integrated. “Automated checks pass” is not the same as interactive IDE, real-provider, or multi-machine acceptance.

This document is the single overview of the product vision, all major ideas collected across the product specification and both supplied implementation briefs, what exists, what remains, and the order for continuing. The original briefs are preserved in [`docs/source-prompts/`](docs/source-prompts/) and contain the exhaustive detailed requirements and scenarios. The product spec, audits, research and build history in [`docs/`](docs/) retain the supporting detail.

## 1. Product in one paragraph

Forge Room is a real, customizable desktop IDE based on Code OSS, with a hackathon-first project room that connects human developers and their owned AI agents. People can coordinate tasks, communicate, inspect permissioned work before commits, and move changes through evidence, review and handoff while Git remains the durable source of truth. Room and AI capabilities must remain optional: the IDE works solo and offline. Sharing, provider transfer, commands and edits are scoped, visible, explicitly authorized where needed, attributable, and recoverable. A lively visual identity and multiplayer-lobby feel should support the coding workflow, not replace it with a static mockup or generic Kanban board.

## 2. Product principles and constraints

- Preserve the complete IDE loop: editing, navigation, language tools, refactoring, terminal, tasks, tests, debugger, Git, extensions, keyboard access and assistive technology.
- Make people and agents first-class participants. Every agent has a human or service owner, identity, runtime/provider, task, permission profile, execution location and provenance.
- Show real work and link activity to evidence. Never infer authorship, fabricate progress, or claim a result is verified without the relevant check.
- Separate presence, metadata, file paths, diffs, saved content, unsaved buffers, terminal output, debugger data, previews and agent context into distinct sharing choices.
- Support read-only spectate/follow and shared editing as distinct modes. Only claim buffer sync for a tested supported client tier; otherwise use reviewed patch proposals.
- Keep Git authoritative for durable history, merge and release. Room state coordinates work between Git operations.
- Require clear permission and approval for commands, sensitive data, network access, edits, delegation, merge/deploy and other high-impact operations. Preserve provenance, audit and undo/recovery paths.
- Keep extension Skills as reusable procedural knowledge, never as security policy, memory authority, scheduler, debugger, event bus or execution sandbox.
- Use progressive client/provider capability tiers and explain unsupported behavior rather than implying universal support.
- Respect privacy, least privilege, data retention, accessibility, readable themes, reduced motion and editor performance.
- Build and accept small end-to-end slices. Keep unsupported or simulated adapters visibly labelled.

## 3. Ideas and feature inventory

The status in the final column is evidence-based. “Foundation exists” means some relevant implementation is present; it does not imply the full idea is delivered or accepted.

| Area | Complete product idea inventory | Current state |
|---|---|---|
| Core IDE | A professional Code OSS desktop product; project and multi-root workspaces; editing, search, navigation, language tooling, run/debug, tasks/tests, terminal, Git, extension ecosystem, keyboard/accessibility, offline and solo use. | Code OSS fork and packaged Forge extensions exist. Continue repeatable product builds and test the real end-user loop. |
| First-run room flow | Rooms/Live entry and commands; create/join/recent/return; minimal progressive setup; bind project/repo/branch optionally; room name, privacy, icon and mode; choose LAN or service availability; choose duration/timezone, milestones and submission details; preview sharing before start. | Room and invite foundations exist. Full product-flow and manual UX acceptance remain. |
| Room membership and lifecycle | Invite-only default; high-entropy codes/links with expiry, use limits, rotation and approval; identity and roles; human/agent ownership; member moderation; create/join/leave/pause/archive/restore/export; host-authoritative countdown and controlled pause/extension; reconnect and presence leases. | Significant host/client room, identity, invite, presence, timer and history foundation exists. Hosted-service operations and full multi-client acceptance remain. |
| Room UI | Room/project selector; connectivity state; member count, time, sharing indicator and alerts; lobby/team, chat, tasks, agents, activity and build/demo surfaces; dock/collapse; loading, empty, offline, error and reconnect states; accessible focus and reduced motion; do-not-disturb and notification controls. | Team Lobby views and Studio exist. A broad UI overhaul and all interaction states are not complete. |
| People communication | Room chat, direct messages, mentions, reactions, threads and searchable decisions; contextual messages anchored to code/diff/task/preview/test; messages to an agent owner stay distinct from direct agent commands; privacy for DMs. | Chat, direct messages and anchored comments exist. Search, reactions, richer threaded/contextual integration are future work. |
| Tasks and hackathon timing | Fast task capture; owner, priority, due time, status, blocker, dependency, linked artifact and handoff; 12/24/36/48/custom countdown; milestones and warnings; audit-controlled pause; critical-path and time-aware triage; submission/demo URL and readiness checklist. | Tasks, ownership/handoff, dependencies, deadlines, worktrees and host-controlled countdown features exist in slices. Task context for AI is the next unfinished slice. |
| Live awareness and activity | Attributed timeline for membership, task, agent, file, tests, terminal/build, Git, decisions, contracts, conflicts, preview, milestones and handoff; immutable IDs, timestamps, sequence, schema, visibility, source and idempotency; “since last seen” with evidence links; real live/delayed/offline/unsupported labels; rate limits and no keystroke surveillance. | Room event history, Git/activity, diff snapshots and collaboration events have a substantial base. Complete event coverage and cross-client acceptance remain. |
| Work visibility | Permissioned active-file metadata, saved diffs, optional unsaved buffer, cursor/selection where supported, conflict awareness, preview and artifact links before Git commit; sensitive-path filtering; explicit consent, bounded transfer, revision, reconnect and deletion rules. | Opt-in diff sharing, comments, shared-document prototype and capability levels exist. Two-IDE and two-machine validation, broader conflict handling and clear tier reporting remain. |
| Spectate and shared edit | Read-only follow without stealing local navigation; ask for edit access; owner grants file/session/time scope; revocation stops writes; tested CRDT/OT, cursor awareness, reconnect, conflict semantics and collaborative undo; safe patch fallback for unsupported clients. | Shared-document capability foundation exists. Safe multi-client IDE accept/revoke/reconnect and general concurrent editing are not verified. |
| Agent participants | Owned agent roster, provider/runtime, local/remote location, capabilities and permissions, session/task/worktree, availability/busy/blocked, activity/progress, logs and artifacts, pending approvals, abort/retry, review and handoff. Keep private chain-of-thought private. | Provider-neutral Ask/Plan/Build, ACP Ask/Plan and agent identity/activity foundations exist. Broad agent fleet orchestration and external provider acceptance remain. |
| AI context and modes | Distinct Ask (understand only), Plan (editable proposal), Build (approved implementation); choose files, folders, symbols, selection, diffs, room tasks/decisions, diagnostics, terminal results, debugger stack and skills; show what will go to which provider; selected-project-root scope and bounded outputs. | Significant Ask/Plan/Build and context tooling exist. The 0.107 file inventory and 0.108 editor navigation close two specific gaps. Room task-context file exists but is not integrated. |
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
| Service / LAN / cloud | Private LAN/self-hosted room first; authenticated encrypted online relay as separate tier; room authority, discovery, NAT/relay decision, durable identities, expiring/revocable invites, persistence, account/key recovery, tenant policy, retention/deletion, observability and independent security review. | Self-hosted room service/control-plane and bounded WSS relay prototype exist. It is not a production managed internet service; consult the threat model before exposure. |
| Replay and deferred concepts | Optional activity replay may be explored after the core flow. Keep Replay lower priority. Do not split low-value concepts such as Forge Twin, Ghost Developer, Forge Universe or Forge What-If into separate flagship products unless the user revisits that direction. | No Replay implementation. Deprioritized concepts are deliberately out of the active roadmap. |

## 4. What is implemented now

### Product foundations

- Branded Code OSS product checkout, with Team Lobby collaboration and Forge Room Studio extensions.
- Team Lobby 0.108.0, protocol 17, and Studio 0.14.0 are the latest versioned extension packages in this handoff snapshot.
- Room features accumulated across prior slices include room hosting/participation, invite identity and revocation, presence, activity/history, chat/direct messages, task ownership and handoff, deadlines/dependencies/worktrees, Git status/diff activity, optional file sharing/comments/shared-document flows and export/review artifacts.
- AI workflow includes provider-neutral Ask/Plan/Build paths, project guidance and skills, workspace context, MCP/ACP integration, task/test/debug/terminal capabilities, exact approved edits, review/checkpoint/rollback and evidence boundaries.
- Studio includes six bundled themes and reversible theme/profile application, contrast tests and opt-in decorative motion.
- `services/room-service/` contains a TLS account/control-plane and bounded relay prototype. It is not ready for public managed service use.

### Last three complete slices

| Release | Completed behavior | Evidence |
|---|---|---|
| Team Lobby 0.106.0 | Save a single-file checkpoint before each exact approved Build edit; restore only when the current file still matches the approved result, so later edits are protected. | Both handoff and Code OSS copies passed TypeScript checks and 175/175 tests. VSIX SHA-256: `D7896744609ED694E7E9A199C6D1E2EAF5B7C3BB900D00C6DA0B2CC38E630AF2`. |
| Team Lobby 0.107.0 | Bounded metadata-only inventory of up to 250 non-sensitive paths under the selected project root for Ask/Plan/Build. | Both copies passed TypeScript checks and 178/178 tests. VSIX SHA-256: `A25DF060D7E6FA2C21088376830A58874C1B320A52B2CD3800EB06EC3FDAA380`. |
| Team Lobby 0.108.0 | Explicitly confirmed AI editor navigation to one safe project file and optional validated line; no source contents returned by navigation. | Both copies passed TypeScript checks and 179/179 tests. VSIX SHA-256: `6EF2EC09E51E25884577C159AA19382A947C682C6F1487D080C9B6ACF8F0FBFA`. |

These are incremental releases. They do not mean every item above is complete.

## 5. Current in-progress item and validation limits

- A standalone [`roomTaskContext.ts`](extensions/team-lobby/src/roomTaskContext.ts) helper has been started for explicit room-task context. It has no integration, tests or package version bump. It is **not a completed feature**.
- Latest full current-state evidence is in [`docs/planning/CURRENT_STATE.md`](docs/planning/CURRENT_STATE.md) and [`docs/planning/BUILD_STATUS.md`](docs/planning/BUILD_STATUS.md).
- Reported automated status at Team Lobby 0.108.0: 179/179 extension tests in the handoff and Code OSS mirrors; Studio 16/16; room service 9/9. The Code OSS compile succeeded before later extension-only updates.
- Still unverified by interactive acceptance: two independent IDE clients, separate-machine TLS/room scenarios, real provider tool selection and transfer, rendered diff/Undo/rollback behavior, actual theme/profile dialogs, voice, hosted agents, mobile, and production service operations.
- Local provider/ACP/MCP processes must not be called sandboxed unless OS-level isolation is added and verified. Current source audits document ambient-permission limitations.

## 6. Roadmap and exit gates

This combines the original Room V1/V2/V3 plan and AI-native P0/P1/P2 stack. Work remains sequential: prove the room core and its acceptance before building broad runtimes.

### Foundation / P0 — finish and accept the current core

1. Finish room task context: register it with Ask/Plan/Build, bind it to the selected workspace and permitted task, show exactly what is shared, bound output, redact evidence, test, mirror, package and update status.
2. Run two isolated IDE clients through create/join, identities, presence, chat, tasks, activity, diff/comment/follow, disconnect/reconnect, invite revocation and host behavior.
3. Run two-machine TLS relay tests, including expiry/revocation, restart, malformed/oversized input and network failure. Record actual logs and unsupported cases.
4. Exercise the AI loop with supported real providers: Ask without edits; editable Plan and handoff; Build approval, exact diff, test evidence, review and safe rollback. Verify displayed transfer, tool approval and cancellation.
5. Validate ordinary IDE operations and Studio themes/profiles using keyboard, high-contrast/reduced-motion settings and representative projects.
6. Close privacy, authorization, no-data-loss, reconnect and build stability issues before public exposure.

### Room V1 — useful hackathon collaboration

Deliver the integrated IDE room entry; private create/join/invites and identity/roles; presence; chat and code comments; task ownership/status/priority/deadline/dependencies; host-authoritative timer and milestone alerts; actual agent presence/progress through at least one supported adapter; basic IDE build/test signals; opt-in uncommitted diff visibility and read-only spectate; recommendation-only deadline dispatch; persistence/reconnect/idempotency; exportable activity summary; revocation and secret filtering. V1 exit: two real IDE clients can independently work, see authorized uncommitted changes and evidence before commit, coordinate and reconnect without data loss.

### Room V2 — collaboration and controlled automation

Add tested shared-buffer CRDT/OT where supported, cursors and collaborative undo; follow/split-pane workflow and guarded patch fallback; room voice and optional preview sharing; bounded queue/execute policies with narrow preauthorization, resource/time/budget limits, escalation and duplicate prevention; agent fleet controls; critical-path and milestone risk; contract/conflict/stale-context signals; Ship Mode and demo readiness; richer decision history and reusable checkpoints. V2 exit: a real multi-person/multi-agent hackathon team completes integration and handoff with reviewable evidence.

### Room V3 — ecosystem and operations

Add adapter SDK and cross-IDE/browser review tiers; skill/capability-aware dispatch; consent-based Auto Rescue; Build Focus/Review/Integration Freeze/Ship/Demo modes; Panic Mode suggestions; project impact graph and contract board; richer collaborative preview/debug and optional CI/CD/Git host integrations; reusable room templates; admin/observability/quotas/retention/data residency at appropriate scale. Voice recording/transcription is optional and consent-gated.

### AI-native expansion

In parallel only when each dependency is real: improve typed capability coverage and provider adapters; resource-aware agent sessions and process isolation; explicit trusted Skills catalog workflow; then a declarative Morph preview format; restricted widget host; Workshop authoring; portable Packs; Mesh/Pipeline scheduler; Pocket mobile participation. Packs/widgets/tools must not execute untrusted generated code before sandboxing, signing/trust and rollback exist. Forge Replay is optional and later. Do not prioritize Twin/Ghost/Universe/What-If as separate flagship initiatives.

### Studio and visual direction

Keep current curated theme foundations readable, original, low-noise over code and accessible. Add theme catalog growth and atmosphere only with preview/restore, contrast checks, opt-in motion/reduced-motion behavior, licensed assets and performance checks. Morph should generate a constrained proposal with preview/undo; full layouts/backgrounds need supported fork-owned workbench APIs. Preserve a reliable plain/default appearance at all times.

### Production room service gate

Do not expose the prototype as a public service until authentication/authorization, key recovery and rotation, durable multi-process storage, invite revocation, rate limits/DoS protections, retention/deletion, operational monitoring, backup/recovery, tenant separation, separate-machine validation and independent security review are designed and exercised. LAN/self-hosted and managed internet are different deployment guarantees.

## 7. Folder map and source of truth

- `extensions/team-lobby/`: room and AI workflow implementation, tests and package scripts.
- `extensions/forge-room-studio/`: themes, profiles, preview logic and tests.
- `services/room-service/`: service prototype and tests.
- `docs/product/`: product requirements and acceptance goals.
- `docs/research/`: feature research and evidence-based audits.
- `docs/architecture/`: integration map and threat model.
- `docs/planning/`: versioned delivery history, prompts reconciliation and current snapshots.
- `docs/source-prompts/`: verbatim source briefs copied from the user's Downloads folder.
- `PROJECT_PROGRESS.md`: current idea inventory, truthful status and continuation roadmap.
- `releases/`: locally retained, generated VSIX archives; excluded from the Git source history.

## 8. Supporting documents

- [Product specification](docs/product/PRODUCT_SPEC.md)
- [Competitor feature research](docs/research/COMPETITOR_FEATURE_RESEARCH.md)
- [AI core feature research](docs/research/AI_CORE_FEATURE_RESEARCH.md)
- [AI-native repository audit](docs/research/AI_NATIVE_REPO_AUDIT.md)
- [Master AI-native audit and gap matrix](docs/research/FORGE_AI_NATIVE_MASTER_AUDIT.md)
- [Threat model and public-service gates](docs/architecture/THREAT_MODEL.md)
- [Implementation map](docs/architecture/IMPLEMENTATION_MAP.md)
- [Detailed build status](docs/planning/BUILD_STATUS.md)
- [Current state snapshot](docs/planning/CURRENT_STATE.md)
- [Complete Room implementation brief](docs/source-prompts/Forge_Room_Complete_Implementation_Prompt.txt)
- [AI-native expansion brief](docs/source-prompts/Forge_AI_Native_Expansion_Master_Prompt.txt)
