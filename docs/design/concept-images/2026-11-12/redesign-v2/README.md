# Forge Room redesign concepts — second set

Eight **newly generated** high-resolution desktop UX concepts for the requested product areas. Open the [visual gallery](gallery.html) to review them together. These PNGs are design references; they are not screenshots of implemented native behavior. The [audit](../../UI-REDESIGN-AUDIT-2026-11-12.md) explains the source baseline and priorities.

| Image | Workflow | Implementation boundary |
| --- | --- | --- |
| [01 Workspace](01-workspace.png) | Editor, Explorer, terminal, local Room tasks | Navigation rail changed in PR #34; image is a concept, not a UI capture. |
| [02 Lobby](02-lobby.png) | Recent projects and local room setup | Network invitations are unavailable. |
| [03 Collaboration](03-collaboration.png) | Local Chat, task link, decision beside editor | Chat is local; shown people and conversation are illustrative. |
| [04 Studio](04-studio.png) | Theme selection and editor preview | Theme selection exists; Morph, layout profiles and widgets are future concepts. |
| [05 Workshop](05-workshop.png) | Widget authoring and permission review | Entire screen is a future concept. SDK, isolation and install behavior are proposals. |
| [06 Packs](06-packs.png) | Pack inspection and permission review | Entire screen is a future concept. Pack names, sources, versions and compatibility are invented examples. |
| [07 Ship Mode](07-ship.png) | Evidence-led release readiness | Entire screen is a future concept; displayed counts are invented examples. |
| [08 Onboarding](08-onboarding.png) | Open folder, trust, recovery, optional local Room | Some actions exist, but this composition is not implemented. |

## Visual direction

Use a calm, flat graphite workbench, warm off-white type, a single muted ember accent, compact controls and 1 px separators. The editor gets the largest area. The 56 px rail is an anchor, not a second text sidebar. Empty or unavailable capabilities must state their status plainly. The Studio image was regenerated to remove a light-theme mismatch; onboarding was regenerated to remove unsupported logging and approval claims. The rejected first variants are not in this selected set.

## Design QA before implementation

Generated images can contain inaccurate text, invented paths, inconsistent branding or impossible code. In particular, the Workshop sample SDK and Packs metadata are illustrative. Review every label and control against Rust source before implementing. Validate at 760×480, 1240×780 and 1440×900, with keyboard navigation, visible focus, and readable 100–200% Windows scaling. These images do not satisfy that native desktop acceptance gate.

Generated with the built-in image-generation tool using a separate screen-specific prompt for each image. The prompts specified a realistic native IDE, the eight workflows above, flat graphite surfaces, compact typography, and explicit local or future capability labels. Studio and onboarding received targeted second prompts to correct the issues noted above.
