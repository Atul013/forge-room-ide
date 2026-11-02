# Editor performance and component decision (E03)

Status: measured 2 November 2026 on the development machine (Windows 11, release build). Re-run with
`cargo test --release large_file_cost -- --ignored --nocapture` in `native/forge-room-app`.

## What was measured

A synthetic Rust file of 72-character lines, highlighted by syntect with the active theme, then laid
out by egui at 14 px monospace with no soft wrap.

| File size | Lines | Open: highlight (first paint) | One keystroke: re-highlight | Layout of the changed text |
|---|---|---|---|---|
| 100 KiB | 1.4 k | 282 ms | 1.5 ms | 5.5 ms |
| 500 KiB | 6.8 k | 1.4 s | 9.8 ms | 12.5 ms |
| 1 MiB | 14 k | 2.7 s | 18 ms | 22 ms |
| 5 MiB | 70 k | 13.7 s | 80 ms | 118 ms |
| 10 MiB | 140 k | 27.3 s | 189 ms | 257 ms |

Before this change every keystroke re-highlighted the whole file: 2.8 s at 1 MiB and 27.6 s at 10 MiB.

## What changed

- Highlighting is incremental. Each line keeps the parser state at its start. After an edit, only lines
  from the first changed line are re-parsed, and parsing stops as soon as the state matches the old state
  again (so opening a block comment re-parses until it closes, and a simple edit re-parses one line).
- Highlighting is time-budgeted (12 ms per frame). Opening a large file never blocks a frame; lines not
  reached yet are drawn as plain text and fill in over the next frames.
- Above 2 MiB (`HIGHLIGHT_LIMIT_BYTES` in `syntax.rs`) highlighting is off, so a keystroke costs layout
  only. That is about 118 ms at 5 MiB and 257 ms at 10 MiB.

## Decision

Keep the egui `TextEdit` for now.

- Up to about 1 MiB (about 14 k lines) typing costs roughly 40 ms, which is usable.
- From 1 to 2 MiB typing is sluggish but works with highlighting.
- Above 2 MiB the editor works but is slow to type in, because egui lays out the whole text on each edit.
  The 10 MiB open limit stays as a safety net, not as a performance promise.

When large files matter, the next step is a virtualised editor (lay out and paint only visible rows, keep
the text in a rope). That replaces `TextEdit` and is a larger piece of work than this slice. Candidates to
evaluate when it is scheduled: a custom widget over `ropey`, or embedding an existing Rust editor core.
The decision should be re-measured with the same benchmark before committing.

## Not measured

- Memory use and startup time of large-file sessions.
- Many open tabs at once.
- Scrolling frame rate in a 1 MiB file.
- Files with very long lines.
