# Durable recovery for reviewed changes

Before a reviewed AI proposal or received Room patch is applied or undone, Forge Room saves
and flushes a workspace-scoped recovery copy in LOCALAPPDATA/Forge Room Early Preview/
review-checkpoints. If saving fails or the capacity is full, no target file is changed.
This is local application data containing source text; it is never included in Room transport.
It does not journal arbitrary Direct CLI writes, LSP renames or every editor keystroke.
New Chat preserves the selected Ask/Plan/Reviewed flow instead of switching back to Direct mode.

Each version-1 record captures the canonical workspace, a fresh ID/timestamp and original/
intended UTF-8 contents for one to eight existing files, bounded to 128 KiB per version and
8 MiB per record. Relative paths are validated, and links/reparse points, alternate streams,
credential paths, outside paths and repeated entries are rejected. A new recovery file uses
create_new and is flushed before reviewed target writes begin. Existing copies are not replaced.

## Review and restore

In AI, expand Recovery checkpoints and Load / refresh checkpoints. Records survive New Chat
and app restart. Choose a record and inspect each saved original; Copy saved original supports
manual recovery when newer work conflicts. Nothing is restored automatically on load/startup.
Restore unchanged checkpoint targets is an explicit trusted-folder action:

- Files already matching the saved original are skipped.
- Files matching the exact intended replacement are restored through the existing staged edit engine.
- Missing files can be recreated without replacing another file that appeared meanwhile. A flushed
  temporary file is linked into the missing name atomically; filesystems lacking this support
  report an error and retain the saved copy for manual recovery.
- Changed contents, dirty open buffers, unsupported paths/links and workspace changes stop recovery.
  A recovery copy changed since review must be loaded and reviewed again.

This supports interrupted multi-file application: a mixture of original, applied and missing
files is handled without assuming all files reached the same stage. Partial I/O failures retain
the checkpoint, report recovery progress/failure and refresh clean editor tabs. Recovery refuses
unknown later edits, instead of treating a checkpoint as permission to overwrite them.

At most 32 records per workspace are accepted. Deleting a selected copy requires its explicit
no-longer-needed checkbox and Delete action; no automatic pruning or source/worktree deletion
occurs. Copies remain after restore so they can be inspected. Root relocation, corrupt records,
file metadata/ACL restoration, full power-loss guarantees, encryption at rest, Direct-agent
per-action snapshots and automated retention/recovery remain outside this bounded implementation.

Tests reload the on-disk record, recover a partial application with a missing file, reject dirty
buffers and changed recovery data, preserve later file edits, retain a preexisting temporary file
on failed creation, and retry a safe missing-file restore. Real desktop recovery controls remain
an open hands-on gate. The existing stand-in CLI and Room patch review tests run through this
checkpoint-before-apply path; they do not establish live provider or desktop acceptance.
