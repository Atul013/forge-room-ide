# Forge Room Early Preview — native core

This is the first editor-independent Rust slice extracted from the existing Team Lobby task policies. It is deliberately additive: the Code OSS product and extensions remain untouched as the behavior reference.

Version `0.109.0-alpha.1` follows Team Lobby `0.108.0` and identifies an early preview, not a beta or complete IDE.

## Included

- Native Rust room state, human/agent identities, owned tasks and bounded activity.
- Host/owner authorization, connected-owner assignment, dependency gates, acceptance confirmations and controlled handoff/reopen.
- Review/check/handoff evidence records.
- Private-by-default sharing flags for paths, diffs, unsaved buffers and terminal output.
- Pretty JSON state with atomic replacement, previous-snapshot backup and corrupt-primary recovery.
- A small offline CLI that exercises create, join, task, handoff, evidence and list flows.

The crate does not use Electron, Node.js, Python, a browser runtime or a webview.

## Run

```text
cargo run --manifest-path native/forge-room-core/Cargo.toml -- init room.json demo host "Atul"
cargo run --manifest-path native/forge-room-core/Cargo.toml -- join room.json host alice "Alice" human
cargo run --manifest-path native/forge-room-core/Cargo.toml -- add room.json host task-1 "Port task policy" alice "Alice" "tests pass"
cargo run --manifest-path native/forge-room-core/Cargo.toml -- status room.json alice task-1 in-progress
cargo run --manifest-path native/forge-room-core/Cargo.toml -- evidence room.json alice task-1 check "cargo test passed"
cargo run --manifest-path native/forge-room-core/Cargo.toml -- status room.json alice task-1 done "tests pass"
cargo run --manifest-path native/forge-room-core/Cargo.toml -- list room.json
```

An unfinished task owner can request a handoff, but the host must approve it:

```text
cargo run --manifest-path native/forge-room-core/Cargo.toml -- handoff-request room.json alice task-1 bob
cargo run --manifest-path native/forge-room-core/Cargo.toml -- handoff-approve room.json host task-1
```

## Not yet included

This is not yet a graphical editor and is not packaged as an installer. Editing, file open/save, LSP, DAP, terminal, Git UI, realtime transport, invites, cryptography, themes and extension loading remain outside this slice. A Windows installer must not be produced until a native graphical open-edit-save loop exists and can be built and tested on Windows 11 x64.
