param([switch]$InspectOnly)
$ErrorActionPreference = 'Stop'
$repo = 'E:\Projects\forge-room-ide'
$mailbox = Join-Path $repo 'CODEX_BRIDGE.md'
$stopFile = Join-Path $repo 'design-assets\forge-room-ui-spec\measurements\mailbox-polling.stop'
if (Test-Path -LiteralPath $stopFile) { exit 0 }
$text = [IO.File]::ReadAllText($mailbox)
$section = ''
$openTasks = @()
foreach ($line in ($text -split '\r?\n')) {
    if ($line -match '^##\s+(.+)$') { $section = $Matches[1] }
    if ($section -match '^Tasks for Codex\b' -and $line -match '^\s*-\s+\[ \]\s+') {
        $openTasks += $line
    }
}
if ($InspectOnly) {
    [pscustomobject]@{ Mailbox = $mailbox; OpenTasks = $openTasks; Count = $openTasks.Count } | ConvertTo-Json -Depth 3
    exit 0
}
if ($openTasks.Count -eq 0) { exit 0 }
$codex = 'C:\Users\Icarus\.vscode\extensions\openai.chatgpt-26.1021.22027-win32-x64\bin\windows-x86_64\codex.exe'
if (!(Test-Path -LiteralPath $codex)) {
    $codex = (Get-Command codex.exe -ErrorAction Stop).Source
}
$message = @'
Scheduled mailbox check authorized by Atul: re-read ALL of E:\Projects\forge-room-ide\CODEX_BRIDGE.md now. Act only on currently open [ ] tasks beneath any Tasks for Codex heading. Claim each by editing its line to [~] with Codex, then complete it and mark [x] with a one-line result and paths. Do not duplicate already claimed work, edit Claude's log entries, or invent tasks. Read COLLAB.md and progress.md; stay in Phase 1 and obey ownership. Codex owns design-assets/* and its artifacts ZIPs; no Rust or packaging edits, commits, or pushes unless that specific mailbox task explicitly authorizes them. Use honest pass/fail/not-tested evidence. Append mailbox Log entries at the bottom. If nothing is open, do nothing and remain quiet. Stop the recurring Windows task ForgeRoom-Codex-Mailbox-2min when Atul tells you to stop. This trigger is a poll, not permission to bypass sandbox or approval controls.
'@
& $codex queue --thread '01a1a16e-21f8-7cf1-b230-9762334348f2' --message $message
if ($LASTEXITCODE -ne 0) { throw "Codex queue failed with exit code $LASTEXITCODE" }
