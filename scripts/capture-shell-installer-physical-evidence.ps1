[CmdletBinding()]
param(
    [string]$Workspace,
    [Parameter(Mandatory)]
    [ValidateSet('DryRun', 'Enable', 'AfterReboot', 'Rollback')]
    [string]$Phase,
    [Parameter(Mandatory)]
    [string]$Installer,
    [Parameter(Mandatory)]
    [string]$App,
    [Parameter(Mandatory)]
    [string]$Guardian,
    [Parameter(Mandatory)]
    [string]$RollbackRecord,
    [Parameter(Mandatory)]
    [string]$EvidenceDirectory,
    [string]$OperatorName,
    [string]$OperatorOrganization,
    [switch]$Apply,
    [switch]$ExplicitOptIn,
    [string]$ConfirmPlan
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = Split-Path -Parent $PSScriptRoot }
$workspacePath = (Resolve-Path -LiteralPath $Workspace).Path
$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$appPath = (Resolve-Path -LiteralPath $App).Path
$guardianPath = (Resolve-Path -LiteralPath $Guardian).Path
$installDirectory = Split-Path -Parent $appPath
$providerHostPath = (Resolve-Path -LiteralPath (Join-Path $installDirectory 'shell-provider-host.exe')).Path
$notificationHostPath = (Resolve-Path -LiteralPath (Join-Path $installDirectory 'notification-area-host.exe')).Path
$superExplorerPath = (Resolve-Path -LiteralPath (Join-Path $installDirectory 'SuperExplorer.exe')).Path
$rollbackPath = [IO.Path]::GetFullPath($RollbackRecord)
$evidencePath = [IO.Path]::GetFullPath($EvidenceDirectory)

$candidatePath = Join-Path $workspacePath 'openspec\changes\verify-superdesktop-shell-completion\evidence\release-candidate.json'
Import-Module (Join-Path $PSScriptRoot 'SuperDesktop.ReferenceProfile.psm1') -Force
$admission = Get-SuperDesktopReferenceProfileAdmission -Workspace $workspacePath -CandidatePath $candidatePath
$revision = [string]$admission.candidate_revision

function Read-ShellObservation {
    try {
        return [ordered]@{
            exists = $true
            value = [string](Get-ItemPropertyValue -LiteralPath 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Winlogon' -Name Shell -ErrorAction Stop)
        }
    } catch [System.Management.Automation.ItemNotFoundException] {
        return [ordered]@{ exists = $false; value = $null }
    } catch [System.Management.Automation.PSArgumentException] {
        return [ordered]@{ exists = $false; value = $null }
    }
}

$binaryRecords = @(
    [ordered]@{ name='shell-installer';path=$installerPath;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $installerPath).Hash.ToLowerInvariant() },
    [ordered]@{ name='superdesktop-app';path=$appPath;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $appPath).Hash.ToLowerInvariant() },
    [ordered]@{ name='superdesktop-guardian';path=$guardianPath;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $guardianPath).Hash.ToLowerInvariant() },
    [ordered]@{ name='shell-provider-host';path=$providerHostPath;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $providerHostPath).Hash.ToLowerInvariant() },
    [ordered]@{ name='notification-area-host';path=$notificationHostPath;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $notificationHostPath).Hash.ToLowerInvariant() },
    [ordered]@{ name='SuperExplorer';path=$superExplorerPath;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $superExplorerPath).Hash.ToLowerInvariant() }
)
$shellBefore = Read-ShellObservation
$rollbackExistedBefore = Test-Path -LiteralPath $rollbackPath -PathType Leaf

$os = $admission.observed
$hostRecord = [ordered]@{
    productName = $os.product
    build = [int]$os.build
    ubr = [int]$os.ubr
    explorerPatcherVersion = $os.explorerpatcher_version
    profileFingerprint = $admission.profile_fingerprint
    profileSources = $admission.sources
    phase = $Phase
    operator = [ordered]@{ name=$OperatorName;organization=$OperatorOrganization }
    revision = $revision
    binaries = $binaryRecords
    shellBefore = $shellBefore
    rollbackRecordPath = $rollbackPath
    rollbackRecordExistedBefore = $rollbackExistedBefore
    capturedAtUtc = [DateTime]::UtcNow.ToString('o')
}
[IO.Directory]::CreateDirectory($evidencePath) | Out-Null
$hostRecord | ConvertTo-Json | Set-Content -Encoding utf8 -LiteralPath (Join-Path $evidencePath "host-$Phase.json")

if ($Apply -and (-not $ExplicitOptIn -or [string]::IsNullOrWhiteSpace($ConfirmPlan))) {
    throw 'Mutation requires -Apply, -ExplicitOptIn, and an exact -ConfirmPlan fingerprint.'
}

if ($Phase -eq 'AfterReboot') {
    $shell = $null
    try {
        $shell = Get-ItemPropertyValue -LiteralPath 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Winlogon' -Name Shell
    } catch [System.Management.Automation.ItemNotFoundException] {
        $shell = $null
    } catch [System.Management.Automation.PSArgumentException] {
        $shell = $null
    }
    [ordered]@{
        shell = $shell
        explorerRunning = [bool](Get-Process -Name explorer -ErrorAction SilentlyContinue)
        capturedAtUtc = [DateTime]::UtcNow.ToString('o')
    } | ConvertTo-Json | Set-Content -Encoding utf8 -LiteralPath (Join-Path $evidencePath 'after-reboot.json')
    return
}

$command = if ($Phase -eq 'Rollback') { 'disable' } else { 'enable' }
$baseArguments = @(
    $command,
    '--app', $appPath,
    '--guardian', $guardianPath,
    '--rollback-record', $rollbackPath
)
$dryRunText = (& $installerPath @baseArguments | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw "Installer dry-run failed with exit code $LASTEXITCODE.`n$dryRunText" }
$dryRun = $dryRunText | ConvertFrom-Json
$dryRunText | Set-Content -Encoding utf8 -LiteralPath (Join-Path $evidencePath "plan-$Phase.json")

if ($Phase -eq 'DryRun') {
    $shellAfter = Read-ShellObservation
    $rollbackExistedAfter = Test-Path -LiteralPath $rollbackPath -PathType Leaf
    $shellUnchanged = (($shellBefore | ConvertTo-Json -Compress) -ceq ($shellAfter | ConvertTo-Json -Compress))
    $rollbackUnchanged = $rollbackExistedBefore -eq $rollbackExistedAfter
    if ($dryRun.audit.disposition -cne 'dry_run' -or -not $shellUnchanged -or -not $rollbackUnchanged) {
        throw 'Installer dry-run mutated Shell or rollback metadata state.'
    }
    [ordered]@{
        schema = 'shell-installer-dry-run-non-mutation/v1'
        revision = $revision
        shellBefore = $shellBefore
        shellAfter = $shellAfter
        shellUnchanged = $shellUnchanged
        rollbackRecordPath = $rollbackPath
        rollbackExistedBefore = $rollbackExistedBefore
        rollbackExistedAfter = $rollbackExistedAfter
        rollbackUnchanged = $rollbackUnchanged
        capturedAtUtc = [DateTime]::UtcNow.ToString('o')
    } | ConvertTo-Json -Depth 10 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $evidencePath 'dry-run-non-mutation.json')
    return
}
if (-not $Apply) { return }
if ($dryRun.plan.fingerprint -cne $ConfirmPlan) {
    throw "Plan fingerprint mismatch. Current plan is $($dryRun.plan.fingerprint)."
}

$applyArguments = $baseArguments + @('--apply', '--explicit-opt-in', '--confirm-plan', $ConfirmPlan)
$applyText = (& $installerPath @applyArguments | Out-String).Trim()
$exitCode = $LASTEXITCODE
$applyText | Set-Content -Encoding utf8 -LiteralPath (Join-Path $evidencePath "result-$Phase.json")
if ($exitCode -ne 0) { throw "Installer apply failed with exit code $exitCode.`n$applyText" }
