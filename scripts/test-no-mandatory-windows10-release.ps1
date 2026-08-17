[CmdletBinding()]
param([string]$Workspace)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = Split-Path -Parent $PSScriptRoot }
$Workspace = (Resolve-Path -LiteralPath $Workspace).Path

$obsoleteFiles = @(
    'scripts/capture-m0-windows10-evidence.ps1',
    'scripts/finalize-shell-completion-windows10-evidence.ps1',
    'openspec/changes/verify-superdesktop-shell-completion/evidence/external/windows10-lifecycle-installer.json'
)
foreach ($relative in $obsoleteFiles) {
    if (Test-Path -LiteralPath (Join-Path $Workspace ($relative -replace '/', '\'))) {
        throw "MANDATORY_WINDOWS10_OBSOLETE_FILE: $relative"
    }
}

$operationalRoots = @(
    'scripts',
    'openspec/changes/build-superdesktop-shell-foundation',
    'openspec/changes/verify-superdesktop-m0',
    'openspec/changes/verify-superdesktop-shell-completion',
    'openspec/changes/complete-superdesktop-windows-shell',
    'openspec/changes/add-superdesktop-shell-installer',
    'openspec/changes/add-superdesktop-shell-takeover-recovery',
    'openspec/changes/add-superdesktop-start-search',
    'openspec/changes/add-superdesktop-taskbar-advanced-interactions'
) | ForEach-Object { Join-Path $Workspace ($_ -replace '/', '\') }

$obsoleteIdentifiers = & rg -n -i `
    -g '!SuperDesktop.ReferenceProfile.psm1' `
    -g '!test-reference-profile-admission.ps1' `
    -g '!test-no-mandatory-windows10-release.ps1' `
    '19045|m0-windows10|windows10-lifecycle-installer|windows10-gate|windows_10_22h2|windows10Passed|expectedWindows10' `
    @operationalRoots
$obsoleteExit = $LASTEXITCODE
if ($obsoleteExit -eq 0 -and $obsoleteIdentifiers) {
    throw "MANDATORY_WINDOWS10_IDENTIFIER_REMAINS:`n$($obsoleteIdentifiers -join "`n")"
}
if ($obsoleteExit -notin @(0, 1)) { throw 'MANDATORY_WINDOWS10_IDENTIFIER_SCAN_FAILED' }

$mandatoryLanguage = & rg -n -i `
    -g '!test-no-mandatory-windows10-release.ps1' `
    'Windows 10.{0,100}(required|mandatory|compatibility target|evidence machine|external gate)|(?:required|mandatory).{0,100}Windows 10' `
    @operationalRoots
$mandatoryExit = $LASTEXITCODE
if ($mandatoryExit -eq 0 -and $mandatoryLanguage) {
    throw "MANDATORY_WINDOWS10_LANGUAGE_REMAINS:`n$($mandatoryLanguage -join "`n")"
}
if ($mandatoryExit -notin @(0, 1)) { throw 'MANDATORY_WINDOWS10_LANGUAGE_SCAN_FAILED' }

$claimFiles = @(
    'openspec/changes/build-superdesktop-shell-foundation/design.md',
    'openspec/changes/verify-superdesktop-m0/design.md',
    'openspec/changes/verify-superdesktop-shell-completion/design.md',
    'openspec/changes/complete-superdesktop-windows-shell/PROGRAM.md'
)
foreach ($relative in $claimFiles) {
    $text = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $Workspace ($relative -replace '/', '\'))
    if ($text -notmatch '(?i)Windows 10.{0,100}not.claim') {
        throw "WINDOWS10_NOT_CLAIMED_MISSING: $relative"
    }
}

[ordered]@{
    result = 'passed'
    obsolete_files_absent = $obsoleteFiles.Count
    obsolete_operational_identifiers = 0
    mandatory_release_statements = 0
    not_claimed_contracts = $claimFiles.Count
    archived_paths_scanned = $false
} | ConvertTo-Json
