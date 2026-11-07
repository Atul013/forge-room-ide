# Windows installer: status and how it works

Approved look: `docs/design/installer-concepts/glass-flow-v4/` (opaque left artwork panel, frosted-glass right pane, restrained red primary action).

## Architecture

The setup is a code-built WPF window (`packaging/windows/ember/`) compiled with the .NET Framework `csc` that ships with Windows 11, so there is no new runtime and no XAML build step. It embeds the Inno Setup engine, which performs the real install, upgrade and uninstall registration.

| Piece | Role |
|---|---|
| `ember/Program.cs` | Setup UI: Welcome, License, Options, Progress, Finish; update and uninstall modes; silent pass-through |
| `ember/InstallerPolicy.cs` | Pure logic: argument quoting, destination validation, disk-space gate, version compare |
| `ember/PolicyTests.cs` | 24 checks, run by every build; a failure stops the build |
| `forge-room-early-preview.iss` | Inno engine script (same AppId, per-user install, `{localappdata}\Programs\Forge Room Early Preview`) |
| `build-ember.ps1` | Policy tests, engine compile, host compile, embeds engine/art/license/notes |
| `render-installer-assets.ps1` | Renders the left-panel artwork and icon |
| `package.ps1` | CI packaging; calls `build-ember.ps1` for the final setup |

Build locally: `powershell -File packaging/windows/build-ember.ps1` (output in `artifacts/installer-redesign/`). Needs Inno Setup 6 (`ISCC.exe`, in `artifacts/build-tools/innosetup/` or Program Files). The Windows SDK is not required.

## Behaviour

- **Glass:** acrylic blur-behind (`SetWindowCompositionAttribute`) with a `#121314` tint at 60%. Opaque fallback if unsupported or in high contrast. Over a plain dark desktop it looks dark and smooth; the blur only shows over varied content.
- **Install:** license must be accepted; destination must be a full folder path (no drive roots); free space is read live and Install is blocked if it is too small. Desktop shortcut is opt-in.
- **Update:** the host reads the installed version from the uninstall registration. Older installed version: "Update Forge Room" (X to Y), folder locked to the existing install, a running Forge Room is closed (`/CLOSEAPPLICATIONS`). Same version: Reinstall. Newer installed version: blocked ("A newer version is installed"). Files are replaced in place; user data under `%LOCALAPPDATA%\Forge Room Early Preview` is untouched.
- **Uninstall:** after each install the setup copies itself to `{app}\forge-room-uninstall.exe` and sets Windows' `UninstallString` to it (`/UNINSTALL`). It runs from a temp copy, closes a running Forge Room, runs Inno's `unins000.exe` silently, shows themed confirm/progress/done screens, and removes its own copy and temp folder.
- **Silent / CI:** `/SILENT` and `/VERYSILENT` pass straight through to the engine (all `/KEY=value` arguments are re-quoted correctly). `unins000.exe /VERYSILENT` still works.

## Verified (2026-11-01, Windows 11 x64, 125% DPI, real executables driven with UI Automation)

- Fresh install: Continue disabled until license accepted; `C:\` rejected; valid path installs 5 files plus uninstaller; Finish reached.
- Update 1.1.0 to 1.3.0 with the app running: updated in place, app closed, files replaced, version registered 1.3.0, one uninstall entry, no folder picker. Running the older 1.1.0 setup afterwards is blocked.
- Uninstall from the registered `UninstallString` with the app running: app closed, all files, registry key, themed uninstaller and temp copy removed, no leftover folder.
- Glass: pane pixels vary over a striped backdrop (`#474329`, `#203A22`, `#434344`); left panel stays `#121314`.

## Not verified yet

- CI path (`package.ps1` then `verify.ps1`) after the switch to `build-ember.ps1`: not run here.
- DPI scales other than 125%, high-contrast mode, Narrator, insufficient-disk-space path, cancel mid-flow.
- The payload in `artifacts/installer-redesign/payload/` is now the app built from the working tree (includes the embedded icon), not the earlier `8e20074` artifact.
- Required space is an estimate (payload + installer copy + 4 MiB); Inno re-checks at install time.
