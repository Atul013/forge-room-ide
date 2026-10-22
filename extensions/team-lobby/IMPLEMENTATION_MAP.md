# Implementation map

## Repositories

- Product checkout: `C:\Users\Admin\Documents\Codex\2026-10-20\che\code-oss` (Code OSS 1.144.0).
- Handoff workspace: `C:\Forge-Room-IDE-Handoff`; extension/service sources, tests, research, and release packages.
- Product requirements: [PRODUCT_SPEC.md](PRODUCT_SPEC.md). Current gap evidence: [FORGE_AI_NATIVE_MASTER_AUDIT.md](FORGE_AI_NATIVE_MASTER_AUDIT.md).

## Main components

### Team Lobby 0.108.0

- `extensions/team-lobby/src/extension.ts`: room lifecycle, tasks, collaboration, Ask/Plan/Build, Git review, commands, evidence, and IDE API integration.
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
