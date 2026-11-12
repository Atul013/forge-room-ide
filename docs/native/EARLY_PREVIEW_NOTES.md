# Forge Room Early Preview 0.110.0-alpha.3

**Developer Preview: for developers and testers only.** The public pre-release is still on its way, so expect rough edges, missing features and breaking changes between builds.

This is an early preview of the native Rust IDE: not a beta, unsigned, and Windows 11 x64 only.

## New in alpha.3

- **AI agent panel** (Ctrl+Shift+A, or the first tab of the right dock): runs Claude Code or Codex, and local models through Codex (Ollama, LM Studio), in the open folder. Your message goes to the agent program installed on this computer on its standard input; Forge Room itself sends nothing anywhere, but that program sends the prompt to its provider (or to your local model). Answers and the tools the agent uses stream into the panel and are drawn as Markdown; a model and an effort level can be picked (Claude Code: default, Fable, Opus, Sonnet, Haiku or any name; effort low to max), and the choices are remembered. Modes: Read only, Edit files, Full access (asks to be confirmed, never remembered). Claude Code conversations continue between messages (New chat starts fresh; Codex starts fresh each time). Stop ends the agent and everything it started. Sign in opens a terminal that runs `claude auth login` or `codex login`. A run from here cannot answer permission questions, so commands need Full access. Not a replacement for review: look at what the agent changed in Source Control.
- **Folder trust:** Git, cargo check, the sidebar's project commands and the AI agent only run in a folder you trust. Opening a new folder asks once (trusting a folder also trusts the folders inside it); until then it opens in a restricted mode. Folder: Trust This Folder asks again. After updating, each folder asks once, because the list of trusted folders starts empty.
- Terminals: split view, maximize, a VS Code-style header (new, split, kill, more, maximize, close), Terminal: Copy All Output, and the same number of fresh shells comes back when the app starts again. View: Reset Layout puts the panels back to their default size.
- Opening a subfolder of a bigger Git repository works (Git runs from the repository's top folder), and opening another folder closes the saved tabs of the old one.
- Files keep their line endings (LF, CRLF; mixed files are kept byte for byte), UTF-8 BOM and UTF-16 LE/BE encoding when saved, and the status bar shows the real encoding. Saves are atomic.
- Editor typing: Enter keeps the indentation and opens bracket blocks, Tab and Shift+Tab indent and unindent, brackets and quotes are completed in pairs (not in prose files, and not a single quote in Rust), and Ctrl+/ comments lines, Alt+Up/Down moves them, Shift+Alt+Up/Down duplicates them, Ctrl+Shift+K deletes them.
- Find and Search in Files: whole word and regular expressions; invalid patterns are reported. Search has a Replace with box and Replace All (literal text, with a confirmation): each file keeps its encoding and line endings, files open with unsaved changes are skipped and listed.
- Quick Open and Search in Files follow `.gitignore` in a repository.
- File: Toggle Auto Save (off by default) saves file tabs once you stop typing for a moment; a file that changed on disk is left for the usual conflict prompt.
- Explorer keyboard: Ctrl+Shift+E (also from a text box) or a click gives the tree the keyboard; Up, Down, Home, End move, Right opens a folder and steps into it, Left closes it and steps out, Enter opens a file or toggles a folder, F2 renames and Delete moves to the Recycle Bin (after the usual question).
- Explorer: Copy relative path and Reveal in File Explorer (also palette commands for the active file); rename files and folders; Delete… moves them to the Recycle Bin (fixed local drives only, never links, never the open folder; saved tabs close, tabs with unsaved changes stay). Tabs: Ctrl+Shift+T reopens the last closed tab; View: Close Saved Tabs. File > Open Recent.
- Source Control: merge a branch, abort a merge, delete a branch, apply (and keep) or drop the latest stash, create and delete tags from the command palette; Changes and History pages; Git status is read on a worker thread.
- Syntax colours for TypeScript, JSX, TOML, INI and dotenv files (borrowed from the closest bundled syntax).
- Editing commands: Ctrl+D (select next occurrence), palette commands for upper/lower case, sorting and de-duplicating lines, trimming trailing whitespace and joining lines; Ctrl+= / Ctrl+- / Ctrl+0 for the editor font size (remembered); Ctrl+Shift+\ jumps to the matching bracket (and a palette command selects up to it), counting only the same kind of bracket and not skipping strings or comments.
- Go to Symbol in File (Ctrl+Shift+Y, or @ in the palette): an outline from simple per-language line patterns for Rust, Python, JavaScript/TypeScript, Go, Java-family languages, Markdown, shell, TOML and INI. It is not a language server: it finds what is written at the start of a line.
- Source Control: right-click a coloured change bar in the editor gutter to stage or revert just that change.
- Starting the app with nothing to open reopens the last folder and the files that were open (the front tab stays in front); files and folders that are gone are skipped. Unsaved text is the job of crash recovery.
- Crash recovery: unsaved text is written to a recovery folder every few seconds and restored as unsaved tabs after an abnormal exit; a normal exit clears it.
- Problems: Rust: Check Project (or Run cargo check in the Problems tab) runs `cargo check` on a worker thread (the Cargo.toml in the folder, or a workspace one or two levels below) and lists the compiler errors and warnings; click one to open the file at that line. Not a language server: the list is as old as the last run.
- Terminal keys and paste for coding agents: Shift+Enter or Alt+Enter insert a new line, Alt+letter, Ctrl+Arrow, Ctrl+Backspace and F1-F12 are sent the way xterm does, multi-line paste is bracketed when the program asks for it, and COLORTERM=truecolor is set. Quitting asks first when shells are still running.
- Terminal: Terminal: Copy All Output copies the whole scrollback (6000 lines), the same number of fresh shells reopens when the app starts again, and terminals nobody is looking at no longer redraw the window for every chunk of output.
- Terminal: more than one (Terminal: New Terminal, Ctrl+Shift+`, or the + in the panel; numbered chips switch between them, the bin kills the shown one and the cross hides the panel). Ctrl+click a `file:line:column` reference in the output to open it (network and device paths are never followed).
- Source Control: right-click a commit in the History graph to cherry-pick or revert it; merge, cherry-pick, revert and a rebase started elsewhere show a banner with an Abort button.

## Included

- **Earlier, in alpha.2:** the new architectural Forge Room logo and icon; a Room Lobby page (goal, team, agents, milestones, project setup, invite and Room Pulse; the countdown is opt-in); a labeled left rail, new title and status bars and a muted-ember palette; a Room Pulse strip over the workspace; Team, Agents, Tasks and Activity tabs in the right panel, all from real room data; a Preview pane that finds a local dev-server address in the terminal and opens it in your browser (no embedded web view); saving over a file changed on disk now asks first, and clean tabs reload on their own; faster incremental syntax highlighting (plain text above 2 MiB or when a line is longer than 4000 bytes); terminal scrollback, selection and copy; a Merge Conflicts group in Source Control with Accept Current/Incoming; copy and paste now work everywhere in the app.
- Native Rust IDE (egui, no Electron or Node): icon navigation rail, Explorer with a lazy folder tree, multi-file tabs with breadcrumbs, command palette and quick open, find/replace, go to line, search in files, syntax highlighting with line numbers, and unsaved-changes prompts.
- Integrated terminal on a real Windows pseudo-console (PowerShell).
- Source Control: stage/unstage, commit, amend, undo last commit, discard, stash, branches, Sync/Publish/Pull/Push/Fetch, colored diffs, plus GitLens-style commit graph, file history and inline blame.
- Rooms: people and coding agents with human owners, tasks with dependencies and acceptance, handoffs, evidence and activity, built on the Rust room/task model derived from Team Lobby 0.108.0 policies. Stored privately per project.
- Forge Room Studio color themes (Forge Room is the default) and a reference-based redesign with a custom title bar, right dock and bottom panel.
- AI agent panel: see the first item of "New in alpha.3". Claude Code or Codex can also be run in the integrated terminal, as before.
- A per-user installer that updates in place (older to newer; a downgrade is blocked in the setup window) and a themed uninstaller. Uninstalling keeps your data under `%LOCALAPPDATA%\Forge Room Early Preview` (settings, session, room data and any unsaved text a crash left behind).

## Missing

- Run/Debug, Extensions, Debug Console and Ports are placeholders (Problems lists cargo check output only).
- Editor-level AI actions (inline edits, plan/build flows, an approval UI for agent commands), token-by-token streaming, Codex follow-up conversations, image paste; LSP and DAP, extensions, a task runner.
- Rebase, line history, minimap, multi-cursor and split editors.
- Explorer rows cannot be reached with Tab (the arrow keys work once the tree is clicked or opened with Ctrl+Shift+E); there is no word wrap.
- Realtime room transport, invitations, encryption and reconnect.
- Theme/profile import, accessibility and high-DPI acceptance.
- A signed release.

## Known limitations

- The CLI state file is local and trusted; it is not an authenticated multi-user protocol.
- Activity timestamps use Unix seconds and are not signed or tamper-evident.
- Atomic save and crash recovery are implemented but have not been verified by crash injection on real Windows filesystems.
- CI compilation and a non-interactive launch probe are not Windows 11 desktop acceptance. The Windows build uses eframe's WGPU renderer because the hosted runner has no usable OpenGL 2.0 context.
- Silent per-user installation is exercised by CI. The workflow installs a 0.109.0-alpha.0 baseline with the same stable AppId, upgrades to the version being built, verifies application-data and external-project preservation, checks default shortcut behavior, and then performs a clean uninstall. Machine-readable install and launch evidence are emitted with the artifact.
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
