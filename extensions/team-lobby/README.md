# Team Lobby IDE

Forge Room Team Lobby adds multiplayer project rooms and scoped AI workspace capabilities to Code OSS.

Room participants can record an attributed decision with a title and short rationale. The extension asks before sharing it publicly with the room. Decisions are retained with room history and can be found through local search. Room protocol 20 is required for this event type.

## AI workspace capabilities

Ask, Plan, and Build can use typed tools for selected-workspace file inventory, text search, named-file reads, Git status and per-file diffs, diagnostics, symbol search, test task discovery, debugger frames, and explicitly approved terminal output. With separate per-call confirmation, `editor_open` opens a non-sensitive project file and optionally reveals a requested line without sending file contents. Build can run an exact registered test task and apply a reviewed exact-match file edit. Direct edits save a rollback checkpoint before application.

The file inventory returns at most 250 workspace-relative paths and no file contents. Sensitive paths and common generated folders are excluded. Room task context is optional and requires an exact preview confirmation; the snapshot is capped at 50 tasks and 12,000 characters and excludes chat, notes, diffs, and file contents. Workspace tools are limited to the selected project root, bounded in output, and recorded as content-free local evidence. Mutating actions and sensitive context use separate approvals.

The **Team Lobby: Triage Room Tasks** command provides a local recommendation-only queue ranked by blockers, deadlines, critical path, priority, in-progress status, and downstream tasks. Optional whole-minute effort estimates are capped at seven days and can be set during task creation or changed from task actions. The estimated critical path is the longest unfinished dependency chain; it stays unknown if an unfinished task lacks an estimate or has an unavailable dependency. Selecting a task opens its existing task actions; it never changes task state or starts an agent by itself.

Optional shared hackathon countdown reminders are available through the `teamLobby.countdownAlerts` setting. It defaults off. When enabled, each IDE shows local reminders at 60, 30, 10, and 5 minutes and at the deadline; reminders are not broadcast or written to room history.

Optional room task deadline reminders use the `teamLobby.taskDeadlineAlerts` setting. It defaults off. When enabled, an assigned task owner receives local reminders 15 minutes and 5 minutes before an unfinished task deadline, plus one at expiry. Reminders are deduplicated for each task deadline, stay on-device, and are not written to room history.

The **Team Lobby: Export Room Activity Summary** command saves a local Markdown snapshot of task counts, up to 50 task titles/owners/statuses/priorities, plus up to 30 recent event times, participant names, and whitelisted event categories. It omits event details, direct messages, message/note text, file paths, code, and patches, and never sends the summary to participants.

Connected room participants can search up to 5,000 messages retained on their device, in pages of 25. Search queries stay local. Direct messages can appear only in the sender's or recipient's local archive. A newly joined participant starts with the room's recent event history; messages are added while connected. This is bounded local history, not a full-room transcript service.

The **Team Lobby: Share Verification Result** command lets a room participant record a test, build, lint, type-check, or custom check as passed, failed, or stopped. The result can link to an open task. Forge asks for confirmation and sends only the check name, outcome, and optional task link; it does not run the command or independently verify the result. The activity is labelled as manually reported.

The **Team Lobby: Prepare Demo Readiness Checklist** command saves a local Markdown report of unfinished or blocked room tasks, overdue task deadlines, owner-attested acceptance criteria, recent recorded verification outcomes, the shared countdown, and up to 20 room milestones. It flags paused or expired clocks, deadlines within one hour, and passed or near milestones; no configured countdown or milestones stay unknown. When all unfinished tasks have valid estimates and dependencies, it compares the longest estimated dependency chain with the active countdown: an overrun needs attention, and a fit is recorded as ready. Paused or unset countdowns and incomplete estimates remain unknown. Participant-reported results do not count as IDE-recorded verification. A manual report newer than the last IDE task result leaves that readiness gate unknown until the task is run in the IDE again. Preview, deployment, and submission readiness stay marked as unknown/manual until those systems have real integrations. The report is saved only to the destination you choose and is not broadcast.

The room host can manage up to 20 shared milestones from the countdown controls. Enter one per line as `Label | 2026-10-25T18:00+05:30`; timestamps require an explicit timezone and future time. Participants see milestone timing and urgency in the room. The readiness report also compares each upcoming milestone with the estimated remaining dependency chain: milestones that arrive before the work estimate need attention. Missing estimates or dependencies leave feasibility unknown. Milestones sync with the host and persist in local room history.

Room tasks support **Low**, **Normal**, **High**, and **Critical** priority. Set priority when creating a task or from its task actions. Priority appears on task cards, helps order equally urgent recommendations, and is included in the opt-in task snapshot only after its explicit provider-transfer confirmation. Older tasks without priority are treated as Normal. Estimates also appear in the room task snapshot only after the same explicit confirmation.

## Collaboration

Team Lobby supports room identity and presence, invites and revocation, tasks and handoffs, room chat and notes with toggleable reactions, searchable recent room activity, anchored comments, selected Git diff sharing, consented live document sessions, opt-in file-and-line follow, deadlines, activity history export, and optional self-hosted room-service relay. Room pulse search filters up to 500 retained events locally and applies direct-message visibility rules before matching. Messages can optionally link to a room task; the shared card names that task and clicking it opens task actions. Direct messages and their reactions remain visible only to their sender and recipient.

The room relay does not persist active room content. ACP and MCP local subprocesses run with the user's operating-system privileges; selecting them does not sandbox their access.

## Development

From this directory, run `npm.cmd run check` and `npm.cmd test`. The extension's automated suite verifies policy, formatter, and process-level room behavior; it does not replace live provider, separate-IDE, or multi-machine acceptance.

### Message threads

Select **Thread** under a room message to view its replies or add a reply. Public message replies stay public. Replies to direct messages remain visible only to the two original participants.
