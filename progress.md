# Forge Room: unified progress and work plan

**Snapshot:** 2 November 2026. **Active direction:** a native Rust IDE for Windows 11 x64. This is the working handoff for both collaborators. Existing documents remain in place as historical evidence and detailed specifications.

## Repository and migration state

- Original repository: `ZayedBH/forge-room-ide`. Its `main` remains the Code OSS handoff at `b669705`. Two native Rust PRs were merged into `rust-pre-release` on 1 November 2026; they did not update `main`.
- Supplied fork: `Atul013/forge-room-ide`. The active Rust source imported here is `native-rust-ide` at `5fadb8f40bf42325e8a9a4e17ec3a59eaa1adea7` (Developer Preview `0.110.0-alpha.1`). The fork also has an open early-preview PR; that branch is older than the imported native branch.
- Imported from `atul/native-rust-ide`: `native/`, `packaging/`, `design-assets/`, `.github/`, `COLLAB.md`, `docs/native/`, and `docs/design/`. The native README is now the entry point. The previous README is preserved at `docs/legacy/README_CODE_OSS.md`; existing documents, `PROJECT_PROGRESS.md`, and planning files remain in the checkout.
- The old `extensions/`, `services/`, `releases/`, and `.build/` directories were moved intact to `C:\Forge-Room-IDE-Legacy-20261102` outside this active checkout. Their source, local edits, untracked files, packages, and embedded documents remain available there. **The active application is `native/forge-room-app`; the Code OSS extension is no longer the product target.**
- The imported native source is currently uncommitted in this checkout. The fork is configured as remote `atul`; `origin` still points to the original repository. Do not publish or merge until the build results are reviewed.

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

The older Code OSS work has a broader collaboration and AI feature inventory, but those features are **not native ports** simply because the legacy source is present. See `PROJECT_PROGRESS.md` for that historical feature ledger and `docs/product/PRODUCT_SPEC.md` for the full intent.

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

## Working rules

1. Make changes on a native feature branch. Keep the imported source commit in the change description and keep historical docs untouched unless both collaborators decide to update them.
2. For each slice, write the expected user outcome, implement it, run focused automated checks, then test the actual Windows app. State the difference between code present, automated pass, and interactive acceptance.
3. Keep room and AI execution permissions explicit. Local policy checks do not establish secure multi-machine behavior. Do not claim extension compatibility with VS Code JavaScript extensions; begin with declarative content and native language servers.
4. Update this file when a milestone changes. Add date, branch/commit, commands, observed result, limitations, and the next owner action. Use source docs for detail, but this is the current status and assignment ledger.

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

Run the baseline checks in milestone 1, inspect the real app on Windows, and record results here. Then prepare the migration boundary decision in milestone 2 before publishing a replacement branch.
