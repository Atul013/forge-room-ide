# Room Lobby: final measured review of v3

Mailbox T5, 2026-11-02. Reference: T1 image **1672 x 941**; app: `artifacts/lobby-compare/lobby-v3.png`, **1550 x 975**. Compare using the same uniform width scale **1550/1672 = 0.92703**, retaining the app's extra vertical capacity. No Rust edits. Only clearly visible differences or geometry deltas greater than approximately 4 px are listed below.

Evidence: `measurements/lobby-diff-3.json`, including source hashes, screenshot landmark boxes and RGB samples. Regenerate with `python -B design-assets/forge-room-ui-spec/tools/measure_lobby_diff.py --third`. Box edges have approximately +/-2 px uncertainty. Font metadata is not recoverable exactly; previously proposed typography remains a visual estimate. This review concerns the supplied capture, not later live edits or interactive acceptance.

## What now matches from LOBBY_DIFF_2

| Previous finding | v3 evidence | Result |
|---|---|---|
| Invite overflow | All right-column card edges now align at **x=1532**; Invite is **454 x 85**, with all controls inside | Fixed |
| Team/agent unequal bottoms | Both top **494**, bottom **710** | Fixed alignment |
| Inflated Add-button header bands | Both populated cards now have ~**38 px** header bands instead of 47 | Within ~2 px of 36 target; accepted at this review threshold |
| Tall status bar | Outer visible band **y=949..975 = 26 px**, versus prior 39 and target 29 | Fixed; remaining 3 px is below threshold |
| Main action uses wrong role | Enter Room sample **#D9837F** at (1400,107) | Fixed exactly |
| Main action width | **165 x 45 px** | Fixed exactly |
| Overflow action | **40 x 40 px** | Fixed |
| Short Lobby tab | **104 px wide** | Fixed exactly |
| Member avatars | **41 x 41 px** | Fixed exactly |
| Duration button | Now plain clock-associated value with no filled button | Fixed |
| Always-visible Add item button | Replaced by compact goal-header plus action | Fixed |
| Agent identity color | Agent marks are purple on purple-tinted circular backgrounds | Fixed color role |
| Missing Pulse framing | Rounded outlined strip **41 px high** | Fixed |
| Wrong title-bar brand geometry | Angular double-chevron SVG is now used | Fixed to delivered T4 asset |
| Column ratio and gaps | Left **922**, gap **12**, right **454**: right 33.0% excluding gap | Remains matched |
| Main logo tile/countdown | **103 x 84 / 242 x 83** | Remain matched |
| Left content inset | Card x=144 minus rail boundary x=118 = **26 px** | Fixed inset; rail width still differs below |

## Remaining differences, largest first

### 1. Goal and overview still exceed their compact-height target

Goal is now **444 x 180 px**, a substantial improvement over 443 x 225, but **15 px taller** and about **19 px narrower** than the proposed ~463 x 165 at the current 922 px overview width. Overview is **922 x 234**, versus target **220 px high**, excess **14 px**. This pushes the next rows down even though the app has enough room to display them.

Use **goal outer width = overview width * 0.503**, approximately **464 px**, with an outer height baseline **165 px** for this two-line description/four-task state. Keep the current plus action. Allocate checklist **22 px total pitch**, text **13 px**, checkbox **17 px**; remove accumulated vertical gaps around the title/description/list. The displayed task baselines are about 24 px apart; reducing three inter-row intervals by 2 px saves 6 px, and the remaining ~9 px should come from title/description padding. Keep key/value **28 px total pitch**. Do not clip longer real goals: allow editing/longer content to grow or scroll explicitly.

### 2. Paired team/agent row still 15 px too tall

Alignment is fixed, but both populated cards are **216 px high** instead of the scaled-reference baseline **201**. Three member rows occupy approximately 53 px pitch, instead of target **51 px**. Use a **201 px populated minimum** with a **36-38 px header**, **51 px row pitch**, and shared remaining body padding. V3 avatar size is correct; do not shrink it again. Agent card keeps the same outer baseline without adding a fake third agent.

Together, the overview and member-row changes would move the milestone row up by approximately **29 px**, addressing the largest remaining vertical rhythm difference. Keep gaps at **12 px**.

### 3. Rail is still 11 px wider than the proportional reference

Reference rail **115/1672 = 6.88%**; v3 **118/1550 = 7.61%**. Use **107 px** at 1550-wide for the same proportion, preserving the now-correct **26 px** content inset, so cards begin near **x=133** rather than 144. Keep the existing 67/33 columns responsive as the content area expands.

Current top rail still contains Search, and the lower group adds AI Assistant and Terminal, unlike this reference screen. This is a clearly visible information-architecture difference. For strict reference composition, put these extra destinations in an overflow or context group while preserving their functional/keyboard access. Do not remove features solely for screenshot resemblance.

Cloud and settings remain two vertically stacked bottom rows. Reference has them side-by-side: use a **30 px bottom row**, **20 px icons**, **14 px horizontal gap**, accessibility above.

### 4. Active navigation/tab backgrounds remain gray-brown rather than smoky ember

These are visible hue differences across the selected regions, not 1-channel pixel noise:

| Role | v3 direct sample | Reference direct sample / target | Use |
|---|---|---|---|
| Active rail fill | **#372F30** at (90,140) | Reference #3E2627; local mode #3C2526 | **#3C2526** |
| Active tab fill | **#2E2729** at (166,205) | #372222 at (166,215) | **#372222** |

Set explicit active-fill tokens instead of deriving both by mixing the foreground into a surface. Preserve the matched #D9837F primary action and the muted focus/outline color.

### 5. Milestone markers and connector still differ visibly

Pending rings are approximately **28 px diameter**, in-progress ring approximately **30 px**, while the scaled reference target is **22 px**. Use **22 px visible diameter across all statuses**, with an independent **34 x 34 px hit area**. Completed circles already look close; normalize them rather than enlarging to the pending size.

The connector is still nearly invisible against the card and has no clearly ember completed segment. Use **2 px stroke**, neutral pending segment **#A9ADB2**, completed segment **#D99A93**, aligned through node centers. Preserve actual completion/in-progress state and the 40% bar.

### 6. Configuration card and compact Invite widths

Configuration is **454 x 252**, target approximately **458 x 239**: **13 px excess height** remains. Keep readable checkbox boxes, but enforce **31 px total field pitch**, **26 px sharing-row pitch**, and **13 px regular** value text. The Setup card is already close in outer height; do not apply a blanket reduction there.

Invite containment is fixed, but its button is about **91 x 31** instead of target **78 x 31**, and dropdown is about **93** instead of **85 px**. This reduces email input space. Within the actual body width, reserve **78** for Invite, **85** for Member, two **8 px** gaps, and give the remaining width to the input. Control height/outer card height are now close enough; preserve them.

### 7. Remaining obvious graphic differences

Reference goal subpanel has a soft ember ribbon along its lower-right corner; v3 has a plain surface. If exact reference graphics are required, commission a separate asset task specifying filename, dimensions and alpha handling. T5 does not authorize generation of an unspecified ribbon asset.

The delivered T4 mark is an A-like double-chevron at both title and project tile. The tiny original top-left reference mark is asymmetric and closer to an angular slash/chevron. **The current mark matches the T4 request**, so treat exact tracing of that separate tiny reference symbol as optional new art direction, not a failed integration.

## Accepted differences: real data and state

- Countdown time and event timestamps advance; they should not equal the reference's frozen values.
- Start/end dates and UTC labels reflect the saved demo configuration. The reference uses IST. Display the actual selected timezone and configuration, rather than substituting Nov 2, 10:00 AM to imitate the image.
- Branch displays `no repository` for this demo workspace; reference `main` requires a real repository. Do not hardcode a branch.
- Human initial avatars are valid fallbacks when profile photos are unavailable. Do not synthesize portraits.
- `Online` versus `In room`, member notes/roles, room participant count, agent readiness/worktree labels and milestone completion must come from state. Styling may be matched without fabricating presence.
- The current room has no setup-warning badge in the Pulse strip. Show that warning only when setup validation actually reports a problem.
- Search shortcut text must match the wired command; don't replace Ctrl+Shift+P with the reference's Ctrl+K cosmetically.
- Workspace path, app version, status counts and extra navigation destinations represent the current product. Preserve correct values and access even when adapting their placement.

Items below the requested threshold are deliberately not reopened: title height differs by ~2 px, rendered status height by ~3 px, regular card-header height by ~2 px, and input fill is within a channel of its target. No pixel-perfect or interaction-complete claim is made. The highest-value final adjustments are the goal/member row heights, 107 px rail, explicit selected fills, and uniform milestone markers.
