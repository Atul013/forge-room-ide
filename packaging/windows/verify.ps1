param([string]$Version = "0.109.0-alpha.1")

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$artifacts = Join-Path $root "artifacts\windows-x64"
$installer = Join-Path $artifacts "Forge-Room-Early-Preview-$Version-windows-x64-setup.exe"
$installDir = Join-Path $env:LOCALAPPDATA "Programs\Forge Room Early Preview"
$sample = Join-Path $env:TEMP "forge-room-preview-smoke.txt"
Set-Content $sample "Forge Room Early Preview smoke file" -Encoding utf8NoBOM

$install = Start-Process $installer -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART" -Wait -PassThru
if ($install.ExitCode -ne 0) { throw "Silent installation failed with exit code $($install.ExitCode)" }
$installedExe = Join-Path $installDir "forge-room-early-preview.exe"
if (-not (Test-Path $installedExe)) { throw "Installed executable is missing" }
$startupLog = Join-Path $env:LOCALAPPDATA "Forge Room Early Preview\logs\startup-error.log"
if (Test-Path $startupLog) { Remove-Item $startupLog -Force }

$clock = [System.Diagnostics.Stopwatch]::StartNew()
$process = Start-Process $installedExe -ArgumentList ('"{0}"' -f $sample) -PassThru
Start-Sleep -Seconds 5
$clock.Stop()
$alive = -not $process.HasExited
$exitCode = if ($process.HasExited) { $process.ExitCode } else { $null }
$workingSet = if ($alive) {
    $process.Refresh()
    $process.WorkingSet64
} else {
    $null
}
$startupErrorCaptured = Test-Path $startupLog
if ($startupErrorCaptured) {
    Copy-Item $startupLog (Join-Path $artifacts "STARTUP_ERROR.log") -Force
}
$launch = [ordered]@{
    processAliveAfterMilliseconds = $clock.ElapsedMilliseconds
    launchProbePassed = $alive
    exitCodeIfExited = $exitCode
    workingSetBytesAfterFiveSeconds = $workingSet
    startupErrorLogCaptured = $startupErrorCaptured
    interactiveUiVerified = $false
    environment = "github-hosted-windows-2025-noninteractive"
}
$launch | ConvertTo-Json | Set-Content (Join-Path $artifacts "LAUNCH_SMOKE.json") -Encoding utf8NoBOM
if ($alive) { Stop-Process -Id $process.Id -Force }

$uninstaller = Join-Path $installDir "unins000.exe"
if (-not (Test-Path $uninstaller)) { throw "Uninstaller is missing" }
$uninstall = Start-Process $uninstaller -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART" -Wait -PassThru
if ($uninstall.ExitCode -ne 0) { throw "Silent uninstall failed with exit code $($uninstall.ExitCode)" }
Start-Sleep -Seconds 2
if (Test-Path $installedExe) { throw "Application executable remained after uninstall" }

Get-Content (Join-Path $artifacts "LAUNCH_SMOKE.json")
if ($startupErrorCaptured) {
    Write-Host "Captured native startup diagnostic:"
    Get-Content (Join-Path $artifacts "STARTUP_ERROR.log")
}
