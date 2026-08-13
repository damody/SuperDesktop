[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
$expectedRevision = '8945e2981b9fd00ca887e042d8adb9acc241b168'
$expectedUrl = 'https://github.com/damody/gpui-ce-explorer.git'
$manifest = Get-Content -Raw -Encoding UTF8 (Join-Path $WorkspaceRoot 'Cargo.toml')
$lockfile = Get-Content -Raw -Encoding UTF8 (Join-Path $WorkspaceRoot 'Cargo.lock')
$config = Get-Content -Raw -Encoding UTF8 (Join-Path $WorkspaceRoot '.cargo/config.toml')

if ($manifest -notmatch [regex]::Escape("git = `"$expectedUrl`"")) {
    throw 'Workspace manifest does not pin the approved GPUI remote URL.'
}
if ($manifest -notmatch [regex]::Escape("rev = `"$expectedRevision`"")) {
    throw 'Workspace manifest does not pin the approved GPUI revision.'
}
$lockedSource = 'git+' + $expectedUrl + '?rev=' + $expectedRevision + '#' + $expectedRevision
if ($lockfile -notmatch [regex]::Escape($lockedSource)) {
    throw 'Cargo.lock does not resolve GPUI to the approved immutable revision.'
}
if ($config -notmatch [regex]::Escape($expectedRevision) -or $config -notmatch 'replace-with = "vendored-sources"') {
    throw 'Cargo vendor configuration does not replace the approved GPUI source with the local vendor directory.'
}
if (-not (Test-Path (Join-Path $WorkspaceRoot 'vendor/gpui/.cargo-checksum.json'))) {
    throw 'Vendored GPUI package checksum is missing.'
}

$superExplorerPath = 'D:\SuperExplorer\vendor\gpui-ce'
if ($manifest -match [regex]::Escape($superExplorerPath) -or $config -match [regex]::Escape($superExplorerPath)) {
    throw 'SuperExplorer vendor path is forbidden as a dependency source.'
}

Write-Output "Dependency provenance assertion passed: gpui uses $expectedUrl at $expectedRevision and vendor/gpui."
