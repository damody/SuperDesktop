[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot
)

$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }
$outputPath = Join-Path $WorkspaceRoot 'compliance/third-party-license-inventory.json'
$metadata = cargo metadata --locked --offline --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed.' }

$workspaceRootCanonical = (Resolve-Path $WorkspaceRoot).Path.TrimEnd('\')
$packages = foreach ($package in $metadata.packages | Sort-Object name, version, source) {
    $manifestPath = [string]$package.manifest_path
    $isWorkspacePackage = $manifestPath.StartsWith($workspaceRootCanonical, [StringComparison]::OrdinalIgnoreCase) -and -not $manifestPath.Contains('\vendor\')
    $checksumPath = Join-Path (Split-Path -Parent $manifestPath) '.cargo-checksum.json'
    [PSCustomObject]@{
        name = $package.name
        version = $package.version
        source = if ($null -eq $package.source) { 'workspace' } else { [string]$package.source }
        revision = if ($null -eq $package.source) { $null } elseif ([string]$package.source -match '#(?<revision>[0-9a-f]{40})$') { $Matches.revision } else { $null }
        license = [string]$package.license
        manifest_path = $manifestPath.Replace($workspaceRootCanonical, '<workspace>')
        vendor_checksum_sha256 = if ($isWorkspacePackage) { $null } elseif (Test-Path $checksumPath) { (Get-FileHash -Algorithm SHA256 $checksumPath).Hash } else { $null }
    }
}

$inventory = [PSCustomObject]@{
    schema_version = 1
    generated_by = 'cargo metadata --locked --offline plus vendor .cargo-checksum.json'
    workspace = 'SuperDesktop'
    packages = @($packages)
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outputPath) | Out-Null
[IO.File]::WriteAllText(
    $outputPath,
    (($inventory | ConvertTo-Json -Depth 5) + [Environment]::NewLine),
    [Text.UTF8Encoding]::new($false)
)

Write-Output "Generated $outputPath with $(@($packages).Count) package records."
