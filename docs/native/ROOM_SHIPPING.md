# Network Room Ship workflow contract

The host defines a bounded release plan with required shared task IDs, assigned manual checklist items, a preview reference and a submission reference. All fields are public to current Room members. References are text only: no network fetch, deployment or credential discovery occurs.

Plan and check updates carry an expected global revision. The server rejects stale edits, non-host plan changes, foreign-owner check reports, invalid task/member references, and updates while paused/archived. Replacing a plan resets check reports so old evidence cannot certify changed scope. A checklist owner may report unknown, claimed pass or claimed fail with a bounded public note. These are human declarations, not verified CI results.

Ship review derives blockers from required unfinished/overdue tasks and dependencies, all unfinished Critical tasks, task readiness blockers, unresolved/disputed impacts (including acknowledgements from previous owners), missing/offline check owners, missing/failed reports and missing preview/submission references. Focus mode changes presentation only. It never completes tasks, starts an agent or expands execution permissions.

The handoff export is a local explicit copy of a public-field projection: plan, task IDs/titles/status/owners/dependencies, manual reports and derived blockers. It excludes chat/DM events, private patch bodies, transport tokens, invites and local files. Public text still contains whatever members explicitly entered, so review it before external sharing. The export is bounded by the shared model limits and labelled manual; it is not a full Room archive or backup.

Source/unit and independent-process tests establish policy and visibility. Visible desktop, physical two-machine and live-provider acceptance are separate gates.
