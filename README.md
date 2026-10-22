# Forge Room IDE

Forge Room is a Code OSS based developer environment for customizable workspaces and multiplayer project rooms where people and coding agents can coordinate with clear ownership, sharing boundaries, and reviewable work.

## Start here

- [Project progress and complete ideas ledger](PROJECT_PROGRESS.md)
- [Product specification](docs/product/PRODUCT_SPEC.md)
- [Build history and slice evidence](docs/planning/BUILD_STATUS.md)
- [Current state snapshot](docs/planning/CURRENT_STATE.md)
- [AI-native expansion gap audit](docs/research/FORGE_AI_NATIVE_MASTER_AUDIT.md)
- [Room and AI threat model](docs/architecture/THREAT_MODEL.md)

## Repository map

```text
docs/
  product/         Product definition and acceptance goals
  research/        Competitor, AI and repository research
  architecture/    Threat model and implementation map
  planning/         Delivery briefs, current state and historical build log
  source-prompts/   Original product briefs retained as source material
extensions/
  team-lobby/       Collaboration, room and AI workflow extension
  forge-room-studio/ Themes, profiles and appearance tools
services/
  room-service/     Self-hosted room control-plane and relay prototype
releases/           Historical VSIX package artifacts, excluded from Git
archive/            Excluded legacy Team Lobby source/test snapshot for reference
```

`PROJECT_PROGRESS.md` is the single overview of product ideas, delivery state, outstanding acceptance, and next work. The source prompts in `docs/source-prompts/` are product requirements; they are not evidence that a requirement is implemented.

## Current implementation

- Team Lobby extension: `extensions/team-lobby/` (0.108.0)
- Forge Room Studio extension: `extensions/forge-room-studio/` (0.14.0)
- Room service prototype: `services/room-service/`
- Code OSS product checkout used during integration: `C:\Users\Admin\Documents\Codex\2026-10-20\che\code-oss`

The extension packages contain their own build and test scripts. For example:

```powershell
cd extensions/team-lobby
npm.cmd run check
npm.cmd test
npm.cmd run package
```

See each extension and service README for its setup. VSIX packages and local Code OSS build output are generated artifacts, not source-controlled files.

## Ideas and requirements

The full feature inventory and phased roadmap are in [PROJECT_PROGRESS.md](PROJECT_PROGRESS.md). Detailed documents are grouped under `docs/`; see the [AI-native research audit](docs/research/FORGE_AI_NATIVE_MASTER_AUDIT.md), [competitor feature research](docs/research/COMPETITOR_FEATURE_RESEARCH.md), and [AI feature research](docs/research/AI_CORE_FEATURE_RESEARCH.md).
