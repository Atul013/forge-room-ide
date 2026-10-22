# Team Lobby IDE

Forge Room Team Lobby adds multiplayer project rooms and scoped AI workspace capabilities to Code OSS.

## AI workspace capabilities

Ask, Plan, and Build can use typed tools for selected-workspace file inventory, text search, named-file reads, Git status and per-file diffs, diagnostics, symbol search, test task discovery, debugger frames, and explicitly approved terminal output. With separate per-call confirmation, `editor_open` opens a non-sensitive project file and optionally reveals a requested line without sending file contents. Build can run an exact registered test task and apply a reviewed exact-match file edit. Direct edits save a rollback checkpoint before application.

The file inventory returns at most 250 workspace-relative paths and no file contents. Sensitive paths and common generated folders are excluded. Workspace tools are limited to the selected project root, bounded in output, and recorded as content-free local evidence. Mutating actions and sensitive context use separate approvals.

## Collaboration

Team Lobby supports room identity and presence, invites and revocation, tasks and handoffs, room chat and notes, anchored comments, selected Git diff sharing, consented live document sessions, opt-in file-and-line follow, deadlines, activity history export, and optional self-hosted room-service relay.

The room relay does not persist active room content. ACP and MCP local subprocesses run with the user's operating-system privileges; selecting them does not sandbox their access.

## Development

From this directory, run `npm.cmd run check` and `npm.cmd test`. The extension's automated suite verifies policy, formatter, and process-level room behavior; it does not replace live provider, separate-IDE, or multi-machine acceptance.
