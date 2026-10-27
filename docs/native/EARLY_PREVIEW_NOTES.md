# Forge Room Early Preview 0.109.0-alpha.1

This is an installable artifact checkpoint for the first native Rust slices. It is an early preview, not a beta, and is not a published GitHub release.

## Included

- Editor-independent Rust room/task model derived from Team Lobby 0.108.0 policies.
- Human and owned-agent identity shape.
- Host/owner task authorization, dependency gates, acceptance confirmations, controlled reopen and handoff.
- Check/review/handoff evidence plus a bounded attributed activity trail.
- Private-by-default sharing policy.
- Offline JSON state with atomic replacement, previous-state backup and recovery from corrupt primary state.
- CLI workflow for creating a room, adding participants/tasks, changing status, handing off work, recording evidence and listing tasks.
- Native graphical UTF-8 text open/edit/save slice with keyboard save, unsaved-state indication, a high-DPI-aware native window and AccessKit integration.

## Missing

- File picker, multi-file workspace, syntax highlighting, LSP, DAP, Git UI, terminal and task runner.
- Realtime room transport, invitations, encryption and reconnect.
- Theme/profile UI, accessibility and high-DPI acceptance.
- Interactive Windows open/edit/save acceptance, upgrade testing and a signed release.

## Known limitations

- The CLI state file is local and trusted; it is not an authenticated multi-user protocol.
- Activity timestamps use Unix seconds and are not signed or tamper-evident.
- Atomic save/recovery still needs crash-injection and Windows filesystem verification.
- CI compilation and a non-interactive launch probe are not Windows 11 desktop acceptance. The GitHub-hosted runner process exited before the five-second launch probe completed, so no launch-time or memory claim is made.
- Silent per-user installation is exercised by CI. Clean uninstall and launch-probe evidence are emitted with the artifact; upgrade remains untested.
- The build is unsigned.
- The installer is per-user and deliberately does not modify PATH or register file associations; the native app has no stable project-file format yet.
- The portable archive includes a locked dependency inventory and a cargo-about-generated report associating shipped Rust dependencies with their full license texts. Packaging fails closed if a dependency license cannot be determined. Windows artifacts are retained temporarily as CI artifacts and are not a published release.

## Extension tiers

- Declarative themes, snippets and grammars: candidate, not yet implemented.
- LSP and DAP: planned native adapters, not yet implemented.
- Compatible local VSIX inspection: under design; no execution support yet.
- Arbitrary VS Code API extensions: unsupported because they require a JavaScript/Node or web extension host outside the strict Rust-only runtime.
- Microsoft Marketplace catalog: unresolved for a non-Microsoft editor.
- Open VSX or a Forge catalog: candidates only; neither is represented as the Microsoft Marketplace.

## Platform support

Windows 11 x64 is the first packaging target. A Windows CI build, silent install/launch/uninstall smoke test and artifact do not by themselves establish interactive Windows support. Linux and macOS remain unsupported until separately built and tested.

## Measurements from this checkpoint

The Windows artifact bundle includes `BUILD_METRICS.json` with the exact executable, portable archive, installer, prepared-payload and license-report byte counts for that build, plus `SHA256SUMS.txt` for both distributable files. The artifacts are unsigned and require no subsequent payload download. The CI runner did not provide interactive desktop acceptance; launch time, stable idle memory, editor workload memory and room workload memory remain unmeasured.
