# Shared Room planning workspace

One public Planning tab connects a dependency-aware task board, reviewed batch plans,
task definition editing, reusable templates and starter workflows. It does not execute
commands, edit files, transfer existing ownership or mark work complete.

## Board and drafting

The board groups tasks by Open, In progress and Done. Search matches ID/title/owner;
owner and blocked-only filters combine. Cards show unfinished prerequisites, owner,
priority and deadline state, and open the existing task/checklist workflow.

The host can add task definitions, bring individual existing tasks or the complete
Room task set into a draft, and insert feature or defect workflows with dependency
chains. Draft rows edit title, criteria, dependencies, priority and explicit deadline
changes. New rows choose joined owners individually or through an explicit bulk
assignment. Existing ownership and status remain protected by their usual workflows.

Keys are unique identifiers within a plan. Dependencies can refer forward to another
plan key, or to an existing Room task ID. The server allocates stable task IDs and
validates the complete resulting graph before committing anything. Limits remain
100 Room tasks, eight criteria and sixteen prerequisites per task.

## Review and apply

Review validates a complete candidate and freezes the expected task snapshot and
draft. The preview shows new/changed definitions, previous criteria/dependencies and
explicit deadline intent. Changes to the draft or Room tasks invalidate review. The
server independently compares the expected task snapshot at Apply; stale, malformed,
cyclic, unauthorized or partially invalid batches change nothing.

Only host-local authority applies a plan. New tasks start Open; existing owner/status
are preserved. Completed tasks must be reopened before edits. A changed task with a
pending handoff is refused, completed checklist items cannot be silently removed,
and started/completed work cannot gain unfinished prerequisites. No tasks are deleted.

Changed existing tasks reset readiness declarations to Unknown at a newer revision,
so late requests using the previous revision are denied. Their help requests clear;
affected impact acknowledgements clear. All release check reports reset and the
configured Ship revision advances. Historical evidence remains historical; plans
never certify checks, providers or deployment. Drafts remain available after Apply
or rejection. Applying an unchanged plan is refused as a no-op.

## Files and reuse

Import loads a draft only, with no server action. Versioned JSON is bounded to 128 KiB,
rejects unknown fields, and requires 1–100 rows. Plan saves are create-only and flushed,
so existing files remain untouched. A saved plan retains explicit edit references.

Reusable template export instead creates new-task definitions, remaps internal graph
keys and clears all owner identities. Prerequisites outside the template become
`external-...` placeholders and must be explicitly remapped before review. This
prevents coincidental task IDs in another Room from silently satisfying external
dependencies. Templates carry no statuses, evidence, DMs, patch bodies, files or keys.

## Compatibility and evidence

The additive `planning_supported` capability defaults to false for older hosts.
Current hosts advertise it; unsupported hosts do not offer Apply. Existing direct
task creation, completion, handoff, evidence, readiness and Ship controls remain.
Plans use the normal authenticated lifecycle and journal path: an enabled journal
flushes the entire public plan result before host success is returned. Checkpoint
and journal recovery retain the task batch and its dependency graph.

Core tests cover forward references, atomic rejection, cycles, owner protections,
staleness and readiness/release invalidation. Independent clients verify the public
batch, non-host denial and journal restart. App tests cover template remapping,
explicit owner choices, bounded import, create-only save and starter chains.
Visible desktop, physical two-machine and native-provider acceptance remain separate.
