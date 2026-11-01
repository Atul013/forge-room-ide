# Forge Room native (Rust) progress

Single place for the state of the native app and its installer. Shared agent hand-off lives in `COLLAB.md`.

## Goal and constraints

- A light, fast IDE built in Rust. **No Electron, no webview, no Node runtime.**
- Aim: cover what people expect from VS Code, delivered in small verified slices.
- Windows 11 x64 first. Per-user install, no admin rights.
- Current UI toolkit: egui/eframe 0.37 (wgpu on Windows). The README flags that this is not a final commitment for the full editor; re-evaluate against Lapce/Floem and Zed/GPUI once the editor core is in (measure memory, startup, large-file scrolling).

## Done

### Installer (commits `be45c68`, `4d3de69`)
- Branded WPF setup host embedding the Inno engine (`packaging/windows/ember/`), frosted-glass right pane, opaque artwork panel, five screens.
- Update flow: older to newer in place, same version reinstall, downgrade blocked, running app closed first.
- Themed uninstaller registered as Windows' `UninstallString`.
- 24 policy checks run on every build. Details: `docs/design/installer-progress.md`.
- Not yet verified: CI path (`package.ps1`/`verify.ps1`), DPI other than 125%, high contrast, low disk space.

### App (`native/forge-room-app`)
- **Step 1, IDE shell (done, verified by driving the real exe):**
  - Menu bar (File, View), activity bar, resizable explorer sidebar with lazy folder tree (folders first, `.git` hidden), Refresh.
  - Multi-file tabs: dirty dot, close button, middle-click close, same file reuses its tab, Ctrl+Tab cycles.
  - Native Open File / Open Folder / Save As dialogs (`rfd`), open a folder or file from the command line.
  - Unsaved-changes prompt on tab close and on window close (Save / Don't Save / Cancel).
  - Status bar: Ln/Col, UTF-8, CRLF/LF, version, last action or error (non-UTF-8 and files over 10 MB are refused with a message).
  - Window title follows the file and folder; shortcuts Ctrl+N/O/Shift+O/S/Shift+S/W/B.
  - 10 unit tests (document, tabs, workspace, position).
- **Step 2, editor core (partly done, verified on screen):**
  - Syntax highlighting per file extension (syntect via egui_extras), line-number gutter aligned to the real text layout, current-line highlight, no soft wrap with horizontal scroll, cursor starts at 1:1.
  - Breadcrumb path bar under the tabs; Git branch in the status bar (read from `.git/HEAD`); file-type colored icons in the explorer.
  - 13 unit tests; rustfmt, clippy `-D warnings` and the cargo-about license gate pass locally.
- **Version control (done, verified against real Git repositories):**
  - Source Control view (Ctrl+Shift+G) with commit box (Ctrl+Enter), staged and unstaged groups, per-file and bulk stage/unstage, change count badge on the activity bar, clickable branch with ahead/behind in the status bar.
  - Click a change to open a read-only, green/red colored diff tab; "Initialize Repository" for plain folders.
  - Gutter change markers (added, modified, deleted) in the editor; refreshes every 3 s, on save and on tab switch.
  - Uses the `git` command line (like VS Code); a clear message appears when git is missing. 16 unit tests including real-repo stage/commit/diff.
- **Themes ported from Forge Room Studio (done, verified on screen):**
  - The six Studio themes (Neon Ronin, Starship Deck, Moss Shrine, Ink & Ember, Paper Studio, Signal Guard) load from their original VS Code theme JSON (`native/forge-room-app/themes/`). Ink & Ember is the default.
  - Whole UI is themed at runtime (panels, bars, tabs, badges, status bar), including a light theme; syntax colors come from each theme's own token rules through a custom highlighter.
  - View > Color Theme: gallery of preview cards, live apply, Keep or Cancel; choice saved to `%LOCALAPPDATA%\Forge Room Early Preview\settings.json` (kept across updates).
  - Tests check every theme still meets 4.5:1 text contrast. 22 unit tests total.
- **Rooms ported from Team Lobby (done, verified by driving the real exe):**
  - Room view (Ctrl+Shift+R, people icon with an open-task badge): create a room per project, people and coding agents (agents must have a human owner), tasks with owner, "done when" criteria and dependencies, Start/Complete/Back/Reopen, handoff request and host approval, evidence, activity feed, "Acting as" selector.
  - All rules (who may update, dependency gates, completion needs every criterion, host-only reopen/approve) come from `forge-room-core`; the view only draws state and forwards intents, and shows the core's error messages.
  - Stored privately per project under `%LOCALAPPDATA%\Forge Room Early Previewooms\` (never inside the project folder), using the core's atomic save with backup recovery.
  - Not ported: realtime relay/invites, chat, anchored comments, live document sessions, ACP/MCP agent clients (they need the old Code OSS/Node extension host).
  - 27 unit tests in the app (5 for room helpers), 7 in the core.
- **Navigation and editing tools (done, verified on screen):**
  - Command Palette (Ctrl+Shift+P) and Quick Open (Ctrl+P) in one box: plain text finds files, `>` runs commands, `:` jumps to a line; fuzzy ranking, arrow keys, Enter. Edit menu and Go to Line (Ctrl+G).
  - Find and Replace in the file (Ctrl+F / Ctrl+H): live match highlighting, current match, n of m counter, next/previous, match case, Replace and Replace All.
  - Search in Files (Ctrl+Shift+F): background search that cancels when you keep typing, skips `.git`, `target`, `node_modules` and binary or large files, highlights the match in each result, click to open at the line.
  - 44 unit tests in the app.
- **Integrated terminal (done, verified with a real shell):** Ctrl+` opens a resizable bottom panel running PowerShell in the project folder over a real Windows pseudo-console (`portable-pty`) with a VT emulator (`vt100`): colors, cursor, arrow keys, Ctrl+C, paste, resize, restart after exit. Needed an answer to ConPTY's cursor-position query before any output appears. Known gaps: one terminal only (no tabs/splits), no scrollback UI, no mouse selection/copy, no shell picker.
- **Full Source Control and built-in GitLens features (done, verified on a demo project with a real remote):**
  - VS Code parity: Sync Changes (pull then push) and Publish Branch with background network jobs, Pull/Push/Fetch, Commit and Commit & Push, amend, undo last commit, stash and pop, discard per file or all (with confirmation), branch switcher (status bar branch) with create-branch and remote checkout, ahead/behind in the status bar.
  - GitLens-style: commit graph with lanes, merges, branch/remote/tag labels and the HEAD ring (click a commit for its full diff), file history for the open file, inline current-line blame (author, time, summary; click for the commit; toggle with the palette command), commit detail tabs.
  - Remote operations tested against a local bare repository in the unit tests (publish, push, fetch, behind/ahead, pull); 54 unit tests.
  - Not yet: line history, tag/remote/contributor trees, merge-conflict resolution UI, stash list view, blame hover card (a tooltip shows today), rebase/cherry-pick, credential prompts beyond what Git Credential Manager shows.
- Logo embedded in the exe (`build.rs`, `assets/forge-room.ico`).
- Toolchain: Rust 1.99.1 (eframe 0.37 deps need 1.95+).

## Roadmap (in order)

1. ~~**IDE shell**~~ done.
2. **Editor core:** ~~line numbers, syntax highlighting, current-line highlight~~ done; find/replace and go to line now done; still to do: bracket matching, indent guides, minimap.
3. ~~Command palette, quick open, find in files~~ done.
4. ~~Git status, diff, stage, commit~~ done; discard, branches, push/pull/fetch, stash, history, blame and the commit graph are now done; still to do: line history, conflict resolution, rebase, tags/remotes views. **Terminal:** ~~integrated terminal~~ done.
5. **Port the features that lived on `main` (Electron/Code OSS era, now rebuilt natively on `forge-room-core`):**
   - **Forge Room Studio:** the six themes (Neon Ronin, Starship Deck, Moss Shrine, Ink & Ember, Paper Studio, Signal Guard high contrast), custom palette, profiles import/export.
   - **Team Lobby:** ~~rooms, participants, tasks, handoffs, evidence, activity~~ done as a native view on `forge-room-core`; still to do: realtime sharing between machines (relay/invites, presence), chat, anchored comments, shared diffs. Not portable as-is: ACP/MCP clients, webview panels and VS Code API workspace tools (they need Code OSS or Node).
6. **Extensions, in the order they are realistic without Node:**
   1. *Declarative* (themes, TextMate grammars, snippets, icon themes, keybindings) installed from Open VSX `.vsix` files. Pure data, no code runs, fits the current engine (syntect already reads TextMate grammars).
   2. *Language servers* as a native LSP client (diagnostics, completion, hover, go to definition): covers most of what language extensions actually do, using the servers' own binaries (rust-analyzer, gopls, ...) with no JavaScript host.
   3. *A Forge extension API in WebAssembly* (the Zed model): new extensions written in Rust or any language that compiles to WASM, sandboxed, no Node.
   4. *Running existing VS Code JavaScript extensions* would need an embedded JS engine plus a re-implementation of the `vscode` API and the Node modules extensions assume (fs, child_process, net, ...). That is a multi-year compatibility project and brings back the weight this app avoids, so it stays out of scope unless a concrete high-value extension justifies a small opt-in host. Microsoft's Marketplace terms also exclude non-Microsoft editors, so the source would be Open VSX.
7. Later: LSP (diagnostics, completion, go to definition), settings and keybindings, themes, session restore, multi-cursor, split editors, extension story, Room/collaboration UI from the product spec.

## Status log

- 2026-11-01: installer finished and pushed to the `atul` fork; branch renamed to `native-rust-ide`; old Electron-era code removed; step 1 (IDE shell) done.

## Known gaps

- No external-change detection (a file edited on disk while open is not noticed).
- No new-file/rename/delete in the explorer yet; no drag to reorder tabs; no split view.
- No TOML/TypeScript colors (syntect default grammar pack has no grammar for them); unknown extensions render as plain text.
- Large files (egui TextEdit re-lays out the whole buffer) will not scale; this is why step 2 may need a custom editor widget.
- Folder/file dialogs block the UI thread while open.
