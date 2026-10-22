# IDE feature research: what stands out and what Forge Room should learn

**Research checked:** 20 October 2026  
**Scope:** Feature-level review of VS Code/Code OSS, Cursor, Windsurf, Zed, JetBrains AI/Junie, GitHub Copilot in VS Code, and Google Antigravity.  
**Method:** Public official product documentation and vendor engineering/product material. This is desk research, not a hands-on performance benchmark or independent quality evaluation. Vendor descriptions are labeled as such; recommendations for Forge Room are analysis.

## Executive view

The separate [AI core feature research](AI_CORE_FEATURE_RESEARCH.md) deepens the analysis of agent context, planning, approvals and sandboxing, session state, review and rollback, and project memory.

There is no single feature list that makes one IDE “the best.” The strongest products are good at different jobs:

- **VS Code / Code OSS** is the strongest general workbench and extension platform. Its standout is how many real development workflows coexist in one configurable window.
- **Cursor** makes repository-aware agent work approachable. Its most useful design choices are explicit planning, direct code context references, checkpoints, isolated Git worktrees, and review of the resulting diff.
- **Windsurf** packages an agent-centric flow with persistent project guidance, lifecycle automation, and isolated parallel model runs. Its lesson is to turn repeated project knowledge and setup chores into reusable, inspectable configuration.
- **Zed** treats collaboration as a persistent work environment: channels, shared projects, presence, live cursors/edits, and room notes support both pairing and asynchronous handoff.
- **JetBrains AI/Junie** benefits from mature language-specific IDE context and makes autonomy modes, debugging, and rollback part of the agent experience.
- **GitHub Copilot in VS Code** is a useful reference for composable tools, explicit permissions, sessions, worktree isolation, repository instructions, and agent delegation.
- **Google Antigravity** makes the agent's work legible through task artifacts: plans, diffs, browser recordings, and walkthroughs. The user can comment on these deliverables rather than reconstructing progress from raw tool calls.

**Recommended Forge Room synthesis:** protect the traditional IDE loop; make workspace personalization first-class; make agents produce reviewable, reversible work; and make the project room the connective tissue between people, tasks, agent sessions, changed files, and verification evidence.

## Research boundaries

Product features change quickly, and marketing pages describe intended behavior. This document does not claim that a listed feature is bug-free, faster, safer, or better than a competitor's implementation. We did not measure latency, completion quality, reliability, pricing, or user preference. Security and privacy descriptions are vendor-provided and should be checked against current terms and technical controls before Forge Room makes product commitments.

The term **IDE** here includes desktop code editors with substantial IDE capabilities and agent-first development environments. We review product features, not every language-specific feature of every JetBrains IDE.

---

## 1. VS Code / Code OSS: the dependable workbench

### Standout: one adaptable workbench for the full development loop

VS Code's product center is the workbench, not a single framework or language. The editor, file/search views, source control, debugger, testing, terminal, and extension surfaces live in one window. A workspace can include one or more folders and preserve project settings, layout, tasks, debug configuration, and open work. Language services, debuggers, and specialized workflows are extended through the extension ecosystem.

**Why this matters:** a developer can move from editing to understanding a symbol, running a test, inspecting a failure, staging a diff, and launching a debugger without re-creating the project context in another app. Code OSS gives Forge Room this mature base and an extension API rather than requiring us to build editor fundamentals ourselves.

**Tradeoffs:** the breadth of extensions creates variation in quality, compatibility, and UI consistency. A fork must track upstream behavior and marketplace constraints. A crowded workbench can feel generic if the product does not establish its own information hierarchy.

**Forge Room decision:** preserve the workbench's core interactions, extension compatibility, keybindings, terminal, testing, Git, and workspace model. Put signature collaboration in a native-feeling view and dedicated agent/task surfaces; do not replace basic IDE affordances with a custom dashboard.

### Standout: profiles as portable work modes

Profiles collect settings, extensions, keyboard shortcuts, snippets, tasks, MCP servers, and UI layout. They can be associated with a folder/workspace and exported or synced. This is more than a color picker: it lets a developer keep a focused setup for a language, project, or demo.

**Why this matters:** customization gets more useful when it follows the work. A theme can become part of a “game jam,” “deep focus,” or “client project” setup without making global preferences chaotic.

**Tradeoffs:** profile contents can include extensions and settings, which have behavior and trust implications. Importing a profile should not silently install code or weaken protections.

**Forge Room decision:** create shareable Forge Room project presets with explicit contents. Separate visual theme, editor colors, icon theme, font choices, layout, extensions, keybindings, and trust-related settings. Preview the manifest and require a separate, clear action for extensions or permissions.

### Standout: a mature theme surface

VS Code themes can cover workbench UI colors, syntax token colors, semantic tokens, file icons, and product icons. Theme previews can update live. Workspace-specific color customization and automatic light/dark/high-contrast selection are supported by the platform.

**Why this matters:** the platform already has the primitives for a strong visual identity while retaining editor functionality. Forge Room can curate, package, and coordinate these primitives rather than inventing a parallel styling engine.

**Tradeoffs:** a color theme does not restyle every webview contributed by extensions, and theme packs do not automatically provide art direction, motion, profile bundles, or a coherent welcome/room experience. Poor contrast can make syntax and diagnostics unusable.

**Forge Room decision:** theme packs should coordinate workbench, editor, icon, typography, and optional atmosphere layers. Validate contrast for text, syntax categories, selections, diagnostics, and diff states. Webviews should consume theme variables and offer reduced-motion support.

### Standout: agent tools paired with permissions and sandboxing

VS Code agents can use built-in tools, extension tools, and MCP tools to search/read/write files, run commands, and connect to services. The current permission model distinguishes approval behavior from terminal/network/file-system boundaries. Tool/URL/terminal approvals and sandboxing can be configured. Worktree isolation is another separate mechanism: it separates code changes but does not by itself restrict command execution or access outside the worktree.

**Why this matters:** users need more than an “agent on/off” switch. They need to know what the agent can do, where its changes land, and which actions are subject to approval.

**Tradeoffs:** multiple permission layers are powerful but can be hard to understand. Isolation can be mistaken for a security boundary when it is only a change-management boundary.

**Forge Room decision:** show two related but distinct controls: **where changes go** (current folder or isolated worktree) and **what actions are allowed** (read, edit, terminal, network, sensitive data). Summarize them in plain language per session and surface the active mode in the room.

### Standout: sessions and an Agents window

An agent session groups a workspace, changes, and chat history around a task. The dedicated Agents window supports monitoring sessions across workspaces and reviewing files, changes, terminal/tasks, and browser state. A session can work in the active folder or a Git worktree. Worktree mode starts from committed state and has practical limitations around uncommitted/ignored files; it is not equivalent to a full sandbox.

**Why this matters:** agent work becomes a managed unit that can be resumed and reviewed instead of a transient conversation attached to one editor tab.

**Tradeoffs:** multiple chats sharing one workspace can mix changes. Separate worktrees require integration and setup. Some harnesses or workspace shapes have different capabilities.

**Forge Room decision:** represent an agent run as a first-class **work session** linked to a task, participant identity, worktree, and evidence. Keep session history, changes, and results connected when moving between the editor and room. Explain which worktree files are absent.

---

## 2. Cursor: codebase-aware agent workflow

### Standout: repository context references and semantic search

Cursor offers explicit references for files, folders, snippets/symbols, documentation, Git history/changes, recent changes, and other context sources. Its docs describe local codebase indexing for semantic search. The user can guide the agent toward relevant code rather than pasting large files into a prompt.

**Why this matters:** most repository tasks depend on finding the right implementation, conventions, callers, and tests. Strong context selection reduces irrelevant edits and repeated explanation.

**Tradeoffs:** indexing and AI context have privacy implications. The actual request can include selected code even when an index stores only derived metadata. Ignore files and privacy controls reduce risk but do not eliminate the need to understand data flow. Indexes can also be stale or miss relevant code.

**Forge Room decision:** build a context picker that can add files, symbols, diffs, tasks, room decisions, diagnostics, and documentation. Show a compact “included context” list before sending. Provide workspace ignore/exclusion rules and a request-level preview of what leaves the machine.

### Standout: Plan Mode before code changes

Cursor Plan Mode researches the codebase, asks clarifying questions, produces a detailed plan, and lets the user edit that plan before starting implementation. Plans can be saved as Markdown in a workspace. The docs recommend it for complex or ambiguous work, while quick tasks can skip it.

**Why this matters:** a plan makes hidden assumptions and scope visible before code changes become expensive to undo. A saved plan can also be shared with teammates and reused as project history.

**Tradeoffs:** planning adds a step for small tasks and does not guarantee the implementation follows the plan. A plan can become stale if the code changes while execution is delayed.

**Forge Room decision:** provide Ask / Plan / Build modes, but let the user move between them without losing context. Plans should identify files, dependencies, risks, and acceptance checks; after execution, show planned versus completed work and explain deviations.

### Standout: checkpoints as a fast undo layer

Cursor Agent creates local checkpoints around significant agent changes. A user can preview and restore a checkpoint. Checkpoints are separate from Git and track Agent changes, not arbitrary manual edits.

**Why this matters:** people are more willing to explore when a bad agent turn can be rolled back quickly.

**Tradeoffs:** checkpoints are not a substitute for Git, can omit manual edits, and may create a false sense of complete recovery if users do not understand their scope.

**Forge Room decision:** keep automatic per-turn restore points, label exactly what is captured, and maintain Git as the durable history. Before restoring, show whether manual or teammate edits overlap.

### Standout: worktree isolation and parallel candidates

Cursor worktrees give each agent a separate Git checkout, keeping its file changes apart from the user's active checkout and other tasks. Setup commands can install dependencies or prepare environment state. Its best-of-N approach can run the same prompt across multiple models in separate worktrees; the user selects a candidate and explicitly applies/integrates it.

**Why this matters:** parallel agents are only useful if they do not trample one another's files. Candidate comparison can expose different approaches before choosing one.

**Tradeoffs:** worktrees consume storage and setup time; untracked/ignored files and secrets may need careful handling. Parallel model runs multiply cost and can tempt teams to select by surface polish rather than correctness. Worktree isolation does not necessarily sandbox network or OS access.

**Forge Room decision:** use one worktree per independent agent task where possible; expose setup requirements and missing ignored files. Add multi-candidate runs only after ordinary single-agent verification works. Show cost/compute implications and compare candidates against the same acceptance checks.

### Standout: background agents and long-lived Projects

Cursor supports remote background agents that can continue asynchronously in an isolated remote environment, with status, follow-up, and takeover. Its Projects concept describes a coordinator that maintains context, delegates work, and can run recurring tasks.

**Why this matters:** users can hand off long work and do something else; multiple tasks can keep moving without a developer supervising every tool call.

**Tradeoffs:** remote execution changes the trust boundary. Source is cloned to a hosted environment, commands can run autonomously, network access may be available, and credentials/repository permissions become important. Long-lived coordinator agents can obscure provenance if they do not retain task-level artifacts.

**Forge Room decision:** make remote/async agents a later phase. Require a clear execution location, repository grant, network policy, secret policy, retention window, and task audit trail. Every delegated child task should return changes and evidence to its parent task, not disappear into an opaque coordinator.

### Standout: review as a normal agent step

Cursor Agent Review can inspect local changes at quick or deep review depth, and can run manually or automatically at configured points. Bugbot uses repository review rules and can learn team-specific rules.

**Why this matters:** code generation and code review are different tasks. A second review pass can catch omissions, risky assumptions, and inconsistencies before a change is handed off.

**Tradeoffs:** automated reviews can miss defects or report false positives. Automatically running a review after every edit may add latency and cost; an “AI approved” label could mislead.

**Forge Room decision:** attach review findings as evidence, cite affected files/lines, and label review as assistance rather than approval. The owner or reviewer makes the merge decision.

---

## 3. Windsurf: persistent guidance and workflow automation

### Standout: Cascade with reusable rules and memories

Windsurf exposes an agent workflow and persistent workspace guidance. Its documented rule/memory approach separates team-authored rules from remembered individual context. The product documentation has changed over time, and current pages are not uniformly indexed; validate exact behavior in the current product before implementation.

**Why this matters:** repeated project conventions, architecture decisions, and common workflows should not need to be retyped in every agent prompt.

**Tradeoffs:** memory can be stale, personal, or wrong. If it is invisible, users may not know why an agent made a choice. Shared and personal context need different ownership and privacy rules.

**Forge Room decision:** implement explicit **Project Knowledge** before hidden auto-memory. Store reviewed decisions, architecture notes, conventions, and workflows with author, timestamp, source, and scope. Let users edit, expire, or delete entries; offer suggested memories only as drafts.

### Standout: lifecycle hooks

Windsurf documents hooks that run before or after agent actions. Hooks can record activity, enforce rules, validate changes, format files, or integrate with other systems. Workspace hooks can be versioned with the project; system/admin hooks can enforce broader policy. A pre-hook can block an action.

**Why this matters:** teams can automate guardrails and repetitive setup close to the point where work happens. A post-edit hook can run a narrow formatter; a pre-command hook can stop a prohibited action.

**Tradeoffs:** hooks are executable code and can fail, be malicious, or slow every action. Team-shared project hooks should not be silently trusted just because the repository opened.

**Forge Room decision:** support explicit hook discovery and a permission prompt that shows source, command, inputs, and whether it can block. Separate personal, workspace, and administrator policy. Log hook results in session evidence.

### Standout: Arena for model comparison

Windsurf Arena can run the same request against multiple selected models in separate sessions/worktrees and then let the user choose an approach. Documentation notes extra credit cost and Git/worktree prerequisites.

**Why this matters:** model comparison is useful for difficult tasks where the best solution is uncertain, and isolation prevents concurrent candidates from colliding.

**Tradeoffs:** it consumes extra time and model usage, results may differ in test conditions, and choosing a “winner” needs more than stylistic preference.

**Forge Room decision:** defer until Forge Room can isolate and test one agent task correctly. Compare outputs by plan quality, diff, acceptance checks, and resource cost, not anonymous preference alone.

---

## 4. Zed: collaboration as a persistent project space

### Standout: Channels for synchronous and asynchronous collaboration

Zed's Collaboration Panel separates persistent project **Channels** from contacts/private calls. A Channel maps to a work stream, holds shared projects and persistent notes, and exposes who is present. Its collaboration model supports simultaneous edits and cursors. Notes help a team hand work over when no one is pairing live.

**Why this matters:** chat alone is ephemeral and disconnected from code. A persistent room with a project, people, and durable notes makes it easier to rejoin context and see who is available.

**Tradeoffs:** shared-project collaboration can expose local project files to collaborators. Live editing requires clear ownership/conflict semantics and reliable network behavior. Channels need lifecycle and organization choices or become another list of stale rooms.

**Forge Room decision:** model a room as a durable project space with presence, shared tasks, activity, decisions, and handoff notes. Separate joining a room from granting access to file contents or live editing. Start with activity metadata and explicit file sharing; do not infer broad file access from presence.

### Standout: multiple collaboration modes

Zed combines live project collaboration with contacts and private calls, offering both active pairing and lightweight social presence. It also exposes device selection/settings for voice chat.

**Why this matters:** collaboration spans more than a single mode. Sometimes developers need a synchronous pair session; other times they only need to see availability or leave notes.

**Tradeoffs:** realtime editing, voice, chat, and async task tracking are distinct products with different reliability and privacy requirements. Adding all at once expands scope considerably.

**Forge Room decision:** ship in increments: presence and async activity first; room messages and direct messages next; then file/diff sharing; true synchronized editing and voice are separate future decisions.

### Standout: flexible model access and external agents

Zed documents several model access paths: hosted, API key, existing subscriptions, gateways, and local/self-hosted models. It separates Zed-owned AI features from external agents and terminal threads, each of which may own its own auth, tools, and data handling.

**Why this matters:** provider choice and local model support reduce lock-in and fit different privacy/cost needs.

**Tradeoffs:** provider flexibility multiplies configuration and support combinations. Data policies differ across pathways, and users can confuse an external agent's handling with Zed's.

**Forge Room decision:** define a provider-neutral agent/task protocol and display the active execution/provider boundary per agent. Start with a small adapter interface; do not claim universal model support before testing each adapter.

---

## 5. JetBrains IDEs and Junie: project intelligence and controllable autonomy

### Standout: language-specific project understanding

JetBrains IDEs are organized around language/framework-specific inspections, navigation, refactoring, build integration, and debugging. Their AI layer can use current editing/session context, and the available project context varies by IDE and configuration.

**Why this matters:** an IDE is more than text plus an LLM. Reliable symbol navigation, type information, inspections, refactoring, and debugger state make agent work more grounded and easier to verify.

**Tradeoffs:** deep integrations require language-specific engineering and cannot be duplicated uniformly for every ecosystem. Forge Room's Code OSS foundation should preserve the extension/LSP ecosystem rather than attempt to rebuild each JetBrains subsystem.

**Forge Room decision:** use language server, diagnostics, test, Git, and debugger surfaces as context providers. Make this context opt-in/visible to the user and attach relevant diagnostics or test results to agent tasks.

### Standout: Junie modes and debugger-aware work

Junie documents Default mode (plan and implement), Plan mode (prepare and wait), and Goal mode (work autonomously toward a configured objective), alongside a Debug mode in supported IDE contexts that can inspect a paused runtime via breakpoints, stack, variables, and expressions. It can select models/reasoning levels and offers rollback of introduced changes.

**Why this matters:** users need different levels of autonomy for exploration, implementation, and long-running work. Runtime state can provide evidence that static code reading cannot.

**Tradeoffs:** autonomy mode names can imply more guarantee than the harness actually provides. Debugger actions can change runtime state or expose sensitive values. Goal-style work needs boundaries and stopping conditions.

**Forge Room decision:** use plain mode names and explain permissions. A later Debug Assist mode can inspect a user-authorized debug session but should be read-only by default. Every autonomous goal needs a scope, success criteria, allowed tools, and stop/redirect controls.

### Standout: appearance and editor schemes are distinct

JetBrains separates the UI theme (windows, controls, dialogs) from editor color schemes (syntax, debugger, console, and code-related text) and fonts. Schemes can be duplicated, adjusted, and shared.

**Why this matters:** a theme that colors the window but leaves code and diagnostics incoherent feels unfinished. Separate theme layers let users personalize without sacrificing editor legibility.

**Tradeoffs:** independent theme layers may clash unless previewed as a coordinated set. Font choices might not exist on another platform.

**Forge Room decision:** distinguish workbench skin, editor scheme, font, icons, and optional artwork in both theme editor and bundle manifest. Offer portability fallbacks and show a preview before sharing.

---

## 6. GitHub Copilot in VS Code: tools, instructions, and permissions

### Standout: composable agent tools

VS Code supports built-in tools, MCP tools, and extension-contributed tools. Agents can search code, read/write files, run commands, navigate, and call connected services. The tool system can be scoped to a request or workflow.

**Why this matters:** tool extensibility lets an IDE connect to project systems without baking every integration into core. Tool scoping also keeps the agent focused and reduces unnecessary capabilities.

**Tradeoffs:** MCP and extension tools widen the trust boundary; tool output can itself contain untrusted text or prompt injection. Tool catalogs can overwhelm both user and agent.

**Forge Room decision:** implement a small, typed tool catalog with clear capability descriptions. Scope enabled tools per task and show which tool produced each room event or evidence artifact.

### Standout: approval levels and sandbox boundaries

VS Code separates permission levels, tool approval, URL approval, terminal approval, and sandboxing. Autopilot can auto-approve tools and retry actions; the documentation warns that this increases risk. A worktree isolates edits but is not a substitute for a sandbox.

**Why this matters:** a useful agent may need to run commands, but approval should be based on action risk and execution boundaries rather than a single vague trust toggle.

**Tradeoffs:** detailed controls can confuse users, and “Allow all” or autopilot can normalize dangerous actions. Sandboxing behavior differs by harness and platform.

**Forge Room decision:** create safe defaults and readable presets: **Ask first**, **Workspace only**, and **Autonomous in isolated worktree**. Show network and secret access separately. Never imply that a worktree prevents access to the rest of the machine.

### Standout: portable project instructions and custom agents

Copilot can read repository-wide instructions, path-specific instructions, `AGENTS.md`, custom agents, and MCP context. This lets teams encode their architecture, build, test, and review conventions in version control.

**Why this matters:** team conventions should survive a new chat, new employee, or new agent. Path-scoped instructions avoid forcing every rule into every prompt.

**Tradeoffs:** multiple instruction systems can conflict; poorly scoped rules add irrelevant context. Repository instructions are untrusted input when a workspace is not trusted.

**Forge Room decision:** support portable `AGENTS.md` and a small Forge Room knowledge format. Show which instructions were loaded and where they came from. Resolve conflicts visibly and let the user inspect active rules per task.

### Standout: agent sessions and worktree management

VS Code sessions tie conversations, workspace, changes, and tool state together. The Agents window can monitor work across sessions; worktrees separate parallel work and produce reviewable branches. Documentation explicitly notes that worktrees begin from committed state, may omit ignored/untracked files, and are not themselves a sandbox.

**Why this matters:** session continuity and isolation enable parallel work while keeping the user's active workspace stable.

**Tradeoffs:** session complexity can separate users from the code; worktree setup and result integration add friction.

**Forge Room decision:** link sessions to room tasks and provide simple lifecycle controls: start, pause, resume, stop, inspect diff, verify, hand off, and integrate. Clearly report missing setup files.

---

## 7. Google Antigravity: artifacts as the agent communication layer

### Standout: a manager for asynchronous parallel agents

Antigravity describes an editor view for one workspace and a manager experience for launching and monitoring asynchronous agents across workspaces. Agents can work across editor, terminal, and browser, including parallel subagents.

**Why this matters:** when work runs longer than an interactive chat, the developer needs an overview of tasks, statuses, workspaces, and outputs. A manager makes delegation a visible workflow instead of a collection of hidden background processes.

**Tradeoffs:** an agent manager can compete with the editor for attention and introduce a second product surface. Cross-workspace context and task ownership can become unclear.

**Forge Room decision:** the team room can become our lightweight manager: people and agents appear together, grouped by task/project. A dedicated orchestration window should wait until actual users have enough concurrent sessions to justify it.

### Standout: artifacts and comments on outcomes

Antigravity uses task-level artifacts such as plans, task lists, code diffs, diagrams, images, browser recordings, and walkthroughs. Users can comment on artifacts to guide the agent. The artifacts summarize what the agent intends, did, and verified.

**Why this matters:** raw tool logs explain mechanism but not whether the result meets the goal. Reviewable deliverables support higher-level feedback: “this flow is wrong” or “the plan missed the migration,” without asking a user to parse every command.

**Tradeoffs:** artifacts can be incomplete, overly polished, or disconnected from the actual code state. A browser recording is evidence of one run, not proof across all environments.

**Forge Room decision:** define task artifacts as first-class room objects: plan, changed-file list, diff, test/build result, screenshot/browser run when relevant, review findings, and handoff summary. Each artifact must link to its source/run and timestamp; label failures and unverified claims.

### Standout: browser verification

Antigravity's browser agent can interact with browser surfaces for research and UI testing. Its IDE docs describe browser usage alongside editor and terminal tools, and browser recordings can be shared as artifacts.

**Why this matters:** an agent can check the interface it built instead of relying only on compilation. It can notice layout issues and demonstrate a user flow.

**Tradeoffs:** browser state may contain authenticated sessions, sensitive data, or actions with external side effects. Screenshots are not comprehensive accessibility, performance, or cross-browser tests.

**Forge Room decision:** browser automation belongs after a reliable local build/test loop. Require an explicit target URL and browser permission; keep authenticated state local and never attach it to room history by default. Present screenshots/recordings as supporting evidence, not a correctness guarantee.

---

## 8. Cross-product comparison by workflow

| Workflow | Strong reference patterns | Forge Room direction |
| --- | --- | --- |
| **Understand a project** | VS Code language ecosystem; Cursor references/indexing; JetBrains inspections; explicit context selectors. | Search symbols and files; show context sources; include repo instructions and reviewed room decisions. |
| **Plan large work** | Cursor editable, saved plan; VS Code plan workflows; Junie Plan; Antigravity task artifacts. | A reviewable plan with scope, files, dependencies, risks, and acceptance checks, linked to a room task. |
| **Run agent work** | Cursor tool loop and background sessions; Copilot tools and permission controls; Antigravity editor/terminal/browser. | Provider-neutral sessions with visible tools, state, approvals, and execution location. |
| **Run tasks in parallel** | Cursor/VS Code/Windsurf isolated Git worktrees; Antigravity manager and subagents. | One task per isolated worktree when practical; central status in room; explicit merge/integration step. |
| **Recover from a bad turn** | Cursor checkpoints; Junie rollback; Git branches/worktrees. | Automatic task checkpoints plus Git-native durable history; show overlap before restore. |
| **Verify work** | Cursor review; JetBrains debugger context; Antigravity browser artifacts; VS Code tests/debugger. | Attach real command/test/debug/browser evidence, including failures and limitations, to the task. |
| **Collaborate** | Zed Channels, presence, shared projects, notes, live cursors. | Persistent project room with human/agent presence, async updates, tasks, and opt-in levels for file sharing. |
| **Personalize** | VS Code themes/profiles; JetBrains separate UI/editor schemes; Zed settings and themes. | Curated, layered theme packs and workspace presets with readable contrast, preview, share, reset. |
| **Project memory** | Cursor project rules; Windsurf rules/memory; Copilot repo instructions; Antigravity knowledge. | Reviewable, attributable project knowledge; suggestions remain drafts until accepted. |

## 9. Forge Room feature decisions

### Adopt early

1. **Reliable Code OSS foundation.** Preserve the editor loop and extension ecosystem.
2. **Ask / Plan / Build flow.** Keep read-only exploration separate from implementation; save and review larger plans.
3. **Context visibility.** Show the files, instructions, decisions, diagnostics, and tools used for a task.
4. **Task sessions.** Tie each agent run to one task, owner, worktree, changed files, and status.
5. **Diff review and rollback.** Provide a task checkpoint and Git-aware change review.
6. **Room presence and async handoff.** Durable room notes, tasks, timeline, and who is available.
7. **Theme foundation.** Curated layered themes, preview/apply/restore, profiles, accessibility and reduced motion.
8. **Evidence artifacts.** Tests/builds/diffs/review notes must be traceable to the task/run.

### Defer until core reliability is proven

- Public internet rooms and hosted collaboration service.
- Live simultaneous editing or shared terminals.
- Autonomous remote background agents with broad network access.
- Multi-model “battle” or best-of-N workflows.
- Voice/video and general-purpose social features.
- Automatically learned project memory that writes itself into durable team knowledge.
- Marketplace-scale theme sharing and untrusted third-party theme bundles.
- Full custom design system replacing native IDE workbench controls.

### Do not promise until validated

- Universal extension compatibility or exact VS Code Marketplace parity.
- A model-agnostic experience where every provider has identical capabilities.
- Complete security from worktree isolation alone.
- Correctness because an agent says tests passed or shows a screenshot.
- Realtime collaboration without explicit data access and conflict rules.
- Any AI privacy guarantee that has not been implemented, audited, and documented for each provider path.

## 10. Recommended user-facing feature map

### Workbench

**Open and understand projects:** folders, multi-root, search, symbols, diagnostics, Git, tasks, tests, terminal, debugging, extensions, remote workspaces.

### Studio

**Shape the environment:** themes, editor schemes, icons, fonts, backgrounds, motion, layouts, profiles, share/import, reset and accessibility checks.

### Room

**Coordinate people and agents:** presence, roles, tasks, messages, updates, activity, ownership, files/diffs by permission, handoffs.

### Agents

**Delegate and review:** Ask, Plan, Build, context selection, model/provider, tool permissions, worktree choice, run status, pause/stop, diffs, checks, rollback.

### Project Knowledge

**Keep shared context current:** architecture, decisions, conventions, ownership, dependencies, links, provenance, review date, expiry.

The surfaces should connect. Example: an agent accepts a Room task → creates a Plan artifact → works in an isolated worktree → reports changed files and test results → a teammate reviews the diff → the room records the handoff.

## 11. Practical sequence for building

1. **Stabilize the fork.** Reproducible build, update process, baseline IDE smoke checks, extension compatibility statement.
2. **Redesign the experience architecture.** Define global layout and design tokens while preserving native editor areas; decide where Room, Studio, and Agents live.
3. **Build Theme Studio MVP.** Theme preview, separate theme layers, curated starter themes, profiles, accessibility/reduced motion, restore.
4. **Replace LAN prototype transport.** Define room protocol/service, identity, reconnect, persistence, membership permissions, threat model.
5. **Ship Room MVP.** Presence, tasks, notes/messages, attributed activity, explicit metadata/file/diff sharing levels.
6. **Ship task-linked agent sessions.** Ask/Plan/Build, context controls, worktree isolation, pause/stop, diff review, checkpoints.
7. **Connect proof to handoff.** Test/build/review/browser evidence, visible failures, merge/commit state, ownership transfer.
8. **Evaluate internet rooms and remote agents.** Only after auth, encryption, data handling, command permissions, audit, and incident response are designed.

## 12. Feature-level source notes

The following sources are official product documentation or first-party material. Use them to validate implementation details during design; do not treat vendor claims as independent evaluations.

### VS Code / Code OSS

- [Core editor overview](https://code.visualstudio.com/docs/editing/getting-started/overview) — workbench, workspace, edit/run/debug/test/Git/terminal/extension model.
- [Profiles](https://code.visualstudio.com/docs/configure/profiles) — configuration bundles, workspace associations, export/import, sync.
- [Themes](https://code.visualstudio.com/docs/configure/themes) — color theme selection, previews, workspace customization, high contrast.
- [Theming extension API](https://code.visualstudio.com/api/extension-capabilities/theming) — color, file icon, and product icon themes.
- [Agent tools](https://code.visualstudio.com/docs/copilot/concepts/tools) — built-in, MCP, and extension tools.
- [Agent approvals and permissions](https://code.visualstudio.com/docs/agents/run/approvals) — approval levels, tools, terminal, URLs, sandboxing.
- [Agents window](https://code.visualstudio.com/docs/agents/run/agents-window) — sessions across workspaces, review, and worktree isolation.
- [Manage agent sessions](https://code.visualstudio.com/docs/agents/run/sessions/manage-sessions) — session lifecycle, chats, folders, and worktrees.
- [Agent workflows](https://code.visualstudio.com/docs/copilot/chat/chat-agent-mode) — agent interaction and plan/build workflows.

### Cursor

- [Agent overview](https://cursor.com/docs/agent/overview) — tools, local indexing, checkpoints, projects, and agent workflow.
- [Plan Mode](https://prod.cursor.com/docs/agent/plan-mode) — repository research, clarification, editable plans, saved plan documents.
- [Rules](https://docs.cursor.com/context/rules) — project/user rules and memories.
- [Context references](https://docs.cursor.com/context/%40-symbols/overview) — files, folders, code, docs, Git, recent changes, and rules.
- [Worktrees](https://cursor.com/docs/configuration/worktrees) — isolated checkouts, setup, review, and best-of-N.
- [Background agents](https://docs.cursor.com/background-agent) — async remote execution and security considerations.
- [Checkpoints](https://docs.cursor.com/en/agent/chat/checkpoints) — local Agent-change snapshots, restore, and limits.
- [Agent Review](https://prod.cursor.com/docs/agent/agent-review) — configurable quick/deep local change review.
- [Security and indexing](https://www.cursor.com/security) — indexing flow, ignore rules, privacy mode, providers, and data boundaries.
- [Projects](https://cursor.com/docs/agent/projects) — coordinator and long-lived project context.

### Windsurf

- [Documentation index](https://docs.windsurf.com/llms.txt) — current docs discovery; names/paths may move.
- [Cascade hooks](https://docs.windsurf.com/de/windsurf/cascade/hooks) — lifecycle hooks, enforcement, validation, setup, and admin/workspace scopes.
- [Arena mode](https://docs.windsurf.com/ro/windsurf/cascade/arena) — isolated model comparisons and worktree implications.
- [Windsurf plugin overview](https://docs.windsurf.com/plugins) — supported editor surfaces and remote development plugins.

### Zed

- [Collaboration overview](https://zed.dev/docs/collaboration/overview) — multiplayer editing, Channels, shared projects, contacts, and voice.
- [Channels first-party article](https://zed.dev/blog/channels) — long-lived work streams, notes, presence, and pairing/async intention.
- [LLM providers](https://zed.dev/docs/ai/llm-providers) — hosted, API, existing subscription, gateway, local model paths.
- [AI privacy](https://zed.dev/docs/ai/privacy-and-security) — request path distinctions and data policies.
- [Edit Prediction](https://zed.dev/docs/ai/edit-prediction) — provider choice and inline multi-line prediction behavior.
- [Parallel agents](https://zed.dev/docs/ai/parallel-agents) — project/thread organization and parallel agent model.

### JetBrains

- [Junie agent](https://www.jetbrains.com/help/ai-assistant/junie-agent.html) — modes, model/reasoning selection, debugger mode, and rollback.
- [AI agents](https://www.jetbrains.com/help/ai-assistant/agents.html) — integrated agent choices, authentication, project instructions, MCP.
- [Colors and fonts](https://www.jetbrains.com/help/idea/configuring-colors-and-fonts.html) — distinction between UI themes, editor schemes, fonts, and sharing.
- [Working with projects](https://www.jetbrains.com/help/idea/working-with-projects.html) — project/global settings boundary.

### GitHub Copilot

- [Use Copilot agents](https://docs.github.com/en/copilot/how-tos/copilot-in-the-ide/use-copilot-agents) — agent, plan, delegation, custom agents, cloud sessions.
- [Repository instructions](https://docs.github.com/en/copilot/how-tos/configure-custom-instructions-in-your-ide/add-repository-instructions-in-your-ide) — repository-wide and path-specific guidance.
- [Agent code review](https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/copilot-code-review) — PR reviews and repo-specific review instructions.

### Google Antigravity

- [IDE overview](https://www.antigravity.google/docs/ide/overview/) — async local agents, editor/browser surfaces, artifacts.
- [Agent overview](https://www.antigravity.google/docs/agent/) — tools, artifacts, knowledge, modes, and parallel conversations.
- [Product overview](https://antigravity.google/product/antigravity-ide?hl=en) — task-level artifacts and user feedback.
- [Google Developers announcement](https://developers.googleblog.com/en/build-with-google-antigravity-our-new-agentic-development-platform/) — editor/agent division, browser work, feedback on artifacts.
- [IDE Codelab](https://codelabs.developers.google.com/getting-started-agy-ide) — plans, diffs, walkthrough artifacts and user comments.

---

## Bottom-line product recommendation

Forge Room should not try to out-feature every IDE at once. It should **inherit the dependable workbench, make themes unusually expressive, and make collaboration unusually coherent**. The strongest differentiated product loop is:

> **Pick a project look → open the project room → assign a human or agent task → work in an inspectable session → review changes and evidence → hand work back to the team.**

That sequence gives the theme system emotional identity and gives the lobby a job beyond chat. Build and validate each link before adding public rooms, free-running agents, or social features.
