# Forge Room Early Preview — native app slice

This crate is a real native graphical executable for the bounded single-file open/edit/save milestone. It uses eframe/egui's native desktop integration; it does not embed Electron, Node.js, Python, a browser or a webview.

The shell intentionally accepts a file path in the UI or as its first command-line argument instead of adding a file-dialog dependency before Windows packaging and permission behavior are reviewed.

```text
cargo run --release --manifest-path native/forge-room-app/Cargo.toml -- path/to/file.txt
```

Implemented:

- open and save UTF-8 text files;
- create a new file by entering its target path, editing and saving;
- keyboard save (`Ctrl+S` / `Cmd+S`);
- unsaved-state indication;
- restrained charcoal UI with a muted red accent;
- native high-DPI window handling and eframe's default AccessKit integration;
- unit-tested document open/edit/save and save-as behavior.

Not implemented or accepted yet:

- file picker, binary/non-UTF-8 files, large-file paging, multi-file workspaces, syntax highlighting, LSP/DAP, terminal, Git UI or room/task GUI;
- unsaved-change close confirmation and external-change conflict handling;
- interactive Windows 11 x64 acceptance, launch/memory measurement, installer, upgrade or uninstall;
- code signing or supported-platform status.

eframe 0.37 was selected for this bounded shell because its official native integration supports Windows and enables AccessKit by default. This is not a final commitment to use egui as Forge's full editor component: upstream documents API churn and heavier framework dependencies, so the editor/room UI decision still requires Windows measurements and comparison with Lapce/Floem- and Zed/GPUI-derived options.
