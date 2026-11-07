# Forge Room Early Preview 0.110.0-alpha.2

**Developer Preview: for developers and testers only.** The public pre-release is still on its way, so expect rough edges, missing features and breaking changes between builds.

This is an early preview of the native Rust IDE: not a beta, unsigned, and Windows 11 x64 only.

## Included

- **New in alpha.2:** the new architectural Forge Room logo and icon; a Room Lobby page (goal, team, agents, milestones, project setup, invite and Room Pulse; the countdown is opt-in); a labeled left rail, new title and status bars and a muted-ember palette; a Room Pulse strip over the workspace; Team, Agents, Tasks and Activity tabs in the right panel, all from real room data; a Preview pane that finds a local dev-server address in the terminal and opens it in your browser (no embedded web view); saving over a file changed on disk now asks first, and clean tabs reload on their own; faster incremental syntax highlighting (plain text above 2 MiB or when a line is longer than 4000 bytes); terminal scrollback, selection and copy; a Merge Conflicts group in Source Control with Accept Current/Incoming; copy and paste now work everywhere in the app.
- Native Rust IDE (egui, no Electron or Node): icon navigation rail, Explorer with a lazy folder tree, multi-file tabs with breadcrumbs, command palette and quick open, find/replace, go to line, search in files, syntax highlighting with line numbers, and unsaved-changes prompts.
- Integrated terminal on a real Windows pseudo-console (PowerShell).
- Source Control: stage/unstage, commit, amend, undo last commit, discard, stash, branches, Sync/Publish/Pull/Push/Fetch, colored diffs, plus GitLens-style commit graph, file history and inline blame.
- Rooms: people and coding agents with human owners, tasks with dependencies and acceptance, handoffs, evidence and activity, built on the Rust room/task model derived from Team Lobby 0.108.0 policies. Stored privately per project.
- Forge Room Studio color themes (Forge Room is the default) and a reference-based redesign with a custom title bar, right dock and bottom panel.
- AI Assistant panel: layout only. No model is connected, and nothing you type there leaves your computer. Claude Code or Codex can be run in the integrated terminal.
- A per-user installer that updates in place (older to newer, downgrades blocked) and a themed uninstaller.

## Missing

- Run/Debug, Extensions, Output, Problems, Debug Console and Ports are placeholders.
- AI features (model-backed chat, editor actions), LSP and DAP, extensions, a task runner.
- Rebase, line history, minimap, bracket matching, multi-cursor and split editors.
- File create/rename/delete in the explorer.
- Realtime room transport, invitations, encryption and reconnect.
- Theme/profile import, accessibility and high-DPI acceptance.
- A signed release.

## Known limitations

- The CLI state file is local and trusted; it is not an authenticated multi-user protocol.
- Activity timestamps use Unix seconds and are not signed or tamper-evident.
- Atomic save/recovery still needs crash-injection and Windows filesystem verification.
- CI compilation and a non-interactive launch probe are not Windows 11 desktop acceptance. The Windows build uses eframe's WGPU renderer because the hosted runner has no usable OpenGL 2.0 context.
- Silent per-user installation is exercised by CI. The workflow installs an alpha.0 baseline with the same stable AppId, upgrades to alpha.1, verifies application-data and external-project preservation, checks default shortcut behavior, and then performs a clean uninstall. Machine-readable install and launch evidence are emitted with the artifact.
- The build is unsigned.
- The installer is per-user and deliberately does not modify PATH or register file associations; the native app has no stable project-file format yet.
- The portable archive includes a locked dependency inventory and a cargo-about-generated report associating shipped Rust dependencies with their full license texts. Packaging fails closed if a dependency license cannot be determined. Windows artifacts are also retained as CI artifacts.

## Extension tiers

- Declarative themes, snippets and grammars: candidate, not yet implemented.
- LSP and DAP: planned native adapters, not yet implemented.
- Compatible local VSIX inspection: under design; no execution support yet.
- Arbitrary VS Code API extensions: unsupported because they require a JavaScript/Node or web extension host outside the strict Rust-only runtime.
- Microsoft Marketplace catalog: unresolved for a non-Microsoft editor.
- Open VSX or a Forge catalog: candidates only; neither is represented as the Microsoft Marketplace.

## Platform support

Windows 11 x64 is the first packaging target. A Windows CI build, silent install/launch/uninstall smoke test and artifact do not by themselves establish interactive Windows support. Linux and macOS remain unsupported until separately built and tested.

## Measurements from this checkpoint

The Windows artifact bundle includes `BUILD_METRICS.json` with the exact executable, portable archive, installer, prepared-payload and license-report byte counts for that build, `SHA256SUMS.txt` for both distributable files, `LAUNCH_SMOKE.json` for the non-interactive process probe and `INSTALL_VERIFY.json` for upgrade/preservation evidence. The artifacts are unsigned and require no subsequent payload download. The CI runner did not provide interactive desktop acceptance; visible-window startup time, stable idle memory, editor workload memory and room workload memory remain unmeasured.
