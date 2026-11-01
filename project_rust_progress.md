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
- Logo embedded in the exe (`build.rs`, `assets/forge-room.ico`).
- Toolchain: Rust 1.99.1 (eframe 0.37 deps need 1.95+).

## Roadmap (in order)

1. ~~**IDE shell**~~ done.
2. **Editor core:** ~~line numbers, syntax highlighting, current-line highlight~~ done; still to do: find/replace in file, go to line, bracket matching, indent guides, minimap.
3. **Command palette and search:** Ctrl+Shift+P command palette, Ctrl+P quick open, find in files.
4. **Terminal and Git:** integrated terminal panel, Git status/diff gutter in the explorer and status bar.
5. Later: LSP (diagnostics, completion, go to definition), settings and keybindings, themes, session restore, multi-cursor, split editors, extension story, Room/collaboration UI from the product spec.

## Status log

- 2026-11-01: installer finished and pushed to the `atul` fork; branch renamed to `native-rust-ide`; old Electron-era code removed; step 1 (IDE shell) done.

## Known gaps

- No external-change detection (a file edited on disk while open is not noticed).
- No new-file/rename/delete in the explorer yet; no drag to reorder tabs; no split view.
- No TOML/TypeScript colors (syntect default grammar pack has no grammar for them); unknown extensions render as plain text.
- Large files (egui TextEdit re-lays out the whole buffer) will not scale; this is why step 2 may need a custom editor widget.
- Folder/file dialogs block the UI thread while open.
