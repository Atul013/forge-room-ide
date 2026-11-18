# Native terminal shell profiles

The terminal panel lists shell executables available on the current machine. On Windows this includes Windows PowerShell, PowerShell 7, Command Prompt and Bash when detected. On Unix-like systems it includes the configured `$SHELL` and common shells found on `PATH`.

Choose **New terminal with** a profile to open another terminal in the current project directory and make that profile the default for future new terminals. The choice is stored in the normal per-user settings file. Restarting a terminal reuses the shell profile of that session; it does not silently switch to a newer global default. If a saved profile is no longer installed, the native platform default is used.

Detection runs once per app process and only checks whether candidate executable files exist. It does not run shell startup commands or inspect shell configuration. Restart Forge Room after installing a shell so it can be discovered. Starting a shell launches that executable with its platform-specific interactive arguments. Existing terminal instances, tab selection, split view, scrollback, and trust rules continue to work as before.

Automated tests cover profile discovery, unique profile IDs, executable paths, profile arguments and backward-compatible settings. Real interactive terminal and visible desktop acceptance are separate from source CI.
