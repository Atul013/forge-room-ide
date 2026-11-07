INTRO = r"""# Forge Room IDE — screenshot measurement and native UI handoff

Source: `source/reference.png`, **1672 × 941 px**. This is a measurement of the supplied flattened PNG, not a recovery of its original design file.

**Read the evidence labels before building.** `M` means a source-pixel observation (an envelope, mask, sampled RGB, or a distance between observations). `F` means a numerical fit to those pixels, with its method retained. `R` means a disclosed reconstruction setting for the SVG/native implementation. `U` means unavailable from the screenshot. Font identity, original nominal font sizes/weights, original layer opacity, source blur kernel, original corner-radius tokens, hidden content, and interaction timing are `U`. They are not silently assigned invented “exact” values.

The PNG is RGB, not RGBA; every stored pixel is opaque. A sampled composite hex cannot uniquely determine both a translucent foreground color and its alpha: `C = alpha × F + (1 − alpha) × B` has multiple solutions when F or B is unknown. The same limitation applies to blur versus a pre-blurred image. Background and border hex values below are **the actual observed composite pixels**; source-layer alpha remains `U`.

The coordinate origin is the screenshot's top-left; x increases right, y increases down. Rectangles are `(x, y, w, h)` with right and bottom excluded. Physical pixel centers are at half-integer positions. Contour envelopes were selected against the source and checked with ±3 px ridge scans; antialiasing makes the underlying vector edge ambiguous by approximately 1 px, or 2 px at soft boundaries. Text/icon ink bounds are exact integer envelopes of the documented threshold masks, not CSS/widget boxes. Actual widget padding and hit targets cannot be inferred from ink alone.

**Scale conversion:** under the requested assumption `pixels_per_point = 1.0`, `logical_pt = physical_px / 1.0`; thus 24 px = 24 logical pt, 1.75 px = 1.75 logical pt. These are egui points, not 1/72-inch print points. No DPI metadata in the PNG establishes the original monitor scale. For a native client reproducing just the framed application, subtract **(28 px / 28 pt, 22 px / 22 pt)** from every screenshot position. Its selected frame size is **1620 × 882 px / 1620 × 882 pt**; do not include the surrounding desktop as client padding. Egui uses logical points and a pixels-per-point scale factor. [egui 0.37 coordinate conventions](https://github.com/emilk/egui/blob/0.37.0/CONTRIBUTING.md)

## Deliverables and evidence

- `annotations/01-window-frame.png` through `09-status-bar.png`: source-based annotated PNGs with global bounds and matching logical-point values.
- `annotations/10-colors-typography-spacing.png`: sampled palette, typography evidence, and measured spacing summary.
- `annotations/00-overview.png`: all primary area boundaries over the original.
- `forge-room-layered.svg`: a fully vector reconstruction with named layers, rectangles, real text nodes, and traced icon paths. It contains no embedded PNG or font. It is an inspectable redraw, **not a pixel-identical or recovered original**. Text is scaled to measured ink envelopes; font choices and corner fits are identified as reconstruction settings.
- `redraw.png`: renderer output of the layered SVG at 1672 × 941 px.
- `measurements/measurements.json`: every rectangle, text ROI, ink envelope, foreground sample coordinate, icon path, border ridge profile, and radius fit.
- `measurements/all-colors.csv`: **every distinct RGB color** with its pixel count and the area(s) where it occurs. Most are antialiasing, texture, or blurred-background samples, not separate design tokens.
- `measurements/typography-fit.json`: local font-comparison evidence, including mismatches. A good match is not proof of original family or nominal size.
- `measurements/warm-fields.json`: actual exterior red-excess regions, with threshold criterion and peak samples; no invented glow radius or alpha.
- `tokens.json`, `layout.rs`: build-oriented values and coordinate conversion helpers, with provenance labels; `validation.json`: artifact integrity checks.

Use the detailed tables for measured envelopes. The content-offset column records visible ink/control offsets, not proven framework padding. Every scalar dimension in an observation or reconstruction uses the same numerical value in px and logical pt at the requested scale.
"""

AREA_NOTES = {
1: r"""The visible frame sits inside desktop margins: left 28 px / 28 pt, top 22 px / 22 pt, right 24 px / 24 pt, bottom 37 px / 37 pt. A bright border ridge is surrounded by dark/sharpened fringes; its original stroke width is U. The window corner contour is fitted from the raster at 0.5 px / 0.5 pt search increments; the fitted radius and original-radius limitation appear below.

The exterior is an already blurred, nonuniform warm gray/black field with several red-tinted regions. The screenshot does not identify its source image, native acrylic/mica material, Gaussian sigma, blur radius, or opacity. None can be measured exactly from this file. `warm-fields.json` reports visible red-excess envelopes and peak colors; those are not the center/radius of an assumed circular glow. The SVG reconstructs only the visible exterior field with RGB-sampled gradient strips. That representation supplies neither the hidden image nor a claim about the original blur shader. Internal active-tab red fringes and button gradients are likewise flattened observations.
""",
2: r"""The title band occupies the top 54 px / 54 pt inside the border. The large brand mark is a red ring with a white plus. The arrows, palette search mark, room people mark, run, chat, bell, and window controls are isolated in the icon table. The bell has a separate red notification dot; the room pill contains a small green presence mark. Their ink envelopes, not inferred hit areas, are recorded.

The command-palette container and room pill are independently measured. Their text is a single line; the command shortcut is not a separate boxed keycap. The forward arrow is visibly dimmer. Separator ridges before the run controls and window controls are independent lines. The native title-bar drag region, platform system-menu behavior, and button hover states are U in this still image.
""",
3: r"""The rail is 76 px / 76 pt wide between its selected boundaries. All eight icons appear in this order: explorer, search, source control, run/debug, extensions, AI, terminal, room. The icon table gives their actual painted sizes and centers; they do not all have a literal 24 px / 24 pt painted envelope. The visible center-to-center deltas are listed in the spacing section.

The selected Room tile is 56 × 46 px / 56 × 46 pt, red-tinted with a red border strongest toward the left. It is not a square in this screenshot. Its foreground geometry is two people. **There is no separate count badge or notification bubble on the active rail item.** Do not mistake the second person's head/shoulder for a badge. The presence mark in the title room pill and the bell's notification dot are separate observations.
""",
4: r"""The Explorer header is 49 px / 49 pt high, including its lower divider band. The tree's measured row-center positions and explicit icon/text anchors are below. The row-center differences vary from 23–26 px / 23–26 pt; there is no evidence for one exact hidden row-height token. The principal text anchors for apps → desktop → src → features → room are x=176,194,211,232,251 px / pt, giving 18,17,21,19 px / pt offsets. Do not label them all “20 px” as a measured fact.

The selected Room envelope is 191 × 26 px / 191 × 26 pt; its gray background is sampled directly. Folder and chevron glyphs are small and measured separately from rail icons. `package.json` has a brace marker; `tsconfig.json` has a drawn `ts` mark in the reference, despite the JSON filename. `README.md` uses a small outlined hexagon/chevron-like marker, not a visibly identifiable standard circled information icon. The redraw preserves that observed shape. Vertical ancestry guides are faint; the original indent algorithm and collapse hit areas are U.
""",
5: r"""Source Control begins at y=531 px / 531 pt and shares the sidebar's x bounds. It has a rounded top envelope, a 44 px / 44 pt header selection, and the `Changes (3)` label. Three file rows appear, with M/M/A badges and the literal paths transcribed in the text table.

The Commit envelope is 240 × 33 px / pt. Its dropdown segment selection begins at x=326 px / pt; the main selection before it is 199 px / pt wide. Sync is 240 × 35 px / pt. Open Pull Request is 240 × 36 px / pt. Their observed top positions are 715,757,801 px / pt, giving 9 and 9 px / pt clear vertical gaps between the respective envelopes. The red Commit fill varies across the button; the listed sample is a composite, not proof of one original flat fill. No pressed, disabled, loading, or validation state is visible.
""",
6: r"""The editor's upper band is 49 px / 49 pt high. The active Room.tsx tab has a 3 px / 3 pt bright underline core, with a wider red fringe. Other visible tabs are AgentSession.ts and room.schema.json; plus is a separate path icon. Breadcrumbs start at x≈408 px / pt and are individually measured rather than laid out from invented equal gaps.

The selected line-number gutter envelope is 67 px / 67 pt wide, ending at x=455 px / pt. First-level code begins around x=474 px / pt. Code row centers are recorded at y=177,198,218,…,598 px / pt; the first delta is 21 px / pt and the subsequent selected centers advance 20 px / pt. The screenshot literally displays `19` after `9`; its line numbers are not a consistent source-file numbering sequence. The redraw retains that artifact. The visible lower code includes MemberList, TaskList, two closing div tags, `)` and `}`.

The active line envelope is y=428..448 px / pt with a red line number. A thin amber caret is visible around x=1041 px / pt; blink timing is U. Syntax colors are sampled from the colored glyphs. Font family/nominal size are U; the comparison file and typography table disclose the local Consolas fit and reconstruction value.

The floating action toolbar is an overlay at (740,363), not a row in the code flow. Its four labeled buttons and More button are separately measured. The agent popup at (894,451) is 307 × 145 px / pt with a pointer toward the active line, five rows, and a red-tinted selected builder row. The final custom row has `Create custom agent...` right-aligned. Original shadow/blur/opacity values, popup-open animation, and keyboard selection behavior are U.
""",
7: r"""The bottom panel begins beneath the y=625 px / pt divider ridge. Its selected tab band is 44 px / pt high; Terminal has a 3 px / pt red underline core. Output, Problems, the 0 pill, Debug Console, Ports, Run with dropdown, split, trash, and More are independently represented.

Terminal text is positioned from measured ink bounds. The prompt has cyan/blue/red/yellow spans, the ready check is green, Local/Network arrows are blue, and localhost is blue with an underline in the screenshot. The Current Branch card at (977,681) measures 230 × 155 px / pt. It includes heading and status dividers, branch/check marks, Ahead 2 / Behind 0 text, and a 208 × 35 px / pt Create Pull Request button. The terminal/card divider is at x=966 px / pt. Terminal emulation, command execution, scrollback, links, resizing, and tab interactions are not established by this still image.
""",
8: r"""The right dock is 428 px / pt wide. AI Assistant is the active tab; Room is inactive; More sits at the far right. The brand header has a separate ring/plus logo, title, and subtitle. The quick-actions card is 402 × 150 px / pt, containing a two-column layout: Implement a feature, Explain this code, Write tests, Find and fix issues. The selected button envelopes give an 11 px / pt horizontal gap and 6 px / pt vertical gap.

The user message has a 352 × 63 px / pt bubble and a separate avatar; its two visible line starts are measured individually. The assistant response has no filled speech bubble and uses a separate small ring/plus logo. The Edited summary is 352 × 82 px / pt; both rows include a green check, Edited label, a filename chip, a green addition count, and a red deletion count. Room.tsx shows +42 and −6; AgentPresence.tsx shows +98 and −2.

The composer is 400 × 124 px / pt. The placeholder wraps onto two explicit lines. Its bottom row contains plus, @, cube, Forge AI Pro with a chevron, and a 42 × 36 px / pt red send button. The button's fill and nearby glow are spatially varying. Placeholder alpha, focus ring, text entry behavior, send-disabled state, scroll behavior, and active Room-tab content are U.
""",
9: r"""The selected status content band is 41 px / pt high, between its upper divider and lower frame border. Left to right: branch icon + feature/room-collab + dropdown; separator; sync icon + `2 ↑ 0 ↓`; separator; branch/PR mark + PR #142; Review needed pill; separator; red lightning + Room: Product Launch; green presence dot + `4 members · 2 agents`; flexible blank space; Ln 14, Col 52; UTF-8; CRLF; home; separator; cube + Forge AI Pro.

The item table contains the actual ink and separator positions, avoiding assumptions about equal widths. The green presence dot is not a text bullet. The visible data describe this one frame; no behavior, menus, live counts, navigation action, or responsive collapse order can be measured from it.
""",
}

BUILD_NOTES = r"""## Native Rust/egui construction notes

1. Use screenshot coordinates for verification, and `(x − 28, y − 22)` for a client covering the chosen frame. Keep the full-window outline and desktop treatment in separate layers. The supplied `layout.rs` contains the nine primary rectangles and a translation helper; it is a handoff artifact, not an app integration.
2. In a reference-sized capture, allocate the explicit column and band dimensions rather than inheriting egui widget defaults. Clip each area to its rectangle. Keep editor overlays on a foreground layer and preserve their independent global anchors.
3. The repository already loads Windows Segoe UI/semibold and Consolas in `native/forge-room-app/src/fonts.rs`. Those are practical reconstruction fonts. That code is not evidence that the reference used the same original fonts. At non-100% display scale, divide physical coordinates by the effective pixels-per-point value; do not convert using print-point formulas.
4. Match the observed colors using opaque sampled colors for a deterministic first reproduction. If adding actual OS background translucency later, treat its alpha/blur as new implementation decisions requiring visual tuning; the screenshot does not supply recoverable exact values.
5. Draw the traced icons as vector paths, or use the earlier authored icon set as a stylistic approximation. They are different sources: this redraw's icon paths come from screenshot masks; the authored 24-viewBox icon pack is not proof of the original icon geometry or stroke width.
6. Source text styles and spacing are not perfectly consistent. The SVG uses explicit ink-envelope transforms to reproduce observed placement; a production native layout may choose a consistent grid, but that would be an intentional deviation from this measured handoff.

### States and behavior not visible

The screenshot supplies one selected rail item, one tree selection, one active editor tab, one active bottom tab, one active dock tab, one active code line, and one selected agent suggestion. Hover/pressed/disabled/focus states, text overflow behavior, minimum window size, alternate DPI captures, responsive breakpoints, accessibility focus order, keyboard bindings, drag targets, scroll state, loading/error states, animation duration, and reduced-motion behavior are U. No additional state values are fabricated in this reference-measurement spec.

### Acceptance check

Capture the native client at 1620 × 882 physical px with effective pixels_per_point=1, then place it at screenshot offset (28,22) for a source overlay comparison. Verify the primary dividers first, followed by container envelopes, text ink boxes, and icons. Compare solid regions separately from font antialiasing and the nonuniform desktop field. A successful geometry check does not prove recovery of the original font, blur, opacity, or design tokens.
"""
