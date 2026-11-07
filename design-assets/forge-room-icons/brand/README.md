# Forge Room architectural brand mark

The user supplied a new architectural F/room logo on 2026-11-02. This replaces Forge Room's previous circle-plus and double-chevron branding. The off-white room frame, ember F beam and warm doorway follow the supplied board. These SVGs are editable vector interpretations, not recovered original vectors or exact pixel traces.

- `forge-room-logo.svg`: colored mark, 256-unit viewBox, transparent background.
- `forge-room-logo-monochrome.svg`: single-color mark using `currentColor`.
- `forge-room-app-icon.svg`: opaque graphite/ember tile, 512-unit viewBox.
- `forge-room-logo-{512,128,32,24}.png`: transparent raster exports at the named square sizes.
- `forge-room-app-icon-512.png`: opaque square app-icon export.

Use the mark rather than cropping the entire brand board. Keep native proportions. Use the colored transparent version for the title bar and other Forge-owned brand labels; use the app-icon tile for executable/installer/shortcut icons. Convert the 512 export to an ICO with multiple embedded sizes in the packaging pipeline. Preserve separate Aether/project logos and purple agent identity; those are not Forge branding. Do not replace unrelated code/file-type icons.

Palette: off-white `#E8E6DD`, light face `#F0EEE6`, ember `#C74C45`, shaded ember `#A4423D`, graphite `#121314`, door `#D67A3F`. The generic icon inventory's outline-only stroke contract does not apply to this filled brand mark. Existing `svg/forge-mark.svg` is retained as the earlier double-chevron asset for compatibility; Claude should explicitly switch Forge brand surfaces to this new asset.

The recolored Code-layout reference and complete brand board are in `design-assets/forge-room-ui-spec/code-reference-ember.png` and `forge-room-brand-ember.png`. They are visual references, not evidence that the running app has already been updated. Implement structure and state from the real application rather than copying fictional participants or sample code.
