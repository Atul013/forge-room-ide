param([string]$Version = "0.109.0-alpha.1")

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$artifacts = Join-Path $root "artifacts\windows-x64"
$installer = Join-Path $artifacts "Forge-Room-Early-Preview-$Version-windows-x64-setup.exe"
$installDir = Join-Path $env:LOCALAPPDATA "Programs\Forge Room Early Preview"
$sample = Join-Path $env:TEMP "forge-room-preview-smoke.txt"
Set-Content $sample "Forge Room Early Preview smoke file" -Encoding utf8NoBOM

& $installer /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
if ($LASTEXITCODE -ne 0) { throw "Silent installation failed with exit code $LASTEXITCODE" }
$installedExe = Join-Path $installDir "forge-room-early-preview.exe"
if (-not (Test-Path $installedExe)) { throw "Installed executable is missing" }

$clock = [System.Diagnostics.Stopwatch]::StartNew()
$process = Start-Process $installedExe -ArgumentList ('"{0}"' -f $sample) -PassThru
Start-Sleep -Seconds 5
if ($process.HasExited) { throw "Installed application exited during the launch smoke check" }
$process.Refresh()
$clock.Stop()
$launch = [ordered]@{
    processAliveAfterMilliseconds = $clock.ElapsedMilliseconds
    workingSetBytesAfterFiveSeconds = $process.WorkingSet64
    interactiveUiVerified = $false
}
$launch | ConvertTo-Json | Set-Content (Join-Path $artifacts "LAUNCH_SMOKE.json") -Encoding utf8NoBOM
Stop-Process -Id $process.Id -Force

$uninstaller = Join-Path $installDir "unins000.exe"
if (-not (Test-Path $uninstaller)) { throw "Uninstaller is missing" }
& $uninstaller /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
if ($LASTEXITCODE -ne 0) { throw "Silent uninstall failed with exit code $LASTEXITCODE" }
Start-Sleep -Seconds 2
if (Test-Path $installedExe) { throw "Application executable remained after uninstall" }

Get-Content (Join-Path $artifacts "LAUNCH_SMOKE.json")
