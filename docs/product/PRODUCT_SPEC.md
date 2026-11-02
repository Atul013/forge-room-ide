# Forge Room IDE â€” Product brief

**Status:** Product direction informed by competitor research, 20 October 2026  
**Platform:** Desktop IDE based on Code OSS  
**Audience:** Individual developers, small teams, hackathon teams, and human/AI groups

For the detailed, feature-by-feature analysisâ€”including how competitor features work, tradeoffs, and Forge Room recommendationsâ€”see [COMPETITOR_FEATURE_RESEARCH.md](../research/COMPETITOR_FEATURE_RESEARCH.md).

## The idea in one sentence

Forge Room is a capable, customizable developer workspace where people and coding agents work on the same project, make their progress visible, and coordinate through a project room.

## Our product promise

1. **A real IDE first.** Opening and working on a project must feel dependable before we add novelty.
2. **Make it yours.** Developers can shape the editor's colors, art direction, icons, and workspace identity without sacrificing readability or performance.
3. **Make teamwork visible.** People can tell who is here, what they own, what changed, and what needs attention.
4. **Make agents accountable teammates.** An agent has an identity, task, activity trail, controls, and reviewable output.
5. **Keep users in control.** Sharing, AI access, commands, and file changes have clear boundaries and understandable choices.

## Who it is for

- **Solo developers** who want a comfortable editor and optional AI assistance.
- **Small teams and hackathon groups** who need quick coordination without constant status meetings.
- **Human + agent teams** that want agents' work to be visible, attributable, and reviewable.
- **Theme and customization enthusiasts** who want their workspace to express a personal style, fandom, or project identity.

## The core product, simplified

### 1. The dependable workspace

Start from Code OSS. Preserve the normal developer loop:

- Open folders and multi-root workspaces.
- Edit, search, navigate, refactor, and use language extensions.
- Run and debug code; inspect tests and problems.
- Use Git, diffs, merge conflict tools, and the integrated terminal.
- Install compatible extensions, configure shortcuts, and use keyboard navigation and assistive technologies.
- Support local projects first, with remote development as a compatible capability rather than a separate product model.

**Acceptance:** A developer can open a representative project, edit files, use Git, run a task, debug, and install compatible extensions without needing the room feature.

### 2. The expressive workspace

Make the full IDE personal while keeping code comfortable to read:

- Switch between curated **workspace themes** such as anime-inspired, retro terminal, cozy, minimal, or high-contrast.
- A theme can coordinate workbench colors, editor syntax colors, file/product icons, typography choices, and optional background artwork.
- Preview before applying; restore the previous theme quickly.
- Save and share a theme or a complete project profile. Themes must not silently install extensions or change keybindings.
- Offer light, dark, and high-contrast variants, plus readable defaults for every theme.
- Respect reduced motion. Decorative movement is opt-in and never runs over source text.
- Provide clear safety boundaries: theme assets are local or from trusted packages; no remote code execution or hidden tracking in a theme.

**Acceptance:** Users can preview and apply a packaged theme, switch back, and use the IDE with readable contrast, keyboard navigation, and reduced motion.

### 3. The project room â€” our signature feature

Every project can have a room that brings human and agent work together:

- Create, join, leave, and invite people to a project room.
- Show who is present, whether they are human or agent, and what task they own.
- Share short project updates and room messages; send direct messages.
- Show a chronological activity feed for presence, task changes, file edit/save events, commits, tests, reviews, and handoffs.
- Let participants open a teammate's changed file in their own workspace. Show diffs only when the sharing model explicitly supports it.
- Keep task ownership, status, dependencies, and handoff visible. The host must enforce task updates against participant identity: only the assigned owner or host can update an existing task, and only the host can reassign owners or reopen completed tasks.
- Give each activity clear attribution and time. Distinguish working changes from tested, reviewed, committed, and merged work.
- Use LAN rooms as an initial transport only. Internet rooms require an authenticated, encrypted service.

**Acceptance:** Two editor instances on a LAN can join the same room, see one another, post updates, observe file/task activity, disconnect and reconnect, and distinguish private data from shared data.

### 4. AI and agent work

AI is a capability of the workspace and room, not the identity of the whole product:

- Support model/provider adapters rather than tying the room protocol to one vendor.
- Provide distinct Ask, Plan, and Build modes: understand without changing files; propose a plan; implement and verify.
- Let the user choose which files, symbols, selection, diagnostics, terminal output, and web sources enter the context.
- Show the agent's task, current activity, files changed, commands run, and verification results.
- Let users pause, redirect, stop, inspect, and revert agent work.
- Use isolated Git worktrees for parallel agent tasks when possible.
- Keep approval controls understandable. Destructive commands, secret access, external network access, and sharing file contents require deliberate policy and clear visibility.
- End an agent task with a reviewable change set and evidence: checks run, results, failures, and remaining risks.

**Acceptance:** A user can ask, plan, implement, inspect the diff, see verification evidence, and revert an agent's changes without losing unrelated work.

## What existing IDEs teach us (summary)

This is a synthesis of public vendor documentation, not a feature-by-feature hands-on benchmark. Capabilities, names, and availability change; treat these as design references rather than guarantees of parity.

| Product | Strong patterns to learn from | Forge Room response |
| --- | --- | --- |
| **VS Code / Code OSS** | Integrated editing, debugging, tests, Git, terminal, language ecosystem, layouts, profiles, extension themes, and remote workflows. | Keep this as our dependable base. Preserve extension compatibility, workbench flows, profiles, and keyboard habits. |
| **Cursor** | Codebase-aware retrieval, inline/multi-file edits, autonomous agents that use tools, plan/review flows, and command confirmation. | Make repo context useful and visible; review multi-file changes clearly; let users tune autonomy. |
| **Windsurf** | Context-aware agent workflows, persistent workspace guidance, model comparisons in isolated worktrees, and lifecycle hooks. | Keep shared project knowledge explicit; allow safe parallel task experiments; add hooks only with clear permissions and auditability. |
| **Zed** | Project channels, real-time presence/collaboration, shared projects, persistent room notes, provider choice, local/self-hosted model options. | Make the room a durable place for project coordination, not just an ephemeral chat. Protect local files with explicit sharing scopes. |
| **JetBrains IDEs / Junie** | Deep project-aware IDE tooling, plan/default/autonomous modes, runtime-debug assistance, rollback, and integrated model choice. | Preserve strong language/debug/test workflows; offer clear autonomy modes and reliable per-task rollback. |
| **GitHub Copilot** | Agent tools, plan and build modes, repository instructions, custom agents, MCP connections, permissions, and code review. | Make project instructions portable; expose tool permissions and context; connect review evidence to room activity. |

### Patterns worth adopting

- **One workspace for the full development loop**, not a thin editor wrapped around a chat panel.
- **Repository-aware search and context selection** so AI can find relevant code without asking users to paste it all.
- **A deliberate Plan â†’ Build â†’ Review loop** for large tasks.
- **Visible permissions and approvals** for command execution and sensitive context.
- **Diff-first change review**, with checkpoints and rollback.
- **Project-scoped instructions and shared knowledge**, with provenance and owner control.
- **Git worktrees for parallel tasks** to reduce file conflicts.
- **Persistent project rooms, presence, and notes** that bridge active pairing with async handoffs.
- **Profiles and themes** that let appearance and project setup travel together when the user chooses.

### What not to copy blindly

- Do not send code, prompts, embeddings, terminal output, or file lists off-device without explaining the destination, purpose, and controls.
- Do not present uncommitted changes as delivered work.
- Do not share a local filesystem simply because someone joined a room.
- Do not run commands or edit sensitive files under an unclear autonomy setting.
- Do not put excessive animation or wallpaper behind code, diagnostics, or dense collaboration data.
- Do not build every competitor feature before validating the core workflow with real users.

## The Forge Room difference

Other tools combine editors with AI, and some provide realtime collaboration. Forge Room's distinctive opportunity is to make the **project room the coordination layer for parallel human and agent work**:

- Presence is tied to task ownership and current activity.
- Agent and human changes share one attributed timeline.
- Work can move from task â€” files â€” diff â€” verification â€” review/handoff.
- The room supports live pairing and async catch-up.
- A living project blueprint can record reviewed architecture, decisions, owners, and interfaces.
- Dependency-aware alerts can tell affected owners why an API or shared file change matters.

These are product opportunities, not claims that the current prototype already implements them.

## Theme direction: expressive, readable, and safe

The visual goal is **a recognizable Forge Room workspace with radically different moods**, not a new unrelated interface for every theme.

### Theme layers

1. **Workbench skin:** window chrome, activity bar, sidebars, panels, tabs, status bar, menus, and dialogs.
2. **Editor scheme:** syntax highlighting, diagnostics, selection, diff colors, and cursor visibility.
3. **Icon set:** file, product, and interface icons.
4. **Typography:** interface and code font choices with fallbacks.
5. **Atmosphere (optional):** subtle backgrounds, small decorative details, and restrained motion in non-code surfaces.

### Theme controls

- Browse curated themes by mood and preview them live.
- Switch between theme, editor scheme, icon pack, and atmosphere independently.
- Save combinations as personal presets or project profiles.
- Share/import a theme bundle with a manifest of included assets and settings.
- Provide compatibility, contrast, high-contrast, and reduced-motion checks before publishing/applying.
- Theme visuals never change editor behavior, permissions, network access, installed extensions, or keybindings by default.

### Example launch collection

- **Neon Ronin:** anime city at night, restrained vermilion accents, readable charcoal editor.
- **Starship Deck:** warm instrument-panel tones, clear status colors, compact data hierarchy.
- **Moss Shrine:** quiet greens and parchment surfaces with low visual noise.
- **Ink & Ember:** near-black graphite, warm paper text, ember activity accent.
- **Paper Studio:** light neutral surfaces, strong code contrast, minimal decoration.

Anime themes can include artwork and character motifs in welcome, room, and empty states; keep source code surfaces calm and legible.

## Product principles

- **The IDE still works without AI or rooms.**
- **The room never silently shares file contents.** Presence, metadata, paths, diffs, and content are separate sharing permissions.
- **An AI action is observable and reversible.** Show intent, files, commands, results, and rollback.
- **Themes alter appearance, not trust.** Visual customization cannot weaken security settings.
- **Use open standards and existing extension surfaces where practical.** Avoid breaking Code OSS behavior to duplicate it.
- **Ship small slices and label their state.** Mark each feature as planned, implemented, tested, or unavailable.

## Roadmap and exit checks

### Phase 0 â€” Research and decision record

- Keep this competitor synthesis and source list current.
- Confirm user priorities and distinguish must-haves from later ideas.
- Test early visual concepts with representative developers, including reduced-motion and high-contrast needs.

### Phase 1 â€” IDE foundation

- Rebase and compile the branded Code OSS fork reproducibly.
- Exercise edit, search, language services, run/debug, tests, terminal, Git, extensions, keyboard navigation, and workspace restore.
- Document marketplace, update, licensing, and extension-compatibility boundaries.

### Phase 2 â€” Customization foundation

- Define Forge Room design tokens and theme bundle format.
- Build curated light/dark/high-contrast themes, icon choices, typography controls, preview, reset, and project profiles.
- Add optional non-code atmosphere with reduced-motion and performance controls.
- Exit when users can install/preview/switch/restore themes and complete normal editing tasks in each.

### Phase 3 â€” Room MVP (signature feature)

- Replace the current single-process prototype with a versioned, testable room service boundary.
- Implement reliable identity, invites, presence, reconnect, persistence, and attributed activity.
- Ship room chat, direct messages, task ownership/status, and LAN collaboration.
- Define opt-in sharing scopes before file diffs, terminal data, or content.
- Exit with successful two-machine LAN validation and a documented threat model.

### Phase 4 â€” Change visibility and handoff

- Associate people/agents and tasks with Git worktrees.
- Show file ownership, diffs, conflict signals, tests, review results, and handoff states.
- Keep private files and terminal output excluded by default.
- Exit with a small team completing a task from assignment through review.

### Phase 5 â€” Agent workflows

- Add Ask, Plan, and Build modes, project instructions, context selection, and model/provider adapters.
- Add tool permission levels, checkpoints, pause/stop, worktree isolation, rollback, and proof-carrying results.
- Connect agent progress and evidence to room tasks and activity.

### Phase 6 â€” Internet rooms and team service

- Add TLS, authentication, authorization, invitation expiry, durable service, audit trail, and administration.
- Threat-model file, workspace, model-provider, agent-command, and theme-package boundaries before public deployment.

## Current implementation snapshot - 21 October 2026

The repository contains the Code OSS fork, Forge Room Studio 0.14.0, and Team Lobby 0.108.0. This summary describes checked-in implementation and automated evidence; it does not claim every feature has passed acceptance in separate IDE instances or machines. Team Lobby 0.107.0 adds bounded AI workspace inventory. Team Lobby 0.108.0 adds confirmed AI editor navigation to safe project files and optional lines without transferring their contents.

### Multi-root AI request scoping

Ask, Plan, and Build prompt for the project root when the IDE has multiple workspace folders. Context file discovery and resolution, editor selection, diagnostics, terminal output, task worktree checks and creation, saved-session continuation, Plan-to-Build handoff, saved plan export, and saved Build resume checks use the selected root. Terminal output is associated with a workspace root only when shell integration reports a cwd; unknown-root captures are excluded from multi-root requests. A one-root workspace keeps its direct flow. Path behavior is covered by the extension test suite. Separate live multi-root IDE/provider acceptance remains open.

### Forge Room Studio

Studio offers six built-in themes, temporary live theme and project-profile previews with explicit Keep/Restore, allowlisted editor/workbench color profiles, seven common syntax token scopes, contrast warnings, and profile import/export. Profile imports preserve unrelated color and syntax customizations and roll back if a setting fails during preview. Decorative gallery motion is opt-in, persists across webview reloads, and is suppressed whenever the operating system requests reduced motion. Automated theme checks require 4.5:1 contrast for core workbench and syntax text, and 3:1 for focus borders and inactive activity icons. Advanced token-scope authoring, installing arbitrary theme extensions, a complete theme editor, prompt-generated full-workbench morphing, and live keyboard/screen-reader acceptance remain open.

### Team Lobby collaboration

Team Lobby provides self-hosted room host/join, 12-hour bearer invites, invite revocation, participant identity pinning, synchronized presence, chat and direct messages, owner-controlled notes, attributed activity, bounded host-local history and user-controlled export. Plain WebSocket binds only to loopback or a private IPv4 LAN interface. Public-address hosting requires TLS 1.2 or newer and a trusted certificate; room invites authenticate clients. A local WSS check accepts trusted certificates and valid invites, and rejects invalid invites and untrusted certificates. It does not establish separate-machine internet routing or a managed multi-tenant service.

Tasks support participant ownership, host-authorized reassignment, handoff, dependencies, deadlines, acceptance criteria, and task-linked Git worktrees. Git diffs and anchored comments require explicit sharing; file activity is opt-in. Live Git diff snapshots can follow saved changes to one explicitly selected tracked file, with a five-second update limit and a visible stop control; already shared snapshots remain in room history. Editor follow sends only workspace-relative path and line. Live document sharing uses Yjs, is started by the host, requires per-participant approval, supports uniquely named workspace roots, is limited to 512 KB per document, and keeps changes unsaved and session-only. The room protocol is version 17.

Team Lobby 0.97.0 adds a confirmed 24-hour hackathon quick start and host-controlled pause/resume for the room submission countdown. The host's deadline and paused remaining duration are persisted with local room history, included in reconnect state, and announced through attributed room activity. Only the host can operate the countdown; participants can view its frozen remaining time. This release uses room protocol 17.

Team Lobby 0.98.0 adds opt-in live Git diff snapshots for one selected tracked file. Saved changes publish a new full patch at most once every five seconds, after explicit review and confirmation. The active file and stop control are visible in the room panel. Sharing ends on room leave, an empty diff, an oversized patch, or a Git refresh error. Previously shared patches are retained in room history.

### AI and agent workflows

Ask, Plan, and Build use the IDE Language Model API with provider/model choice, per-request context and transfer consent, provider token estimates, bounded read-only workspace capabilities, optional project guidance, selected Agent Skills and reusable profiles. Plan handoff is reviewable. Build changes are reviewed before apply, with task-linked checkpoints, verification evidence, pause/continue/rollback, and optional local session recovery. ACP v1 is an optional Ask/Plan adapter; when an agent advertises `loadSession`, Forge Room can continue a prior session after explicit consent. Only the session ID and selected authentication-method ID are stored in workspace state; prior history is replayed by the agent and is not re-sent as Forge context. The process runs with ambient OS privileges and is not sandboxed. Ask/Plan can also select configured local MCP stdio servers and call their tools with per-call approval. MCP processes are not sandboxed. Build, Plan, and Ask can request the latest captured terminal output for the selected workspace, with an exact, mandatory per-call approval that names the redacted command, status, and captured size; terminal data is never sent under the general read-only tool grant. OS-isolated command execution is not implemented.

Team Lobby 0.89.0 adds local MCP stdio tool integration to Ask/Plan using the official TypeScript MCP client SDK 2.3.1. Machine-scoped `teamLobby.ai.mcp.servers` settings accept up to eight named command/argument/working-directory entries. The user selects servers per request; Forge Room starts them only after provider-transfer confirmation, offers at most 40 advertised tools with per-tool schemas capped at 8 KB and aggregate schemas capped at 120 KB, prompts separately for every call with the complete JSON arguments (calls above 3,000 characters are rejected), bounds each result to 12,000 characters and the turn to 30,000 characters/six calls, and closes server processes at turn completion. Content-free local audit entries record actor, workspace, server/tool, approval, outcome, and output size. Build and ACP requests do not use this integration. This first slice does not support remote HTTP servers, OAuth, MCP resources/prompts, persistent tool grants, or OS process isolation. Configure only trusted servers; they run as the IDE user's OS account and can independently access files, run commands, or use the network. Settings must not contain secrets. Both Team Lobby mirrors pass TypeScript checking/build and 128/128 extension tests, including an MCP stdio handshake/list/call/close integration. The nine-file VSIX SHA-256 `97662D3A9C31469081038A987B5462E4A3793A2C540D9A9A8BA3CE311036AD98` matches Code OSS built-in metadata. Third-party server/provider and manual IDE UI acceptance remain outstanding.

Team Lobby 0.90.0 hardens MCP startup and cancellation. Startup and tool discovery use a 15-second request timeout and a cancellable progress flow; MCP tool calls use a two-minute timeout and inherit request cancellation. Timed-out startup closes every process opened for the request; canceling an active turn aborts the MCP request and closes its local server session when the turn exits. Fake stdio-server tests cover stalled initialization, stalled tool discovery, and stalled tool execution. Both mirrors pass typechecking/build and 130/130 tests; the nine-file VSIX hash `09DD41F4932A2A17462FD203C668D249DF50711D8CC6154CCE9376805888E42E` matches Code OSS built-in metadata.

### Verification and remaining gates

Team Lobby 0.85.0 adds an optional HTTPS-configured account-backed room service relay. Account credentials use VS Code SecretStorage. The extension connects the service WSS endpoint to a loopback-only local authority, creating expiring invites through the service and supporting invite revocation that disconnects active guests. The service relays protocol-16 traffic but does not persist room messages, tasks, notes, or shared buffers. Plan exports now include consistent request, plan, model, time, and optional room-task sections. Both extension copies pass TypeScript checks and 121/121 tests; both service copies pass 6/6 tests. The nine-file VSIX SHA-256 is `AB011EE13810CF47F5A85209AFE7ED6E5683B17ACC5CD7CD8873504F32FEE646`, matched in the Code OSS built-in extension manifest.

Separate two-IDE room/shared-editing/reconnect acceptance, separate-machine TLS routing, and provider-backed Ask/Plan/Build acceptance remain open. The self-hosted `services/room-service` now implements TLS account provisioning, owned room metadata, durable expiring/revocable invites, account disable, lifecycle audit, and a bounded WSS relay integrated by Team Lobby. Active room traffic passes through the relay but is processed by the host's local room authority; content is not persisted by the service. Tenant operations, key recovery/rotation, multi-process storage, monitoring, retention controls, independent review and separate-machine validation remain before managed-service readiness. Release gates and residual security risks are in [THREAT_MODEL.md](../architecture/THREAT_MODEL.md). Voice/mobile, widget/Workshop execution, Forge Packs, and automated pipeline dispatch are later platform projects, not shipped features.
## Research sources

AI workflow feature synthesis and source-by-source recommendations: [AI core feature research](../research/AI_CORE_FEATURE_RESEARCH.md).

Official product documentation, checked 20 October 2026:

- [VS Code core editor overview](https://code.visualstudio.com/docs/editing/getting-started/overview)
- [VS Code editing features](https://code.visualstudio.com/features/code-editing)
- [VS Code profiles](https://code.visualstudio.com/docs/configure/profiles)
- [VS Code color themes](https://code.visualstudio.com/docs/configure/themes)
- [VS Code extension theming API](https://code.visualstudio.com/api/extension-capabilities/theming)
- [VS Code remote development FAQ](https://code.visualstudio.com/docs/remote/faq)
- [VS Code agent tools](https://code.visualstudio.com/docs/copilot/concepts/tools)
- [VS Code agent approvals and permissions](https://code.visualstudio.com/docs/agents/run/approvals)
- [VS Code agent workflows](https://code.visualstudio.com/docs/copilot/chat/chat-agent-mode)
- [Cursor agent overview](https://cursor.com/docs/agent/overview)
- [Cursor agent capabilities](https://www.cursor.com/features)
- [Cursor security and codebase indexing](https://www.cursor.com/security)
- [Windsurf documentation index](https://docs.windsurf.com/llms.txt)
- [Windsurf Cascade hooks](https://docs.windsurf.com/de/windsurf/cascade/hooks)
- [Windsurf model Arena](https://docs.windsurf.com/ro/windsurf/cascade/arena)
- [Zed collaboration](https://zed.dev/docs/collaboration/overview)
- [Zed Channels design and async notes](https://zed.dev/blog/channels)
- [Zed model/provider choices](https://zed.dev/docs/ai/llm-providers)
- [Zed AI privacy](https://zed.dev/docs/ai/privacy-and-security)
- [JetBrains Junie modes and rollback](https://www.jetbrains.com/help/ai-assistant/junie-agent.html)
- [JetBrains integrated agent support](https://www.jetbrains.com/help/ai-assistant/agents.html)
- [JetBrains editor color schemes and fonts](https://www.jetbrains.com/help/idea/configuring-colors-and-fonts.html)
- [GitHub Copilot agents](https://docs.github.com/en/copilot/how-tos/copilot-in-the-ide/use-copilot-agents)
- [GitHub Copilot repository instructions](https://docs.github.com/en/copilot/how-tos/configure-custom-instructions-in-your-ide/add-repository-instructions-in-your-ide)


















## 21 October 2026 update - Team Lobby 0.49.0

Agent lifecycle controls now include pause, resume, and discard for active provider requests. Pause preserves the bounded session and partial answer for later continuation; Build remains review-first and does not apply an incomplete proposal. Discard cancels active generation and clears both saved session forms after the provider call returns. Build and 45/45 extension tests pass. The attached AI-native expansion prompt was reconciled into the repository audit, architecture decisions, and a staged implementation sequence. ACP/MCP, remote collaboration, voice, widgets and packs remain future work with real infrastructure prerequisites.
## Implementation update - Team Lobby 0.50.0

Task dependency rules are enforced during status changes, remote host acceptance, isolated-worktree creation, and Build task selection. Prerequisites must be done before dependent work starts or completes. A prerequisite cannot be reopened while downstream work is active or complete. Focused tests cover direct and transitive dependency cases.

## Implementation update - Team Lobby 0.51.0

Completing a task with acceptance criteria now requires the assigned owner or host to confirm every criterion. The host validates the confirmation list; task cards show owner-confirmed criteria. Confirmation is not labeled as automated verification, and test results stay separate. Editing criteria requires reopening a completed task. 52/52 focused tests pass; runtime room validation remains outstanding.
## Implementation update - Team Lobby 0.52.0

The local member snapshot now carries the stable participant ID already used by room authorization and task ownership; the UI marks local presence with `isSelf`. Task assignment and reassignment can target the local user without emitting the invalid placeholder ID `self`. Saved legacy tasks and task events with owner ID `self` are remapped to the durable host/participant ID during load. 54/54 focused tests pass; LAN runtime validation remains open.

## Implementation status update â€” 21 October 2026

Team Lobby 0.56.0 now routes host WebSocket sessions through a testable connection lifecycle module. Two local clients exercise room history, presence, shared updates, task owner checks, disconnect/reconnect, and task-state recovery; invalid invite and protocol versions are rejected. Build and 60/60 tests pass in the handoff extension and built-in Code OSS copy. Package SHA-256: `95f3b2c4387f579524a19bbf4c94b65e2020747c0bd13b403901ed752d1147b9`. This advances automated evidence for the LAN room acceptance criterion but does not satisfy the requested two-machine IDE runtime validation. The shipped product still lacks authenticated hosted rooms, live shared editing, voice, and several broader custom-workspace features; see the staged gap list in `FORGE_AI_NATIVE_EXPANSION_DELIVERY.md`.


The 0.56 LAN transport integration checks also cover private message delivery: only the sender and target receive the live event, and a nonparticipant's reconnect history omits it. The recipient retains it in reconnect history. This does not replace a two-machine extension-host runtime pass.


## Implementation status update â€” Team Lobby 0.57.0

Room hosting now enforces the product's LAN-versus-internet transport boundary. Unencrypted WebSocket listeners bind only to a private IPv4 interface (or loopback for local-only use); they do not bind to `0.0.0.0`. Public invite addresses require configured TLS certificate and key. Hosting-policy tests cover private/public addresses, interface selection, partial TLS configuration, and malformed public-address values. Build and 63/63 tests pass in handoff and Code OSS extension copies. SHA-256: `CD8BAFD311763AE5906AC9CDEB3AA6E5806DDD96A9915AC0689DB38F051CC33D`. Public TLS service deployment still requires an operator-supplied trusted certificate and correct firewall/router configuration.


## Implementation status update â€” Team Lobby 0.58.0

AI context file selection, project instructions, selected Agent Skills, and model-requested workspace read/search now resolve canonical targets inside the selected workspace before reading. Escaping symlinks are rejected, and likely credential/private-key paths are classified by both selected alias and resolved target path. The extra-sensitive-file confirmation remains distinct from request-wide tool approval. Build and 65/65 tests pass in both source copies; package hash is `1dd732c13e64b3c3e33382f6f6ab9ec8f5526258d408024b264a10806e1497b3`. Provider UI and remote-filesystem URI behavior still need runtime validation.

## Implementation status update â€” Team Lobby 0.59.0

Verification runs after Build now add bounded, redacted command evidence, exit code, and timestamp to the local rollback checkpoint. Review exports label evidence as local-only or room-shared. A command is included in room activity only after explicit per-run sharing consent. Overlong/multiline commands are omitted; redaction is heuristic. Team Lobby TypeScript build and 66/66 extension tests pass, and VSIX packaging is synchronized. IDE runtime consent and room-share flows remain to validate.

## Implementation status update â€” Team Lobby 0.60.0

Build verification now supports selecting up to ten configured workspace checks after apply. Each check runs sequentially with its own confirmation; exit results (including failures and missing process exit codes), timestamps, and sanitized command evidence are retained in the checkpoint and review bundle. Up to twenty outcomes are kept. Room sharing sends each result separately, while command sharing still requires existing workspace opt-in and explicit per-run consent. TypeScript build and 66/66 extension tests pass; IDE runtime task-picker validation remains open.

## Implementation status update â€” Team Lobby 0.61.0

Build checkpoints now include bounded source provenance for approved workspace read/search calls: tool name, relative source paths, returned character count, outcome, and time. Search queries and returned content are not retained. Up to twenty provenance records survive opt-in Build session resume and appear in the Markdown review bundle; its save prompt discloses this metadata. TypeScript build and 70/70 tests pass. Provider-backed and IDE export flows remain to exercise.
## Forge Room Studio 0.10.0 â€” portable custom theme bundles

Studio can export the currently active allowlisted workbench color overrides and supported syntax colors as a named JSON theme bundle, then preview that bundle at user or workspace scope and keep or restore the preview. Bundle values are validated; only 12 supported workbench color IDs and seven common syntax scopes are imported. Unrelated customizations are retained, the active base theme remains selected, potential low contrast is flagged, and bundle code/unsupported properties are ignored. This is custom palette portability, not a dynamic install of arbitrary VS Code theme extensions. Studio syntax checks and 3/3 bundle tests pass; VSIX packaging succeeds (SHA-256: C21D9081812E05236C840D248ECDDF0CD7E08D28ADAF27FBDE08EE35BCAE94D2). IDE settings round-trip remains to validate.

## Implemented AI profile capability

Team Lobby supports optional per-request reusable agent profiles stored in .forge-room/agent-profiles.json. Profiles can be opened with the **Forge Room Agent: Open Reusable Agent Profiles** command and selected for Ask, Plan, or Build. The profile contributes only untrusted workflow/style instructions; it grants no tools or permissions. Its name and instruction character count appear in transfer consent, and its text is included in the provider's exact-request token estimate. Input is bounded and validated (64 KB file, twelve profiles, 8,000 instruction characters each).


## Implementation update â€” Team Lobby 0.63.0

Folder-scoped project guidance is implemented. Ask, Plan, and Build can discover and explicitly select nested AGENTS.md files in addition to root guidance. Discovery skips generated and dependency directories; each selected source is re-resolved inside the workspace and checked for sensitive-path patterns. Context is bounded to ten guidance files, 12,000 characters per file, and 48,000 characters total and appears in the existing source and aggregate transfer receipt. Starter-file creation rejects symbolic-link destinations outside or within the workspace. Team Lobby passes TypeScript checking and 76/76 tests.


## Implementation update â€” Team Lobby 0.64.0

Ask, Plan, and Build now show live progress through model turns and read-only tool requests. After each tool call, progress includes the tool name, outcome, bounded source paths, and returned character count, reusing the content-free evidence model. Search arguments and returned file/search text are excluded. Team Lobby check and 77/77 tests pass.


## Implementation update â€” Team Lobby 0.65.0

Room-linked Build lifecycle and verification activity now carries the task's stable ID in the event record. This lets local room history and downstream review tooling associate progress with the exact task even if titles collide or change. Host-side validation restricts task-linked activity to the assigned owner or host, and the versioned room protocol was raised to 12. TypeScript check and 78/78 focused tests pass.

## Team Lobby 0.66.0 â€” task-owned agent presence

A new or resumed task-linked Build now appears in the room as a separate virtual agent participant, with the selected model label, owning human participant ID, task ID, and working/review status. Presence is sent in room protocol 13, is accepted by the host only when the connected participant owns the linked task, is cleared when the Build exits, and is included in room history/count snapshots for reconnecting clients. Agent cards resolve their task through the owner ID. This does not create a remote agent runtime or grant the agent separate workspace permissions. TypeScript check and all 78 focused tests pass in the Code OSS extension; handoff mirror and VSIX package are synchronized. Provider-backed and multi-window IDE flows remain unverified. VSIX SHA-256: `8a82682364c55aff36ff79bc518c7cc1df25bccb51f928fcd896a83177e6a8db`.

## Master expansion handoff status

The supplied Forge AI Native Expansion master prompt is product direction, not code. Current repo coverage includes Forge Studio appearance profiles and themes, Team Lobby tasks/chat/activity, task-linked agent workflows, guarded context/tools, and local transport tests. The prompt's remaining platform-scale areas include a native capability catalog, real external agent adapters, voice calls, live shared editing/follow mode, mobile participation, sandboxed widget/workshop execution, and portable Forge Packs. These require separate verified vertical slices and runtime acceptance; they are not represented as complete by this release.
## Team Lobby 0.67.0 â€” task-linked room reviews

Review summaries can now optionally reference a room task. The host confirms the task exists, permits any authenticated room participant to post the review, and stores the task ID with the event. The roster task owner does not change. Task-linked agent and verification events retain their stricter owner-or-host rule. Protocol 14 distinguishes this expanded event contract. TypeScript check and 78/78 focused tests pass in both extension copies; local multi-client transport coverage verifies reviewers can link a task and missing task IDs are rejected. VSIX package SHA-256 is recorded in product.json.
## Team Lobby 0.68.0 â€” remaining-risk review evidence

Build review export now asks for an optional user-entered note describing remaining risks or open questions (up to 2,000 characters). The note is local to the exported Markdown bundle, is displayed as quoted text, and is labeled as unverified. Canceling the prompt cancels export. Three focused tests cover empty notes, Markdown line handling, control-character removal, and size limits. Both extension copies pass TypeScript checking and 81/81 focused tests. Provider-backed Build and IDE UI flow still require hands-on validation. The VSIX package hash is registered in product.json.
## Team Lobby 0.69.0 â€” stable task-agent identity across Build continuations

A task-linked Build creates one virtual agent participant ID and retains it in the local Build session. Continue, pause/resume and opt-in restart recovery reuse that ID, so teammates see one agent across turns. Older saved Build sessions without an ID receive one on their next continuation; malformed saved IDs are replaced. The ID is persisted before the follow-up provider request. No room protocol change was needed.

**Verification:** `apm.cmd run check` passes; `apm.cmd test` builds TypeScript and passes 84/84 tests, including three session identity cases. The 0.69.0 VSIX is mirrored into the Code OSS checkout and handoff. SHA-256: `725D6EEAA6EF1DDC1C12A8B0D9C1EF4CC6FB0EFF5F428DCC664731976F035441`. Provider-backed session recovery and multi-window IDE behavior remain unverified.

## Team Lobby 0.70.0 â€” native read-only capability registry

The existing workspace search and file-read tools now pass through registered Forge capability IDs (`workspace.search` and `workspace.readFile`). The registry validates plain-object inputs, rejects extra fields and unsafe paths/globs, binds each call to the requesting workspace, and normalizes validated inputs before execution. Existing per-call or current-request approval choices remain the authorization boundary. Provider results stay within the existing 12,000-character per-call and 30,000-character per-request limits. The Forge Room output channel records actor, workspace, resource/scope, approval decision, outcome and returned character count without recording search queries or result text. Existing sensitive-file confirmation still applies to file reads.

**Verification:** `apm.cmd run check` passes. `apm.cmd test` builds and passes 88/88 tests, including capability registration, invalid/extra input rejection, traversal rejection, workspace scope binding, output limits and audit redaction. VSIX SHA-256: `53C09218C38256E2124DC57EEFE6E21F9FFFB338ED9C915B38ACBB2400D01436`. Provider-backed execution and UI approval flow remain unverified.

## Team Lobby 0.71.0 â€” multiplayer artifact and invite lifecycle proof

Room diff and comment payloads now use a shared production validator. It accepts a bounded, relative-path Git patch or a comment with a valid line anchor, and rejects fields that could smuggle selected source, prompts, terminal output or arbitrary content. The local two-client WebSocket suite now verifies both peers receive attributed, timestamped diff/comment events, reconnect history includes those events, and an attempted comment carrying selected source is rejected. The suite also revokes an invite used by an active guest, confirms the guest is disconnected, checks the host-attributed audit event excludes the invite secret, and verifies the revoked token cannot reconnect.

**Verification:** `apm.cmd run check` passes; `apm.cmd test` builds and passes 91/91 tests. These are process-level transport and policy tests; two separate IDE windows and two-machine networking/TLS remain unverified. The 0.71.0 VSIX SHA-256 is `3F82E2B8610C1E9002B7F35E513D7251389FDBBEDD6175DF3FD72D4BF2AD98F0`.

## Team Lobby 0.72.0 â€” user controlled room history export

Added the command **Team Lobby: Export Room History**. It writes a JSON bundle with the current room events, tasks, and notes. Before choosing a destination, the user sees that the bundle can contain messages, task details, comments, and shared Git patches, and must choose whether direct messages are included. Direct messages are excluded when the user chooses the default-safe option. Invite secrets and TLS credentials are not included by the serializer. Export failures are reported in the IDE.

**Verification:** `apm.cmd run check` passes and `apm.cmd test` passes 92/92 tests in both the Code OSS extension and handoff mirror. The added unit test checks direct-message filtering, retention of public events, and that filtering does not mutate saved history. VSIX SHA-256: `6AD5F86F7AB3F9C2CF3F502EA276AE795252FE19ED4A20B3E4884A1CE16E3619`. This is user-requested export, not tamper-evident audit storage or a retention/deletion control.

## Team Lobby 0.73.0 â€” explicit editor location sharing

Added **Team Lobby: Share Current Editor Location**. The user confirms each share. The room event carries only a workspace-relative file path and one-based line number, not selection text or file contents. The host validates the path and line before persisting and broadcasting the event. A teammate can open the event to navigate to that line in their own workspace; missing files and out-of-range lines are handled locally. This is a one-time navigation handoff, not live cursor tracking or shared editing.

**Verification:** `apm.cmd run check` passes and `apm.cmd test` passes 94/94 tests in both the Code OSS extension and handoff mirror; `node --check media/room.js` passes. The new path validator rejects traversal, absolute paths, control characters, and invalid line numbers. VSIX SHA-256: `F05899D3F70388BE1592D52F0CCAE925590EDBA8180DAFEAF976E456AE456EDC`. Two-IDE UI acceptance remains open.

## Team Lobby 0.74.0 â€” opt-in live editor follow

Added **Team Lobby: Toggle Live Editor Location Sharing**, **Team Lobby: Follow Teammate**, and **Team Lobby: Stop Following Teammate**. Sharing requires a modal confirmation, streams only an in-workspace relative file path and one-based line number, and is limited to four updates per second. It sends no selection, buffer content, or cursor text and remains ephemeral rather than entering room history. Participants can stop sharing or stop following independently. Follow navigation resolves paths inside the receiver's own workspace and clamps stale line numbers. Reconnect history contains the current locations only for participants still actively sharing; disconnect, invite revocation, and explicit stop clear the host's live location state.

**Verification:** `apm.cmd run check` passes and `apm.cmd test` passes 97/97 tests in both the Code OSS extension and handoff mirror; `node --check media/room.js` passes. The new two-client transport test covers location delivery, invalid path rejection, stop-sharing removal, and exclusion from persisted events; helper tests cover the four-per-second throttle and strict payload reduction. VSIX SHA-256: `7AEA38B1D8A98D49ECF05128BCE102387ACE784B9A16F1C3B37A7CB8A2E6D74B`. Separate IDE follow behavior remains to be exercised. This provides spectate/follow at file-and-line level; shared editing and shared buffer synchronization remain unimplemented.

## Team Lobby 0.75.0 - read-only Git status for AI

Registered `git_status` as a typed workspace-scoped capability alongside search and named-file read. It uses the built-in Git extension API and reports branch name, staged/modified/untracked/conflict counts, and up to 40 non-sensitive paths relative to the selected workspace. It reads no diffs or file contents and runs no shell command. Existing per-call or request-scoped approval, capability audit, output budgeting, and content-free evidence apply.

## Team Lobby 0.91.0 â€” provider-approved per-file Git diffs

Ask, Plan and Build now expose a typed `git_diff` tool for the staged and unstaged tracked diff of one explicitly named file in the selected workspace. The tool is read-only and uses the built-in Git extension API. Each call uses the existing user approval flow; the prompt names the workspace-relative path and says the patch will be sent to the selected provider. Likely sensitive paths are rejected during capability normalization and checked again before reading. Untracked files, binary patches, rename/copy patches and unsaved editor buffers are not included. Patch output is capped at 12,000 characters (and remains subject to the existing per-request output budget). Local audit/evidence stores the actor, scope, selected path, decision, outcome and output size, never the patch text.

**Verification:** TypeScript check and extension build pass. Both mirrors pass 137/137 tests. Six new Git diff tests cover staged and unstaged retrieval, nested workspace/repository selection, sensitive-path and traversal rejection, no-change, binary and rename/copy results, truncation, approval summary and content-free evidence. Provider-backed invocation and IDE approval UI remain to be exercised.

## Team Lobby 0.92.0 â€” approved workspace diagnostics tool

Ask, Plan and Build now expose `diagnostics_list`, which reads up to 100 current VS Code diagnostics either across the selected workspace or for one named workspace-relative file. Each call uses the existing approval flow; the approval summary names the requested scope and says diagnostic messages will be sent to the selected provider. Results include severity, source, code, line/column and a sanitized, bounded message. Paths outside the selected workspace, likely sensitive paths and malformed paths are excluded. Output is capped at 12,000 characters. Local evidence retains only paths and output size.

**Verification:** Both mirrors pass TypeScript checks/build and 142/142 tests. Coverage checks capability authorization summaries, workspace/file selection, sensitive path and traversal rejection, formatting, malformed entries, output bounds, and evidence that excludes diagnostic text. The nine-file VSIX SHA-256 `CB4616885C3A99988D7C39ECEF5F4CB6503DD897888BD526FAD76648B42201EC` matches built-in metadata. Provider-backed and IDE approval UI acceptance remain open.

## Team Lobby 0.93.0 â€” approved workspace symbol search

Ask, Plan and Build expose `symbols_find`, which queries the IDE's registered workspace symbol providers for a required 1â€“160 character search string. Approval names the query and explains that registered symbol providers receive it; matching symbol names and locations are sent to the selected model. Results return at most 100 symbol names, kinds, containers and one-based workspace-relative file/line locations without reading source text. Results outside the selected workspace, sensitive paths and unsafe paths are excluded. Output is capped at 12,000 characters. Audit/evidence records scope, source paths and result size without storing the query, symbol names or code.

**Verification:** Both mirrors pass TypeScript checking/build and 147/147 tests. Tests cover capability validation and transfer disclosure, bounded/sanitized formatting, unsafe location rejection and content-free evidence. The nine-file VSIX SHA-256 `8AA3DDF975B4432D26E8668C4A024C701B09290C4EC9B0AA5A9C0F3EFA98FADF` matches built-in metadata. Provider-backed behavior and interactive approval flow remain to be exercised.

**Verification:** `apm.cmd run check` passes and `apm.cmd test` passes 100/100 tests in both the Code OSS extension and handoff mirror. The capability registry rejects arguments for Git status; formatter tests cover the 40-path cap, sensitive-path omission, unrelated repositories, and nested workspaces; evidence tests confirm branch and result text are not persisted. Separate IDE/provider acceptance remains open. This completes only the read-only Git status capability; external ACP adapters and write/command capabilities remain open.
VSIX SHA-256: `988E532B5455651CB2CEC19E417638C2EB222738D34A2F2548E83FE9F6B23089`.

## Team Lobby 0.76.0 - external ACP Ask and Plan

Added optional ACP v1 as a provider choice for Ask and Plan. A machine-scoped executable and argument list launch through direct stdio without a shell. Forge Room negotiates protocol version 1, creates a one-shot session with no MCP servers, streams agent text, supports cancellation, advertises no filesystem or terminal client methods, and rejects session permission requests. External Build is not enabled; native Build remains on the IDE Language Model API path with Forge Room's reviewed diff/apply flow.

Before transfer, a modal confirmation shows the exact executable/arguments, workspace, and selected-context summary. It states that the local process runs with the user's OS account and environment and may read or change files, execute commands, or access the network outside Forge Room's client permissions. ACP sessions are not resumable and token/cost estimation is unavailable.

**Verification:** fake ACP process tests cover v1 negotiation, one-shot session/prompt, text streaming, no advertised client tools/MCP servers, permission denial, cancellation, unsupported protocol rejection, and shell-free command validation. `apm.cmd run check` passes and `apm.cmd test` passes 104/104 tests in both Code OSS and handoff copies. Real-agent/manual UI acceptance and OS process isolation remain open.
VSIX SHA-256: `04BF33E7A850B8AA1D675164D714D0147841817C94F833A26EB35FF2777D2F49`.

## Team Lobby 0.77.0 - ACP agent sign-in

ACP v1 Ask/Plan now handles agents that advertise in-agent authentication methods. After the configured process starts, Forge Room offers the supported agent-managed sign-in choices by name and description; the selected method is sent using ACP `authenticate`. Canceling the chooser stops the request before a session is created. Terminal-auth methods remain unavailable because Forge Room does not provide an interactive terminal login flow. The existing consent warns that the external process has the user's OS privileges and environment; ACP method selection does not sandbox it.

Research: [ACP v1 initialization](https://agentclientprotocol.com/protocol/v1/initialization) defines advertised auth methods and client capabilities; [ACP v1 authentication](https://agentclientprotocol.com/protocol/v1/authentication) defines agent-managed `authenticate` and terminal-auth requirements. Checked 21 October 2026.

**Verification:** `apm.cmd run check` passes and `apm.cmd test` passes 106/106 tests in both Code OSS and handoff copies. Fake-agent tests cover the offered method filter, chosen method ID, and canceled login. Real-agent/manual IDE acceptance and OS process isolation remain open.
VSIX SHA-256: `74EF40D8570683FA83019C3CC7EB0DB00A5E59373EFB2BE090A5CDAEE6EB0815`.


## Team Lobby 0.78.0 - assigned Git worktree verification

Task-linked Build and Build resume now require an exact Git identity match: the active workspace root must match a registered or discovered linked worktree in the same repository, its actual branch must match the branch recorded for the local path, and both must match the room task's assigned branch. This rejects stale records that point to another branch, the main checkout, or a worktree in another repository. Filesystem path comparison is case-insensitive on Windows and case-sensitive on POSIX.

A new integration test creates and commits a temporary repository, adds a linked worktree in a path containing spaces, reads the real Git root/branch/git-dir/common-dir values, and verifies correct identity plus wrong-task-branch, wrong-checked-out-branch, and main-worktree rejection.

**Verification:** `apm.cmd run check` passes and `apm.cmd test` passes 108/108 tests in both Code OSS and handoff copies. `@vscode/vsce package --no-dependencies` succeeds; package SHA-256: `A02CB58F0A29AABC02D93BA0B51F38EBC408F3A85558D22BAE7219E9C471595E`. Full IDE Build selection and resume UI flows still need runtime acceptance.


## Team Lobby 0.79.0 - stale assigned worktree recovery

When a cached local worktree record is absent, stale, or points to a branch other than the task's assigned branch, Forge Room now checks Git's registered worktrees for that exact task branch. The candidate must still pass actual root, repository common-directory, checked-out branch, and linked-worktree identity checks before its device-local path mapping is refreshed. This lets another IDE window or a recovered workspace find an existing valid worktree rather than attempting to add a branch that Git already has checked out.

**Verification:** `apm.cmd run check` passes and `apm.cmd test` passes 108/108 tests in both Code OSS and handoff copies. The Git integration test uses a real temporary repository and linked worktree with a path containing spaces, then verifies matching and mismatched branches plus worktree discovery. `@vscode/vsce package --no-dependencies` succeeds; package SHA-256: `BF22296F3965C8CFFA528091A8A142917B5F9F56FE2F329818F82836BE14D13C`. Full IDE startup/recovery UX remains to validate.


## Team Lobby 0.80.0 - integrated task worktree recovery

Task worktree validation and rediscovery now run through one production resolver. It checks cached records against the actual Git root, repository common directory, checked-out branch and linked-worktree metadata. When a cached record is stale, it searches Git's registered worktrees for the task's exact assigned branch, validates that candidate through the same checks, and refreshes the device-local path mapping. A different repository with a coincidentally identical branch is rejected.

The Git integration test creates a temporary repository and linked worktree, then calls the production resolver with a stale path, a valid cached path, and a task branch that is not present. It verifies recovery, cache retention, and fail-closed missing-branch behavior.

**Verification:** `apm.cmd run check` passes and `apm.cmd test` passes 108/108 tests in both Code OSS and handoff copies. `@vscode/vsce package --no-dependencies` succeeds; package SHA-256: `C6165382FF41927C08A856B9CCCEEEDA142CBE9675C1552F19323AD29CC7AA56`. Full IDE Build selection/resume interaction remains to validate.


## Team Lobby 0.81.0 - safe local task worktree cleanup

Added the **Team Lobby: Remove Clean Local Task Worktree** command to close the local task worktree lifecycle. It only offers worktrees verified against the task branch and current repository. It refuses to remove the workspace currently open in the IDE, checks for tracked and untracked changes, and requires modal confirmation showing the exact folder and deletion effect. After confirmation it revalidates the same worktree and calls `git worktree remove` without `--force`; Git can reject a change that occurs after the inspection. The shared task assignment and local Git branch are preserved. No file paths, contents, or room events are sent by cleanup.

A real Git integration test proves dirty files prevent removal and remain present, while a clean reviewed worktree is removed and its branch remains. **Verification:** `apm.cmd run check` passes and `apm.cmd test` passes 109/109 tests in both Code OSS and handoff copies. `@vscode/vsce package --no-dependencies` succeeds; package SHA-256: `646E955E42FAFAF3A8AC85F2178AC214A4C1BC4E2CBDCE7AA99C728002073D5B`. IDE command-palette interaction remains to validate.

## Team Lobby 0.82.0 - opt-in live shared documents

The room host can share its active document from the first local workspace folder. The path is announced to room participants; the complete text buffer is sent only after the participant requests to join and the host explicitly approves. The consent dialogs state whether transport uses TLS or unencrypted LAN WebSocket. A Yjs CRDT merges concurrent text changes into each participant's unsaved editor buffer; Forge Room does not save files automatically. Shared state exists only in memory for the room session. A reconnecting participant must be approved again, then local offline edits merge into the current room state. Stopping a session ends subscriptions and leaves local buffers open.

Documents are limited to 512 KB. Update payloads and per-peer throughput are bounded, room messages require an active approved subscription, and paths must resolve safely within the selected local workspace folder. This delivers the first shared-editing slice; two-IDE editing and reconnect interaction, multi-root workspaces, cursor/selection awareness, and richer concurrent-edit review still need validation or follow-up. See [AI_CORE_FEATURE_RESEARCH.md](../research/AI_CORE_FEATURE_RESEARCH.md) for the CRDT/API research sources.

**0.82.0 package verification:** Code OSS and handoff copies pass TypeScript checks and 115/115 tests. The VSIX contains the bundled extension entry point, `package.json`, README, license, and media with no `node_modules`; the bundle loads with its runtime dependencies resolved under a mocked VS Code API. Package SHA-256: `4902A209CCE3487A4DBC9932D36EE1188207C77378D4152A31CACE5EE65B4F51`. The built-in Code OSS manifest points to the same version and hash. Two-IDE live editing/reconnect acceptance remains open.

## Team Lobby 0.83.0 - multi-root live shared documents

Live shared editing supports documents in any uniquely named local workspace folder. Multi-root room paths include the folder name, and joining clients resolve that name to exactly one local root before opening the file. Duplicate folder names are rejected as ambiguous. Room protocol version 16 marks the path semantics. TypeScript checks and all 116 tests pass in both extension copies. The nine-file VSIX hash matches the Code OSS built-in extension manifest. Separate two-IDE editing and reconnect acceptance remain open.

## AI core implementation update - bounded plans and observable context

Saved Plan artifacts include the original request, proposed plan, model name, creation time, and optional linked room task in a consistent Markdown structure. Transfer approval shows selected source character counts and a provider estimate for the serialized prompt; tool results remain individually approved, bounded, and checked against the model limit on each turn. Live tool progress reports outcome, bounded source paths, and output size without query or result contents. Focused tests cover artifact structure and invalid or oversized input. Provider-backed runtime validation remains open.

## Security implementation status - self-hosted room threat model

The Phase 6 threat-model deliverable is documented in [THREAT_MODEL.md](../architecture/THREAT_MODEL.md). It covers workspace and unsaved-buffer data, provider and ACP trust boundaries, invite bearer credentials, TLS, room history, path containment, denial-of-service limits, and release gates for a public managed service. Current self-hosted WSS verifies TLS with the system trust store and authenticates participants through expiring, revocable bearer invites. This is not a managed multi-tenant service; separate-machine deployment, durable managed storage, account administration, and independent security review remain open.

## Implementation status update â€” account key rotation

Team Lobby 0.86.0 adds a confirmed **Rotate Room Service Account Key** command. Rotation invalidates the prior credential, persists only the replacement hash, records account-scoped audit evidence, and disconnects active service-hosted rooms. The new credential is stored in VS Code SecretStorage. Self-hosted key rotation is now implemented; service account recovery and managed operations remain outside this slice.

## Implementation status update â€” operator account recovery

The room-service admin API now supports one-time replacement of a lost account key for active accounts. The former key immediately fails, hosted relay sessions are disconnected, ownership data remains intact, and the audit record identifies the operator action. Admin credentials remain a local service bootstrap secret; this is not a hosted administration console or self-service recovery.

Operator recovery of a disabled account atomically replaces its key and reactivates it; existing invites are permanently revoked during that restore. User self-service recovery remains absent.

## Implementation status update â€” room-service audit history

The room-service audit is available from Team Lobby through **View Room Service Audit History**. The account-wide HTTPS API is cursor paginated, includes only that account's account/room/invite lifecycle events, and returns bounded pages so older history does not exceed the extension response limit. The viewer shows metadata only; it does not display shared room content or credentials.

## Implementation status update â€” safe checkpoint rollback

Build checkpoint rollback now repeats its target and content validation after the user confirms and immediately before the workspace edit. It cancels the whole restore if a file changed, disappeared, escaped the workspace, or a Build-created target became a symlink. No checkpoint file is modified on that cancellation path.

## Team Lobby 0.94.0 â€” IDE test task discovery

### User-visible behavior

Ask, Plan and Build may request the read-only `tests_discover` tool. Forge Room asks for the existing workspace-tool approval, describing that task names and provider labels are sent to the selected language model and that no task will be run. The tool lists at most 100 tasks belonging to the IDE Test task group, scoped to the selected workspace. It returns the task name and provider label only. Task commands, arguments, environment variables, terminal output and test results are outside this capability.

### Safety and evidence

The model tool accepts no input fields. Per-request and aggregate output caps continue to apply; cancellation returns no partial task list. Local evidence stores the capability, output size and outcome, not task names. Test task execution requires a separate feature with exact-task confirmation, cancellation, bounded duration and honest process-result evidence before implementation.

### Acceptance status

Formatter, registry, approval summary, redacted evidence, TypeScript and build tests are implemented. Automated extension tests pass 151/151. Runtime behavior with real IDE task providers and provider-backed model calls remains unverified.

## Team Lobby 0.95.0 â€” confirmed IDE test execution

Build can call `tests_run` for one registered IDE Test-group task in the selected workspace. The request has to name a task surfaced by `tests_discover`; Forge re-fetches current tasks and rejects stale names, then asks a separate modal confirmation naming the task and provider. Request-wide approval for read-only inspection never authorizes execution. Project task code runs with the user's OS privileges and may modify files or use the network. Output remains in the local task terminal; the model receives only the actual process exit code. Tasks without a process exit event report an unverified result. Cancellation and the fixed 10-minute timeout request task termination. Local evidence keeps only capability, outcome and result size.

Task execution is exposed only in Build, not Ask or Plan. Duplicate exact task names/providers require a user selection. Automated tests cover strict input registration, exact matching, mandatory per-call approval despite request-wide read-only approval, exit-status evidence and cancellation/timeout classification.

## Team Lobby 0.96.0 â€” confirm the selected test task

When multiple test tasks share the requested name and provider, the IDE now asks the user to choose a specific matching task before showing the approval modal. The approval summary identifies that choice (including its match number). The same `Task` object selected by the user is passed to `tasks.executeTask`; the selection prompt no longer appears after approval. This is a correction to the 0.95.0 execution flow, with exact selection preceding consent.

### Active debugger stack inspection

Ask, Plan, and Build may request `debugger_frames` as a read-only IDE capability. Forge uses the currently active, paused thread in the active debug session and requires that the session belongs to the selected workspace. It returns at most 20 sanitized frame names and source locations after filtering for existing, non-sensitive files within that workspace. It does not read variables or invoke debugger control actions. The ordinary capability approval explains that frame metadata is sent to the selected provider. Debug adapter support can vary; unsupported requests and resumed threads produce an explicit unavailable result. Forge stops waiting after 10 seconds and on request cancellation; the VS Code API does not cancel the underlying adapter request.

### Approved terminal result inspection

`terminal_last_output` returns the newest terminal capture for the selected project. A command must first be captured through the existing local terminal capture flow. Forge freezes the selected capture before consent, discloses the redacted command, status, character count and potential secret exposure, and asks for a separate per-call confirmation in every mode. Request-wide read-only approval never authorizes terminal output. The output is sanitized and bounded; local evidence stores no command or output text.


### Controlled AI workspace edits

Team Lobby Build exposes `apply_workspace_edit` only when Build tools are enabled. The tool replaces one exact text occurrence in an existing, non-sensitive file up to 256 KB; old and replacement excerpts are each capped at 12,000 characters. Forge checks for a unique match, opens the full document diff, and requires separate per-call approval even when read-only tools were approved. It rechecks the original open-document version and contents after the preview; stale edits are rejected. Accepted changes remain unsaved, use the normal IDE Undo stack, and save an ordinary single-file Build checkpoint before application. Rollback restores only while the file still matches the checkpoint; later changes cause a conflict refusal. Room task activity records the change; paths are shared only if file-activity sharing is enabled. If a request stops after an edit, Forge says the edit remains applied, and an interrupted session with prior edits must be restarted from current file contents. Local evidence retains the path, result size, and outcome but never either text excerpt.
