# Forge Room current state

Snapshot: 21 October 2026. Forge Room Studio 0.14.0 and Team Lobby 0.108.0 are current. Team Lobby uses room protocol 17. Its AI workflow scopes context and tools to the selected project root, including bounded file inventory, direct editor navigation, diagnostics, terminal output, worktree validation, ACP session continuation, Plan-to-Build handoff, debugger stack inspection, and checkpoint-backed exact-match edits.

## Verification

Team Lobby 0.108.0 passes TypeScript checks and 179/179 automated tests in both the handoff and Code OSS extension copies. The 17-file VSIX SHA-256 is `6EF2EC09E51E25884577C159AA19382A947C682C6F1487D080C9B6ACF8F0FBFA` and matches both handoff package copies, the Code OSS extension package, and the built-in product manifest. Forge Room Studio passes 16/16 tests and the room service passes 9/9 tests. The Code OSS corepack yarn compile passed before the 0.104 and 0.105 extension updates; this slice does not change workbench source. Live provider-backed edits, the rendered diff/Undo/checkpoint rollback flow, two-IDE and two-machine behavior remain unverified.
## Team Lobby 0.99.0 multi-root AI request scoping

AI Ask/Plan/Build now prompts for a project root in multi-root workspaces. Context files, editor selection, diagnostics, Build worktree checks, worktree creation, and Build resume validation stay scoped to that root. Single-root workspaces continue without an extra prompt. Automated path coverage rejects files outside the chosen root.

## Team Lobby 0.100.0 multi-root AI session continuity

Saved Ask/Plan/Build continuations now resolve the session's exact workspace-folder URI rather than comparing only the first folder. Plan-to-Build handoff remains pinned to the Plan's project root, and saved plans default to that root. If the original root is no longer open, Forge Room explains that it must be added back. A focused test covers selecting a non-first root and rejecting a missing root. Both Team Lobby mirrors pass TypeScript checks and 158/158 tests; live multi-root IDE/provider validation remains open.

## Team Lobby 0.101.0 multi-root terminal context

Captured terminal commands now record their workspace root from shell integration's reported cwd. In a multi-root workspace, Ask/Plan/Build only offer captured output from the selected root; unknown-cwd captures are excluded. A command receives a root only when the shell reports the same workspace at start and end. Root-known captures remain excluded from other roots if the workspace later changes. Single-root workspaces continue to offer explicitly captured output when its cwd is unavailable. Both Team Lobby copies pass TypeScript checks and 161/161 tests; multi-root IDE/runtime behavior remains to validate.

Team Lobby 0.103.0 VSIX SHA-256: 18E3F8105A5E41330F0392BB3AA3CBA3E4BC8DB107A88C321C21629DE4615550.

## Studio 0.13.0 live preview, contrast, and opt-in motion

Decorative Studio entrance animations and transition movement are off by default. The accessible Motion control persists its preference in webview state; the operating system reduced-motion setting always overrides it. Theme selection now applies a temporary preview and requires an explicit Keep choice; restoring or dismissing returns the previous appearance. All six themes are tested for 4.5:1 core workbench/syntax text contrast and 3:1 focus-border/inactive-activity-icon contrast. Tests also cover default-off motion, enable/disable, reload persistence, and system reduced-motion override. Both Studio mirrors pass syntax checks and 12/12 tests. Interactive screen-reader/accessibility validation remains open.

Studio 0.13.0 VSIX SHA-256: 8D530420468C9CE543BF1DE0A79994978C98E0847867FC4CF79705CD0DEF0EF3.

## Studio 0.14.0 reversible project profile preview

Project profile import now previews the selected profile in the active user or workspace scope and asks whether to keep it or restore the previous settings. Dismissal and partial-apply errors restore the saved values. Imported supported color and syntax settings merge over existing customizations, preserving unrelated colors, token scopes, and rule settings. Tests cover profile merge behavior and preview rollback, including failure partway through applying settings. Both Studio copies pass syntax checks and 16/16 tests. Live IDE profile interaction remains open.

Studio 0.14.0 VSIX SHA-256: EB9ADD6F074E3FD327F4062B4AC515F553347A63224B1262ED46125475B41502.

## Implemented foundation

Forge Room is a Code OSS fork with Forge Room Studio themes and profiles, and Team Lobby collaboration, room tasks and handoffs, explicit sharing, Git activity, shared notes/documents, worktrees, and provider-neutral Ask/Plan/Build. AI requests use selected context, transfer consent, bounded approved capabilities, reviewed diffs, checkpoints, verification evidence, recovery, and exportable review artifacts.

## Remaining acceptance and product work

Live acceptance in separate IDE clients and across two machines remains open, as does validation with supported real AI providers. The attached AI-native expansion prompt is being implemented incrementally; major staged areas include calls, mobile, persistent hosted agents, remote IDE control, widgets, Workshop, Packs, richer workspace morphing, and durable gated pipelines. ACP and MCP child processes run with local OS privileges and are not sandboxed by the extension.

## Team Lobby 0.103.0 ACP session continuation

Ask/Plan ACP agents can continue a previous session when the agent advertises `loadSession`. Forge asks before restoring the external conversation history and checks that the selected authentication method matches. It stores only the session ID and auth-method ID in local workspace state. Replayed history is excluded from the new answer; agents without the capability start a new session. Both extension copies pass TypeScript checks and 168/168 tests. The 17-file VSIX SHA-256 is `18E3F8105A5E41330F0392BB3AA3CBA3E4BC8DB107A88C321C21629DE4615550`; agent-specific and interactive IDE behavior remain to validate.


Team Lobby 0.105.0 adds a Build-only, diff-previewed, separately approved, undoable single-occurrence text edit capability. Both copies passed checks and 174/174 tests for that release.


Team Lobby 0.106.0 saves each exact approved Build edit as a standard single-file checkpoint before applying it. The existing Roll Back Last Build command uses the normal conflict guard; unrelated later edits prevent restoration instead of being overwritten.


## Team Lobby 0.107.0 workspace inventory

Ask, Plan, and Build can request a bounded list of non-sensitive file paths in the selected workspace. The capability returns paths only, skips common generated folders, enforces selected-root containment and output limits, and stores content-free evidence. Both extension copies pass TypeScript checks and 178/178 tests. The 17-file VSIX SHA-256 is `A25DF060D7E6FA2C21088376830A58874C1B320A52B2CD3800EB06EC3FDAA380` and matches the built-in manifest and package copies.


## Team Lobby 0.108.0 - editor navigation capability

Ask, Plan, and Build can request opening one non-sensitive file in the selected project and optionally reveal a one-based line. The IDE checks file containment and existence; invalid lines are rejected. A separate confirmation is required for every navigation call, even under a request-wide read-only grant. The model receives only the open result, not file contents. Both extension copies pass TypeScript checks and 179/179 tests. The 17-file VSIX SHA-256 is 6EF2EC09E51E25884577C159AA19382A947C682C6F1487D080C9B6ACF8F0FBFA and matches the built-in manifest and package copies. Live provider behavior remains unverified.
