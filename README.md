# Forge Room

A light, fast, native IDE written in Rust. No Electron, no webview, no bundled Node runtime.

**Status:** early preview (`0.110.0-alpha.3`), Windows 11 x64, unsigned. The goal is to grow it toward the everyday feature set people expect from VS Code, in small verified steps. Progress and roadmap: [progress.md](progress.md).

## What works today

- IDE shell: folder explorer (with rename), multi-file tabs (reopen a closed tab, close saved tabs), recent folders, native dialogs, and unsaved-change prompts. A file changed by another program is reloaded when you have no edits, and saving over it asks first.
- Safe editing: files keep their line endings, BOM and UTF-16 encoding; saves are atomic (a crash never leaves a half-written file) and unsaved text is kept in a recovery folder and brought back after a crash; Enter keeps the indentation, Tab and Shift+Tab indent lines, brackets and quotes are completed in pairs, Ctrl+/, Alt+Up/Down, Shift+Alt+Up/Down and Ctrl+Shift+K comment, move, duplicate and delete lines, Ctrl+D selects the next occurrence, palette commands change case, sort and de-duplicate lines, trim trailing whitespace and join lines, Ctrl+Shift+Y goes to a symbol in the file, and Ctrl+= / Ctrl+- / Ctrl+0 change the editor font size.
- Line numbers, syntax highlighting (incremental, so large files stay responsive; TypeScript, JSX, TOML and similar files borrow the closest bundled syntax), command palette, quick open and file search that follow `.gitignore`, find/replace with match case, whole word and regular expressions, and an integrated terminal with scrollback, selection, copy and Ctrl+click on `file:line:column` references to open them in the editor.
- Source Control with staging, commits, branches, merging, cherry-pick and revert from the History graph (with abort), staging or reverting a single change from the editor gutter, branch deletion, synchronization, diffs, history, blame, and merge-conflict resolution (accept current or incoming). Git is read on a worker thread, so a big repository does not freeze the window.
- Rooms: local saved tasks/chat remain private. Network Rooms add explicit TLS host/join, expiring/revocable invites, reconnect, private conversations, shared tasks/handoffs/timing and recipient-only saved patches. Protocol/process tests passed; visible desktop and two-machine LAN acceptance remain open.
- AI panel (Ctrl+Shift+A): installed Claude Code/Codex agents remain available, alongside native text model connections for Chat Completions-compatible endpoints and Anthropic Messages. Native requests show the complete prompt content, endpoint, model and limits for approval before transmission; API keys come from named environment variables. Ask/Plan and explicit reviewed edits share stale-file guards, Apply, undo and durable recovery. Native HTTP has no tools or automatic workspace access. Git, project tools and AI require folder trust. See [native model connections](docs/native/NATIVE_MODEL_CONNECTIONS.md) for supported behavior and unverified live-provider gates.
- Several terminals with split view, maximize and a VS Code-style header. Choose an installed shell profile for each new terminal; the default choice is saved, and restarting an individual terminal keeps its selected shell. See [terminal shell profiles](docs/native/TERMINAL_SHELLS.md).
- Six appearance themes and a per-user Windows installer with in-place updates and a themed uninstaller.

Current gaps and the shared work plan are in [progress.md](progress.md).

## Build and run

Requires Rust 1.95 or newer.

```powershell
cargo run --release --manifest-path native/forge-room-app/Cargo.toml -- path\to\file.txt
cargo test --manifest-path native/forge-room-app/Cargo.toml
```

## Build the Windows installer

```powershell
powershell -File packaging/windows/build-ember.ps1
```

Needs Inno Setup 6 (`ISCC.exe`). The setup is a WPF host compiled with the .NET Framework compiler that ships with Windows, embedding the Inno engine. Details and verification notes: [docs/design/installer-progress.md](docs/design/installer-progress.md).

## Repository map

```text
native/
  forge-room-app/   The graphical app (egui/eframe)
  forge-room-core/  Shared core library and persistence
packaging/windows/  Installer: WPF setup host, Inno engine script, build scripts
docs/
  native/           Early preview notes and migration compatibility
  design/           Approved installer mockups and installer status
COLLAB.md           Shared hand-off log between coding agents
progress.md   Status and roadmap
```

## License

MIT. See [native/LICENSE-MIT](native/LICENSE-MIT).
