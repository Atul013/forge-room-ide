# AI core features: competitor research and Forge Room implementation direction

**Research checked:** 20 October 2026  
**Scope:** AI features that materially change software development inside an IDE: context gathering, planning, tool use, permissions, execution, review, recovery, project memory, and collaboration.  
**Method:** Review of public first-party documentation from VS Code, Cursor, Google Antigravity, Anthropic Claude Code, and OpenAI Codex. This is product and workflow research, not an independent quality or model benchmark. Capabilities and availability vary by harness, operating system, account, model, and preview status.

## Executive synthesis

The standout AI IDE is not just a chat panel with file generation. It is a controlled work loop:

1. **Understand:** find relevant workspace evidence without pushing the whole repository into context.
2. **Plan:** create a concrete, reviewable proposal before broad changes.
3. **Authorize:** show what tools can do, which data will leave the workspace, and how approval applies.
4. **Execute:** keep the task, context, tool results, progress, and changes attached to one resumable session.
5. **Review:** expose diffs and verification evidence as first-class artifacts.
6. **Recover:** continue, redirect, cancel, or roll back without losing track of what already happened.
7. **Coordinate:** link the agent session to the human room, owner, task, and handoff.

Forge Room already has provider-neutral Ask/Plan/Build, opt-in context selection, approved bounded search/read tools, visible transfer consent, diff proposals, task association, checkpoints, transcript retention, and restart recovery. The Plan handoff supports editable review before Build, and saved Plan artifacts consistently include the original request, proposal, model, creation time, and optional room task. Transfer consent lists selected source paths and character counts, then asks the selected provider to estimate the exact serialized request size before a separate generation confirmation. During tool use, progress reports the outcome, bounded source paths, and output size without exposing query arguments or returned content. Applied Build checkpoints export the request, complete diff, task/provider metadata, and recorded verification outcomes. Remaining gap is provider-backed runtime validation across supported IDE and provider combinations. Direct shell/write/network tools, unattended work, and wider persistent permissions should wait for stronger sandboxing, policy, audit, and revocation controls.

## Feature-by-feature research

### 1. Workspace understanding and context selection

- **VS Code:** agent mode can call built-in, MCP, and extension tools. Official guidance recommends narrowing available tools: each tool adds a choice for the model and its output consumes the context window. Workspace search and file reading retrieve relevant excerpts without inserting the entire project into every request. Tool references do not bypass approval.
- **Cursor:** Ask is read-only and codebase-oriented; Agent can search, edit, and run tools. Codebase search, file references, and reusable project rules reduce repeated manual context assembly.
- **Claude Code:** project and user memory files establish persistent conventions; scoped instructions make recurring architectural guidance available across sessions.
- **Forge Room implication:** context should remain selected, attributed, size-bounded, and visible before provider transfer. Search results should show file and line provenance. Exclusions and workspace boundaries should be applied before content reaches the provider.

**Trade-off:** indexing and automatic retrieval reduce friction but can include sensitive or irrelevant content. Broad default context is not a substitute for explicit provenance and review.

### 2. Modes and the plan-to-build boundary

- **VS Code:** its planning workflow researches first, asks for clarification when needed, presents a plan for review, and supports implementation handoff. Plan and conversation context may carry into the implementation agent.
- **Cursor:** Ask, Agent, and configurable modes vary tool access. Ask is for read-only investigation; Agent is for multi-file implementation. This makes the capability boundary legible before the user submits work.
- **Antigravity:** `/plan` produces a structured implementation-plan artifact, supports inline feedback, and waits for a separate Proceed decision under its review policy. Planning and reasoning are distinct from tool permission.
- **Claude Code:** permission modes include planning-only behavior as a distinct way to prevent changes while exploring.
- **Forge Room implication:** Plan is instructed to return a stable structure (goal, affected files, ordered steps, verification, risks and assumptions). Forge Room implements a deliberate Build-from-Plan action: it includes the original request and latest Plan response in a new Build request, asks again about context, tools, and provider transfer, and preserves separate diff and apply confirmation. The user can edit the plan before handoff and save it as a Markdown artifact with the original request, proposal, model, timestamp, and optional room task. The artifact preserves the model's Markdown rather than validating a machine-readable schema, so schema validation and provider-backed runtime validation remain open.

**Trade-off:** a plan can become stale as files change. The Build handoff should label it as a proposal and re-check current files rather than treating it as a trusted execution script.

### 3. Tool access, approval, and sandboxing

- **VS Code:** approval can be per call or remembered for a session/workspace/tool; permission presets control autonomy. Sandboxing is a separate layer from approval and can constrain file and network access after a user approves a command.
- **Antigravity:** sensitive operations map to fine-grained allow/ask/deny rules with explicit precedence. Its documented presets separate terminal, file, and web/MCP policies; sandbox mode separately constrains execution.
- **Cursor:** foreground Auto-run is configurable; background agents run in remote, internet-connected machines and therefore change the trust boundary. Its documentation calls out the associated prompt-injection and exfiltration risk.
- **Claude Code:** tool allow/deny lists and permission modes make tool capabilities configurable; bypassing prompts is presented as a higher-risk choice.
- **OpenAI Codex:** product and harness materials describe execution sandboxing, approval policies, tool routing, and human review as parts of the surrounding agent runtime rather than something the language model provides by itself.
- **Forge Room implication:** keep four independent switches: available tools, per-action approval, execution boundary, and artifact/apply approval. `Allow for this request` must expire with the request. Denied actions should stay denied for that call. No confirmation choice should silently enable network, terminal, write, or external service actions.

**Trade-off:** fewer prompts improve flow but make it easier to miss a risky input. Tool-specific, time-limited grants are clearer than a single global â€œautonomousâ€ toggle.

### 4. Execution progress and session state

- **VS Code/Copilot:** agents expose sessions, tool calls, output, approval requests, and recovery controls. Session continuity can span supported interfaces, depending on harness/provider.
- **Cursor:** chat tabs keep conversations and model selection distinct; checkpoints and history provide task continuity. Its documentation clarifies that checkpoints track agent changes, not manual edits, and are not version control.
- **Codex:** the harness owns streaming, conversation state, approvals, tools, progress, and carrying work across turns; the app surface provides a place to inspect and review.
- **Forge Room implication:** keep task state and conversation state related but separate. Show active mode/provider, selected task, currently approved context, requested tool, files proposed/applied, and whether verification has actually run. Resume should reload a bounded snapshot, require provider transfer approval again, revalidate paths, and preserve an honest â€œnot testedâ€ state.

**Trade-off:** persistence aids long tasks but stores sensitive prompts and file content. Retention must be optional, bounded, local, clearly described, and deletable.

### 5. Change review, verification, and rollback

- **Cursor:** changes are presented through diff review and can be applied selectively. Automatic checkpoints are local recovery points for agent changes, with explicit limitations.
- **Antigravity:** plans and code diffs are reviewable artifacts; users can annotate plans and decide when to proceed.
- **VS Code:** approval and sandboxing protect actions, while review workflows encourage checking generated edits and evidence before integration.
- **Codex:** its public material emphasizes traced tasks, reviewable diffs, tests, and high-signal code review; this is a vendor-described product direction rather than a comparative benchmark.
- **Forge Room implication:** each Build result should connect the original request, plan, changed paths, diff, user decision, verification command/task, exit result, and known failures. Never imply a check passed unless it ran and its result is recorded. Rollback should guard against intervening user edits.

**Trade-off:** a successful command is not proof of correctness. Report exactly what ran, what passed, what failed, and what was skipped.

### 6. Project memory and reusable behavior

- **Cursor:** version-controlled project rules can be always-on, path-scoped, or manually attached; `AGENTS.md` is supported as a conventional lightweight alternative.
- **Claude Code:** project and user memory files carry stable instructions across sessions, with separate scopes for team conventions and personal preferences.
- **VS Code:** custom agents and prompt files can define reusable roles, tool sets, and instructions.
- **Codex:** skills package reusable workflows and team standards; repository guidance applies to recurring project behavior.
- **Forge Room implication:** project instructions should be discoverable, editable, version-controlled, and selected per request with their file and character count named in the transfer confirmation. Future scoped rule files should state their matching paths and whether they are always applied.

**Trade-off:** persistent instructions can become stale or contain unsafe directives. Show what is active and allow review before transfer; do not let instructions bypass tool approvals.

### 7. Collaboration and parallel work

- **Cursor and Codex:** worktrees separate independent task changes and enable parallel sessions. Remote/background agents can continue asynchronously but introduce repository upload, network, credential, and retention concerns.
- **Antigravity:** project-scoped permissions, worktrees, and structured artifacts make multi-step work reviewable.
- **VS Code:** task artifacts and session controls connect planning, implementation, and review inside the workbench.
- **Forge Room implication:** the Project Room can be the differentiator: one task should link owner, agent session, worktree, active files, diffs, verification, and handoff. Start with local/self-hosted rooms and explicit metadata/content sharing; do not infer code access from room presence.

**Trade-off:** remote parallelism is not merely a speed feature. It requires explicit execution location, repository access, network policy, secrets handling, retention, cost limits, and audit trail.

Room membership now syncs the host and connected participants to every client. Direct messages use participant IDs, and event history is filtered by sender and recipient rather than display name, preventing participants with the same display name from receiving each other's messages. The room host can revoke an active invite: the secret is removed before connected sessions are closed, the server rejects subsequent operations on that socket, and a host-attributed history event records revocation without the secret. Per-resource capability grants and live two-client validation remain open. The current collaboration slice adds code-anchored comments that share a bounded comment, relative path and one-based line range only; selected source text is not transmitted. The event is versioned and bounded, and recipients can navigate to the saved location in their matching workspace. Anchors can shift as files change. See [AI-native repository audit](AI_NATIVE_REPO_AUDIT.md) for repository evidence, architecture decisions, requirements gaps and the staged acceptance plan.

## Forge Room priority decisions

| Priority | Decision | Rationale | Proposed acceptance evidence |
| --- | --- | --- | --- |
| Shipped partial; validate | Plan review editor and explicit Build-from-Plan handoff | Lets the user revise a proposal before implementation while preserving a distinct approval boundary | The editor shows the original request and allows up to 100,000 characters of plan edits; Build gets the original request and edited plan, then context/provider/tools are confirmed again. IDE runtime behavior remains unverified; richer structured planning controls remain open |
| Shipped; validate | Make tool grant scope visible and time-limited | Read-only tools offer per-call or current-request approval, with a separate context/provider transfer confirmation | Request-only access expires after the request, remains read-only, and does not alter shell/network/write permissions; runtime provider validation remains outstanding |
| Shipped; extend and validate | User-selected Markdown Build review bundle | Turns an applied checkpoint, diff and captured verification into a portable review document; it does not yet combine every agent/tool event or room state | Export includes the request when retained, changed paths, full patch, checkpoint and recorded verification results; no verification is claimed when none was recorded; the current focused extension unit suite passes; VS Code export and Git API integration runtime validation remain outstanding |
| Shipped; extend | Context provenance in transfer consent | Users can see each selected source path and character count, plus the aggregate selected-context character count, before sending; tool results are separately approved and bounded | Provider token estimate and tool/source provenance in review bundles are implemented; tool-call progress now shows outcome, bounded source paths, and output size without query or result text; exact runtime mapping across provider tool turns remains open |
| Shipped partial; extend | Scoped project rules and reusable agent profiles | Reduces repeated setup while preserving control | Root `AGENTS.md` and `.forge-room/instructions.md` can be selected per request; source paths and character counts appear in transfer consent, and rules grant no permissions. Path-scoped guidance and reusable profiles are implemented in Team Lobby 0.63.0 with explicit per-request selection, bounded context, and transfer disclosure. |
| Later | Write/shell/web adapters and broader permission levels | Needed for autonomous execution, but safe only after a sandbox and policy boundary exist | Tool allow/ask/deny rules; per-request grants; workspace boundary; command/network sandbox; redaction; audit and rollback |
| Defer | Remote background and unattended multi-agent execution | Large security, hosting, cost, identity, and operations surface | Only after managed accounts, isolated execution, secret broker, network egress policy, retention and auditable handoff exist |

## Current Forge Room gap assessment

Implemented in current source before and during this pass: provider-neutral Ask/Plan/Build; provider choice; selection of files, editor selection, diagnostics, symbols, terminal capture, web pages, and per-request `AGENTS.md`/`.forge-room/instructions.md` project guidance; bounded read-only search/read tools; per-call or request-only approval for those tools; model-transfer consent; Build diff proposal and explicit apply; guarded rollback checkpoints; task association; local opt-in session persistence; local task-linked worktrees; room task history and verification event.

Team Lobby 0.100.0 also resolves saved AI sessions against their recorded workspace-folder URI. In multi-root workspaces, a continued session stays on its original root, Plan-to-Build handoff carries that same root forward, and Plan export defaults to it. Helper coverage proves resolution of a non-first root and rejection of a removed root; actual provider-backed UI behavior remains unverified.

Team Lobby 0.101.0 initially associates captured terminal output with the workspace folder containing the command's shell-reported cwd. At command end, a different or missing reported cwd clears that association. Context selection filters known roots and omits unknown-root captures for multi-root requests. Helper tests cover root stability/filtering and single-root compatibility; live shell integration behavior remains unverified.

Not implemented or not yet proven: runtime reliability of Plan-to-Build handoff; richer evidence linking across plan, tool calls, room activity, failed checks and applied changes; enforceable aggregate token budget and richer live tool-result provenance UI; permission rules for write/shell/network tool adapters; OS-level sandbox for model-initiated commands; provider-backed runtime validation; automatic conflict-path monitoring validation; public managed room service; remote agent execution.

These findings are source-based and do not establish feature quality, performance, or reliability under real provider workloads.

## Sources

- [VS Code agent tools](https://code.visualstudio.com/docs/copilot/concepts/tools)
- [VS Code approvals and permissions](https://code.visualstudio.com/docs/agents/run/approvals)
- [VS Code planning](https://code.visualstudio.com/docs/agents/run/planning)
- [VS Code workspace context](https://code.visualstudio.com/docs/agents/reference/workspace-context)
- [Cursor modes](https://docs.cursor.com/en/agent/modes)
- [Cursor agent overview and checkpoints](https://docs.cursor.com/chat/overview)
- [Cursor checkpoints](https://docs.cursor.com/en/agent/chat/checkpoints)
- [Cursor project rules](https://docs.cursor.com/context/rules-for-ai)
- [Claude Code CLI permissions and resume](https://docs.anthropic.com/en/docs/claude-code/cli-usage)
- [Claude Code memory](https://docs.anthropic.com/en/docs/claude-code/memory)
- [Antigravity plans](https://www.antigravity.google/docs/plan/)
- [Antigravity artifact review](https://antigravity.google/docs/artifact-review)
- [Antigravity permissions](https://antigravity.google/docs/permissions)
- [OpenAI Codex harness architecture](https://developers.openai.com/blog/codex-as-a-platform)
- [OpenAI Agents sandbox model](https://developers.openai.com/api/docs/guides/agents/sandboxes)

## Implementation update - model input budget consent

Team Lobby now asks separately before sending selected context to the provider token-count API, reports the resulting prompt-text estimate against that model's declared input limit, and asks again before generation. Invalid estimates and over-limit inputs fail closed before generation. Counting is itself a provider operation and is disclosed; it is not represented as local-only. The estimate may omit provider-side tool overhead, while tool-result turns continue to be recounted before each request. The Code OSS language model provider contract assigns token estimation to the provider and describes it as an estimate: [Language Model Chat Provider API](https://github.com/microsoft/vscode-docs/blob/main/api/extension-guides/ai/language-model-chat-provider.md).


## Implementation update â€” workspace-bound AI context reads

In Team Lobby 0.58.0, selected workspace files, project guidance, selected Agent Skills, and model read/search results are resolved to filesystem realpaths under the selected root before being read. Escaping symlinks are rejected. Likely credential sensitivity is computed against both the selected path and its resolved in-workspace target, preventing an ordinary-looking alias from bypassing a sensitive-file prompt. Separate user confirmations and provider-transfer approval remain in place. 65/65 extension tests pass, including path traversal, outside-link rejection, canonical in-root links, and target path sensitivity. Full IDE provider-backed UI validation remains open.

## Verification evidence and review handoff

Team Lobby 0.59.0 stores the redacted, bounded verification command, exit code, and timestamp in the local Build checkpoint. It marks whether evidence was explicitly shared with the room, and review bundle output distinguishes room-shared from local-only evidence. The room receives command text only through the explicit per-run share choice. Masking is heuristic; runtime approval flows still require IDE validation.


Team Lobby 0.60.0 allows selecting and running up to ten configured verification checks after Build apply. Each result, including nonzero exits and process/launch failures, is retained in the checkpoint and exported in the review bundle; at most twenty records are kept. Build and 66/66 focused tests pass. Interactive task-picking and multiple task execution need IDE runtime validation.

Team Lobby 0.61.0 adds bounded source provenance to Build checkpoint review exports for approved read-only tools. It records tool name, matched/read source paths, output character count, outcome, and timestamp while excluding query strings and returned text. Tests validate these privacy boundaries and evidence limits; provider-backed operation and IDE export still need runtime exercise.

## Implementation update â€” Team Lobby 0.62.0

Reusable agent profiles now complement selected project instructions and Agent Skills. The local .forge-room/agent-profiles.json format supports up to twelve named profiles, with bounded descriptions and instructions. A user chooses one profile per request; profile guidance is explicitly untrusted and adds no tools or permissions. The selected name and instruction length appear in the provider transfer receipt, and the profile text is counted in the exact prompt token estimate. The editor command creates a starter profile file without overwriting an existing one. Validation and all 73 Team Lobby tests pass. Runtime provider behavior remains unvalidated in this dev profile.


Team Lobby 0.63.0 extends project guidance discovery to nested AGENTS.md files. Per-request source selection, workspace containment, sensitive-path confirmation, candidate and character bounds are implemented; provider-backed IDE interaction remains to validate.


## Implementation update â€” Team Lobby 0.64.0

Live tool lifecycle progress is now visible for Ask, Plan, and Build. It discloses the tool, outcome, up to three source paths, and output size while excluding query arguments and returned content. The same formatter is covered by a privacy-boundary test. Provider-backed runtime validation remains open.


## Implementation update â€” Team Lobby 0.65.0

Task-linked Build and verification events now include a stable task identifier with host-side owner authorization. This improves evidence linking beyond title text. Protocol 12 and local transport coverage are included; multi-window IDE runtime and provider-backed validation remain open.

## Expansion prompt reconciliation â€” 21 October 2026

The supplied master prompt is a product and implementation brief, not evidence of shipped features. Its audit, gap matrix, architecture decisions, skills sourcing report and staged delivery plan have been reconciled against the actual Code OSS checkout in [FORGE_AI_NATIVE_EXPANSION_RECONCILIATION.md](../planning/FORGE_AI_NATIVE_EXPANSION_RECONCILIATION.md). The first multiplayer slice is substantially implemented in source; separate IDE runtime acceptance is still required. Team Lobby 0.69.0 adds stable task-agent identity across Build continuations. The package passes its build/check and 84 tests. Package SHA-256: `725D6EEAA6EF1DDC1C12A8B0D9C1EF4CC6FB0EFF5F428DCC664731976F035441`.

## Implementation update â€” Team Lobby 0.70.0

The AI request loop now resolves workspace tool calls through a typed capability registry before approval or execution. Registered IDs separate workspace search from file reading; validation rejects unknown tools, extra properties, traversal paths, malformed input and control characters. Each request is bound to the selected workspace, and result text remains capped at 12,000 characters per call and 30,000 per request. Local audit records retain actor, workspace/resource scope, approval, outcome and output size while omitting queries and result contents. Sensitive reads keep their additional file-specific consent. Eighty-eight tests pass. A real provider-backed approval/cancellation flow still needs IDE validation.

## Implementation update â€” Team Lobby 0.71.0

The room transport integration harness now exercises reviewed diff and code-anchored comment events through the same validator used by the extension host. Tests prove room attribution, event timestamps, delivery to both clients, reconnect-history replay and rejection of selected-source fields on comments. Invite revocation tests verify active-session disconnect, a host-attributed event without the invite secret and rejection of a subsequent join. The tests validate protocol and validation behavior; they do not exercise IDE dialogs or real network interfaces.


## Implementation update - task worktree identity validation (0.78.0)

Task-linked Build and Build resume now verify the current Git worktree root, repository common directory, recorded local branch, and task-assigned branch together. Integration coverage creates a real temporary repository and linked worktree and checks valid identity and branch mismatches. Both extension copies pass 108/108 tests; full IDE Build and resume behavior still needs runtime acceptance.



## Implementation update - stale task worktree recovery (0.79.0)

If a task's local worktree path is missing or stale, Forge Room scans Git's registered worktrees for the assigned branch, checks the candidate's actual path, branch, and repository identity, then refreshes the local path record. Tests exercise discovery against a real Git worktree list and reject wrong branches. Both copies pass 108 tests; full IDE recovery behavior still needs runtime validation.


## Implementation update - task worktree resolver proof (0.80.0)

The production task-worktree resolver now owns cached-path validation, branch-based Git discovery, repository identity checks, and recovery mapping. A real Git integration test exercises that resolver against valid, stale, and missing task-worktree states. Both extension copies pass TypeScript checks and 108/108 tests. The separate IDE Build/resume UI flow remains to validate.


## Implementation update - safe task worktree cleanup (0.81.0)

The task worktree lifecycle now includes a user-confirmed cleanup command. It only acts on the exact task-linked local worktree, refuses dirty/untracked files and the currently open workspace, rechecks after confirmation, uses Git without force, and keeps the branch and shared task assignment. A real-Git test covers dirty refusal and clean removal with branch retention. Both extension copies pass 109/109 tests; IDE command-palette interaction remains to validate.

## Shared editing research and implementation status - 21 October 2026

Yjs is a network-agnostic CRDT layer: its official update documentation describes updates as commutative, associative and idempotent, so peers converge when they eventually receive the same updates, including duplicate or reordered delivery. Yjs does not define authorization, consent, transport or room membership; Forge Room must enforce those separately. The VS Code extension API provides `workspace.applyEdit` for applying document changes from an extension. These sources support a maintained CRDT approach over ad-hoc whole-file last-write-wins replacement, while leaving revocation, recovery and concurrent editor UX to Forge Room.

Team Lobby 0.82.0 connects the Yjs helper to an opt-in room session and VS Code text documents. The host announces the relative path, approves each subscriber, and relays bounded updates only among approved peers. A joiner confirms that the host buffer replaces the local editor's in-memory text. Reconnect state merges after renewed host approval; state is not persisted and edits are not saved to disk automatically. Two-IDE editor interaction and reconnect still require manual acceptance.

## Implementation update - multi-root live shared documents (0.83.0)

Shared editing now supports files in uniquely named roots of a multi-root workspace. Room paths prefix the root name; the client resolves that name to exactly one local folder using the existing realpath containment check. Ambiguous folder names and unsafe paths fail closed. Protocol version 16 marks the path semantics. Both extension copies pass TypeScript checks and 116/116 tests; the bundled nine-file VSIX hash matches the Code OSS built-in extension manifest. Separate IDE interaction remains to verify.

Sources checked 21 October 2026: [Yjs document updates](https://docs.yjs.dev/api/document-updates), [Yjs collaborative editor guide](https://docs.yjs.dev/getting-started/a-collaborative-editor), and [VS Code API reference: workspace.applyEdit](https://code.visualstudio.com/api/references/vscode-api).

## MCP tools implementation update â€” 21 October 2026

The [MCP TypeScript SDK v2 client guide](https://ts.sdk.modelcontextprotocol.io/v2/get-started/first-client.html) documents `Client` plus `StdioClientTransport`, tool listing and tool invocation. The SDK's latest stable v2 release line implements the 2026-07-28 protocol. Team Lobby 0.89.0 uses `@modelcontextprotocol/client` 2.3.1 for local stdio servers in Ask/Plan. Users configure trusted commands at machine scope, select servers for each request, approve provider transfer before process startup, then approve each tool call with its argument summary visible. Tools and output are bounded and processes close at request completion.

This adds MCP as an optional context/tool source, while preserving the product's separation between tool availability, user approval, runtime isolation, and Build apply review. Approval is not containment: an MCP server can independently read/write files, execute child processes, or access the network with the IDE user's privileges. Real-server interoperability and IDE-provider acceptance remain open. Remote Streamable HTTP/OAuth, MCP resources/prompts, per-tool remembered grants, and OS isolation are not in this slice.

### Task/test capability research update â€” 21 October 2026

The official VS Code Extension API confirms that `tasks.executeTask` returns a `TaskExecution` that can be terminated, `onDidEndTaskProcess` provides the process exit code (which is undefined when terminated), and `onDidEndTaskProcess` does not fire for tasks without an underlying process. Forge therefore waits for process-end evidence, treats an IDE task-end without process evidence as unverified, keeps task output in the IDE terminal, and reports cancellation/timeout as a termination request rather than claiming the process is definitely stopped. Test execution is an OS-privileged project-code boundary and has its own exact-task modal approval. Sources: [VS Code Extension API: Tasks](https://code.visualstudio.com/api/references/vscode-api#tasks), [Task Provider guide](https://code.visualstudio.com/api/extension-guides/task-provider), checked 21 October 2026.

## Team Lobby 0.102.0 capability update â€” active debugger frames

The attached expansion prompt calls for a native IDE capability surface that agents can invoke through typed, observable tools. Team Lobby now exposes `debugger_frames` in Ask, Plan, and Build. It is read-only, requires the active paused thread to belong to the selected debug session and workspace, caps results at 20 frames, filters sensitive or out-of-workspace sources, and returns only frame names and relative source locations. Per-call approval applies unless the user has granted the request-scoped read-only tool approval. It never returns variables or invokes debugger control actions.

The implementation uses VS Code's documented `DebugSession.customRequest` API with DAP `stackTrace`. The protocol defines the request shape, but concrete adapter behavior may vary; runtime acceptance remains open. Sources checked 21 October 2026: [VS Code Extension API: DebugSession](https://code.visualstudio.com/api/references/vscode-api#DebugSession), [Debug Adapter Protocol overview](https://microsoft.github.io/debug-adapter-protocol/overview.html).

### ACP v1 session continuation

Configured ACP Ask/Plan agents can resume prior sessions only if they advertise `agentCapabilities.loadSession`. Forge stores the provider-issued session ID and authentication-method ID in workspace state, keyed to workspace, mode, command, and arguments. It never stores the ACP prompt or response in this record. Before loading, Forge asks for explicit permission and warns that the agent will restore conversation history previously sent to it. A changed auth method prevents session reuse. History replay is discarded from the new answer. No capability means a fresh session. An expired or unavailable session fails with the agent's error and leaves the saved ID for the user to retry or replace.

This follows the official [ACP v1 session setup documentation](https://agentclientprotocol.com/protocol/v1/session-setup), which gates `session/load` on the agent's advertised `loadSession` capability and replays history through `session/update`. The ACP agent runs as a local process with ambient OS privileges; session resume does not add containment. Checked 21 October 2026.

## Team Lobby 0.104.0 capability update â€” terminal results under explicit consent

The attached expansion prompt lists terminal output only when explicitly shared. The native tool `terminal_last_output` now lets an agent request the newest locally captured shell output for its selected project. Forge freezes the exact snapshot before displaying a mandatory per-call confirmation, even with a broad read-only grant. The dialog identifies a credential-redacted command label, terminal, run status, size and truncation, and warns that output may contain secrets. Multi-root requests exclude unknown or other-root captures. Evidence keeps capability and size only. The source implementation and test coverage are documented in `BUILD_STATUS.md`.


## Team Lobby 0.105.0 capability update â€” controlled file edits

The native Build tool catalog now includes a constrained pply_workspace_edit operation, closing the basic iles/apply-patch capability gap called out in the attached prompt. Forge accepts only one exact unique replacement in a selected-root non-sensitive file, displays the full before/after diff and requires a separate approval. It rejects the action if the document version changes during review, then applies unsaved through the IDE WorkspaceEdit API. This is not an arbitrary unified-diff or multi-file patch engine; broader patch workflows and live UI/provider acceptance remain open.


## Team Lobby 0.106.0 capability update â€” rollback for approved direct edits

The exact-match pply_workspace_edit operation now uses the established Build checkpoint lifecycle. Forge stores only the touched file's before/after text in a normal checkpoint before applying the edit, links its ID to room task activity, and lets users restore it through Roll Back Last Build. The standard conflict guard prevents overwriting later file changes. Both manual Undo and the explicit checkpoint path are available; live IDE interaction remains to verify.


## Team Lobby 0.107.0 - workspace inventory capability

The expansion prompt's minimum native tool catalog includes workspace/list. Forge now implements this as a read-only workspace_list operation shared by Ask, Plan, and Build. The tool returns file paths only, scoped to the user's selected file-based workspace root, and caps results at 250 paths and 12,000 characters. It excludes likely sensitive paths and common generated folders, resolves candidates through the existing canonical path-containment check, and respects cancellation. The approval summary discloses that names are sent to the selected provider. Local evidence stores tool name, outcome and output size without retaining the path list. This fills a discovery gap between workspace search and named-file reads; provider-backed invocation remains an acceptance requirement.


## Team Lobby 0.108.0 - native editor navigation

A workspace-aware agent needs to move from a path it found to the user's actual editor. Forge's editor_open capability completes that navigation step without making the file readable to the model by virtue of opening it. The path remains rooted and non-sensitive, an optional line is validated against the document, and every call requires a dedicated approval. The structured result reports the opened path and line only. This is a local UI action; provider-backed behavior and focus/preview expectations still need live acceptance.
