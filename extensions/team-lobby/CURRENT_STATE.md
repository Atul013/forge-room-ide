# Forge Room current state

Snapshot: 25 October 2026. Forge Room Studio 0.14.0 and Team Lobby 0.129.0 are current. Team Lobby uses room protocol 20. Participants can add attributed room decisions with a title and rationale; decisions join local searchable history. Tasks support priority and bounded optional effort estimates; triage identifies estimated critical paths. Hosts can define up to 20 future room milestones, shared with participants and persisted with room state. Readiness compares estimated remaining work with active countdown and future milestones, excluding completed tasks from milestone estimates; manually reported results do not count as IDE-recorded verification and a newer manual outcome makes an older IDE result unknown; incomplete estimates/dependencies and paused/unset countdown feasibility remain unknown. Preview, deployment and submission integrations remain unknown/manual. Task owners can opt into local reminders at 15 minutes, 5 minutes, and task deadline; the setting defaults off and reminders never leave the device.

## Verification

Team Lobby 0.129.0 passes TypeScript builds and 233/233 automated tests in both handoff and Code OSS copies. The webview syntax check passes. Task owners can opt into 15-minute, 5-minute, and deadline reminders; default is off. The nine-file VSIX SHA-256 is `55D685464D3EB5DFE22C1E92CFB28B0A0DA61C94CA9A05462164F3154D71C10F` and matches six retained package copies and three product manifests. Disposable-profile VS Code installation succeeds; live reminder behavior remains unverified. Interactive cross-client decision search, provider and multi-machine acceptance remain open.

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

## Team Lobby 0.109.0 - explicitly approved room task context

Ask, Plan, and Build can optionally include a room task metadata snapshot capped at 50 tasks and 12,000 characters. The preview names every field sent; chat, direct messages, notes, diffs, and file contents are excluded. Both copies pass TypeScript checks and 181/181 tests. The nine-file VSIX SHA-256 is `185703A4563CA1DE9D0F96E2148D1121DC1BE41DF0FBAEF2ADB321936B636DA6`, synchronized with retained packages and the Code OSS product manifest. Real-provider and live IDE acceptance remain open.

Team Lobby 0.110.0 adds a local recommendation-only task triage command. It ranks open room tasks by blockers, deadlines, in-progress status, and downstream dependents, then opens existing actions after selection. No automatic task changes or agent starts occur. The IDE QuickPick interaction remains to validate.

## Team Lobby 0.110.0 - local room task triage

The triage command ranks open tasks by blockers, deadlines, in-progress state, and downstream dependents; selected tasks open existing task actions. No automatic task changes or agent starts occur. Both mirrors pass TypeScript checks and 184/184 tests. VSIX SHA-256: FE927C7CDA57D582374200CBCA8FE32906D8945FF45F2A1095AD6CD56857E2F3. Interactive QuickPick acceptance remains open.

## Team Lobby 0.111.0 - opt-in local countdown milestone alerts

The `teamLobby.countdownAlerts` setting defaults off and shows local reminders for the shared countdown at 60, 30, 10, and 5 minutes and at expiry. Delivery state is bounded and persisted locally. Both extension copies pass TypeScript checking and 188/188 tests. VSIX SHA-256: `5D42753F3EEA43FDF777C3800FF9774FBFA4E2C7401E782BC0C34CA50DE16A3F`. Live timing and settings interaction remain to validate.


## Team Lobby 0.115.0 - task estimates and critical path

Tasks can carry an optional whole-minute effort estimate from 1 to 10,080 minutes. Local triage marks the deterministic longest unfinished dependency chain and shows its total only when all unfinished tasks have valid estimates and dependencies. Missing estimates and unavailable dependencies are reported as unknown. Estimates appear in task cards, readiness summaries, and the explicitly approved room task snapshot. Both mirrors pass TypeScript checks and 203/203 tests; media/room.js passes the room webview syntax check.


## Team Lobby 0.116.0 - critical-path countdown feasibility

The local demo-readiness report compares the estimated unfinished critical path with the active shared countdown. It reports attention when the estimate exceeds remaining time, ready when the estimate fits, and unknown when any unfinished estimate or dependency is missing. Paused and unset countdowns remain unknown because this check cannot establish deadline feasibility.

**Verification:** both extension copies pass `npm.cmd run check` and 205/205 tests; room webview syntax checks pass. Nine-file VSIX SHA-256 is `2EF5B3F97CCD4905C9CB141BBCD72FE60995E4A11972602155D170442A1C7E68` and matches retained packages and product manifests. Interactive readiness generation remains unverified.
