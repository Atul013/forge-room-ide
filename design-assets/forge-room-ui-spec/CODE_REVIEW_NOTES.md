# Rust correctness review: mailbox T6

Reviewed `git diff 5fadb8f..HEAD -- native/forge-room-app/src` at starting HEAD **799bd3cdb4ff3edafb87094edb59e2e3f730149a**. Line references below are pinned to that commit, under `native/forge-room-app/src/`. Claude advanced HEAD to **49cf83f20124c2b2e5f4312a26a44e507daf8e6a** during the review; the four focused module files (document, syntax, terminal, lobby) were checked byte-identical to the starting snapshot before writing this report. Main's save/conflict control flow was checked against `git show 799bd3c:.../main.rs`. This is not a review of every later Git UI change.

No application source or Git history was changed. Three standalone probes were compiled from stdin to temporary executables, using the actual source modules or already-built dependency libraries; executables/fixtures were removed after successful probes. No repository test files were added. **CONFIRMED** below means either reproduced in those probes or established by a deterministic control-flow trace, with the evidence type stated; it does not mean every case was driven through the GUI.

1. **[P1] CONFIRMED: unchanged timestamp/length bypasses outside-change protection and silently overwrites external content.**

   **File/line:** `document.rs:120`, inside `changed_on_disk()`; affects `save()` and `main.rs::sync_external_changes()`.

   **Failure:** returning false solely because `(mtime, length)` matches the cached stamp treats that pair as a content identity. An external tool can replace bytes while retaining those metadata values, so both automatic reload and the save conflict dialog are bypassed. The next ordinary save erases the external edit without consent.

   **Concrete trigger:** open four-byte `old\n`; change the editor buffer to `mine\n`; another process writes four-byte `new\n` and restores the original last-write time (for example a timestamp-preserving synchronization/restore tool). Save normally.

   **Evidence:** a probe included the original `document.rs` as a module, executed that sequence using `File::set_times`, asserted `changed_on_disk() == false`, then successfully saved and asserted the disk now contained `mine\n`. Probe output: `CONFIRMED: original DocumentSession silently overwrites same-length external content with restored mtime.`

   **Suggested fix:** before a destructive save, compare current disk bytes/content to `saved_text` even when the cheap stamp matches; use stamps only as a polling optimization, not as overwrite authorization. Consider a stronger identity/checksum for watcher updates. Also validate that the stamp corresponds to the bytes read: `open()` reads content before taking its stamp, leaving a separate read/stamp race. That race was not independently reproduced here; do not count it as a second confirmed finding. There will still be an unavoidable check/write race without a suitable atomic/concurrency strategy.

2. **[P1] CONFIRMED: a single moderately large line blocks the UI far beyond the 12 ms highlighting budget.**

   **File/line:** `syntax.rs:244` checks the budget only before `parse.parse_line()` at **249**. `new_lines`/prefix/suffix work precedes the timer at **232** as well.

   **Failure:** one parser call is uninterruptible from this loop. The byte-size fallback does not help a long line below 2 MiB. The UI thread finishes parsing the whole line and constructs its spans before checking time again. This contradicts the guarantee that highlighting is spread over bounded frame work.

   **Concrete trigger:** a `.rs` document containing `"let x = 1; ".repeat(50_000)` with no newline: **550,000 bytes**, far below the 2 MiB plain-text cutoff. Open it or change that line.

   **Evidence:** a warmed, optimized probe compiled the unchanged `fonts.rs`, `theme.rs` and `syntax.rs` against the existing release libraries. After warming a short Rust highlight, one public `highlight()` call for this exact line took **1.5687554 seconds**, returned all original text, and reported `pending=false`. This measures the highlighter call, excluding widget layout and initial syntax loading. The first standalone link attempt needed the local Windows import-library path; it was supplied on retry. No app-source workaround was used.

   **Suggested fix:** add a maximum line-length/token-work guard that renders pathological lines plain, or move parsing off the UI thread with versioned results. Keep the 2 MiB fallback but supplement it with line-complexity limits. Include line collection, state cloning and job rebuild costs in the performance accounting; the existing 72-character-line benchmark cannot verify the long-line case. Preserve parser state correctness when choosing a fallback, especially if later lines depend on an open string/comment.

3. **[P2] CONFIRMED: resizing while a multirow terminal selection exists can panic during Copy.**

   **File/line:** `terminal.rs:167` (`resize`) does not clear or clamp `selection`; **144-150** pass those stale coordinates to `contents_between()`.

   **Failure:** selection columns come from the old grid. After narrowing the terminal, the stored start column can exceed the new column count. In the actual **vt100 0.16.2** dependency, `screen.rs:189` computes `cols - start_col` for the first selected row. That underflows with checked arithmetic. It occurs while holding the parser mutex, so unwinding also poisons the shared parser. In unchecked release arithmetic the behavior wraps rather than producing the same debug panic; extraction remains invalid. Do not claim a release panic from this evidence alone.

   **Concrete trigger:** in a 100-column terminal, select from `(row 0, col 80)` to `(row 1, col 90)`, resize to 20 columns without clicking/typing, then press Ctrl+C.

   **Evidence:** a probe linked the installed `libvt100-bead662f91c8161a.rlib` for vt100 0.16.2, called `set_size(3,20)`, then `contents_between(0,80,1,91)`, and caught the resulting panic. The app control flow preserves exactly these coordinates across `resize()`.

   **Suggested fix:** clear selection whenever the grid resizes, or clamp both endpoints to the actual parser `screen.size()` and normalize them again before extraction. Do not rely solely on pointer-time clamping: the grid may have changed since the drag.

4. **[P2] CONFIRMED: valid saved lobby JSON can overflow the room end timestamp.**

   **File/line:** `lobby.rs:227` (`remaining`) and the separate end-time addition in `overview()` around **734**.

   **Failure:** deserialized `start_epoch` is an unrestricted `u64`, but the end time is computed with ordinary addition. `saturating_sub(now())` happens after the overflow, so it does not protect the addition. Debug builds panic while displaying the lobby; ordinary unchecked release arithmetic wraps and can immediately mark a far-future room expired.

   **Concrete trigger:** save this otherwise valid partial config at the configured lobby settings path: `{"start_epoch":18446744073709551615,"duration_hours":24}`. `#[serde(default)]` supplies the other fields, then the normal header countdown calls `remaining()`.

   **Evidence:** deterministic code/arithmetic trace: `u64::MAX + 86400` cannot fit in `u64`; the loading path has no validation. This was not injected into the user's real room or tested through the live GUI.

   **Suggested fix:** validate timestamp/duration ranges when loading, then use a shared checked/saturating end-time helper in both rendering paths. If invalid settings are rejected, retain the original file and surface an actionable error instead of silently rewriting defaults over it.

5. **[P2] CONFIRMED: lobby edits can fail to persist with no error or retry.**

   **File/line:** `lobby.rs:213-222` (`save`), called after the per-frame config comparison in `show()` around **284**.

   **Failure:** directory-creation and write errors are discarded. The in-memory configuration is already changed, so the next frame's before/after comparison is equal and there is no retry or persistent dirty flag. Users can edit a goal, duration, milestone or member note, see the new value, then lose it on reopening without any warning.

   **Concrete trigger:** load a readable lobby settings file with its read-only attribute set, then edit the goal or a milestone. Alternatively fill the volume before editing. `fs::write` fails, the UI keeps the value for this session, reopening reads the old value.

   **Evidence:** deterministic error-handling trace, not a filesystem-permission change to the live project. Both relevant I/O results use `let _ = ...`; `save()` returns no status.

   **Suggested fix:** return `Result`, keep a config dirty flag until a successful save, and display a persistence error with Retry/Save Elsewhere. Use a temporary-file/atomic replacement strategy so failed or interrupted saves also do not leave a truncated settings file. Respect existing file attributes and do not silently force-write read-only files.

6. **[P2] CONFIRMED: terminal selection drifts to different output while commands continue printing.**

   **File/line:** `terminal.rs:90` processes new output; **144-150** extract from current `screen()` using only visible `(row,col)` endpoints.

   **Failure:** output can scroll the grid underneath an active selection. The stored coordinates never refer to an immutable selected range or text snapshot. Ctrl+C then copies whichever text happens to occupy those rows, rather than the text the user selected.

   **Concrete trigger:** stream numbered lines with a delay, drag-select a visible line, let enough output arrive to scroll it away, then press Ctrl+C without scrolling or typing yourself.

   **Evidence:** the parser probe wrote `SELECT-ME\r\nother\r\n` to a 3-row grid, extracted `(0,0)..(0,9)`, then processed `new output\r\n` and extracted the same range. Result changed from **`SELECT-ME` to `other`**. App output processing neither clears selection nor pins its content. This verifies the parser/extraction mechanism; the full pointer workflow was not driven in the GUI.

   **Suggested fix:** anchor selection to stable scrollback positions, snapshot the selected text at selection completion, or explicitly freeze the view while selection is active. If output invalidates a selection, clear it visibly rather than silently copying unrelated text.

7. **[P2] CONFIRMED: copying an explicitly selected whitespace range sends Ctrl+C to the running process.**

   **File/line:** `terminal.rs:151` (`!text.trim().is_empty()`) and **248-255** (`Event::Copy`).

   **Failure:** `selected_text()` returns `None` for a selection containing only spaces/newlines, and Copy interprets `None` as “there is no selection, interrupt the child.” A user's copy action can stop a running build/server merely because the selected range was blank or the selection drifted onto an empty line.

   **Concrete trigger:** run a long command, drag over a blank line or whitespace-only cells, then press Ctrl+C. The selection exists, but the trim check routes to `send(&[0x03])` instead of a clipboard operation.

   **Evidence:** deterministic branch trace from existing selection -> whitespace extraction -> `None` -> ETX. This specific GUI interaction was not replayed.

   **Suggested fix:** distinguish “no selection” from “selection with empty/whitespace content.” Only send ETX when `selection.is_none()`. Preserve literal whitespace when it can be extracted, and otherwise treat Copy with an explicit empty selection as a harmless clipboard/no-op operation. Ctrl+X mapping to `0x18` is not itself an identified defect.

8. **[P2] CONFIRMED: resolving a disk conflict loses the user's pending Close/Save All/Quit continuation.**

   **File/line:** `main.rs:2456` takes the pending operation before calling `save_index`; `save_index` replaces it with `Pending::DiskConflict`; `disk_conflict_dialog` clears it at **2518** and only overwrites/reloads/saves-as.

   **Failure:** the operation that led to Save is forgotten when a conflict is discovered. Overwrite succeeds, but the requested tab close or application quit does not resume. In Quit/Save All, later dirty tabs are not saved because the `.all(...)` loop stopped at the conflicting document, and the continuation list has been discarded.

   **Concrete trigger:** edit two tabs; modify the first file externally; close the window, choose Save All, then choose Overwrite in the conflict modal. The first tab is saved, the application stays open, and later dirty tabs remain unsaved. Similarly, close a dirty tab, choose Save, then Overwrite: the tab remains open.

   **Evidence:** deterministic state-machine trace against the pinned `main.rs` source. This is not a claim that unsaved tabs are silently discarded: they remain open, and a new close attempt can prompt again. The defect is losing the explicit operation and Save All continuation.

   **Suggested fix:** retain a parent intent/continuation with the conflict (`SaveOnly`, `CloseTab`, or queued `SaveAllThenQuit`) and resume it after successful Overwrite/Save As or an explicitly accepted reload. Cancel should cancel the parent operation; failed I/O should leave the intent recoverable and show the error.

## Focused areas without an identified additional bug

- The suffix splice compares both parser and highlight state, and is enabled only when the old cache was complete. Prefix/suffix limits avoid overlap; an empty text update does not index the empty tail. I did not find a concrete stale-style or indexing counterexample in those paths. Existing incremental-edit tests were read, not claimed as rerun.
- The **above-2-MiB** branch returns a full plain `LayoutJob` and clears pending work. Its byte coverage is intact. The long-line problem above occurs **below** that cutoff, not in the plain branch. Whole-document widget layout cost is already documented and is not reported again as a new bug.
- Empty milestones use `len().max(1)` before division and the per-item loop does not index an empty vector. Person rows use optional lookups/defaults, not unchecked participant indexing. No concrete empty-state panic was found there. The timestamp overflow is a separate load-validation issue.
- Deleted files being recreated on save is explicitly documented/tested behavior in this change, so it is not listed as a defect. Save As keeps the old path when its write fails. The unmodified-tab reload path skips dirty buffers.

Prioritize the reproduced external-edit overwrite, long-line stall, and terminal resize/copy issue. Findings 4-8 have concrete inputs and control-flow evidence but still need their own regression tests and interactive acceptance when Claude implements fixes. No blanket “tests pass” or app-wide approval is implied by these focused probes.
