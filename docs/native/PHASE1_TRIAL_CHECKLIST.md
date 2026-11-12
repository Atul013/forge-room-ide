# Phase 1 hands-on trial (Atul's track): installer, editor, Git, terminal

Use **Forge Room Early Preview 0.110.0-alpha.3**: https://github.com/Atul013/forge-room-ide/releases/tag/v0.110.0-alpha.3
Record each step as **pass**, **fail** or **not tested**, with notes, in `docs/native/ATUL_PHASE1_LOG.md`. Do not mark Phase 1 closed on automated checks alone.

## 0. Prepare (5 min)
1. Create a **new standard Windows user** (Settings > Accounts > Other users) or use a clean VM. Sign in as that user.
2. Download the setup exe, the zip and `SHA256SUMS.txt` into one folder. Run `Get-FileHash <file> -Algorithm SHA256` for both and compare with `SHA256SUMS.txt`. **Pass** only if both match.
3. Note: app version, Windows build (`winver`), display scale (Settings > Display), and the date.

## 1. Installer (15 min)
1. Run the setup exe. SmartScreen will warn (unsigned): note the exact wording, choose More info > Run anyway.
2. Check: the installer window opens, uses the Forge Room look, shows the logo, no clipped text. Choose the desktop-shortcut task once.
3. After install: Start Menu entry "Forge Room Early Preview" exists; desktop shortcut exists; the taskbar and shortcut icons are the **new logo** (house-shaped F), not a generic icon.
4. Launch from Start. Title bar shows the logo; window opens centred; version in the status bar reads 0.110.0-alpha.3.
5. Check install folder `%LOCALAPPDATA%\Programs\Forge Room Early Preview` has the exe and the uninstaller; **no** admin prompt appeared.
6. **Upgrade test:** install alpha.2 first on a second clean user (or after uninstalling), create a room and a lobby goal, then run the alpha.3 setup. Check: app updates in place, room and lobby settings survive, shortcuts still work.
7. **Downgrade test:** run the alpha.2 setup over alpha.3. Expected: blocked with a clear message.
8. **Uninstall:** Settings > Apps > Installed apps > uninstall. Check: app, shortcuts and Start entry are gone; your sample project folder is untouched; `%LOCALAPPDATA%\Forge Room Early Preview` (rooms/settings) is kept (or the installer says otherwise).

## 2. Editor and save paths (20 min), disposable folder with a few text files
1. Open Folder, open 3 files, edit two, save one with Ctrl+S, close the other with Save / Discard / Cancel (three separate trials). Reopen and confirm content.
2. Edit a file in Notepad while it is open and **clean** in Forge Room: it should reload by itself within ~1 s.
3. Edit a file in Forge Room (unsaved), change it in Notepad, press Ctrl+S: expect the Overwrite / Reload from Disk / Save As / Cancel dialog. Try each.
4. Close the window with a dirty tab: expect the unsaved-changes prompt.
5. Quick open (Ctrl+P), command palette (Ctrl+Shift+P), find/replace (Ctrl+F / Ctrl+H), search in files, go to line.
6. Open a large text file (3-5 MB): it should stay responsive; above 2 MiB it shows without colors.
7. Copy and paste between the editor, the terminal and another app.

## 3. Git (20 min), disposable repo (`git init`, a few commits)
1. Source Control: the Changes tab lists edits; stage, unstage, commit with a message, amend, undo last commit.
2. History tab: graph shows commits; click one to see its diff; open a file and check file history.
3. Create a branch, switch, make a change on each, merge to create a **conflict**. Check the Merge Conflicts group, right-click Accept Current / Incoming, and that Commit is blocked until resolved.
4. Add a remote (a second local bare repo is fine): Publish Branch, Push, Fetch, Pull.
5. Branch names with `/`, file names with spaces and `[1]` in them: stage and resolve them.

## 4. Terminal (10 min)
1. Ctrl+` opens PowerShell in the project folder. Run `dir`, `git status`.
2. `ping -t localhost`, then Ctrl+C: it must stop.
3. Scroll back with the wheel and Shift+PageUp; select text with the mouse and Ctrl+C: clipboard has it.
4. Start any local server (for example `python -m http.server 5173`): the Preview pane should pick up the address and show "Server is running"; Open in Browser opens it.
5. Resize the window while the terminal prints output.

## 5. Report
Add the results to `docs/native/ATUL_PHASE1_LOG.md`: pass / fail / not tested per section, plus defects with steps to reproduce. Then tell Zayed so you can close Phase 1 together (both of your tracks must be recorded).
