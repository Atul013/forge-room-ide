param(
    [string]$Version = "0.109.0-alpha.1",
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$manifest = Join-Path $root "native\forge-room-app\Cargo.toml"
$releaseExe = Join-Path $root "native\forge-room-app\target\$Target\release\forge-room-early-preview.exe"
$artifacts = Join-Path $root "artifacts\windows-x64"
$package = Join-Path $artifacts "Forge-Room-Early-Preview-$Version-windows-x64"
$licenses = Join-Path $package "licenses"

Remove-Item $artifacts -Recurse -Force -ErrorAction SilentlyContinue
New-Item $licenses -ItemType Directory -Force | Out-Null

if (-not (Test-Path $releaseExe)) {
    throw "Release executable is missing: $releaseExe"
}

Copy-Item $releaseExe (Join-Path $package "forge-room-early-preview.exe")
Copy-Item (Join-Path $root "docs\native\EARLY_PREVIEW_NOTES.md") $package
Copy-Item (Join-Path $root "native\LICENSE-MIT") (Join-Path $package "LICENSE-MIT.txt")

cargo fetch --locked --manifest-path $manifest --target $Target
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
    $sourceDir = Split-Path $item.manifest_path -Parent
    $licenseFiles = Get-ChildItem $sourceDir -File | Where-Object {
        $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)(\.|-|$)'
    }
    if (-not $licenseFiles) {
        throw "Dependency $($item.name) $($item.version) has no packaged license/notice file"
    }
    $destination = Join-Path $licenses "$($item.name)-$($item.version)"
    New-Item $destination -ItemType Directory -Force | Out-Null
    foreach ($file in $licenseFiles) { Copy-Item $file.FullName $destination }
    $noticeRows += "- $($item.name) $($item.version) — $($item.license) — $($item.repository)"
}

$noticeHeader = @(
    "# Third-party notices",
    "",
    "The corresponding license and notice texts are stored under the licenses directory.",
    "This inventory was generated from Cargo metadata for $Target.",
    ""
)
Set-Content (Join-Path $package "THIRD_PARTY_NOTICES.md") ($noticeHeader + $noticeRows) -Encoding utf8NoBOM

$installedBytes = (Get-ChildItem $package -Recurse -File | Measure-Object Length -Sum).Sum
$portable = "$package.zip"
Compress-Archive -Path "$package\*" -DestinationPath $portable -CompressionLevel Optimal

$sideSvg = Join-Path $PSScriptRoot "installer-side.svg"
$smallSvg = Join-Path $PSScriptRoot "installer-small.svg"
& magick $sideSvg (Join-Path $PSScriptRoot "installer-side.png")
& magick $smallSvg (Join-Path $PSScriptRoot "installer-small.png")
& magick -background none -density 256 $smallSvg -define icon:auto-resize=256,128,64,48,32,16 (Join-Path $PSScriptRoot "installer.ico")

$isccCandidates = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
)
$iscc = $isccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 compiler is not installed" }
$iss = Join-Path $PSScriptRoot "forge-room-early-preview.iss"
& $iscc "/DPackageDir=$package" "/DOutputDir=$artifacts" "/DAppVersion=$Version" $iss
if ($LASTEXITCODE -ne 0) { throw "Inno Setup compilation failed with exit code $LASTEXITCODE" }

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
    dependencyLicenseDirectories = (Get-ChildItem $licenses -Directory).Count
    signed = $false
}
$metrics | ConvertTo-Json | Set-Content (Join-Path $artifacts "BUILD_METRICS.json") -Encoding utf8NoBOM
Get-Content (Join-Path $artifacts "BUILD_METRICS.json")
