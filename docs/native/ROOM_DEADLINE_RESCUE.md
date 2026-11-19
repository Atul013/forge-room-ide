# Deadline Rescue and Ship Triage

One native Room workflow connects release targets, transitive prerequisites, owner workload, deadline risk, explicit help controls, frozen public comparisons and host-reviewed release scope. Open **Network Room → Rescue** beside Planning and Ship.

## Review a scenario

Choose targets manually, from published Ship scope or from unfinished critical tasks. Reviewing freezes the host-time observation, displayed countdown and current public tasks/readiness/Ship source. All prerequisites, including completed prerequisites, are included exactly once. Empty targets, missing tasks, duplicate targets and dependency cycles fail closed. Deferred work stays in the Room. Unfinished critical tasks remain Ship blockers even when deferred.

Required critical work sorts first, followed by required work ordered by priority and warnings, then deferred work. Search task/owner names or show required work only. Each row retains its owner/status, dependency critical-path estimate, deadline budget, warning/unknown counts and consent summary. The current-owner readiness estimate contributes to a per-owner sum once per unfinished required task; missing or prior-owner estimates stay unknown. Known workload above a running countdown adds a blocker. Critical paths assume independent branches can run in parallel. Neither estimate is a resource schedule or completion guarantee.

Help is recommendation only. Current-owner opt-in, an open current-owner request and permission to execute are separate things. A prior owner's profile/request cannot grant help for a new owner. Open the existing readiness/help controls from a row for an explicit owner request, opt-out, impact review or task brief. This bundle adds no agent execution, ownership transfer, auto-completion or deployment.

## Frozen reports and comparison

Inspect the exact public JSON before copying or saving. Save uses create-only writes and reports an error for an existing filename; partial write failures are not claimed as successful. The snapshot remains frozen while time and the Room continue changing. Keep a report as a local in-memory baseline, or save it for a later session and explicitly import it as comparison data. Clear a baseline deliberately.

The strict version-1 portable JSON is capped at 512 KiB. Unknown fields, controls, oversized strings/inventories, duplicate or unknown task references, inconsistent dependency closure, cycles and inconsistent owner totals are rejected. File imports reject linked/non-regular files and bound reads. Imported reports are unverified claims, including their source digest and host timestamp; import never changes the current targets, frozen authoritative review or publish permission. Different stable Room identifiers cannot be compared, even when their display names match.

The explicit projection includes public task/owner identifiers, titles, status/priority, dependencies, estimates, warnings/unknowns, assistance summaries, per-owner totals and release blockers. It excludes chat/DMs, private patches, local source text, worktree branches, file contents, local histories and authentication material. It can include public text entered by members; it is not a content-classification or secret-redaction guarantee. A digest binds source bytes for stale review; it is not a signature or proof of independent verification.

## Host-reviewed scope change

A host with an active Room and an already-published Ship plan may acknowledge the reset of all existing manual check reports and explicitly publish the reviewed scenario. Scope review expires locally after two minutes. Changing targets clears the acknowledgement. A different Room clears the frozen review and acknowledgement; a saved comparison baseline grants no authority.

`SetRescueScope` is host-local authority only. Under the existing host lock, core rechecks a digest of the stable Room identifier, exact task records, readiness/help/impact state and complete Ship state. It resolves prerequisites again, rejects unchanged scope, then reuses Ship plan validation and revision guards. Only required tasks change. Title, assigned checks and preview/submission references are preserved. Check reports reset, activity is attributed, and the public journal persists the resulting state using existing acknowledgement rules. Task owners, statuses, criteria and task records remain untouched. A new manual check report, readiness/consent update, task change or Ship edit invalidates an earlier review. Countdown/presence sampling is deliberately outside the mutation digest: report clock values stay explicitly frozen observations and no scheduling action is authorized.

Scenario blockers preview the same check-report reset that publication performs. Removed required work does not make unrelated critical tasks, unresolved impacts or unknown manual gates pass. Host pause/archive rejects scope mutation. Guest/member requests are denied server-side, regardless of UI. Older hosts do not understand the additive action and may reject it; existing protocol-v3 messages and saved snapshots are unchanged. No automatic downgrade to an unguarded scope update occurs.

## Evidence and acceptance

Focused policy tests cover closure, missing/cyclic graphs, unknown estimates, consent ownership, deferred critical blockers, shared-prerequisite workload, report validation/projection/comparison, distinct Room identities, stale review and TLS host authorization. Native frame tests cover normal/narrow rendering without sending actions, expiry/acknowledgement, active-host requirements, changed source and Room switching. An independent-process test covers guest denial, pause refusal, public scope propagation, report reset, private-message/patch exclusion and journal recovery while preserving all tasks. These are automated source/process checks.

Visible desktop, interactive installer, real-provider and physical two-client acceptance remain open. CI, frame rendering, loopback processes and package verification do not close any of them.

Manual desktop script:

1. Host a disposable Room, join an owner and publish Ship scope with a manually reported smoke check. Give two required tasks to the same owner and share a prerequisite.
2. Publish current-owner estimates and try opted-out, opted-in and open-request help states. Inspect warnings and owner totals in Rescue at ordinary and narrow widths; confirm no agent starts.
3. Select a smaller target set. Verify the prerequisite is still required, deferred critical work still blocks Ship, and smoke reports will need renewal. Inspect/copy/save the frozen public JSON; import it only as comparison data.
4. Change a task/readiness/check report from another client while host review is open. Publication must be refused until refreshed. Pause the Room and verify scope publication remains unavailable.
5. Refresh, acknowledge the reset and publish. Confirm scope and empty check reports on both clients; task owners/statuses remain unchanged. Restore from a fresh journal through the existing Recovery workflow and inspect the same public scope.
6. Independently record desktop, provider and physical-device observations in Release Evidence Desk. Leave any unperformed gate unknown.
