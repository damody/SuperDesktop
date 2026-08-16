[CmdletBinding()]
param(
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
    [switch]$Apply,
    [switch]$ExplicitOptIn,
    [string]$ConfirmPlan
)

$ErrorActionPreference = 'Stop'
$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$appPath = (Resolve-Path -LiteralPath $App).Path
$guardianPath = (Resolve-Path -LiteralPath $Guardian).Path
$evidencePath = [IO.Path]::GetFullPath($EvidenceDirectory)
[IO.Directory]::CreateDirectory($evidencePath) | Out-Null

$os = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
$hostRecord = [ordered]@{
    productName = $os.ProductName
    displayVersion = $os.DisplayVersion
    build = [int]$os.CurrentBuildNumber
    ubr = [int]$os.UBR
    phase = $Phase
    capturedAtUtc = [DateTime]::UtcNow.ToString('o')
}
$hostRecord | ConvertTo-Json | Set-Content -Encoding utf8 -LiteralPath (Join-Path $evidencePath "host-$Phase.json")

if ($Apply -and $hostRecord.build -ne 19045) {
    throw 'Physical mutation evidence is admitted only on the Windows 10 22H2 build 19045 test host.'
}
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
    '--rollback-record', [IO.Path]::GetFullPath($RollbackRecord)
)
$dryRunText = (& $installerPath @baseArguments | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw "Installer dry-run failed with exit code $LASTEXITCODE.`n$dryRunText" }
$dryRun = $dryRunText | ConvertFrom-Json
$dryRunText | Set-Content -Encoding utf8 -LiteralPath (Join-Path $evidencePath "plan-$Phase.json")

if (-not $Apply -or $Phase -eq 'DryRun') { return }
if ($dryRun.plan.fingerprint -cne $ConfirmPlan) {
    throw "Plan fingerprint mismatch. Current plan is $($dryRun.plan.fingerprint)."
}

$applyArguments = $baseArguments + @('--apply', '--explicit-opt-in', '--confirm-plan', $ConfirmPlan)
$applyText = (& $installerPath @applyArguments | Out-String).Trim()
$exitCode = $LASTEXITCODE
$applyText | Set-Content -Encoding utf8 -LiteralPath (Join-Path $evidencePath "result-$Phase.json")
if ($exitCode -ne 0) { throw "Installer apply failed with exit code $exitCode.`n$applyText" }
