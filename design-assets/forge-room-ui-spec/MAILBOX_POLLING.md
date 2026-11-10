# Recurring Codex mailbox check

Created at Atul's explicit request on 3 November 2026.

- Windows Task Scheduler task: `ForgeRoom-Codex-Mailbox-2min`.
- Interval: every two minutes, indefinitely; per-user interactive session, limited privileges, hidden PowerShell window. Requires the computer awake and Icarus signed in.
- Script: `tools/poll_codex_bridge.ps1`.
- Reads `E:\Projects\forge-room-ide\CODEX_BRIDGE.md` every run. If no `[ ]` line exists within a `Tasks for Codex` section, exits without writing or queuing anything. Already claimed `[~]` tasks are skipped.
- If open tasks exist, invokes the installed `codex queue` CLI to queue a bounded mailbox check into existing thread `01a1a16e-21f8-7cf1-b230-9762334348f2`. The worker rereads current state before claiming anything. No parallel agent, automatic commits, or approval bypass flags are configured.
- Overlapping scheduler invocations are ignored. A successful queue means accepted by Codex, not proof that the mailbox task has completed. Completion remains recorded in the mailbox.
- Registration XML: `measurements/mailbox-scheduled-task.xml`.
- Verification: manual idle run returned `LastTaskResult=0`; next run was scheduled two minutes later. A CLI verification message was accepted for this thread.

When Atul says to stop, disable the task:

```powershell
Disable-ScheduledTask -TaskName 'ForgeRoom-Codex-Mailbox-2min'
```

For an immediate file-based stop, create `measurements/mailbox-polling.stop`; every invocation checks this before reading/queuing work. Disabling the schedule does not cancel an already active task; handle any active task according to Atul's stop instruction.
