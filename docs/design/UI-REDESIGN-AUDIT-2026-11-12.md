# Forge Room native UI review — 12 November 2026

## Baseline and evidence

The active product is `native/forge-room-app` (Rust/egui). `progress.md` and the root README describe its current behavior. The 32 images in `concept-images/2026-11-12` are generated references; the 20-image flat gallery is the selected visual direction. `prototype.html` demonstrates interactions but is not app code. The older Code OSS modules are migration references. Scores below assess the current native experience and concept coverage from source and assets; they are provisional until a hands-on, multi-size Windows review.

Git baseline: `origin/main` at `791b86d`; no open PRs on 12 November. Recently merged PRs #33 and #32 add the alpha.3 native work and Codex Ask. `atul/native-rust-ide` is newer at `fe53ee2` and must be reviewed before any broad UI merge. Several old feature branches remain, but no branch should be deleted without checking unique commits. No merge or branch cleanup is part of this design slice.

## Prioritized findings

| Priority | Surface (provisional /10) | What works and should stay | Structural problem | Visual problem and action |
| --- | --- | --- | --- | --- |
| 1 | Main workspace (6) | Real editor, Explorer, Git, terminal, recovery; editor stays central. | The 112 px activity rail lists more tools than fit a short window and consumes editor width. Compact it, scroll secondary entries, and keep key tools reachable. | Mixed text wrapping and badges weaken scan speed. Use one icon scale, a clear selected state and full names in tooltips. |
| 1 | Lobby / room setup (5) | Native setup and local room state. | Setup, team and countdown compete in one view; network invite and presence are still unavailable. Lead with the next actionable step and explicit local status. | Reduce nested cards and align form rows to a common grid. |
| 1 | Collaboration dock (5) | Local team, Chat, Tasks and Activity have real persistence. | Dense tab set and local-only behavior can be mistaken for live collaboration. Put connection scope beside the room title; keep filters close to their lists. | Use compact rows and separators instead of decorative containers. |
| 2 | Studio / Morph (4) | Theme gallery can preview and apply themes. | Morph, layout profiles and reversible customization are roadmap work. Show only theme controls as working; separate future ideas. | Give previews consistent size and typography. |
| 2 | Ship / readiness (2) | Local task and test evidence can inform it. | No dedicated native Ship Mode. Concept imagery must not imply verified build or deployment readiness. | Reserve status color for real results and show evidence per check. |
| 2 | Onboarding (4) | Open folder, recovery and trust flows exist. | First-run choices are scattered; trust consequences need plain copy. | Use one clear primary action and concise supporting text. |
| 3 | Workshop (1) | Architecture is documented. | Widget authoring and execution are not native capabilities. Keep concepts labeled as future work. | Do not render simulated install success. |
| 3 | Packs (1) | Product intent is documented. | Pack discovery, inspection, compatibility and rollback are not implemented. | Design a permission review before any install flow. |

The most damaging issue is structural: too many destinations occupy permanent navigation space while the editor is the principal tool. Generated images also sometimes contain approximate controls or copy. These are reference errors to correct in code, not product features to claim.

## Direction and system rules

Use the flat, matte graphite direction in concepts 13–32: restrained ember selection, 1 px dividers, compact type, and no ambient glow. A 56 px rail provides a stable anchor; full labels remain in tooltips and keyboard commands. Keep editor, sidebar, dock and bottom panel as distinct resizable work areas. The existing `theme.rs` palette is the color-token source; the Forge Room theme JSON supplies semantic colors. Use 4/8/12/16 px spacing steps, 28–40 px interactive targets, visible keyboard focus, and explicit empty, loading, error and offline copy. Status color must represent actual status.

Implementation should proceed by verified native slices: compact navigation first; then lobby hierarchy; then room dock labels and states; then theme and settings consistency. Workshop, Packs and Ship Mode need backend and acceptance design before a polished surface can honestly claim functionality.

## Validation and remaining review

The compact rail slice passed `cargo fmt --check`, `cargo test --locked` (241 passed, 5 ignored), and `cargo clippy --all-targets --locked -- -D warnings`. Its icon-only entries now use native button responses with accessible names and a visible focus ring; this follow-on passed a locked graphical compile check. A separate [eight-screen generated concept set](concept-images/2026-11-12/redesign-v2/README.md) covers the workspace, Lobby, local collaboration, Studio, Workshop, Packs, Ship Mode and onboarding. Workshop, Packs and Ship Mode are explicitly future concepts. Desktop screenshots and interaction at 760×480, 1240×780 and 1440×900 are still required. A build or generated image alone does not verify native visual quality. Re-score every surface after a screenshot-backed desktop pass.

**Native trial note:** A freshly built debug app opened a disposable `main.rs` file. Windows UI Automation exposed all 14 compact-rail entries as named buttons, including Code, Search, Room Lobby, Studio, Terminal, Settings and Accessibility. The app showed its folder-trust dialog. Window capture timed out twice, and the automation click could not obtain geometry, so visual layout, keyboard activation and the Room/Studio flows were not accepted. The trial process was closed afterward.
