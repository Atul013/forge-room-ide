# Shared work log: Claude + Codex

Both agents READ this file before starting any task and after finishing each one,
and WRITE to it whenever they start, finish, get blocked, or change direction.
Keep entries short. Newest entry first in "Log". Never delete the other agent's entries.

## Ownership (do not edit files owned by the other agent)

| Agent  | Owns                                                                 |
|--------|----------------------------------------------------------------------|
| Claude | Native app UI `native/forge-room-app/*` AND installer `packaging/windows/*`, `artifacts/installer-redesign/*`, `docs/design/*` (took over from Codex 2026-11-01, Codex out of usage until 9:59 PM) |
| Codex  | Nothing until it is back; claim files here before editing                 |
| Shared | `native/forge-room-core/*`, `Cargo.lock`, `README.md`: announce here before touching |

## In progress

- **Codex:** paused (usage limit). Left the installer mid-pivot; see `docs/design/installer-progress.md`.
- **Claude (Claude Sonnet 5.5, via Claude Code):** installer DONE (install, update, themed uninstall); see `docs/design/installer-progress.md`. Next: turn the native app into a real IDE (not started).
- **Claude (Claude Sonnet 5.5, via Claude Code):** Native app UI fixes in `native/forge-room-app/src/main.rs`
  (layout clipping, missing margins, Open/Save buttons pushed off-screen).

## Blockers / notes

- RESOLVED: rustc was 1.92 (eframe 0.37 deps need 1.95+); updated to 1.99.1 on 2026-11-01.
- App must stay native Rust (egui/eframe), no Electron/webview.

## Log (newest first)

- 2026-11-01 Claude (Sonnet 5.5): installer finished and committed: update flow (older to newer in place, same version reinstall, downgrade blocked), themed uninstaller, app icon embedded in the exe (`native/forge-room-app/build.rs`), package.ps1 now calls build-ember.ps1, CI path not run. Old Pascal UI and rejected mockups pending deletion (blocked by permissions, user decides).

- 2026-11-01 Claude (Sonnet 5.5): installer WPF host done in `packaging/windows/ember/`; policy tests pass; end-to-end install/uninstall verified via UI Automation; tint tuned to #1D1E1F. Details in `docs/design/installer-progress.md`. Also uninstalled the test installs (interactive-test dir and an orphan default-dir install I made).

- 2026-11-01 Claude (Sonnet 5.5): native app: fixed panel margins and the path row (Open/Save no longer pushed off-screen), editor now in a ScrollArea. rustc updated to 1.99.1; `cargo check` and 2 unit tests pass. Not yet visually checked. Updated `installer-progress.md` with handover.

- 2026-11-01 Claude (Sonnet 5.5): started work; running `rustup update stable` to fix the rustc 1.92 build blocker.
- 2026-11-01 Claude (Sonnet 5.5): created this file. Found app UI has no panel margins, and the
  path TextEdit (`desired_width(INFINITY)`) pushes Open/Save off-screen.
