# T13: post-T7 review and alpha.2 release verification

Reviewed 3 November 2026. Requested base: `5fa4de7`. Initial checkout: `90727c5`; another process advanced HEAD to **`5b64cb66fb62e493b1b2b5b66c8bdcd394f5c7ef`** during review. That additional commit changes only logo adaptation on light themes; reviewed separately. Application and packaging source remain untouched by Codex; no repository commit or push was made. The local alpha.2 tag points to `841a5c48b32fa0d5a667f857fd1bc67804bea8b4`, so release verification and current-source verification are separate results.

Scope: `preview.rs`, `mark.rs`, `source_control.rs`, `rooms.rs`, `main.rs`, `lobby.rs`, `terminal.rs`, `git.rs`, `document.rs`, and `syntax.rs`. Phase 1 remains open; this report does not establish desktop or installer acceptance.

## Findings

1. **[P1] CONFIRMED: a failed Lobby save still loses pending settings on folder switch.**

   **File/line:** `lobby.rs:222-231`, `main.rs:322-339`, `main.rs:2814-2816`.

   **Trigger:** edit a goal while the settings destination is unwritable, then open another folder. `load_for()` calls `finish()` but replaces the entire Lobby even when that returns an error. Main changes workspace and room before examining the returned error. Quit similarly ignores `finish()` failure in `on_exit`.

   **Evidence:** disposable copy invokes the actual `load_for()`. A regular file blocks the settings parent directory; the only in-memory goal is `ONLY UNSAVED COPY`. Rebinding returns error 183 but leaves `goal=""`, `dirty=false`, and no saved old configuration. The switch path is reproduced; the quit path is a static control-flow finding. This is the remaining part of T7 finding 6, despite app-loop retries now working.

   **Fix:** stop rebinding on failed `finish()` before resetting any workspace/room state, or retain the old pending configuration separately. Resolve failed saves before allowing exit, with explicit retry/save elsewhere/discard choices. Reporting an error after discarding the buffer does not preserve it.

2. **[P2] CONFIRMED by static branch review: Lobby repository links permit non-web URL schemes.**

   **File/line:** `lobby.rs:1527-1534`.

   **Trigger:** enter or load `httpx://example.test/payload` as Repository and click Open in browser. The check is `url.starts_with("http")`, so arbitrary schemes starting with those four letters pass unchanged to `context.open_url`.

   **Evidence:** the branch forwards `httpx://example.test/payload` without an `http`/`https` scheme check. A branch-expression probe records the same string. No browser or protocol handler was launched; actual external-handler behavior is not tested. This finding is about the unvalidated URL passed to egui, not a claim of demonstrated code execution. Preview's separate parser correctly rejects this scheme.

   **Fix:** parse and allow exactly `http` or `https` with a valid host; optionally prepend `https://` only to a bare hostname. Reject other explicit schemes and keep Open disabled for invalid input. Reuse one validator at both outbound URL sites.

3. **[P2] CONFIRMED: 40+ graph lanes consume the entire 180 px sidebar.**

   **File/line:** `source_control.rs:833-834`, `867-871`.

   **Trigger:** History containing 40 or more lanes, sidebar at its allowed 180 px minimum. The 4 px minimum spacing defeats the supposed 64 px graph budget. Subject placement still uses the full lane count.

   **Evidence:** actual `graph_row()` rendered headlessly with synthetic 1/16/40/48/64-lane rows at 180 px. Subject x positions: **36/86/182/214/278**. At 40 lanes its clip width is **0**; at 48/64 it is **-32/-96**. Subjects disappear; no panic occurred in these probes. These are egui layout/paint results, not visual GUI acceptance. The input is a synthetic row fixture rather than a 40-branch Git repository.

   **Fix:** bound graph width by actual available width and a minimum subject budget. Remove the unconditional 4 px floor when it would exceed that budget, or offer horizontal scroll/a wider graph view with explicit overflow handling. Never construct an inverted subject clip rectangle.

4. **[P2] CONFIRMED: Changes/History tabs overflow the supported narrow sidebar.**

   **File/line:** `source_control.rs:396-429`, especially `421`.

   **Trigger:** 180 px sidebar with 123 changed files and 80 commits. Each button computes its minimum width from the remaining width inside the loop; natural text width then expands the buttons beyond that budget.

   **Evidence:** actual `page_tabs()` at 180 px produces a **201.5 px** content rectangle. `History  80` text starts at **119.375** with width **62.125**, extending past x=180 before button padding. These counts are valid product state, not mocked product data. A headless egui fixture supplied them only for the test.

   **Fix:** compute equal tab widths once from the parent content width, then constrain text/badge layout within them. At narrow widths, abbreviate/hide counts or wrap/scroll the switch. `min_size()` alone does not cap natural widget width.

5. **[P2] CONFIRMED: exactly 4,000 bytes plus newline still loses syntax parser state.**

   **File/line:** `syntax.rs:136-140`, `260-261`.

   **Trigger:** Rust text `"/*" + "a".repeat(3998) + "\nlet x = 1;\n*/\n"`. Whole-file fallback checks `split('\n')`, excluding the newline, so the first 4,000-byte line passes. `LinesWithEndings` gives `refresh()` a 4,001-byte line, which it skips without updating the comment parser.

   **Evidence:** actual `highlight()` in the disposable source copy, allowed to finish pending work: expected `let` color inside a comment **#6F7176**, actual keyword color **#E0766F**. T7's greater-than-limit regression passes but misses this exact boundary.

   **Fix:** use identical line representation and limit semantics in preflight and parsing, ideally `LinesWithEndings` in both, and preserve whole-document plain fallback whenever parsing would skip a line. Cover 3,999/4,000/4,001-byte content with LF, CRLF, and no final newline.

6. **[P3] CONFIRMED: valid IPv6 loopback URLs without an explicit port are rejected.**

   **File/line:** `preview.rs:74-80`.

   **Trigger:** `http://[::1]/` or `https://[::1]/`. Splitting authority at its final colon mistakes a colon inside the brackets for a port delimiter.

   **Evidence:** actual private parser probe returns `None` and `is_web_url=false` for `http://[::1]/`, while `http://[::1]:5173/` is accepted and classified local. Open in Browser becomes disabled for the first valid URL.

   **Fix:** use a URL parser or explicitly handle bracketed IPv6 authority before parsing an optional port, with scheme-default ports when no port follows `]`.

7. **[P2] PLAUSIBLE runtime race, confirmed missing result binding: Preview can show a previous address's status.**

   **File/line:** `preview.rs:114-143`, `176-186`.

   **Trigger:** check local address A, then change to B before A's worker completes. Every worker holds the same status Arc, and writes an untagged Listening/Silent result. Changing the field does not clear a previous local result. A late A result can also overwrite the Unknown set for a remote B.

   **Evidence:** static shared-state/control-flow trace; exact worker scheduling was **not reproduced**. The display reads only Status and cannot verify which URL produced it. This does not cause a remote socket probe; it misattributes health status.

   **Fix:** bind each result to the checked normalized URL and a generation ID. Clear status on address change; publish only if the generation still matches. Display Checking/Unknown until the current check completes.

## T7 fixes and other reviewed boundaries

| Area | Result and evidence limit |
|---|---|
| Literal conflict resolution and missing-side deletion | Existing real-Git regression tests pass. `--literal-pathspecs` covers ls-files/checkout/add/rm in the resolver. Other older per-file Git commands are not globally literalized by this change. |
| Unreadable document refuses ordinary save | Existing Windows read-sharing and same-size/time tests pass. No new ordinary-save bypass reproduced. This is not a proof against all filesystem races. |
| Overlong-line plain fallback | Existing over-limit test passes; exact boundary remains finding 5. |
| Terminal new drag invalidates snapshot | Actual branch clears snapshot before assigning endpoints. Normal unit tests pass; five ignored benchmark/real-PTY tests were not run. No GUI drag/copy acceptance claim. |
| Lobby retries outside its page | `flush_pending()` now runs from app UI. Failed switch/exit remains finding 1. |
| Room recovery | Existing damaged-primary/backup guard and missing-primary recovery tests pass. No new empty-roster indexing panic found in reviewed changes. |
| Logo | Canonical current asset is fixed input to parser; existing geometry and area tests pass. Later light-theme adaptation test passes. General arbitrary SVG support is not claimed. |
| Changes/History separation | Code separates the lists and keeps composer outside their vertical scroll. Findings 3/4 limit narrow-width usability. Two-line path rows were probed separately; long names still clip to their available width and expose the full path on hover. |
| Preview sockets | Only `localhost`, `127.0.0.1`, `[::1]`, and `0.0.0.0` pass the pre-connect hostname allowlist. Ordinary remote names/IPs do not reach `to_socket_addrs`/connect. No remote network probes were performed. For strict loopback enforcement, also validate resolved socket IPs and normalize wildcard hosts to loopback. |
| Preview URL handoff | Scheme match requires lowercase `http`/`https`; file/javascript/httpx rejected by parser probe. Lobby uses a different weaker check, finding 2. Browser action requires a user click. No webview added. |
| Small Lobby header | Headless 604 px header fixture kept the sampled title/chips/Enter Room/more text within the header width. This narrow fixture is not a general whole-window/DPI acceptance pass. |

## Release verification: PASS within the stated scope

Fresh downloads came from [Atul013 alpha.2 release](https://github.com/Atul013/forge-room-ide/releases/tag/v0.110.0-alpha.2), published **2 November 2026, 21:58:39 IST**, into a newly created folder outside the repository:

`C:\Users\Icarus\AppData\Local\Temp\forge-alpha2-t13-60e745bf4e2340a08fdcf1e41c692109`

| File | Computed SHA-256 | Check |
|---|---|---|
| `Forge-Room-Early-Preview-0.110.0-alpha.2-windows-x64.zip` | `aad963a6a951834297ade4e43872b9875d056b779b16a5eeef981534bf7c86cc` | Matches downloaded SHA256SUMS.txt and GitHub asset digest |
| `Forge-Room-Early-Preview-0.110.0-alpha.2-windows-x64-setup.exe` | `5462b2bc0d018959668e20641e9699846213b5e95f9c6f83f5d78109a4a94712` | Matches downloaded SHA256SUMS.txt and GitHub asset digest |

ZIP CRC/integrity: **PASS**, all five members read, no bad member. Embedded PE RT_GROUP_ICON/RT_ICON data: **PASS** for both portable app EXE and setup EXE. Every 16/24/32/48/64/128/256 frame has exactly the same payload SHA-256 as `design-assets/forge-room-brand/forge-room.ico`; this compares embedded resource bytes, not Windows icon-cache appearance.

Release metadata states unsigned, x86_64-pc-windows-msvc, app EXE 19,525,632 bytes. INSTALL_VERIFY.json reports noninteractive hosted install/upgrade/uninstall checks and explicitly `interactiveInstallerUiVerified=false`. Those are publisher-supplied records, not a fresh install trial by this reviewer. Binaries were not installed or launched for this release check.

## Commands, evidence, and limits

- `git diff 5fa4de7..HEAD -- native/forge-room-app/src/...` and direct source review; initial HEAD and later logo-only commit distinguished above.
- `cargo test --locked --manifest-path native/forge-room-app/Cargo.toml`: initial sandbox run **89 passed, 1 failed, 5 ignored**. Local-remote Git test failed with Windows Git sh `couldn't create signal pipe, Win32 error 5`; isolated rerun outside sandbox **1 passed**.
- After the logo-only commit and restoring original generated app build output: `cargo clean --manifest-path native/forge-room-app/Cargo.toml -p forge-room-app`, then `cargo test --locked --offline --manifest-path native/forge-room-app/Cargo.toml` outside sandbox: **91 passed, 0 failed, 5 ignored**. Full output: `measurements/t13-cargo-test.txt`.
- `python design-assets/forge-room-ui-spec/tools/t13_source_probes.py`: adds review probes only to disposable app copies, then invokes offline Cargo tests: **8 passed**. Final transcript: `measurements/t13-source-probes.txt`. Harness iterations needed theme assets, egui 0.37 `run_ui`, texture-delta handling and byte-index conversion; those harness failures are not app defects. Earlier copying under temp was sandbox-denied and retried with approval. The first probe harness reused a generated test-binary name; original generated app output was subsequently cleaned/rebuilt, and the saved script now uses the distinct `forge-room-t13-review-probe` name.
- `gh release view ... --json tagName,targetCommitish,publishedAt,url,assets` and `gh release download ... --dir <new-temp-folder>`. Initial network call failed through sandbox proxy; approved retry downloaded the five assets.
- `python design-assets/forge-room-ui-spec/tools/t13_release_check.py`: hashlib SHA-256, ZipFile.testzip/member reads, pefile icon-resource extraction and canonical frame comparison. Detailed results: `measurements/t13-release-check.json`.

**Not tested:** live GUI, pointer/keyboard acceptance, full minimum-window/DPI/theme matrix, ignored PTY/benchmark tests, fresh interactive install/upgrade/uninstall, shortcut/icon-cache refresh, external protocol handlers, or timing reproduction of finding 7. No clippy/rustfmt claim is made by this read-only review.

**Handoff priority:** preserve pending Lobby state first; enforce shared outbound URL validation; fix graph/tab width budgets and syntax boundary; then address IPv6 parsing and status generation binding.
