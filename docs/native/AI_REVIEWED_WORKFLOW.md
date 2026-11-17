# Native AI planning and reviewed file changes

The AI panel offers Direct agent, Ask, Plan and Reviewed edits. Direct agent retains the
existing installed-CLI permission modes and conversation continuation. Ask/Plan/Reviewed
edits start a fresh CLI conversation using its read-only mode, even if Direct agent was
configured with Full access. The CLI remains responsible for its own tool/sandbox behavior;
Forge Room does not claim an OS execution sandbox or that attachments restrict CLI reads.

## User journey

1. Open and trust a folder, choose an installed provider/model and the workflow.
2. Explicitly attach saved UTF-8 files from the open folder. Active-file attachment reads
   saved disk content, not unsaved editor text. Inspect attachment names/sizes; remove any
   unwanted file. At most eight files of 64 KiB each, 256 KiB total, enter the prompt.
3. Choose Plan and send the goal. The completed response becomes an editable plan. Plans
   persist locally with chat history, older histories default to an empty plan, and New Chat
   clears it. Export Plan creates a new file and refuses to replace an existing filename.
4. Edit the plan and select Generate reviewed edits from this plan. The provider receives
   the plan and fresh snapshots of the selected files, with instructions to return exact
   file replacements. This generation uses a fresh read-only conversation and does not
   reuse or overwrite the Direct agent conversation ID.
5. Inspect complete Before/After versions of every proposed file. Apply reviewed edits is
   a separate explicit action. A completed model response alone never applies a proposal.
6. Undo last applied proposal is available for this chat while touched files remain unchanged.
   Later edits to those files block undo. Unrelated files and dirty editor buffers are preserved.
7. Recovery checkpoints now retain pre-apply originals across New Chat/app restart. Load one,
   review it and explicitly restore only matching or missing targets. See RECOVERY_CHECKPOINTS.md.

## File boundary and recovery

Proposals can replace only the explicitly attached existing files. New files, renames,
deletions and commands are unsupported. The parser accepts a strict object or one JSON
code fence, checks duplicate and outside paths, and bounds each result to 128 KiB/1 MiB total.
Malformed/oversized responses do not write files. Before generation the app captures exact
file baselines; changes during drafting, after review, or before undo cause rejection.

Linked child paths, directory links/reparse points, alternate streams, .git/.ssh, .env files,
common private-key filenames/extensions, binary files and outside-folder paths are refused.
Context exclusion is not a complete secret scanner. Attached source can still contain secrets;
users must inspect what they attach. The installed CLI may read other workspace files under
its own permissions. Context snapshots/proposals/undo remain in memory for this chat; the
plan and bounded transcript are saved locally. Generated answers may contain source text.

Apply/undo reuses the existing staged multi-file edit engine with fresh workspace/path and
dirty-buffer checks. Originals are backed up during replacement; failed writes attempt
rollback. Rollback preserves files changed after application and reports remaining .bak
files when restoration fails. Undo uses exact inverse replacements, not git reset or stash.
Immediate undo remains session-scoped; durable reviewed-change recovery is described in
RECOVERY_CHECKPOINTS.md. Direct-agent Git isolation is described in AGENT_WORKSPACES.md.

## Evidence and open gates

Tests cover unattached targets, changed baselines, dirty buffers, exact apply, later-edit
preservation during undo, restored originals, old-history loading and plan persistence.
A Windows stand-in CLI process emits a real proposal through the installed-agent protocol:
the test verifies read-only arguments, a fresh session, no write before approval, and apply/undo.
This is not live Codex/Claude/Ollama/LM Studio inference acceptance. A Windows locked-second-
file test checks rollback of the first file after a partial multi-file failure.

Visible desktop, provider authentication/inference, accessibility and two-language/project
acceptance remain open. Autonomous Build, native tool approvals, test execution/evidence,
Direct-agent per-action checkpoints, resource budgets and provider-neutral native adapters
remain future work. Git task workspaces and reviewed-change checkpoints are now separate bounded features.
