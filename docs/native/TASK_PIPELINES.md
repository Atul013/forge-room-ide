# Native project task pipelines

Workflows, Tests and Run & Debug now show reviewable `.vscode/tasks.json` pipelines beside
the existing Forge task and detected project commands. Nothing runs on discovery or folder
open. Review the ordered commands, working folders and declared environment changes, then
click Run in a trusted folder. Execution uses a fresh Windows PowerShell terminal; failure
to create that terminal never sends the command to an existing busy session.

## Supported import contract

- Version 2.0.0 JSONC, comments and trailing commas, at most 128 KiB / 50 task definitions.
- Shell and process commands, string arguments and commandless dependency aggregates.
- One dependency, or multiple dependencies with `dependsOrder: "sequence"`. Dependencies
  run before their dependent command, in listed order. Shared prerequisites run once per plan.
  Duplicate/missing labels, cycles, depth over 16, more than 16 direct dependencies or more
  than 32 command steps disable the plan before any prerequisite runs.
- `${workspaceFolder}`, `${workspaceFolderBasename}`, `${pathSeparator}` / `${/}` and active
  saved-file variables `${file}`, `${fileDirname}`, `${fileBasename}`,
  `${fileBasenameNoExtension}`, `${relativeFile}`, `${relativeFileDirname}`.
- Variables are expanded in process program names, arguments, cwd and environment values.
  Arguments are passed through literal PowerShell quoting. Raw shell command interpolation
  is refused; put variables in the task's args instead. Shell command text is project code,
  interpreted as PowerShell, rather than a portable Bash/cmd script.
- `options.cwd` must resolve to an existing directory inside the current workspace at review.
  Paths resolving outside the root and active-file variables for outside/missing files fail.
  `options.env` supports at most 32 string/null entries, with bounded plain names. Values are
  visible in the local review; do not put secrets into a shared task file. Null unsets a variable.
- Individual Windows overrides merge over their task, including nested options/environment.
  Top-level defaults/overrides are rejected explicitly rather than being silently dropped.

Each step prints its ordinal/label, installs its declared environment, enters its reviewed
folder, runs its command and checks PowerShell success plus native LASTEXITCODE. A failed
step stops the remaining plan. Finally blocks restore the declared environment and working
folder on normal completion or a thrown failure. The terminal prints an all-steps-succeeded
message only after all commands pass. This is process exit evidence, not parsed test-case
evidence or proof that a project command really tests the requested feature.

Plans are cached by workspace, active file and task-file metadata to avoid rebuilding graphs
every UI frame. Task metadata is checked at most once per second while the page is visible;
Refresh task plans forces a rebuild, including after creating a formerly missing cwd. The
visible reviewed script is the one sent on Run. The expanded script is bounded to 8 KiB.
Tasks can run arbitrary project code with the user's rights and can change files/environment
themselves. This workflow does not sandbox commands, rollback build output, protect against
concurrent filesystem replacement, or impose a runtime budget. Use terminal Ctrl+C to interrupt.

## Explicitly unsupported

Parallel dependencies, background/readiness matching, input/config/env/named-root variables,
custom task shells, extension-defined task types, argument quoting objects, automatic runOn,
task instance-limit enforcement, presentation settings and problem matcher interpretation are
not implemented. Background tasks, unknown variables/types and custom shells disable affected
plans with a reason. Automatic startup never occurs; presentation/problem-matchers/run options
do not change the manual foreground runner. Imported pipelines currently require Windows;
other project actions remain available on their existing platforms.

## Validation

Graph/schema/JSONC/path/variable/cache regressions cover refusal and review invalidation.
Real Windows PowerShell tests verify ordered shared prerequisites, per-step environment,
failure stopping later commands, and finally restoring environment/cwd. A compiled native
argument peer verifies workspace/file paths containing spaces, apostrophes and dollar syntax
arrive literally. It avoids changing execution policy or requiring PowerShell script files.
The full local native app suite, build, formatting and Clippy are recorded in the PR; CI and
visible desktop/PTY interaction are separate gates. No desktop acceptance is claimed while
the Windows Computer Use pipe is unavailable.

References: [VS Code tasks](https://code.visualstudio.com/docs/debugtest/tasks),
[task variables](https://code.visualstudio.com/docs/reference/variables-reference).
