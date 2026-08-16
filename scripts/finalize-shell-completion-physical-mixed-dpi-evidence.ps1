[CmdletBinding()]
param(
    [string]$Workspace,
    [Parameter(Mandatory)][string]$M0PhysicalEvidence,
    [Parameter(Mandatory)][string]$CompletionConfirmation
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = Split-Path -Parent $PSScriptRoot }
$root = (Resolve-Path -LiteralPath $Workspace).Path
$physicalPath = (Resolve-Path -LiteralPath $M0PhysicalEvidence).Path
$confirmationPath = (Resolve-Path -LiteralPath $CompletionConfirmation).Path
function Get-RepositoryRelativePath([string]$Path, [string]$Label) {
    $full = [IO.Path]::GetFullPath($Path)
    $prefix = $root.TrimEnd('\') + '\'
    if (-not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw "$Label must be stored inside the repository." }
    return $full.Substring($prefix.Length).Replace('\','/')
}
Get-RepositoryRelativePath $physicalPath 'M0 physical evidence' | Out-Null
Get-RepositoryRelativePath $confirmationPath 'Completion confirmation' | Out-Null
$candidate = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence\release-candidate.json') | ConvertFrom-Json
$revision = [string]$candidate.reviewed_revision
$shortRevision = $revision.Substring(0, 8)
& git -C $root cat-file -e "$revision^{commit}"
if ($candidate.schema_version -ne 1 -or $LASTEXITCODE -ne 0) { throw 'Unable to bind frozen release-candidate revision.' }
$physical = Get-Content -Raw -Encoding utf8 -LiteralPath $physicalPath | ConvertFrom-Json
$confirmation = Get-Content -Raw -Encoding utf8 -LiteralPath $confirmationPath | ConvertFrom-Json
if ($physical.schema -cne 'm0-physical-mixed-dpi-gate/v1' -or $physical.status -cne 'passed' -or $physical.revision -cne $shortRevision) { throw 'Physical M0 evidence is invalid or stale.' }
if ([string]::IsNullOrWhiteSpace($physical.reviewer.name) -or [string]::IsNullOrWhiteSpace($physical.reviewer.organization)) { throw 'M0 physical evidence reviewer is not attributable.' }
$m0ConfirmationPath = (Resolve-Path -LiteralPath (Join-Path $root ([string]$physical.confirmation_path -replace '/', '\'))).Path
Get-RepositoryRelativePath $m0ConfirmationPath 'M0 physical confirmation' | Out-Null
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $m0ConfirmationPath).Hash.ToLowerInvariant() -cne ([string]$physical.confirmation_sha256).ToLowerInvariant()) { throw 'M0 physical confirmation hash drift.' }
if ($physical.dispositions.'G-DPI-MONITOR' -cne 'passed' -or @($physical.physical_identities).Count -lt 2) { throw 'Physical monitor evidence is incomplete.' }
$distinctDpi = @($physical.monitor_geometry.dpi_x | Sort-Object -Unique).Count
if (@($physical.monitor_geometry).Count -lt 2 -or $distinctDpi -lt 2) { throw 'Two physical monitors with distinct DPI are required.' }
if ($physical.interactions.pointer -cne 'passed' -or $physical.interactions.keyboard_focus -cne 'passed' -or $physical.interactions.drag -cne 'passed' -or $physical.topology.primary_change -cne 'passed' -or $physical.topology.hot_plug_recovery -cne 'passed' -or -not $physical.topology.work_area_restored) { throw 'Physical interaction/topology contract failed.' }
if ($confirmation.schema -cne 'shell-completion-physical-confirmation/v1' -or $confirmation.reviewed_revision -cne $revision) { throw 'Completion physical confirmation is invalid or stale.' }
if ([string]::IsNullOrWhiteSpace($confirmation.operator.name) -or [string]::IsNullOrWhiteSpace($confirmation.operator.organization) -or
    [string]$confirmation.operator.name -like 'REPLACE_WITH_*' -or [string]$confirmation.operator.organization -like 'REPLACE_WITH_*') { throw 'Attributable physical operator identity is required.' }
try { [DateTimeOffset]::Parse($confirmation.recorded_at_utc) | Out-Null } catch { throw 'recorded_at_utc must be ISO-8601.' }
$requiredFeatures = @('desktop_file_operations','context_menu','start_search','taskbar_flyouts','notification_area','virtual_desktop_query_move','accessibility')
foreach ($feature in $requiredFeatures) { if ($confirmation.features.$feature -cne 'passed') { throw "Completion physical feature is not passed: $feature" } }
$artifacts = @($physical.screenshots) + @($physical.photos)
if ($artifacts.Count -lt 4) { throw 'At least four photo/screenshot artifacts are required.' }
$artifactHashes = @()
foreach ($item in $artifacts) {
    $path = (Resolve-Path -LiteralPath $item.path).Path
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
    if ($hash -cne ([string]$item.sha256).ToLowerInvariant()) { throw "Physical artifact hash drift: $path" }
    $artifactHashes += [ordered]@{ path=(Get-RepositoryRelativePath $path 'Physical photo/screenshot');sha256=$hash }
}
$artifact = [ordered]@{
    schema_version = 1
    kind = 'physical-mixed-dpi'
    status = 'passed'
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    revision = $revision
    operator = $confirmation.operator
    monitor_count = @($physical.monitor_geometry).Count
    distinct_dpi_count = $distinctDpi
    interactions = [ordered]@{ pointer='passed';keyboard_focus='passed';drag='passed';primary_change='passed';hot_plug='passed';work_area_restored='passed' }
    completion_features = $confirmation.features
    artifact_hashes = $artifactHashes
    source_hashes = @(
        [ordered]@{ path=(Get-RepositoryRelativePath $physicalPath 'M0 physical evidence');sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $physicalPath).Hash.ToLowerInvariant() },
        [ordered]@{ path=(Get-RepositoryRelativePath $m0ConfirmationPath 'M0 physical confirmation');sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $m0ConfirmationPath).Hash.ToLowerInvariant() },
        [ordered]@{ path=(Get-RepositoryRelativePath $confirmationPath 'Completion confirmation');sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $confirmationPath).Hash.ToLowerInvariant() }
    )
    gates = [ordered]@{ 'G-DPI-MONITOR-PHYSICAL'='passed' }
}
$output = Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence\external\physical-mixed-dpi.json'
[IO.Directory]::CreateDirectory((Split-Path -Parent $output)) | Out-Null
[IO.File]::WriteAllText($output, (($artifact | ConvertTo-Json -Depth 20) + "`n"), [Text.UTF8Encoding]::new($false))
Write-Output "Physical completion evidence finalized at $output"
