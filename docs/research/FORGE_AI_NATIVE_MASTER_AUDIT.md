# Forge AI Native Expansion: Current Audit and Build Plan

**Snapshot:** 21 October 2026; refreshed for Team Lobby 0.108.0 and Studio 0.14.0
**Input:** `Forge_AI_Native_Expansion_Master_Prompt.txt` in the user's Downloads folder. Its contents are treated as product requirements, not as proof of implementation.  
**Code inspected:** Code OSS 1.144.0 checkout, handoff mirror, extension tests, room-service tests, package manifests and current Team Lobby release package.

## 1. Repository audit

Forge is a Code OSS desktop IDE fork with two first-party extensions. Team Lobby 0.108.0 provides the collaboration and AI feature set described below. Forge Room Studio 0.14.0 provides six native themes and allowlisted appearance profiles through supported extension APIs. Optional self-hosted room-service code lives in `services/room-service`; it provisions accounts, rooms and expiring/revocable invite credentials over HTTPS, then relays the existing room protocol over WSS. Team Lobby bridges that relay to a loopback room authority. The service stores account/room/invite control data and lifecycle audit records, but not active room content.

The AI path supports provider-neutral Ask/Plan/Build through the IDE Language Model API, optional ACP v1 Ask/Plan through a locally configured subprocess, and optional MCP stdio tools in Ask/Plan through the official TypeScript client SDK 2.3.1. Context receipts, explicit transfer consent, typed read-only workspace capabilities, per-call approval, bounded Build edits/checkpoints, verification evidence and review export are implemented. ACP and MCP child processes are not sandboxed and run with local OS privileges. MCP server selection is per-request; every tool call requires separate confirmation. Skills are discovered under workspace `.agents/skills`, validated for required metadata and explicitly selected; skill instructions are read as text, while scripts/resources are not executed or automatically loaded.

The room feature set includes identity and presence, tasks/dependencies, chat, anchored comments, opt-in location following, selected tracked-Git diff sharing, host-local event history, reconnection/revocation, live text editing with consent and scoped document subscriptions, and the optional relay bridge. Studio only uses supported themes/settings/profile APIs. It does not inject styles into the workbench DOM.

## 2. Requirement and gap matrix

| Prompt area | Current evidence | Gap / next acceptance gate | Priority |
|---|---|---|---|
| Desktop IDE foundation | Code OSS fork and first-party extension registrations | Full upstream/product build and installer verification are outside this feature slice | P1 |
| Rooms, roles, presence, invite revoke | Team Lobby room protocol and room-service account/invite lifecycle tests | Run two separate IDE instances and a two-machine TLS relay acceptance; measure disconnect and revoke behavior | P0 |
| Session countdown and deadline controls | Team Lobby 0.97.0 has a confirmed 24-hour hackathon preset, custom timezone-aware deadline, host-only pause/resume, persisted remaining duration, reconnect synchronization and attributed transition history (protocol 17) | No authoritative managed-service clock, subdeadline/milestone scheduler, or urgency/escalation automation; verify clock drift and reconnect behavior in separate IDEs | P1 |
| Shared review context | Explicit tracked-Git diff sharing, line-anchored comments, audit/history events | Confirm user consent flow and navigation in separate IDEs; document patch size/privacy behavior | P0 |
| Shared editing / spectate | Opt-in editor-location follow and consented Yjs document sessions | Manual multi-window reconnect, conflict, path mapping and revoke testing | P0 |
| Live uncommitted change visibility | Team Lobby 0.98.0 adds explicit per-file live Git snapshots on save, a five-second update limit, a room-visible stop control, automatic stop on room leave/errors, and retained history of previously shared patches | One tracked file per participant; save-based snapshots, not remote unsaved-buffer streaming. Validate freshness, sensitive-file review and stop behavior across separate IDEs | P0 |
| Task graph / pipeline / agent dispatch | Task ownership, dependencies, deadlines, worktrees, verification evidence and task-linked Build activity | No durable scheduler, bounded pipeline runner, contract invalidation or automatic agent dispatch; add only with idempotency, isolated worktrees, gates and audit | P1 |
| AI core | Ask/Plan/Build, selected context receipts, project-root selection for multi-root workspaces (including file/selection/diagnostic/terminal context, Build worktrees, saved-session continuation, Plan-to-Build handoff and plan export), typed read-only workspace capabilities including bounded paths-only workspace inventory, separately approved editor navigation, per-file Git diffs, workspace diagnostics, symbol search, approved active debugger stack inspection, exact-consent terminal output sharing, and Build-only exact-match file edits with diff preview and standard conflict-checked rollback checkpoints, approval boundary, checkpoints, verification evidence, review export and Markdown Plan artifact | Run real supported providers, capture failure/cancel/retry evidence, and validate plans against actual room tasks | P0 |
| Agent adapters | Optional ACP v1 Ask/Plan; optional local MCP stdio servers for Ask/Plan with per-call confirmation and bounded tool output | Remote MCP/OAuth, MCP resources/prompts, ACP Build and process isolation are absent; perform real-agent acceptance and review process-isolation design | P1 |
| Skills | `.agents/skills` discovery, frontmatter checks and explicit per-request selection | No installer/catalog/signature lifecycle or isolated runtime; scripts remain inert | P1 |
| Theme / morph direction | Six Studio themes, temporary theme and project-profile previews, Keep/Restore, merge-safe profile import, allowlisted appearance profiles, opt-in motion, reduced-motion override, and core text contrast tests | Full prompt-driven morph beyond extension APIs needs bounded Code OSS workbench APIs; avoid DOM hacks | P2 |
| Voice | No call stack in current implementation | Signaling, WebRTC/SFU decision, consent, moderation, network and privacy prototype | P2 |
| Mobile | No mobile client | Separate responsive client, auth/session model, push/offline and approval threat model | P3 |
| Widgets, Packs, Workshop | Not shipped | Define manifest, sandbox, permissions, resource limits, preview, audit, uninstall and rollback before generated extensions | P3 |
| Managed service / hosted agents | Local self-hosted service prototype only | Separate threat model, operator controls, monitoring, retention, recovery, abuse handling and deployment design | P3 |

## 3. Architecture decisions

1. **Keep room service separate from the IDE extension.** Use HTTPS for account/room/invite control and WSS for protocol relay. Continue bridging only to a loopback room authority. Do not present the current prototype as a production managed service.
2. **Keep transport, room authorization and content storage distinct.** TLS protects transport; invite/role checks authorize room operations; content persistence and retention require separate product decisions. The relay currently does not persist room content.
3. **Treat ACP as an adapter protocol.** Negotiate the documented protocol version/capabilities and keep Forge grants outside ACP. A child process is not constrained by the clientÃ¢â‚¬â„¢s advertised methods; do not claim sandboxing.
4. **Keep AI capabilities typed, narrow and user-approved.** Context selection and tool grants remain separate. Every mutating action needs a reviewable diff or explicit confirmation plus bounded rollback/checkpoint evidence.
5. **Treat skills as untrusted instructions.** Validate the portable `SKILL.md` metadata, disclose selected files and sizes, load only selected instruction text, and never infer tool permission from skill content.
6. **Use supported IDE extension points for customization.** Themes, settings, views and webviews are supported. Full workbench morphing requires APIs owned by the Code OSS fork rather than brittle DOM injection.
7. **Defer widgets/Packs until isolation is real.** Define signed/owned manifests, explicit capabilities, resource budgets, CSP, preview and reliable rollback before accepting generated code.
8. **Keep voice, mobile, hosted agents and remote control as separate projects.** Each needs an explicit service boundary and its own threat, operational and acceptance plan.
9. **Treat MCP as an optional local stdio tools transport, not a sandbox or model provider.** Ask/Plan server selection is per request and each call is approved; selected executables retain ambient OS access. Keep ACP separate, and do not add remote MCP, persistent grants, script execution or ACP Build before auth and isolation decisions.

## 4. Skills sourcing report

The product repository has no bundled `.agents/skills` directory. The Team Lobby parser in `src/agentSkills.ts` accepts workspace skill files, checks frontmatter and name-to-directory match, requires a non-empty description and body, caps file size at 64 KB and returned body at 20 KB, and does not evaluate YAML or run scripts. The extension workflow makes selected skill instructions explicit and discloses paths and sizes before sending context. Installed assistant-side skills available in the development environment are not Forge product dependencies and are not silently imported into the IDE.

The Agent Skills specification was checked at [agentskills.io/specification](https://agentskills.io/specification). It defines a portable directory with required `SKILL.md`, YAML frontmatter, `name` and `description`, plus optional supporting files. Forge's parser covers the core metadata contract but is not a complete YAML implementation or a trust/signature system. Keep instructions untrusted and tools independently authorized.

## 5. Prior-art and standards research

- [ACP v1 overview](https://agentclientprotocol.com/protocol/v1/overview) describes the current stable client-agent session protocol; [ACP v2 overview](https://agentclientprotocol.com/protocol/v2/overview) is marked draft. ACP supplies interoperability, not OS isolation or Forge authorization.
- The [MCP specification](https://modelcontextprotocol.io/specification/2026-07-28), [official TypeScript SDK client guide](https://ts.sdk.modelcontextprotocol.io/v2/get-started/first-client.html), and [SDK v2 release line](https://github.com/modelcontextprotocol/typescript-sdk) guided the 0.89.0 local stdio client. The extension launches only user-configured and per-request selected servers; remote HTTP/OAuth, resources/prompts and OS isolation remain out of scope. No MCP servers are sent by the ACP client.
- [VS Code extension capabilities](https://code.visualstudio.com/api/extension-capabilities/overview), [workbench extension points](https://code.visualstudio.com/api/extension-capabilities/extending-workbench) and [webview UX guidance](https://code.visualstudio.com/api/ux-guidelines/webviews) support themes, views, commands and themeable webviews but do not provide workbench DOM access. This bounds Studio's safe customization surface.
- [Zed collaboration](https://zed.dev/docs/collaboration/overview) combines shared projects, channels and voice; its filesystem access model is a key trust consideration when comparing collaboration products.
- [Microsoft Live Share](https://learn.microsoft.com/en-us/visualstudio/liveshare/) documents collaboration, shared terminals and server forwarding; Microsoft's docs describe the product as maintenance mode, so treat it as established prior art rather than a fast-moving feature roadmap.

These sources inform architecture choices; they do not establish feature parity or validate Forge's implementation.

## 6. Milestones and exit criteria

### M0 Ã¢â‚¬â€ Room service bridge slice (implemented, automated)

Two authenticated participants connect through the relay bridge, complete the local protocol handshake, exchange room traffic, and are denied after invite revocation. Control-plane lifecycle and relay authorization are tested. **Evidence:** room-service 9/9 tests; roomServiceClient integration coverage and Team Lobby 130/130 tests in both mirrors. This proves the automated path only; separate IDE and two-machine acceptance remains open.

### M1 Ã¢â‚¬â€ Desktop collaboration acceptance

Automated coverage already exercises two local room clients exchanging presence/history/events, reconnecting with stable identity, receiving approved shared documents, delivering reviewed diff/comment artifacts, and rejecting revoked invites. This is protocol-level evidence, not full IDE acceptance. Run two independent IDE instances, first on one machine and then across two machines with trusted TLS. Verify identity, presence, comments, selected diff, task ownership, Yjs edit consent, follow, reconnect and revocation. Capture the user-visible flows and test revocation during an active session. Exit only when both clients show consistent state and the denied client cannot resume.

### M2 Ã¢â‚¬â€ AI runtime acceptance

Run Ask/Plan/Build against at least one real IDE provider and one ACP v1 agent. The local MCP SDK handshake/list/call/close path now has a protocol fixture test; add a real third-party MCP server acceptance. Verify selected context receipts, no context transfer without consent, native and MCP tool approval denial, cancellation, rejected ACP permissions, bounded diff/checkpoint, verification result, room-task linking and Plan artifact contents. Record provider/model/environment and failures.

### M3 Ã¢â‚¬â€ Agent and skill trust boundary

Keep the explicit per-request MCP server selection, visible per-call arguments and no-persistent-grant policy. A tool approval is not process containment: local MCP servers still have ambient user privileges. Before adding remote MCP, unattended grants, ACP Build, or skill script execution, decide and test an OS-level isolation boundary. Perform provider-backed adapter acceptance and strengthen server/tool provenance and display.

### M4 Ã¢â‚¬â€ UI and customization

Audit the current Studio and Team Lobby surfaces against accessibility and theme behavior. Implement the desired anime/custom themes through supported theme/profile mechanisms. Any full-shell morph API should be built into this Code OSS fork and documented as a stable extension contract.

### M5 Ã¢â‚¬â€ Voice and mobile feasibility

Prototype call setup and mobile room observation separately. Decide relay/SFU, auth, permissions, moderation, battery/offline behavior and failure recovery before committing to production feature scope.

### M6 Ã¢â‚¬â€ Widgets, Packs and Workshop

Only proceed after a verifiable sandbox and package supply-chain plan exists. Add component manifest validation, per-capability grants, data-source contracts, CSP/resource limits, preview, audit and rollback; then test malicious or malformed packages.

### V1 acceptance remaining from the attached expansion prompt

The code and protocol fixtures cover several required flows, but automated helper tests do not satisfy the prompt's user-visible exit criteria. The next acceptance pass is two separate IDE instances with independently pinned identities: invite/join, online presence, selected uncommitted diff, anchored comment, file/line follow, scoped Yjs edit request, disconnect/reconnect and invite revocation. Confirm no ignored/secret file disclosure, show when content is a selected patch versus a live shared buffer, and verify that revoked clients cannot read or write further. Then validate one real IDE provider and a third-party MCP server, and exercise voice feasibility before promising the V1 call exit condition.

## 7. Current implementation result

The room-service relay slice and local MCP Ask/Plan slice are implemented and automated. The service control plane and relay tests cover persisted account/room/invite lifecycle, audit and relay admission; Team Lobby tests cover the trusted-TLS WSS bridge and MCP handshake/list/call/close via a local fixture. Extension and service sources are present in both the handoff and Code OSS checkout. The 0.90.0 VSIX is built and registered as a built-in extension.

This is not completion of every V1-V3 vision in the prompt. In particular, room multi-machine acceptance, real-provider AI acceptance, voice, mobile, hosted agents, isolated widget runtime and Workshop remain outstanding. See [THREAT_MODEL.md](../architecture/THREAT_MODEL.md), [AI_CORE_FEATURE_RESEARCH.md](AI_CORE_FEATURE_RESEARCH.md) and [FORGE_AI_NATIVE_EXPANSION_RECONCILIATION.md](../planning/FORGE_AI_NATIVE_EXPANSION_RECONCILIATION.md) for detailed prior evidence and historical development notes.

**21 October 2026 update:** Team Lobby 0.86.0 now supports room-service account key rotation. The old credential is immediately invalidated; account-owned rooms/invites remain, active hosted sessions disconnect, an account-scoped audit event is added, and the new key is stored in SecretStorage with a copy recovery flow if secure storage fails. Automated acceptance is recorded in BUILD_STATUS.md.

**21 October 2026 update:** Room-service operators can recover a lost key for an account, including restoring disabled accounts using the admin API. Recovery is one-time-visible, hash-only at rest, audit logged, closes hosted relay rooms, and revokes outstanding invites when restoring a disabled account. See `POST /v1/admin/accounts/{id}/rotate-key` in the service README. This covers operator recovery only; user self-service remains absent.

**21 October 2026 update:** Team Lobby 0.87.0 adds an account-scoped room-service audit viewer backed by bounded cursor pagination, and the service admin API can recover disabled accounts while revoking their prior invites. Automated evidence: both Team Lobby copies pass 121/121 tests and both room-service copies pass 9/9 tests. Full two-IDE and separate-machine acceptance remains open.

**21 October 2026 update:** Team Lobby 0.88.0 revalidates every rollback target after confirmation and immediately before applying edits. The guard prevents a stale checkpoint from overwriting later changes and rejects paths that no longer resolve under the selected workspace. Regression coverage is in `extensions/team-lobby/test/rollbackGuard.test.js`.

**21 October 2026 update:** Team Lobby 0.90.0 adds optional local MCP stdio tools for Ask/Plan through `@modelcontextprotocol/client` 2.3.1, with explicit per-call approval and bounded schemas, inputs, results and call counts. Startup/discovery time out after 15 seconds; tool calls time out after two minutes and inherit turn cancellation. Each request closes its local MCP processes. This does not sandbox MCP servers; they retain ambient OS privileges. Both mirrors pass 130/130 tests, including stalled initialize/list/call cases. The nine-file VSIX hash is `09DD41F4932A2A17462FD203C668D249DF50711D8CC6154CCE9376805888E42E`. Real third-party MCP and provider-backed IDE validation remain open.

**21 October 2026 update:** Team Lobby 0.91.0 adds `git_diff` as a typed read-only capability for Ask, Plan and Build. It returns staged and unstaged tracked changes for one non-sensitive, workspace-relative file after the existing per-call approval; the approval explicitly states that patch text is sent to the selected provider. Binary and rename/copy patches, untracked files and unsaved editor buffers are omitted. The result is capped at 12,000 characters, while local evidence stores only path and result size. Both mirrors pass TypeScript checking/build and 137/137 tests. Provider-backed and interactive IDE acceptance remain open.

**21 October 2026 update:** Team Lobby 0.92.0 adds `diagnostics_list` as a typed read-only Ask/Plan/Build capability. It returns up to 100 diagnostics from the selected workspace or one file, filters paths outside the selected workspace and likely sensitive paths, sanitizes messages, and caps output at 12,000 characters. The approval prompt identifies the scope and discloses transfer of diagnostic messages to the selected provider. Local evidence retains paths and result size, never diagnostic text. Both mirrors pass TypeScript checks/build and 142/142 tests; provider-backed and interactive IDE acceptance remain open.

**21 October 2026 update:** Team Lobby 0.93.0 adds `symbols_find` as a typed read-only Ask/Plan/Build capability using the IDE's workspace symbol-provider API. The query goes to registered symbol providers; up to 100 non-sensitive names, kinds and locations may be returned to the selected model, with no source text read. Output is capped at 12,000 characters; audit/evidence omits query and symbol names. Both mirrors pass TypeScript checks/build and 147/147 tests. Provider-backed and interactive IDE acceptance remain open.

## Team Lobby 0.94.0 Ã¢â‚¬â€ approved workspace test task discovery

This is a V1 capability-registry increment from the attached AI-native expansion prompt. `tests_discover` exposes IDE Test-group task names and provider labels, filtered to the selected workspace, after the established read-only tool approval. It does not run task code or expose execution commands, arguments, environment values or output. Automated tests pass in the handoff and Code OSS extension copies. Live provider-backed and IDE runtime acceptance remain outstanding. Running tests is intentionally a separate execution capability and still requires exact-task approval, cancellation, bounded runtime and verified exit evidence.


## Team Lobby 0.95.0 â€” confirmed IDE test execution

Build can call `tests_run` for one registered IDE Test-group task in the selected workspace. The request has to name a task surfaced by `tests_discover`; Forge re-fetches current tasks and rejects stale names, then asks a separate modal confirmation naming the task and provider. Request-wide approval for read-only inspection never authorizes execution. Project task code runs with the user's OS privileges and may modify files or use the network. Output remains in the local task terminal; the model receives only the actual process exit code. Tasks without a process exit event report an unverified result. Cancellation and the fixed 10-minute timeout request task termination. Local evidence keeps only capability, outcome and result size.

Task execution is exposed only in Build, not Ask or Plan. Duplicate exact task names/providers require a user selection. Automated tests cover strict input registration, exact matching, mandatory per-call approval despite request-wide read-only approval, exit-status evidence and cancellation/timeout classification.

### Task/test capability research update â€” 21 October 2026

The official VS Code Extension API confirms that `tasks.executeTask` returns a `TaskExecution` that can be terminated, `onDidEndTaskProcess` provides the process exit code (which is undefined when terminated), and `onDidEndTaskProcess` does not fire for tasks without an underlying process. Forge therefore waits for process-end evidence, treats an IDE task-end without process evidence as unverified, keeps task output in the IDE terminal, and reports cancellation/timeout as a termination request rather than claiming the process is definitely stopped. Test execution is an OS-privileged project-code boundary and has its own exact-task modal approval. Sources: [VS Code Extension API: Tasks](https://code.visualstudio.com/api/references/vscode-api#tasks), [Task Provider guide](https://code.visualstudio.com/api/extension-guides/task-provider), checked 21 October 2026.
## Team Lobby 0.99.0 implementation update

Multi-root Ask/Plan/Build requests now select one workspace folder and scope file discovery/resolution, editor selection, diagnostics, Build task-worktree checks and creation, and saved Build resume validation to that folder. The shared relative-path helper rejects outside paths. Both extension copies pass TypeScript checking and 157/157 tests; the VSIX and Code OSS built-in extension manifest are synchronized. Separate multi-root IDE/provider validation remains outstanding.

The attached expansion prompt's first mandatory collaboration slice is represented in automated two-client tests for presence, confirmed uncommitted diff, anchored comments, reconnect, revoke and activity-history checks. Automated transport coverage is not equivalent to the prompt's separate-IDE/two-machine acceptance scenario; that gate remains P0. Voice, mobile, hosted agents, remote control, prompt-driven full-workbench Morph, widgets, Workshop and Packs remain unimplemented or exploratory as stated in the matrix.

Studio 0.11.0 moves decorative Studio animation and transition movement to an explicit off-by-default preference. The user toggle persists through webview reload; prefers-reduced-motion remains authoritative. Both mirrors pass 6/6 tests. Live keyboard, screen-reader, and interactive theme readability checks remain open.


**21 October 2026 update:** Team Lobby 0.105.0 adds controlled exact-match file edits in Build. Forge previews a full-file diff and requires separate consent, verifies the document has not changed, then applies an unsaved IDE edit. Both extension copies pass TypeScript checks and 174/174 tests. The 17-file VSIX SHA-256 is ``838583216E620109596DEF720D218B87E551DF6D360A6751EF1D6E6AED346E41`` and matches the product manifest and handoff package copies. The registered tool closes the narrow V1 patch-application capability; a generalized patch engine and interactive/provider validation are still absent.


**21 October 2026 update:** Team Lobby 0.106.0 integrates approved exact-match Build edits with the existing checkpoint and rollback system. It records the before/after text before applying the edit, links checkpoint identity to task activity, and reuses the rollback conflict guard. Both extension copies pass TypeScript checks and 175/175 tests. The 17-file VSIX SHA-256 is `D7896744609ED694E7E9A199C6D1E2EAF5B7C3BB900D00C6DA0B2CC38E630AF2` and matches the package copies and built-in manifest. Live IDE interaction and provider acceptance remain open.


**21 October 2026 update:** Team Lobby 0.107.0 adds the workspace_list read-only capability from the prompt's native workspace/list catalog. It returns at most 250 relative file paths for the selected file-based root, omits sensitive paths and common generated folders, checks canonical workspace containment, and returns no file contents. The operation uses the existing typed capability approval and content-free evidence pipeline. Both copies pass TypeScript checks and 178/178 tests; provider-backed runtime remains unverified.


**21 October 2026 update:** Team Lobby 0.108.0 adds editor_open from the native editor/open capability catalog. The tool accepts a validated selected-root path and optional one-based line, rejects sensitive files and out-of-range locations, opens the editor only after separate mandatory confirmation, and returns no source contents to the model. Both copies pass TypeScript checks and 179/179 tests. The packaged release is synchronized; real-provider and interactive IDE acceptance remain open.
