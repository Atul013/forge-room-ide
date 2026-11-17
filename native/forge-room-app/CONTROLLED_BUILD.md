# Controlled native build sessions

Choose Native HTTP, then **Controlled build**, select one to eight saved files, and enter a task. Review the complete outgoing request before sending. The first request contains selected path names, not their contents.

Models return one strict JSON action: `read_file`, `propose_edits`, `run_check`, or `finish`. A read needs approval after its contents are shown locally. Edits require an approved read and use the existing exact replacement preview, dirty/stale guards, Apply, undo and recovery copies. No new-file, deletion, rename or arbitrary shell action is available in this protocol. Provider-native function/tool calls remain disabled.

Checks must first be enabled in session settings, then individually approved with the executable, literal arguments and working directory visible. Supported recipes are Cargo check/test, Python compilation of selected files and pytest. These run trusted project code with ordinary local process permissions: they can write files, load project configuration, start other programs and access the network. This feature is not an OS sandbox. Stop/timeouts terminate the owned process tree on Windows; process escape is not a security boundary.

Validation records exit status, timeout/cancellation, elapsed time and bounded stdout/stderr. Inspect it, then choose whether to include output or status only in the next reviewed request. Only selected saved files are compared at check start/finish. Other inputs and transient edits are not sealed; exit status is not parsed test-case acceptance. Model finish summaries do not mark roadmap acceptance complete.

Session settings bound requests, approved checks, elapsed time and reserved maximum output tokens. An optional USD estimate uses user-entered prices and a conservative input estimate; it is not a provider billing cap. Started requests retain their reservations even if they fail. Each further request requires another explicit send. No unattended loop runs.

Grants are scoped to the workspace, exact connection settings and session time. Changing trust, workspace, runtime or flow revokes the session. Changing model settings invalidates existing grants. End the session to choose a new scope. Approved snapshots and evidence are session-only; the visible chat may retain model responses under existing local chat storage behavior.

Implementation: `controlled_build.rs`, `controlled_build/validation.rs`, with native request review and reviewed edits reused from the existing AI panel.
