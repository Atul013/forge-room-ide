# Native project tasks

Put `.forge-room/tasks.json` in the project root to add project-defined commands to **Workflows**, **Tests**, and **Run & Debug**. Example:

```json
{
  "tasks": [
    { "label": "Build release", "group": "build", "command": "cargo build --release" },
    { "label": "Integration tests", "group": "test", "command": "cargo test --test integration" },
    { "label": "Run server", "group": "run", "command": "cargo run -- --serve" }
  ]
}
```

Groups `build` and `check` appear in Workflows; `test` appears in Tests; `run` appears in Run & Debug. Forge Room also shows its detected project commands. A task starts only after its button is pressed in a trusted folder, in a separate integrated terminal rooted at that project. The visible command is executed by the terminal shell, so review task files from other people before running them.

This first native task format supports one command per task. It does not yet support task dependencies, problem matchers, background readiness, or debugger attachment. Files over 128 KiB are rejected; at most 50 tasks are read, and invalid individual entries are skipped. Invalid JSON is shown on the page and does not hide detected project commands.

Forge Room also reads common `.vscode/tasks.json` version `2.0.0` entries. `shell` and `process` tasks with a `label`, `command`, and `group` of `build`, `test`, or `run` appear alongside native and detected commands. String arguments are supported. JSONC comments and trailing commas are accepted. Tasks with `dependsOn`, `options`, or `${...}` variables are skipped because those features cannot yet be reproduced safely. Process tasks are quoted for the integrated PowerShell shell. Task execution remains explicit and does not run during discovery.
