# Forge Room UI reference handoff

Start with [SPEC.md](SPEC.md), then use [the overview](annotations/00-overview.png) to locate the nine areas. The `annotations/` folder contains a measured source overlay per area and a color/typography/spacing board.

The source is the supplied 1672×941 PNG, copied byte-for-byte into `source/reference.png`. All source positions and dimensions also have logical-point values under the requested assumption of 1 physical pixel per egui point. `layout.rs` supplies measured area/component constants and window-local conversion helpers. At 100% scale the application's selected frame is 1620×882 points; screenshot origin (28,22) is outside-client placement.

`forge-room-layered.svg` is a vector reconstruction with named editable layers, real text nodes and traced icon paths. It has no embedded image or font. `redraw.png` is its rendered output. It is not a recovery of the original design file: font identity/nominal sizes/weights, original layer opacity, blur settings, original radius/stroke parameters and hidden interactions cannot be determined exactly from a flattened RGB screenshot. The spec labels observations, fits, reconstruction choices and unavailable values separately.

The complete source palette is in `measurements/all-colors.csv`: every distinct RGB value, pixel count and observed area. `tokens.json` and `color-roles.csv` provide named samples for implementation. The complete rectangle/text/icon inventory, source sample coordinates and border/radius evidence are in `measurements/measurements.json`.

`validation.json` reports source integrity, sample checks, palette counts, SVG structure, PNG integrity and source/redraw difference statistics. Difference statistics describe the redraw, not a claim of pixel equivalence or exact font recovery.

## Rebuild

From the repository root, use Python with Pillow, NumPy and OpenCV, plus Node.js and `@resvg/resvg-js` 2.6.2 in the existing local rendering-tool directory. The labels and SVG preview use Windows Segoe UI/semibold and Consolas from `C:/Windows/Fonts` without embedding them.

```powershell
python design-assets/forge-room-ui-spec/tools/inspect_source.py
python design-assets/forge-room-ui-spec/tools/fit_typography.py
python design-assets/forge-room-ui-spec/tools/measure.py
python design-assets/forge-room-ui-spec/tools/build_handoff.py
node design-assets/forge-room-ui-spec/tools/render.cjs
New-Item -ItemType Directory -Force artifacts/build-tools/ui-spec-verify | Out-Null
rustc --edition=2024 --crate-name forge_room_layout --crate-type lib design-assets/forge-room-ui-spec/layout.rs --out-dir artifacts/build-tools/ui-spec-verify
python design-assets/forge-room-ui-spec/tools/validate.py
python design-assets/forge-room-ui-spec/tools/package.py
```

The source-inspection command reads the included source copy by default; it also accepts an explicit original PNG path. The package excludes temporary enlarged inspection crops and Python bytecode.
