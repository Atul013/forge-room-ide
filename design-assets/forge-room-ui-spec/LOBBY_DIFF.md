# Room Lobby: reference versus app, prioritized visual differences

Measured 2026-11-02 for mailbox T2. This is a handoff for Claude; no Rust files were edited.

## Evidence and scaling

- Reference: `C:\Users\Icarus\.codex\generated_images\01a19bb3-cacd-7363-a6ab-0171bbb54092\exec-3b3cc3a1-3bd6-44ba-8fcd-aecf6e97295b.png`, **1672 x 941 px**.
- App: `artifacts/lobby-compare/lobby-current.png`, **1550 x 975 px**.
- Reproducible image hashes, landmark rectangles, normalized coordinates, and RGB samples: `measurements/lobby-diff.json`; regenerate with `python design-assets/forge-room-ui-spec/tools/measure_lobby_diff.py`.
- Boxes below are native screenshot pixel bounds, with approximately **+/-2 px** uncertainty at antialiased edges. Reference colors vary across the image; sampled values are not original design tokens.
- Original font metadata is unavailable. Reference font sizes/weights below are **optical estimates and proposed implementation values**, not recovered font declarations. Current numeric font sizes were checked in `src/lobby.rs` and `src/main.rs`; `src/fonts.rs` already supplies Segoe UI and a real semibold family.

Compare widths as fractions of the window, and internal card sizes as fractions of the content area. For a reproducible 1550-wide target, use the uniform reference scale **s = 1550 / 1672 = 0.92703**. Scale measured geometry uniformly, not x/y independently: the reference becomes 1550 x 872.34, leaving approximately **103 px additional vertical capacity** in the app. Do not stretch cards to consume that capacity. The target values below are rounded pixels at this window width; convert to egui points by dividing by the actual pixels-per-point. The source screenshot appears close to 1 px/point, but runtime DPI was not independently inspected.

The largest mismatches are vertical density, column proportions, and the saturated red. Fix those before decorative detail.

## 1. P0: card density and clipped invite controls

| Element | Reference | Current app | Exact proposed value at 1550-wide |
|---|---|---|---|
| Overview bounds | (143,260)-(1146,497), 1003 x 237 | (146,269)-(1010,585), 864 x 316 | Height **220** for this six-row layout; width responsive |
| Setup card | 494 x 228 | 514 x 269 | Height **212** |
| Configuration card | 493 x 258 | 514 x 313 | Height **239** |
| Invite card | 494 x 95, all controls visible | Starts y=875; only header visible before y=930 status bar | Height **88**, keep all input/action controls inside the viewport |
| Overview row centers | Approximately y=325,356,385,415,445,473; ~30 px pitch | Approximately y=342,384,426,468,514,556; ~42-44 px pitch | **28 px total row pitch**, including item spacing |
| Shared key/value allocator | Reference visually compact | `kv()` requests 30 px, with additional vertical spacing in callers | Row allocation **26 px**, spacing **2 px**; do not add a second 12 px gap per row |
| Configuration field height | ~26 px, row pitch ~33 px | ~30 px, pitch ~42 px | Control **25 px**, row pitch **31 px** |
| Sharing checkboxes | ~18 px square; ~28 px pitch | Tiny dark marks; ~32 px pitch | **17 px** square, **26 px** row pitch |

Keep the 12 px inter-card gap. Reference-scale stack targets after the tab divider: overview y≈241-461; team row y≈470-671; milestones y≈680-822. The right stack should finish around y≈798, well above the proposed status bar at y=946. These are a diagnostic baseline, not absolute positioning instructions: build rows from measured heights, and let content grow or scroll when required. The current clipped invitation cannot be explained by the app having a shorter window; it is actually taller.

## 2. P0: column proportions

Reference content spans approximately x=143..1653. Its overview column is **66.4%** of this span; right column **32.7%**, with ~13 px gutter. App content spans x=146..1536: left is **62.2%**, right **37.0%**, gutter 12 px. The right panel takes too much width, squeezing both team cards and the project goal.

Use **right_width = (content_width - 12) * 0.33**, **left_width = content_width - 12 - right_width**. At the proposed inner bounds x≈133..1532, this gives left≈929, gutter=12, right≈458. Keep the existing 320 px minimum only at narrower window sizes; if two columns no longer fit, stack instead of letting controls overflow.

In the left column's team/agent row, the reference team:agent widths are ~478:515, or **48.1:51.9** after the gutter. Use **0.48 / 0.52**, not a strict half split. Align the two outer card bottoms in the populated reference state. Do not add fake members to achieve that height.

## 3. P0: muted ember and surface hierarchy

The app button and logo sample exactly **#F04440**. This is conspicuously brighter and more saturated than the requested muted result. Do not use the same saturated color for the logo, countdown, outline controls, active tabs, and status labels.

| Role | Reference evidence | Current evidence | Use this flat token |
|---|---|---|---|
| Primary button | #D9837F at (1480,130), local modal pixel #D9837F | #F04440 at (1400,120) | **#D9837F**, hover **#E19A94**, text **#151315** |
| Logo/accent strokes | Median bright warm pixels ~#F78D84 inside logo box | #F04440 | **#D99A93** for general accents; logo **#E6A098** |
| Active rail fill | Sample #3E2627; local patch mode #3C2526 | #3D1919 | **#3C2526** |
| Active tab fill | #372222 at (166,215) | #151718, no ember wash | **#372222** |
| Active underline | Dusty salmon, ~2-3 px | Saturated red, ~2 px | **#D99A93**, **2 px** |
| Countdown | Warm amber/salmon; selected bright pixel median #F49D78 | #F04440 | **#F49D78**, separate from primary accent |
| Canvas | #0D1115 sampled at (130,200) | #0D0F10 at (135,250) | **#0D1115** |
| Rail | #13171A sampled at (50,65) | #0B0D0E | **#13171A** |
| Card body | #101317 sampled at (500,470), patch mode #0F1217 | #151718, flat | **#101317** |
| Card header | #161B1E at (900,280) | #151718, same as card body | **#161B1E** |
| Input | #1B1E23 at (1480,354) | #1B1D1E | **#1B1E23** |
| Border | Direct edge sample #33363A at (450,260) | #25282A at (450,269) | **#33363A**, **1 px** |
| Primary text | Reference bright strokes approach white | App ~#F5F5F6 | **#EFEFEE** |
| Secondary text | Antialiased and variable | Theme-dependent | **#B3B5B9** (implementation choice, not a uniquely measured source value) |

Use a flat approximation for the reference's subtle shading first. Preserve amber warnings and purple agent identity; avoid changing every semantic color to ember. An empty milestone track should remain neutral. Apply ember to completed milestones only when the real data says they are complete.

## 4. P1: left navigation rail

| Property | Reference | App | Proposed value |
|---|---|---|---|
| Rail width | **115 / 1672 = 6.88%** | **132 / 1550 = 8.52%** | **107 px** at 1550-wide |
| Active tile | x=4..110, y=131..167, **106 x 36** | x=6..126, y=141..179, **120 x 38** | Insets **4 px**, height **34 px**, width rail-8 |
| Standard item pitch | ~37-38 px in the main group | ~42 px | **35 px** |
| Wrapped Source Control item | ~52 px reserved | 48 px source allocation plus surrounding spacing | **48 px** total |
| Label | Optical ~14 px regular; room label brighter | Source 13 px, inactive items dim | **13 px Segoe UI**, regular; active #EFEFEE, normal #B3B5B9 |
| Icon | ~20 px | 20 px | **19 px**, center x=24 |
| Label start | ~48 px from window left | x=48 | **44 px** from left after rail resize |
| Active corner | ~5 px | 8 px | **5 px** |
| Bottom utilities | Cloud and gear share the bottom horizontal row; accessibility above | Each utility occupies a separate vertical row | **20 px** icons, bottom row height **30 px**, 14 px horizontal gap |

Reference has no Search row between Code and Rooms, and no AI Assistant/Terminal entries in this page rail. Extra app functionality is legitimate; collapse those into an overflow or another navigation group if reference fidelity is required, keeping keyboard/menu access. Do not silently remove working commands. Active fill is #3C2526; the thin bright leading mark is optional because fill and label already convey selection.

## 5. P1: title bar and Room Pulse strip

| Element | Reference | App | Proposed value |
|---|---|---|---|
| Title bar | 36 px, **3.83% of height** | 46 px, **4.72%** | **34 px** at target width |
| Brand/project/branch chips | ~28-30 px high, separated by thin dividers | Bulky filled brand chip, 34 px high | Chip **28 px**, internal horizontal pad **10 px**, gap **8 px**, divider 1 px #292D32 |
| Brand text | Optical ~16 px semibold | 16 px true semibold in source | **15 px semibold** |
| Secondary chip text | Optical ~14 px | ~13-14 px | **13 px** |
| Window control allocation | Compact ~36 px slots | 46 px slots | **34 px** wide each, bar height |
| Search field | Approximately 270 x 28 | Approximately 310 x 32 | **250 x 26**, keyboard hint in a small inset capsule |
| Brand mark | Angular Forge mark | Circular plus mark | Use the actual Forge brand geometry when supplied; this is a geometry mismatch, not a recolor |
| Pulse strip | (120,40)-(1668,84), 44 px high; bordered rounded container | Approximately y=47..97, label and setup alert floating without outer frame | **41 px** high, top gap **4 px**, left/right inset **5 px**, radius **6 px**, border #33363A |
| Pulse items | Icon, Room Pulse, event items with timestamps/dividers | Only title visible in this screenshot | Render real events in one row with **18 px** gap, **12 px** timestamp text; truncate low-priority events before the setup alert |

Reference top-avatar stack is visually richer, but current room has one member. Use genuine participants; do not fabricate the four reference portraits. A single member is a valid reduced state.

## 6. P1: header block, logo, title, chips, countdown

| Element | Reference | App | Proposed value |
|---|---|---|---|
| Logo tile | (144,101)-(255,191), **111 x 90**, ratio 1.23 | **92 x 92** | **103 x 84**, radius **7 px**, dark neutral/ember-tinted fill #20191B, border #68403D |
| Mark | Soft salmon, ~55 x 49 within tile | Brighter red, similar footprint | **51 x 45**, center in tile, #E6A098 |
| Title | Aether, optical ~34 px semibold | Long workspace name, source 30 px `.strong()`; looks regular | **32 px**, actual `fonts::semibold()` |
| Subtitle | Project Lobby, optical ~22 px regular | Source 18 px, dim gray | **20 px**, #CFD0D1 |
| Tile-to-title gap | ~24 px | 16 px | **22 px** |
| Tagline | Reference starts x≈543, optical ~17 px | Starts x≈477, source 15 px; width fixed 360 | **16 px**, reserve responsive flexible width |
| Header pills | Height ~29 px; text ~14 px; icon ~16 px | Height ~31 px; subdued dark fields | **27 px** high, font **13 px**, icon **15 px**, radius **4 px**, gap **12 px** |
| Hackathon pill | Dusty outline and very dark ember fill | Highly saturated red outline/text | Outline **#B86259**, fill **#2C2122**, text **#D99A93** |
| Countdown card | **261 x 89** | **260 x 88**; proportionally larger | **242 x 83**, radius **6 px** |
| Countdown digits | Optical ~34 px, semibold, amber/salmon | Source 34 px regular, saturated red | **32 px semibold**, #F49D78 |
| Countdown caption/units | ~14 / 11 px | 13 / 11 px | **13 / 10.5 px** |
| Enter Room | **178 x 49** | **140 x 42** | **165 x 45**, font **16 px semibold**, arrow **20 px**, radius **6 px** |
| Overflow | ~43 px square | ~37 x 26 | **40 x 40**, center vertically with Enter Room |

Do not force a long workspace name into the Aether-sized title slot. Fix the countdown + action group at the right, give the name block a maximum width, elide long names with a tooltip, and let the tagline use the remaining width. Stack the tagline below the title at narrower widths. Values such as Solo versus Multiplayer and the countdown itself reflect actual room state and are not visual defects.

## 7. P1: tab row and outer page insets

Reference active tab is **112 x 40**, with tinted background; app is **100 x 40**, with nearly neutral background. Reference row starts x=143 and divider y=245; app starts x=146, divider y=257. At target scale use active tab **104 x 37**, radius **5 px** on top corners, font **15 px**, underline **2 px #D99A93**, fill **#372222**. Horizontal tab gap **12 px**; use approximately **104 px** minimum width for short tabs and measured label width + **28 px** for long tabs. Keep the divider at row bottom, 1 px #33363A.

Reference first card is **28 px** from the rail boundary; app is 14 px. Use **26 px left content inset** after the resized rail; right inset **18 px**. The outer page frame/pulse may use a smaller 5 px inset. Below the tab divider, use **14 px** before cards, and between header tile bottom and tabs use **13 px**. Do not reuse the same 14 px inset for every nested surface.

## 8. P1: card headers, borders, radius, padding, typography

Reference outer cards have ~**9-10 px radius**, a clearly visible 1 px border, a ~**38-40 px header band**, and a horizontal separator. App radius **10 px** is already close; the bigger issue is that its header and body are the same fill, padding is **14 px everywhere**, and there is no full-width header separator.

Use **radius 9 px**, **border 1 px #33363A**, **header height 36 px**, header fill **#161B1E**, body **#101317**, separator **1 px #33363A**. Header inset **14 px horizontally / 8 px vertically**; body inset **12 px horizontally / 8 px vertically**. Nested goal panel radius **6 px**, inset **10 px**, border #33363A. Header icon **18 px**, title font **16 px real semibold**, icon-to-title gap **8 px**. Reference small card titles look semibold; app uses `.strong()` on the normal family, so use the existing semibold family explicitly for the intended hierarchy.

Reference goal subpanel is **504 x 178**, current is **410 x 138**. With the column-width fix, use about **50.3% of the overview width**, an **inside width near 466 px**, and height **165 px** when the goal and four checklist items exist. Let an empty goal display its real placeholder; the absent ribbon and task list are content/decorative differences, not reasons to seed fictional project tasks. A ribbon asset would require a separate mailbox asset task specifying its filename and dimensions.

## 9. P2: key/value alignment and form treatment

In the reference setup card, labels begin ~49 px after its left edge; values begin ~168 px after it, or **34% of card width**. App label starts ~49 px in, value starts ~179 px in, approximately **35%**. The current ratio is close; do not solve the density issue by squeezing labels further.

Use icon center **22 px** from left card edge, label start **45 px**, and value start **34% of inner body width**. Body label/value font **13 px**, regular. Row pitch **28 px** for read-only overview; **31 px** for controls. Use compact bordered inputs **25 px high**, radius **4 px**, fill #1B1E23, border #3A3F45. Reference repo URL is a blue link with GitHub/external-link actions; current screenshot shows an editable placeholder. Preserve empty-state editing, but render the configured URL as a real clickable link once present.

## 10. P2: avatar/member rows and empty-state alignment

Reference avatar is approximately **44 x 44**, app **44 x 44**. Relative to window width these are 2.63% and 2.84%, so use **41 px** at 1550-wide. Reference presence dot ~11 px with dark ring; use **10 px**, 2 px dark ring, pinned bottom-right. Member row pitch ~55 px in reference; target **51 px**. Name **14 px semibold**, secondary line **13 px**, badge **18 px** high / **11 px** text, avatar-to-text gap **12 px**.

Reference puts Online and role notes on the same secondary line, just under the name. Current Online is detached below with generous whitespace. Start the secondary line **20 px** below the name baseline; keep it inside a **51 px** row. Put the overflow action in a fixed **24 x 24** slot at the far right.

Actual app has **one member / zero agents**, reference has three / two. Do not interpret missing reference people, agent badges, or portraits as layout errors. Keep a useful empty-agent message and add action. Equalize the two card minimum heights to **136 px** for the current empty/single-member state, or to **201 px** for a populated three-row state; avoid the current 147 px team card next to a 96 px agent card if matching the paired-card rhythm.

## 11. P2: milestone track and progress inset

Reference milestone card **1005 x 153**; current **864 x 174**. Use height **142 px** at target width, after fixing column widths and card padding. Reference track occupies approximately **70%** of inner width, progress card **25%**, separated by ~14 px. Current progress inset is fixed **220 px**, consuming too much of the smaller left column. Use progress width **24.5% of inner card width**, minimum **190 px**, maximum **226 px**, gap **12 px**.

Reference circles are ~**24 px diameter**, current pending circles ~**28 px**. Use **22 px** visible diameter, 1.5 px outline, **2 px** track stroke, and keep the hit rectangle **34 x 34 px**. Reference center-to-center spacing ~144 px; target **133 px** for the reference-shaped track. Prefer evenly spaced centers over a hardcoded step. Pending track #A9ADB2, done segment #D99A93; pending circle fill #101317. Do not use the nearly invisible card-border color for the entire pending connector.

Use **12.5 px semibold** milestone names, **12 px** status, **11.5 px** dates; relative to circle center, name baseline around **+29 px**, status **+48 px**, date **+66 px**. Progress inset target around **226 x 88**, radius **6 px**, inset **11 px**; progress bar **14 px** high, radius **7 px**, track #343A40, fill #D99A93. Current fill height is 12 px. Reference shows 40% and completed steps; app shows genuine 0%. Keep 0% until state changes.

## 12. P2: status bar

Reference bottom bar y=910..941, **31 px / 3.29% of height**. Current y=930..975, **45 px / 4.62%**. Use **29 px high**, 1 px top border #33363A, fill #111519, text **12 px #B3B5B9**. Icon size **14 px**, gap **8 px**, chip height **24 px**, chip radius **4 px**. Keep the current real branch/room/count information, but truncate the long opened-folder message before compressing the right status group. Reference puts cloud/gear in the rail, and editor encoding/language/notification items at the far right; do not invent UTF-8/LF/TypeScript status unless an open document actually provides those values.

## Suggested implementation order and acceptance

1. Change the column ratio, card row pitch and header/body spacing. Verify all invite controls fit at **1550 x 975**, without horizontal overflow.
2. Replace #F04440 with the role-specific ember tokens above; preserve amber timer/warnings. Add the card header band and separator.
3. Compact rail/title/status bars; fix tile/title/action sizes and long-name behavior.
4. Tune font families, member rows, milestone geometry and input heights with the actual populated/empty room states.

Capture another screenshot at **1550 x 975** and compare normalized landmark ratios against `measurements/lobby-diff.json`. Then check a narrower window for header overflow and a three-member/two-agent room for card height. This document verifies a static screenshot comparison, not the app's interaction behavior, accessible contrast across every state, or the result of changes made after the captured screenshot.
