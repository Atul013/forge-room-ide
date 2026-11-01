# Forge Room

A light, fast, native IDE written in Rust. No Electron, no webview, no bundled Node runtime.

**Status:** early preview (`0.109.0-alpha.1`), Windows 11 x64, unsigned. The goal is to grow it toward the everyday feature set people expect from VS Code, in small verified steps. Progress and roadmap: [project_rust_progress.md](project_rust_progress.md).

## What works today

- Open, edit and save UTF-8 text files, with unsaved-change tracking.
- A per-user Windows installer with in-place updates and a themed uninstaller.

The IDE shell (explorer, tabs, status bar), syntax highlighting, command palette, search, terminal and Git are next; see the roadmap.

## Build and run

Requires Rust 1.95 or newer.

```powershell
cargo run --release --manifest-path native/forge-room-app/Cargo.toml -- path\to\file.txt
cargo test --manifest-path native/forge-room-app/Cargo.toml
```

## Build the Windows installer

```powershell
powershell -File packaging/windows/build-ember.ps1
```

Needs Inno Setup 6 (`ISCC.exe`). The setup is a WPF host compiled with the .NET Framework compiler that ships with Windows, embedding the Inno engine. Details and verification notes: [docs/design/installer-progress.md](docs/design/installer-progress.md).

## Repository map

```text
native/
  forge-room-app/   The graphical app (egui/eframe)
  forge-room-core/  Shared core library and persistence
packaging/windows/  Installer: WPF setup host, Inno engine script, build scripts
docs/
  native/           Early preview notes and migration compatibility
  design/           Approved installer mockups and installer status
COLLAB.md           Shared hand-off log between coding agents
project_rust_progress.md   Status and roadmap
```

## License

MIT. See [native/LICENSE-MIT](native/LICENSE-MIT).
