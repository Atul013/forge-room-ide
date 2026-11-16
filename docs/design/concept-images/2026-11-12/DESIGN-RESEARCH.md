# Forge Room interface research and interaction map

12 November 2026. This is a design brief for the native Rust/egui application, not a claim that all controls exist. The concept PNGs are visual references. The source inventory below was checked against `native/forge-room-app/src/` and `progress.md` on this date.

## Evidence and design decisions

| Evidence | Decision for Forge Room |
| --- | --- |
| [VS Code workbench](https://code.visualstudio.com/docs/editing/getting-started/userinterface) separates editor, primary sidebar, secondary sidebar, panel, activity bar and status bar; tabs and layout are configurable. | Give the editor the majority of width. Use sidebars and a bottom panel for supporting work; keep their widths adjustable and state persistent. |
| [VS Code custom layout](https://code.visualstudio.com/docs/configure/custom-layout) provides movable views and compact density. | Default to one practical density, with compact and comfortable options. Avoid permanent card stacks that steal editor space. |
| [JetBrains tool windows](https://www.jetbrains.com/help/idea/tool-windows.html) use tabs or a header selector and support docked/floating placement. | Allow a utility to move or float only when it helps comparison; show a clear dock/return control and remember placement. Floating is a window behavior, not a translucent visual effect. |
| [VS Code view guidance](https://code.visualstudio.com/api/ux-guidelines/views) recommends short labels, existing icons, limited actions and shallow trees. | Short rail labels, one icon style, at most three visible row actions; excess commands in context menus. |
| [WAI-ARIA tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) specifies arrow, Home/End, Enter/Space and Delete behavior. | Tab strips need roving keyboard focus and predictable close/focus restoration. |
| [WAI-ARIA menu pattern](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/) documents anchored menus, keyboard navigation and ellipsis for dialog actions. | File/Edit/View and context menus should use grouped actions, shortcut column, submenu arrow, check state and Escape dismissal. |
| [WCAG 2.2 target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum) sets a 24×24 CSS pixel minimum or adequate spacing for pointer targets. | Aim for 28–32 px hit areas in dense desktop controls, 36 px for primary buttons; do not shrink click areas to icon outlines. |
| [WCAG focus appearance](https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance) describes a visible, sufficiently contrasting focus indicator. | Use a strong 2 px focus ring on keyboard focus, including tabs, menus, trees, splitters and close buttons. |

The sizes below are Forge Room design recommendations inferred from these patterns, not values mandated by those sources. Validate on Windows at 100%, 150% and 200% scaling in the actual egui UI.

## Visual system

- Flat, matte neutral surfaces. One muted accent for selected navigation and primary actions. No gradients, glow, heavy blur, glass panels or decorative neon.
- Recommended starting geometry at a 1440×900 logical viewport: title/menu 36 px; activity rail 48–56 px; primary sidebar 260–320 px; right dock 300–360 px when open; bottom panel 180–300 px; editor gets the remaining space. The current native source uses a 112 px activity width and 42 px tab height; treat any change as a measured redesign rather than silently assuming the concept is implemented.
- UI type: a legible system sans at 13–14 px, 11–12 px only for secondary metadata. Editor: 13–15 px monospaced, line height around 1.45. Keep font choice user-configurable; verify glyph coverage and Windows rendering.
- Use the existing Forge Room icon set where possible. Keep outline weight and optical size consistent; pair icons with text when an action is ambiguous. Reserve status colors for status, not decoration.
- Prefer aligned rows, headings and 1 px separators over nested cards. Use 4/8/12/16 px spacing steps. An anchored menu or dialog may be elevated to explain focus and ownership.
- Disabled commands remain readable and explain why they are disabled. Empty, loading, offline, error, denied and reconnecting states need distinct copy and actions.

## Native UI inventory and concept coverage

| Surface | Existing source or roadmap | Detailed states to review |
| --- | --- | --- |
| Title/menu | `main.rs`: File, Edit, View; recent folders | Open File/Recent submenu, save/dirty, checked Auto Save, View toggles, shortcut alignment, keyboard menus |
| Left rail | `main.rs`: Code, Search, Rooms, Studio, Workflows, Models, Source Control, Run & Debug, Tests, Extensions, AI, Terminal, Settings, Accessibility | Active/hover/focus/badge, compact rail, overflow at short heights; AI work is paused by user direction |
| Editor | `tabs.rs`, `find_bar.rs`, `main.rs`: tabs, breadcrumbs, find/replace, palette | Preview/pinned/dirty/close, split, drag/reorder, tab overflow, file conflict dialog, find options and zero results |
| Explorer | `explorer.rs`, `file_ops.rs` | Tree expand, new file/folder, rename, copy path, refresh, delete confirmation, invalid name and existing-file errors |
| Search | `search_view.rs` | Query/replace, file scope, result groups, no matches, stale results, keyboard jump |
| Source Control | `source_control.rs`, `git.rs` | Change list, stage/unstage, diff, commit, branch, sync, stash, conflict and network failure |
| Project tools | `project_tools.rs`: Workflows, Models, Run & Debug, Tests, Extensions, Settings, Accessibility | Available vs unsupported commands, runtime detection, test results, setting controls, accessible help |
| Right dock | `main.rs`: Team, Chat, Agents, Tasks, Activity, AI | Tab focus/scroll/close, roster selector, public message, task link, task filters, evidence chooser, handoff, local/offline status |
| Bottom panel | `main.rs`: Terminal, Output, Problems | Panel resize/maximize, terminal overflow, output source, problem list, empty/error states |
| Lobby | `lobby.rs`: setup, room recovery, invite role, manage menu | Create/join/recent, privacy/mode dropdowns, sharing preview, role, permission denied, expired invite, recovery |
| Studio | `theme_gallery.rs`, `theme.rs` | Theme preview, selection, reset, contrast, keyboard focus, scale |
| Later product | `progress.md`: shipping, sessions, companion, mesh, service | Concept-only; label future capability explicitly in any mockup or prototype |

## Interaction rules for handoff

1. The selected tab, view and menu must change a real panel in the prototype. Static PNGs can show one state only.
2. Menus anchor to the control that opened them, keep one active menu at a time, close on Escape/outside click, and return focus to the trigger.
3. Destructive actions open a confirmation with the object named. A cancelled action makes no state change.
4. Do not let popovers cover the text field they are filtering. Constrain them within the window and reposition at narrow widths.
5. Panels resize with visible keyboard-focusable splitters, sane minimum widths and a Reset Layout command.
6. Show local, offline and unavailable states honestly. An image with a green dot is only a visual idea, not proof of network presence.

## Review criteria

Check 760×480 minimum window, 1240×780 default, 1440×900 and ultrawide; 100%, 150%, 200% Windows scaling; keyboard-only operation; high contrast; reduced motion; long filenames and translated labels. A generated image can contain inaccurate text or controls, so the interactive gallery and native implementation must use reviewed copy.
