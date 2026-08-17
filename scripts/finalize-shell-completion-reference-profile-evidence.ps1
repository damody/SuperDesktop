[CmdletBinding()]
param(
    [string]$Workspace,
    [Parameter(Mandatory)][string]$M0ReferenceProfileEvidence,
    [Parameter(Mandatory)][string]$InstallerEvidenceDirectory,
    [Parameter(Mandatory)][string]$RollbackRecord
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = Split-Path -Parent $PSScriptRoot }
$root = (Resolve-Path -LiteralPath $Workspace).Path
$m0Path = (Resolve-Path -LiteralPath $M0ReferenceProfileEvidence).Path
$installerRoot = (Resolve-Path -LiteralPath $InstallerEvidenceDirectory).Path
function Get-RepositoryRelativePath([string]$Path, [string]$Label) {
    $full = [IO.Path]::GetFullPath($Path)
    $prefix = $root.TrimEnd('\') + '\'
    if (-not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw "$Label must be stored inside the repository." }
    return $full.Substring($prefix.Length).Replace('\','/')
}
Get-RepositoryRelativePath $m0Path 'M0 reference-profile evidence' | Out-Null
Get-RepositoryRelativePath $installerRoot 'Installer evidence directory' | Out-Null
$candidatePath = Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence\release-candidate.json'
Import-Module (Join-Path $PSScriptRoot 'SuperDesktop.ReferenceProfile.psm1') -Force
$admission = Get-SuperDesktopReferenceProfileAdmission -Workspace $root -CandidatePath $candidatePath
$revision = [string]$admission.candidate_revision
$shortRevision = $revision.Substring(0, 8)

$m0 = Get-Content -Raw -Encoding utf8 -LiteralPath $m0Path | ConvertFrom-Json
if ($m0.schema -cne 'm0-reference-profile-gate/v1' -or $m0.status -cne 'passed' -or $m0.revision -cne $shortRevision) { throw 'M0 reference-profile evidence is invalid or stale.' }
if ($m0.host.build -ne 26200 -or $m0.host.ubr -ne 9168 -or $m0.host.explorerpatcher_version -cne '26100.8457.70.3' -or
    $m0.host.profile_fingerprint -cne $admission.profile_fingerprint) { throw 'M0 exact reference profile is required.' }
if ([string]::IsNullOrWhiteSpace($m0.operator.name) -or [string]::IsNullOrWhiteSpace($m0.operator.organization) -or
    [string]$m0.operator.name -like 'REPLACE_WITH_*' -or [string]$m0.operator.organization -like 'REPLACE_WITH_*') { throw 'M0 reference-profile operator is not attributable.' }
if ($m0.dispositions.'G-SHELL-TAKEOVER' -cne 'passed' -or $m0.dispositions.'G-GUARDIAN-RECOVERY' -cne 'passed') { throw 'M0 lifecycle gates are not passed.' }
if (-not $m0.forced_crash.production_path -or $m0.forced_crash.run_count -ne 10 -or $m0.forced_crash.max_elapsed_ms -gt 10000) { throw 'Guardian recovery matrix is incomplete or did not exercise the production Shell path.' }
foreach ($run in @($m0.forced_crash.runs)) {
    if (-not $run.production_shell_crashed -or -not $run.parent_terminal_observed -or -not $run.recovery_verified -or
        -not $run.work_area_owned_before_crash -or -not $run.work_area_baseline -or $run.guardian_pid -le 0 -or
        $run.explorer_pid -le 0 -or $run.unique_terminal_count -ne 1 -or $run.ready_elapsed_ms -gt 10000) {
        throw "Production guardian recovery run is incomplete: $($run.run)"
    }
}
if (-not $m0.preview.zero_registry_mutation -or -not $m0.normal_exit.registry_restored -or -not $m0.normal_exit.explorer_restored -or -not $m0.normal_exit.work_areas_restored) { throw 'Lifecycle baseline was not restored.' }

$requiredInstallerFiles = @(
    'host-DryRun.json','plan-DryRun.json','dry-run-non-mutation.json',
    'host-Enable.json','host-AfterReboot.json','host-Rollback.json',
    'plan-Enable.json','result-Enable.json','after-reboot.json','plan-Rollback.json','result-Rollback.json'
)
$installerDocuments = @{}
foreach ($name in $requiredInstallerFiles) {
    $path = Join-Path $installerRoot $name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing installer evidence: $name" }
    $installerDocuments[$name] = Get-Content -Raw -Encoding utf8 -LiteralPath $path | ConvertFrom-Json
}
$phaseHosts = [ordered]@{
    DryRun = $installerDocuments['host-DryRun.json']
    Enable = $installerDocuments['host-Enable.json']
    AfterReboot = $installerDocuments['host-AfterReboot.json']
    Rollback = $installerDocuments['host-Rollback.json']
}
Assert-SuperDesktopInstallerHostSet -Hosts $phaseHosts -Revision $revision -ProfileFingerprint $admission.profile_fingerprint
foreach ($name in @('host-DryRun.json','host-Enable.json','host-AfterReboot.json','host-Rollback.json')) {
    $host = $installerDocuments[$name]
    if ($host.build -ne 26200 -or $host.ubr -ne 9168 -or $host.explorerPatcherVersion -cne '26100.8457.70.3' -or
        $host.profileFingerprint -cne $admission.profile_fingerprint) { throw "Installer phase is not the exact reference profile: $name" }
    if ($host.revision -cne $revision) { throw "Installer phase revision drift: $name" }
    if ([string]::IsNullOrWhiteSpace($host.operator.name) -or [string]::IsNullOrWhiteSpace($host.operator.organization) -or
        [string]$host.operator.name -like 'REPLACE_WITH_*' -or [string]$host.operator.organization -like 'REPLACE_WITH_*') { throw "Installer phase operator is not attributable: $name" }
    foreach ($binaryName in @('shell-installer','superdesktop-app','superdesktop-guardian')) {
        $records = @($host.binaries | Where-Object name -CEQ $binaryName)
        if ($records.Count -ne 1 -or [string]$records[0].sha256 -notmatch '^[0-9a-f]{64}$') { throw "Installer phase binary manifest is invalid: $name/$binaryName" }
    }
}
$enableBoot = [DateTimeOffset]::Parse([string]$phaseHosts.Enable.boot.lastBootUpUtc).ToUniversalTime()
$afterRebootBoot = [DateTimeOffset]::Parse([string]$phaseHosts.AfterReboot.boot.lastBootUpUtc).ToUniversalTime()
if ($afterRebootBoot -le $enableBoot) { throw 'Installer evidence did not cross a real Windows boot boundary.' }
$hostBaseline = $installerDocuments['host-DryRun.json']
foreach ($name in @('host-Enable.json','host-AfterReboot.json','host-Rollback.json')) {
    if ($installerDocuments[$name].operator.name -cne $hostBaseline.operator.name -or
        $installerDocuments[$name].operator.organization -cne $hostBaseline.operator.organization -or
        $installerDocuments[$name].profileFingerprint -cne $hostBaseline.profileFingerprint) { throw "Installer operator or profile drift across phases: $name" }
    foreach ($binaryName in @('shell-installer','superdesktop-app','superdesktop-guardian')) {
        $baselineHash = [string](@($hostBaseline.binaries | Where-Object name -CEQ $binaryName)[0].sha256)
        $phaseHash = [string](@($installerDocuments[$name].binaries | Where-Object name -CEQ $binaryName)[0].sha256)
        if ($phaseHash -cne $baselineHash) { throw "Installer binary drift across reboot phases: $binaryName/$name" }
    }
}
$m0AppRecords = @($m0.binaries | Where-Object name -CEQ 'superdesktop-app')
$m0GuardianRecords = @($m0.binaries | Where-Object name -CEQ 'superdesktop-guardian')
if ($m0AppRecords.Count -ne 1 -or $m0GuardianRecords.Count -ne 1) { throw 'M0 lifecycle binary manifest is incomplete.' }
$m0AppHash = [string]$m0AppRecords[0].sha256
$m0GuardianHash = [string]$m0GuardianRecords[0].sha256
if ($m0AppHash.ToLowerInvariant() -cne [string](@($hostBaseline.binaries | Where-Object name -CEQ 'superdesktop-app')[0].sha256) -or
    $m0GuardianHash.ToLowerInvariant() -cne [string](@($hostBaseline.binaries | Where-Object name -CEQ 'superdesktop-guardian')[0].sha256)) {
    throw 'M0 lifecycle and installer phases did not use the same product binaries.'
}
$dryRunPlan = $installerDocuments['plan-DryRun.json']
$dryRunProof = $installerDocuments['dry-run-non-mutation.json']
if ($dryRunPlan.audit.disposition -cne 'dry_run' -or $dryRunProof.schema -cne 'shell-installer-dry-run-non-mutation/v1' -or
    $dryRunProof.revision -cne $revision -or -not $dryRunProof.shellUnchanged -or -not $dryRunProof.rollbackUnchanged) {
    throw 'Installer dry-run non-mutation proof is invalid.'
}
$enablePlan = $installerDocuments['plan-Enable.json'].plan
$enableResult = $installerDocuments['result-Enable.json'].audit
$afterReboot = $installerDocuments['after-reboot.json']
$rollbackPlan = $installerDocuments['plan-Rollback.json'].plan
$rollbackResult = $installerDocuments['result-Rollback.json'].audit
$rollbackPath = [IO.Path]::GetFullPath($RollbackRecord)
$dryRunPlanRecord = $dryRunPlan.plan
if ([IO.Path]::GetFullPath([string]$dryRunPlanRecord.rollback_record_path) -cne $rollbackPath -or
    [IO.Path]::GetFullPath([string]$dryRunProof.rollbackRecordPath) -cne $rollbackPath -or
    [IO.Path]::GetFullPath([string]$enablePlan.rollback_record_path) -cne $rollbackPath -or
    [IO.Path]::GetFullPath([string]$rollbackPlan.rollback_record_path) -cne $rollbackPath) { throw 'Installer plans are not bound to the supplied rollback metadata path.' }
$expectedRollbackTarget = "rollback_record:$rollbackPath"
if (@($dryRunPlan.audit.affected_targets | Where-Object { $_ -ceq $expectedRollbackTarget }).Count -ne 1 -or
    @($enableResult.affected_targets | Where-Object { $_ -ceq $expectedRollbackTarget }).Count -ne 1 -or
    @($rollbackResult.affected_targets | Where-Object { $_ -ceq $expectedRollbackTarget }).Count -ne 1) { throw 'Installer audit does not identify the exact rollback metadata target.' }
if ($dryRunProof.rollbackExistedBefore -or $dryRunProof.rollbackExistedAfter -or
    $hostBaseline.rollbackRecordExistedBefore -or $installerDocuments['host-Enable.json'].rollbackRecordExistedBefore -or
    -not $installerDocuments['host-AfterReboot.json'].rollbackRecordExistedBefore -or
    -not $installerDocuments['host-Rollback.json'].rollbackRecordExistedBefore) {
    throw 'Installer rollback metadata lifecycle is not a clean create/reboot/remove sequence.'
}
function Test-ObservationEqualsValue($Observation, $Value) {
    if ($null -eq $Value) { return -not [bool]$Observation.exists }
    return [bool]$Observation.exists -and [string]$Observation.value -ceq [string]$Value
}
if (-not (Test-ObservationEqualsValue $hostBaseline.shellBefore $dryRunPlanRecord.observed) -or
    -not (Test-ObservationEqualsValue $installerDocuments['host-Enable.json'].shellBefore $enablePlan.observed) -or
    -not (Test-ObservationEqualsValue $installerDocuments['host-AfterReboot.json'].shellBefore $enablePlan.desired) -or
    -not (Test-ObservationEqualsValue $installerDocuments['host-Rollback.json'].shellBefore $enablePlan.desired)) {
    throw 'Installer phase host observations do not match the exact plan/reboot lifecycle.'
}
if ($dryRunPlanRecord.command -cne 'enable' -or $dryRunPlan.audit.command -cne 'enable' -or
    $dryRunPlan.audit.fingerprint -cne $dryRunPlanRecord.fingerprint -or
    $dryRunPlan.audit.before -cne $dryRunPlanRecord.observed -or $dryRunPlan.audit.desired -cne $dryRunPlanRecord.desired) {
    throw 'Installer dry-run plan and audit are inconsistent.'
}
if ($enablePlan.command -cne 'enable' -or $enableResult.command -cne 'enable' -or
    $enableResult.fingerprint -cne $enablePlan.fingerprint -or $enableResult.disposition -cne 'applied' -or
    $enableResult.before -cne $enablePlan.observed -or $enableResult.desired -cne $enablePlan.desired -or
    $enableResult.after -cne $enablePlan.desired) { throw 'Installer enable transaction is not verified.' }
if ($afterReboot.shell -cne $enablePlan.desired) { throw 'Reboot did not retain the exact enabled Shell value.' }
if ($rollbackPlan.command -cne 'disable' -or $rollbackResult.command -cne 'disable' -or
    $rollbackResult.fingerprint -cne $rollbackPlan.fingerprint -or $rollbackPlan.observed -cne $enablePlan.desired -or
    $rollbackResult.disposition -cne 'applied' -or $rollbackPlan.desired -cne $enablePlan.observed -or
    $rollbackResult.before -cne $rollbackPlan.observed -or $rollbackResult.desired -cne $rollbackPlan.desired -or
    $rollbackResult.after -cne $enablePlan.observed) { throw 'Installer rollback did not restore the exact prior state.' }
if (Test-Path -LiteralPath $rollbackPath) { throw 'Rollback metadata remains after verified restore.' }

$sourceHashes = @(
    [ordered]@{ path=(Get-RepositoryRelativePath $m0Path 'M0 reference-profile evidence');sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $m0Path).Hash.ToLowerInvariant() }
)
foreach ($name in $requiredInstallerFiles) {
    $path = Join-Path $installerRoot $name
    $sourceHashes += [ordered]@{ path=(Get-RepositoryRelativePath $path "Installer evidence $name");sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant() }
}
$artifact = [ordered]@{
    schema_version = 1
    kind = 'reference-profile-lifecycle-installer'
    status = 'passed'
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    revision = $revision
    host = [ordered]@{ build=26200;ubr=9168;explorerpatcher_version='26100.8457.70.3';profile_fingerprint=$admission.profile_fingerprint;profile_sources=$admission.sources;session_id=$m0.host.session_id;interactive=$m0.host.interactive }
    operators = [ordered]@{ lifecycle=$m0.operator;installer=$hostBaseline.operator }
    lifecycle = [ordered]@{ preview_zero_mutation=$true;normal_exit_restored=$true;production_guardian_path=$true;forced_crash_runs=10;max_recovery_ms=[double]$m0.forced_crash.max_elapsed_ms }
    installer = [ordered]@{ reboot_verified=$true;enable_boot_utc=$enableBoot.ToString('o');after_reboot_boot_utc=$afterRebootBoot.ToString('o');exact_rollback_verified=$true;metadata_removed=$true;prior_shell=$enablePlan.observed;enabled_shell=$enablePlan.desired }
    source_hashes = $sourceHashes
    gates = [ordered]@{ 'G-SHELL-TAKEOVER'='passed';'G-GUARDIAN-RECOVERY'='passed';'G-INSTALL-ROLLBACK'='passed' }
}
$output = Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence\external\reference-profile-lifecycle-installer.json'
[IO.Directory]::CreateDirectory((Split-Path -Parent $output)) | Out-Null
[IO.File]::WriteAllText($output, (($artifact | ConvertTo-Json -Depth 20) + "`n"), [Text.UTF8Encoding]::new($false))
Write-Output "Reference-profile completion evidence finalized at $output"
