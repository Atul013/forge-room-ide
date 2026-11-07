# Forge Room icons

65 standalone SVGs and a reusable symbol sprite: the original 24 **Forge Room IDE: Product Launch Workspace** icons, plus 40 Room Lobby icons from mailbox task T1 and the angular Forge mark from T4. The original 24 SVG files are byte-identical to the earlier delivery.

## Files

The user's new architectural F/room brand assets are in `brand/`. This filled brand system is separate from the 65 outline UI icons. See `brand/README.md` for the app/installer integration handoff and exact PNG export sizes. The earlier double-chevron `forge-mark` stays in the outline inventory for compatibility; Forge-owned brand surfaces should now use the architectural mark.

- `svg/`: one SVG per icon; kebab-case filenames.
- `forge-room-sprite.svg`: 65 symbols with `forge-` prefixed IDs.
- `preview.png`: labeled, dark-background comparison sheet. Each icon is rendered at exactly 24 px and 48 px.
- `preview.html`: offline preview at native CSS pixel sizes; no external dependencies.
- `manifest.json`: file names, symbol IDs, rail order, and geometry aliases.
- `validation.json`: export and rendering checks.
- `tools/`: editable path source, renderer, preview builder, and validator.

## Geometry

Every icon uses a `0 0 24 24` viewBox, `stroke="currentColor"`, `fill="none"`, a 1.75-unit stroke, and round line caps and joins. Artwork stays approximately 2 px inside the viewBox, with smaller optical bounds for narrow glyphs. Rounded containers use 2 px corners. At 48 px, the stroke scales proportionally to 3.5 px.

All visible SVG geometry consists of paths. There are no raster images, fonts, SVG text elements, fixed colors, filters, gradients, shadows, or background shapes. The three dots use short round-capped strokes. `file-json` uses drawn braces, and `file-ts` uses a hand-drawn TS mark, matching the reference's compact file-type markers; neither depends on a font or text element.

Rail order: explorer, search, source-control, run-debug, extensions, ai, terminal, room.

Additional icons: new-file, refresh, more, close, plus, chevron-right, chevron-down, check, arrow-up, arrow-down, folder-open, file, file-json, file-ts, commit-dropdown, room-active.

Room Lobby additions: code, studio, workflows, models, tests, cloud, settings, accessibility-off, pulse, users, user-plus, user, lock, clock, calendar, eye, bot, target, flag, layers, external-link, github, shield, send, warning, bell, rocket, hourglass, upload-box, share, panel-left, panel-bottom, panel-right, window-maximize, window-minimize, circle-check, circle-dot, circle, tag, arrow-right. Their editable paths are in `tools/lobby.py`. All additions inherit `currentColor`; ember accents belong to the host UI.

`room-active.svg` is byte-identical to `room.svg`. Apply the active color and tile in the host UI. `commit-dropdown.svg` is byte-identical to `chevron-down.svg`, giving the commit action its own semantic asset without varying the geometry.

## Usage

Inline an individual SVG and set `color` on it or its parent. Alternatively, serve the sprite from the same origin:

```html
<button type="button" aria-label="Open explorer" class="rail-button">
  <svg width="24" height="24" aria-hidden="true" focusable="false">
    <use href="/icons/forge-room-sprite.svg#forge-explorer"></use>
  </svg>
</button>
```

For an inline sprite, insert its root SVG once in the document, then use `href="#forge-explorer"`. Its root is zero-size; each symbol supplies its own viewBox and complete stroke styling. The sprite is a symbol library and does not display a gallery when opened by itself; use the PNG or HTML preview to inspect the full set.

```css
.rail-button {
  display: inline-grid;
  place-items: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: #A0A1A5;
}

.rail-button[aria-pressed="true"] {
  color: #EFEFEE;
  background: rgb(199 76 69 / 16%);
  border-color: rgb(199 76 69 / 52%);
}

.rail-button:focus-visible {
  outline: 2px solid #EFEFEE;
  outline-offset: 3px;
}
```

The preview background is `#121314`; accent red is `#C74C45`. These colors exist only in the previews and integration examples. When using `<img src="...">`, CSS `color` on the parent cannot recolor an external SVG document; use inline SVG or the sprite when recoloring is needed. Add accessible names to the containing controls, or to the host SVG when it conveys standalone information.

## Rebuild and validation

From the repository root, with Python, Pillow, Node.js, and the local SVG renderer installed:

```powershell
npm install --prefix artifacts/build-tools/icon-renderer --no-audit --no-fund --save-exact @resvg/resvg-js@2.6.2
python design-assets/forge-room-icons/tools/build.py
node design-assets/forge-room-icons/tools/render.cjs
python design-assets/forge-room-icons/tools/preview.py
python design-assets/forge-room-icons/tools/validate.py
```

The PNG builder uses Windows Segoe UI for labels only. Label text is rasterized into the PNG and is never included in the icon SVGs. For another operating system, change the preview builder's font directory and font names.

The T4 `forge-mark.svg` is a double-chevron angular A-like outline for the title bar and project tile. It follows the same path-only/currentColor/1.75 stroke contract and is available as `forge-forge-mark` in the sprite. This is a clean vector interpretation of the requested A-like mark, not a pixel trace of the small asymmetric top-left logo.

Validation covers the exact requested inventory, XML structure, path-only artwork, currentColor and stroke settings, room/chevron geometry aliases, all 130 standalone-versus-sprite raster comparisons, painted bounds at 16x resolution, original-icon SHA-256 hashes, and PNG integrity. Review `preview.png` at 100% zoom to judge the requested pixel sizes. The expanded PNG is 1376 x 2990 pixels.
