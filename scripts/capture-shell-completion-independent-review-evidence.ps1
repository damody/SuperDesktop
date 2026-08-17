[CmdletBinding()]
param(
    [string]$Workspace,
    [Parameter(Mandatory)][string]$ConfirmationPath
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = Split-Path -Parent $PSScriptRoot }
$root = (Resolve-Path -LiteralPath $Workspace).Path
$confirmationPath = (Resolve-Path -LiteralPath $ConfirmationPath).Path
function Get-RepositoryRelativePath([string]$Path, [string]$Label) {
    $full = [IO.Path]::GetFullPath($Path)
    $prefix = $root.TrimEnd('\') + '\'
    if (-not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw "$Label must be stored inside the repository." }
    return $full.Substring($prefix.Length).Replace('\','/')
}
$confirmationRelativePath = Get-RepositoryRelativePath $confirmationPath 'Independent review confirmation'
$candidate = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence\release-candidate.json') | ConvertFrom-Json
$revision = [string]$candidate.reviewed_revision
& git -C $root cat-file -e "$revision^{commit}"
if ($candidate.schema_version -ne 1 -or $LASTEXITCODE -ne 0) { throw 'Unable to bind frozen release-candidate revision.' }
& git -C $root merge-base --is-ancestor $revision HEAD
if ($LASTEXITCODE -ne 0) { throw 'Current checkout does not descend from the frozen release candidate.' }
$confirmation = Get-Content -Raw -Encoding utf8 -LiteralPath $confirmationPath | ConvertFrom-Json
if ($confirmation.schema -cne 'shell-completion-independent-review/v1' -or $confirmation.reviewed_revision -cne $revision) { throw 'Independent-review confirmation is invalid or stale.' }
try { [DateTimeOffset]::Parse($confirmation.recorded_at_utc) | Out-Null } catch { throw 'recorded_at_utc must be ISO-8601.' }
if ([string]::IsNullOrWhiteSpace($confirmation.reviewer.name) -or [string]::IsNullOrWhiteSpace($confirmation.reviewer.organization) -or [string]::IsNullOrWhiteSpace($confirmation.reviewer.role) -or
    [string]$confirmation.reviewer.name -like 'REPLACE_WITH_*' -or [string]$confirmation.reviewer.organization -like 'REPLACE_WITH_*') { throw 'Reviewer identity is incomplete or still contains a template placeholder.' }
if (-not $confirmation.independence.not_implementation_owner -or -not $confirmation.independence.not_remediation_owner) { throw 'Both reviewer independence attestations are required.' }
foreach ($area in @('architecture','security','accessibility','evidence_lineage')) { if ($confirmation.scope.$area -cne 'passed') { throw "Review area is not passed: $area" } }
$requiredChanges = @(
    'extend-superdesktop-shell-contracts','add-superdesktop-desktop-file-operations','add-superdesktop-shell-context-menu-host',
    'add-superdesktop-start-search','add-superdesktop-taskbar-advanced-interactions','add-superdesktop-notification-area-host',
    'add-superdesktop-virtual-desktops','add-superdesktop-shell-installer','adopt-superdesktop-windows11-reference-release',
    'verify-superdesktop-shell-completion','complete-superdesktop-windows-shell'
)
$reviewedChanges = @($confirmation.scope.changes | Sort-Object -Unique)
if (@($requiredChanges | Where-Object { $_ -notin $reviewedChanges }).Count -ne 0 -or @($reviewedChanges | Where-Object { $_ -notin $requiredChanges }).Count -ne 0) { throw 'Review scope must contain every completion change exactly once.' }
$findings = @($confirmation.findings)
$findingIds = @{}
foreach ($finding in $findings) {
    if ([string]::IsNullOrWhiteSpace($finding.id) -or [string]::IsNullOrWhiteSpace($finding.summary) -or
        $finding.severity -notin @('P0','P1','P2','P3') -or $finding.status -notin @('open','resolved')) { throw 'Every finding needs id, summary, P0-P3 severity, and open/resolved status.' }
    if ($findingIds.ContainsKey([string]$finding.id)) { throw "Duplicate review finding id: $($finding.id)" }
    $findingIds[[string]$finding.id] = $true
    if ($finding.status -ceq 'resolved' -and [string]::IsNullOrWhiteSpace($finding.resolution)) { throw "Resolved finding lacks resolution: $($finding.id)" }
}
$unresolved = @($findings | Where-Object { $_.severity -in @('P0','P1') -and $_.status -ne 'resolved' })
if ($unresolved.Count -ne 0) { throw 'Independent review contains unresolved P0/P1 findings.' }
if ($confirmation.final_disposition -cne 'no-unresolved-p0-p1') { throw 'Final review disposition is not releasable.' }

& git -C $root diff --quiet
if ($LASTEXITCODE -ne 0) { throw 'Independent review must target a revision without tracked worktree changes.' }
& git -C $root diff --cached --quiet
if ($LASTEXITCODE -ne 0) { throw 'Independent review must target a revision without staged changes.' }

$requiredPaths = @(
    'docs/superpowers/specs/2026-08-16-superdesktop-windows-shell-completion-design.md',
    'docs/superpowers/specs/2026-08-17-superdesktop-windows11-reference-release-design.md',
    'openspec/changes/adopt-superdesktop-windows11-reference-release/design.md',
    'openspec/changes/adopt-superdesktop-windows11-reference-release/specs/windows11-reference-release-baseline/spec.md',
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
    reviewed_tree_oid = (& git -C $root rev-parse "$revision^{tree}").Trim()
    confirmation = [ordered]@{ path=$confirmationRelativePath;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $confirmationPath).Hash.ToLowerInvariant() }
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
