# Canonical architectural Forge Room brand delivery

Prepared for the user's new logo and Code-screen direction, and Claude's mailbox request titled T8 under Log. This folder is the canonical production handoff, superseding the earlier provisional exports in `forge-room-icons/brand/`.

- `forge-room-logo.svg`: horizontal logo lockup with the architectural symbol and Forge Room wordmark outlined as paths.
- `forge-mark-color.svg`: 256-square, transparent architectural F/room symbol with flat warm neutral, muted ember, and a small doorway highlight.
- `forge-mark-mono.svg`: 256-square single-color symbol using `currentColor`, suitable for compact title/activity use.
- `forge-mark-{16,32,48,64,128,256,512}.png`: transparent square symbol exports at named sizes.
- `forge-room.ico`: seven PNG-backed icon frames at 16/24/32/48/64/128/256, for modern Windows executable/window/installer resources.
- `code-section-recolored.png`: 1600 x 900 visual reference.
- `CODE_SECTION_SPEC.md`: measured original layout and implementation tokens.
- `build_vectors.py`: editable geometry plus native Segoe UI Semibold wordmark outlining. No font file is embedded in delivery SVGs.

All SVG drawing geometry is paths with flat fills, without gradients, blur, filters, external resources, text elements or transforms. The geometry is a faithful clean interpretation of a raster reference, not an exact recovered vector; the wordmark uses the native font rather than claiming the original board's unknown font. Colors: #E8E6DD warm neutral, #D9837F muted ember, #D67A3F small doorway highlight. UI focus/accent remains #D99A93. The compact mono variant has no fixed color.

Do not pass these filled brand assets through the generic 1.75 px outline icon renderer as strokes; tessellate filled paths (including font counters/holes), or load the PNGs. Copy assets into app/installer directories from the owning Claude process. Preserve Aether/project identities and unrelated agent/file-type icons. Existing shortcuts and installed executable resources require a packaging rebuild to reflect the new icon; a local asset delivery alone does not update an already installed app.
