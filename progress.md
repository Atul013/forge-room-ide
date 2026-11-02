# Forge Room — progress and complete working plan

**Updated:** 2 November 2026. **Active product:** native Rust IDE, Developer Preview `0.110.0-alpha.1`, Windows 11 x64. **Current phase:** Phase 1, visible Windows baseline. **Owners:** You and Codex.

This is the current project status, assignment, and next-action document. Historical documents remain available for requirements and evidence, but their older Code OSS status statements do not describe the native app. **Done** means code or an action is complete; **CI passed** means automated checks passed; **Needs hands-on check** means actual product behavior remains unverified; **Planned** means it is not implemented.

## Repository and migration state

- Original repository: `ZayedBH/forge-room-ide`. Its `main` now contains the native Rust replacement through PR #3. The Code OSS handoff at `b669705` is history.
- Supplied fork: `Atul013/forge-room-ide`. The active Rust source imported here is `native-rust-ide` at `5fadb8f40bf42325e8a9a4e17ec3a59eaa1adea7` (Developer Preview `0.110.0-alpha.1`). The fork also has an open early-preview PR; that branch is older than the imported native branch.
- Imported from `atul/native-rust-ide`: `native/`, `packaging/`, `design-assets/`, `.github/`, `COLLAB.md`, `docs/native/`, and `docs/design/`. The native README is now the entry point. The previous README is preserved at `docs/legacy/README_CODE_OSS.md`; existing documents, `PROJECT_PROGRESS.md`, and planning files remain in the checkout.
- The former Code OSS `extensions/`, `services/`, `releases/`, and `.build/` directories have been restored at their original workspace paths, including local edits, new modules, packages, and embedded documents. They are retained as migration references; the active application is `native/forge-room-app`. The source, tests, and documents in the first three folders are restored to Git. Generated `.build/` output and ignored VSIX packages remain available locally, outside Git.
- The native replacement was merged into `ZayedBH/forge-room-ide` `main` through [PR #3](https://github.com/ZayedBH/forge-room-ide/pull/3) on 2 November 2026 (merge commit `f6f0a7d`; follow-up ledger commit `9c382af`). The fork is remote `atul`; `origin` remains the original repository. Windows PR CI passed Rust formatting, locked dependency resolution, checks, tests, and clippy for both core and graphical app. The PR packaging job was skipped by workflow design. Installer packaging and hands-on Windows acceptance remain open. Earlier local edits to historical documents remain uncommitted.

## Product goal and hard constraints

Build a fast, native, customizable IDE with a complete daily coding loop and Forge Room collaboration. The current app uses Rust, egui/eframe, wgpu, and a separate `forge-room-core` policy crate. No Electron, webview, or bundled Node runtime. Windows 11 x64 and per-user installation are the first supported target. Keep solo/offline use, private sharing defaults, owner-approved agent actions, recoverable changes, Git as durable source of truth, accessibility, and honest feature/verification labels. The egui editor is an early implementation choice; benchmark it against other Rust editor components before committing to it for large files.

## Implemented in the imported native branch

| Area | Current implementation | Evidence boundary |
|---|---|---|
| Editor shell | Folder explorer, multiple tabs, native dialogs, UTF-8 open/edit/save, dirty prompts, shortcuts, status bar | Prior branch progress reports real executable checks; recheck in this checkout. |
| Editing/navigation | Syntax colors, line numbers, current line, breadcrumbs, command palette, quick open, go to line, find/replace, file search | Large-file performance, external edits, bracket matching, indent guides, and minimap remain open. |
| Git | Status, diff, stage, commit, branch, sync/pull/push/fetch, stash, commit graph, file history, inline blame | Conflict resolution, line history, rebase/cherry-pick, and richer remote/tag views remain open. |
| Terminal | One integrated PowerShell session through ConPTY | Scrollback UI, selection/copy, tabs/splits, shell picker remain open. |
| Room | Local per-project people, agents, tasks, ownership, dependencies, handoffs, evidence, activity; policy in `forge-room-core` | No native network relay, invites, presence, chat, shared documents, or agent provider clients yet. |
| Appearance | Six Studio themes with live preview and saved choice; newer Forge default visual shell | Profile import/export, custom palettes, accessibility and display-scale acceptance remain open. |
| Installer | Per-user Windows installer, upgrade/reinstall/downgrade rules, themed uninstaller | CI packaging, actual install/upgrade/uninstall, DPI and high-contrast checks need fresh evidence. |
| AI panel | UI layout and composer | No functional provider backend. Do not present it as working AI. |

The older Code OSS work has a broader collaboration and AI feature inventory. Those features are **not native ports** simply because their sources are present. The full intent is retained below; `PROJECT_PROGRESS.md` and `docs/product/PRODUCT_SPEC.md` carry the detailed requirements and old implementation evidence.

## Complete product feature inventory and migration status

This inventory restores the product scope that the short roadmap hid. **Legacy** means the Code OSS implementation or prototype is available in the restored folders. **Native** means behavior is integrated into the Rust app or core. **Planned** means the native product still needs it. The order column maps each area to the roadmap phase where it should be handled; it is not a claim that all items in that phase are already scheduled as one slice.

| Product area | Intended experience | Current native state | Legacy reference | Roadmap |
|---|---|---|---|---|
| Core IDE | Project and multi-root workspaces, fast editing, search/navigation, language tools, terminal, tasks/tests, debugger, Git, accessibility, offline solo work | Editor, search, terminal and broad Git foundation; LSP, DAP, task runner and deeper editing absent | Code OSS product and Team Lobby workspace tools | 1, 3, 4 |
| First-run room flow | Create/join/recent rooms, project/repo binding, privacy and sharing preview, LAN/service selection, duration and milestones | Local project room only | Team Lobby host/join and invite flows | 5 |
| Room membership and lifecycle | Invite-only identity and roles, participant/agent ownership, moderation, pause/archive/export, revocation, reconnect | Local identities, tasks, handoff and room persistence | Team Lobby protocol, room-service account/invite controls | 5 |
| Room UI | Lobby, members, tasks, chat, agents, activity, build/demo, connection and error states, keyboard access | Local Room view; several panels are placeholders | Team Lobby webview | 5 |
| Communication | Public chat, direct messages, mentions, reactions, threads, decisions, search, anchored comments with privacy | Not ported | Team Lobby chat, DMs, threads, decision and search modules | 5 |
| Tasks and hackathon timing | Ownership, status, priority, estimates, dependencies, deadlines, milestones, countdown, critical path, reminders, handoff and readiness | Basic tasks, dependencies, acceptance, handoff and evidence; advanced timing not ported | Team Lobby task and countdown modules | 5 |
| Activity and awareness | Attributed event feed with live/delayed/offline labels, evidence links, since-last-seen and retention rules | Local activity | Team Lobby room history and activity modules | 5 |
| Work visibility | Consent-based paths, saved diffs, optional unsaved buffers, terminal/debug output, previews and artifact links | Local Git diffs only | Team Lobby opt-in diff and sharing features | 5 |
| Spectate and shared edit | Read-only follow, scoped edit access, revocation, tested concurrent editing or reviewed patch fallback | Not ported | Team Lobby shared-document prototype | 5, later |
| Agent participants | Human-owned agents with provider, runtime, permissions, task, location, activity, approvals and stop controls | Local agent identities only | Team Lobby agent roster and ACP/MCP integration | 6 |
| AI context and modes | Ask, editable Plan, approved Build; selected files, symbols, diagnostics, terminal/debug results, room context and skills | Assistant UI layout only | Team Lobby Ask/Plan/Build and context tools | 6 |
| AI tools and evidence | Typed native capabilities, bounded outputs, exact approved edits, test results, checkpoints and rollback | Not ported | Team Lobby capability, review and rollback modules | 6 |
| Autonomy and isolation | Per-task worktrees, distinct action grants, stop/reclaim, budgets and execution isolation | Not ported | Team Lobby worktree and approval controls; no reliable OS sandbox | 6 |
| Studio appearance | Curated themes, readable profiles, preview, import/export, contrast and reduced motion | Six themes and preview; profiles and deeper controls absent | Forge Room Studio source and theme bundles | 3, 6 |
| Forge Morph | Prompt-to-typed appearance/layout proposal with preview and undo | Not implemented | Theme/profile primitives only | Later |
| Forge Widgets | Project-aware generated panels with typed data bindings, sandbox and rollback | Not implemented | Product concept only | Later |
| Forge Workshop | Reviewed authoring of commands, tools, widgets and extensions | Not implemented | Product concept only | Later |
| Forge Packs | Portable versioned bundles of appearance, tools, workflows and room templates | Not implemented | Product concept only | Later |
| Skills-first intelligence | Discover, inspect, select and safely use procedural `SKILL.md` knowledge | Not ported | Team Lobby local skill reader | 6, later |
| Forge Sessions | Voice, optional screen share, follow, and controlled shared session actions | Not implemented | Product concept only | Later |
| Forge Pocket | Mobile companion for rooms, approvals and read-only project awareness | Not implemented | Product concept only | Later |
| Forge Mesh / Pipeline | Task/event graph, agent handoffs, bounded context, gated test/deploy pipeline | Not implemented | Task dependency foundation | Later |
| Deadline rescue | Recommendations, owner opt-out, optional narrow preauthorized dispatch | Not ported | Team Lobby local triage/reminders; no production scheduler | Later |
| Conflict and contract awareness | Detect overlapping files/API changes and alert affected owners | Not ported | Limited legacy diff/path awareness | Later |
| Demo and shipping | Authorized preview, readiness evidence, last-hour triage, submission, archive/export | Not ported | Team Lobby readiness and export modules | Later |
| Cross-IDE ecosystem | Capability-tier adapters, CLI/guest clients, truthful sync support | Not implemented | Code OSS extension target only | Later |
| LAN/cloud service | Authenticated encrypted transport, invites, persistence, operations and retention | Not ported | `services/room-service/` self-hosted prototype | 5, later |
| Replay and deferred ideas | Optional activity replay after core; other speculative concepts remain low priority | Not implemented | Product ideas ledger | Later |

**Migration rule:** preserve a legacy behavior only when its user outcome is still wanted. Port its policy or data model into Rust, build the native interface/transport, and test it end to end. Do not move a webview or Node extension host into the Rust runtime merely to claim parity.

## One ordered workflow, split equally

Each row is a paired milestone with one owner on each side. The two owners should swap reviewer/implementer roles on the next row to balance effort. Neither owner closes a row from a build alone; record the observed app or service outcome and remaining limitations here.

| Order | You | Codex | Exit evidence |
|---|---|---|---|
| 1. Establish the native baseline | Run the Windows app and installer on your machine; record launch, open/edit/save, Git, terminal and room behavior, defects, and screen/DPI details. | Verify imported commit/file parity, run Rust tests/lints and packaging checks where available; fix reproducible source defects. | Baseline log with versions, commands, pass/fail, screenshots for UI defects, and measured gaps. |
| 2. Resolve migration boundary | Decide which legacy assets/features must be retained and where the old code should be archived; approve the new primary branch/repo workflow. | Map each legacy feature to native port, replacement, or deferred status; prepare a reviewable cleanup/change set without touching historical docs. | Explicit feature matrix and approved branch/legacy disposition. |
| 3. Make the editor dependable | Exercise real projects: large files, unusual encodings, external file changes, crashes, multiple windows and accessibility. Prioritize failures. | Implement file-change detection/recovery, editor performance work, and focused regression tests. Benchmark egui against candidate native components. | Repeatable startup/memory/large-file measures and successful data-loss checks. |
| 4. Daily IDE loop | Choose representative language/project workflows and test navigation, diagnostics, Git merge, terminal, tasks and debugging. | Build native LSP, settings/keybindings, Git conflict flow, terminal usability and then DAP/tasks in small slices. | Real project acceptance for each shipped capability. |
| 5. Native collaboration | Define a two-machine room scenario, consent rules, account/invite experience and evidence expectations. Test as a second participant. | Port authenticated transport, presence, chat, revocation, shared diffs and reconnection onto `forge-room-core`; keep private defaults. | Two-machine session with disconnect/rejoin, authorization and privacy checks. |
| 6. AI and extension model | Choose the first provider and must-have extension workflows, including acceptable runtime and permission boundaries. Test prompts and approval UX. | Implement a provider-neutral native capability layer; start with declarative themes/grammars/snippets and LSP, then evaluate WASM extensions. | Real provider task with scoped context, approval, attributable edits, rollback and evidence. |
| 7. Release candidate | Run installer, upgrade, uninstall, high-DPI/high-contrast and real-project acceptance; sign off on release notes. | Build reproducible CI artifacts, license/security checks, versioning and release measurements; fix release blockers. | Artifact hashes and a tested Windows release path with stated unsupported features. |

### Phase 1 — establish the native baseline (current)

**Why:** Source-branch claims and a green CI job do not prove the merged app works on your Windows desktop. This phase establishes a reproducible starting point before more code changes.

1. **You prepare a disposable trial.** Note the app commit/version, Windows build, display scale, and project path. Use a sample project without private credentials.
2. **You exercise the editor.** Open a folder and multiple files; edit and save UTF-8 text; close a dirty tab using Save, Discard, and Cancel in separate trials; reopen the file and confirm the right content. Try quick open, command palette, find/replace, file search, and theme preview.
3. **You exercise the other active areas.** On a disposable Git repo, try status, stage, commit, diff, branch, and sync. Run a command in the integrated terminal. Create a local room with a person, agent, task, dependency, evidence, and handoff. Record pass, fail, or not tested for each path.
4. **You trial packaging.** On a disposable Windows user or VM, install, upgrade, launch, and uninstall. Check that a project outside the install directory and saved settings survive upgrade. Note installer UI, shortcuts, DPI, and any warnings.
5. **Codex verifies the source and builds.** Confirm version references and module boundaries; run Rust formatting, checks, tests, and clippy for both crates; trigger the Windows package workflow separately from PR verification; inspect artifact names, checksums, build metrics, launch smoke, and install verification. Fix reproducible defects with targeted tests.
6. **Together close the phase.** Record actual desktop behavior and outstanding defects. The exit condition is a visibly working edit/save loop and fresh installer result, with failures assigned by severity. Packaging smoke alone does not close interactive acceptance.

### Phase 2 — decide what migrates from Code OSS

**Why:** Old Team Lobby and Studio source remains useful as a behavior reference, but the Rust app cannot run those VS Code extensions as-is.

1. **You identify required outcomes.** Read the old feature inventory in `PROJECT_PROGRESS.md` and list the workflows you actually need. Mark each as daily-use essential, collaboration essential, later, or drop. Name exact extension capabilities you depend on, rather than only extension names.
2. **You choose reference scenarios.** Pick one real development project and one two-person room workflow. Decide what the external legacy backup must retain. Keep it intact until every needed source and document is accounted for.
3. **Codex maps the implementation.** For each legacy feature, record its native equivalent, missing domain rule, needed platform adapter, test evidence, and priority. Separate reusable task/sharing rules from webview and Node host code.
4. **Together order the backlog.** Select the first three bounded native slices, each with a user outcome and a checkable exit condition. This phase ends with an explicit feature matrix, not a claim of parity.

### Phase 3 — protect files and measure editor performance

**Why:** Data safety and responsiveness must be established before expanding the editor.

1. **You test difficult files.** Try a large text file, many tabs, a file changed externally while open, Save As, crash/relaunch, non-UTF-8 input, permission errors, and high display scale. Record startup time, idle and loaded memory, responsiveness, and any lost or overwritten edits.
2. **You rank failures.** Mark silent data loss or overwrite as a blocker. Record keyboard and accessibility issues separately from visual preferences.
3. **Codex implements safety first.** Add external-change detection and a conflict choice before overwriting changed content. Cover dirty-buffer, save-failure, and room-state recovery paths with meaningful regression tests.
4. **Codex benchmarks the editor.** Measure representative file sizes and compare Rust editor components only if egui misses agreed thresholds. Record startup, memory, large-file behavior, accessibility, integration cost, and maintenance risk.
5. **Together make the component decision.** Exit only after there is no known silent edit loss, measurements can be repeated, and the editor-core choice is evidence-based.

### Phase 4 — complete the daily IDE loop

**Why:** The native product should support normal coding work before collaboration adds more moving parts.

1. **You define two sample projects and languages.** Write the daily path: open, search, navigate, diagnose, edit, run tests, inspect Git, resolve a conflict, and commit. Specify essential shortcuts and settings.
2. **Codex builds in small slices.** Add native LSP diagnostics/completion/hover/definition; settings and keybinding persistence; terminal scrollback, copy and shell selection; Git conflict handling; task/test execution; then a native DAP path. Keep placeholder tabs labelled until they show live data.
3. **You test each slice.** Include missing tools, failed tests, disconnected remotes, and invalid files, not just success paths.
4. **Together close against real projects.** Both sample projects must complete the agreed path without returning to Code OSS. A unit test or UI placeholder does not satisfy this gate.

### Phase 5 — bring Forge Room collaboration online

**Why:** The local room model is not yet a secure, recoverable multi-machine room.

1. **You define a two-machine trial.** Include host, invited participant, owned agent, task handoff, shared diff, chat, disconnect, and reconnect. Specify default visibility for presence, paths, diffs, saved content, unsaved buffers, terminal output, and agent context.
2. **Codex adds authenticated transport.** Bind network identities to `forge-room-core`; add invite creation, expiry, revocation, presence, synchronized tasks, chat, diff proposals, and reconnection one slice at a time. Validate authorization and recipient visibility before display.
3. **You test both sides.** Try revoked and expired invites, an unauthorized task change, stale state after reconnect, and a mismatched project.
4. **Together record the supported tier.** Exit only with two independent clients completing the scenario and preserving privacy. State whether the service is LAN/self-hosted or production-ready; a local protocol test cannot decide that.

### Phase 6 — connect AI and define extensions

**Why:** The AI panel is layout only, and arbitrary VS Code JavaScript extensions do not run in this Rust process.

1. **You choose one provider and one real task.** Define exactly which files/context may be read, which commands/edits need approval, what evidence is required, and how you will reject or roll back an action. Identify required extension capabilities as theme, grammar, snippet, LSP, DAP, or executable logic.
2. **Codex builds a native capability layer.** Distinguish context read, command execution, file edit, network use, and delegation. Keep grants scoped, visible, attributable, and recoverable.
3. **Codex connects modes incrementally.** Implement Ask, then Plan, then Build with explicit proposed edits, checkpoints, review, and rollback. Add declarative themes/grammars/snippets and native language-server support before evaluating a sandboxed WASM extension API.
4. **You test the approval flow with the real provider.** Include successful execution, denied permission, rejected edit, and rollback. Exit only when the selected task works end to end with correct scope and evidence.

### Phase 7 — prepare a release candidate

**Why:** Artifact creation, CI smoke, and a usable release are different milestones.

1. **Codex builds reproducible artifacts.** Run Windows CI for Rust build, license report, portable archive, installer, checksums, silent install/upgrade/uninstall, and launch smoke. Compare binary, installer, release-note, and tag versions; review signing and permissions.
2. **Codex measures and documents.** Record package and installed size, visible startup, stable idle memory, editor and room workload memory, exact hashes, known limits, and a recovery path to the prior release.
3. **You perform hands-on acceptance.** Install, upgrade, launch, and uninstall on Windows 11 x64; test default and high-DPI displays, keyboard access, high contrast, a real project, and preservation of settings and external files.
4. **Together sign off narrowly.** Release notes must distinguish working features, placeholders, unsupported extension tiers, unsigned status, and platforms not tested. Exit when artifacts and observed Windows behavior support every published claim.

## Working rules

1. **Define the outcome.** Write what the person should be able to do, what must never happen, and the observable success condition. Link the requirement or bug.
2. **Record the starting point.** Note branch, commit, existing behavior, affected files, and related uncommitted work. Preserve historical local edits and the external legacy backup.
3. **Change one bounded slice.** Branch from `main`. Put reusable room policy in `forge-room-core` and UI/platform adapters in `forge-room-app`. Do not mix feature work with historical-document cleanup.
4. **Run focused checks.** Format, check, test, and clippy the affected crate; then use Windows CI. Add tests for policy, persistence, permission boundaries, and real regressions.
5. **Try the actual workflow.** You test a representative Windows project. For network or provider behavior, test two independent clients or the real provider. Capture observed behavior, not only a passing process.
6. **Review and integrate.** Codex records code and test evidence; you review the product result. Merge only when the slice's exit gate is met or clearly label an experimental gap.
7. **Update this ledger.** Record date, owner, branch/commit, commands, Windows observation, limitations, and next owner action. Keep source integrated, automated pass, interactive pass, and multi-client/provider pass distinct.

Keep room and AI execution permissions explicit. Local policy checks do not establish secure multi-machine behavior. Do not claim compatibility with VS Code JavaScript extensions. Keep credentials, tokens, private project files, and room data out of commits and screenshots.

### Verification commands

Run these from the repository root in PowerShell with a Rust toolchain installed. They do not install or uninstall the app. `cargo` was unavailable in the local environment during the repository switch, so use a Rust-capable machine or CI until that changes.

```powershell
cargo fmt --manifest-path native/forge-room-core/Cargo.toml -- --check
cargo check --locked --manifest-path native/forge-room-core/Cargo.toml
cargo test --locked --manifest-path native/forge-room-core/Cargo.toml
cargo clippy --locked --manifest-path native/forge-room-core/Cargo.toml --all-targets -- -D warnings

cargo fmt --manifest-path native/forge-room-app/Cargo.toml -- --check
cargo check --locked --manifest-path native/forge-room-app/Cargo.toml
cargo test --locked --manifest-path native/forge-room-app/Cargo.toml
cargo clippy --locked --manifest-path native/forge-room-app/Cargo.toml --all-targets -- -D warnings

cargo run --release --manifest-path native/forge-room-app/Cargo.toml -- C:\path\to\sample-project
```

The workflow is `.github/workflows/native-rust.yml`. Pull requests run `verify`; `package-windows` runs on its configured push branches, tags, or manual dispatch. Inspect its artifacts before claiming an installer was built. `packaging/windows/verify.ps1` performs silent install/upgrade/uninstall and a non-interactive launch probe; it changes the test user's Forge Room installation, so run it in a disposable test environment after building its expected artifacts. A visible desktop trial remains necessary.

### Evidence entry template

Append one short entry per completed slice or failed gate:

```text
Date / owner / branch / commit:
Outcome and test project:
Commands and CI link:
Observed Windows behavior:
Status: source integrated | automated pass | interactive pass | multi-client/provider pass
Open defects or limits:
Next action and owner:
```

## Source index and reconciliation

- `PROJECT_PROGRESS.md`: historical Code OSS product inventory, delivery evidence and outstanding acceptance gates. Its Code OSS release numbers are **legacy history**, not native version numbers.
- `docs/product/PRODUCT_SPEC.md` and `docs/source-prompts/`: full feature requirements and original briefs.
- `docs/architecture/IMPLEMENTATION_MAP.md`, `THREAT_MODEL.md`: legacy component map and security assumptions; revisit for Rust architecture.
- `docs/planning/CURRENT_STATE.md`, `BUILD_STATUS.md`, `FORGE_AI_NATIVE_EXPANSION_DELIVERY.md`, `FORGE_AI_NATIVE_EXPANSION_RECONCILIATION.md`, `BUILD_PROMPT.md`, `PROTOTYPE_README.md`: prior build and AI delivery details; their current-status claims predate the Rust switch.
- `docs/research/`: audit and competitor research that informs later slices.
- The supplied fork's `project_rust_progress.md` informed this consolidated ledger. Its `docs/native/MIGRATION_COMPATIBILITY.md`, `docs/native/EARLY_PREVIEW_NOTES.md`, and `docs/design/installer-progress.md` are available in this checkout under their original paths.
- `docs/native/` and `docs/design/`: native release, migration, and installer source documents copied from the fork without replacing existing planning documents.
- `COLLAB.md`: imported source-branch agent handoff; this file supersedes its project status and assignments.

## Next checkpoint

1. **You:** perform the Phase 1 visible Windows trial in a disposable project. Record editor, Git, terminal, room, themes, and installer results.
2. **Codex:** run or trigger the separate Windows package job, inspect artifact evidence, and reconcile stale version/status claims in current-facing docs.
3. **Together:** review failures, choose the first bounded fix, and close the Phase 1 gate before Phase 2.
