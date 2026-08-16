[CmdletBinding()]
param(
    [string]$Workspace,
    [Parameter(Mandatory)][string]$M0Windows10Evidence,
    [Parameter(Mandatory)][string]$InstallerEvidenceDirectory,
    [Parameter(Mandatory)][string]$RollbackRecord
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = Split-Path -Parent $PSScriptRoot }
$root = (Resolve-Path -LiteralPath $Workspace).Path
$m0Path = (Resolve-Path -LiteralPath $M0Windows10Evidence).Path
$installerRoot = (Resolve-Path -LiteralPath $InstallerEvidenceDirectory).Path
$revision = (& git -C $root rev-parse HEAD).Trim()
$shortRevision = (& git -C $root rev-parse --short=8 HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $revision -notmatch '^[0-9a-f]{40}$') { throw 'Unable to bind current Git revision.' }

$m0 = Get-Content -Raw -Encoding utf8 -LiteralPath $m0Path | ConvertFrom-Json
if ($m0.schema -cne 'm0-windows10-gate/v1' -or $m0.status -cne 'passed' -or $m0.revision -cne $shortRevision) { throw 'M0 Windows 10 evidence is invalid or stale.' }
if ($m0.host.build -ne 19045 -or $m0.host.display_version -cne '22H2') { throw 'Windows 10 build 19045 22H2 evidence is required.' }
if ($m0.dispositions.'G-SHELL-TAKEOVER' -cne 'passed' -or $m0.dispositions.'G-GUARDIAN-RECOVERY' -cne 'passed') { throw 'M0 lifecycle gates are not passed.' }
if ($m0.forced_crash.run_count -ne 10 -or $m0.forced_crash.max_elapsed_ms -gt 10000) { throw 'Guardian recovery matrix is incomplete.' }
if (-not $m0.preview.zero_registry_mutation -or -not $m0.normal_exit.registry_restored -or -not $m0.normal_exit.explorer_restored -or -not $m0.normal_exit.work_areas_restored) { throw 'Lifecycle baseline was not restored.' }

$requiredInstallerFiles = @('host-Enable.json','host-AfterReboot.json','host-Rollback.json','plan-Enable.json','result-Enable.json','after-reboot.json','plan-Rollback.json','result-Rollback.json')
$installerDocuments = @{}
foreach ($name in $requiredInstallerFiles) {
    $path = Join-Path $installerRoot $name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing installer evidence: $name" }
    $installerDocuments[$name] = Get-Content -Raw -Encoding utf8 -LiteralPath $path | ConvertFrom-Json
}
foreach ($name in @('host-Enable.json','host-AfterReboot.json','host-Rollback.json')) {
    if ($installerDocuments[$name].build -ne 19045 -or $installerDocuments[$name].displayVersion -cne '22H2') { throw "Installer phase is not Windows 10 22H2: $name" }
}
$enablePlan = $installerDocuments['plan-Enable.json'].plan
$enableResult = $installerDocuments['result-Enable.json'].audit
$afterReboot = $installerDocuments['after-reboot.json']
$rollbackPlan = $installerDocuments['plan-Rollback.json'].plan
$rollbackResult = $installerDocuments['result-Rollback.json'].audit
if ($enableResult.disposition -cne 'applied' -or $enableResult.after -cne $enablePlan.desired) { throw 'Installer enable transaction is not verified.' }
if ($afterReboot.shell -cne $enablePlan.desired) { throw 'Reboot did not retain the exact enabled Shell value.' }
if ($rollbackResult.disposition -cne 'applied' -or $rollbackPlan.desired -cne $enablePlan.observed -or $rollbackResult.after -cne $enablePlan.observed) { throw 'Installer rollback did not restore the exact prior state.' }
if (Test-Path -LiteralPath ([IO.Path]::GetFullPath($RollbackRecord))) { throw 'Rollback metadata remains after verified restore.' }

$sourceHashes = @(
    [ordered]@{ path=$m0Path;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $m0Path).Hash.ToLowerInvariant() }
)
foreach ($name in $requiredInstallerFiles) {
    $path = Join-Path $installerRoot $name
    $sourceHashes += [ordered]@{ path=$path;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant() }
}
$artifact = [ordered]@{
    schema_version = 1
    kind = 'windows10-lifecycle-installer'
    status = 'passed'
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    revision = $revision
    host = [ordered]@{ build=19045;display_version='22H2';session_id=$m0.host.session_id;interactive=$m0.host.interactive }
    lifecycle = [ordered]@{ preview_zero_mutation=$true;normal_exit_restored=$true;forced_crash_runs=10;max_recovery_ms=[double]$m0.forced_crash.max_elapsed_ms }
    installer = [ordered]@{ reboot_verified=$true;exact_rollback_verified=$true;metadata_removed=$true;prior_shell=$enablePlan.observed;enabled_shell=$enablePlan.desired }
    source_hashes = $sourceHashes
    gates = [ordered]@{ 'G-SHELL-TAKEOVER'='passed';'G-GUARDIAN-RECOVERY'='passed';'G-INSTALL-ROLLBACK'='passed' }
}
$output = Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence\external\windows10-lifecycle-installer.json'
[IO.Directory]::CreateDirectory((Split-Path -Parent $output)) | Out-Null
[IO.File]::WriteAllText($output, (($artifact | ConvertTo-Json -Depth 20) + "`n"), [Text.UTF8Encoding]::new($false))
Write-Output "Windows 10 completion evidence finalized at $output"
