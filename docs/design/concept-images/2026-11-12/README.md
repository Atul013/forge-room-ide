# Forge Room UI concepts — 12 November 2026

**New redesign set:** [Eight generated major-screen concepts](redesign-v2/README.md) and their [visual gallery](redesign-v2/gallery.html). This is a separate, selected follow-on set; the earlier 32-image library below remains available for comparison.

Thirty-two generated visual explorations for the native Forge Room product. The [interactive gallery](gallery.html) curates twenty flat, restrained concepts (13–32). The [interaction prototype](prototype.html) makes the main rail, editor tabs, Room tabs, bottom tabs, menus, selectors and confirmation flow clickable. Both HTML files run locally with no dependencies. Images 01–12 are the earlier exploration and are retained as historical references; several use brighter treatments that are no longer the chosen direction. These are design references, not screenshots or implementation claims. In-image text and controls may be approximate. The existing native Rust app and roadmap remain the source of truth for available behavior. Read [the design research](DESIGN-RESEARCH.md) for evidence, dimensions and interaction requirements.

| Image | Section |
| --- | --- |
| `01-workspace.png` | Daily coding workspace |
| `02-room-lobby.png` | Create/join room lobby |
| `03-team-chat.png` | Team roster and local communication |
| `04-tasks.png` | Tasks, dependencies, evidence, handoff |
| `05-git-review.png` | Source control and review |
| `06-run-debug.png` | Run, tests, problems, future debug layout |
| `07-studio-settings.png` | Studio, themes, accessibility settings |
| `08-ship-mode.png` | Deadline and demo readiness |
| `09-first-run.png` | First-run and project opening |
| `10-activity.png` | Live activity and handoffs |
| `11-extensions.png` | Extensions and language tools |
| `12-companion.png` | Forge Pocket mobile companion concept |
| `13-menu-file.png` | File menu and shortcuts |
| `14-menu-view.png` | View menu and layout actions |
| `15-editor-tabs.png` | Editor tabs and find controls |
| `16-palette-search.png` | Command palette and Search |
| `17-explorer-context.png` | Explorer context menu |
| `18-room-controls.png` | Room setup dropdowns |
| `19-dock-tabs.png` | Right dock tabs and task selectors |
| `20-terminal-overflow.png` | Terminal and bottom panel controls |
| `21-workflows.png` | Project workflows |
| `22-models-status.png` | Tool and model status |
| `23-tests-detail.png` | Test discovery and results |
| `24-settings-detail.png` | Settings |
| `25-accessibility.png` | Accessibility and keyboard help |
| `26-chat-detail.png` | Chat detail and task-linked decisions |
| `27-agents-detail.png` | Agent identity and permission state |
| `28-activity-states.png` | Activity filters and no-match state |
| `29-flat-workspace.png` | Revised flat workspace |
| `30-flat-lobby.png` | Revised flat room lobby |
| `31-flat-ship.png` | Revised flat Ship Mode concept |
| `32-flat-pocket.png` | Revised flat Pocket companion concept |

## Generation brief

Generated with the built-in image generation tool using one prompt per image. The first twelve prompts requested high-fidelity UI concepts for the named sections with dark graphite, restrained warm ember accents, readable hierarchy and no in-app AI assistant. Prompts for communication and service screens requested honest offline or availability states rather than implied live connectivity. The first eleven assets use desktop layouts; the twelfth explores mobile screens beside a desktop view.

Prompts 13–28 share this visual brief: minimal native IDE screenshot, flat matte neutral charcoal surfaces, one muted rust accent for selection or primary action, crisp 1 px dividers, no gradients/glow/glass/neon, compact 13 px UI typography, consistent 16 px outline icons and an editor-first layout. Each adds the specific open menu, tab, dropdown, selector or state listed above.

Prompts 29–32 use the same flat style with explicit exclusions for gradients, neon, glow, glass, 3D decoration, oversized cards and atmospheric backgrounds. They fill the workspace, lobby, Ship Mode and Pocket gaps in the curated gallery.

The image filenames map to the prompts' section names above. These images are for visual direction only. Validate labels, interactions, accessibility, and capability claims in native UI code before adopting any detail.
