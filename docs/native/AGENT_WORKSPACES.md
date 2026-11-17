# Isolated agent workspaces

The AI panel can create a separate Git branch/worktree for a Direct-agent task. Creation is
explicit, requires a trusted Git folder and runs in a worker. It starts from HEAD: primary
uncommitted disk changes and unsaved editor buffers are not copied into the checkout.
If the open project is a tracked subfolder, the task uses the corresponding subfolder in the
new checkout. Source Git hooks/filters may run during checkout under the existing trust choice.

## Use

1. In AI, expand Isolated agent workspaces, enter a task name and create the workspace.
2. Select the resulting workspace and enable Run Direct agent in selected isolated workspace.
3. Send a Direct-agent message. The installed CLI starts with this task folder as cwd and the
   selected CLI permission mode. Its Claude/Codex conversation IDs are separate from the primary
   folder conversation and from other task folders. Task conversation IDs remain in memory;
   primary chat continuation/persistence remains unchanged. New Chat clears the selected task's
   conversation without deleting its files or branch. Creation must finish before agent launch.
4. Summarize isolated changes shows Git status and a bounded diff stat, including untracked names.
   Open isolated folder in another window preserves the current window/buffers and lets the user
   inspect full per-file diffs, run tests and decide how to integrate. The new window uses the
   normal folder trust flow. Interactive Direct agent commands also enter the validated task folder
   before starting the CLI; a failed directory change does not fall back to the primary folder.

Ask/Plan/Reviewed edits remain scoped to their original folder and do not implicitly target a task
worktree. Attached file data still comes from the explicitly attached original saved files.
Git worktree separation is not an OS security sandbox: installed CLI/provider permissions and the
user's OS rights remain authoritative. There is no implicit agent commit, merge, push or cleanup.

## Ownership and retention

Folders live under the app's LOCALAPPDATA agent-workspaces directory with fresh per-task branch
names. Before execution/open/review, validation checks the source repository, owned target path,
links/reparse points, Git worktree registration/backlink, common directory and selected branch.
Windows Git arguments omit verbatim path prefixes; canonical paths remain authoritative for checks.
Primary files are never stashed, reset or copied by this feature. Agent Stop uses the existing owned
process cancellation and retains workspace files/branch. Workspace records persist locally (up to
32 per source folder); enabled/selected execution state is not automatically restored on restart.

Worktrees/branches remain until the user removes them through Git. If the folder/app is closed
while Git checkout is still running, Git may finish its registration; inspect `git worktree list`
to recover an interrupted creation that did not reach the UI record save. Automatic cleanup,
committing/integration, durable edit checkpoints and resource-budget enforcement are future work.

## Evidence and open acceptance

Real disposable Git repository tests cover dirty primary preservation, independent task edits,
registration validation, persisted records, nonpersisted conversation IDs and tracked-subfolder
mapping. A Windows stand-in CLI test verifies the actual task cwd, changed task file, untouched
primary dirty file and separate primary/task conversation IDs. New Chat leaves task files intact.
Native window opening, desktop controls and live provider behavior remain unverified; there is
no claim that test stand-ins establish real provider, desktop or two-machine acceptance.
