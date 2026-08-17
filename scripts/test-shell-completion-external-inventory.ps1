[CmdletBinding()]
param([string]$Workspace)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = Split-Path -Parent $PSScriptRoot
}
$Workspace = (Resolve-Path -LiteralPath $Workspace).Path
$collector = Join-Path $Workspace 'scripts\collect-shell-completion-evidence.ps1'
$externalRoot = Join-Path $Workspace 'openspec\changes\verify-superdesktop-shell-completion\evidence\external'
$buildRoot = Join-Path $Workspace 'build'
$outputPath = Join-Path $buildRoot "shell-completion-incremental-$PID.json"
$unexpectedPath = Join-Path $externalRoot "unexpected-fixture-$PID.json"

New-Item -ItemType Directory -Force -Path $buildRoot | Out-Null
try {
    & $collector -RepositoryRoot $Workspace -ExternalEvidenceDirectory $externalRoot -OutputPath $outputPath
    $rollup = Get-Content -Raw -Encoding utf8 -LiteralPath $outputPath | ConvertFrom-Json
    if (@($rollup.external_sources.psobject.Properties).Count -ne 0) { throw 'EMPTY_EXTERNAL_INVENTORY_EMITTED_SOURCES' }
    $expectedPending = @(
        'g-dpi-monitor-physical:external_pending',
        'g-guardian-recovery:external_pending',
        'g-install-rollback:external_pending',
        'g-review:external_pending',
        'g-shell-takeover:external_pending'
    )
    if ($rollup.decision.release_allowed -or @($rollup.decision.blockers).Count -ne $expectedPending.Count -or
        @($expectedPending | Where-Object { $_ -notin $rollup.decision.blockers }).Count -ne 0) {
        throw 'EMPTY_EXTERNAL_INVENTORY_DID_NOT_FAIL_CLOSED'
    }

    [IO.File]::WriteAllText($unexpectedPath, "{}`r`n", [Text.UTF8Encoding]::new($false))
    $unknownRejected = $false
    try {
        & $collector -RepositoryRoot $Workspace -ExternalEvidenceDirectory $externalRoot | Out-Null
    } catch {
        if ($_.Exception.Message -match "^Unexpected external evidence: $([regex]::Escape([IO.Path]::GetFileName($unexpectedPath)))$") {
            $unknownRejected = $true
        } else {
            throw
        }
    }
    if (-not $unknownRejected) {
        throw 'UNKNOWN_EXTERNAL_INVENTORY_NOT_REJECTED'
    }

    [ordered]@{
        result = 'passed'
        mutation_performed = $false
        empty_inventory_fail_closed = $true
        unknown_json_rejected = $true
        pending_gate_count = $expectedPending.Count
    } | ConvertTo-Json
} finally {
    if (Test-Path -LiteralPath $unexpectedPath) { Remove-Item -LiteralPath $unexpectedPath -Force }
    if (Test-Path -LiteralPath $outputPath) { Remove-Item -LiteralPath $outputPath -Force }
}
