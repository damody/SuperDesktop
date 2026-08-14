[CmdletBinding()]
param(
    [string]$Workspace,
    [Parameter(Mandatory = $true)]
    [string]$ConfirmationPath,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$utf8 = [Text.UTF8Encoding]::new($false)
if (-not $Workspace) { $Workspace = Split-Path -Parent $PSScriptRoot }
if (-not $OutputPath) { $OutputPath = Join-Path $Workspace 'openspec/changes/verify-superdesktop-m0/evidence/artifacts/5.3/independent-review-gate.json' }

function Write-Json([string]$Path, $Value) {
    New-Item -ItemType Directory -Force (Split-Path -Parent $Path) | Out-Null
    [IO.File]::WriteAllText($Path, (($Value | ConvertTo-Json -Depth 40) + "`n"), $utf8)
}

function Git-Checked([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments) {
    $output = & git -C $Workspace @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) { throw "git $($Arguments -join ' ') failed`n$($output | Out-String)" }
    return [string]($output | Select-Object -Last 1)
}

$Workspace = (Resolve-Path -LiteralPath $Workspace).Path
$ConfirmationPath = (Resolve-Path -LiteralPath $ConfirmationPath).Path
$status = (& git -C $Workspace status --porcelain=v1 --untracked-files=all | Out-String).Trim()
if ($status) { throw 'Independent review must target a clean Git worktree' }

$confirmation = Get-Content -Raw -Encoding UTF8 $ConfirmationPath | ConvertFrom-Json
$revision = (Git-Checked rev-parse HEAD).Trim()
if ($confirmation.schema -ne 'm0-independent-review-confirmation/v1') { throw 'Unsupported independent-review confirmation schema' }
if ($confirmation.reviewed_revision -ne $revision -or $confirmation.reviewed_revision -notmatch '^[a-fA-F0-9]{40}$') { throw 'reviewed_revision must equal the current full Git revision' }
if ([string]::IsNullOrWhiteSpace($confirmation.recorded_at)) { throw 'recorded_at is required' }
try { [DateTimeOffset]::Parse($confirmation.recorded_at) | Out-Null } catch { throw 'recorded_at must be an ISO-8601 timestamp' }

$reviewer = $confirmation.reviewer
if ([string]::IsNullOrWhiteSpace($reviewer.name) -or [string]::IsNullOrWhiteSpace($reviewer.organization) -or [string]::IsNullOrWhiteSpace($reviewer.role)) { throw 'reviewer name, organization, and role are required' }
if ($confirmation.independence.not_implementation_owner -ne $true -or $confirmation.independence.not_remediation_owner -ne $true) { throw 'Reviewer independence attestations must both be true' }

$requiredGates = @('G-ARCH','G-SHELL-TAKEOVER','G-GUARDIAN-RECOVERY','G-DESKTOP','G-TASKBAR','G-EXPLORER-BRIDGE','G-A11Y-I18N','G-DPI-MONITOR','G-PERF','G-SAFETY','G-TRACE')
$reviewedGates = @($confirmation.review_scope.gates | Sort-Object -Unique)
if (@($requiredGates | Where-Object { $_ -notin $reviewedGates }).Count -ne 0 -or @($reviewedGates | Where-Object { $_ -notin $requiredGates }).Count -ne 0) { throw 'Review scope must contain every blocking gate exactly once' }
foreach ($area in @('architecture','security','evidence_lineage')) {
    if ($confirmation.review_scope.$area -ne 'passed') { throw "Review scope $area is not passed" }
}

$findings = @($confirmation.findings)
foreach ($finding in $findings) {
    if ([string]::IsNullOrWhiteSpace($finding.id) -or $finding.severity -notin @('P0','P1','P2','P3') -or $finding.status -notin @('open','resolved')) { throw 'Every finding requires id, P0-P3 severity, and open/resolved status' }
    if (@($finding.affected_gates).Count -eq 0 -or @($finding.affected_gates | Where-Object { $_ -notin $requiredGates }).Count -ne 0) { throw "Finding $($finding.id) has invalid affected_gates" }
}
$unresolvedP0P1 = @($findings | Where-Object { $_.severity -in @('P0','P1') -and $_.status -ne 'resolved' })
if ($unresolvedP0P1.Count -ne 0) { throw 'Independent review contains unresolved P0/P1 findings' }
if ($confirmation.primary_integration.p0_p1_assignments_complete -ne $true -or $confirmation.primary_integration.affected_gates_rerun -ne $true) { throw 'Primary integration assignment and affected-gate rerun dispositions must be true' }
if ($confirmation.final_review.remediation_lineage -ne 'passed' -or $confirmation.final_review.disposition -ne 'no-unresolved-p0-p1') { throw 'Final independent review disposition is not releasable' }

$requiredPaths = @(
    'openspec/changes/verify-superdesktop-m0/design.md',
    'openspec/changes/verify-superdesktop-m0/specs/shell-foundation-verification/spec.md',
    'openspec/changes/verify-superdesktop-m0/tasks.md',
    'openspec/changes/verify-superdesktop-m0/evidence/coverage.json',
    'openspec/changes/verify-superdesktop-m0/evidence/index.jsonl',
    'openspec/changes/verify-superdesktop-m0/evidence/adjustments.jsonl',
    'openspec/changes/verify-superdesktop-m0/evidence/artifacts/1.2/build-offline-gate.json',
    'openspec/changes/verify-superdesktop-m0/evidence/artifacts/5.2/safety-license-source.json',
    'openspec/changes/verify-superdesktop-m0/evidence/artifacts/5.3/traceability-review.json'
)
$sourceManifest = @()
foreach ($path in $requiredPaths) {
    $blob = (Git-Checked rev-parse "$revision`:$path").Trim()
    if ($blob -notmatch '^[a-fA-F0-9]{40,64}$') { throw "Unable to bind reviewed Git blob for $path" }
    $sourceManifest += [ordered]@{ path = $path; git_blob_oid = $blob }
}

$artifact = [ordered]@{
    schema = 'm0-independent-review-gate/v1'
    status = 'passed'
    recorded_at = $confirmation.recorded_at
    reviewed_revision = $revision
    reviewer = [ordered]@{ name = $reviewer.name; organization = $reviewer.organization; role = $reviewer.role }
    independence = [ordered]@{ not_implementation_owner = $true; not_remediation_owner = $true }
    review_scope = [ordered]@{ gates = $requiredGates; architecture = 'passed'; security = 'passed'; evidence_lineage = 'passed' }
    source_manifest = $sourceManifest
    findings = $findings
    unresolved_p0_p1 = 0
    primary_integration = [ordered]@{ p0_p1_assignments_complete = $true; affected_gates_rerun = $true }
    final_review = [ordered]@{ remediation_lineage = 'passed'; disposition = 'no-unresolved-p0-p1' }
    task_ids = @('5.3.3','5.3.4','5.3.5','5.3.6')
}
Write-Json $OutputPath $artifact
Write-Output "Independent M0 review evidence captured at $OutputPath"
