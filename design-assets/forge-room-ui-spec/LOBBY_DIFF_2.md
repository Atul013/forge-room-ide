# Room Lobby: second measured comparison, remaining differences

Mailbox T3, 2026-11-02. Reference remains the T1 generated image, **1672 x 941**. New app capture is `artifacts/lobby-compare/lobby-v2.png`, **1550 x 975**. No Rust files edited.

Measurements and source hashes: `measurements/lobby-diff-2.json`. Reproduce with `python -B design-assets/forge-room-ui-spec/tools/measure_lobby_diff.py --second`. Native screenshot landmarks have approximately +/-2 px edge uncertainty. Reference font metadata remains unavailable; proposed font values are optical estimates. Use the same width-normalized comparison as T2: **s = 1550 / 1672 = 0.92703**, uniform scaling, not independent x/y stretching. The app has about 103 px more vertical space than the uniformly scaled reference.

## T2 items now matched or substantially resolved

| T2 item | Evidence in v2 | Status |
|---|---|---|
| Left/right column ratio | Left x=140..1067 = 927 px; gutter 12; right x=1079..1536 = 457 px. Right is **33.0%** of widths excluding gutter. | Matched |
| Team/agent width split | Team 439 px, agents 476 px, 12 px gap; **48.0/52.0**. | Matched |
| Dark canvas and rail | Direct samples **#0D1115 / #13171A**. | Matched |
| Main border | Direct sample **#33363A**, 1 px. | Matched |
| Primary/secondary theme text | Theme declares #EFEFEE / #B3B5B9. | Matched tokens; screenshot visually consistent |
| Card hierarchy | Distinct header bands, separator, real semibold section names, 9 px outer radius. | Resolved structurally; small fill/height differences remain below |
| Header logo tile | **103 x 84**, target 103 x 84. | Matched size |
| Title/subtitle | Now approximately 32 px semibold / 20 px regular, as proposed. | Visually matched hierarchy |
| Countdown card | **242 x 83**, warm amber/salmon digits, no bright error red. | Matched size and color family |
| Primary action height | Enter Room **45 px**. | Matched height; width remains short |
| Tabs | Approximately 37-38 px high, muted underline, dark ember background. | Height/underline resolved; width/fill remain below |
| Key/value row allocator | Source uses **26 px**, rather than 30. | Correct allocator; total pitch still needs tightening |
| Invite visibility | All invitation controls are visible vertically. | Original clipping resolved; horizontal sizing remains below |
| Sharing checkboxes | Filled ember boxes with visible dark ticks; empty box outlined. | Resolved readability |
| Populated member/agent rendering | Three humans and two agents now render. | Previously a content-state difference, now directly comparable |
| Progress state | Completed checks, amber in-progress ring and 40% progress now visible. | State rendering confirmed in this capture |
| Saturated primary red | #F04440 is gone from main actions/logo/countdown. | Resolved palette family; exact role separation remains below |

The following list contains only remaining differences, largest first.

## 1. P0: project goal stretches the entire overview card

| Element | Reference at source size / 1550-wide target | v2 measured | Use |
|---|---|---|---|
| Overview card | 1003 x 237 / height **220** | **927 x 280** | **220 px** populated six-row baseline |
| Goal subpanel | 504 x 178 / height **165** | **443 x 225** | **165 px** baseline, width **466 px** within corrected column |
| Goal panel top offset | 48 / **45 px** below overview top | **46 px** | Keep current top alignment |
| Goal list pitch | ~24 / **22 px** | ~26 px | **22 px total** |
| Overview key/value pitch | ~30 / **28 px** | Mostly **30 px**, plus a larger gap around Duration | **28 px total**, including caller spacing and widget height |

The 60 px excess goal height is now the main source of accumulated vertical drift. The app shows an always-visible **+ Add item** button; reference does not. Keep adding available through an inline edit mode or the goal header action. If the button stays visible, include it in the 165 px budget or allow the panel to grow for editing only. Reduce checklist text from 14 to **13 px**, checkbox from 18 to **17 px**, and keep label-to-box gap **8 px**. A two-line goal description needs about **38 px**, followed by 4 x 22 px task rows, with the remaining space allocated to title and padding.

Reference goal width is ~50.3% of overview width. Current is ~47.8%. Set the goal **outer width = overview width * 0.503**, and size the key/value half from the remaining width after a **12 px** gutter; do not add inner-frame padding again to the requested outer width. Keep text editable, but align and wrap it inside that budget.

Reference has a lower-right ember ribbon, v2 has none. This is a decorative asset gap, lower priority than sizing; do not improvise it in code as part of this report. No image generation is requested by T3.

## 2. P0: invite row exceeds its card width

Reference invite card x=1164..1658, scaled target approximately **458 x 88**. Current card starts x=1079 but its right edge reaches the **1550 px window boundary**, whereas Setup/Configuration end at **1536**. The Invite button extends to about x=1538, leaving insufficient border/right padding. The invite row is borrowing roughly **14 px outside the intended card**, unlike the other right-column cards.

Constrain the card to the same **457 px** outer width as Setup. For a body with 12 px insets, use **W=433 px** available row width. Allocate **85 px** to the Member dropdown, **78 px** to Invite, **8 px** for each of the two gaps, and input width **W - 85 - 78 - 16 = 254 px**. Calculate from the actual inner width rather than mixing outer width with widget width. Invite should be **78 x 31 px**, not the **96 x 45 px** main-CTA treatment currently visible. Input/dropdown height **27-28 px**; center all three vertically.

Target outer card height **88 px**; current is approximately **100 px**. At narrow widths, wrap or stack explicitly before any control exceeds the right edge. Preserve the main Enter Room button's separate 45 px height.

## 3. P1: team card headers are inflated, and paired bottoms differ

Reference Team and Agents share a bottom edge and header height approximately **39 px**, or **36 px** scaled. In v2 both begin y=528; Team ends y=751 (**223 px** high), Agents ends y=698 (**170 px** high). The populated three-person/two-agent state leaves a **53 px** bottom mismatch. Their header bands are ~**47 px**, because add buttons expand the nominal card header; the Overview header is only ~38 px.

Use a **fixed 36 px card header band**, centered contents, **18 px** icons, **16 px semibold** titles, and compact Add buttons **27 px high**. Give each button approximately **4 px** vertical clearance inside the band; don't add 8 px padding above and below a 27 px button. Target Add Member **105 x 27**, Add Agent **99 x 27**, font **12.5 px**, radius **4 px**. Current Add Member is approximately **119 x 30**.

For this populated comparison, give both cards **201 px outer height**: 36 px header, 12 px combined body vertical insets, 3 x 51 px member-row capacity. Agent content still contains two real rows; leave the remaining capacity neutral rather than inventing a third agent. In a small/empty real room a lower shared minimum is valid, but both cards should use the same row-capacity rule.

Avatar diameter is still **44 px**; set **41 px**, presence dot **10 px** with dark ring. Team row pitch is approximately **53 px**; use **51 px**. Keep the now-improved two-line name/status arrangement.

## 4. P1: rail still too wide; page begins too near the rail boundary

Reference rail **115/1672 = 6.88% of window width**. V2 rail **126/1550 = 8.13%**. The target at 1550-wide remains **107 px**. Item height is now approximately **35 px**, which is close; do not redo that improvement.

Reference cards start 28 px after its rail; target inset **26 px**. V2 cards start at x=140, only **14 px** after its rail. Apply the two changes together: rail **107**, card/content left inset **26**, so first card x≈**133**. Right inset **18**. Keep the corrected 67/33 column ratio and 12 px gutter when this moves the content origin.

Active tile should use **4 px side insets**, height **34 px**, radius **5 px**. V2 tile is x=6..120, height ~35, more rounded. Active fill is currently **#372F30**, while reference sample is **#3E2627**, with patch mode #3C2526. Use explicit **#3C2526**, rather than mixing the accent into the activity background. Normal rail labels remain visibly dim; use **#B3B5B9**, font **13 px**, icon **19 px**. Do not recolor disabled actions to look enabled.

Search, AI Assistant and Terminal entries remain additional navigation versus this reference. Keep their real functionality accessible; move extras to overflow if exact page-rail density is the objective. Bottom cloud and gear still stack vertically; place them side-by-side in a **30 px** bottom row, **20 px** icons / **14 px** gap. Accessibility stays above.

## 5. P1: rendered status bar is 39 px, despite the intended compact setting

In the screenshot, status top is **y=936**, window bottom y=975, so the **visible status area is 39 px**. The reference is 31 px high at 1672-wide, target **29 px**. This is a screenshot finding; changing only a row's minimum height has not constrained the containing panel plus margins.

Set the **entire outer panel to 29 px**, including top border and frame margins. At 1550 x 975 its top should be **y=946**. Allocate a **24 px** active chip and **12 px** text, center vertically, ensure vertical frame padding plus child height does not exceed the outer budget. The current status fill is in the correct dark family; keep it. Preserve real room/branch/workspace values.

## 6. P1: custom buttons use the accent instead of the configured button color

Theme `button.background` is correctly **#D9837F**, but the actual Enter Room button samples **#D99A93** at (1400,96), matching the accent/focus color. The theme change therefore does not fully reach custom-painted buttons. Route main button paint to the button-role color, preserving #D99A93 for focus/outline accents. Reference button sample is #D9837F; target **#D9837F**, hover **#E19A94**, text **#151315**.

Enter Room width is **140 px** instead of target **165**, though height **45** now matches. Use **165 x 45**, actual **16 px semibold** text, 20 px arrow, radius 6. Current label looks lighter than reference. Overflow remains about **37 x 26**; make it **40 x 40** and vertically center both actions in the header group. Keep the already-correct **242 x 83** countdown and amber color.

Reference chip outline/text are more distinctly ember. Use Hackathon fill **#2C2122**, border **#B86259**, text **#D99A93**, height **27**, radius **4**. Current pill is in the right family but has a mostly gray outline.

## 7. P2: tabs are short and their active fill is too gray

Current Lobby tab x=140..233, **93 px wide**. Reference width 112, target **104 px**. Keep current ~37-38 px height and underline. Set short-tab minimum width **104 px**, horizontal gap **12 px**, font **15 px**, long-tab width measured text + **28 px**. Active fill sample **#2E2729**, reference sample **#372222**: use explicit **#372222**. Current active text/underline are already close; don't return to the original bright red.

## 8. P2: project/configuration controls and fill precision

Project Setup **457 x 208** is now close to target **458 x 212**; no wholesale height correction needed. Configuration **457 x 253** is still ~**14 px** taller than target **458 x 239**. Use **31 px total** field pitch, sharing checkbox rows **26 px**, and body font **13 px**. Keep the now-readable checkbox treatment; reduce boxes **18 -> 17 px** only for exact scale.

Reference-inspired flat body/header targets remain **#101317 / #161B1E**. Current samples are **#121519 / #181C20**; these are only slightly lighter, so tune after geometry. Input sample **#1B1E22** is within one channel of target #1B1E23 and is effectively matched. Borders #33363A already match; don't darken them again.

Overview Duration remains a filled button where reference has plain clock + text. Use a frameless **13 px** value in read-only display, retaining an accessible edit affordance when editing the room. This also removes the row's extra widget-driven height.

Reference repo branch `main`, current `no repository`, and different start/end dates are data differences. Do not hardcode the reference branch/dates to make a screenshot look closer.

## 9. P2: milestone geometry, progress proportions and agent identity

Milestone outer height **151 px**, target **142 px**. Pending marker diameter remains about **28 px**; use **22 px** visible diameter, keep the 34 px hit area. The completed filled circles are closer to target scale than pending rings; normalize all statuses to the same visible diameter. Track stroke **2 px** is appropriate.

Progress inset is now **220 x 92**, close to target **226 x 88**. Keep its responsive width near **24.5%** of inner width; reduce vertical height by **4 px**, progress fill **12 -> 14 px**, radius **7 px**. Pending connector is still very dark; use **#A9ADB2** for the remaining neutral segment, ember for completed segment. Milestone names **12.5 px semibold**, statuses **12 px**, dates **11.5 px**. Preserve the real 40% state.

Reference agent avatars use purple identity marks; v2 paints them ember like the project mark. If matching this visual role, use **#9E7BE8** agent mark on **#292537**, keeping presence ember. Do not reuse the project brand accent for every agent identity. Human initial avatars are a legitimate fallback for unavailable portraits; do not synthesize user portraits without a separate task.

## 10. P2: top bar/pulse framing and the pending Forge mark swap

Title bar is now **36 px** versus scaled target **34 px**, a minor 2 px difference. Keep readable chip sizing while removing the remaining bulky brand-button fill: reference brand sits in a lightly divided transparent slot. Use chip **28 px**, gap **8 px**, text **15 px semibold** for brand / **13 px** for other chips. Search still uses Ctrl+Shift+P instead of Ctrl+K, and the branch chip is absent for a room without a repo; those must reflect actual commands/state, not be changed cosmetically.

Room Pulse events now render, which resolves the missing-content issue. Reference strip has a **rounded full outline**, ~41 px at target width; v2 is an unframed row, roughly 38 px high. Give it **41 px** outer height, **6 px** radius, **1 px #33363A** border, 5 px side insets relative to the outer page. Keep the current real event text; reference event icons/timestamps are a detail, not a reason to fabricate activity.

The circular plus brand mark is still visible in this capture. **T4 is delivered** at `design-assets/forge-room-icons/svg/forge-mark.svg`, sprite ID `forge-forge-mark`: Claude can copy it and swap it in. It is the requested A-like double-chevron interpretation; the tiny original top-left symbol is more asymmetric, so do not call the asset a pixel-exact trace.

Post-capture mailbox update: Claude reports T4 is already copied and used for title/project/agent marks. Treat the brand-swap item as implemented after v2 and verify it in the next capture; do not repeat the swap based on this older screenshot. Purple agent identity remains a separate styling suggestion.

## Recheck order

1. Goal/overview height and invite width. Capture the whole right edge and ensure each right-column card ends at the same x.
2. Equal populated team/agent card heights, true outer status height, then rail/content inset.
3. Main button role/width, tabs and remaining control/track details.

Keep **1550 x 975** for the next comparable screenshot. Expected improvements should be visible in the outer rectangles, not just source constants. This is a review of the supplied v2 capture; it does not verify interactive behavior or later live app changes.
