# Forge Room IDE — screenshot measurement and native UI handoff

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


## Primary measured envelopes

| Area | Screenshot bounds, px (M) | Logical bounds at 100%, pt | Window-local logical bounds, pt |
|---|---|---|---|
| Window frame | (28, 22, 1620, 882) | (28, 22, 1620, 882) | (0, 0, 1620, 882) |
| Title bar | (29, 23, 1618, 54) | (29, 23, 1618, 54) | (1, 1, 1618, 54) |
| Left rail | (29, 77, 76, 784) | (29, 77, 76, 784) | (1, 55, 76, 784) |
| Explorer | (106, 77, 281, 455) | (106, 77, 281, 455) | (78, 55, 281, 455) |
| Source Control | (106, 531, 281, 330) | (106, 531, 281, 330) | (78, 509, 281, 330) |
| Editor | (387, 77, 832, 549) | (387, 77, 832, 549) | (359, 55, 832, 549) |
| Bottom panel | (387, 626, 832, 235) | (387, 626, 832, 235) | (359, 604, 832, 235) |
| Right dock | (1219, 77, 428, 784) | (1219, 77, 428, 784) | (1191, 55, 428, 784) |
| Status bar | (29, 862, 1618, 41) | (29, 862, 1618, 41) | (1, 840, 1618, 41) |

Shared divider evidence: the source has bright/antialiased bands near x=104–105,385–386,1215–1218 px / pt and y=75–76,124–125,624–626,668–669,861–862 px / pt. Keep the sampled ridge widths separate from an assumed original one-pixel stroke.


## 1. Window frame

[Annotated PNG](annotations/01-window-frame.png)

The visible frame sits inside desktop margins: left 28 px / 28 pt, top 22 px / 22 pt, right 24 px / 24 pt, bottom 37 px / 37 pt. A bright border ridge is surrounded by dark/sharpened fringes; its original stroke width is U. The window corner contour is fitted from the raster at 0.5 px / 0.5 pt search increments; the fitted radius and original-radius limitation appear below.

The exterior is an already blurred, nonuniform warm gray/black field with several red-tinted regions. The screenshot does not identify its source image, native acrylic/mica material, Gaussian sigma, blur radius, or opacity. None can be measured exactly from this file. `warm-fields.json` reports visible red-excess envelopes and peak colors; those are not the center/radius of an assumed circular glow. The SVG reconstructs only the visible exterior field with RGB-sampled gradient strips. That representation supplies neither the hidden image nor a claim about the original blur shader. Internal active-tab red fringes and button gradients are likewise flattened observations.


### Container, border, and content-offset measurements

Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.

| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |
|---|---|---|---|---|---|---|
| `window` | (28, 22, 1620, 882) | (28, 22, 1620, 882) | #0E0E10 | #282829; 1/2/2/1 px = 1/2/2/1 pt | 11 px / 11 pt | left 0; top title band 55; bottom status band 42. Visible frame envelope; AA contour extends into adjacent pixels. Screenshot includes desktop outside this rectangle. |

## 2. Title bar

[Annotated PNG](annotations/02-title-bar.png)

The title band occupies the top 54 px / 54 pt inside the border. The large brand mark is a red ring with a white plus. The arrows, palette search mark, room people mark, run, chat, bell, and window controls are isolated in the icon table. The bell has a separate red notification dot; the room pill contains a small green presence mark. Their ink envelopes, not inferred hit areas, are recorded.

The command-palette container and room pill are independently measured. Their text is a single line; the command shortcut is not a separate boxed keycap. The forward arrow is visibly dimmer. Separator ridges before the run controls and window controls are independent lines. The native title-bar drag region, platform system-menu behavior, and button hover states are U in this still image.


### Container, border, and content-offset measurements

Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.

| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |
|---|---|---|---|---|---|---|
| `title-band` | (29, 23, 1618, 54) | (29, 23, 1618, 54) | #0D0F10 | no distinct outline selected; U source width | 0 px / 0 pt | brand x+19; controls centered near y=49. Bottom divider ridge at y=76. |
| `command-palette` | (496, 31, 503, 36) | (496, 31, 503, 36) | #181A1B | #303132; 2/2/1/2 px = 2/2/1/2 pt | 9 px / 9 pt; boundary-limited | icon left 11; label left 37; y 10. Visible key hint uses Ctrl+Shift+P, not a separate keycap. |
| `room-pill` | (1085, 31, 212, 36) | (1085, 31, 212, 36) | #19191A | #2C2C2D; 2/1/2/1 px = 2/1/2/1 pt | 9 px / 9 pt; boundary-limited | icon left 13; label left 46.  |
| `title-room-run-separator` | (1322, 37, 1, 24) | (1322, 37, 1, 24) | #101011 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |
| `title-controls-separator` | (1484, 36, 1, 27) | (1484, 36, 1, 27) | #19191A | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |

### Text instances

Use the style key in the typography table for the disclosed reconstruction family/size/weight. These bounds are thresholded **ink**, not text-layout allocation. Every foreground hex is an actual source sample; colors can differ slightly between instances. Mask thresholds and sample coordinates are in measurements.json.

| Text / instance | Style | Ink bounds px (M) | Ink bounds logical pt | Foreground composite (M) |
|---|---|---|---|---|
| Forge Room · `brand-label` | `brand` | (90, 42, 104, 19) | (90, 42, 104, 19) | #FFFFFF |
| Command Palette  (Ctrl+Shift+P) · `palette-label` | `ui` | (533, 43, 209, 13) | (533, 43, 209, 13) | #E2E1E3 |
| Room: Product Launch · `room-label` | `ui` | (1132, 43, 144, 11) | (1132, 43, 144, 11) | #F3F4F5 |

### Icon instances

These are painted-mask sizes. Original icon viewBoxes, exact source stroke widths, and hit targets are U. SVG paths are traced from the selected masks at subpixel contour placement; the original vectors are not recovered.

| Icon | Painted bounds px (M) | Painted bounds logical pt | Painted size px / pt | Foreground composite color(s) | Notes |
|---|---|---|---|---|---|
| `brand-logo` | (47, 35, 28, 28) | (47, 35, 28, 28) | 28×28 px / 28×28 pt | #FAFCF8, #FF433D |  |
| `back` | (375, 42, 17, 14) | (375, 42, 17, 14) | 17×14 px / 17×14 pt | #F1F3F1 |  |
| `forward` | (424, 42, 7, 14) | (424, 42, 7, 14) | 7×14 px / 7×14 pt | #868787 | Visibly dimmer than Back. |
| `palette-search` | (507, 40, 17, 16) | (507, 40, 17, 16) | 17×16 px / 17×16 pt | #DDDDDD |  |
| `title-room-people` | (1099, 40, 22, 17) | (1099, 40, 22, 17) | 22×17 px / 22×17 pt | #E9EAEB, #B9EEB9 | Small green presence mark at lower right. |
| `title-run` | (1349, 40, 14, 17) | (1349, 40, 14, 17) | 14×17 px / 14×17 pt | #E4E6E4 |  |
| `title-chat` | (1395, 40, 18, 18) | (1395, 40, 18, 18) | 18×18 px / 18×18 pt | #C9C8CA |  |
| `title-bell` | (1446, 40, 20, 18) | (1446, 40, 20, 18) | 20×18 px / 20×18 pt | #DDDDDF |  |
| `notification-dot` | (1457, 35, 11, 10) | (1457, 35, 11, 10) | 11×10 px / 11×10 pt | #F24A3F |  |
| `window-minimize` | (1515, 48, 14, 1) | (1515, 48, 14, 1) | 14×1 px / 14×1 pt | #E5E3E2 |  |
| `window-maximize` | (1562, 41, 16, 15) | (1562, 41, 16, 15) | 16×15 px / 16×15 pt | #E7E7E6 |  |
| `window-close` | (1612, 42, 14, 13) | (1612, 42, 14, 13) | 14×13 px / 14×13 pt | #F0F0EE |  |

## 3. Left rail

[Annotated PNG](annotations/03-left-rail.png)

The rail is 76 px / 76 pt wide between its selected boundaries. All eight icons appear in this order: explorer, search, source control, run/debug, extensions, AI, terminal, room. The icon table gives their actual painted sizes and centers; they do not all have a literal 24 px / 24 pt painted envelope. The visible center-to-center deltas are listed in the spacing section.

The selected Room tile is 56 × 46 px / 56 × 46 pt, red-tinted with a red border strongest toward the left. It is not a square in this screenshot. Its foreground geometry is two people. **There is no separate count badge or notification bubble on the active rail item.** Do not mistake the second person's head/shoulder for a badge. The presence mark in the title room pill and the bell's notification dot are separate observations.


### Container, border, and content-offset measurements

Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.

| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |
|---|---|---|---|---|---|---|
| `rail-band` | (29, 77, 76, 784) | (29, 77, 76, 784) | #0B0D0E | no distinct outline selected; U source width | 0 px / 0 pt | icons centered approximately x=66; first center y=113.  |
| `room-active-tile` | (39, 439, 56, 46) | (39, 439, 56, 46) | #281717 | #56201D; 2/2/1/2 px = 2/2/1/2 pt | 7.5 px / 7.5 pt | room ink approximately x+15,y+14. No separate count badge is visible in the rail. |

### Icon instances

These are painted-mask sizes. Original icon viewBoxes, exact source stroke widths, and hit targets are U. SVG paths are traced from the selected masks at subpixel contour placement; the original vectors are not recovered.

| Icon | Painted bounds px (M) | Painted bounds logical pt | Painted size px / pt | Foreground composite color(s) | Notes |
|---|---|---|---|---|---|
| `rail-explorer` | (54, 103, 23, 19) | (54, 103, 23, 19) | 23×19 px / 23×19 pt | #D4D3D3 |  |
| `rail-search` | (55, 150, 21, 21) | (55, 150, 21, 21) | 21×21 px / 21×21 pt | #F1F0F1 |  |
| `rail-source-control` | (57, 200, 18, 22) | (57, 200, 18, 22) | 18×22 px / 18×22 pt | #F6F6F2 |  |
| `rail-run-debug` | (58, 251, 17, 20) | (58, 251, 17, 20) | 17×20 px / 17×20 pt | #DDE0DF |  |
| `rail-extensions` | (56, 300, 20, 22) | (56, 300, 20, 22) | 20×22 px / 20×22 pt | #E2E2E2 |  |
| `rail-ai` | (55, 350, 21, 21) | (55, 350, 21, 21) | 21×21 px / 21×21 pt | #E9EBE8 |  |
| `rail-terminal` | (55, 399, 22, 19) | (55, 399, 22, 19) | 22×19 px / 22×19 pt | #E6E7E7 |  |
| `rail-room` | (54, 452, 26, 19) | (54, 452, 26, 19) | 26×19 px / 26×19 pt | #F6E7E3 |  |

## 4. Explorer

[Annotated PNG](annotations/04-explorer.png)

The Explorer header is 49 px / 49 pt high, including its lower divider band. The tree's measured row-center positions and explicit icon/text anchors are below. The row-center differences vary from 23–26 px / 23–26 pt; there is no evidence for one exact hidden row-height token. The principal text anchors for apps → desktop → src → features → room are x=176,194,211,232,251 px / pt, giving 18,17,21,19 px / pt offsets. Do not label them all “20 px” as a measured fact.

The selected Room envelope is 191 × 26 px / 191 × 26 pt; its gray background is sampled directly. Folder and chevron glyphs are small and measured separately from rail icons. `package.json` has a brace marker; `tsconfig.json` has a drawn `ts` mark in the reference, despite the JSON filename. `README.md` uses a small outlined hexagon/chevron-like marker, not a visibly identifiable standard circled information icon. The redraw preserves that observed shape. Vertical ancestry guides are faint; the original indent algorithm and collapse hit areas are U.


### Container, border, and content-offset measurements

Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.

| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |
|---|---|---|---|---|---|---|
| `explorer-header` | (106, 77, 281, 49) | (106, 77, 281, 49) | #0C0C0E | no distinct outline selected; U source width | 0 px / 0 pt | label x+19; action icons at x=296 and x=342.  |
| `selected-tree-row` | (183, 253, 191, 26) | (183, 253, 191, 26) | #212324 | no distinct outline selected; U source width | 7 px / 7 pt; boundary-limited | chevron x+25; folder x+43; label x+68. Only Room is selected; no visible row outline. |
| `explorer-actions-divider` | (327, 91, 1, 22) | (327, 91, 1, 22) | #171819 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |
| `explorer-header-divider` | (106, 124, 281, 2) | (106, 124, 281, 2) | #0B0C0E | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |

### Text instances

Use the style key in the typography table for the disclosed reconstruction family/size/weight. These bounds are thresholded **ink**, not text-layout allocation. Every foreground hex is an actual source sample; colors can differ slightly between instances. Mask thresholds and sample coordinates are in measurements.json.

| Text / instance | Style | Ink bounds px (M) | Ink bounds logical pt | Foreground composite (M) |
|---|---|---|---|---|
| Explorer · `explorer-heading` | `panel-heading` | (127, 95, 60, 17) | (127, 95, 60, 17) | #FFFFFF |
| forge-room-app · `tree-forge-room-app` | `tree` | (144, 136, 109, 15) | (144, 136, 109, 15) | #FEFEFF |
| apps · `tree-apps` | `tree` | (177, 165, 30, 11) | (177, 165, 30, 11) | #EDF0EF |
| desktop · `tree-desktop` | `tree` | (195, 187, 50, 14) | (195, 187, 50, 14) | #ECEEF0 |
| src · `tree-src` | `tree` | (213, 215, 16, 7) | (213, 215, 16, 7) | #EFF0F0 |
| features · `tree-features` | `tree` | (233, 235, 50, 12) | (233, 235, 50, 12) | #EFEDEE |
| room · `tree-room` | `tree` | (252, 263, 32, 9) | (252, 263, 32, 9) | #EBEDEF |
| editor · `tree-editor` | `tree` | (251, 285, 37, 10) | (251, 285, 37, 10) | #E7E5E2 |
| ai · `tree-ai` | `tree` | (251, 309, 10, 10) | (251, 309, 10, 10) | #EEEDED |
| scm · `tree-scm` | `tree` | (251, 336, 24, 8) | (251, 336, 24, 8) | #E4E8E8 |
| components · `tree-components` | `tree` | (233, 358, 79, 13) | (233, 358, 79, 13) | #E5E5E4 |
| services · `tree-services` | `tree` | (237, 381, 51, 11) | (237, 381, 51, 11) | #EDF0EF |
| main.ts · `tree-main-ts` | `tree` | (238, 405, 44, 11) | (238, 405, 44, 11) | #E4E7E6 |
| assets · `tree-assets` | `tree` | (217, 431, 36, 9) | (217, 431, 36, 9) | #EAE7E7 |
| package.json · `tree-package-json` | `tree` | (197, 453, 78, 14) | (197, 453, 78, 14) | #E6E4E4 |
| tsconfig.json · `tree-tsconfig-json` | `tree` | (196, 476, 79, 14) | (196, 476, 79, 14) | #E7E7E6 |
| README.md · `tree-README-md` | `tree` | (197, 500, 78, 11) | (197, 500, 78, 11) | #EDECEB |

### Icon instances

These are painted-mask sizes. Original icon viewBoxes, exact source stroke widths, and hit targets are U. SVG paths are traced from the selected masks at subpixel contour placement; the original vectors are not recovered.

| Icon | Painted bounds px (M) | Painted bounds logical pt | Painted size px / pt | Foreground composite color(s) | Notes |
|---|---|---|---|---|---|
| `new-file` | (296, 93, 17, 17) | (296, 93, 17, 17) | 17×17 px / 17×17 pt | #CACECF |  |
| `explorer-refresh` | (342, 94, 16, 16) | (342, 94, 16, 16) | 16×16 px / 16×16 pt | #D5D7D8 |  |
| `tree-chevron-forge-room-app` | (124, 140, 9, 5) | (124, 140, 9, 5) | 9×5 px / 9×5 pt | #FFFFFF | down |
| `tree-icon-apps` | (152, 162, 15, 13) | (152, 162, 15, 13) | 15×13 px / 15×13 pt | #E8E9E8 | folder marker; source outline traced. |
| `tree-chevron-apps` | (133, 166, 9, 5) | (133, 166, 9, 5) | 9×5 px / 9×5 pt | #F4F4F4 | down |
| `tree-icon-desktop` | (169, 187, 15, 12) | (169, 187, 15, 12) | 15×12 px / 15×12 pt | #E3E4E0 | folder marker; source outline traced. |
| `tree-chevron-desktop` | (151, 190, 9, 5) | (151, 190, 9, 5) | 9×5 px / 9×5 pt | #DAD8D9 | down |
| `tree-icon-src` | (187, 212, 15, 12) | (187, 212, 15, 12) | 15×12 px / 15×12 pt | #EDECEC | folder marker; source outline traced. |
| `tree-chevron-src` | (169, 215, 9, 5) | (169, 215, 9, 5) | 9×5 px / 9×5 pt | #FCFEFD | down |
| `tree-icon-features` | (208, 235, 15, 13) | (208, 235, 15, 13) | 15×13 px / 15×13 pt | #D2D3D0 | folder marker; source outline traced. |
| `tree-chevron-features` | (189, 239, 10, 5) | (189, 239, 10, 5) | 10×5 px / 10×5 pt | #ECEFEF | down |
| `tree-icon-room` | (226, 260, 15, 13) | (226, 260, 15, 13) | 15×13 px / 15×13 pt | #DCDCDC | folder marker; source outline traced. |
| `tree-chevron-room` | (209, 262, 6, 9) | (209, 262, 6, 9) | 6×9 px / 6×9 pt | #E0E1E1 | right |
| `tree-icon-editor` | (226, 284, 15, 13) | (226, 284, 15, 13) | 15×13 px / 15×13 pt | #D3D3D2 | folder marker; source outline traced. |
| `tree-chevron-editor` | (209, 286, 5, 9) | (209, 286, 5, 9) | 5×9 px / 5×9 pt | #E8EBE9 | right |
| `tree-icon-ai` | (226, 308, 15, 13) | (226, 308, 15, 13) | 15×13 px / 15×13 pt | #D5D6D4 | folder marker; source outline traced. |
| `tree-chevron-ai` | (209, 310, 5, 9) | (209, 310, 5, 9) | 5×9 px / 5×9 pt | #EEEEEE | right |
| `tree-icon-scm` | (226, 332, 15, 13) | (226, 332, 15, 13) | 15×13 px / 15×13 pt | #DCDCDB | folder marker; source outline traced. |
| `tree-chevron-scm` | (209, 334, 5, 9) | (209, 334, 5, 9) | 5×9 px / 5×9 pt | #F5F6F5 | right |
| `tree-icon-components` | (208, 357, 15, 12) | (208, 357, 15, 12) | 15×12 px / 15×12 pt | #E9EAE9 | folder marker; source outline traced. |
| `tree-chevron-components` | (190, 360, 10, 5) | (190, 360, 10, 5) | 10×5 px / 10×5 pt | #FAFAFA | down |
| `tree-icon-services` | (212, 380, 14, 14) | (212, 380, 14, 14) | 14×14 px / 14×14 pt | #E1E1E2 | file marker; source outline traced. |
| `tree-icon-main-ts` | (212, 404, 14, 14) | (212, 404, 14, 14) | 14×14 px / 14×14 pt | #CED1D2 | file marker; source outline traced. |
| `tree-icon-assets` | (192, 429, 15, 12) | (192, 429, 15, 12) | 15×12 px / 15×12 pt | #DADADD | folder marker; source outline traced. |
| `tree-chevron-assets` | (176, 431, 4, 8) | (176, 431, 4, 8) | 4×8 px / 4×8 pt | #F9FAF8 | right |
| `tree-icon-package-json` | (172, 452, 12, 13) | (172, 452, 12, 13) | 12×13 px / 12×13 pt | #F9FAF8 | json marker; source outline traced. |
| `tree-icon-tsconfig-json` | (172, 476, 13, 11) | (172, 476, 13, 11) | 13×11 px / 13×11 pt | #DDDCDD | ts marker; source outline traced. |
| `tree-icon-README-md` | (172, 499, 13, 13) | (172, 499, 13, 13) | 13×13 px / 13×13 pt | #E5E5E6 | readme marker; source outline traced. |

### Tree anchors and row-center cadence

| Item | Depth label | Selected center y px | Center y logical pt | Label anchor x px | Label anchor x logical pt | Marker |
|---|---|---|---|---|---|---|
| forge-room-app | 0 | 142 | 142 | 144 | 144 | none; down |
| apps | 1 | 168 | 168 | 176 | 176 | folder; down |
| desktop | 2 | 193 | 193 | 194 | 194 | folder; down |
| src | 3 | 218 | 218 | 211 | 211 | folder; down |
| features | 4 | 242 | 242 | 232 | 232 | folder; down |
| room | 5 | 266 | 266 | 251 | 251 | folder; right |
| editor | 5 | 291 | 291 | 251 | 251 | folder; right |
| ai | 5 | 315 | 315 | 251 | 251 | folder; right |
| scm | 5 | 339 | 339 | 251 | 251 | folder; right |
| components | 4 | 363 | 363 | 232 | 232 | folder; down |
| services | 4 | 388 | 388 | 236 | 236 | file; no chevron |
| main.ts | 4 | 412 | 412 | 236 | 236 | file; no chevron |
| assets | 2 | 436 | 436 | 217 | 217 | folder; right |
| package.json | 1 | 459 | 459 | 196 | 196 | json; no chevron |
| tsconfig.json | 1 | 483 | 483 | 196 | 196 | ts; no chevron |
| README.md | 1 | 507 | 507 | 196 | 196 | readme; no chevron |

## 5. Source Control

[Annotated PNG](annotations/05-source-control.png)

Source Control begins at y=531 px / 531 pt and shares the sidebar's x bounds. It has a rounded top envelope, a 44 px / 44 pt header selection, and the `Changes (3)` label. Three file rows appear, with M/M/A badges and the literal paths transcribed in the text table.

The Commit envelope is 240 × 33 px / pt. Its dropdown segment selection begins at x=326 px / pt; the main selection before it is 199 px / pt wide. Sync is 240 × 35 px / pt. Open Pull Request is 240 × 36 px / pt. Their observed top positions are 715,757,801 px / pt, giving 9 and 9 px / pt clear vertical gaps between the respective envelopes. The red Commit fill varies across the button; the listed sample is a composite, not proof of one original flat fill. No pressed, disabled, loading, or validation state is visible.


### Container, border, and content-offset measurements

Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.

| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |
|---|---|---|---|---|---|---|
| `scm-panel` | (106, 531, 281, 330) | (106, 531, 281, 330) | #0D0D0E | #242526; 2/1/2/1 px = 2/1/2/1 pt | 9 px / 9 pt | content x+21; right 20.  |
| `scm-header` | (106, 531, 281, 44) | (106, 531, 281, 44) | no separate fill selected | no distinct outline selected; U source width | 0 px / 0 pt | heading x+22,y+17; divider y=574.  |
| `badge-room` | (129, 617, 31, 23) | (129, 617, 31, 23) | #15181C | no distinct outline selected; U source width | 6 px / 6 pt; boundary-limited | glyph centered. M = modified |
| `badge-types` | (129, 647, 31, 23) | (129, 647, 31, 23) | #15191D | no distinct outline selected; U source width | 6 px / 6 pt; boundary-limited | glyph centered. M = modified |
| `badge-agents` | (129, 679, 31, 23) | (129, 679, 31, 23) | #261515 | no distinct outline selected; U source width | 6 px / 6 pt; boundary-limited | glyph centered. A = added |
| `commit-button` | (127, 715, 240, 33) | (127, 715, 240, 33) | #E23635 | #731D1B; 3/2/2/2 px = 3/2/2/2 pt | 9 px / 9 pt; boundary-limited | label centered; dropdown starts x=326. Main segment 199 px; dropdown segment 41 px. |
| `commit-dropdown-segment` | (326, 716, 40, 31) | (326, 716, 40, 31) | no separate fill selected | no distinct outline selected; U source width | 0 px / 0 pt | chevron centered near (347,732).  |
| `sync-button` | (127, 757, 240, 35) | (127, 757, 240, 35) | #1A1A1C | #2F2F30; 2/2/2/2 px = 2/2/2/2 pt | 9 px / 9 pt; boundary-limited | icon x+91; label x+119.  |
| `open-pr-button` | (127, 801, 240, 36) | (127, 801, 240, 36) | #191A1C | #333435; 2/2/1/2 px = 2/2/1/2 pt | 9 px / 9 pt; boundary-limited | text x+45; external-link right 10.  |
| `scm-header-divider` | (106, 573, 281, 2) | (106, 573, 281, 2) | #0C0D0E | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |
| `commit-segment-divider` | (326, 716, 1, 31) | (326, 716, 1, 31) | #E6423E | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |

### Text instances

Use the style key in the typography table for the disclosed reconstruction family/size/weight. These bounds are thresholded **ink**, not text-layout allocation. Every foreground hex is an actual source sample; colors can differ slightly between instances. Mask thresholds and sample coordinates are in measurements.json.

| Text / instance | Style | Ink bounds px (M) | Ink bounds logical pt | Foreground composite (M) |
|---|---|---|---|---|
| Source Control · `scm-heading` | `section-heading` | (129, 548, 111, 12) | (129, 548, 111, 12) | #FFFFFF |
| Changes (3) · `changes-count` | `tab` | (129, 588, 85, 16) | (129, 588, 85, 16) | #F7F8F8 |
| M · `badge-letter-room` | `badge-blue` | (139, 623, 10, 10) | (139, 623, 10, 10) | #97D1F1 |
| src/features/room/Room.tsx · `change-path-room` | `small-ui` | (172, 621, 173, 13) | (172, 621, 173, 13) | #D0D4D5 |
| M · `badge-letter-types` | `badge-blue` | (139, 653, 10, 10) | (139, 653, 10, 10) | #9DD5F7 |
| src/features/room/types.ts · `change-path-types` | `small-ui` | (172, 652, 163, 14) | (172, 652, 163, 14) | #D2D2D3 |
| A · `badge-letter-agents` | `badge-red` | (139, 684, 10, 11) | (139, 684, 10, 11) | #F95946 |
| src/lib/agents.ts · `change-path-agents` | `small-ui` | (172, 683, 99, 15) | (172, 683, 99, 15) | #D3D6D3 |
| Commit... · `commit-label` | `ui` | (207, 726, 61, 11) | (207, 726, 61, 11) | #FEF0EF |
| Sync · `sync-label` | `ui` | (247, 769, 28, 14) | (247, 769, 28, 14) | #F6F6F6 |
| Open Pull Request... · `open-pr-label` | `ui` | (173, 813, 128, 14) | (173, 813, 128, 14) | #F6F8F7 |

### Icon instances

These are painted-mask sizes. Original icon viewBoxes, exact source stroke widths, and hit targets are U. SVG paths are traced from the selected masks at subpixel contour placement; the original vectors are not recovered.

| Icon | Painted bounds px (M) | Painted bounds logical pt | Painted size px / pt | Foreground composite color(s) | Notes |
|---|---|---|---|---|---|
| `scm-more` | (350, 552, 16, 3) | (350, 552, 16, 3) | 16×3 px / 16×3 pt | #FFFFFF |  |
| `commit-chevron` | (341, 729, 12, 7) | (341, 729, 12, 7) | 12×7 px / 12×7 pt | #FCE8E0 |  |
| `sync-icon` | (220, 766, 16, 16) | (220, 766, 16, 16) | 16×16 px / 16×16 pt | #EDEFEF |  |
| `open-pr-link` | (342, 812, 13, 13) | (342, 812, 13, 13) | 13×13 px / 13×13 pt | #E3E2E4 |  |

## 6. Editor

[Annotated PNG](annotations/06-editor.png)

The editor's upper band is 49 px / 49 pt high. The active Room.tsx tab has a 3 px / 3 pt bright underline core, with a wider red fringe. Other visible tabs are AgentSession.ts and room.schema.json; plus is a separate path icon. Breadcrumbs start at x≈408 px / pt and are individually measured rather than laid out from invented equal gaps.

The selected line-number gutter envelope is 67 px / 67 pt wide, ending at x=455 px / pt. First-level code begins around x=474 px / pt. Code row centers are recorded at y=177,198,218,…,598 px / pt; the first delta is 21 px / pt and the subsequent selected centers advance 20 px / pt. The screenshot literally displays `19` after `9`; its line numbers are not a consistent source-file numbering sequence. The redraw retains that artifact. The visible lower code includes MemberList, TaskList, two closing div tags, `)` and `}`.

The active line envelope is y=428..448 px / pt with a red line number. A thin amber caret is visible around x=1041 px / pt; blink timing is U. Syntax colors are sampled from the colored glyphs. Font family/nominal size are U; the comparison file and typography table disclose the local Consolas fit and reconstruction value.

The floating action toolbar is an overlay at (740,363), not a row in the code flow. Its four labeled buttons and More button are separately measured. The agent popup at (894,451) is 307 × 145 px / pt with a pointer toward the active line, five rows, and a red-tinted selected builder row. The final custom row has `Create custom agent...` right-aligned. Original shadow/blur/opacity values, popup-open animation, and keyboard selection behavior are U.


### Container, border, and content-offset measurements

Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.

| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |
|---|---|---|---|---|---|---|
| `editor-tab-strip` | (387, 77, 832, 49) | (387, 77, 832, 49) | #0C0D0E | no distinct outline selected; U source width | 0 px / 0 pt | active label x+45; plus near x=984.  |
| `room-tab` | (388, 80, 158, 46) | (388, 80, 158, 46) | #17181A | no distinct outline selected; U source width | 8 px / 8 pt | label x+44; close right 14. Visible active underline at y=122..125. |
| `room-tab-underline` | (391, 122, 151, 3) | (391, 122, 151, 3) | #F94944 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Bright core plus a wider red fringe; source blur parameters unrecoverable. |
| `breadcrumb-band` | (388, 127, 829, 31) | (388, 127, 829, 31) | no separate fill selected | no distinct outline selected; U source width | 0 px / 0 pt | first label x+20; baseline near y=146.  |
| `line-number-gutter` | (388, 159, 67, 453) | (388, 159, 67, 453) | no separate fill selected | no distinct outline selected; U source width | 0 px / 0 pt | line-number right edge near x=440; code starts x=474. First visible text ink starts around x=474; 19 px gap from gutter end. |
| `active-line` | (420, 428, 797, 20) | (420, 428, 797, 20) | #20171A | no distinct outline selected; U source width | 0 px / 0 pt | 0. Warm strip across row 14; line number red. Caret near x=1041. |
| `caret` | (1040, 430, 2, 18) | (1040, 430, 2, 18) | #433E33 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Observed amber vertical stroke; blink timing cannot be measured from a still image. |
| `floating-toolbar` | (740, 363, 444, 37) | (740, 363, 444, 37) | #1F2022 | #2D2E2F; 1/1/1/2 px = 1/1/1/2 pt | 9 px / 9 pt | left inset 6; top inset 4; button gaps 10,11,9,11. No editor reflow; overlay above the active line. |
| `toolbar-explain` | (746, 367, 91, 28) | (746, 367, 91, 28) | #1A1B1D | #27282A; 2/2/1/2 px = 2/2/1/2 pt | 7.5 px / 7.5 pt | icon left 10; label after 22 px.  |
| `toolbar-refactor` | (847, 367, 96, 28) | (847, 367, 96, 28) | #1A1A1C | #232426; 2/1/1/2 px = 2/1/1/2 pt | 5.5 px / 5.5 pt | icon left 10; label after 22 px.  |
| `toolbar-add-tests` | (954, 367, 104, 28) | (954, 367, 104, 28) | #191A1C | #252627; 2/1/1/2 px = 2/1/1/2 pt | 7.5 px / 7.5 pt | icon left 10; label after 22 px.  |
| `toolbar-fix` | (1067, 367, 62, 28) | (1067, 367, 62, 28) | #1C1D1E | #262729; 2/1/1/2 px = 2/1/1/2 pt | 6.5 px / 6.5 pt | icon left 10; label after 22 px.  |
| `toolbar-more` | (1140, 367, 39, 28) | (1140, 367, 39, 28) | #1B1B1E | #272829; 2/2/1/2 px = 2/2/1/2 pt | 7 px / 7 pt | icon left 10; label after 22 px.  |
| `agent-dropdown` | (894, 451, 307, 145) | (894, 451, 307, 145) | #141516 | #261718; 2/3/1/1 px = 2/3/1/1 pt | 11 px / 11 pt | row inset 6; rows approximately 27 px; label x+43. Small pointer points toward (915,447); popup occludes the code. |
| `agent-selected-row` | (900, 456, 295, 27) | (900, 456, 295, 27) | #602728 | #662D2B; 2/2/3/3 px = 2/2/3/3 pt | 6.5 px / 6.5 pt | cube x+9; title x+36; role right 10.  |
| `tab-agent-divider` | (749, 91, 1, 22) | (749, 91, 1, 22) | #161617 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |
| `tab-schema-divider` | (956, 91, 1, 22) | (956, 91, 1, 22) | #171718 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |

### Text instances

Use the style key in the typography table for the disclosed reconstruction family/size/weight. These bounds are thresholded **ink**, not text-layout allocation. Every foreground hex is an actual source sample; colors can differ slightly between instances. Mask thresholds and sample coordinates are in measurements.json.

| Text / instance | Style | Ink bounds px (M) | Ink bounds logical pt | Foreground composite (M) |
|---|---|---|---|---|
| Room.tsx · `room-tab-label` | `tab` | (433, 96, 64, 12) | (433, 96, 64, 12) | #FEFEFE |
| AgentSession.ts · `agent-tab-label` | `tab` | (587, 96, 108, 15) | (587, 96, 108, 15) | #FAFBFC |
| room.schema.json · `schema-tab-label` | `tab` | (791, 96, 125, 16) | (791, 96, 125, 16) | #FEFEFE |
| apps · `breadcrumb-apps` | `breadcrumb` | (409, 139, 30, 11) | (409, 139, 30, 11) | #CCCBCA |
| desktop · `breadcrumb-desktop` | `breadcrumb` | (460, 136, 50, 14) | (460, 136, 50, 14) | #CACACC |
| src · `breadcrumb-src` | `breadcrumb` | (531, 139, 17, 8) | (531, 139, 17, 8) | #C2C5C3 |
| features · `breadcrumb-features` | `breadcrumb` | (569, 136, 50, 11) | (569, 136, 50, 11) | #D4D3D1 |
| room · `breadcrumb-room` | `breadcrumb` | (641, 139, 32, 8) | (641, 139, 32, 8) | #D1D2D1 |
| Room.tsx · `breadcrumb-file` | `breadcrumb` | (722, 136, 57, 11) | (722, 136, 57, 11) | #DBE1E0 |
| 1 · `line-number-01` | `line-number` | (432, 172, 6, 11) | (432, 172, 6, 11) | #8C8D8F |
| import { useRoom, useAgents } from '@forge-room/sdk' · `code-line-01` | `code` | (475, 171, 452, 15) | (475, 171, 452, 15) | #EECAEF |
| 2 · `line-number-02` | `line-number` | (432, 193, 6, 10) | (432, 193, 6, 10) | #8F8F91 |
| import { RoomHeader, MemberList, ThreadList, TaskList } from '../components' · `code-line-02` | `code` | (475, 191, 656, 15) | (475, 191, 656, 15) | #B6E7F1 |
| 3 · `line-number-03` | `line-number` | (432, 213, 6, 11) | (432, 213, 6, 11) | #959596 |
| import { StartAgentSession } from './actions' · `code-line-03` | `code` | (475, 212, 385, 15) | (475, 212, 385, 15) | #B8E8F7 |
| 4 · `line-number-04` | `line-number` | (431, 233, 8, 10) | (431, 233, 8, 10) | #989C9D |
| 5 · `line-number-05` | `line-number` | (432, 252, 6, 11) | (432, 252, 6, 11) | #828387 |
| export function Room() { · `code-line-05` | `code` | (475, 251, 202, 15) | (475, 251, 202, 15) | #EAE595 |
| 6 · `line-number-06` | `line-number` | (432, 272, 7, 10) | (432, 272, 7, 10) | #8D8E8F |
| const { room, members, threads, tasks } = useRoom() · `code-line-06` | `code` | (496, 270, 441, 15) | (496, 270, 441, 15) | #AEEFFB |
| 7 · `line-number-07` | `line-number` | (432, 292, 6, 10) | (432, 292, 6, 10) | #8C8E8F |
| const { agents, startSession } = useAgents(room.id) · `code-line-07` | `code` | (496, 290, 439, 15) | (496, 290, 439, 15) | #E6E5E6 |
| 8 · `line-number-08` | `line-number` | (432, 311, 6, 11) | (432, 311, 6, 11) | #979999 |
| 9 · `line-number-09` | `line-number` | (432, 331, 6, 11) | (432, 331, 6, 11) | #999A99 |
| return ( · `code-line-09` | `code` | (497, 330, 65, 14) | (497, 330, 65, 14) | #E1ADEE |
| 19 · `line-number-10` | `line-number` | (425, 351, 14, 10) | (425, 351, 14, 10) | #909294 |
| <div className="room"> · `code-line-10` | `code` | (517, 349, 187, 12) | (517, 349, 187, 12) | #79F2F5 |
| 11 · `line-number-11` | `line-number` | (425, 371, 14, 10) | (425, 371, 14, 10) | #888789 |
| <RoomHeader · `code-line-11` | `code` | (537, 370, 94, 11) | (537, 370, 94, 11) | #85F0EF |
| 12 · `line-number-12` | `line-number` | (425, 391, 14, 11) | (425, 391, 14, 11) | #918F92 |
| room={room} · `code-line-12` | `code` | (558, 390, 92, 15) | (558, 390, 92, 15) | #90EDF2 |
| 13 · `line-number-13` | `line-number` | (425, 412, 14, 10) | (425, 412, 14, 10) | #808484 |
| onInvite={() => openInvite()} · `code-line-13` | `code` | (557, 410, 251, 15) | (557, 410, 251, 15) | #E7E4CD |
| 14 · `line-number-14` | `line-number-active` | (424, 432, 16, 11) | (424, 432, 16, 11) | #EF6051 |
| onStartAgent={() => startSession({ agent: 'builder' })} · `code-line-14` | `code` | (557, 431, 483, 15) | (557, 431, 483, 15) | #6FFAFD |
| 15 · `line-number-15` | `line-number` | (425, 452, 14, 10) | (425, 452, 14, 10) | #87898C |
| /> · `code-line-15` | `code` | (537, 451, 15, 13) | (537, 451, 15, 13) | #E4E7E6 |
| 16 · `line-number-16` | `line-number` | (425, 471, 14, 11) | (425, 471, 14, 11) | #8B8F8E |
| <div className="grid grid-cols-2 gap-6"> · `code-line-16` | `code` | (537, 470, 339, 15) | (537, 470, 339, 15) | #B7C7C4 |
| 17 · `line-number-17` | `line-number` | (425, 491, 14, 10) | (425, 491, 14, 10) | #8D8D92 |
| <MemberList members={members} /> · `code-line-17` | `code` | (557, 490, 273, 14) | (557, 490, 273, 14) | #CFD7E2 |
| 18 · `line-number-18` | `line-number` | (425, 511, 14, 11) | (425, 511, 14, 11) | #919495 |
| <TaskList tasks={tasks} /> · `code-line-18` | `code` | (557, 510, 222, 15) | (557, 510, 222, 15) | #74F5F7 |
| 19 · `line-number-19` | `line-number` | (425, 531, 14, 11) | (425, 531, 14, 11) | #88888A |
| </div> · `code-line-19` | `code` | (538, 530, 49, 13) | (538, 530, 49, 13) | #DFDFE1 |
| 20 · `line-number-20` | `line-number` | (424, 551, 15, 10) | (424, 551, 15, 10) | #949897 |
| </div> · `code-line-20` | `code` | (517, 550, 49, 13) | (517, 550, 49, 13) | #DEDEDE |
| 21 · `line-number-21` | `line-number` | (425, 571, 14, 10) | (425, 571, 14, 10) | #999B9D |
| ) · `code-line-21` | `code` | (497, 570, 4, 14) | (497, 570, 4, 14) | #E0A7DF |
| 22 · `line-number-22` | `line-number` | (424, 590, 15, 10) | (424, 590, 15, 10) | #949695 |
| } · `code-line-22` | `code` | (476, 589, 5, 14) | (476, 589, 5, 14) | #F1DC21 |
| Explain · `toolbar-label-explain` | `ui` | (782, 375, 43, 14) | (782, 375, 43, 14) | #F3F4F4 |
| Refactor · `toolbar-label-refactor` | `ui` | (882, 375, 51, 11) | (882, 375, 51, 11) | #F2F2F3 |
| Add tests · `toolbar-label-add-tests` | `ui` | (988, 375, 59, 11) | (988, 375, 59, 11) | #F3F3F3 |
| Fix · `toolbar-label-fix` | `ui` | (1102, 375, 16, 11) | (1102, 375, 16, 11) | #EBEAEB |
| builder · `agent-name-builder` | `tab` | (938, 464, 46, 12) | (938, 464, 46, 12) | #FCF9FA |
| Agent (recommended) · `agent-role-builder` | `small-helper` | (1057, 465, 127, 12) | (1057, 465, 127, 12) | #E7CCC9 |
| reviewer · `agent-name-reviewer` | `tab` | (939, 491, 53, 11) | (939, 491, 53, 11) | #F5F6F4 |
| Agent · `agent-role-reviewer` | `small-helper` | (1150, 492, 33, 12) | (1150, 492, 33, 12) | #A5A7A3 |
| tester · `agent-name-tester` | `tab` | (938, 519, 37, 10) | (938, 519, 37, 10) | #F6F6F7 |
| Agent · `agent-role-tester` | `small-helper` | (1150, 518, 33, 12) | (1150, 518, 33, 12) | #ABAFB1 |
| docs · `agent-name-docs` | `tab` | (938, 544, 30, 11) | (938, 544, 30, 11) | #EFF2F4 |
| Agent · `agent-role-docs` | `small-helper` | (1150, 544, 33, 13) | (1150, 544, 33, 13) | #A6A7A8 |
| custom... · `agent-name-custom` | `tab` | (938, 572, 57, 10) | (938, 572, 57, 10) | #F0EFF1 |
| Create custom agent... · `agent-role-custom` | `small-helper` | (1052, 572, 124, 13) | (1052, 572, 124, 13) | #B6B6B9 |

### Icon instances

These are painted-mask sizes. Original icon viewBoxes, exact source stroke widths, and hit targets are U. SVG paths are traced from the selected masks at subpixel contour placement; the original vectors are not recovered.

| Icon | Painted bounds px (M) | Painted bounds logical pt | Painted size px / pt | Foreground composite color(s) | Notes |
|---|---|---|---|---|---|
| `room-tab-close` | (521, 97, 9, 9) | (521, 97, 9, 9) | 9×9 px / 9×9 pt | #F6F6F5 |  |
| `editor-new-tab` | (977, 95, 13, 13) | (977, 95, 13, 13) | 13×13 px / 13×13 pt | #D1D1D2 |  |
| `breadcrumb-chevron-0` | (447, 138, 4, 8) | (447, 138, 4, 8) | 4×8 px / 4×8 pt | #ACADAB |  |
| `breadcrumb-chevron-1` | (518, 138, 5, 8) | (518, 138, 5, 8) | 5×8 px / 5×8 pt | #9D9FA0 |  |
| `breadcrumb-chevron-2` | (559, 140, 2, 3) | (559, 140, 2, 3) | 2×3 px / 2×3 pt | #929394 |  |
| `breadcrumb-chevron-3` | (630, 140, 2, 4) | (630, 140, 2, 4) | 2×4 px / 2×4 pt | #BEBEBC |  |
| `breadcrumb-chevron-4` | (683, 138, 4, 7) | (683, 138, 4, 7) | 4×7 px / 4×7 pt | #BCBEBD |  |
| `breadcrumb-ts` | (698, 137, 10, 7) | (698, 137, 10, 7) | 10×7 px / 10×7 pt | #8D908E |  |
| `toolbar-icon-explain` | (758, 374, 15, 14) | (758, 374, 15, 14) | 15×14 px / 15×14 pt | #D95B4B |  |
| `toolbar-icon-refactor` | (858, 373, 14, 15) | (858, 373, 14, 15) | 14×15 px / 14×15 pt | #E5E4E5 |  |
| `toolbar-icon-add-tests` | (965, 374, 16, 14) | (965, 374, 16, 14) | 16×14 px / 16×14 pt | #ECEAEC |  |
| `toolbar-icon-fix` | (1079, 373, 14, 15) | (1079, 373, 14, 15) | 14×15 px / 14×15 pt | #E15850 |  |
| `toolbar-icon-more` | (1152, 379, 15, 4) | (1152, 379, 15, 4) | 15×4 px / 15×4 pt | #FFFFFF |  |
| `agent-cube-builder` | (911, 461, 16, 17) | (911, 461, 16, 17) | 16×17 px / 16×17 pt | #ECDBD5 |  |
| `agent-cube-reviewer` | (911, 489, 15, 16) | (911, 489, 15, 16) | 15×16 px / 15×16 pt | #E3DBDB |  |
| `agent-cube-tester` | (911, 515, 15, 16) | (911, 515, 15, 16) | 15×16 px / 15×16 pt | #DFDFDD |  |
| `agent-cube-docs` | (911, 542, 15, 16) | (911, 542, 15, 16) | 15×16 px / 15×16 pt | #DFDFDF |  |
| `agent-cube-custom` | (911, 569, 15, 16) | (911, 569, 15, 16) | 15×16 px / 15×16 pt | #E9E5EA |  |

## 7. Bottom panel

[Annotated PNG](annotations/07-bottom-panel.png)

The bottom panel begins beneath the y=625 px / pt divider ridge. Its selected tab band is 44 px / pt high; Terminal has a 3 px / pt red underline core. Output, Problems, the 0 pill, Debug Console, Ports, Run with dropdown, split, trash, and More are independently represented.

Terminal text is positioned from measured ink bounds. The prompt has cyan/blue/red/yellow spans, the ready check is green, Local/Network arrows are blue, and localhost is blue with an underline in the screenshot. The Current Branch card at (977,681) measures 230 × 155 px / pt. It includes heading and status dividers, branch/check marks, Ahead 2 / Behind 0 text, and a 208 × 35 px / pt Create Pull Request button. The terminal/card divider is at x=966 px / pt. Terminal emulation, command execution, scrollback, links, resizing, and tab interactions are not established by this still image.


### Container, border, and content-offset measurements

Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.

| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |
|---|---|---|---|---|---|---|
| `bottom-tabs` | (388, 626, 829, 44) | (388, 626, 829, 44) | #0F0F10 | no distinct outline selected; U source width | 0 px / 0 pt | first tab x+15; y+7.  |
| `terminal-tab` | (402, 633, 88, 37) | (402, 633, 88, 37) | #1B1B1C | no distinct outline selected; U source width | 9 px / 9 pt | label left 17; underline y=666..669.  |
| `terminal-tab-underline` | (406, 666, 80, 3) | (406, 666, 80, 3) | #F94241 | no distinct outline selected; U source width | 0 px / 0 pt | 0.  |
| `problem-count-pill` | (668, 637, 21, 22) | (668, 637, 21, 22) | #323337 | no distinct outline selected; U source width | 10.5 px / 10.5 pt | 0.  |
| `terminal-run-button` | (1004, 633, 88, 30) | (1004, 633, 88, 30) | #1A1B1D | #252526; 1/2/2/2 px = 1/2/2/2 pt | 7.5 px / 7.5 pt | play left 11; label x+33; chevron right 9.  |
| `current-branch-card` | (977, 681, 230, 155) | (977, 681, 230, 155) | #0E0F10 | #1F1F21; 2/2/1/2 px = 2/2/1/2 pt | 11.5 px / 11.5 pt | title left 12; internal dividers y=711 and y=752. Card is inside the terminal pane at right; left splitter at x=966. |
| `create-pr-button` | (988, 791, 208, 35) | (988, 791, 208, 35) | #1A1B1D | #1B1D1E; 1/2/1/2 px = 1/2/1/2 pt | 8 px / 8 pt | label centered left; external link right 9.  |
| `terminal-card-splitter` | (966, 670, 1, 191) | (966, 670, 1, 191) | #141516 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |
| `branch-heading-divider` | (979, 711, 226, 1) | (979, 711, 226, 1) | #111213 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |
| `branch-status-divider` | (979, 751, 226, 1) | (979, 751, 226, 1) | #0C0C0D | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |

### Text instances

Use the style key in the typography table for the disclosed reconstruction family/size/weight. These bounds are thresholded **ink**, not text-layout allocation. Every foreground hex is an actual source sample; colors can differ slightly between instances. Mask thresholds and sample coordinates are in measurements.json.

| Text / instance | Style | Ink bounds px (M) | Ink bounds logical pt | Foreground composite (M) |
|---|---|---|---|---|
| Terminal · `panel-tab-terminal` | `ui` | (419, 643, 54, 11) | (419, 643, 54, 11) | #F5F3F3 |
| Output · `panel-tab-output` | `ui` | (516, 643, 45, 13) | (516, 643, 45, 13) | #C6CBCA |
| Problems · `panel-tab-problems` | `ui` | (600, 642, 58, 11) | (600, 642, 58, 11) | #CDD0D1 |
| Debug Console · `panel-tab-debug-console` | `ui` | (729, 642, 97, 14) | (729, 642, 97, 14) | #C8CBCD |
| Ports · `panel-tab-ports` | `ui` | (866, 643, 31, 10) | (866, 643, 31, 10) | #CECED1 |
| 0 · `problem-count` | `small-ui` | (675, 643, 7, 10) | (675, 643, 7, 10) | #DEDEE0 |
| Run · `terminal-run-label` | `ui` | (1038, 643, 23, 11) | (1038, 643, 23, 11) | #F6F8F9 |
| forge-room-app git:(feature/room-collab) ✗ npm run dev · `terminal-prompt` | `terminal-prompt` | (443, 689, 449, 15) | (443, 689, 449, 15) | #E6E5E6 |
| > forge-room-app@0.1.0 dev · `terminal-package` | `terminal` | (421, 720, 206, 15) | (421, 720, 206, 15) | #FAFAF9 |
| > vite · `terminal-vite` | `terminal` | (421, 740, 46, 11) | (421, 740, 46, 11) | #F5F6F5 |
| VITE v5.4.2   ready in 412 ms · `terminal-ready` | `terminal` | (443, 769, 236, 14) | (443, 769, 236, 14) | #E4E3E3 |
| Local: · `terminal-local-label` | `terminal` | (454, 797, 43, 11) | (454, 797, 43, 11) | #F7F7F7 |
| http://localhost:5173 · `terminal-local-url` | `terminal-link` | (549, 797, 164, 15) | (549, 797, 164, 15) | #88CDF3 |
| Network: · `terminal-network-label` | `terminal` | (453, 818, 60, 11) | (453, 818, 60, 11) | #FAF9F9 |
| http://192.168.1.24:5173 · `terminal-network-url` | `terminal` | (549, 818, 187, 14) | (549, 818, 187, 14) | #F9FAFA |
| Current Branch · `branch-card-heading` | `ui` | (990, 692, 91, 11) | (990, 692, 91, 11) | #F0F0EF |
| feature/room-collab · `branch-card-name` | `ui` | (1018, 725, 122, 12) | (1018, 725, 122, 12) | #F1F4F3 |
| Ahead 2 ↓ Behind 0 · `branch-ahead` | `ui` | (1024, 765, 122, 11) | (1024, 765, 122, 11) | #E3E3E2 |
| Create Pull Request... · `create-pr-label` | `ui` | (1014, 803, 134, 14) | (1014, 803, 134, 14) | #F2F2F2 |

### Icon instances

These are painted-mask sizes. Original icon viewBoxes, exact source stroke widths, and hit targets are U. SVG paths are traced from the selected masks at subpixel contour placement; the original vectors are not recovered.

| Icon | Painted bounds px (M) | Painted bounds logical pt | Painted size px / pt | Foreground composite color(s) | Notes |
|---|---|---|---|---|---|
| `terminal-run` | (1017, 641, 12, 14) | (1017, 641, 12, 14) | 12×14 px / 12×14 pt | #E3E6E8 |  |
| `terminal-run-chevron` | (1072, 646, 9, 5) | (1072, 646, 9, 5) | 9×5 px / 9×5 pt | #F4F6F6 |  |
| `terminal-split` | (1107, 641, 16, 15) | (1107, 641, 16, 15) | 16×15 px / 16×15 pt | #E4E2E3 |  |
| `terminal-trash` | (1146, 639, 16, 17) | (1146, 639, 16, 17) | 16×17 px / 16×17 pt | #EDECED |  |
| `terminal-more` | (1184, 646, 16, 3) | (1184, 646, 16, 3) | 16×3 px / 16×3 pt | #FFFFFF |  |
| `terminal-prompt-dot` | (421, 691, 9, 9) | (421, 691, 9, 9) | 9×9 px / 9×9 pt | #58F183 |  |
| `terminal-ready-check` | (421, 769, 12, 10) | (421, 769, 12, 10) | 12×10 px / 12×10 pt | #6FEC8C |  |
| `terminal-local-arrow` | (428, 800, 12, 7) | (428, 800, 12, 7) | 12×7 px / 12×7 pt | #98D4EB |  |
| `terminal-network-arrow` | (428, 820, 12, 8) | (428, 820, 12, 8) | 12×8 px / 12×8 pt | #B4D7EC |  |
| `branch-card-branch` | (993, 722, 13, 17) | (993, 722, 13, 17) | 13×17 px / 13×17 pt | #F1F1EF |  |
| `branch-card-check` | (1177, 725, 15, 11) | (1177, 725, 15, 11) | 15×11 px / 15×11 pt | #6DEF8B |  |
| `branch-ahead-behind` | (998, 765, 14, 11) | (998, 765, 14, 11) | 14×11 px / 14×11 pt | #D7D5D5 |  |
| `create-pr-external` | (1173, 802, 13, 13) | (1173, 802, 13, 13) | 13×13 px / 13×13 pt | #DDDFDE |  |

## 8. Right dock

[Annotated PNG](annotations/08-right-dock.png)

The right dock is 428 px / pt wide. AI Assistant is the active tab; Room is inactive; More sits at the far right. The brand header has a separate ring/plus logo, title, and subtitle. The quick-actions card is 402 × 150 px / pt, containing a two-column layout: Implement a feature, Explain this code, Write tests, Find and fix issues. The selected button envelopes give an 11 px / pt horizontal gap and 6 px / pt vertical gap.

The user message has a 352 × 63 px / pt bubble and a separate avatar; its two visible line starts are measured individually. The assistant response has no filled speech bubble and uses a separate small ring/plus logo. The Edited summary is 352 × 82 px / pt; both rows include a green check, Edited label, a filename chip, a green addition count, and a red deletion count. Room.tsx shows +42 and −6; AgentPresence.tsx shows +98 and −2.

The composer is 400 × 124 px / pt. The placeholder wraps onto two explicit lines. Its bottom row contains plus, @, cube, Forge AI Pro with a chevron, and a 42 × 36 px / pt red send button. The button's fill and nearby glow are spatially varying. Placeholder alpha, focus ring, text entry behavior, send-disabled state, scroll behavior, and active Room-tab content are U.


### Container, border, and content-offset measurements

Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.

| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |
|---|---|---|---|---|---|---|
| `dock-tabs` | (1220, 77, 427, 49) | (1220, 77, 427, 49) | #0E0E0F | no distinct outline selected; U source width | 0 px / 0 pt | active tab x+4.  |
| `ai-tab` | (1224, 81, 156, 45) | (1224, 81, 156, 45) | #18191A | no distinct outline selected; U source width | 9 px / 9 pt | sparkle left 17; label x+47.  |
| `ai-tab-underline` | (1227, 122, 149, 3) | (1227, 122, 149, 3) | #F74646 | no distinct outline selected; U source width | 0 px / 0 pt | 0.  |
| `quick-actions-card` | (1235, 209, 402, 150) | (1235, 209, 402, 150) | #191B1D | #2E2F30; 1/2/1/1 px = 1/2/1/1 pt | 12 px / 12 pt | heading left 15, top 15; button grid left 13, right 13; gap 11 horizontally, 6 vertically.  |
| `quick-action-implement` | (1248, 261, 182, 40) | (1248, 261, 182, 40) | #1B1C1D | #252627; 1/1/2/1 px = 1/1/2/1 pt | 9 px / 9 pt | icon and label centered as a group.  |
| `quick-action-explain` | (1441, 261, 183, 40) | (1441, 261, 183, 40) | #1B1C1E | #252627; 1/1/2/1 px = 1/1/2/1 pt | 8.5 px / 8.5 pt | icon and label centered as a group.  |
| `quick-action-tests` | (1248, 307, 182, 39) | (1248, 307, 182, 39) | #1B1C1D | #292A2B; 1/1/1/2 px = 1/1/1/2 pt | 9 px / 9 pt | icon and label centered as a group.  |
| `quick-action-fix` | (1441, 307, 183, 39) | (1441, 307, 183, 39) | #1B1C1D | #292A2C; 1/1/1/2 px = 1/1/1/2 pt | 8.5 px / 8.5 pt | icon and label centered as a group.  |
| `user-message-bubble` | (1285, 375, 352, 63) | (1285, 375, 352, 63) | #1F2022 | no distinct outline selected; U source width | 12.5 px / 12.5 pt | left 16; top 10; line advance 23. Small left pointer toward the avatar; no border visible. |
| `edited-summary` | (1285, 510, 352, 82) | (1285, 510, 352, 82) | #171719 | #2C2D2E; 2/2/1/2 px = 2/2/1/2 pt | 10 px / 10 pt | checks left 16; label left 43; filename left 98; counts right 14; divider y=550.  |
| `filename-chip-room` | (1383, 517, 81, 27) | (1383, 517, 81, 27) | #212223 | #1D1E20; 2/1/1/2 px = 2/1/1/2 pt | 6.5 px / 6.5 pt | text left 11; top 5.  |
| `filename-chip-presence` | (1383, 555, 147, 27) | (1383, 555, 147, 27) | #1C1D20 | #282A2C; 1/2/1/1 px = 1/2/1/1 pt | 6.5 px / 6.5 pt | text left 11; top 5.  |
| `composer` | (1237, 617, 400, 124) | (1237, 617, 400, 124) | #1A1B1C | #202121; 2/2/2/1 px = 2/2/2/1 pt | 12 px / 12 pt | text left 16, top 14; line advance 23; bottom tools center near y=713; send right 9.  |
| `send-button` | (1587, 695, 42, 36) | (1587, 695, 42, 36) | #DD3E3B | #DB3A39; 2/3/3/3 px = 2/3/3/3 pt | 9.5 px / 9.5 pt | paper plane centered. Red area is visibly nonuniform; sample is a composite color, source gradient unknown. |
| `edited-row-divider` | (1290, 549, 341, 1) | (1290, 549, 341, 1) | #161718 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |

### Text instances

Use the style key in the typography table for the disclosed reconstruction family/size/weight. These bounds are thresholded **ink**, not text-layout allocation. Every foreground hex is an actual source sample; colors can differ slightly between instances. Mask thresholds and sample coordinates are in measurements.json.

| Text / instance | Style | Ink bounds px (M) | Ink bounds logical pt | Foreground composite (M) |
|---|---|---|---|---|
| AI Assistant · `ai-tab-label` | `tab` | (1271, 96, 79, 12) | (1271, 96, 79, 12) | #F6F7F4 |
| Room · `dock-room-label` | `tab` | (1439, 96, 38, 12) | (1439, 96, 38, 12) | #F3F3F5 |
| Forge Room AI · `assistant-brand-title` | `section-heading` | (1300, 151, 106, 16) | (1300, 151, 106, 16) | #FFFFFF |
| Agentic coding in your workspace · `assistant-brand-subtitle` | `small-ui` | (1299, 173, 207, 14) | (1299, 173, 207, 14) | #BAC0C2 |
| How can I help you today? · `quick-actions-heading` | `card-heading` | (1251, 227, 215, 18) | (1251, 227, 215, 18) | #FFFFFF |
| Implement a feature · `quick-action-label-implement` | `small-ui` | (1290, 275, 119, 14) | (1290, 275, 119, 14) | #ECEDEE |
| Explain this code · `quick-action-label-explain` | `small-ui` | (1496, 275, 102, 14) | (1496, 275, 102, 14) | #F3EFF2 |
| Write tests · `quick-action-label-tests` | `small-ui` | (1303, 320, 64, 11) | (1303, 320, 64, 11) | #EAEAED |
| Find and fix issues · `quick-action-label-fix` | `small-ui` | (1496, 320, 109, 11) | (1496, 320, 109, 11) | #E8E7E8 |
| Update the Room component to show · `user-message-line-1` | `chat-body` | (1302, 388, 272, 16) | (1302, 388, 272, 16) | #F5F4F5 |
| agent presence and a live status badge. · `user-message-line-2` | `chat-body` | (1301, 411, 279, 16) | (1301, 411, 279, 16) | #F9F8F8 |
| I’ll update the Room component to include · `assistant-message-line-1` | `chat-body` | (1285, 458, 300, 16) | (1285, 458, 300, 16) | #F3F1F2 |
| agent presence with real-time status indicators. · `assistant-message-line-2` | `chat-body` | (1285, 481, 330, 16) | (1285, 481, 330, 16) | #F3F4F3 |
| Edited · `edited-label-room` | `ui` | (1329, 526, 39, 11) | (1329, 526, 39, 11) | #EDEDEF |
| Room.tsx · `edited-file-room` | `ui` | (1395, 525, 58, 11) | (1395, 525, 58, 11) | #EFF0F1 |
| +42 · `edited-plus-room` | `diff-green` | (1574, 526, 23, 11) | (1574, 526, 23, 11) | #5ECD76 |
| −6 · `edited-minus-room` | `diff-red` | (1605, 526, 16, 11) | (1605, 526, 16, 11) | #E54330 |
| Edited · `edited-label-presence` | `ui` | (1329, 563, 39, 11) | (1329, 563, 39, 11) | #EFEEEE |
| AgentPresence.tsx · `edited-file-presence` | `ui` | (1395, 563, 122, 14) | (1395, 563, 122, 14) | #F3F2F5 |
| +98 · `edited-plus-presence` | `diff-green` | (1574, 563, 24, 11) | (1574, 563, 24, 11) | #63DB82 |
| −2 · `edited-minus-presence` | `diff-red` | (1605, 563, 15, 11) | (1605, 563, 15, 11) | #DF473E |
| Ask anything, or use @ to reference files, · `composer-placeholder-1` | `placeholder` | (1254, 633, 295, 16) | (1254, 633, 295, 16) | #B8BCC0 |
| agents, or Room context... · `composer-placeholder-2` | `placeholder` | (1254, 657, 182, 15) | (1254, 657, 182, 15) | #B6BABC |
| Forge AI Pro · `composer-model` | `ui` | (1476, 707, 72, 13) | (1476, 707, 72, 13) | #E7E8E8 |

### Icon instances

These are painted-mask sizes. Original icon viewBoxes, exact source stroke widths, and hit targets are U. SVG paths are traced from the selected masks at subpixel contour placement; the original vectors are not recovered.

| Icon | Painted bounds px (M) | Painted bounds logical pt | Painted size px / pt | Foreground composite color(s) | Notes |
|---|---|---|---|---|---|
| `ai-tab-sparkle` | (1241, 94, 17, 17) | (1241, 94, 17, 17) | 17×17 px / 17×17 pt | #FF4538 |  |
| `room-tab-people` | (1409, 95, 19, 14) | (1409, 95, 19, 14) | 19×14 px / 19×14 pt | #BFBEBE |  |
| `dock-more` | (1607, 100, 15, 3) | (1607, 100, 15, 3) | 15×3 px / 15×3 pt | #FFFFFF |  |
| `assistant-brand-logo` | (1245, 150, 36, 35) | (1245, 150, 36, 35) | 36×35 px / 36×35 pt | #FAF9FA, #EE4338 |  |
| `quick-action-icon-implement` | (1265, 275, 17, 11) | (1265, 275, 17, 11) | 17×11 px / 17×11 pt | #F8F7F6 |  |
| `quick-action-icon-explain` | (1459, 272, 20, 17) | (1459, 272, 20, 17) | 20×17 px / 20×17 pt | #E9E8E7 |  |
| `quick-action-icon-tests` | (1267, 316, 17, 20) | (1267, 316, 17, 20) | 17×20 px / 17×20 pt | #F3F2F6 |  |
| `quick-action-icon-fix` | (1458, 316, 20, 20) | (1458, 316, 20, 20) | 20×20 px / 20×20 pt | #FDFEFD |  |
| `user-avatar` | (1247, 391, 19, 20) | (1247, 391, 19, 20) | 19×20 px / 19×20 pt | #E5E8E6 | Circular dark avatar behind a head-and-shoulders outline. |
| `assistant-message-logo` | (1243, 457, 28, 27) | (1243, 457, 28, 27) | 28×27 px / 28×27 pt | #F5F5F6, #F44645 |  |
| `edited-check-room` | (1301, 526, 15, 11) | (1301, 526, 15, 11) | 15×11 px / 15×11 pt | #59E679 |  |
| `edited-check-presence` | (1301, 563, 16, 11) | (1301, 563, 16, 11) | 16×11 px / 16×11 pt | #5FE87D |  |
| `composer-plus` | (1258, 704, 17, 16) | (1258, 704, 17, 16) | 17×16 px / 17×16 pt | #D3D5D5 |  |
| `composer-at` | (1299, 704, 17, 17) | (1299, 704, 17, 17) | 17×17 px / 17×17 pt | #BEBFC3 |  |
| `composer-cube` | (1340, 702, 19, 20) | (1340, 702, 19, 20) | 19×20 px / 19×20 pt | #BFC1C1 |  |
| `composer-model-chevron` | (1558, 710, 9, 5) | (1558, 710, 9, 5) | 9×5 px / 9×5 pt | #E5E4E3 |  |
| `send-plane` | (1601, 704, 16, 17) | (1601, 704, 16, 17) | 16×17 px / 16×17 pt | #FFFAF9 |  |

## 9. Status bar

[Annotated PNG](annotations/09-status-bar.png)

The selected status content band is 41 px / pt high, between its upper divider and lower frame border. Left to right: branch icon + feature/room-collab + dropdown; separator; sync icon + `2 ↑ 0 ↓`; separator; branch/PR mark + PR #142; Review needed pill; separator; red lightning + Room: Product Launch; green presence dot + `4 members · 2 agents`; flexible blank space; Ln 14, Col 52; UTF-8; CRLF; home; separator; cube + Forge AI Pro.

The item table contains the actual ink and separator positions, avoiding assumptions about equal widths. The green presence dot is not a text bullet. The visible data describe this one frame; no behavior, menus, live counts, navigation action, or responsive collapse order can be measured from it.


### Container, border, and content-offset measurements

Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.

| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |
|---|---|---|---|---|---|---|
| `status-band` | (29, 862, 1618, 41) | (29, 862, 1618, 41) | #0D0F10 | no distinct outline selected; U source width | 0 px / 0 pt | left item x+23; common visual center y=882.  |
| `review-needed-pill` | (470, 870, 108, 24) | (470, 870, 108, 24) | #381717 | no distinct outline selected; U source width | 12 px / 12 pt | text left 10, top 4.  |
| `status-separator-1` | (245, 872, 1, 21) | (245, 872, 1, 21) | #151517 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |
| `status-separator-2` | (357, 872, 1, 21) | (357, 872, 1, 21) | #171819 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |
| `status-separator-3` | (635, 872, 1, 21) | (635, 872, 1, 21) | #1A1B1C | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |
| `status-separator-4` | (1490, 872, 1, 21) | (1490, 872, 1, 21) | #151617 | no distinct outline selected; U source width | 0 px / 0 pt | 0. Measured visible separator band. |

### Text instances

Use the style key in the typography table for the disclosed reconstruction family/size/weight. These bounds are thresholded **ink**, not text-layout allocation. Every foreground hex is an actual source sample; colors can differ slightly between instances. Mask thresholds and sample coordinates are in measurements.json.

| Text / instance | Style | Ink bounds px (M) | Ink bounds logical pt | Foreground composite (M) |
|---|---|---|---|---|
| feature/room-collab · `status-branch` | `status` | (81, 876, 129, 13) | (81, 876, 129, 13) | #F6F6F6 |
| 2 ↑ 0 ↓ · `status-sync` | `status` | (294, 877, 41, 11) | (294, 877, 41, 11) | #E4E2E3 |
| PR #142 · `status-pull-request` | `status` | (407, 877, 50, 10) | (407, 877, 50, 10) | #F3F2F2 |
| Room: Product Launch · `status-room` | `status` | (686, 876, 143, 11) | (686, 876, 143, 11) | #F2F6F4 |
| 4 members · 2 agents · `status-members` | `status` | (870, 877, 141, 13) | (870, 877, 141, 13) | #C1C1C4 |
| Ln 14, Col 52 · `status-position` | `status` | (1192, 876, 78, 13) | (1192, 876, 78, 13) | #E4E4E6 |
| UTF-8 · `status-encoding` | `status` | (1318, 877, 36, 10) | (1318, 877, 36, 10) | #CACDCD |
| CRLF · `status-line-ending` | `status` | (1389, 877, 30, 10) | (1389, 877, 30, 10) | #D7D7D8 |
| Forge AI Pro · `status-model` | `status` | (1536, 877, 78, 14) | (1536, 877, 78, 14) | #ECEBEC |
| Review needed · `review-needed-label` | `small-ui` | (480, 877, 89, 10) | (480, 877, 89, 10) | #F3E3E2 |

### Icon instances

These are painted-mask sizes. Original icon viewBoxes, exact source stroke widths, and hit targets are U. SVG paths are traced from the selected masks at subpixel contour placement; the original vectors are not recovered.

| Icon | Painted bounds px (M) | Painted bounds logical pt | Painted size px / pt | Foreground composite color(s) | Notes |
|---|---|---|---|---|---|
| `status-icon-branch` | (54, 873, 13, 18) | (54, 873, 13, 18) | 13×18 px / 13×18 pt | #EDECED |  |
| `status-icon-sync` | (265, 874, 16, 16) | (265, 874, 16, 16) | 16×16 px / 16×16 pt | #EDF0ED |  |
| `status-icon-pull-request` | (382, 873, 13, 18) | (382, 873, 13, 18) | 13×18 px / 13×18 pt | #EDECEA |  |
| `status-icon-room` | (657, 872, 15, 19) | (657, 872, 15, 19) | 15×19 px / 15×19 pt | #E5593F |  |
| `status-icon-members` | (852, 878, 8, 8) | (852, 878, 8, 8) | 8×8 px / 8×8 pt | #5EF27D |  |
| `status-icon-model` | (1509, 873, 18, 18) | (1509, 873, 18, 18) | 18×18 px / 18×18 pt | #EAEDEA |  |
| `status-branch-chevron` | (219, 880, 10, 5) | (219, 880, 10, 5) | 10×5 px / 10×5 pt | #EEF0F0 |  |
| `status-home` | (1456, 874, 18, 16) | (1456, 874, 18, 16) | 18×16 px / 18×16 pt | #EAEAE9 |  |

## 10. Color table, typography, and spacing

[Annotated token board](annotations/10-colors-typography-spacing.png)

The image contains **66,627 distinct RGB values**. The complete table is [all-colors.csv](measurements/all-colors.csv), including pixel counts and every observed area. A useful implementation palette is below. These role names are assigned to observed samples; they do not imply that the original image had one uniform token per role. Source-layer opacity is U for every color; stored raster alpha is 1.

| Role | Observed hex (M) | Sample (x,y), px | Sample (x,y), logical pt | Where |
|---|---|---|---|---|
| `desktop.warm` | #756563 | (10, 10) | (10, 10) | Top-left exterior |
| `desktop.dark` | #0A0B0C | (1000, 925) | (1000, 925) | Bottom exterior |
| `frame.border` | #292A2C | (800, 22) | (800, 22) | Top frame ridge |
| `divider.sidebar` | #292B2C | (386, 200) | (386, 200) | Explorer/editor divider |
| `divider.dock` | #242527 | (1216, 200) | (1216, 200) | Editor/right-dock divider |
| `divider.panel` | #25282A | (700, 625) | (700, 625) | Editor/bottom-panel divider |
| `surface.explorer` | #0C0D0E | (270, 320) | (270, 320) | Tree background |
| `surface.editor` | #0D0F10 | (1180, 250) | (1180, 250) | Empty editor region |
| `surface.dock` | #0D0E0F | (1230, 600) | (1230, 600) | Dock gutter background |
| `room.active.border` | #942223 | (40, 460) | (40, 460) | Left red border of active Room tile |
| `window.sample` | #0E0E10 | (800, 800) | (800, 800) | window |
| `title-band.sample` | #0D0F10 | (1050, 50) | (1050, 50) | title-band |
| `rail-band.sample` | #0B0D0E | (40, 300) | (40, 300) | rail-band |
| `command-palette.sample` | #181A1B | (750, 40) | (750, 40) | command-palette |
| `room-pill.sample` | #19191A | (1200, 35) | (1200, 35) | room-pill |
| `room-active-tile.sample` | #281717 | (86, 465) | (86, 465) | room-active-tile |
| `selected-tree-row.sample` | #212324 | (350, 264) | (350, 264) | selected-tree-row |
| `scm-panel.sample` | #0D0D0E | (120, 710) | (120, 710) | scm-panel |
| `badge-room.sample` | #15181C | (132, 621) | (132, 621) | badge-room |
| `badge-agents.sample` | #261515 | (132, 683) | (132, 683) | badge-agents |
| `commit-button.sample` | #E23635 | (180, 731) | (180, 731) | commit-button |
| `sync-button.sample` | #1A1A1C | (140, 780) | (140, 780) | sync-button |
| `open-pr-button.sample` | #191A1C | (145, 809) | (145, 809) | open-pr-button |
| `room-tab.sample` | #17181A | (470, 86) | (470, 86) | room-tab |
| `room-tab-underline.sample` | #F94944 | (470, 123) | (470, 123) | room-tab-underline |
| `active-line.sample` | #20171A | (470, 438) | (470, 438) | active-line |
| `caret.sample` | #433E33 | (1041, 438) | (1041, 438) | caret |
| `floating-toolbar.sample` | #1F2022 | (1176, 369) | (1176, 369) | floating-toolbar |
| `agent-dropdown.sample` | #141516 | (1189, 586) | (1189, 586) | agent-dropdown |
| `agent-selected-row.sample` | #602728 | (1100, 459) | (1100, 459) | agent-selected-row |
| `terminal-tab.sample` | #1B1B1C | (446, 638) | (446, 638) | terminal-tab |
| `terminal-tab-underline.sample` | #F94241 | (450, 667) | (450, 667) | terminal-tab-underline |
| `problem-count-pill.sample` | #323337 | (670, 648) | (670, 648) | problem-count-pill |
| `terminal-run-button.sample` | #1A1B1D | (1084, 639) | (1084, 639) | terminal-run-button |
| `current-branch-card.sample` | #0E0F10 | (990, 688) | (990, 688) | current-branch-card |
| `create-pr-button.sample` | #1A1B1D | (1190, 799) | (1190, 799) | create-pr-button |
| `ai-tab.sample` | #18191A | (1300, 86) | (1300, 86) | ai-tab |
| `ai-tab-underline.sample` | #F74646 | (1320, 123) | (1320, 123) | ai-tab-underline |
| `quick-actions-card.sample` | #191B1D | (1590, 222) | (1590, 222) | quick-actions-card |
| `quick-action-implement.sample` | #1B1C1D | (1422, 268) | (1422, 268) | quick-action-implement |
| `user-message-bubble.sample` | #1F2022 | (1600, 410) | (1600, 410) | user-message-bubble |
| `edited-summary.sample` | #171719 | (1624, 519) | (1624, 519) | edited-summary |
| `filename-chip-room.sample` | #212223 | (1457, 522) | (1457, 522) | filename-chip-room |
| `composer.sample` | #1A1B1C | (1244, 679) | (1244, 679) | composer |
| `send-button.sample` | #DD3E3B | (1592, 702) | (1592, 702) | send-button |
| `status-band.sample` | #0D0F10 | (1100, 892) | (1100, 892) | status-band |
| `review-needed-pill.sample` | #381717 | (573, 881) | (573, 881) | review-needed-pill |
| `text.brand` | #FFFFFF | (154, 54) | (154, 54) | brand-label; other instances can vary |
| `text.panel-heading` | #FFFFFF | (157, 102) | (157, 102) | explorer-heading; other instances can vary |
| `text.section-heading` | #FFFFFF | (217, 556) | (217, 556) | scm-heading; other instances can vary |
| `text.tab` | #F7F8F8 | (160, 592) | (160, 592) | changes-count; other instances can vary |
| `text.ui` | #E2E1E3 | (584, 49) | (584, 49) | palette-label; other instances can vary |
| `text.tree` | #FEFEFF | (246, 144) | (246, 144) | tree-forge-room-app; other instances can vary |
| `text.small-ui` | #D0D4D5 | (195, 625) | (195, 625) | change-path-room; other instances can vary |
| `text.small-helper` | #E7CCC9 | (1146, 471) | (1146, 471) | agent-role-builder; other instances can vary |
| `text.card-heading` | #FFFFFF | (1353, 239) | (1353, 239) | quick-actions-heading; other instances can vary |
| `text.chat-body` | #F5F4F5 | (1545, 398) | (1545, 398) | user-message-line-1; other instances can vary |
| `text.placeholder` | #B8BCC0 | (1290, 638) | (1290, 638) | composer-placeholder-1; other instances can vary |
| `text.breadcrumb` | #CCCBCA | (410, 145) | (410, 145) | breadcrumb-apps; other instances can vary |
| `text.code` | #EECAEF | (497, 176) | (497, 176) | code-line-01; other instances can vary |
| `text.line-number` | #8C8D8F | (435, 179) | (435, 179) | line-number-01; other instances can vary |
| `text.line-number-active` | #EF6051 | (436, 433) | (436, 433) | line-number-14; other instances can vary |
| `text.terminal` | #FAFAF9 | (425, 726) | (425, 726) | terminal-package; other instances can vary |
| `text.terminal-link` | #88CDF3 | (632, 803) | (632, 803) | terminal-local-url; other instances can vary |
| `text.terminal-prompt` | #E6E5E6 | (877, 695) | (877, 695) | terminal-prompt; other instances can vary |
| `text.status` | #F6F6F6 | (139, 880) | (139, 880) | status-branch; other instances can vary |
| `text.badge-blue` | #97D1F1 | (139, 623) | (139, 623) | badge-letter-room; other instances can vary |
| `text.badge-red` | #F95946 | (146, 690) | (146, 690) | badge-letter-agents; other instances can vary |
| `text.diff-green` | #5ECD76 | (1583, 533) | (1583, 533) | edited-plus-room; other instances can vary |
| `text.diff-red` | #E54330 | (1614, 533) | (1614, 533) | edited-minus-room; other instances can vary |
| `icon.notification-dot.0` | #F24A3F | (1466, 39) | (1466, 39) | notification-dot; representative mask foreground |
| `icon.title-room-people.0` | #E9EAEB | (1110, 51) | (1110, 51) | title-room-people; representative mask foreground |
| `icon.title-room-people.1` | #B9EEB9 | (1116, 54) | (1116, 54) | title-room-people; representative mask foreground |
| `icon.terminal-prompt-dot.0` | #58F183 | (424, 692) | (424, 692) | terminal-prompt-dot; representative mask foreground |
| `icon.status-icon-members.0` | #5EF27D | (858, 880) | (858, 880) | status-icon-members; representative mask foreground |
| `icon.ai-tab-sparkle.0` | #FF4538 | (1243, 102) | (1243, 102) | ai-tab-sparkle; representative mask foreground |
| `icon.edited-check-room.0` | #59E679 | (1304, 533) | (1304, 533) | edited-check-room; representative mask foreground |
| `syntax.keyword-purple` | #A57BA4 | (475, 256) | (475, 256) | Editor source glyphs; sampled color cluster |
| `syntax.identifier-cyan` | #85C5D4 | (642, 281) | (642, 281) | Editor source glyphs; sampled color cluster |
| `syntax.string-orange` | #A56D4C | (854, 176) | (854, 176) | Editor source glyphs; sampled color cluster |
| `syntax.function-yellow` | #E3E4B4 | (620, 256) | (620, 256) | Editor source glyphs; sampled color cluster |
| `syntax.tag-red` | #A43433 | (930, 481) | (930, 481) | Editor source glyphs; sampled color cluster |
| `syntax.blue` | #7DBDDB | (898, 437) | (898, 437) | Editor source glyphs; sampled color cluster |
| `syntax.punctuation` | #E0E1E0 | (559, 498) | (559, 498) | Low-chroma editor source glyphs |
| `terminal.ready-green` | #727C65 | (444, 773) | (444, 773) | Terminal ready line |
| `terminal.ready-text` | #C0BEC0 | (675, 774) | (675, 774) | Terminal ready line |

### Typography table

**Original family, nominal font size, and weight are U for all styles.** The following native/SVG values are R: practical reconstruction settings, not measurements masquerading as original tokens. Per-instance ink bounds and colors above remain M. The SVG additionally fits each text run to its measured ink envelope.

| Style | Family (R) | Size px (R) | Size logical pt (R) | Weight (R) | Observed ink-height range px / pt (M) | Color evidence |
|---|---|---|---|---|---|---|
| `brand` | Segoe UI | 19.25 | 19.25 | 600 | 19–19 px / 19–19 pt | #FFFFFF |
| `panel-heading` | Segoe UI | 16 | 16 | 600 | 17–17 px / 17–17 pt | #FFFFFF |
| `section-heading` | Segoe UI | 16.5 | 16.5 | 600 | 12–16 px / 12–16 pt | #FFFFFF |
| `tab` | Segoe UI | 16 | 16 | 400 | 10–16 px / 10–16 pt | #F7F8F8, #FEFEFE, #FAFBFC, #FCF9FA, #F5F6F4, #F6F6F7, #EFF2F4, #F0EFF1, #F6F7F4, #F3F3F5 |
| `ui` | Segoe UI | 14 | 14 | 400 | 10–14 px / 10–14 pt | #E2E1E3, #F3F4F5, #FEF0EF, #F6F6F6, #F6F8F7, #F3F4F4, #F2F2F3, #F3F3F3, #EBEAEB, #F5F3F3, #C6CBCA, #CDD0D1, #C8CBCD, #CECED1, #F6F8F9, #F0F0EF, #F1F4F3, #E3E3E2, #F2F2F2, #EDEDEF, #EFF0F1, #EFEEEE, #F3F2F5, #E7E8E8 |
| `tree` | Segoe UI | 14.5 | 14.5 | 400 | 7–15 px / 7–15 pt | #FEFEFF, #EDF0EF, #ECEEF0, #EFF0F0, #EFEDEE, #EBEDEF, #E7E5E2, #EEEDED, #E4E8E8, #E5E5E4, #E4E7E6, #EAE7E7, #E6E4E4, #E7E7E6, #EDECEB |
| `small-ui` | Segoe UI | 14 | 14 | 400 | 10–15 px / 10–15 pt | #D0D4D5, #D2D2D3, #D3D6D3, #DEDEE0, #BAC0C2, #ECEDEE, #F3EFF2, #EAEAED, #E8E7E8, #F3E3E2 |
| `small-helper` | Segoe UI | 13 | 13 | 400 | 12–13 px / 12–13 pt | #E7CCC9, #A5A7A3, #ABAFB1, #A6A7A8, #B6B6B9 |
| `card-heading` | Segoe UI | 18 | 18 | 600 | 18–18 px / 18–18 pt | #FFFFFF |
| `chat-body` | Segoe UI | 16 | 16 | 400 | 16–16 px / 16–16 pt | #F5F4F5, #F9F8F8, #F3F1F2, #F3F4F3 |
| `placeholder` | Segoe UI | 16 | 16 | 400 | 15–16 px / 15–16 pt | #B8BCC0, #B6BABC |
| `breadcrumb` | Segoe UI | 14 | 14 | 400 | 8–14 px / 8–14 pt | #CCCBCA, #CACACC, #C2C5C3, #D4D3D1, #D1D2D1, #DBE1E0 |
| `code` | Consolas | 15.5 | 15.5 | 400 | 11–15 px / 11–15 pt | #EECAEF, #B6E7F1, #B8E8F7, #EAE595, #AEEFFB, #E6E5E6, #E1ADEE, #79F2F5, #85F0EF, #90EDF2, #E7E4CD, #6FFAFD, #E4E7E6, #B7C7C4, #CFD7E2, #74F5F7, #DFDFE1, #DEDEDE, #E0A7DF, #F1DC21 |
| `line-number` | Consolas | 14 | 14 | 400 | 10–11 px / 10–11 pt | #8C8D8F, #8F8F91, #959596, #989C9D, #828387, #8D8E8F, #8C8E8F, #979999, #999A99, #909294, #888789, #918F92, #808484, #87898C, #8B8F8E, #8D8D92, #919495, #88888A, #949897, #999B9D, #949695 |
| `line-number-active` | Consolas | 14 | 14 | 400 | 11–11 px / 11–11 pt | #EF6051 |
| `terminal` | Consolas | 15 | 15 | 400 | 11–15 px / 11–15 pt | #FAFAF9, #F5F6F5, #E4E3E3, #F7F7F7, #FAF9F9, #F9FAFA |
| `terminal-link` | Consolas | 15 | 15 | 400 | 15–15 px / 15–15 pt | #88CDF3 |
| `terminal-prompt` | Consolas | 15 | 15 | 600 | 15–15 px / 15–15 pt | #E6E5E6 |
| `status` | Segoe UI | 14 | 14 | 400 | 10–14 px / 10–14 pt | #F6F6F6, #E4E2E3, #F3F2F2, #F2F6F4, #C1C1C4, #E4E4E6, #CACDCD, #D7D7D8, #ECEBEC |
| `badge-blue` | Segoe UI | 14 | 14 | 400 | 10–10 px / 10–10 pt | #97D1F1, #9DD5F7 |
| `badge-red` | Segoe UI | 14 | 14 | 400 | 11–11 px / 11–11 pt | #F95946 |
| `diff-green` | Segoe UI | 14 | 14 | 400 | 11–11 px / 11–11 pt | #5ECD76, #63DB82 |
| `diff-red` | Segoe UI | 14 | 14 | 400 | 11–11 px / 11–11 pt | #E54330, #DF473E |

### Font comparison evidence (F, not identification)

Local Segoe UI, Arial, Calibri, Consolas and Courier New files were compared at 0.25 px / pt size steps. Binary glyph masks use luminance >110; candidates are ranked by normalized shape IoU minus relative width/height error. Even the best candidate is not a recovered font. In particular the Explorer-heading sample's best candidate differs from the reconstruction choice, and short samples do not uniquely identify a family.

| Sample | Measured ink w×h px / pt | Best local candidate (F) | Candidate size px / pt | Normalized shape IoU | Relative size error |
|---|---|---|---|---|---|
| brand: Forge Room | 104×19 px / 104×19 pt | Segoe UI 600 | 19.25 px / 19.25 pt | 0.7658 | 0.0000 |
| panel-heading: Explorer | 60×17 px / 60×17 pt | Calibri 400 | 19 px / 19 pt | 0.7636 | 0.0333 |
| tab: Room.tsx | 64×12 px / 64×12 pt | Segoe UI 600 | 16 px / 16 pt | 0.7881 | 0.0312 |
| tree: desktop | 50×14 px / 50×14 pt | Segoe UI 400 | 14.5 px / 14.5 pt | 0.7870 | 0.0000 |
| section-heading: Source Control | 111×12 px / 111×12 pt | Segoe UI 600 | 16.5 px / 16.5 pt | 0.7595 | 0.0180 |
| card-heading: How can I help you today? | 215×18 px / 215×18 pt | Segoe UI 600 | 18 px / 18 pt | 0.8280 | 0.0000 |
| chat-body: Update the Room component to show | 272×16 px / 272×16 pt | Segoe UI 400 | 15.75 px / 15.75 pt | 0.7047 | 0.0221 |
| breadcrumb: apps | 30×11 px / 30×11 pt | Calibri 400 | 16.75 px / 16.75 pt | 0.6691 | 0.0000 |
| code: import | 49×15 px / 49×15 pt | Consolas 400 | 16.5 px / 16.5 pt | 0.6863 | 0.0612 |
| terminal: ready in 412 ms | 121×14 px / 121×14 pt | Consolas 700 | 15 px / 15 pt | 0.5663 | 0.0083 |
| status: feature/room-collab | 129×13 px / 129×13 pt | Segoe UI 400 | 13.75 px / 13.75 pt | 0.7238 | 0.0543 |
| small-helper: Agent | 33×12 px / 33×12 pt | Segoe UI 600 | 12 px / 12 pt | 0.5794 | 0.0000 |

### Measured spacing inventory

This is an inventory of observed distances, not an invented 4/8-point design system. Original spacing tokens and flex/layout constraints are U.

| Observation | px (M) | logical pt | Evidence |
|---|---|---|---|
| Frame left/top/right/bottom margins | 28 / 22 / 24 / 37 | 28 / 22 / 24 / 37 | source vs frame envelope |
| Rail width | 76 | 76 | rail envelope |
| Sidebar width | 281 | 281 | Explorer/SCM envelope |
| Editor width | 832 | 832 | editor envelope |
| Right dock width | 428 | 428 | dock envelope |
| Title band / editor tab band / bottom tab band | 54 / 49 / 44 | 54 / 49 / 44 | selected band envelopes |
| Selected tree row height | 26 | 26 | selected-tree-row |
| Code row-center deltas | 21 then 20 | 21 then 20 | line 1→2, then following rows |
| Quick-action column / row gaps | 11 / 6 | 11 / 6 | button envelopes |
| Commit→Sync / Sync→Open PR gaps | 9 / 9 | 9 / 9 | button envelopes |
| Toolbar horizontal button gaps | 10 / 11 / 9 / 11 | 10 / 11 / 9 / 11 | individual button selections |
| Popup left inset / selected row height | 6 / 27 | 6 / 27 | agent popup and selected row |
| Active underline bright core height | 3 | 3 | room-tab, Terminal, AI tab |
| Selected Room tile w×h | 56 × 46 | 56 × 46 | active rail envelope |
| Composer send button w×h | 42 × 36 | 42 × 36 | send-button envelope |
| Rail painted-center vertical deltas | 48 / 50.5 / 50 / 50 / 49.5 / 48 / 53 | 48 / 50.5 / 50 / 50 / 49.5 / 48 / 53 | eight isolated ink envelopes |
| Tree selected-center deltas | 26 / 25 / 25 / 24 / 24 / 25 / 24 / 24 / 24 / 25 / 24 / 24 / 23 / 24 / 24 | 26 / 25 / 25 / 24 / 24 / 25 / 24 / 24 / 24 / 25 / 24 / 24 / 23 / 24 / 24 | tree anchors above |

### Exterior warm-field measurements

Criterion: outside the framed window, R − (G+B)/2 >12, 8-connected regions of at least 50 pixels. The visible field can be truncated by the window. No source glow center/radius, shader color, or alpha is inferred from these regions.

| Visible red-excess region | Bounds px | Bounds logical pt | Peak observed RGB | Peak position px / pt | Original radius / opacity |
|---|---|---|---|---|---|
| 1 | (0, 0, 51, 47) | (0, 0, 51, 47) | #776663 | (13, 0) / (13, 0) | U / U |
| 2 | (51, 0, 104, 21) | (51, 0, 104, 21) | #71615F | (86, 0) / (86, 0) | U / U |
| 3 | (0, 169, 28, 183) | (0, 169, 28, 183) | #341A1A | (4, 200) / (4, 200) | U / U |
| 4 | (0, 447, 28, 40) | (0, 447, 28, 40) | #3B1B1B | (24, 466) / (24, 466) | U / U |
| 5 | (0, 620, 28, 32) | (0, 620, 28, 32) | #3A1D1E | (15, 634) / (15, 634) | U / U |
| 6 | (6, 762, 22, 127) | (6, 762, 22, 127) | #332124 | (25, 807) / (25, 807) | U / U |
| 7 | (20, 915, 103, 26) | (20, 915, 103, 26) | #331E1F | (84, 929) / (84, 929) | U / U |
## Native Rust/egui construction notes

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


## Source integrity

Source SHA-256: `4a57ade63d634e88e429d6b24971b0328d4d2575efc767992cd5e8af90356c9c`. The stored copy has no PNG metadata. All measurements use that exact file, not an upscaled chat preview. See validation.json for current artifact checks and renderer-comparison limits.

