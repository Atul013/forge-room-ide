# Shared work log: Claude + Codex

This file records the 1 November 2026 early-preview collaboration. Current native ownership and acceptance status are in [`progress.md`](progress.md); the ownership table and in-progress entries below are historical.

Both agents READ this file before starting any task and after finishing each one,
and WRITE to it whenever they start, finish, get blocked, or change direction.
Keep entries short. Newest entry first in "Log". Never delete the other agent's entries.

## Ownership (do not edit files owned by the other agent)

| Agent  | Owns                                                                 |
|--------|----------------------------------------------------------------------|
| Claude | Native app UI `native/forge-room-app/*` AND installer `packaging/windows/*`, `artifacts/installer-redesign/*`, `docs/design/*` (took over from Codex 2026-11-01, Codex out of usage until 9:59 PM) |
| Codex  | `design-assets/forge-room-icons/*`, `design-assets/forge-room-ui-spec/*`, and their delivery ZIPs under `artifacts/` |
| Shared | `native/forge-room-core/*`, `Cargo.lock`, `README.md`: announce here before touching |

## In progress

- **Codex:** screenshot-measured UI handoff complete in `design-assets/forge-room-ui-spec/`; delivery ZIP is `artifacts/forge-room-ui-spec.zip`.
- **Claude (Claude Sonnet 5.5, via Claude Code):** installer DONE (install, update, themed uninstall); see `docs/design/installer-progress.md`. Next: turn the native app into a real IDE (not started).
- **Claude (Claude Sonnet 5.5, via Claude Code):** Native app UI fixes in `native/forge-room-app/src/main.rs`
  (layout clipping, missing margins, Open/Save buttons pushed off-screen).

## Blockers / notes

- RESOLVED: rustc was 1.92 (eframe 0.37 deps need 1.95+); updated to 1.99.1 on 2026-11-01.
- App must stay native Rust (egui/eframe), no Electron/webview.

## Log (newest first)

- 2026-11-06 Codex: PR #21 merged real Git changes/conflict review status into the Room dock. PR #22 merged native dock/terminal tab buttons and a horizontally scrollable Room tab row for narrow widths. Both passed Windows CI; desktop layout and keyboard interaction are still open.

- 2026-11-06 Codex: PR #19 and #20 integrated the saved Aether concept's segmented Room Pulse into the coding workspace and Project Lobby, with local event buttons opening Activity. The default right dock is Team; Lobby setup Review is a native button. Windows CI passed. A portable archive hash matched; an isolated app launched, but the minimized window blocked screenshot/keyboard acceptance.

- 2026-11-06 Codex: PR #18 merged Copy task summary into `main`, with a focused test that excludes another task's evidence. Windows core and graphical CI passed; desktop clipboard trial remains open.

- 2026-11-06 Codex: PR #17 merged Copy visible activity into `main`; it respects the local filter and 10/50 display limit. PR #16 merged blocked-task filtering, unfinished-dependency navigation, and missing-dependency labels. Both passed Windows core and graphical CI; desktop checks remain open.

- 2026-11-06 Codex: PR #15 merged a local Room task owner selector into `main`. It combines with text/status filters and includes owners from saved tasks. Windows CI passed core and graphical checks; desktop interaction remains open.

- 2026-11-03 Codex: started `feat/room-activity-filter` atop PR #12. Activity dock filters local events by actor or detail, with Clear filter and no-match state. Focused match test added; CI and desktop trial pending.
- 2026-11-03 Codex: started `feat/room-activity-history` atop task PR #11. Activity dock shows an empty state and toggles between latest 10 and up to 50 local events. No local Rust toolchain; CI and desktop trial pending.
- 2026-11-03 Codex: started `feat/room-task-filter` from `main`, then stacked it on keyboard-access PR #10. Tasks dock filters by title, owner, or status, with Clear filters and no-match state; new-task title gets focus; expanded tasks show Add evidence. Focused query-match test added. No local Rust toolchain; CI and desktop trial pending.
- 2026-11-03 Codex: E02 keyboard-access PR #10 passed Windows formatting, locked checks, tests, and clippy for both Rust crates. Lobby recovery focus, native task/Lobby controls, and accessible names are source complete; desktop keyboard acceptance remains open.
- 2026-11-02 Codex: audited merged Atul PR #4 and Codex PR #5 on `main`; no unmerged entries or branch divergence. PR #6 restored 149 legacy reference files deleted by PR #4, with exact pre-PR Git blob hashes. Current ownership and remaining acceptance are recorded in `progress.md`.

- 2026-11-01 Codex: completed measured Markdown spec, 11 annotated PNGs, layered SVG/redraw, exhaustive 66,627-color palette, and Rust layout constants. Checked 245 source samples and 86 role samples, SVG/PNG integrity, compiled the Rust helper, and verified all 35 ZIP members. Original font/alpha/blur metadata stays explicitly unavailable; redraw is not pixel identical. App source unchanged.

- 2026-11-01 Codex: started the screenshot-measured Rust/egui handoff; source found in Downloads. Claimed `design-assets/forge-room-ui-spec/*` and its ZIP; no app source edits planned.

- 2026-11-01 Codex: completed 24 path-only icons, symbol sprite, labeled 24/48 px PNG and offline HTML previews. Validated geometry/style/padding, 48 pixel-identical sprite comparisons, and all 34 ZIP members against source bytes. No app source changes.

- 2026-11-01 Codex: started the reference-matched icon delivery; claimed only `design-assets/forge-room-icons/*`, the output ZIP, and temporary render tooling under `artifacts/build-tools/icon-renderer/`.

- 2026-11-01 Claude (Sonnet 5.5): installer finished and committed: update flow (older to newer in place, same version reinstall, downgrade blocked), themed uninstaller, app icon embedded in the exe (`native/forge-room-app/build.rs`), package.ps1 now calls build-ember.ps1, CI path not run. Old Pascal UI and rejected mockups pending deletion (blocked by permissions, user decides).

- 2026-11-01 Claude (Sonnet 5.5): installer WPF host done in `packaging/windows/ember/`; policy tests pass; end-to-end install/uninstall verified via UI Automation; tint tuned to #1D1E1F. Details in `docs/design/installer-progress.md`. Also uninstalled the test installs (interactive-test dir and an orphan default-dir install I made).

- 2026-11-01 Claude (Sonnet 5.5): native app: fixed panel margins and the path row (Open/Save no longer pushed off-screen), editor now in a ScrollArea. rustc updated to 1.99.1; `cargo check` and 2 unit tests pass. Not yet visually checked. Updated `installer-progress.md` with handover.

- 2026-11-01 Claude (Sonnet 5.5): started work; running `rustup update stable` to fix the rustc 1.92 build blocker.
- 2026-11-01 Claude (Sonnet 5.5): created this file. Found app UI has no panel margins, and the
  path TextEdit (`desired_width(INFINITY)`) pushes Open/Save off-screen.
