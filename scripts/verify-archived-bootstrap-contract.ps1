[CmdletBinding()]
param([string]$WorkspaceRoot)

$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }
$workspace = (Resolve-Path -LiteralPath $WorkspaceRoot).Path.TrimEnd('\')
$programHandoffPath = Join-Path $workspace 'openspec/changes/build-superdesktop-shell-foundation/evidence/handoffs/2.1.json'
$programHandoff = Get-Content -Raw -Encoding UTF8 -LiteralPath $programHandoffPath | ConvertFrom-Json
$ArchiveRelative = [string]$programHandoff.archive_path
$ArchiveRevision = [string]$programHandoff.archive_revision
if (-not $ArchiveRelative -or -not $ArchiveRevision -or -not $programHandoff.child_contract_sha256) { throw 'BOOTSTRAP_PROGRAM_HANDOFF_INCOMPLETE' }
$workspaceBoundary = $workspace + [IO.Path]::DirectorySeparatorChar
$archive = [IO.Path]::GetFullPath((Join-Path $workspace $ArchiveRelative))
$archiveBoundary = $archive.TrimEnd('\') + [IO.Path]::DirectorySeparatorChar
if (-not $archive.StartsWith($workspaceBoundary, [StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $archive -PathType Container)) { throw 'BOOTSTRAP_ARCHIVE_PATH_INVALID' }

& git -C $workspace cat-file -e "$ArchiveRevision`^{commit}"
if ($LASTEXITCODE -ne 0) { throw 'BOOTSTRAP_ARCHIVE_REVISION_MISSING' }
& git -C $workspace diff --quiet $ArchiveRevision -- $ArchiveRelative
if ($LASTEXITCODE -ne 0) { throw 'BOOTSTRAP_ARCHIVE_TREE_DRIFT' }

function Resolve-ArchivedInput([string]$DeclaredPath) {
    if ([IO.Path]::IsPathRooted($DeclaredPath) -or $DeclaredPath -match '(^|[\\/])\.\.([\\/]|$)') { throw "BOOTSTRAP_CONTRACT_PATH_ESCAPE: $DeclaredPath" }
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
        $requiredBoundary = if ($Aggregate -and $Matches.scope -eq 'change') { $archiveBoundary } else { $workspaceBoundary }
        if (-not $candidate.StartsWith($requiredBoundary, [StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $candidate -PathType Leaf)) { throw "BOOTSTRAP_CONTRACT_INPUT_MISSING: $($Matches.path)" }
        if ((Get-FileHash -Algorithm SHA256 -LiteralPath $candidate).Hash -ne $Matches.hash) { throw "BOOTSTRAP_CONTRACT_INPUT_DRIFT: $($Matches.path)" }
    }
}

$aggregate = Join-Path $archive 'evidence/artifacts/2.5/aggregate-contract-inputs.sha256'
$handoff = Get-Content -Raw -Encoding UTF8 (Join-Path $archive 'evidence/handoffs/2.5.json') | ConvertFrom-Json
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $aggregate).Hash -ne $handoff.contract_hash.value) { throw 'BOOTSTRAP_ARCHIVE_HANDOFF_HASH_DRIFT' }
if ($handoff.contract_hash.value -ne $programHandoff.child_contract_sha256) { throw 'BOOTSTRAP_PROGRAM_CHILD_HASH_DRIFT' }
Test-Manifest $aggregate -Aggregate
foreach ($manifest in @('workspace-current-inputs.sha256','dependency-current-inputs.sha256','source-boundary-current-inputs.sha256')) { Test-Manifest (Join-Path $archive "evidence/artifacts/2.5/$manifest") }
Write-Output "Archived bootstrap contract passed at $ArchiveRevision with relocation-only path mapping."
