[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot,

    [Parameter()]
    [string]$Fixture
)

$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }
$currentInventory = Join-Path $WorkspaceRoot 'compliance/third-party-license-inventory.json'
$activeInventory = Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace/compliance/third-party-license-inventory.json'
$inventoryCandidates = @()
if (Test-Path -LiteralPath $currentInventory -PathType Leaf) {
    $inventoryCandidates += $currentInventory
} elseif (Test-Path -LiteralPath $activeInventory -PathType Leaf) {
    $inventoryCandidates += $activeInventory
} else {
    $archiveRoot = Join-Path $WorkspaceRoot 'openspec/changes/archive'
    if (Test-Path -LiteralPath $archiveRoot -PathType Container) {
        $inventoryCandidates += @(
            Get-ChildItem -LiteralPath $archiveRoot -Directory |
                Where-Object { $_.Name -like '*-bootstrap-superdesktop-workspace' } |
                ForEach-Object { Join-Path $_.FullName 'compliance/third-party-license-inventory.json' } |
                Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }
        )
    }
}
$inventoryCandidates = @($inventoryCandidates | Sort-Object -Unique)
if ($inventoryCandidates.Count -ne 1) {
    throw "LICENSE_INVENTORY_RESOLUTION: expected exactly one active or archived inventory, found $($inventoryCandidates.Count)."
}
$inventoryPath = $inventoryCandidates[0]
$forbiddenPathPattern = '(?i)(?:D:\\+SuperExplorer|D:\\+SuperDesktop\\+PExplorer|(?:\.\.[\\/])+SuperExplorer|vendor[\\/]gpui-ce)'
$forbiddenDerivedPattern = '(?i)(?:PExplorer|ReactOS)'
$approvedGitSources = @('git+https://github.com/damody/gpui-ce-explorer.git?rev=54f1339ef1791cf1d1a281ff7fa37d7e0d6b8436#54f1339ef1791cf1d1a281ff7fa37d7e0d6b8436')

function Test-TextForBoundaryViolation {
    param([string]$Path)
    $text = Get-Content -Raw -Encoding UTF8 $Path
    if ($text -match $forbiddenPathPattern) {
        throw "SUPEREXPLORER_PATH_DEPENDENCY: $Path references a forbidden local source boundary."
    }
    if ($text -match $forbiddenDerivedPattern) {
        throw "PEXPLORER_DERIVED_SOURCE: $Path contains a prohibited derivation marker."
    }
}

if ($Fixture) {
    $fixturePath = Join-Path $WorkspaceRoot $Fixture
    if (-not (Test-Path $fixturePath)) { throw "Fixture does not exist: $Fixture" }
    Get-ChildItem -Path $fixturePath -Recurse -File | ForEach-Object { Test-TextForBoundaryViolation $_.FullName }
    throw "Fixture did not violate source-boundary policy: $Fixture"
}

$inventory = Get-Content -Raw -Encoding UTF8 $inventoryPath | ConvertFrom-Json
$metadata = cargo metadata --locked --offline --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed.' }
$workspaceRootCanonical = (Resolve-Path $WorkspaceRoot).Path.TrimEnd('\')

foreach ($package in $metadata.packages) {
    $source = if ($null -eq $package.source) { 'workspace' } else { [string]$package.source }
    $record = @($inventory.packages | Where-Object { $_.name -eq $package.name -and $_.version -eq $package.version -and $_.source -eq $source })
    if ($record.Count -ne 1) {
        throw "LICENSE_INVENTORY_COVERAGE: $($package.name) $($package.version) has $($record.Count) matching inventory records."
    }
    if ([string]::IsNullOrWhiteSpace([string]$record[0].license)) {
        throw "LICENSE_MISSING: $($package.name) $($package.version) has no recorded license."
    }
    if ($source -like 'git+*' -and $source -notin $approvedGitSources) {
        throw "UNAPPROVED_GIT_ORIGIN: $($package.name) $($package.version) uses $source"
    }
    if ($source -ne 'workspace' -and $source -notlike 'registry+*' -and $source -notin $approvedGitSources) {
        throw "UNAPPROVED_DEPENDENCY_ORIGIN: $($package.name) $($package.version) uses $source"
    }

    $manifestPath = [string]$package.manifest_path
    if ($null -eq $package.source) {
        if (-not $manifestPath.StartsWith($workspaceRootCanonical, [StringComparison]::OrdinalIgnoreCase)) {
            throw "OUT_OF_WORKSPACE_PATH_PACKAGE: $manifestPath"
        }
    } else {
        $checksumPath = Join-Path (Split-Path -Parent $manifestPath) '.cargo-checksum.json'
        if (-not (Test-Path $checksumPath)) {
            throw "VENDOR_CHECKSUM_MISSING: $($package.name) $($package.version)"
        }
        $actualChecksum = (Get-FileHash -Algorithm SHA256 $checksumPath).Hash
        if ($record[0].vendor_checksum_sha256 -ne $actualChecksum) {
            throw "VENDOR_CHECKSUM_DRIFT: $($package.name) $($package.version)"
        }
    }
}

$productionManifests = @((Join-Path $WorkspaceRoot 'Cargo.toml')) + (Get-ChildItem -Path (Join-Path $WorkspaceRoot 'crates') -Filter Cargo.toml -Recurse -File | Select-Object -ExpandProperty FullName)
foreach ($manifest in $productionManifests) { Test-TextForBoundaryViolation $manifest }
Get-ChildItem -Path (Join-Path $WorkspaceRoot 'crates') -Directory | ForEach-Object {
    $sourceRoot = Join-Path $_.FullName 'src'
    if (Test-Path -LiteralPath $sourceRoot -PathType Container) {
        Get-ChildItem -LiteralPath $sourceRoot -Filter '*.rs' -Recurse -File |
            ForEach-Object { Test-TextForBoundaryViolation $_.FullName }
    }
}

Write-Output "Source boundary audit passed: $(@($metadata.packages).Count) package records have inventory/license/checksum coverage; no external local path dependency or PExplorer-derived production source marker was found."
