[CmdletBinding()]
param([string]$Workspace)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = Split-Path -Parent $PSScriptRoot }
$Workspace = (Resolve-Path -LiteralPath $Workspace).Path
Import-Module (Join-Path $PSScriptRoot 'SuperDesktop.ReferenceProfile.psm1') -Force

function Copy-Object($Value) {
    return ($Value | ConvertTo-Json -Depth 30 | ConvertFrom-Json)
}

function Assert-Rejected([string]$Name, [scriptblock]$Action, [string]$Diagnostic) {
    try {
        & $Action
    } catch {
        if ($_.Exception.Message -notmatch "^$([regex]::Escape($Diagnostic))") {
            throw "FIXTURE_WRONG_DIAGNOSTIC: $Name expected $Diagnostic observed $($_.Exception.Message)"
        }
        return
    }
    throw "FIXTURE_UNEXPECTED_PASS: $Name"
}

$bound = Get-ReferenceProfileContract -Workspace $Workspace
$contract = $bound.contract
$expected = [ordered]@{
    build = [int]$contract.os_session_display_monitor.values.os.build
    ubr = [int]$contract.os_session_display_monitor.values.os.ubr
    explorerpatcher_version = [string]$contract.explorerpatcher.expected_version
    binaries = @($contract.explorerpatcher.binaries | ForEach-Object {
        [ordered]@{
            path = [string]$_.path
            length = [long]$_.length
            file_version = [string]$_.file_version
            product_version = [string]$_.product_version
            sha256 = ([string]$_.sha256).ToLowerInvariant()
        }
    })
}
$baseline = [ordered]@{
    product = 'Microsoft Windows 11 Pro'
    build = $expected.build
    ubr = $expected.ubr
    product_type = 1
    interactive = $true
    session_id = 1
    explorerpatcher_version = $expected.explorerpatcher_version
    binaries = Copy-Object $expected.binaries
}
Assert-ReferenceProfileValues -Expected $expected -Observed $baseline

$cases = @(
    @{ name='product';diagnostic='REFERENCE_OS_PRODUCT_DRIFT';mutate={ param($v) $v.product='Windows Server' } },
    @{ name='build';diagnostic='REFERENCE_OS_BUILD_DRIFT';mutate={ param($v) $v.build++ } },
    @{ name='ubr';diagnostic='REFERENCE_OS_UBR_DRIFT';mutate={ param($v) $v.ubr++ } },
    @{ name='interactive';diagnostic='REFERENCE_SESSION_UNSUPPORTED';mutate={ param($v) $v.interactive=$false } },
    @{ name='session';diagnostic='REFERENCE_SESSION_UNSUPPORTED';mutate={ param($v) $v.session_id=0 } },
    @{ name='product-type';diagnostic='REFERENCE_SESSION_UNSUPPORTED';mutate={ param($v) $v.product_type=3 } },
    @{ name='explorerpatcher-version';diagnostic='REFERENCE_EXPLORERPATCHER_VERSION_DRIFT';mutate={ param($v) $v.explorerpatcher_version='0.0.0.0' } },
    @{ name='binary-count';diagnostic='REFERENCE_BINARY_SET_DRIFT';mutate={ param($v) $v.binaries=@($v.binaries | Select-Object -First 2) } },
    @{ name='binary-path';diagnostic='REFERENCE_BINARY_DRIFT: 0';mutate={ param($v) $v.binaries[0].path='C:\wrong.dll' } },
    @{ name='binary-length';diagnostic='REFERENCE_BINARY_DRIFT: 0';mutate={ param($v) $v.binaries[0].length++ } },
    @{ name='binary-file-version';diagnostic='REFERENCE_BINARY_DRIFT: 0';mutate={ param($v) $v.binaries[0].file_version='0.0.0.0' } },
    @{ name='binary-product-version';diagnostic='REFERENCE_BINARY_DRIFT: 0';mutate={ param($v) $v.binaries[0].product_version='0.0.0.0' } },
    @{ name='binary-hash';diagnostic='REFERENCE_BINARY_DRIFT: 0';mutate={ param($v) $v.binaries[0].sha256=('0' * 64) } }
)
foreach ($case in $cases) {
    $fixture = Copy-Object $baseline
    & $case.mutate $fixture
    Assert-Rejected $case.name { Assert-ReferenceProfileValues -Expected $expected -Observed $fixture } $case.diagnostic
}

Assert-SuperDesktopExternalEvidenceKind -Kind 'reference-profile-lifecycle-installer'
Assert-Rejected 'obsolete-kind' { Assert-SuperDesktopExternalEvidenceKind -Kind 'windows10-lifecycle-installer' } 'REFERENCE_OBSOLETE_WINDOWS10_KIND'
Assert-Rejected 'unknown-kind' { Assert-SuperDesktopExternalEvidenceKind -Kind 'other' } 'REFERENCE_EXTERNAL_KIND_INVALID'
Assert-Rejected 'missing-candidate' { Assert-ReleaseCandidateLineage -Workspace $Workspace -Revision ('0' * 40) } 'REFERENCE_CANDIDATE_MISSING'
Assert-Rejected 'invalid-candidate' { Assert-ReleaseCandidateLineage -Workspace $Workspace -Revision 'short' } 'REFERENCE_CANDIDATE_INVALID'

$rollupSchemaPath = Join-Path $Workspace 'openspec/changes/verify-superdesktop-shell-completion/evidence/shell-completion-rollup.schema.json'
$rollupSchema = Get-Content -Raw -Encoding utf8 -LiteralPath $rollupSchemaPath | ConvertFrom-Json
$externalKinds = @($rollupSchema.properties.external_sources.propertyNames.enum)
$definitionKinds = @($rollupSchema.'$defs'.externalSource.properties.kind.enum)
foreach ($kinds in @($externalKinds, $definitionKinds)) {
    if ($kinds -notcontains 'reference-profile-lifecycle-installer' -or $kinds -contains 'windows10-lifecycle-installer') {
        throw 'REFERENCE_ROLLUP_SCHEMA_KIND_MIGRATION_INCOMPLETE'
    }
}

$revision = '1' * 40
$profileFingerprint = 'sha256:' + ('2' * 64)
$baselineHost = [ordered]@{
    revision = $revision
    build = 26200
    ubr = 9168
    explorerPatcherVersion = '26100.8457.70.3'
    profileFingerprint = $profileFingerprint
    operator = [ordered]@{ name='Fixture Operator';organization='Fixture Organization' }
    rollbackRecordPath = 'C:\fixture\rollback.json'
    binaries = @(1..6 | ForEach-Object { [ordered]@{ name="binary-$_";sha256=([string]$_ * 64) } })
}
function New-HostSet {
    $set = [ordered]@{}
    foreach ($phase in @('DryRun','Enable','AfterReboot','Rollback')) { $set[$phase] = Copy-Object $baselineHost }
    return $set
}
$hostSet = New-HostSet
Assert-SuperDesktopInstallerHostSet -Hosts $hostSet -Revision $revision -ProfileFingerprint $profileFingerprint
$hostSet = New-HostSet; $hostSet.Remove('Rollback')
Assert-Rejected 'phase-missing' { Assert-SuperDesktopInstallerHostSet -Hosts $hostSet -Revision $revision -ProfileFingerprint $profileFingerprint } 'REFERENCE_INSTALLER_PHASE_MISSING: Rollback'
$hostSet = New-HostSet; $hostSet['Enable'].revision = '3' * 40
Assert-Rejected 'phase-revision' { Assert-SuperDesktopInstallerHostSet -Hosts $hostSet -Revision $revision -ProfileFingerprint $profileFingerprint } 'REFERENCE_INSTALLER_REVISION_DRIFT: Enable'
$hostSet = New-HostSet; $hostSet['AfterReboot'].profileFingerprint = 'sha256:' + ('4' * 64)
Assert-Rejected 'phase-profile' { Assert-SuperDesktopInstallerHostSet -Hosts $hostSet -Revision $revision -ProfileFingerprint $profileFingerprint } 'REFERENCE_INSTALLER_PROFILE_DRIFT: AfterReboot'
$hostSet = New-HostSet; $hostSet['Rollback'].operator.name = 'Other Operator'
Assert-Rejected 'phase-operator' { Assert-SuperDesktopInstallerHostSet -Hosts $hostSet -Revision $revision -ProfileFingerprint $profileFingerprint } 'REFERENCE_INSTALLER_OPERATOR_DRIFT: Rollback'
$hostSet = New-HostSet; $hostSet['Enable'].rollbackRecordPath = 'C:\fixture\other.json'
Assert-Rejected 'phase-rollback-path' { Assert-SuperDesktopInstallerHostSet -Hosts $hostSet -Revision $revision -ProfileFingerprint $profileFingerprint } 'REFERENCE_INSTALLER_ROLLBACK_PATH_DRIFT: Enable'
$hostSet = New-HostSet; $hostSet['AfterReboot'].binaries[0].sha256 = '5' * 64
Assert-Rejected 'phase-binary' { Assert-SuperDesktopInstallerHostSet -Hosts $hostSet -Revision $revision -ProfileFingerprint $profileFingerprint } 'REFERENCE_INSTALLER_BINARY_DRIFT: AfterReboot/binary-1'

$buildRoot = [IO.Path]::GetFullPath((Join-Path $Workspace 'build'))
$fixtureRoot = [IO.Path]::GetFullPath((Join-Path $buildRoot "reference-profile-admission-fixture-$PID"))
if (-not $fixtureRoot.StartsWith($buildRoot.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'FIXTURE_PATH_ESCAPE'
}
if (Test-Path -LiteralPath $fixtureRoot) { Remove-Item -LiteralPath $fixtureRoot -Recurse -Force }
try {
    $fixtureChange = Join-Path $fixtureRoot 'openspec/changes/validate-superdesktop-windows-platform'
    $fixtureArtifacts = Join-Path $fixtureChange 'evidence/artifacts/1.1'
    $fixtureScripts = Join-Path $fixtureChange 'scripts'
    New-Item -ItemType Directory -Force -Path $fixtureArtifacts, $fixtureScripts | Out-Null
    Copy-Item -LiteralPath $bound.paths.contract -Destination (Join-Path $fixtureArtifacts 'frozen-profile-contract.json')
    Copy-Item -LiteralPath $bound.paths.settings -Destination (Join-Path $fixtureArtifacts 'explorerpatcher-profile.json')
    Copy-Item -LiteralPath $bound.paths.reference_image -Destination (Join-Path $fixtureArtifacts 'reference-taskbar.jpg')
    Copy-Item -LiteralPath $bound.paths.allowlist -Destination (Join-Path $fixtureScripts 'profile-allowlist.json')
    Add-Content -Encoding utf8 -LiteralPath (Join-Path $fixtureArtifacts 'explorerpatcher-profile.json') -Value 'drift'
    Assert-Rejected 'referenced-source-hash' { Get-ReferenceProfileContract -Workspace $fixtureRoot | Out-Null } 'REFERENCE_HASH_DRIFT: settings'
} finally {
    if (Test-Path -LiteralPath $fixtureRoot) { Remove-Item -LiteralPath $fixtureRoot -Recurse -Force }
}

[ordered]@{
    result = 'passed'
    mutation_performed = $false
    negative_fixtures = $cases.Count + 12
    obsolete_windows10_rejected = $true
    candidate_failures_rejected = $true
    referenced_hash_drift_rejected = $true
    installer_phase_identity_drift_rejected = $true
    rollup_schema_obsolete_kind_rejected = $true
} | ConvertTo-Json
