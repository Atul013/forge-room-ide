# Code section: measured reference and ember handoff

Source: `C:\Users\Icarus\Downloads\WhatsApp Image 2026-11-02 at 13.16.42.jpeg`, 1600 x 900. Recolored delivery: `code-section-recolored.png`, 1600 x 900. The original screenshot governs geometry; the generated recolor is a visual palette reference, not a pixel-exact redraw. Edge measurements are approximate +/-2 px. Font sizes are visual estimates; source font metadata is unavailable.

## Layout at 1600 x 900

| Surface | Reference bounds / size | Native implementation value |
|---|---|---|
| Title bar | y=0..35 | **35 px** outer height |
| Left navigation rail | x=0..110 | **110 px** width; labeled rail, 36 px main item pitch |
| Room Pulse | x≈116..1598, y≈38..79 | **41 px** height, 4 px top gap, radius 6, 1 px border |
| Explorer | x≈116..326, y≈88..868 | **210 px** width, 6 px gap after rail |
| Explorer header | y≈89..124 | **35 px** height |
| Center editor group | x≈330..1228 | **898 px** outer width at this window size; flexible remainder at other widths |
| Editor tabs | y≈90..123 | **33 px** height; 1 px separators |
| Breadcrumbs | y≈124..151 | **27 px** height, 13 px text |
| Code canvas including minimap | x≈330..1023, y≈151..614 | **693 px** width at baseline |
| Gutter | x≈331..384 | **53 px**; align line numbers to the right |
| Minimap | x≈933..1023 | **90 px** width; optional collapse below narrow-window breakpoint |
| Embedded web/product preview | x≈1025..1227, y≈151..614 | **202 px** width; 2 px splitter gap; same vertical bounds as code canvas |
| Preview title/address | y≈153..220 | Approx. **32 px title + 32 px address**; real preview state only |
| Bottom terminal group | x≈330..1228, y≈621..868 | **247 px** outer height, **34 px** tab/tool header |
| Right collaboration panel | x≈1238..1598, y≈89..868 | **360 px** width; 10 px gap from center group |
| Right panel Team/Agents/Tasks/Activity tabs | y≈90..128 | **38 px** height |
| Status bar | y≈871..900 | **29 px** outer height, including divider |

Baseline horizontal budget: 110 rail + 6 gap + 210 Explorer + 4 gap + 898 center + 10 gap + 360 collaboration + 2 right edge = **1600**. The preview lives *inside* the center editor group, so don't subtract it twice. At narrower widths, preserve useful code width by collapsing the preview/minimap, then optionally the collaboration panel or Explorer; never let fixed columns push controls off screen. Preserve adjustable splitters. Terminal is below the full center group, not below the Team panel.

## Visual tokens

| Role | Value |
|---|---|
| Canvas | #0D1115 |
| Rail | #13171A |
| Panels/header bands | #161B1E |
| Card/input surfaces | #1B1E23 |
| Hairline border | #33363A, 1 px |
| Main text | #EFEFEE |
| Secondary text | #B3B5B9 |
| Muted ember focus/icons | #D99A93 |
| Primary buttons | #D9837F; text #151315 |
| Selected item/tab fill | #3C2526 / #372222 |
| Forge symbol warm neutral | #E8E6DD |
| Doorway highlight | #D67A3F |

Keep purple agent identities and amber warnings distinct. Presence and active indicators use ember in this visual direction; errors still need distinct error treatment and text/icons. Colored code syntax is separate from the UI accent. Do not tint every code token ember.

## Component rhythm and hierarchy

- UI labels about **13 px** regular, section titles **14 px** semibold, editor/terminal **13-14 px monospace** with **17 px** row pitch. Use the installed Segoe UI/Consolas setup rather than inserting raster text.
- Explorer rows around **21 px** high; nested indentation **16 px** per level; compact 14-16 px file icons. Selected file receives the dark ember wash and legible text.
- Editor tab horizontal padding **14 px**, close target at least **22 px**. Keep dirty/modified state real.
- Team cards have **6-8 px radius**, **10-12 px inner padding**, **40 px avatars**, and **53 px participant-row pitch**. Names are semibold; branch/worktree badges are secondary. Keep action controls accessible without hover.
- Terminal header tabs share the center-group width. The source code/error/terminal text is illustrative; actual document content and process output must remain live.

## Brand integration and behavior boundaries

Use `forge-mark-color.svg` for Forge title/status branding and the ICO/PNG exports for executable/window/installer resources. Keep Aether's project mark and purple agent mark separate. The supplied reference calls for Code to open the editor workspace, not the Room Lobby.

Keep native Rust architecture. A live product preview needs an explicit native-compatible implementation choice; a mocked screenshot is not a working browser. For an unavailable preview or team capability, show a truthful empty/disabled state rather than sample localhost content, invented collaborators, test progress, or approvals. Keep Code navigation, editor save, terminal input and source control usable during layout work.

The supplied reference uses a 1600-wide baseline. At the existing 1550-wide comparison size, keep 110 / 210 / 360 px side columns and let the center shrink to **848 px** before optional panel collapse. Heights remain fixed UI units, converted from screenshot pixels using runtime pixels-per-point. Recheck at the user's actual display scale before claiming visual acceptance.
