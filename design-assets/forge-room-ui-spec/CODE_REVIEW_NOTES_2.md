# Second correctness review: T6 fixes and merge-conflict actions

Mailbox T7. Initial requested range was `799bd3c..8ea8df829dfee173814c112ecf41df659da1921a`; review continued through **beed7e05a2de8bbe91f9e4dd140190b1b3933b1a** as Claude worked on branding/layout. Findings below cite current source lines/functions observed during that review. No application Rust file was edited, and no commit was created by Codex.

Runtime evidence: `measurements/merge-review-probes.json`, reproduced by `tools/review_merge_probes.py`. It compiles the actual `git.rs` and `document.rs` from stdin, then uses disposable Git indexes with stage-1/2/3 blobs. **It creates no commits, modifies no main-repository index, and removes its verified disposable fixtures.** Separate temporary original-module probes exercised the document sharing-error path and syntax state after long lines. The document module's eight existing tests were compiled directly and run sequentially: **8 passed**. This is focused evidence, not a full application/GUI test run.

An approval-review quota temporarily blocked the first sharing probe; that command did not execute. After the stated quota window passed, shell access was checked again and the probe was run normally. No approval bypass was used. Initial disposable-directory access under the OS temp directory failed; the successful Git probes instead used a checked, task-owned scratch directory that was removed afterward.

## T6 fix verification

| T6 finding | Current result |
|---|---|
| 1. Same-size/time overwrite | **Original input fixed.** `save()` reads content; the existing exact stamp-preserving regression test passes. A different read-error bypass remains, finding 2 below. |
| 2. Long-line stall | **Original performance input fixed.** The 550,000-byte single line now returns in **525.1 microseconds**, versus the earlier 1.5687554 seconds. Skipping parser state creates a new correctness issue, finding 4. |
| 3. Resize/Copy underflow | **Code path fixed.** Resize clears selection/snapshot, and live extraction additionally clamps to actual screen rows/columns. No new resize arithmetic counterexample found; the private real-PTY test was inspected, not rerun here. |
| 4. Lobby timestamp overflow | **Code path fixed.** Shared `end_epoch()` caps duration and uses saturating addition, and both countdown and overview use it. Extreme-epoch rendering no longer performs the previous unchecked addition. |
| 5. Silent config write error | **Improved but incomplete.** Errors and dirty state are retained and visible; rename replaces the complete temporary file. Dirty settings can still be abandoned when leaving/switching context, finding 6. |
| 6. Selection drift during output | **Completed-selection copy fixed.** A released selection copies its snapshot. Starting another drag fails to invalidate that snapshot, finding 5. Live selection while actively dragging is still a different case from a completed selection. |
| 7. Whitespace selection interrupts child | **Code path fixed.** Explicit selection yields `Some`, including empty extracted text; Copy doesn't treat it as no-selection. |
| 8. Lost Close/Quit continuation | **Successful resolution paths fixed by control-flow review.** Overwrite/Reload/Save As success calls `resume_after_conflict` once; Quit recalculates dirty tabs and doesn't close intermediate tabs, so its indexes don't shift. Modal input prevents ordinary tab changes underneath it. Cancel aborts the parent action. Failed I/O still aborts the attempt with a status message; no automatic retry is claimed. No concrete double-run, wrong-tab or self-reopening-dialog loop was found. GUI acceptance was not rerun. |

## Findings, numbered by priority

1. **[P1] CONFIRMED: resolving one bracketed filename resolves and stages another file too.**

   **File/line:** `git.rs:371-372`, `resolve_conflict()`.

   **Trigger:** have separate conflicted files **`a[1].txt`** and **`a1.txt`**. On only `a[1].txt`, choose Accept Incoming Change. Both checkout and add receive an ordinary Git pathspec. `--` ends option parsing but does not turn off wildcard matching.

   **Evidence:** actual resolver probe `pathspec-brackets` exited successfully, replaced both working files with stage-3 content and collapsed **both** sets of unmerged index entries to stage 0. The evidence JSON records both paths at stage 0 with the same selected-side object ID. This is a real selection-boundary/data-loss risk: the UI promises one file but overwrites and stages another independently conflicted file without choosing its side.

   **Suggested fix:** make per-file paths literal for **both** Git commands, e.g. global `--literal-pathspecs` before `checkout`/`add`, or a correctly constructed `:(literal)` pathspec. Preserve `--` as well for leading-dash safety. Add a two-file bracket regression asserting the unselected path retains its original unmerged stages and working bytes. Audit other per-file staging actions for the same pathspec semantics without widening this fix unnecessarily.

2. **[P1] CONFIRMED: a read-sharing failure still permits a destructive ordinary save.**

   **File/line:** `document.rs:146`, `content_differs()`.

   **Trigger:** after opening `old\n` and editing the buffer to `mine\n`, another writer opens that file with Windows **FILE_SHARE_WRITE only**, writes `new\n`, and keeps its handle open. Reads are denied while concurrent writes remain permitted. Ordinary Save attempts the read, classifies the error as “not different,” then successfully writes over `new\n`.

   **Evidence:** original-module probe used `std::os::windows::fs::OpenOptionsExt::share_mode(2)`, asserted a normal `read_to_string()` failed, then observed `doc.save().unwrap()` succeed. After dropping the other handle, disk content was **`mine\n`**. This disproves the comment that any read permission/sharing problem necessarily surfaces from the write. No live user file/ACL was modified; the fixture was temporary.

   **Suggested fix:** return a `Result` from the content check. Only **NotFound** should deliberately take the documented recreate-file path. Invalid UTF-8 should count as a conflict or explicit read error. Propagate all other read failures or conservatively refuse overwrite; an inability to inspect current bytes is not evidence they are unchanged. Keep explicit Overwrite as a separate user-authorized action. Add the Windows sharing-mode regression alongside the existing metadata-preserving test.

3. **[P2] CONFIRMED: choosing the deleted side cannot resolve a delete/modify conflict.**

   **File/line:** `git.rs:371`; invoked by `source_control.rs` conflict context menu at approximately **676-694**.

   **Trigger:** a modify/delete conflict where incoming deleted the file (index has stages 1 and 2 but no 3); choose Accept Incoming Change. Conversely, for current-deleted/incoming-modified (stages 1 and 3, no 2), choose Accept Current Change.

   **Evidence:** actual `incoming-deleted` probe returns **`error: path 'deleted.txt' does not have their version`**; `current-deleted` returns **`does not have our version`**. Both retain unmerged index entries and the working file. Ordinary content/content tests do not cover the absent-stage cases.

   **Suggested fix:** inspect unmerged stages for that literal path. If the chosen side has no blob, resolve as the chosen deletion using the appropriate literal-path removal/staging operation. Make deletion explicit in menu text/details. Keep the existing whole-side checkout/stage path when that side actually exists. Test both directions, plus an add/add and binary conflict, without inventing a textual merge for binary blobs.

4. **[P2] CONFIRMED: a skipped long line leaves all later parser state potentially wrong.**

   **File/line:** `syntax.rs:253-255`, `MAX_HIGHLIGHT_LINE_BYTES` fallback inside `refresh()`.

   **Trigger:** this Rust text, below the 2 MiB file cutoff: `"/*" + "a".repeat(5000) + "\nlet x = 1;\n*/\n"`. The first line opens a multiline comment, but it is over 4,000 bytes and never reaches the parser. The next line is still parsed as code. The reverse error occurs when a skipped long line closes a previously open string/comment.

   **Evidence:** a warmed optimized probe of the unchanged current `syntax.rs` compared the `let` span with a short equivalent comment. Expected comment color was **#6F7176**; actual was keyword color **#E0766F**. Text byte coverage stayed correct. The same probe verified the old 550 KB stall input is now fast, so reverting the guard would reintroduce T6's performance issue rather than solve both requirements.

   **Suggested fix:** mark parser state unknown after an unparsed line and avoid colored continuation or suffix reuse based on that invalid state. A simple safe policy is plain-text fallback for the whole document whenever any line exceeds the complexity limit. A stronger solution needs bounded/off-thread parsing that preserves state. Do not reset blindly to the initial syntax state or parse only a string prefix and claim later state is valid. Test both opener-on-long-line and closer-on-long-line, including edits crossing the threshold.

5. **[P2] CONFIRMED by control-flow trace: a second terminal drag can copy the first selection's snapshot.**

   **File/line:** `terminal.rs:225-230` starts a new drag without clearing `snapshot`; `selected_text()` at **167-169** prefers that old snapshot.

   **Trigger:** select/release text A, leaving `snapshot=Some(A)`; begin dragging a new range B, and press Ctrl+C **before releasing the mouse**. The selection endpoints now identify B, but `selected_text()` still returns A. The old snapshot is replaced only on `drag_stopped()`.

   **Evidence:** deterministic state trace of the real methods; this exact pointer/keyboard timing was not replayed through the GUI. A normal released second selection does work, so the regression test must copy while the second drag is still active.

   **Suggested fix:** clear the previous snapshot at `drag_started()` before assigning new endpoints. Leave it `None` during the active drag so Copy reads the current range; capture the new snapshot on release. Do not clear the new selection after merely drawing it. Add a state test or real-PTY interaction test covering two successive selections and Copy during the second drag.

6. **[P2] CONFIRMED by control-flow trace: pending lobby edits can still be abandoned on context changes.**

   **File/line:** `lobby.rs:220-226` (`load_for`) overwrites the full state with defaults; `flush()` at **261** is only called by `Lobby::show()`. Main's folder-open path rebinds the lobby without a dirty-settings guard; app quit checks editor documents rather than lobby dirty state.

   **Trigger:** make the lobby settings temporarily unwritable, edit a goal, and see the new “edits are kept and saving will be retried” warning. Leave Lobby for Code, restore writability, then quit without revisiting Lobby; the page is no longer calling `flush()`. Or open another folder while the settings are still dirty: `load_for()` discards the only unsaved configuration and its error/dirty flags immediately.

   **Evidence:** direct lifecycle trace; no user settings were made read-only during this review. The same-page retry test passes the narrower use case of retaining the same `Lobby` and calling `flush()` again. It does not cover leaving that page, switching projects, or quitting while dirty.

   **Suggested fix:** poll/flush pending configuration from the app lifecycle independently of which page is visible, and guard workspace switches/quit when unsaved lobby state cannot be persisted. Return an error/decision from rebinding rather than unconditionally resetting. Offer Retry/Save Elsewhere/Discard or retain per-workspace pending state. A retry timer alone cannot preserve edits after the owning object has been replaced.

## Merge cases checked without an additional finding

| Input | Actual probe result |
|---|---|
| `a file.txt` | Resolves/stages the selected side; argument vector preserves spaces. |
| `-notes.txt` | Resolves/stages correctly; `--` prevents interpreting the filename as an option. |
| Binary bytes containing NUL | Whole-side resolution succeeds; the helper delegates bytes to Git rather than decoding as UTF-8. |
| Conflict file also open and dirty | Resolver stages the selected side; editor retains its dirty buffer, and a subsequent ordinary document Save returns `ChangedOnDisk`. No silent destruction of the dirty buffer was observed. Do not force-reload that buffer while addressing other issues. |
| Commit gate | Both click and command-Enter require `conflicts.is_empty()` and nonempty staged/message state. Git itself will reject still-unmerged entries. Manual Stage marks conflict content resolved by design; checking for textual markers after explicit staging is a separate product policy, not proof that this gate is broken. |

## Persistence and continuation notes

The settings write uses a temporary file in the same directory and renames after writing; normal single-instance failure preserves the old target. No Windows replacement failure was inferred solely from POSIX-versus-Windows folklore. Reusing a fixed temporary basename across concurrent app instances deserves a concurrency test, but no independent corruption repro was established, so it is not promoted to a numbered finding here.

The Resume state is captured only when a save encounters `ChangedOnDisk`; ordinary success resets the temporary intent. Resolving a conflict then restarting Quit recomputes remaining dirty tabs, avoiding stale saved-tab queues. I found no concrete index-shift or double-close counterexample within the normal modal flow. An Overwrite/Reload I/O error currently aborts the parent attempt instead of retaining a retry continuation; this is an explicit limitation, not a claimed silent discard of editor buffers.

Review priority: fix the two reproduced overwrite/selection-boundary issues first, then deleted-side resolution and syntax-state validity. Tests passing for the original T6 inputs do not cover these new inputs. This report does not claim the broader app, new layout or installed resources are fully verified.
