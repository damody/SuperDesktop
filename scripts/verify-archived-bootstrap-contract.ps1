[CmdletBinding()]
param(
    [string]$WorkspaceRoot,
    [string]$ArchiveRelative = 'openspec/changes/archive/2026-08-13-bootstrap-superdesktop-workspace',
    [string]$ArchiveRevision = '9f115980af3804829fc156029ae3b22382c7a146'
)

$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }
$workspace = (Resolve-Path -LiteralPath $WorkspaceRoot).Path.TrimEnd('\')
$archive = [IO.Path]::GetFullPath((Join-Path $workspace $ArchiveRelative))
if (-not $archive.StartsWith($workspace, [StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $archive -PathType Container)) { throw 'BOOTSTRAP_ARCHIVE_PATH_INVALID' }

& git -C $workspace cat-file -e "$ArchiveRevision`^{commit}"
if ($LASTEXITCODE -ne 0) { throw 'BOOTSTRAP_ARCHIVE_REVISION_MISSING' }
& git -C $workspace diff --quiet $ArchiveRevision -- $ArchiveRelative
if ($LASTEXITCODE -ne 0) { throw 'BOOTSTRAP_ARCHIVE_TREE_DRIFT' }

function Resolve-ArchivedInput([string]$DeclaredPath) {
    if ([IO.Path]::IsPathRooted($DeclaredPath) -or $DeclaredPath -match '(^|[\/])\.\.([\/]|$)') { throw "BOOTSTRAP_CONTRACT_PATH_ESCAPE: $DeclaredPath" }
    $oldPrefix = 'openspec/changes/bootstrap-superdesktop-workspace/'
    if ($DeclaredPath.StartsWith($oldPrefix, [StringComparison]::OrdinalIgnoreCase)) { return Join-Path $archive $DeclaredPath.Substring($oldPrefix.Length) }
    return Join-Path $workspace $DeclaredPath
}

function Test-Manifest([string]$ManifestPath, [switch]$Aggregate) {
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $ManifestPath) {
        if (-not $line.Trim()) { continue }
        if ($line -notmatch '^(?<hash>[A-F0-9]{64})  (?:(?<scope>change|workspace)/)?(?<path>.+)$') { throw "BOOTSTRAP_CONTRACT_MANIFEST_MALFORMED: $line" }
        if ($Aggregate -and $Matches.scope -eq 'change') { $candidate = Join-Path $archive $Matches.path }
        elseif ($Matches.scope -eq 'workspace') { $candidate = Join-Path $workspace $Matches.path }
        else { $candidate = Resolve-ArchivedInput $Matches.path }
        $candidate = [IO.Path]::GetFullPath($candidate)
        if (-not $candidate.StartsWith($workspace, [StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $candidate -PathType Leaf)) { throw "BOOTSTRAP_CONTRACT_INPUT_MISSING: $($Matches.path)" }
        if ((Get-FileHash -Algorithm SHA256 -LiteralPath $candidate).Hash -ne $Matches.hash) { throw "BOOTSTRAP_CONTRACT_INPUT_DRIFT: $($Matches.path)" }
    }
}

$aggregate = Join-Path $archive 'evidence/artifacts/2.5/aggregate-contract-inputs.sha256'
$handoff = Get-Content -Raw -Encoding UTF8 (Join-Path $archive 'evidence/handoffs/2.5.json') | ConvertFrom-Json
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $aggregate).Hash -ne $handoff.contract_hash.value) { throw 'BOOTSTRAP_ARCHIVE_HANDOFF_HASH_DRIFT' }
Test-Manifest $aggregate -Aggregate
foreach ($manifest in @('workspace-current-inputs.sha256','dependency-current-inputs.sha256','source-boundary-current-inputs.sha256')) { Test-Manifest (Join-Path $archive "evidence/artifacts/2.5/$manifest") }
Write-Output "Archived bootstrap contract passed at $ArchiveRevision with relocation-only path mapping."
