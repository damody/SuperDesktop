[CmdletBinding()]
param(
    [string]$Workspace,
    [Parameter(Mandatory)][string]$ConfirmationPath
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = Split-Path -Parent $PSScriptRoot }
$root = (Resolve-Path -LiteralPath $Workspace).Path
$confirmationPath = (Resolve-Path -LiteralPath $ConfirmationPath).Path
$revision = (& git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $revision -notmatch '^[0-9a-f]{40}$') { throw 'Unable to bind current Git revision.' }
& git -C $root diff --quiet
if ($LASTEXITCODE -ne 0) { throw 'Independent review must target a revision without tracked worktree changes.' }
& git -C $root diff --cached --quiet
if ($LASTEXITCODE -ne 0) { throw 'Independent review must target a revision without staged changes.' }

$confirmation = Get-Content -Raw -Encoding utf8 -LiteralPath $confirmationPath | ConvertFrom-Json
if ($confirmation.schema -cne 'shell-completion-independent-review/v1' -or $confirmation.reviewed_revision -cne $revision) { throw 'Independent-review confirmation is invalid or stale.' }
try { [DateTimeOffset]::Parse($confirmation.recorded_at_utc) | Out-Null } catch { throw 'recorded_at_utc must be ISO-8601.' }
if ([string]::IsNullOrWhiteSpace($confirmation.reviewer.name) -or [string]::IsNullOrWhiteSpace($confirmation.reviewer.organization) -or [string]::IsNullOrWhiteSpace($confirmation.reviewer.role)) { throw 'Reviewer identity is incomplete.' }
if (-not $confirmation.independence.not_implementation_owner -or -not $confirmation.independence.not_remediation_owner) { throw 'Both reviewer independence attestations are required.' }
foreach ($area in @('architecture','security','accessibility','evidence_lineage')) { if ($confirmation.scope.$area -cne 'passed') { throw "Review area is not passed: $area" } }
$requiredChanges = @(
    'extend-superdesktop-shell-contracts','add-superdesktop-desktop-file-operations','add-superdesktop-shell-context-menu-host',
    'add-superdesktop-start-search','add-superdesktop-taskbar-advanced-interactions','add-superdesktop-notification-area-host',
    'add-superdesktop-virtual-desktops','add-superdesktop-shell-installer','verify-superdesktop-shell-completion','complete-superdesktop-windows-shell'
)
$reviewedChanges = @($confirmation.scope.changes | Sort-Object -Unique)
if (@($requiredChanges | Where-Object { $_ -notin $reviewedChanges }).Count -ne 0 -or @($reviewedChanges | Where-Object { $_ -notin $requiredChanges }).Count -ne 0) { throw 'Review scope must contain every completion change exactly once.' }
$findings = @($confirmation.findings)
foreach ($finding in $findings) {
    if ([string]::IsNullOrWhiteSpace($finding.id) -or $finding.severity -notin @('P0','P1','P2','P3') -or $finding.status -notin @('open','resolved')) { throw 'Every finding needs id, P0-P3 severity, and open/resolved status.' }
}
$unresolved = @($findings | Where-Object { $_.severity -in @('P0','P1') -and $_.status -ne 'resolved' })
if ($unresolved.Count -ne 0) { throw 'Independent review contains unresolved P0/P1 findings.' }
if ($confirmation.final_disposition -cne 'no-unresolved-p0-p1') { throw 'Final review disposition is not releasable.' }

$requiredPaths = @(
    'docs/superpowers/specs/2026-08-16-superdesktop-windows-shell-completion-design.md',
    'openspec/changes/complete-superdesktop-windows-shell/PROGRAM.md',
    'openspec/changes/complete-superdesktop-windows-shell/design.md',
    'openspec/changes/complete-superdesktop-windows-shell/specs/windows-shell-completion-program/spec.md',
    'openspec/changes/verify-superdesktop-shell-completion/design.md',
    'openspec/changes/verify-superdesktop-shell-completion/specs/shell-completion-verification/spec.md',
    'openspec/changes/verify-superdesktop-shell-completion/evidence/current-rollup.json'
)
$sourceManifest = @()
foreach ($path in $requiredPaths) {
    $oid = (& git -C $root rev-parse "$revision`:$path").Trim()
    if ($LASTEXITCODE -ne 0 -or $oid -notmatch '^[0-9a-f]{40,64}$') { throw "Reviewed source is unavailable: $path" }
    $sourceManifest += [ordered]@{ path=$path;git_blob_oid=$oid }
}
$artifact = [ordered]@{
    schema_version = 1
    kind = 'independent-review'
    status = 'passed'
    recorded_at_utc = $confirmation.recorded_at_utc
    revision = $revision
    reviewer = $confirmation.reviewer
    independence = [ordered]@{ not_implementation_owner=$true;not_remediation_owner=$true }
    scope = [ordered]@{ architecture='passed';security='passed';accessibility='passed';evidence_lineage='passed';changes=$requiredChanges }
    source_manifest = $sourceManifest
    findings = $findings
    unresolved_p0_p1 = 0
    final_disposition = 'no-unresolved-p0-p1'
    gates = [ordered]@{ 'G-REVIEW'='passed' }
}
$output = Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence\external\independent-review.json'
[IO.Directory]::CreateDirectory((Split-Path -Parent $output)) | Out-Null
[IO.File]::WriteAllText($output, (($artifact | ConvertTo-Json -Depth 30) + "`n"), [Text.UTF8Encoding]::new($false))
Write-Output "Independent completion review evidence captured at $output"
