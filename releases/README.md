# Folder structure

This repository keeps working source, tests, product documentation and local package artifacts separate.

```text
extensions/       IDE extensions and their tests
services/         Room service prototype and its tests
docs/             Product, research, architecture, planning, source briefs
releases/         Locally retained VSIX outputs (ignored by Git)
```

Package artifacts are not required to build the extensions. Regenerate them from the package scripts when needed.
