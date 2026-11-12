# Builds the Ember setup: a WPF host (no new runtime; .NET Framework ships with Windows 11)
# that embeds the Inno Setup engine, which performs the real install, upgrade and uninstall.
param(
    [string]$PackageDir = (Join-Path $PSScriptRoot "..\..\artifacts\installer-redesign\payload"),
    [string]$OutputDir = (Join-Path $PSScriptRoot "..\..\artifacts\installer-redesign"),
    [string]$Version = "0.110.0-alpha.3"
)

$ErrorActionPreference = 'Stop'
$PackageDir = (Resolve-Path $PackageDir).Path
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$OutputDir = (Resolve-Path $OutputDir).Path
$ember = Join-Path $PSScriptRoot "ember"
$work = Join-Path $OutputDir "build"
New-Item -ItemType Directory -Path $work -Force | Out-Null

$iscc = @(
    (Join-Path $PSScriptRoot "..\..\artifacts\build-tools\innosetup\ISCC.exe"),
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6.7+ compiler not found" }
$csc = Join-Path $env:WINDIR "Microsoft.NET\Framework64\v4.0.30319\csc.exe"
if (-not (Test-Path $csc)) { throw ".NET Framework C# compiler not found" }

# 1. Policy checks first: a failure here must stop the build.
$tests = Join-Path $work "policytests.exe"
& $csc /nologo /out:$tests (Join-Path $ember "PolicyTests.cs") (Join-Path $ember "InstallerPolicy.cs")
if ($LASTEXITCODE -ne 0) { throw "Policy tests failed to compile" }
& $tests
if ($LASTEXITCODE -ne 0) { throw "Installer policy checks failed" }

# 2. Engine: the Inno installer without its own wizard UI.
& (Join-Path $PSScriptRoot "render-installer-assets.ps1") -OutputDirectory $work | Out-Null
& $iscc "/DPackageDir=$PackageDir" "/DOutputDir=$work" "/DAppVersion=$Version" "/DInstallerSuffix=-engine" "/DEngineOnly=1" (Join-Path $PSScriptRoot "forge-room-early-preview.iss")
if ($LASTEXITCODE -ne 0) { throw "Inno engine compilation failed" }
$engine = Join-Path $work "Forge-Room-Early-Preview-$Version-windows-x64-setup-engine.exe"
if (-not (Test-Path $engine)) { throw "Engine not produced: $engine" }

# 3. Metadata shown by the UI. Required space is payload plus 4 MiB for the uninstaller and
#    registration; Inno still performs its own authoritative check at install time.
$payloadFiles = "forge-room-early-preview.exe", "EARLY_PREVIEW_NOTES.md", "LICENSE-MIT.txt", "THIRD_PARTY_NOTICES.md", "THIRD_PARTY_LICENSES.html"
$payloadBytes = ($payloadFiles | ForEach-Object { (Get-Item (Join-Path $PackageDir $_)).Length } | Measure-Object -Sum).Sum
$requiredMb = [math]::Round(($payloadBytes + (Get-Item $engine).Length + 4MB) / 1MB, 1).ToString([cultureinfo]::InvariantCulture)
$meta = Join-Path $work "meta.txt"
@("version=$Version", "payloadBytes=$payloadBytes", "requiredText=At least $requiredMb MB of free disk space is required.") |
    Set-Content $meta -Encoding utf8

# 4. Host executable with everything embedded.
$art = Join-Path $work "installer-art.png"
$output = Join-Path $OutputDir "Forge-Room-Early-Preview-$Version-windows-x64-setup.exe"
$wpf = Join-Path $env:WINDIR "Microsoft.NET\Framework64\v4.0.30319\WPF"
$cscArgs = @(
    "/nologo", "/target:winexe", "/optimize+", "/platform:x64", "/out:$output",
    "/win32icon:$(Join-Path $PSScriptRoot 'installer.ico')",
    "/r:$wpf\PresentationFramework.dll", "/r:$wpf\PresentationCore.dll", "/r:$wpf\WindowsBase.dll",
    "/r:System.Xaml.dll", "/r:System.Windows.Forms.dll",
    "/resource:$engine,engine.exe", "/resource:$art,art.png", "/resource:$meta,meta.txt",
    "/resource:$(Join-Path $PackageDir 'LICENSE-MIT.txt'),LICENSE-MIT.txt",
    "/resource:$(Join-Path $PackageDir 'EARLY_PREVIEW_NOTES.md'),EARLY_PREVIEW_NOTES.md",
    (Join-Path $ember "Program.cs"), (Join-Path $ember "InstallerPolicy.cs")
)
& $csc @cscArgs
if ($LASTEXITCODE -ne 0) { throw "Ember host compilation failed" }
Write-Host "Built $output"
Get-Item $output | Select-Object Name, Length
