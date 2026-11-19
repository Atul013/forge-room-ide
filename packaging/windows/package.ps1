param(
    [string]$Version = "0.110.0-alpha.3",
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$manifest = Join-Path $root "native\forge-room-app\Cargo.toml"
$releaseExe = Join-Path $root "native\forge-room-app\target\$Target\release\forge-room-early-preview.exe"
$artifacts = Join-Path $root "artifacts\windows-x64"
$package = Join-Path $artifacts "Forge-Room-Early-Preview-$Version-windows-x64"

$resolvedArtifacts = [IO.Path]::GetFullPath($artifacts)
$expectedArtifacts = [IO.Path]::GetFullPath((Join-Path $root "artifacts\windows-x64"))
if ($resolvedArtifacts -ne $expectedArtifacts -or -not $resolvedArtifacts.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw "Artifact cleanup escaped the workspace" }
foreach ($artifactPath in @((Join-Path $root "artifacts"), $resolvedArtifacts)) {
    if (Test-Path -LiteralPath $artifactPath) {
        $artifactItem = Get-Item -LiteralPath $artifactPath -Force
        if (($artifactItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Artifact cleanup refuses linked directories" }
    }
}
Remove-Item -LiteralPath $resolvedArtifacts -Recurse -Force -ErrorAction SilentlyContinue
New-Item $package -ItemType Directory -Force | Out-Null

if (-not (Test-Path $releaseExe)) {
    throw "Release executable is missing: $releaseExe"
}

$identityPath = Join-Path $artifacts "BUILD_IDENTITY.json"
$identityProcess = Start-Process -FilePath $releaseExe -ArgumentList "--forge-build-info" -RedirectStandardOutput $identityPath -WindowStyle Hidden -PassThru
$null = $identityProcess.Handle
if (-not $identityProcess.WaitForExit(10000)) { $identityProcess.Kill(); throw "Build identity probe timed out" }
$identityProcess.WaitForExit()
if ($identityProcess.ExitCode -ne 0) { throw "Build identity probe failed: $($identityProcess.ExitCode)" }
$identity = Get-Content -LiteralPath $identityPath -Raw | ConvertFrom-Json
$source = (git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw "Source identity unavailable" }
$nativeTree = (git -C $root rev-parse HEAD:native).Trim()
if ($LASTEXITCODE -ne 0) { throw "Native tree identity unavailable" }
if (-not $identity.recorded -or $identity.sourceRevision -ne $source -or $identity.nativeTree -ne $nativeTree -or $identity.version -ne $Version -or $identity.target -ne $Target -or $identity.profile -ne "release") { throw "Executable identity does not match the requested package source/target/version" }
$identity | Add-Member -NotePropertyName executableSha256 -NotePropertyValue (Get-FileHash -LiteralPath $releaseExe -Algorithm SHA256).Hash.ToLowerInvariant()
$identity | ConvertTo-Json | Set-Content -LiteralPath $identityPath -Encoding utf8NoBOM
Copy-Item $releaseExe (Join-Path $package "forge-room-early-preview.exe")
Copy-Item (Join-Path $root "docs\native\EARLY_PREVIEW_NOTES.md") $package
Copy-Item (Join-Path $root "native\LICENSE-MIT") (Join-Path $package "LICENSE-MIT.txt")

cargo fetch --locked --manifest-path $manifest --target $Target
$aboutConfig = Join-Path $root "native\forge-room-app\about.toml"
$aboutTemplate = Join-Path $PSScriptRoot "third-party-licenses.hbs"
$aboutOutput = Join-Path $package "THIRD_PARTY_LICENSES.html"
cargo about generate --locked --fail --manifest-path $manifest --config $aboutConfig --output-file $aboutOutput $aboutTemplate
if ($LASTEXITCODE -ne 0) { throw "cargo-about license generation failed with exit code $LASTEXITCODE" }

$metadata = cargo metadata --locked --manifest-path $manifest --filter-platform $Target --format-version 1 | ConvertFrom-Json
$nodeById = @{}
foreach ($node in $metadata.resolve.nodes) { $nodeById[$node.id] = $node }
$packageById = @{}
foreach ($item in $metadata.packages) { $packageById[$item.id] = $item }
$rootPackage = $metadata.packages | Where-Object { $_.name -eq "forge-room-app" -and -not $_.source } | Select-Object -First 1
if (-not $rootPackage) { throw "Could not identify the Forge Room app package" }

$queue = [System.Collections.Generic.Queue[string]]::new()
$queue.Enqueue($rootPackage.id)
$visited = [System.Collections.Generic.HashSet[string]]::new()
while ($queue.Count -gt 0) {
    $id = $queue.Dequeue()
    if (-not $visited.Add($id)) { continue }
    foreach ($dependency in $nodeById[$id].dependencies) { $queue.Enqueue($dependency) }
}

$noticeRows = @()
foreach ($id in ($visited | Sort-Object)) {
    $item = $packageById[$id]
    if (-not $item.source) { continue }
    $noticeRows += "- $($item.name) $($item.version) — $($item.license) — $($item.repository)"
}

$noticeHeader = @(
    "# Third-party notices",
    "",
    "The corresponding full license texts and crate associations are stored in THIRD_PARTY_LICENSES.html.",
    "This inventory was generated from locked Cargo metadata for $Target.",
    ""
)
Set-Content (Join-Path $package "THIRD_PARTY_NOTICES.md") ($noticeHeader + $noticeRows) -Encoding utf8NoBOM

$installedBytes = (Get-ChildItem $package -Recurse -File | Measure-Object Length -Sum).Sum
$portable = "$package.zip"
Compress-Archive -Path "$package\*" -DestinationPath $portable -CompressionLevel Optimal

# Branded setup: WPF host embedding the Inno engine (see build-ember.ps1).
& (Join-Path $PSScriptRoot "build-ember.ps1") -PackageDir $package -OutputDir $artifacts -Version $Version
if ($LASTEXITCODE -ne 0) { throw "Ember installer build failed with exit code $LASTEXITCODE" }

$installer = Join-Path $artifacts "Forge-Room-Early-Preview-$Version-windows-x64-setup.exe"
if (-not (Test-Path $installer)) { throw "Expected installer was not produced: $installer" }

$checksumTargets = @($portable, $installer)
$checksumLines = foreach ($file in $checksumTargets) {
    $hash = (Get-FileHash $file -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $([System.IO.Path]::GetFileName($file))"
}
Set-Content (Join-Path $artifacts "SHA256SUMS.txt") $checksumLines -Encoding ascii

$metrics = [ordered]@{
    version = $Version
    target = $Target
    releaseExecutableBytes = (Get-Item $releaseExe).Length
    portableArchiveBytes = (Get-Item $portable).Length
    installerBytes = (Get-Item $installer).Length
    preparedInstalledFootprintBytes = $installedBytes
    thirdPartyLicenseReportBytes = (Get-Item $aboutOutput).Length
    dependencyInventoryEntries = $noticeRows.Count
    signed = $false
}
$metrics | ConvertTo-Json | Set-Content (Join-Path $artifacts "BUILD_METRICS.json") -Encoding utf8NoBOM
Get-Content (Join-Path $artifacts "BUILD_METRICS.json")
