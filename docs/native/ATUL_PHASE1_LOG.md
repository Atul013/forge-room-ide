# Atul's Phase 1 slice: what was done and what was observed

Branch `native-rust-ide`, 2 November 2026. Each line says pass, fail or not tested, and what the claim rests on.
Nothing here replaces the hands-on desktop trial that `progress.md` Phase 1 requires.

## Checks run

| Check | Result | Evidence |
|---|---|---|
| `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --locked` for `forge-room-core` | Pass | 7 tests |
| Same three for `forge-room-app` | Pass | 66 tests, 4 ignored (benchmark and real-terminal tests, run by hand) |
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

### Room Lobby and shell (new reference design)

- Added `lobby.rs`: overview, goal, team, agents, milestones, project setup, hackathon configuration, invite, Room Pulse.
- People, agents and activity come from the room state. Goal, milestones, setup and sharing options are local configuration
  saved beside the room file (`<room>.lobby.json`); the core does not model them yet.
- Rebuilt the left rail, title bar and status bar from the reference; adopted the muted ember palette in the default theme.
- Invites do nothing over a network: the Invite button says so. "Content Sharing" checkboxes are stored settings with no effect yet.
- The countdown starts when a project's lobby is first created and counts down a 24-hour box. Whether new rooms should start
  a timer automatically is a product decision for Zayed and Atul.
- Evidence: screenshots of the running app at 1550 x 975 with an empty room and a populated demo room; compared against the
  reference by Codex (`design-assets/forge-room-ui-spec/LOBBY_DIFF.md`). Interaction behavior beyond clicking tabs and
  checkboxes was not exercised.

## Not started

I02 LSP, I04 debugger, R01/R04 authenticated transport, A01/A04 execution boundaries, L01 release engineering.
