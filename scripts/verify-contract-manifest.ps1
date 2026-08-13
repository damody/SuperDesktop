[CmdletBinding()]
param([string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot), [Parameter(Mandatory)][string]$Manifest)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path $WorkspaceRoot).Path.TrimEnd('\')
if (-not (Test-Path -LiteralPath $Manifest -PathType Leaf)) { throw 'CONTRACT_MANIFEST_PATH_INVALID' }
foreach ($line in Get-Content -Encoding UTF8 $Manifest) {
    if (-not $line.Trim()) { continue }
    if ($line -notmatch '^(?<hash>[A-F0-9]{64})  (?:(?<scope>change|workspace)/)?(?<path>.+)$') { throw "CONTRACT_MANIFEST_MALFORMED: $line" }
    $relative = $Matches.path
    if ([IO.Path]::IsPathRooted($relative) -or $relative -match '(^|[\\/])\.\.([\\/]|$)') { throw "CONTRACT_PATH_ESCAPE: $relative" }
    $base = if ($Matches.scope -eq 'change') { Join-Path $root 'openspec/changes/bootstrap-superdesktop-workspace' } else { $root }
    $candidate = [IO.Path]::GetFullPath((Join-Path $base $relative))
    if (-not $candidate.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)) { throw "CONTRACT_PATH_ESCAPE: $relative" }
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { throw "CONTRACT_INPUT_MISSING: $relative" }
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $candidate).Hash -ne $Matches.hash) { throw "CONTRACT_INPUT_DRIFT: $relative" }
}
Write-Output "Contract manifest passed: $Manifest"
