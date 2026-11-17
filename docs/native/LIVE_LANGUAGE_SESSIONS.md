# Persistent native language sessions

In a trusted folder, open a saved Rust, Python, TypeScript or JavaScript file and choose
**Problems → Start language session**. Start each language family explicitly. Rust uses
`rust-analyzer`, Python uses `pylsp`, and TypeScript/JavaScript share
`typescript-language-server --stdio`. Installed programs are resolved only from absolute PATH
directories; empty/relative PATH entries and the workspace directory are not searched.
Missing servers and failed initialization are shown; no server is downloaded automatically.

Each family keeps one process for the current workspace. Open buffers are coalesced every
350 ms, and a navigation request forces a fresh snapshot first. The active file has priority
within 16 files, 256 KiB per file and 2 MiB total. Generated/untitled views, missing files,
unsupported languages and files resolving outside the root are omitted. Bounds are checked
before copying editor text; omitted buffers are labelled. Servers receive unsaved text.
They run with the user's OS permissions and can inspect workspace files or run their own tools;
this is a trusted local language integration, not an OS sandbox.

The client sends initialize/initialized, didOpen, increasing document versions, didChange,
advertised didSave, and didClose. Full synchronization and a full-range incremental replacement
are supported. Requests and navigation locations use UTF-16 positions, converted to/from the
editor's scalar cursor columns. Other negotiated encodings and no-change synchronization are
rejected explicitly. Definition, hover, plain-text completion, references and reviewed rename
share the live process. Existing one-shot navigation remains available when no sessions run.
Starting a live session discards pending one-shot results. Unsupported language families must
be started before querying while other live sessions are running.

Problems combines diagnostics for the synchronized open buffers. Versioned results for old or
closed buffers are discarded; editing removes the previous version's list while new results
are pending. The status shows buffers awaiting diagnostics. For servers that omit diagnostic
versions, results are associated with the latest sent snapshot; their exact freshness cannot
be proven. Live syntax/analysis diagnostics depend on the server and enabled plugins and do
not substitute for cargo check or project tests. Rust type errors may require saved compiler
checks even while syntax diagnostics update on unsaved buffers.

Query IDs distinguish superseded requests. Answers are discarded when any synchronized buffer
changes before delivery, or when the active file/workspace changes. Completion insertion also
checks the exact text and cursor again. Rename remains an explicit preview/apply operation
through the existing stale-file and dirty-buffer guards. Server-initiated workspace edits are
refused; unknown server requests receive an unsupported-method error.

Stop language sessions, folder changes, restricted mode and app shutdown release owned sessions.
The worker requests shutdown then exit. A 500 ms watchdog kills an unresponsive owned process;
Windows cleanup also targets its process tree while holding the child handle. Cleanup runs
off the UI thread. Failed servers are not automatically restarted; Start retries them explicitly.
No session or execution choice is silently resumed on app restart.

Messages have 8 KiB header / 4 MiB payload limits; duplicate lengths, oversized or malformed
frames fail the session. Input snapshots replace queued typing bursts; protocol and query queues
are bounded. Per-file diagnostics and the combined Problems list are bounded to 500 entries.

## Evidence and remaining acceptance

The standalone Rust protocol peer tests process reuse, open/change/save/close, full/incremental
sync, UTF-16 requests, stale diagnostic versions, latest-query identity, changed-other-buffer
rejection, path/text/count limits, server-edit refusal, initialization failure and Stop during a
stalled initialization. The peer is compiled for tests; it is not an application runtime component.
Framing tests reject duplicate/oversized headers, oversized payloads and truncated frames.

Explicit local trials passed with rust-analyzer and an isolated pylsp 1.15.0 / Pyflakes 4.1.0
environment: unsaved syntax diagnostics, definition navigation using the same session, and
unchanged saved source. The two real-server tests are opt-in and remain ignored in general CI.
Windows CI is tracked on the PR. Native desktop controls, typing/click behavior, layout,
accessibility, large real projects and TypeScript-server behavior remain hands-on gates.

Protocol reference: [LSP document synchronization](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_synchronization).
Python installation/plugin contract: [python-lsp-server](https://github.com/python-lsp/python-lsp-server).
Dynamic registration, pull diagnostics, automatic completion popups, snippets/additional edits,
code actions, formatting and multi-root sessions remain outside this batch.
