# Atul's Phase 1 slice: what was done and what was observed

Branch `native-rust-ide`, 2 November 2026. Each line says pass, fail or not tested, and what the claim rests on.
Nothing here replaces the hands-on desktop trial that `progress.md` Phase 1 requires.

## Checks run

| Check | Result | Evidence |
|---|---|---|
| `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --locked` for `forge-room-core` | Pass | 7 tests |
| Same three for `forge-room-app` | Pass | 79 tests, 5 ignored (benchmark and real-terminal tests, run by hand) |
| Ignored real-terminal tests (`runs_a_real_command_through_the_pty`, `scrolling_back_reveals_older_output`, `ctrl_c_at_the_prompt_discards_the_typed_line`) | Pass | Run by hand against ConPTY and PowerShell |
| Windows package workflow in CI | Not tested | Needs a push and a workflow run |
| Installer install, upgrade, uninstall on a clean Windows user | Not tested | Needs a disposable user or VM |

## Slices

### E01: external file change warning (done in code)

- `DocumentSession` records the file's on-disk fingerprint on open and save. A normal save is refused with
  `ChangedOnDisk` when the content on disk differs from what the session last saw.
- A rewrite with identical content is not a conflict. A deleted file is not a conflict (saving recreates it).
- The app shows "File changed on disk" with Overwrite, Reload from Disk, Save As and Cancel. A tab with no unsaved
  edits is reloaded silently once a second and the status bar says so.
- Evidence: 3 unit tests in `document.rs` (refuses and overwrites, reload clears the conflict, identical rewrite and
  deletion). The exit condition in the roadmap is a two-process edit; that **has not been tried by hand in the running app**.

### E03: large files (done, decision recorded)

See [EDITOR_PERFORMANCE.md](EDITOR_PERFORMANCE.md). Incremental, time-budgeted highlighting; highlighting off above 2 MiB.
Equivalence of incremental and fresh highlighting is covered by tests. Memory, startup time, many-tab and scroll-rate
measurements were not taken.

### I03 (partial): terminal scrollback, selection and copy

- Mouse wheel and Shift+PageUp/PageDown scroll through the 2000-line history; dragging selects; Ctrl+C copies a selection.
- egui turns Ctrl+C into a `Copy` event and Ctrl+X into `Cut`, which the terminal used to drop. They now map back to
  0x03 and 0x18 when nothing is selected.
- Moved `vt100` from 0.15 to 0.16: 0.15 panics when scrolled back further than one screen.
- **Ctrl+C interrupting a running program: verified in the running app.** Launched through Explorer (so it inherits
  nothing from the agent's shell), typing `ping -n 40 127.0.0.1` and pressing Ctrl+C printed `Control-C`, returned
  the prompt, and the next command ran. The same check inside a test started from the agent's own shell failed
  because that shell passes "ignore Ctrl+C" on to its children, so there is no automated test for it.
- Not done: shell selection, copy of a selection that spans scrollback, paste confirmation for multi-line text.

### I03 (partial): merge conflicts in Source Control

- A conflicted file is shown once, in a "Merge Conflicts" group, instead of under both staged and unstaged changes.
- Right-click offers Accept Current Change or Accept Incoming Change (`git checkout --ours|--theirs`, then `git add`).
  Staging a hand-edited file marks it resolved. Committing is blocked while any conflict is open.
- Staging a conflicted file is refused while `<<<<<<<` and `>>>>>>>` markers remain (message in the status bar).
- Evidence: a unit test against a real merge conflict (both sides), and the whole flow driven in the running app on a real
  repository: right-click, Accept Incoming Change, the file moves to Staged Changes, type a message, Ctrl+Enter, and the
  merge commit appears in the graph with a clean working tree.
- Not done: an in-editor merge view with per-hunk choices; conflict resolution for delete/modify conflicts (`checkout
  --ours` fails when a side has no version of the file and the error is shown as is).

### Bugs found by trying the running app

- **Copy and paste did not work anywhere in the app** (editor, find box, terminal): `eframe` was built with default
  features off and never re-enabled `clipboard`, so Ctrl+C/Ctrl+V never reached the system clipboard. Found while testing
  terminal copy in the running app (the `Copy` event fired with the right text but the clipboard stayed unchanged).
  Enabled `clipboard` and `links` (the latter makes the Lobby's "open repository" link work). Verified: Ctrl+A, Ctrl+C
  in the editor and a mouse selection plus Ctrl+C in the terminal both reach the clipboard. `cargo about` (the packaging
  licence gate) still passes with the added crates. Paste (Ctrl+V) from an outside clipboard also works in the editor
  and the terminal.
- A terminal selection was anchored a few pixels after the press instead of at it (egui reports the drag late).
  It now anchors at the press position.
- The current-line highlight was drawn over the text of the line being edited, hiding it. Fixed (paint order).
- Save As onto an existing file reported a false "changed on disk" conflict and switched the tab to a file it had not
  written. Fixed; tests added.
- At the default 1240 x 780 window the Lobby header ran under the countdown and the right-hand column was clipped.
  The cause was the Room Pulse strip: in egui an overflowing row also widens the parent's maximum rectangle, so
  everything below it laid out 32 px too wide. Pulse now shows only the events that fit; the header, columns, tabs and
  title-bar chips adapt (checked at 1550, 1240, 900 and the 760 x 480 minimum).
- A 1.5 MB Rust file opened and highlighted fully within a few seconds, and typing at the top stayed responsive.
- One run of the full test suite failed `git::tests::stage_commit_and_diff_in_a_real_repository`. It passed on every
  rerun (about 60, including parallel runs), so the cause is unknown. Treat it as a possible timing flake until seen again.

### Independent code review (Codex), all eight findings fixed

Codex reviewed the Rust changes read-only (`design-assets/forge-room-ui-spec/CODE_REVIEW_NOTES.md`), reproducing most
findings with probes. Each fix has a regression test unless noted (3, 6 and 7 share one real-terminal test that is
`#[ignore]`d and run by hand, because it needs a console).

1. Saving trusted an unchanged file size and time, so an outside tool that rewrote the bytes and restored the time
   was overwritten silently. Saves now compare content; `open()` takes the fingerprint before reading.
2. One very long line could stall a frame far past the 12 ms budget. Lines over 4000 bytes are drawn plain.
3. Resizing the terminal with a selection could panic in `contents_between`. The selection is cleared on resize
   and clamped when read.
4. A settings file with huge `start_epoch` or `duration_hours` overflowed the end time. It saturates; duration is capped.
5. Failed lobby saves were silently dropped. Saves are atomic (temp file, then rename), report errors, and retry.
6. Output scrolling under a terminal selection changed what Copy copied. The text is captured when the drag ends.
7. Copying a blank selection sent Ctrl+C to the running program. Only "no selection" interrupts now.
8. After a disk conflict, the Close, Save All or Quit that triggered the save was forgotten. The dialog now remembers
   it and resumes after Overwrite, Reload or Save As; Cancel cancels the whole operation. Verified in the running app
   (edit, change the file outside, close tab, Save, Overwrite: file saved and tab closed). No automated test: it is
   window-state logic.

### Room Lobby and shell (new reference design)

- Added `lobby.rs`: overview, goal, team, agents, milestones, project setup, hackathon configuration, invite, Room Pulse.
- People, agents and activity come from the room state. Goal, milestones, setup and sharing options are local configuration
  saved beside the room file (`<room>.lobby.json`); the core does not model them yet.
- Rebuilt the left rail, title bar and status bar from the reference; adopted the muted ember palette in the default theme.
- Invites do nothing over a network: the Invite button says so. "Content Sharing" checkboxes are stored settings with no effect yet.
- The countdown is opt-in: a new room shows "No timer running" and a Start timer button. Starting it moves the
  milestone dates with it and shows the countdown in the title bar; the corner x stops it. The default mode and tags
  still say "Hackathon"; whether new rooms should default to another kind is a product decision.
- Evidence: screenshots of the running app at 1550 x 975 with an empty room and a populated demo room; compared against the
  reference by Codex (`design-assets/forge-room-ui-spec/LOBBY_DIFF.md`). Interaction behavior beyond clicking tabs and
  checkboxes was not exercised.

### Brand, Code view and Source Control (2-3 November)

- New architectural logo drawn from Codex's flat SVG (`mark.rs`; the off-white frame uses the ink color on light
  themes). The app icon and installer icon are the seven-size `forge-room.ico` from `design-assets/forge-room-brand/`.
- Code view: Room Pulse strip with real events, right dock tabs Team / Agents / Tasks / Activity / AI from room state,
  native Preview pane (finds a localhost address in terminal output or takes a typed one, checks loopback only, opens the
  system browser for http/https; no web view). Verified against a real local server.
- Source Control: Changes and History pages, two-line change rows, graph lanes bounded to 40% of the row.
- Review fixes: T7 (6 findings) and T13 (7 findings) from Codex's independent reviews are fixed with regression tests
  where the input could be tested (literal Git pathspecs, deleted-side conflicts, unreadable-file save refusal, newline-exact
  syntax line limit, IPv6 loopback, shared http(s) validator, lobby edits kept when a switch cannot save).
- Releases: `v0.110.0-alpha.2` published by CI on Atul013 and ZayedBH (unsigned). Codex re-downloaded the assets: checksums
  match and the embedded icons equal the canonical ICO. This is a file check, not an install trial.
- Merged `origin/main` at `0aa3000` (Zayed's Room chat, decisions, snapshot export, project panels): `cargo fmt --check`,
  `clippy -D warnings`, 98 app tests and 10 core tests pass.
- **Still not done (needs a person at a clean Windows user or VM):** installer install / upgrade / downgrade / uninstall
  trial, and the interactive editor, Git and terminal pass. Steps are in `docs/native/PHASE1_TRIAL_CHECKLIST.md`.
  Layout fixes for the narrow Source Control sidebar and the light-theme logo were checked by reading code and unit
  tests, not on screen.

## Not started

I02 LSP, I04 debugger, R01/R04 authenticated transport, A01/A04 execution boundaries, L01 release engineering.
