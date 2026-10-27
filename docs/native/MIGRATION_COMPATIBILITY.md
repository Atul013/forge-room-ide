# Native migration and compatibility matrix

**Target identity:** Forge Room Early Preview

**First native version:** `0.109.0-alpha.1`

**Packaging target:** Windows 11 x64 only; every other platform is unsupported until built and tested.

## Implementation choice

The first slice uses reusable native Rust components behind an editor-independent domain boundary. It does not fork or combine the full Zed and Lapce applications. Forge's existing Code OSS extensions remain the behavior reference while domain policies move into testable native crates.

This keeps the migration bounded and avoids inheriting either editor's entire release cadence, product model and UI architecture before Forge-specific task, sharing and evidence behavior is decoupled. A graphical shell/editor component decision remains open until Windows builds compare Floem/Lapce-derived components, GPUI/Zed-derived components and smaller Rust editor primitives against accessibility, high-DPI, startup, memory and maintenance gates.

Primary references used for this decision:

- Zed Windows development: <https://zed.dev/docs/development/windows>
- Lapce source, platform statement and Apache-2.0 license: <https://github.com/lapce/lapce>
- VS Code extension-host architecture: <https://code.visualstudio.com/api/advanced-topics/extension-host>
- VSIX publishing and installation mechanics: <https://code.visualstudio.com/api/working-with-extensions/publishing-extension>
- Zed language and debugger extension prerequisites: <https://zed.dev/docs/extensions/publishing/prerequisites.html> and <https://zed.dev/docs/extensions/debugger-extensions>
- WiX upstream: <https://github.com/wixtoolset>
- `cargo-wix` WiX v3 limitation: <https://github.com/volks73/cargo-wix/issues/302>

## Source-backed behavior matrix

| Capability | Legacy evidence | Native status | Next gate |
|---|---|---|---|
| Human/agent identity and ownership | `extension.ts`, `taskAuthorization.ts` | Ported in `forge-room-core` | Bind authenticated transport identities |
| Task open/in-progress/done lifecycle | `extension.ts` | Ported | Graphical room/task view |
| Host/owner update authorization | `taskAuthorization.ts` | Ported and tested | Multi-client acceptance |
| Dependencies and reopen protection | `taskDependencyPolicy.ts` | Ported and tested | Cycle validation UI |
| Acceptance confirmations | `taskCompletionPolicy.ts` | Ported and tested | Attach check artifacts |
| Owner-requested handoff to connected participants | `taskHandoffPolicy.ts` and host approval flow | Ported and tested with separate host approval | Host approval UX and network race tests |
| Review/check/handoff evidence | extension activity/review flows | Foundation ported | Signed/linked artifact model |
| Recovery-safe local state | host-local history and rollback requirements | New native atomic save/backup recovery | Crash-injection and Windows filesystem tests |
| Scoped sharing | product spec and threat model | Explicit private defaults only | Consent UI, transport and revocation |
| Offline/solo use | product requirement | CLI slice works locally | Native graphical editor loop |
| Editor open/edit/save | Code OSS | Deferred | Select native editor component and verify Windows workflow |
| Git, terminal, tasks/tests | Code OSS + Team Lobby capabilities | Deferred | Native adapters with explicit permissions |
| Rooms/reconnect/invites | Team Lobby protocol 17 | Deferred | Native authenticated encrypted transport |
| Appearance profiles | Forge Room Studio 0.14.0 | Deferred | Native design tokens, contrast and reduced motion |
| AI Ask/Plan/Build | Team Lobby 0.108.0 | Deferred | Provider-neutral native capability registry |
| Installer/prerelease | None for native app | Blocked by missing graphical usable slice and Windows runner | Build, measure and test unsigned Windows 11 x64 package |

## Extension compatibility tiers

| Tier | Early Preview position |
|---|---|
| Themes, snippets and declarative grammars | Candidate for native parsing/import after schema and license checks |
| Language Server Protocol | Planned native process adapter; heavyweight servers remain optional downloads |
| Debug Adapter Protocol | Planned native process adapter |
| Local VSIX inspection/install | May support explicitly compatible declarative tiers; a VSIX archive alone does not make its code compatible |
| Arbitrary VS Code API extension execution | Unsupported. Desktop VS Code extensions rely on a Node.js extension host; silently adding it would violate the strict Rust-only runtime requirement |
| Microsoft Visual Studio Marketplace catalog | Unresolved for a non-Microsoft editor; no scraping, bypass or claim of universal access |
| Open VSX / Forge catalog | Candidate discovery sources after API, license, provenance and moderation review; Open VSX is not represented as the Microsoft Marketplace |

## Packaging gates

Direct modern WiX configuration is the current Windows installer candidate; `cargo-wix` is not selected because its documented path remains coupled to WiX v3. The final engine/version requires a license and maintenance check on the Windows build runner.

Do not produce an installer until all of these are true:

1. Native graphical open, edit and save work on Windows 11 x64.
2. Room/task state survives launch, upgrade and uninstall without deleting user projects/settings.
3. Release build measurements exist for installer payload, installed footprint, launch time and idle/editor/room memory.
4. Install, launch, upgrade and uninstall are actually exercised in a Windows environment.
5. Unsigned status, missing features, limitations and extension tiers are explicit in release notes.
