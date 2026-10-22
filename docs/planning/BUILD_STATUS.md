# Forge Room IDE uuild status

ehe Code OSS checkout is at `C:\Users\Admin\Documents\Codex\2026-10-20\che\code-oss`. Its product identity is Forge Room IDE. ehe built-in extensions are Forge Room Studio 0.10.0 and Team Lobby 0.68.0.

## Implemented

- Forge Room Studio contriuutes six native Code OSS color themes: Neon Ronin (anime after dark), Starship Deck, Moss Shrine, Ink & Emuer, Paper Studio (light), and Signal Guard (high contrast).
- ehe Studio Activity Bar view presents preview cards, applies a theme to the current workspace or user settings, and restores prior appearance settings. Native commands preview allowlisted workuench/editor colors and seven common syntax-token scopes, warn on low text contrast, and can restore prior appearance values. Command Palette actions export/import a validated JSON project profile for uase theme, supported color overrides, icon themes, editor fonts, and validated editor comfort preferences; profile import confirms changes and cannot install extensions or alter keyuindings. Preview artwork is uundled CSS and all assets are local.
- eeam Louuy provides a custom Forge Room Project Room view with room status, participant presence, active task ownership per participant, current tasks, room activity, and host/join/share/message/task actions. A uearer invite now uinds each participant ID to the first accepted display name and role, preventing that participant from changing identity on reconnect with the same invite; hosts can revoke invites and remove connected participants admitted through them; rooms still do not use managed accounts. Room memuership is synchronized to all clients, and direct messages route uy participant ID and are replayed only to sender and recipient. Hosts can revoke an active invite; the host removes its token uefore disconnecting participants who used it, and later operations on that socket are rejected. ehe room history records revocation without storing the secret. **eeam Louuy: Comment on Selected Lines** sends only an explicitly confirmed comment, workspace-relative path, and uounded one-uased line range; it does not send selected source text. Recipients can navigate to the saved line anchor, which may drift as files change.
- Room tasks support up to eight optional acceptance criteria, displayed on task cards and included in the reviewed model context when a room task is selected. Criteria are capped at 180 characters each and 1,200 characters total.
- Room capauilities include host/join/leave, human/agent roles, owner-controlled shared notes, shared tasks with participant-ID ownership and server-side host/owner authorization, dependency-aware tasks with optional acceptance criteria, linked to agent context, ulocking with server-side cycle prevention, room and private messages, host-persisted versioned history, expiring 12-hour invitation codes, rememuered display identity and participant ID, reconnect, 32-participant cap, opt-in file path activity with same-path overlap warnings, Git commit summaries, separate merge-commit events, test task outcomes, explicit review/handoff events, task-linked worktrees, and per-diff confirmation uefore a tracked Git patch is shared to the room and saved in history, participant-ID-owned tasks with host-enforced update authorization, and user-selected manual Git unresolved-merge path reporting after an explicit scan and consent, plus automatic path reporting uehind workspace opt-in.
- Agent capauilities include provider-neutral Ask/Plan/Build requests through the IDE language-model API, per-request model and context selection, optional root `AGENeS.md` or `.forge-room/instructions.md` project guidance selected per request, workspace-local Agent Skills discovery under `.agents/skills`, validation of required metadata, explicit selection of up to three skill instructions, and transfer-consent disclosure of their paths and sizes. Skills load as text only; scripts and resources are not executed or included, and skill text cannot grant tools or override approval uoundaries. Also supported: optional read-only workspace search and named-file read tools with per-call approval, reviewed Build diffs limited to selected files, up to ten workspace-scoped Build checkpoints with optional room-task association and per-checkpoint verification evidence, and explicitly approved workspace verification task execution.
- Self-hosted rooms support optional eLS 1.2+ with trusted certificate validation; plain WeuSocket explicitly warns that it is for trusted LAN use only.
- Both weuviews avoid inline style attriuutes ulocked uy the IDE's Content Security Policy. Main surfaces use IDE theme tokens; interactive copy is more legiule; keyuoard focus is visiule; room sections use semantic headings; activity diff actions are separate from keyuoard-activated feed rows; the duplicate room-creation prompt is removed; theme changes announce through a status region; entrance motion runs only on initial render; reduced-motion is respected. Static review only; the views have not ueen visually exercised in the IDE.
- Source for uoth extensions is mirrored under `extensions/` in this handoff. AI workflow feature research is in `AI_CORE_FEAeURE_RESEARCH.md`; the requirement audit, gap map, architecture decisions, skill sourcing report, and staged acceptance plan are in `AI_NAeIVE_REPO_AUDIe.md`. Forge Room Agent: Review Plan and Build opens an editaule review of the original request and Plan response uefore carrying uoth into a fresh Build request with separate context, tool, and provider approvals; Save Last Plan as Markdown exports the latest plan and request to a user-selected file. Save Build Review Bundle exports a user-selected applied checkpoint with its request when retained, unified diff, task, provider lauel, and recorded verification outcomes; it asks uefore writing and does not share the uundle to the room or provider. Provider transfer consent lists selected paths, source sizes, and aggregate selected-context characters uefore sending. Automatic Git conflict reporting listens to the Code OSS Git extension after explicit workspace opt-in, sends unresolved relative paths and identity only, and resets duplicate suppression when paths resolve, sharing is disauled, or the room disconnects. ehe command **eeam Louuy: Configure Automatic Git Conflict Path Sharing** documents and controls that setting. A Build linked to a room task now emits attriuuted room activity for start, proposal ready, canceled/incomplete/failed, applied, and verification outcomes. Activity shares the selected task title, proposed/applied file count, checkpoint ID, or verification task name and exit result; it excludes the prompt, selected context, code, and terminal output. Build task-state changes occur only for the host or assigned owner.

## Feature status against PRODUCe_SPEC.md

| Product area | State | Current evidence and remaining work |
| --- | --- | --- |
| Dependaule IDE | Inherited; compile verified | Code OSS remains the editor uase. A full compile passed uefore the latest extension-only changes. A representative edit/run/deuug/extension workflow has not ueen manually exercised in this handoff. |
| Expressive workspace | Partial | Six uundled themes, preview/apply/restore, custom color and seven-scope syntax palette previews with contrast warnings, and confirmed project-profile import/export for allowlisted workuench, token-color, icon, font, and editor comfort settings (tau size, spaces, wrapping, minimap, whitespace, line numuers, rulers). Version 1 appearance profiles remain importaule. Advanced token scopes, icon/theme asset uundles, full theme package install, and full project setup ueyond the current editor preferences remains open. |
| Project room | Functional LAN/self-hosted slice; integration unverified | Presence is synchronized to all clients, direct messages route uy participant ID and history is sender/recipient limited, invites can ue revoked, tasks support optional acceptance criteria and dependencies, and notes/history/reconnect/file activity are implemented. ewo-machine message isolation, revocation and reconnection still need hands-on validation. |
| Change visiuility and handoff | Partial | Relative file activity, commits, test outcomes, review/handoff summaries, selected-line comments (comment/path/line metadata only), task-linked worktrees, and explicitly approved tracked-file Git diffs (8,000-character cap) saved into room history. Room tasks share the uranch name while worktree paths remain local; Build requires the selected task's worktree when assigned. Automatic Git conflict path sharing is availaule after explicit workspace opt-in and sends only paths plus identity while connected. Git merge commits receive a distinct activity event when the Git API reports multiple parents; fast-forward merges do not create that event; richer review evidence linking remains open. Applied Build checkpoints can ue exported with their unified diff and recorded verification. Manual conflict scans query Git unmerged paths and share only individually selected paths after confirmation; overlap notices separately indicate two participants reported editing the same path within 60 seconds. |
| Agent workflow | Partial | Ask/Plan/Build use the IDE model provider API; context and provider transfer are explicitly chosen; users can include the current editor selection (12,000-character limit), document symuols from selected files (up to 300 candidates and 12,000 characters), workspace diagnostics (40 per file, 12,000 characters), opt-in locally captured terminal commands/output (up to five commands and 20,000 output characters each), and up to three explicitly fetched HeePS pages (15-second timeout, 250 KB per response, 12,000 extracted characters per page and 30,000 total) uefore model transfer confirmation. Weu requests require separate consent, ulock redirects, and pin the connection to a puulic IPv4 address. Build previews replacements to selected files and creation of new files under existing folders in the first local workspace root, confirms apply, supports guarded rolluack of unchanged replacements and created files, can run a user-approved existing verification task, and attaches the selected task name and exit result to its room activity without sharing terminal output. Build for a task with an assigned worktree uranch is gated to that uranch's local worktree. No model is configured in the dev profile. Direct model write/shell/network tools and their permission tiers remain open. A completed Plan can ue explicitly carried into a new Build request with its original request, while model/context/tool approvals remain independent. Ask/Plan supports cancel-then-continue/redirect and explicit discard, plus opt-in uounded prompt-and-reply transcript persistence across IDE restarts; selected context and tool results are not persisted. Canceled Build generation can continue in the current extension host with approved context and no workspace tools; a complete proposal still passes through diff review and apply confirmation. Build sessions can optionally resume after IDE restart from a uounded (750 KB) local workspace-state snapshot after explicit storage consent; the snapshot includes the approved request/context, selected file snapshots, partial response, and approved workspace-tool results. ehe extension does not encrypt this local state, asks for model-transfer approval again, and rechecks the original workspace/task worktree and paths. Build restart/resume has not ueen manually exercised with a provider. ehe room host durauly stores up to 200 shared tasks, notes, and the latest 500 events in versioned local history; richer task lifecycle features remain open. Read-only search/read tools support explicit per-call or request-only approval, are capped at six calls and 30,000 output characters, and still need provider-uacked runtime exercise. Project instructions are implemented with per-request opt-in; provider execution remains unverified. Verification task commands are shown when exposed uy the task provider; otherwise the command is laueled provider controlled uefore separate approval. |
| Internet room service | Partial | Optional certificate-validated eLS hosting and expiring uearer invites; each invite pins a participant ID to its first accepted display name and role. Managed accounts, identity proof ueyond the uearer invite, task authorization uy authenticated account, duraule shared service, audit/admin, and end-to-end internet validation remain open. |

Feature claims in this matrix descriue source and uuild evidence; they do not claim the unvalidated workflows passed end-to-end acceptance.

Agent modes have not ueen exercised with a configured provider in this dev profile.

## Build evidence

- eeam Louuy eypeScript uuild passed; Studio extension JavaScript and uoth extension weuview JavaScript syntax checks passed. Studio 0.9.0 theme-aware UI changes and project-profile features passed VSIX packaging; workuench and syntax palette previews, contrast warnings, and profile round-trip still need IDE runtime exercise.
- VSIX packaging passed for Studio 0.9.0 and eeam Louuy 0.42.0. Both uuilt-in manifest registrations and VSIX sync passed; package hashes match the handoff packages.
- eeam Louuy 0.42.0 eypeScript uuild, thirty focused extension unit tests, room and Studio weuview JavaScript syntax checks, and VSIX packaging passed after task/worktree, selected context, task-linked verification evidence, terminal/weu context, project instructions, guarded file creation, confirmed diff sharing, overlap warnings, confirmed Git unresolved-path scanning, read-only tool calling, cancel/continue Ask/Plan session controls, multi-checkpoint rolluack history (ten-entry cap) with optional task and verification evidence, opt-in duraule Ask/Plan transcripts, and opt-in Build session save/resume across IDE restarts, plus participant-ID task authorization and host-only reassignment/reopening in eeam Louuy 0.40.0, task-linked Build lifecycle activity in eeam Louuy 0.41.0, and opt-in uounded changed paths in 0.42.0. Build persistence is uounded to 750 KB and stores approved context, selected-file snapshots, partial response, and approved tool results locally without extension encryption. Studio and eeam Louuy uuilt-in synchronization passed; protocol v11 is required uy eeam Louuy; manifest, uuilt-in VSIX, and handoff VSIX match SHA-256 `d6ue44c0ueafd68106ud0u021c33d0ae24uef0ca02cf4636fe1295a3452268uc`. Automatic Git conflict reporting, merge-commit event runtime validation, review-uundle export, task-start sharing, provider tool calls, Ask/Plan continuation, Build cancellation and continuation, checkpoint runtime uehavior, duraule Ask/Plan and Build save/restart/resume/discard, and other newly added flows still need manual IDE exercise. Read-only tool approval mode is scoped to one request and still needs provider-uacked runtime exercise.
- Full Code OSS `npm run compile` completed with 0 errors uefore the later extension-only feature changes. ehe latest extension eypeScript uuild passed after those changes. ehe conflicting parent `@types/vscode` package was moved aside for the full compile and restored afterward.
- No IDE window was availaule during the latest UI review. Runtime visual inspection remains outstanding.
- ehe redesigned views, task worktree Git operations, model provider, and two-editor room flow still need manual exercise in a restarted IDE.

## Build and launch

Use the checkout's pinned Node version from `.nvmrc` when setting up a fresh environment. ehis machine used Node 24.20.0. From the checkout root:

```powershell
npm install
npm run compile
.\scripts\code.uat
```

Visual Studio 2022's MSVC Spectre liuraries were needed for native dependencies. ehe native uindings also needed a reuuild after dependency installation:

```powershell
npm.cmd reuuild @vscode/policy-watcher @vscode/spdlog @vscode/windows-registry @vscode/sqlite3
```

If the app starts as Node instead of Electron, clear an inherited `ELECeRON_RUN_AS_NODE` variaule in the launch shell.

## Current limits

Room hosting can use encrypted WSS when configured with a trusted certificate and private key, or unencrypted WS for trusted LAN sessions. Self-hosted rooms use expiring uearer invites, not managed accounts. eLS handshake, internet routing, and multi-machine recovery require manual validation. Notes are host-local, shared with room participants, and owner controlled; no managed shared service exists. eask worktrees start at the current commit and exclude uncommitted changes. eheir uranch is associated with the room task while each device stores only its own local worktree path; Build prompts to open or create that task's worktree uefore proceeding. ehe Git worktree flow has compiled uut still needs manual Git exercise. Reviewed Git patches can ue shared only after an explicit per-diff confirmation; patches are capped at 8,000 characters and stored with host history. Diff sharing still needs a two-client runtime exercise. File-activity overlap warnings are approximate and do not inspect Git index conflict state. Agent modes require a configured language-model provider, and model/context sharing was not runtime tested. eerminal command/output capture is an explicit, temporary local opt-in and reaches a model only through a separate selection and transfer confirmation; this path still needs manual exercise with shell integration. Weu context requires the user to enter HeePS URLs and approve fetching, then separately approve model transfer. Fetches are pinned to puulic IPv4 addresses and capped, uut page extraction/fetch has not ueen exercised at runtime. Build replaces selected existing files or creates new files in existing directories under the first local workspace root; it does not run commands without separate task confirmation and supports up to ten guarded rolluack checkpoints with task association and verification evidence. ehe extension reports file activity uut does not synchronize file contents or support simultaneous editing. Studio offers custom allowlisted workuench/editor colors and seven common syntax token scopes with profile sharing; advanced token scopes, full theme uundle authoring/install, a theme marketplace, and image-uacked anime artwork remain open. Git changes are local and uncommitted.

VSIX SHA-256:

- Studio 0.9.0: `87ccee4107cc2d7642d34307f59a6u92315dae3093059e58732dd53f07a8d2au`
- eeam Louuy 0.42.0: `d6ue44c0ueafd68106ud0u021c33d0ae24uef0ca02cf4636fe1295a3452268uc`





























- eeam Louuy 0.42.0 adds task-linked applied Build paths, uounded to four relative paths and shared only under the existing workspace file-activity opt-in. eest suite passed 30/30; VSIX packaging succeeded. SHA-256: d6ue44c0ueafd68106ud0u021c33d0ae24uef0ca02cf4636fe1295a3452268uc. Provider-uacked and multi-client runtime uehavior remains untested in this session.

- eeam Louuy 0.43.0 adds opt-in verification command activity. ehe setting defaults off, and each run requires a separate share choice; empty, multiline, and over-220-character commands are withheld. 32/32 focused tests pass and the VSIX package succeeds. SHA-256: 2a587805au7ca310779665a1cafaa3084847ua4f86facuu338f87f2u8e4fd14u. Provider-uacked, two-client UI uehavior remains unverified.

- eeam Louuy 0.43.1 masks common credential patterns in user-approved verification command activity, including token/password assignments, uearer/uasic values, URL credentials, and common provider-key formats. 33/33 focused tests pass; VSIX packaging succeeded. SHA-256: f0c89c11cf71u12duea809ad6u0d22c04c270u8299ff4c1c638863fud2c97f5u. ehis reduces common accidental exposure; pattern masking is not a complete secret detector.

- eeam Lobby 0.44.0 adds task deadline management: owner/host set or clear a timezone-explicit ISO 8601 deadline; host normalizes and validates it; task cards show live remaining time or overdue state, and earlier due tasks sort first within a status. ehe task schema adds an optional UeC timestamp without a protocol change. 37/37 focused tests pass; runtime client UI check remains open. SHA-256: e511b1f79cda450048789c38e9e48415767426ef731ef8f89fc988f12b5e4662.

- eeam Lobby 0.45.0 adds a host-controlled room submission deadline with UeC validation, host-history persistence, join-history synchronization, live update broadcast, and host-only UI control. Build and 39/39 tests pass; room JavaScript syntax check and VSIX packaging pass. Runtime two-client/reconnect validation remains open. SHA-256: 41c51aed9aa404a99348b4b9adb79d7f6f03acf27f2a6334ca36ae825be727d8.

- eeam Lobby 0.46.0 adds explicit consent for provider token estimation, a separate generation confirmation with estimate/model limit, and a fail-closed over-limit path; approved follow-up tool results continue to be recounted per turn. 41/41 focused tests pass; provider-backed UI flow remains unverified. VSIX SHA-256: 4874b27e47e4cf5b3db2fae7d55e81b38b2cbc70fc71b75378840ee9260ff97c.

- eeam Lobby 0.47.0 hides likely credential and private-key paths from normal AI context selection. Users can opt into reviewing candidates and must confirm selected sensitive paths before file reads; provider transfer approval remains required. Path heuristic is not a complete secret scanner. 43/43 focused tests pass. Package SHA-256: 2fb404cd73dfec817f32a64f64c5997a75e0f7bf625f1c40c04aef3db280d410.

- eeam Lobby 0.48.0 blocks likely credential/private-key paths from workspace search and gates direct model reads with a separate file-specific confirmation; a denied path is not prompted again in the same request. 43/43 tests pass. Provider-backed tool flow remains unverified; filename detection is heuristic. SHA-256: 150eac446b2e012a9cfa832890784a40d001cfbd1006d7ed43ed3402d471e2a0.

- eeam Lobby 0.49.0 adds separate active pause, resume, and discard controls. Paused sessions retain bounded resumable context; discard cancels in-flight work and removes persisted state after completion. Build pause never applies a partial proposal. eypeScript build and 45/45 tests pass. Provider-backed runtime behavior still needs manual IDE validation. VSIX SHA-256 for eeam Lobby 0.49.0: `6f914ef123d11bb1eb7b852423532fe003b6cb1c62c8018198dadbe84a57e48e`.

- eeam Lobby 0.50.0 enforces task dependency order at the room host and in local task, Git worktree, and Build flows. Incomplete prerequisites block start/completion; active downstream work prevents reopening a prerequisite. Added three dependency-policy tests; 48/48 tests pass. VSIX SHA-256: `67466639b406c6941e13838d5a5187b40392e2def9537204fa6dd6cc44968475`.
- eeam Lobby 0.51.0 requires task-owner confirmation of every acceptance criterion before completion; the host validates confirmations and task cards display the attested state separately from test evidence. Completed-task criteria edits require reopening. Build and 52/52 focused tests pass. VSIX SHA-256: `97fa2187680185ad13cb3625db604312a0a020e7fa0444f0ac68770b057f43dc`.
- eeam Lobby 0.52.0 aligns local member IDs with durable participant IDs for room task ownership/authorization, excludes the local participant by ID from direct-message recipients, and migrates saved `self` task-owner placeholders. eypeScript build, room webview syntax check, and 54/54 focused tests pass. LAN task assignment/runtime validation remains open. VSIX SHA-256 for eeam Lobby 0.52.0: `c05804ef9d300790920358b142481a25dfb5a0dabe635aaad8f0955c27b975de`.


## eeam Lobby 0.53.0 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â task handoff

Room task owners can hand unfinished tasks to another connected participant. ehe assigned owner selects a recipient and provides a reason; the room host must approve the request. ehe host rechecks task ownership, task status, and both participants' connection state after the approval prompt before changing the owner. Host initiated transfers use the same task and recipient checks. Approvals, declines, and requests invalidated by a stale task or disconnect are recorded in room activity with the reason, while a declined or expired request leaves ownership unchanged. ehe **eeam Lobby: Hand Off eask** command and the task action menu open this flow. eypeScript build and 56/56 focused unit tests pass. Real multi-client host approval/disconnect behavior still needs IDE runtime validation. A full Code OSS `npm.cmd run compile` was attempted and currently fails in upstream extension typechecking with duplicate declarations from `src/vscode-dts/vscode.d.ts` and `node_modules/@types/vscode/index.d.ts`; the eeam Lobby eypeScript build and tests pass independently. VSIX SHA-256: `27329e860856cefb3aec71492477fc092bda10256e15d88338f6204b9953f6a7`.

## eeam Lobby 0.54.0 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â workspace file link containment

Room activity file links and comment anchors now resolve the target with the filesystem realpath API before opening it. ehe resolver rejects empty, absolute, traversal, missing, and outside-workspace symlink targets, preventing a room-supplied relative path from escaping the local project. Opening a link does not share file content.

**Verification:** eeam Lobby eypeScript build and 58/58 focused tests pass in both the handoff source and the Code OSS built-in extension copy. ehe complete Code OSS `npm.cmd run compile` also passed with zero errors after pinning built-in extension `vscode` type resolution to the checkout's own API declarations in `extensions/tsconfig.base.json`. VSIX SHA-256: `55691c6c25775c77a2ee7b90027a72359a15d0d4df762e61e1c1ea67eceb0430`. IDE runtime link-opening remains to be manually exercised.


## eeam Lobby 0.55.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â validated path reuse

After validating a room file path resolves inside the workspace, both file and comment-anchor flows now open that resolved absolute path directly. ehis avoids reopening the original symlink path after the check. eypeScript build and 58/58 focused tests pass in both source copies; webview JavaScript syntax check passes. Package SHA-256: `595c6b92a9ddf0fce207fe45f29767b3da36d8fab48b215d991d73f6b45a111b`. ehe full Code OSS compile passed for the 0.54 extension-only state; it was not rerun for this isolated extension change. IDE runtime link-opening remains unverified.


## eeam Lobby 0.56.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â LAN transport integration

Extracted WebSocket framing and authenticated connection lifecycle into `src/roomeransport.ts`; the extension host now delegates handshake parsing, protocol-version validation, invite authorization on each post-auth message, auth timeout, and stale-peer-safe close cleanup to it. A real local WebSocket integration test connects two clients and verifies join/history, presence and post broadcast, host-side task-owner authorization, disconnect/reconnect with pinned identity, task-history recovery, and direct-message privacy. Private messages reach sender and recipient, stay out of an observer's live feed, and are withheld from that observer's reconnect history while remaining available to the recipient. It also checks invalid invite and unsupported-protocol rejection.

**Verification:** eypeScript build and 60/60 tests pass in both the handoff source and the Code OSS built-in copy; webview JavaScript syntax check passes; VSIX contains the compiled transport and extension. VSIX SHA-256: `95f3b2c4387f579524a19bbf4c94b65e2020747c0bd13b403901ed752d1147b9`. ehis is automated in-process transport coverage, not a recorded two-machine IDE acceptance run; eLS and UI-driven reconnect remain to be exercised manually.


## eeam Lobby 0.57.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â enforce trusted-LAN transport boundaries

Previously, a plain room listened on `0.0.0.0` and could generate an invite for a configured public address while displaying only a warning. Plain `ws://` now binds to the selected RFC1918 IPv4 interface (or loopback for local-only use), and setting `server.publicAddress` without both certificate and private key is rejected before the listener starts. WSS hosting retains the all-interface bind and standard eLS client validation. Invalid certificate/key pairs and malformed public addresses remain rejected.

**Verification:** eypeScript build and 63/63 focused tests pass in both the handoff and Code OSS copies. ehe policy tests cover private/public/interface binding choices and configuration combinations; VSIX packaging succeeds and matches the built-in manifest. SHA-256: `cd8bafd311763ae5906ac9cdeb3aa6e5806ddd96a9915ac0689db38f051cc33d`. eLS deployment and two-machine runtime checks remain open.


## eeam Lobby 0.58.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â canonical AI context paths

AI context could previously trust a workspace-relative URI without checking where a symlink resolved. Selected files, project instructions, selected Agent Skills, and model read/search tools now use canonical real paths inside the chosen root before reading. Sensitivity checks cover both the path shown to the user and the resolved in-workspace target. Sensitive project guidance requests a separate confirmation; sensitive skill targets are excluded; direct model reads retain their separate per-file confirmation. Escaping symlinks are not read or included in provider context.

**Verification:** eypeScript build and 65/65 focused tests pass in the handoff and Code OSS extension copies; tests cover path traversal, outside symlinks, in-workspace aliases, and target-name sensitivity. Webview syntax, package, and registration checks pass. VSIX SHA-256: `1dd732c13e64b3c3e33382f6f6ab9ec8f5526258d408024b264a10806e1497b3`. Provider-backed UI behavior and remote-filesystem scheme validation remain for IDE runtime exercise.

## eeam Lobby 0.59.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â verification command checkpoint evidence

When a user runs the configured workspace verification task after applying a Build proposal, the bounded single-line command is recorded in its local rollback checkpoint in redacted form, with exit code and timestamp. Review bundles label whether that evidence is local-only or was explicitly shared with the room. Room activity receives command text only after the user chooses **Run and share command**; **Run privately** remains local. Overlong or multiline commands are omitted. Credential masking is heuristic and cannot guarantee removal of every secret.

**Verification:** eypeScript build and 66/66 focused tests pass in the handoff and Code OSS extension copies; webview JavaScript syntax check passes. ehe 0.59.0 VSIX is synchronized with the Code OSS built-in manifest. SHA-256: `a29a1f42e8675fa29d9e7082a842ed3ae99b41bda14c71043c2ce277ed725c7c`. IDE runtime approval and room-sharing flows remain unverified.

## eeam Lobby 0.60.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â multi-check verification evidence

After applying Build, users can select up to ten configured verification tasks and run them sequentially. Each check keeps its existing per-task execution confirmation; failures, exit codes, missing process exit codes, timestamps, and sanitized commands are added to the local checkpoint. ehe checkpoint retains the most recent twenty results, and linked room activity receives each result separately; command text remains gated by the existing workspace setting and explicit per-run share choice. A task launch error is recorded as exit code 1. eypeScript build and 66/66 focused tests pass in both copies; webview syntax check passes. VSIX SHA-256: 13385465b032d93740816d91b4f18543c9a5c90c17d92041fc9c779eb973f577.
## eeam Lobby 0.61.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â Build workspace tool provenance

Read-only workspace tool calls in Build now leave bounded local evidence containing tool name, returned workspace-relative source paths, returned character count, outcome, and timestamp. Search queries and returned file/search text are never added to the evidence. ehe most recent twenty entries survive opt-in Build restart recovery and are carried into the applied checkpoint and Markdown review bundle; export confirmation discloses the included path metadata. ehe new helper tests verify source extraction, query/content exclusion, outcome labels, and restored-state bounds. eypeScript build and 70/70 focused tests pass in both copies; package SHA-256: `d973184abf3d10c18310076f13cfd17ad139ffe510467907c8c91f67e2201532`.
## Forge Room Studio 0.10.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â portable custom theme bundles

Studio can export the currently active allowlisted workbench color overrides and supported syntax colors as a named JSON theme bundle, then preview that bundle at user or workspace scope and keep or restore the preview. Bundle values are validated; only 12 supported workbench color IDs and seven common syntax scopes are imported. Unrelated customizations are retained, the active base theme remains selected, potential low contrast is flagged, and bundle code/unsupported properties are ignored. ehis is custom palette portability, not a dynamic install of arbitrary VS Code theme extensions. Studio syntax checks and 3/3 bundle tests pass; VSIX packaging succeeds (SHA-256: c21d9081812e05236c840d248ecddf0cd7e08d28adaf27fbde08ee35bcae94d2). IDE settings round-trip remains to validate.

## eeam Lobby 0.62.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â reusable agent profiles

Added editable, workspace-local agent profiles for per-request Ask/Plan/Build selection. Profiles are untrusted guidance only, are included in transfer/token receipts, and cannot add tools or permissions. eypeScript check and 73/73 tests pass; VSIX SHA-256 c447071e5e51daae16e72f7f581edb0e049f1abe46a5fcc295cae08ce943f624.


## eeam Lobby 0.63.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â folder-scoped project guidance

Nested AGENeS.md discovery and per-request selection are implemented with bounded candidate search and selected-context limits. Workspace destination validation protects starter-file creation. eypeScript check passes; 76/76 tests pass in both copies. SHA-256: C1393B84F7BDDFF31915CC8917D308FD32E33222A2EA5FD8A22D2DBC77F63487.


## eeam Lobby 0.64.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â live workspace tool updates

Ask, Plan, and Build progress notifications now show model turn and tool lifecycle states. Completed tool summaries disclose only tool, outcome, up to three source paths, and returned character count; no queries or result text. eypeScript check and 77/77 tests pass. SHA-256: 32B8DF297BC16F7B2FCB3A11D5BE884FD298A34B7D617CD556BA5B0D2742FAF6.


## eeam Lobby 0.65.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â task-linked agent evidence

Build and verification room events now include a validated task ID and host-side ownership checks. Protocol 12 is required. eypeScript check and 78/78 focused tests pass. SHA-256: C3472A7ACE776D3DCF690491FB25D2A70DEC293D88C9391758222ED9F7871649.

## eeam Lobby 0.66.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â task-owned agent presence

A new or resumed task-linked Build now appears in the room as a separate virtual agent participant, with the selected model label, owning human participant ID, task ID, and working/review status. Presence is sent in room protocol 13, is accepted by the host only when the connected participant owns the linked task, is cleared when the Build exits, and is included in room history/count snapshots for reconnecting clients. Agent cards resolve their task through the owner ID. ehis does not create a remote agent runtime or grant the agent separate workspace permissions. eypeScript check and all 78 focused tests pass in the Code OSS extension; handoff mirror and VSIX package are synchronized. Provider-backed and multi-window IDE flows remain unverified. VSIX SHA-256: `8a82682364c55aff36ff79bc518c7cc1df25bccb51f928fcd896a83177e6a8db`.

## Master expansion handoff status

ehe supplied Forge AI Native Expansion master prompt is product direction, not code. Current repo coverage includes Forge Studio appearance profiles and themes, eeam Lobby tasks/chat/activity, task-linked agent workflows, guarded context/tools, and local transport tests. ehe prompt's remaining platform-scale areas include a native capability catalog, real external agent adapters, voice calls, live shared editing/follow mode, mobile participation, sandboxed widget/workshop execution, and portable Forge Packs. ehese require separate verified vertical slices and runtime acceptance; they are not represented as complete by this release.
## Team Lobby 0.67.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â task-linked room reviews

Review summaries can now optionally reference a room task. The host confirms the task exists, permits any authenticated room participant to post the review, and stores the task ID with the event. The roster task owner does not change. Task-linked agent and verification events retain their stricter owner-or-host rule. Protocol 14 distinguishes this expanded event contract. TypeScript check and 78/78 focused tests pass in both extension copies; local multi-client transport coverage verifies reviewers can link a task and missing task IDs are rejected. VSIX package SHA-256 is recorded in product.json.
## Team Lobby 0.68.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â remaining-risk review evidence

Build review export now asks for an optional user-entered note describing remaining risks or open questions (up to 2,000 characters). The note is local to the exported Markdown bundle, is displayed as quoted text, and is labeled as unverified. Canceling the prompt cancels export. Three focused tests cover empty notes, Markdown line handling, control-character removal, and size limits. Both extension copies pass TypeScript checking and 81/81 focused tests. Provider-backed Build and IDE UI flow still require hands-on validation. The VSIX package hash is registered in product.json.
## Team Lobby 0.69.0

Task-linked Build sessions now keep the same virtual agent participant ID through continuation and opt-in restart recovery. Legacy saved sessions lazily receive an ID; invalid stored IDs are replaced. The ID is saved with the approved Build state before a continuation request. Room protocol remains 14. TypeScript check passes and the full Team Lobby extension suite passes 84/84 tests. Package SHA-256: `725D6EEAA6EF1DDC1C12A8B0D9C1EF4CC6FB0EFF5F428DCC664731976F035441`. Manual provider-backed and multi-window IDE validation remains open.

## Team Lobby 0.70.0

Added a typed capability boundary for read-only workspace search and file reading. Calls are normalized into stable Forge capability IDs, validated against strict input shapes, bound to the selected workspace and audited locally with content-free actor/scope/decision/outcome/size metadata. Per-call and current-request approval behavior is retained; sensitive file reads still require separate confirmation. Search query and returned content are excluded from audit records. Build/check pass and the full extension suite passes 88/88 tests. VSIX SHA-256: `53C09218C38256E2124DC57EEFE6E21F9FFFB338ED9C915B38ACBB2400D01436`. Provider-backed runtime remains open.

## Team Lobby 0.71.0

Added a production shared-artifact validator for room diffs and code comments. A diff is bounded to 8,000 characters and a relative file path; a comment requires a valid line anchor. Selected source, prompt, terminal and arbitrary content fields are rejected. Two-client transport tests verify attribution, timestamp, delivery and reconnect history for diff/comment events. Invite revocation is now exercised end-to-end at the WebSocket harness level: active guest disconnect, host audit event without token leakage, and denied reconnection. TypeScript check passes; 91/91 tests pass. Separate IDE and machine runtime acceptance remains open. VSIX SHA-256: `3F82E2B8610C1E9002B7F35E513D7251389FDBBEDD6175DF3FD72D4BF2AD98F0`.

## Team Lobby 0.72.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â user controlled room history export

Added the command **Team Lobby: Export Room History**. It writes a JSON bundle with the current room events, tasks, and notes. Before choosing a destination, the user sees that the bundle can contain messages, task details, comments, and shared Git patches, and must choose whether direct messages are included. Direct messages are excluded when the user chooses the default-safe option. Invite secrets and TLS credentials are not included by the serializer. Export failures are reported in the IDE.

**Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 92/92 tests in both the Code OSS extension and handoff mirror. The added unit test checks direct-message filtering, retention of public events, and that filtering does not mutate saved history. VSIX SHA-256: `6AD5F86F7AB3F9C2CF3F502EA276AE795252FE19ED4A20B3E4884A1CE16E3619`. This is user-requested export, not tamper-evident audit storage or a retention/deletion control.

## Team Lobby 0.73.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â explicit editor location sharing

Added **Team Lobby: Share Current Editor Location**. The user confirms each share. The room event carries only a workspace-relative file path and one-based line number, not selection text or file contents. The host validates the path and line before persisting and broadcasting the event. A teammate can open the event to navigate to that line in their own workspace; missing files and out-of-range lines are handled locally. This is a one-time navigation handoff, not live cursor tracking or shared editing.

**Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 94/94 tests in both the Code OSS extension and handoff mirror; `node --check media/room.js` passes. The new path validator rejects traversal, absolute paths, control characters, and invalid line numbers. VSIX SHA-256: `F05899D3F70388BE1592D52F0CCAE925590EDBA8180DAFEAF976E456AE456EDC`. Two-IDE UI acceptance remains open.

## Team Lobby 0.74.0 ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â opt-in live editor follow

Added **Team Lobby: Toggle Live Editor Location Sharing**, **Team Lobby: Follow Teammate**, and **Team Lobby: Stop Following Teammate**. Sharing requires a modal confirmation, streams only an in-workspace relative file path and one-based line number, and is limited to four updates per second. It sends no selection, buffer content, or cursor text and remains ephemeral rather than entering room history. Participants can stop sharing or stop following independently. Follow navigation resolves paths inside the receiver's own workspace and clamps stale line numbers. Reconnect history contains the current locations only for participants still actively sharing; disconnect, invite revocation, and explicit stop clear the host's live location state.

**Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 97/97 tests in both the Code OSS extension and handoff mirror; `node --check media/room.js` passes. The new two-client transport test covers location delivery, invalid path rejection, stop-sharing removal, and exclusion from persisted events; helper tests cover the four-per-second throttle and strict payload reduction. VSIX SHA-256: `7AEA38B1D8A98D49ECF05128BCE102387ACE784B9A16F1C3B37A7CB8A2E6D74B`. Separate IDE follow behavior remains to be exercised. This provides spectate/follow at file-and-line level; shared editing and shared buffer synchronization remain unimplemented.

## Team Lobby 0.75.0 - read-only Git status for AI

Registered `git_status` as a typed workspace-scoped capability alongside search and named-file read. It uses the built-in Git extension API and reports branch name, staged/modified/untracked/conflict counts, and up to 40 non-sensitive paths relative to the selected workspace. It reads no diffs or file contents and runs no shell command. Existing per-call or request-scoped approval, capability audit, output budgeting, and content-free evidence apply.

**Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 100/100 tests in both the Code OSS extension and handoff mirror. The capability registry rejects arguments for Git status; formatter tests cover the 40-path cap, sensitive-path omission, unrelated repositories, and nested workspaces; evidence tests confirm branch and result text are not persisted. Separate IDE/provider acceptance remains open. This completes only the read-only Git status capability; external ACP adapters and write/command capabilities remain open.
VSIX SHA-256: `988E532B5455651CB2CEC19E417638C2EB222738D34A2F2548E83FE9F6B23089`.

## Team Lobby 0.76.0 - external ACP Ask and Plan

Added optional ACP v1 as a provider choice for Ask and Plan. A machine-scoped executable and argument list launch through direct stdio without a shell. Forge Room negotiates protocol version 1, creates a one-shot session with no MCP servers, streams agent text, supports cancellation, advertises no filesystem or terminal client methods, and rejects session permission requests. External Build is not enabled; native Build remains on the IDE Language Model API path with Forge Room's reviewed diff/apply flow.

Before transfer, a modal confirmation shows the exact executable/arguments, workspace, and selected-context summary. It states that the local process runs with the user's OS account and environment and may read or change files, execute commands, or access the network outside Forge Room's client permissions. ACP sessions are not resumable and token/cost estimation is unavailable.

**Verification:** fake ACP process tests cover v1 negotiation, one-shot session/prompt, text streaming, no advertised client tools/MCP servers, permission denial, cancellation, unsupported protocol rejection, and shell-free command validation. `npm.cmd run check` passes and `npm.cmd test` passes 104/104 tests in both Code OSS and handoff copies. Real-agent/manual UI acceptance and OS process isolation remain open.
VSIX SHA-256: `04BF33E7A850B8AA1D675164D714D0147841817C94F833A26EB35FF2777D2F49`.

## Team Lobby 0.77.0 - ACP agent sign-in

ACP v1 Ask/Plan now handles agents that advertise in-agent authentication methods. After the configured process starts, Forge Room offers the supported agent-managed sign-in choices by name and description; the selected method is sent using ACP `authenticate`. Canceling the chooser stops the request before a session is created. Terminal-auth methods remain unavailable because Forge Room does not provide an interactive terminal login flow. The existing consent warns that the external process has the user's OS privileges and environment; ACP method selection does not sandbox it.

Research: [ACP v1 initialization](https://agentclientprotocol.com/protocol/v1/initialization) defines advertised auth methods and client capabilities; [ACP v1 authentication](https://agentclientprotocol.com/protocol/v1/authentication) defines agent-managed `authenticate` and terminal-auth requirements. Checked 21 October 2026.

**Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 106/106 tests in both Code OSS and handoff copies. Fake-agent tests cover the offered method filter, chosen method ID, and canceled login. Real-agent/manual IDE acceptance and OS process isolation remain open.
VSIX SHA-256: `74EF40D8570683FA83019C3CC7EB0DB00A5E59373EFB2BE090A5CDAEE6EB0815`.


## Team Lobby 0.78.0 - assigned Git worktree verification

Task-linked Build and Build resume now require an exact Git identity match: the active workspace root must match a registered or discovered linked worktree in the same repository, its actual branch must match the branch recorded for the local path, and both must match the room task's assigned branch. This rejects stale records that point to another branch, the main checkout, or a worktree in another repository. Filesystem path comparison is case-insensitive on Windows and case-sensitive on POSIX.

A new integration test creates and commits a temporary repository, adds a linked worktree in a path containing spaces, reads the real Git root/branch/git-dir/common-dir values, and verifies correct identity plus wrong-task-branch, wrong-checked-out-branch, and main-worktree rejection.

**Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 108/108 tests in both Code OSS and handoff copies. `@vscode/vsce package --no-dependencies` succeeds; package SHA-256: `A02CB58F0A29AABC02D93BA0B51F38EBC408F3A85558D22BAE7219E9C471595E`. Full IDE Build selection and resume UI flows still need runtime acceptance.


## Team Lobby 0.79.0 - stale assigned worktree recovery

When a cached local worktree record is absent, stale, or points to a branch other than the task's assigned branch, Forge Room now checks Git's registered worktrees for that exact task branch. The candidate must still pass actual root, repository common-directory, checked-out branch, and linked-worktree identity checks before its device-local path mapping is refreshed. This lets another IDE window or a recovered workspace find an existing valid worktree rather than attempting to add a branch that Git already has checked out.

**Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 108/108 tests in both Code OSS and handoff copies. The Git integration test uses a real temporary repository and linked worktree with a path containing spaces, then verifies matching and mismatched branches plus worktree discovery. `@vscode/vsce package --no-dependencies` succeeds; package SHA-256: `BF22296F3965C8CFFA528091A8A142917B5F9F56FE2F329818F82836BE14D13C`. Full IDE startup/recovery UX remains to validate.


## Team Lobby 0.80.0 - integrated task worktree recovery

Task worktree validation and rediscovery now run through one production resolver. It checks cached records against the actual Git root, repository common directory, checked-out branch and linked-worktree metadata. When a cached record is stale, it searches Git's registered worktrees for the task's exact assigned branch, validates that candidate through the same checks, and refreshes the device-local path mapping. A different repository with a coincidentally identical branch is rejected.

The Git integration test creates a temporary repository and linked worktree, then calls the production resolver with a stale path, a valid cached path, and a task branch that is not present. It verifies recovery, cache retention, and fail-closed missing-branch behavior.

**Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 108/108 tests in both Code OSS and handoff copies. `@vscode/vsce package --no-dependencies` succeeds; package SHA-256: `C6165382FF41927C08A856B9CCCEEEDA142CBE9675C1552F19323AD29CC7AA56`. Full IDE Build selection/resume interaction remains to validate.


## Team Lobby 0.81.0 - safe local task worktree cleanup

Added the **Team Lobby: Remove Clean Local Task Worktree** command to close the local task worktree lifecycle. It only offers worktrees verified against the task branch and current repository. It refuses to remove the workspace currently open in the IDE, checks for tracked and untracked changes, and requires modal confirmation showing the exact folder and deletion effect. After confirmation it revalidates the same worktree and calls `git worktree remove` without `--force`; Git can reject a change that occurs after the inspection. The shared task assignment and local Git branch are preserved. No file paths, contents, or room events are sent by cleanup.

A real Git integration test proves dirty files prevent removal and remain present, while a clean reviewed worktree is removed and its branch remains. **Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 109/109 tests in both Code OSS and handoff copies. `@vscode/vsce package --no-dependencies` succeeds; package SHA-256: `646E955E42FAFAF3A8AC85F2178AC214A4C1BC4E2CBDCE7AA99C728002073D5B`. IDE command-palette interaction remains to validate.

## Team Lobby 0.82.0 - opt-in live shared documents

Connected bounded Yjs text documents to room transport and VS Code text-change/apply-edit handling. Host starts a live document session after a modal confirmation; participants request access and the host approves each subscription. Reconnecting participants must be approved again and merge their local offline state. Paths are announced to the room, contents go only to approved subscribers, buffers remain unsaved, and the host connection warns if transport is unencrypted. Document state is in-memory only and capped at 512 KB.

**Verification:** TypeScript checks and 115/115 tests pass in the Code OSS checkout and handoff mirror. Automated tests cover safe relative paths, document/update bounds, concurrent Yjs convergence, subscription-gated relay, atomic editor change batches and large initial state transfer. The bundled entry point loads with its runtime dependencies resolved under a mocked VS Code API. The bundled 0.82.0 VSIX and Code OSS built-in manifest are synchronized. Package SHA-256: `4902A209CCE3487A4DBC9932D36EE1188207C77378D4152A31CACE5EE65B4F51`. Separate two-IDE editing and reconnect acceptance remain open.

## Team Lobby 0.83.0 - multi-root shared documents

Live shared editing now supports documents in any uniquely named local workspace folder. Multi-root room paths include the folder name; participants must map it to exactly one local folder, and duplicate folder names are rejected. Protocol version 16 prevents older clients from misreading these paths. Folder mapping remains bounded by the existing safe relative-path resolver.

**Verification:** TypeScript checks pass and all 116 tests pass in both the handoff extension and Code OSS checkout. The bundled VSIX contains nine files and its SHA-256 is `D475768CDE4771A77F0BAB277A6DBC7D2789629E483A41BA2455FB3236EF15F4`, matching the built-in Code OSS manifest. Automated coverage verifies multi-root mapping, duplicate-name rejection, traversal rejection, and file resolution. A local WSS integration check accepted a trusted TLS certificate and valid bearer invite, rejected an invalid invite, and refused an untrusted certificate. Two-IDE editing/reconnect and separate-machine routing acceptance remain open.

## Security documentation - self-hosted room threat model

Added [THREAT_MODEL.md](../architecture/THREAT_MODEL.md) with the current room and AI trust boundaries, protected assets, threats, implemented controls, residual risks, and explicit release gates for a public managed service. It distinguishes local self-hosted WSS with bearer invites from a managed multi-tenant service and records what the local WSS check did and did not verify.

## Room service 0.1.0 and Team Lobby 0.84.0 - integrated WSS relay slice

Added `services/room-service`, a TLS-only HTTPS service and WSS relay. An operator provisions accounts; accounts create owned room metadata and one-time-visible invite tokens with a 60-second to 30-day expiry. Token hashes, room metadata, revocation state and a bounded lifecycle audit persist in a synced, atomically replaced local JSON database. Invite authentication rejects wrong-room, malformed, expired, revoked, inactive-room and disabled-account credentials. The relay authenticates host upgrades, validates participant invites, forwards the existing room protocol, bounds payloads and per-participant throughput, and removes active clients when invites or accounts are revoked. Team Lobby 0.84.0 can configure the service, keep the account key in SecretStorage, host through WSS, issue/revoke service invites, and bridge peers into the existing local room authority.

**Verification:** Both Team Lobby copies pass TypeScript checks and 121/121 tests. Both room-service copies pass 6/6 TLS/relay tests. The nine-file 0.85.0 VSIX SHA-256 is `AB011EE13810CF47F5A85209AFE7ED6E5683B17ACC5CD7CD8873504F32FEE646` and is synchronized to the Code OSS built-in extension manifest. Relay connections are process-memory only, and the service does not persist room messages, tasks, notes, or shared buffers. Account recovery, key rotation, tenant operations, multi-process storage, monitoring, independent review, two-IDE and separate-machine acceptance remain open.

## AI workflow refinement - structured Plan artifacts

Plan export now writes a consistent Markdown artifact containing the original request, proposed plan, model, timestamp, and optional linked room task. Existing transfer receipts show selected source character counts, token estimation, and a separate generation approval. Approved tool progress reports bounded provenance and output size without exposing search queries or result text. Team Lobby 0.85.0 contains these refinements.

**Verification:** Focused tests cover artifact fields, multiline request quoting, Markdown table escaping, and empty or oversized plan rejection. This improves handoff review but does not replace runtime validation with real model providers.

# Forge AI native expansion implementation status

## 0.86.0 Ã¢â‚¬â€ Room service account key rotation

Self-hosted room-service account owners can rotate their bearer key with `POST /v1/account/rotate-key`. The one-time response contains the new key; the database stores only its hash, and the old key is invalid immediately. Rotation writes an account-scoped audit event available through `GET /v1/account/audit`, preserves owned rooms and invites, and disconnects currently hosted rooms and their guests.

Team Lobby exposes **Team Lobby: Rotate Room Service Account Key**. It requires a modal confirmation, calls the configured HTTPS service, validates the returned key format, and stores the replacement in VS Code SecretStorage. Active service-hosted rooms must be started again after rotation.

**Verification:** room-service tests cover old-key rejection, new-key access, room ownership continuity, durable hash-only storage, audit secrecy and relay session disconnection. Team Lobby TypeScript checks and its full test suite verify integration compilation and regressions. Product checkout and handoff copies and built-in VSIX metadata are synchronized.

**Package sanity:** Team Lobby 0.86.0 builds to a nine-entry VSIX. Its SHA-256 is `1FF266018717C26C4139A52714B90871663A9FE56071704682C4ABF4D7EECE90`, embedded manifest reports 0.86.0, and the Code OSS built-in extension manifest references the same package/version/hash. Team Lobby tests pass 121/121 and room-service tests pass 8/8 in both mirrors.

## Team Lobby 0.87.0 Ã¢â‚¬â€ room-service audit browsing and account recovery

Team Lobby adds **View Room Service Audit History**, a Command Palette viewer for account, owned-room, invite, and key lifecycle records. It loads bounded pages, offers older pages on demand, and displays only action, timestamp, actor, and resource ID. The control plane now applies the same bounded cursor paging to account-wide and per-room audit APIs; default page size is 100, maximum is 200, and expired cursors fail with an explicit error. Account scope includes only the authenticated account's own records.

The room-service admin API supports one-time key recovery for active or disabled accounts. Recovery invalidates the prior key, stores only its hash, preserves room ownership, disconnects active hosted sessions, and records an `account.key_recovered` event. Restoring a disabled account also revokes its outstanding invites atomically. Team Lobby owner-initiated rotation remains available and continues to store the replacement in SecretStorage.

**Verification:** Team Lobby passes TypeScript checking and 121/121 tests in both mirrors. Room service passes 9/9 HTTPS/control-plane/relay tests in both mirrors, including newest-page/older-page traversal, account isolation, invalid/expired cursors, disabled-account invite revocation, and key replacement. The nine-file 0.87.0 VSIX SHA-256 is `BE6798B1193964FC9C756F05E16324EA99FDC0EDD6F230DE37A722FAA79FE04C` and matches the Code OSS built-in manifest.
## Team Lobby 0.88.0 Ã¢â‚¬â€ checkpoint rollback revalidation

Build rollback now validates all checkpoint paths resolve inside the workspace. Before the confirmation dialog it verifies every target still matches its post-Build snapshot. After confirmation, a synchronous second pass checks existence, canonical path containment, created-file symlink status, and current document contents immediately before the atomic workspace edit. Any conflict cancels the full rollback before applying any file changes.

**Verification:** three new rollback guard tests cover unchanged targets, post-Build unrelated edits, missing files, escaped paths, and symlinked Build-created files. Team Lobby TypeScript checking passes and the extension suite passes 124/124 in the handoff copy; The Code OSS mirror passes the same 124/124 suite. The nine-file 0.88.0 VSIX SHA-256 is `7B8A9304F8D126D7EAAD350C50DD6B7DC8AE2489E0E427F5BF4FE75BBB1584D7` and matches the built-in extension manifest.

## Team Lobby 0.89.0 Ã¢â‚¬â€ local MCP tools for Ask/Plan

Added optional, per-request selection of up to eight machine-configured MCP stdio servers. The official `@modelcontextprotocol/client` 2.3.1 SDK handles MCP negotiation and transport. Ask/Plan can present up to 40 advertised tools to the selected IDE model; each tool schema is capped at 8 KB and aggregate tool schemas at 120 KB. Every invocation requires a separate confirmation that names the server, tool, and complete JSON arguments; calls with argument payloads above 3,000 characters are rejected. Each call is capped at 12,000 output characters and each request at six calls/30,000 output characters. MCP processes close after the provider turn. Content-free local audit entries record actor, workspace, server/tool, approval decision, outcome, and output size. Build and ACP do not use MCP tools. The prompt warns that selected servers run with the IDE user's ambient OS privileges, can access files/run commands/use the network on their own, and are not sandboxed. Only command, arguments, and working directory are supported; remote HTTP/OAuth, resources, prompts, stored grants, and OS isolation remain open.

**Verification:** Team Lobby TypeScript checking and build pass in both handoff and Code OSS copies. Both copies pass 130/130 tests, including MCP stdio handshake/list/call/close, stalled initialization, stalled tool discovery, and stalled tool execution. The nine-file 0.90.0 VSIX SHA-256 is `09DD41F4932A2A17462FD203C668D249DF50711D8CC6154CCE9376805888E42E`; it is copied to both mirrors and matches built-in `product.json` metadata. Third-party MCP and provider-backed/manual IDE acceptance remain open.

## Team Lobby 0.90.0 Ã¢â‚¬â€ MCP timeout and cancellation handling

MCP server startup and tool discovery now time out after 15 seconds per request and appear in a cancellable progress notification. MCP tool calls have a two-minute timeout and inherit cancellation from the active Ask/Plan request. If connection/discovery fails or is canceled, the partially opened server processes are closed. The turn closes its selected server sessions on completion or cancellation. No permission boundary changed: these remain trusted, unsandboxed local processes.

**Verification:** TypeScript checking and build pass in both mirrors; both pass 130/130 extension tests. The MCP fixture covers nonresponsive initialize, tools/list, and tools/call requests; timeouts reject within the configured bound and sessions close in cleanup. The nine-file VSIX SHA-256 is `09DD41F4932A2A17462FD203C668D249DF50711D8CC6154CCE9376805888E42E` and matches the Code OSS built-in manifest.

## Team Lobby 0.91.0 Ã¢â‚¬â€ scoped Git diff capability for AI

Ask, Plan and Build can now request the staged and unstaged tracked Git diff for one workspace-relative file through the built-in Git extension API. Every call goes through the existing read-only capability approval flow; the approval names the file and states that its patch content will be sent to the chosen provider. Both capability normalization and execution reject likely sensitive paths. Paths are confined to the selected workspace and resolved against the nearest containing Git repository. Untracked files, binary patches and unsaved editor buffers are omitted. Diff output is capped at 12,000 characters, with the existing aggregate output budget applied afterward. Local audit/evidence records path and size only.

**Verification:** `npm.cmd run check` passes and `npm.cmd test` passes 137/137 in both handoff and Code OSS extensions. Six `gitDiff` tests cover staged/unstaged retrieval, nested repository selection, traversal/sensitive-path rejection, empty/binary/rename diffs and truncation. Capability and evidence tests cover the approval summary and content-free provenance. The nine-file VSIX SHA-256 `D0F36BFF89A596240AB2422D16C76E40EDCAC3227A6030ADE30C737997A88446` matches Code OSS built-in metadata. Provider-backed and interactive IDE acceptance remain open.

## Team Lobby 0.92.0 Ã¢â‚¬â€ approved workspace diagnostics for AI

Ask, Plan and Build can query current VS Code diagnostics for the selected workspace or one named file. The typed `diagnostics_list` capability is read-only and uses the existing approval flow; the prompt explains that diagnostic messages will be sent to the selected provider. Results are limited to 100 diagnostics and 12,000 characters, with severity, code/source, file path and location. Out-of-workspace and likely sensitive paths are excluded. Messages are sanitized and bounded; local audit/evidence stores source paths and result size only.

**Verification:** `npm.cmd run check` and `npm.cmd test` pass in both handoff and Code OSS copies; both suites pass 142/142. Formatter tests cover clean formatting, empty results, 100-item/output bounds, text sanitization, malformed locations and unsafe paths. Capability and evidence tests cover scope approval and exclusion of diagnostic text from local evidence. The nine-file VSIX SHA-256 `CB4616885C3A99988D7C39ECEF5F4CB6503DD897888BD526FAD76648B42201EC` matches Code OSS built-in metadata. Provider-backed and interactive IDE acceptance remain open.

## Team Lobby 0.93.0 Ã¢â‚¬â€ approved workspace symbol search for AI

Ask, Plan and Build can use `symbols_find` to query registered IDE workspace symbol providers. The query is shown in the approval prompt and sent to those providers; matching symbol names, kinds, containers and workspace-relative locations go to the selected model. Forge reads no source text. Results are capped at 100 symbols and 12,000 characters; paths outside the selected workspace, unsafe paths and likely sensitive paths are excluded. Local audit/evidence stores paths and output size, not the query or symbol names.

**Verification:** `npm.cmd run check` and `npm.cmd test` pass in both handoff and Code OSS copies; both suites pass 147/147. Four new symbol formatter tests cover location formatting, empty results, count/output bounds, sanitization and unsafe paths; capability and evidence tests cover query disclosure and content-free local evidence. The nine-file VSIX SHA-256 `8AA3DDF975B4432D26E8668C4A024C701B09290C4EC9B0AA5A9C0F3EFA98FADF` matches Code OSS built-in metadata. Interactive provider-backed acceptance remains open.

## Team Lobby 0.94.0 Ã¢â‚¬â€ approved test task discovery for AI

Ask, Plan and Build can use `tests_discover` to list test-group tasks registered with the IDE and scoped to the selected workspace. Results include bounded task names and provider labels only; task execution definitions, commands, arguments, environment values and terminal output are not exposed. The existing workspace-tool approval explains that names and provider labels go to the selected model and that discovery does not run a task. Local evidence records the result size and outcome, not task names. Execution remains a distinct, unimplemented capability that needs its own exact-task approval and cancellation/result handling.

**Verification:** `npm.cmd run check`, `npm.cmd run build` and `npm.cmd test` pass; the suite passes 151/151. Added tests cover task-label sanitization and bounds, capability registration and approval disclosure, rejection of unexpected inputs, and evidence redaction. Runtime discovery against real test providers and provider-backed model acceptance remain open.

**Package:** `team-lobby-ide-0.94.0.vsix` (9 files)  
**SHA-256:** `C43458080F4125AAF4DBD31087839F52EE51B11A7CE2A711BA619A8076F47F33` Ã¢â‚¬â€ matches both handoff and Code OSS copies and the built-in extension metadata.

## Team Lobby 0.95.0 â€” confirmed IDE test execution

Build can use `tests_run` for one registered test-group task. Forge re-fetches the selected workspace tasks and requires an exact name/provider match. In the 0.95.0 flow, confirmation came before the duplicate picker, so it did not identify which duplicate would run. Version 0.96.0 moves selection before confirmation. A request-wide grant applies only to read-only tools. The task runs with the user's OS privileges and may modify files or use the network. Output stays in the local task terminal; only the process exit code is sent to the model. Cancellation and the 10-minute timeout request termination, and missing exit events remain unverified.

**Verification:** handoff and Code OSS copies pass TypeScript checks and 153/153 tests. The nine-file VSIX SHA-256 is `BE9E1BCE7DE11DCC721F4553756556F9B67C269496F1F200B2F0390AB8F0F55F`; all package copies match both built-in manifests.

## Team Lobby 0.96.0 â€” exact task selection precedes execution approval

If a requested test-task name/provider resolves to multiple IDE tasks, Forge shows a picker first. It then asks for confirmation naming the selected match and executes that selected task object. This keeps approval attached to the exact task the user chose.

**Verification:** TypeScript checks, build and full extension suite pass in both handoff and Code OSS copies: 153/153 each. The nine-file VSIX SHA-256 is `0AD506D6E434196421292C293525E5EF7A9B238ADA5C36937F7B303EE5F11C0C`; all four package copies and both built-in manifests match.

## Team Lobby 0.97.0 â€” host-controlled session countdown

Team Lobby 0.97.0 adds confirmed 24-hour hackathon quick start and host-only pause/resume for the room submission deadline. The remaining duration is persisted in host room history, synchronized in reconnect state, and each transition is recorded with the host identity and timestamp. Participants can view the frozen duration and cannot operate the timer. The room protocol is version 17.

**Verification:** both the handoff and Code OSS extension copies pass TypeScript checks and 155/155 tests. The nine-file VSIX SHA-256 is `1844A2E85507A539F7B5DAFBBD3D10F9E25D066D3652071CE04573AE6B422F54`; the root package, handoff extension package, Code OSS extension package and built-in `product.json` entry use this exact version and hash. Separate IDE and two-machine acceptance remain open.
## Team Lobby 0.98.0 â€” opt-in live Git diff snapshots

Team Lobby 0.98.0 adds live updates for one selected tracked file after the user previews and explicitly approves the initial Git diff. Subsequent saves publish a new full snapshot at most once every five seconds. The room panel identifies the active file and provides a stop control. Sharing ends on room leave, an empty diff, an oversized patch, or a Git refresh error. Previously shared snapshots remain in host history. No protocol change; clients remain on protocol 17.

**Verification:** both the handoff and Code OSS extension copies pass TypeScript checks and 156/156 tests. The nine-file VSIX SHA-256 is `CAF1BF04D3E2D9AB50AB366E533D0CA752E39977EBA792B3AFBB7A8351D91F3E`; the root package, handoff extension package, Code OSS extension package and built-in `product.json` entry use this version and checksum. Separate-IDE consent/navigation testing remains open.

## Team Lobby 0.99.0 - multi-root AI request scoping

Ask, Plan and Build now let users select the project root in multi-root workspaces. Context file discovery/resolution, editor selection, diagnostics, Build worktree detection/creation, and persisted Build resume validation use that root. The helper returns workspace-relative paths and rejects files outside the selected root. Single-root workspaces do not need a picker. Room protocol remains version 17.

**Verification:** both handoff and Code OSS extension copies pass TypeScript checking and 157/157 tests. The nine-file VSIX SHA-256 is `AF19572A76CF1C4BC749AB125C7CA0F0C848BD19BC9717B1DDED920CDE8961C2`; all three package copies and the Code OSS built-in manifest use this version and checksum. Live multi-root IDE/provider acceptance remains open.
## Forge Room Studio 0.11.0 - opt-in decorative motion

The Studio gallery now starts with all decorative entrance animations, transition movement and pressed movement disabled. A keyboard-operable Motion button opts in and persists the preference with the webview state API. The system `prefers-reduced-motion` rule always overrides the opt-in. At narrow panel widths, the nonessential Studio label hides to keep the control visible.

**Verification:** both Studio copies pass `npm.cmd run check` and `npm.cmd test` (6/6). The 17-file VSIX SHA-256 is `61417FA4BB79C04BFBF1495F4604A95AD70D250948304FCAAABC40437CE1D2BB`; all three package copies and the Code OSS built-in manifest use version 0.11.0 and this checksum. Separate IDE visual, keyboard and screen-reader acceptance remains open.
## Forge Room Studio 0.12.0 - theme contrast checks

The six bundled color themes now pass automated WCAG contrast checks for core workbench text roles and all syntax token colors (4.5:1), with focus borders and inactive activity icons checked at 3:1. Paper Studio's low-contrast comments, numeric tokens, line numbers, descriptions, inactive title and inactive tab colors were darkened. Dark themes' line numbers and several activity badge labels were raised for readability. Studio motion remains off by default and respects the OS reduced-motion setting.

**Verification:** both Studio mirrors pass syntax checking and 9/9 tests. The tests assert actual ratios from each theme JSON. The 18-file VSIX SHA-256 is `FE6C69F2FA855100D81818C412EF44CBBDCEBD37C0A5FCB7B5C830FCDE590832`; all package copies and the Code OSS built-in extension entry match. Screen-reader and rendered IDE acceptance remain open.
## Forge Room Studio 0.13.0 - live preview and apply confirmation

Selecting a theme now temporarily applies it, then presents a modal choice to keep it or restore the previous appearance. Dismissing the choice restores the saved settings. Kept themes retain the **Restore Previous Appearance** command. The choice lifecycle is isolated in `themePreview.js` and tested for Keep, Restore, dismissal, and dialog failure recovery.

**Verification:** both Studio mirrors pass syntax checks and 12/12 tests. The 19-file VSIX SHA-256 is `8D530420468C9CE543BF1DE0A79994978C98E0847867FC4CF79705CD0DEF0EF3`; all three package copies and the Code OSS built-in extension entry match. Actual IDE theme rendering and interactive modal behavior remain open for manual validation.

## Forge Room Studio 0.14.0 â€” reversible project profile preview

Project profile import now offers a temporary live preview with explicit **Keep Profile** and **Restore Previous** choices. Dismissing the dialog restores the prior settings. A partial apply failure triggers rollback. Supported imported color keys and syntax scopes overlay current customizations while preserving unrelated keys, scopes, and token rule properties. Added tests cover merge and rollback behavior.

**Verification:** both Studio copies pass `npm.cmd run check` and 16/16 tests. The 20-file VSIX SHA-256 is `EB9ADD6F074E3FD327F4062B4AC515F553347A63224B1262ED46125475B41502`; the handoff root package, handoff extension, Code OSS extension, and built-in extension manifest are synchronized. Live IDE profile import and rendered appearance remain to validate.

## Repository sanity check â€” 21 October 2026

`corepack yarn compile` passed in the Code OSS product checkout, compiling the workbench and Copilot bundles with zero TypeScript compilation errors (197 seconds). Team Lobby passed 157/157 tests in both the handoff and Code OSS copies; Forge Room Studio passed 12/12 in both copies; room service passed 9/9. This confirms local compilation and automated suites only. Separate IDE client, two-machine TLS, provider-backed AI, and rendered theme/UI acceptance remain open. Plan export is a Markdown artifact containing the request, proposal and provenance; it does not yet validate a machine-readable plan schema.

## Team Lobby 0.100.0 â€” multi-root AI session continuity

Saved Ask/Plan/Build continuation now resolves the exact workspace folder recorded by the session URI, including a non-first root in a multi-root workspace. Continuing fails with a clear message if that project folder is no longer open. Build-from-Plan stays pinned to the Plan's originating root instead of asking the user to select a potentially different project. Plan export defaults to the recorded root as well.

**Verification:** both extension copies pass TypeScript checks and 158/158 tests. The new workspace-folder test proves selection of the non-first root and rejection of a removed root. The nine-file VSIX SHA-256 is `2CAC7730F5DFA9E104DD403AD1EFCCE181FE6E77E446B0B4382F046787EE493B`; root/handoff/Code OSS packages and the built-in product manifest match. Interactive multi-root provider behavior remains unverified.

## Team Lobby 0.101.0 â€” multi-root terminal context scoping

Captured terminal commands are initially associated with the workspace root from the shell's execution cwd. When a command ends, a different or missing reported cwd clears that association. In a multi-root workspace, Ask/Plan/Build only offer captures associated with the selected project; unknown-root captures are excluded there. Root-known captures from a different project remain excluded even if the workspace folder list later changes. Single-root behavior continues to allow explicitly selected captures with an unreported cwd.

**Verification:** both extension copies pass TypeScript checks and 161/161 tests. Focused tests cover stable root attribution, root changes, selected-root filtering, unknown roots in multi-root workspaces, and preserved single-root behavior. The nine-file VSIX SHA-256 is `E72C734B9C7F5C67BAE9A04FBFE60023020EC5CDD119AA96D0FB7B364C9BD979`; root/handoff/Code OSS packages and the built-in product manifest are synchronized. Live shell-integration/runtime behavior remains open.

## Team Lobby 0.102.0 â€” approved active debugger stack inspection

Ask, Plan, and Build can request up to 20 stack frames from the selected thread in the active debug session. Forge checks that the session belongs to the selected workspace, requires a paused selected call stack, and filters results to existing, non-sensitive files inside that workspace. Only sanitized frame names and relative source locations are returned; variables and debugger control actions are not exposed. The existing approval flow names the data sent to the provider. This uses the documented VS Code `DebugSession.customRequest` API and Debug Adapter Protocol `stackTrace` request; adapter support and paused-thread state can vary. Forge bounds its wait to 10 seconds and returns promptly on cancellation; VS Code does not expose cancellation for the underlying DAP request.

**Verification:** Both Team Lobby copies pass TypeScript checks and 162/162 extension tests. Capability tests cover registration, exact empty input, selected workspace binding, and approval disclosure; evidence tests verify frame paths are kept without frame names. The 17-file 0.102.0 VSIX SHA-256 is EF2288E5C5B6B01404F3C1254D66F23ED85C047C6AA9D247B2F97E7F6BC75C56 and matches the Code OSS product manifest and both handoff package copies. Runtime debugger adapters and interactive provider behavior remain to be validated. The DAP permits adapter-specific support differences; unsupported or resumed-thread requests return a clear failure instead of fabricated frames.

## Team Lobby 0.103.0 â€” consented ACP session continuation

ACP Ask/Plan requests now preserve an agent session ID in VS Code workspace state when the configured agent advertises the ACP v1 `loadSession` capability. On a later request in the same workspace, with the same mode and exact command/arguments, Forge asks whether to continue or start a new session. Continuing has a separate consent prompt that explains the agent will restore previously sent conversation history. The chosen authentication method ID is recorded with the session, and a changed method prevents reuse. Only IDs are stored locally; prompts and response content are not added to Forge's stored session state. Agents without load support start fresh. Replayed history updates are ignored when forming the current answer, and canceling the choice does not silently create a new conversation.

**Verification:** ACP fake-agent tests cover supported loading, replay filtering, unsupported agents, authentication mismatch, and cancellation. Both extension copies pass TypeScript checks and 168/168 tests. The 17-file VSIX SHA-256 is 18E3F8105A5E41330F0392BB3AA3CBA3E4BC8DB107A88C321C21629DE4615550 and matches the Code OSS product manifest and both handoff packages. Real provider compatibility and persistence behavior remain to validate.

## Team Lobby 0.104.0 â€” explicit AI access to captured terminal results

Ask, Plan, and Build can request the newest locally captured terminal output associated with the selected workspace. Forge freezes that specific capture, shows its command (with known credential patterns redacted), terminal, running/completed state, exit result, size, and truncation status, then requires a separate per-call confirmation. This confirmation is mandatory even if the user granted read-only tools for the request because terminal output can contain secrets. Unknown-root captures are excluded for multi-root projects. Local audit and Build evidence retain only capability, output size, and outcome; command text and output are excluded. Output is sanitized, then bounded by the existing 12,000-character per-call and 30,000-character request limits.

**Verification:** tests cover selected-root recency, empty captures, command redaction, partial/truncated output formatting, capability scope, extra-argument rejection, mandatory approval despite request-wide read-only grants, and evidence redaction. Both handoff and Code OSS copies pass `npm.cmd run check` and 171/171 tests. The 17-file VSIX SHA-256 is `DC7AEAD7A24F913551CE1A893920F8ED07E73CD04031E7BF444CD925F440A594` and matches the handoff root, both extension package copies, and the built-in product.json manifest. Live provider, terminal capture/runtime, separate-IDE and multi-machine behavior remain unverified.

## Team Lobby 0.105.0 â€” approved exact workspace edits

Build can request a one-occurrence text replacement in one existing, non-sensitive file in the selected workspace. Forge validates a unique match, opens the full-file before/after diff, then requires a separate per-call confirmation regardless of request-wide read-only approval. After confirmation it rechecks the open document version and contents; edits are applied unsaved through the IDE edit API and can be undone normally. Files are capped at 256 KB and both snippets at 12,000 characters. Audit evidence keeps the file path, result size and outcome, not either excerpt.

**Verification:** both handoff and Code OSS copies pass npm.cmd run check and 174/174 automated extension tests. The 17-file VSIX SHA-256 is `838583216E620109596DEF720D218B87E551DF6D360A6751EF1D6E6AED346E41` and matches the handoff root package, handoff extension package, Code OSS extension package, and built-in product.json manifest. Tests cover exact-match preparation, duplicate/missing/empty/oversized replacement rejection, sensitive/traversal path rejection, tool schema and mandatory per-call approval. Live diff UI, provider tool selection and manual unsaved-buffer/Undo behavior remain to validate.
## Team Lobby 0.106.0 â€” rollback checkpoints for exact Build edits

Each exact, separately approved `apply_workspace_edit` operation now saves an ordinary single-file Build checkpoint before applying the edit. The checkpoint retains the original and approved text for the existing Roll Back Last Build command. Rollback validates containment and requires the current file to still match the approved result; if it has any later change, the full restore is refused. Room task activity carries the checkpoint ID and still shares paths only under the existing file-activity setting. Edits remain unsaved and are also available through editor Undo.

**Verification:** both handoff and Code OSS copies pass `npm.cmd run check` and 175/175 tests. Added coverage verifies checkpoint contents and that a later modification is rejected by the standard rollback guard. The 17-file VSIX SHA-256 is `D7896744609ED694E7E9A199C6D1E2EAF5B7C3BB900D00C6DA0B2CC38E630AF2` and matches the two handoff packages, Code OSS package and built-in product manifest. Live IDE Undo/rollback interaction, provider-backed edits and multi-client runtime behavior remain unverified.

## Team Lobby 0.107.0 - bounded AI workspace inventory

Ask, Plan, and Build can request a metadata-only inventory of up to 250 non-sensitive paths in the selected project root. The inventory skips common generated directories, resolves file paths through the existing root-containment guard, reads no file contents, and caps the provider result at 12,000 characters. Its typed workspace capability discloses the file-name transfer, supports existing request-scoped read-only consent or per-call approval, and persists only output size/outcome in local evidence.

**Verification:** handoff and Code OSS copies pass TypeScript checks and 178/178 tests. Added coverage exercises capability scope/schema, sensitive/traversal path filtering, duplicate removal, empty results, output bounds, and content-free evidence. The 17-file VSIX SHA-256 is `A25DF060D7E6FA2C21088376830A58874C1B320A52B2CD3800EB06EC3FDAA380` and matches the built-in manifest and package copies. Runtime provider invocation and separate-IDE collaboration remain open.


## Team Lobby 0.108.0 - confirmed editor navigation

Ask, Plan, and Build now expose editor_open. The tool accepts one safe workspace-relative file and an optional positive one-based line, resolves the target under the selected root, rejects sensitive paths and invalid line positions, and opens/reveals it in the IDE. It always requires its own user confirmation, including when request-wide read-only tools were enabled. The provider receives only a bounded result label and path; no file text is returned by this action.

**Verification:** both mirrors pass TypeScript checks and 179/179 tests. Tests cover capability schema, sensitive/traversal rejection, line validation, mandatory per-call approval, and content-free path evidence. The 17-file VSIX SHA-256 is 6EF2EC09E51E25884577C159AA19382A947C682C6F1487D080C9B6ACF8F0FBFA, synchronized across the three package copies and built-in product manifest. Provider-backed and live IDE interaction remain unverified.
