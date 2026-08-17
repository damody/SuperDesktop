[CmdletBinding()]
param([string]$RepositoryRoot)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
    $RepositoryRoot = Split-Path -Parent $PSScriptRoot
}
$root = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$rollupPath = Join-Path $root 'openspec\changes\complete-superdesktop-windows-shell\evidence\program-rollup.json'
$rollup = Get-Content -Raw -Encoding utf8 -LiteralPath $rollupPath | ConvertFrom-Json
$expected = @(
    'extend-superdesktop-shell-contracts',
    'add-superdesktop-desktop-file-operations',
    'add-superdesktop-shell-context-menu-host',
    'add-superdesktop-start-search',
    'add-superdesktop-taskbar-advanced-interactions',
    'add-superdesktop-notification-area-host',
    'add-superdesktop-virtual-desktops',
    'add-superdesktop-shell-installer',
    'verify-superdesktop-shell-completion'
)
$externalGateNames = @(
    'G-DPI-MONITOR-PHYSICAL',
    'G-GUARDIAN-RECOVERY',
    'G-INSTALL-ROLLBACK',
    'G-REVIEW',
    'G-SHELL-TAKEOVER'
)
if ($rollup.schema_version -ne 1 -or $rollup.archived -ne $false) { throw 'Invalid program envelope.' }
if ($rollup.ordered_changes.Count -ne $expected.Count) { throw 'Program child count mismatch.' }
$verification = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence\current-rollup.json') | ConvertFrom-Json
$localVerificationComplete = @($verification.gates.psobject.Properties | Where-Object {
    $_.Name -notin $externalGateNames -and $_.Value -cne 'passed'
}).Count -eq 0

Push-Location $root
try {
    $openSpec = (openspec list --json | Out-String) | ConvertFrom-Json
    foreach ($index in 0..($expected.Count - 1)) {
        $entry = $rollup.ordered_changes[$index]
        if ($entry.order -ne $index + 1 -or $entry.change -cne $expected[$index]) {
            throw "Dependency order mismatch at index $index."
        }
        $live = $openSpec.changes | Where-Object name -CEQ $entry.change
        if (@($live).Count -ne 1) { throw "OpenSpec child identity mismatch: $($entry.change)" }
        if ($live.completedTasks -ne $entry.tasks.complete -or $live.totalTasks -ne $entry.tasks.total) {
            throw "OpenSpec task count drift: $($entry.change)"
        }
        $expectedState = if ($live.completedTasks -eq $live.totalTasks) {
            'complete'
        } elseif ($entry.change -ceq 'verify-superdesktop-shell-completion' -and $localVerificationComplete) {
            'local_complete_external_pending'
        } else {
            'incomplete'
        }
        if ($entry.state -cne $expectedState) { throw "OpenSpec child state drift: $($entry.change)" }
        if (-not (Test-Path -LiteralPath (Join-Path $root ($entry.evidence -replace '/', '\')) -PathType Leaf)) {
            throw "Missing evidence: $($entry.evidence)"
        }
        git cat-file -e "$($entry.commit)^{commit}"
        if ($LASTEXITCODE -ne 0) { throw "Missing commit: $($entry.commit)" }
    }
} finally {
    Pop-Location
}

$implementationComplete = @($rollup.ordered_changes | Select-Object -First 8 | Where-Object state -cne 'complete').Count -eq 0
$expectedReleaseAllowed = $implementationComplete -and $localVerificationComplete -and [bool]$verification.decision.release_allowed
$expectedBlockers = @($verification.decision.blockers | ForEach-Object { (($_ -split ':')[0]).ToUpperInvariant() })
if (-not $implementationComplete) { $expectedBlockers += 'G-PROGRAM-IMPLEMENTATION' }
if (-not $localVerificationComplete) { $expectedBlockers += 'G-LOCAL-VERIFICATION' }
$expectedBlockers = @($expectedBlockers | Sort-Object -Unique)
if ($rollup.implementation_complete -ne $implementationComplete -or $rollup.local_verification_complete -ne $localVerificationComplete -or $rollup.release_allowed -ne $expectedReleaseAllowed) {
    throw 'Program derived disposition is inconsistent.'
}
if ((Compare-Object $expectedBlockers $rollup.release_blockers).Count -ne 0) { throw 'Program blocker set drift.' }
if ($verification.decision.release_allowed -ne ($verification.decision.disposition -ceq 'passed')) {
    throw 'Verification roll-up decision fields contradict each other.'
}
$strict = (openspec validate --all --strict --json | Out-String) | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $strict.summary.totals.failed -ne 0 -or
    $rollup.local_commands.openspec_all_strict -cne "$($strict.summary.totals.passed) passed, $($strict.summary.totals.failed) failed") {
    throw 'Strict OpenSpec validation summary drift.'
}

[ordered]@{
    result = 'passed'
    implementation_complete = $implementationComplete
    release_allowed = $expectedReleaseAllowed
    children = $expected.Count
    blockers = $expectedBlockers
} | ConvertTo-Json -Depth 4
