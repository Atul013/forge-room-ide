param([string]$Version = "0.109.0-alpha.1")

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$artifacts = Join-Path $root "artifacts\windows-x64"
$installer = Join-Path $artifacts "Forge-Room-Early-Preview-$Version-windows-x64-setup.exe"
$package = Join-Path $artifacts "Forge-Room-Early-Preview-$Version-windows-x64"
$installDir = Join-Path $env:LOCALAPPDATA "Programs\Forge Room Early Preview"
$dataDir = Join-Path $env:LOCALAPPDATA "Forge Room Early Preview"
$sample = Join-Path $env:TEMP "forge-room-preview-smoke.txt"
$preservedProject = Join-Path $env:TEMP "forge-room-preview-preserved-project.txt"
$settingsMarker = Join-Path $dataDir "upgrade-preservation-smoke.txt"
$startMenuShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Forge Room Early Preview\Forge Room Early Preview.lnk"
$desktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "Forge Room Early Preview.lnk"
Set-Content $sample "Forge Room Early Preview smoke file" -Encoding utf8NoBOM
Set-Content $preservedProject "Forge Room external project preservation marker" -Encoding utf8NoBOM

$isccCandidates = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
)
$iscc = $isccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 compiler is not installed" }

$baselineVersion = "0.109.0-alpha.0"
$baselineDir = Join-Path $artifacts "upgrade-baseline"
New-Item $baselineDir -ItemType Directory -Force | Out-Null
$iss = Join-Path $PSScriptRoot "forge-room-early-preview.iss"
& $iscc "/DPackageDir=$package" "/DOutputDir=$baselineDir" "/DAppVersion=$baselineVersion" $iss
if ($LASTEXITCODE -ne 0) { throw "Baseline installer compilation failed with exit code $LASTEXITCODE" }
$baselineInstaller = Join-Path $baselineDir "Forge-Room-Early-Preview-$baselineVersion-windows-x64-setup.exe"
if (-not (Test-Path $baselineInstaller)) { throw "Baseline installer is missing" }

$baselineInstall = Start-Process $baselineInstaller -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART" -Wait -PassThru
if ($baselineInstall.ExitCode -ne 0) { throw "Baseline installation failed with exit code $($baselineInstall.ExitCode)" }
$installedExe = Join-Path $installDir "forge-room-early-preview.exe"
if (-not (Test-Path $installedExe)) { throw "Baseline executable is missing" }
New-Item $dataDir -ItemType Directory -Force | Out-Null
Set-Content $settingsMarker "Forge Room settings preservation marker" -Encoding utf8NoBOM

$install = Start-Process $installer -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART" -Wait -PassThru
if ($install.ExitCode -ne 0) { throw "Silent upgrade failed with exit code $($install.ExitCode)" }
if (-not (Test-Path $installedExe)) { throw "Installed executable is missing" }
if (-not (Test-Path $settingsMarker)) { throw "Application data marker was not preserved during upgrade" }
if ((Get-Content $settingsMarker -Raw).Trim() -ne "Forge Room settings preservation marker") { throw "Application data marker changed during upgrade" }
if (-not (Test-Path $preservedProject)) { throw "External project file was not preserved during upgrade" }
if ((Get-Content $preservedProject -Raw).Trim() -ne "Forge Room external project preservation marker") { throw "External project file changed during upgrade" }
if (-not (Test-Path $startMenuShortcut)) { throw "Default Start Menu shortcut is missing" }
if (Test-Path $desktopShortcut) { throw "Desktop shortcut was created despite being opt-in" }
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
if (Test-Path $startMenuShortcut) { throw "Start Menu shortcut remained after uninstall" }
if (-not (Test-Path $settingsMarker)) { throw "Application data marker was removed by uninstall" }
if (-not (Test-Path $preservedProject)) { throw "External project file was removed by uninstall" }

$installVerification = [ordered]@{
    baselineVersion = $baselineVersion
    upgradedVersion = $Version
    stableAppIdUpgradePassed = $true
    applicationDataPreservedAcrossUpgrade = $true
    externalProjectPreservedAcrossUpgrade = $true
    defaultStartMenuShortcutCreated = $true
    desktopShortcutRemainedOptIn = $true
    cleanUninstallPassed = $true
    applicationDataPreservedAfterUninstall = $true
    externalProjectPreservedAfterUninstall = $true
    interactiveInstallerUiVerified = $false
    environment = "github-hosted-windows-2025-noninteractive"
}
$installVerification | ConvertTo-Json | Set-Content (Join-Path $artifacts "INSTALL_VERIFY.json") -Encoding utf8NoBOM

Get-Content (Join-Path $artifacts "LAUNCH_SMOKE.json")
Get-Content (Join-Path $artifacts "INSTALL_VERIFY.json")
if ($startupErrorCaptured) {
    Write-Host "Captured native startup diagnostic:"
    Get-Content (Join-Path $artifacts "STARTUP_ERROR.log")
}
