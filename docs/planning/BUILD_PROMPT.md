# Paste this prompt into your VS Code coding agent

You are building **Forge Room IDE**, a desktop AI-native IDE based on the local Code OSS fork. Work in `C:\Users\Admin\Documents\Codex\2026-10-20\che\code-oss`. Read `C:\Forge-Room-IDE-Handoff\docs\product\PRODUCT_SPEC.md` and `CURRENT_STATE.md` first, then inspect the actual repository and its `AGENTS.md` files. Treat those documents as requirements/context, and verify any statement that may have become stale against the source.

## Product intent

Keep Code OSS's complete editor foundation. Add a game-like project lobby in which people and AI agents join the same team room, see each other's live work feed, communicate directly, coordinate tasks, and inspect progress without repeatedly pulling the repo or reading status Markdown files. Support human coding and agent coding equally. Local/LAN rooms come first; authenticated internet rooms are a later phase. AI generation, parallel agents, model routing, review, shared context, Git worktrees, a living project blueprint, dependency-aware notifications, and proof-carrying changes are planned product capabilities. Do not claim they exist until implemented.

## Existing work

The fork is cloned locally. `product.json` has Forge Room branding edits. `extensions/team-lobby/` contains a TypeScript lobby prototype and compiles independently. The standalone version was packaged and installed into local VS Code. The fork source edits are uncommitted. The Code OSS dependency install failed on MSB8040 because MSVC Spectre-mitigated libraries are missing; Visual Studio Build Tools requested a Windows restart. Never invent a Git author identity or say the branded app is runnable before you verify it.

## Work to do, in order

1. Inspect Code OSS's current build guide, local `.nvmrc`, package scripts, product configuration, and the extension build pipeline. Verify the right Windows commands for this checkout.
2. Resolve the native build prerequisite after the user has restarted Windows. Confirm the Spectre component is installed. Retry `npm install`, then compile and launch the branded fork. If the same blocker remains, capture the exact error and solve that prerequisite before expanding product scope.
3. Ensure `extensions/team-lobby/` is included as a built-in extension in the development and packaged app. Fix packaging, activation, or dependency issues in the fork. Verify its activity-bar view and commands in the running branded app.
4. Exercise the lobby across two editor windows, then two LAN machines if available. Verify host/join, secret rejection, presence, posting, edit/save events, disconnect, and reconnection behavior. Fix real failures at the responsible layer.
5. Implement the next thin team-flow slice: participant list, persistent activity history, task ownership/status, and direct room messages. Keep protocol and storage separate from UI. Include schema/versioning and migration if you persist events.
6. Build change visibility: associate an activity event with a file and workspace, expose a safe diff preview, and clearly distinguish uncommitted work from Git commits. Do not transmit contents or terminal output without a user action.
7. After the above is stable, plan authenticated internet rooms, AI provider adapters/Codex integration, context propagation, living blueprint, and proof-carrying changes. Use current official API documentation for these integrations.

## Acceptance for each slice

- State the user-visible behavior, implement it, compile it, and test the main flow in a running editor.
- Preserve baseline Code OSS editing, terminal, search, debugging, and Git behavior.
- Add focused tests only for meaningful protocol/storage/security behavior; do not write tests that merely mirror implementation.
- Update the fork README with exact launch steps, implemented features, limitations, and the next slice.
- Report the changed files, commands run, observed results, and remaining blockers. Label unverified behavior clearly.

Begin with the build blocker and a running branded editor. Continue autonomously through the first verified vertical slice. Ask me only when you need a decision or an action you cannot perform, such as restarting Windows or providing credentials.

