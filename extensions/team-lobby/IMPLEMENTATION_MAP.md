# Implementation map

## Repositories

- Product checkout: `C:\Users\Admin\Documents\Codex\2026-10-20\che\code-oss` (Code OSS 1.144.0).
- Handoff workspace: `C:\Forge-Room-IDE-Handoff`; extension/service sources, tests, research, and release packages.
- Product requirements: [PRODUCT_SPEC.md](PRODUCT_SPEC.md). Current gap evidence: [FORGE_AI_NATIVE_MASTER_AUDIT.md](FORGE_AI_NATIVE_MASTER_AUDIT.md).

## Main components

### Team Lobby 0.128.0

- `src/roomMessageSearch.ts`: bounded local message matching and newest-first pagination; participant visibility filtering occurs before query matching.
- `src/roomDecision.ts`: validates and formats bounded decision titles and rationales before explicit room confirmation.
- `src/roomThreads.ts`: parent-linked message grouping and host validation for public/DM reply audience. Webview thread controls open messages and submit replies.

- `src/roomThreads.ts`: parent-linked message grouping and host validation for public/DM reply audience. Webview thread controls open messages and submit replies.

- `extensions/team-lobby/src/extension.ts`: room lifecycle, tasks, collaboration, Ask/Plan/Build, Git review, commands, evidence, and IDE API integration.
- `src/manualVerification.ts`: validates and formats participant-reported room verification outcomes, clearly labelled as manual.
- `src/agentCapabilities.ts`: typed workspace capabilities, scope validation, approval summaries, and request-grant separation.
- `src/workspaceInventory.ts`: bounded paths-only inventory formatting with sensitive-path and traversal rejection.
- `src/workspaceTasks.ts`: bounded registered-test task listing and exact name/provider matching.
- `src/toolEvidence.ts`: content-free, bounded local tool evidence and outcome summaries.
- `src/roomServiceClient.ts`: HTTPS account/room/invite client and WSS-to-loopback room protocol bridge.
- `src/agentSkills.ts`, `src/planArtifact.ts`: selected Agent Skills metadata and structured Plan export.
- `media/room.js`, `media/room.css`: Project Room webview.
- A confirmed 24-hour hackathon preset and host-only countdown pause/resume; frozen time persists, synchronizes on reconnect and records attributed activity.
- Explicit per-file live Git diff snapshots refresh after saves, at most once every five seconds, with a visible stop control and retained history.
- AI Ask/Plan/Build scopes context files, editor selection, diagnostics, captured terminal output, task worktree validation, saved-session continuation, Plan-to-Build handoff, and plan export to the exact chosen root in multi-root workspaces.
- 0.109.0: Ask/Plan/Build can include an explicitly previewed and approved room task metadata snapshot, bounded to 50 tasks and 12,000 characters; no chat, notes, diffs, or file contents are included.
- 0.110.0: the local task triage command ranks runnable tasks using blockers, deadlines, in-progress state, and active downstream dependents. It opens existing task actions only after selection and makes no automatic task or agent changes.
- 0.111.0: optional, default-off countdown alerts show local reminders at 60, 30, 10, and 5 minutes and at expiry. Per-deadline reminder state is bounded and persisted locally; no room event or transfer is created.
- 0.112.0: the local demo-readiness command saves a Markdown snapshot of room task/dependency/deadline and recent verification evidence. Preview, deployment, and submission gates remain explicitly unknown/manual.
- 0.113.0: tasks support low/normal/high/critical priority in create/update, cards, recommendation ranking, readiness output and the explicitly approved AI task snapshot. Older tasks default to Normal.
- 0.114.0: demo-readiness output includes the room countdown; unset is unknown, paused/expired/within one hour needs attention.
- 0.115.0: bounded optional task estimates appear in cards/readiness/approved AI context; local triage marks and sorts the longest estimated unfinished dependency path, remaining unknown when active estimates or dependencies are incomplete.
- 0.116.0: local demo-readiness report compares estimated critical-path duration to active shared countdown; overrun needs attention, incomplete estimates or dependencies and paused/unset countdown remain unknown.
- 0.117.0: host manages up to 20 named future room milestones; milestones persist, sync with countdown state, display to all participants and flag passed/within-hour items in readiness.
- 0.118.0: readiness compares each future milestone with the estimated unfinished dependency chain, excluding completed tasks; overruns need attention, incomplete estimates/dependencies stay unknown.
- 0.119.0: participants can explicitly share manually reported test/build/lint/type-check outcomes, optionally linked to a room task; the feature runs nothing and sends no command or output.
- 0.120.0: demo readiness excludes participant-reported results from IDE-recorded verification and never treats them as verified passes.
- 0.121.0: locally export metadata-only room activity and task summaries; private messages, event details, file paths, code and patches are omitted.
- `services/room-service/`: optional self-hosted HTTPS/WSS control plane and relay; active room content is not persisted by the service.
- `test/`: policy and integration tests. Run `npm.cmd run check` and `npm.cmd test` in the extension directory.

### Forge Room Studio 0.14.0

- `extensions/forge-room-studio/`: supported workbench themes, preview gallery, and allowlisted appearance profile import/export. Studio 0.14.0 adds temporary project-profile preview with Keep/Restore, rollback on partial apply failure, and merge of supported colors/token scopes over existing customizations. Studio 0.13.0 added temporary theme preview; 0.12.0 added contrast tests for six palettes. Motion remains opt-in and follows system reduced motion. It does not modify workbench DOM or load arbitrary theme code.

## Current AI test-task slice

- `tests_discover` is an approved read-only capability in Ask/Plan/Build. It exposes registered Test-group task names and provider labels only.
- `tests_run` is available in Build only. For duplicates, the user chooses a specific match before the separate confirmation modal. Request-wide read-only approval never authorizes it.
- Project task code runs with the user's OS privileges and may modify files or use the network. Output stays in the IDE task terminal; only process exit status is sent to the model. Cancellation/timeout requests termination and may leave outcome unverified.
- 0.104.0: Agents may request the latest selected-root terminal capture after a separate exact per-call approval; broad read-only grants do not authorize it. Local evidence stores no command or terminal text.

## Remaining product and acceptance work

The room relay is self-hostable, not a managed public SaaS, and does not store shared room content. Voice, mobile, persistent hosted agents, remote IDE control, Widgets, Packs, Workshop, and full prompt-driven Morph remain unimplemented. Real provider-backed AI runs, two separate IDE clients, and two-machine relay acceptance remain open. ACP/MCP subprocesses have ambient local OS privileges and are not sandboxed by the extension.

- 0.105.0: Build can request an exact unique text replacement; Forge opens the full diff, requires a dedicated approval, rejects stale document versions, applies unsaved via WorkspaceEdit, and keeps evidence content-free.

- 0.106.0: Exact approved Build edits save normal rollback checkpoints before applying. The standard rollback path restores only unchanged file content and reports a conflict if later edits exist.

- 0.107.0: Ask, Plan, and Build can enumerate up to 250 non-sensitive workspace-relative file paths without reading file contents.

- 0.108.0: Ask, Plan, and Build can open a safe project file and reveal an optional line after separate per-call confirmation.
