# Forge Room Early Preview 0.109.0-alpha.1

This is a source checkpoint for the first native Rust slice. It is not yet an installable IDE release and must not be published or presented as a beta.

## Included

- Editor-independent Rust room/task model derived from Team Lobby 0.108.0 policies.
- Human and owned-agent identity shape.
- Host/owner task authorization, dependency gates, acceptance confirmations, controlled reopen and handoff.
- Check/review/handoff evidence plus a bounded attributed activity trail.
- Private-by-default sharing policy.
- Offline JSON state with atomic replacement, previous-state backup and recovery from corrupt primary state.
- CLI workflow for creating a room, adding participants/tasks, changing status, handing off work, recording evidence and listing tasks.

## Missing

- Native graphical shell and editor.
- Open/edit/save, syntax highlighting, LSP, DAP, Git UI, terminal and task runner.
- Realtime room transport, invitations, encryption and reconnect.
- Theme/profile UI, accessibility and high-DPI acceptance.
- Installer, portable archive, checksums and draft prerelease artifacts.

## Known limitations

- The CLI state file is local and trusted; it is not an authenticated multi-user protocol.
- Activity timestamps use Unix seconds and are not signed or tamper-evident.
- Atomic save/recovery still needs crash-injection and Windows filesystem verification.
- CI compilation is not Windows 11 acceptance. No launch, footprint, memory, upgrade or uninstall measurement exists yet.
- The build is unsigned.

## Extension tiers

- Declarative themes, snippets and grammars: candidate, not yet implemented.
- LSP and DAP: planned native adapters, not yet implemented.
- Compatible local VSIX inspection: under design; no execution support yet.
- Arbitrary VS Code API extensions: unsupported because they require a JavaScript/Node or web extension host outside the strict Rust-only runtime.
- Microsoft Marketplace catalog: unresolved for a non-Microsoft editor.
- Open VSX or a Forge catalog: candidates only; neither is represented as the Microsoft Marketplace.

## Platform support

No platform is yet supported as an installable IDE. Windows 11 x64 is the first target. Linux and macOS remain unsupported until separately built and tested.

## Measurements from this checkpoint

The optimized CLI binary built in the Linux cloud runner is 816,288 bytes. This is not an installer size, total payload, graphical-editor footprint, Windows measurement or platform-support claim. Launch time and memory were not recorded because the runner lacks a suitable measurement tool; no values are inferred.
