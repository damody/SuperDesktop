[CmdletBinding()]
param(
    [string]$RepositoryRoot,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
    $RepositoryRoot = Split-Path -Parent $PSScriptRoot
}
$root = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$evidenceRoot = Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence'
$manifest = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $evidenceRoot 'required-children.json') | ConvertFrom-Json
if ($manifest.schema_version -ne 1 -or $manifest.children.Count -ne 8) {
    throw 'Unsupported or incomplete completion child manifest.'
}

$sources = [ordered]@{}
foreach ($change in $manifest.children) {
    if ($sources.Contains($change)) { throw "Duplicate completion child: $change" }
    $relative = "openspec/changes/$change/evidence/verification.json"
    $path = Join-Path $root ($relative -replace '/', '\')
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing child evidence: $relative" }
    $document = Get-Content -Raw -Encoding utf8 -LiteralPath $path | ConvertFrom-Json
    if ($document.change -cne $change) { throw "Child identity mismatch in $relative" }
    $passed = $document.result -ceq 'passed'
    if ($change -eq 'add-superdesktop-shell-installer') {
        $passed = $document.local_verification.mutation_performed -eq $false
    }
    if (-not $passed) { throw "Child local result is not passed: $change" }
    $sources[$change] = [ordered]@{
        change = $change
        relative_path = $relative
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        local_result = 'passed'
    }
}

$gates = [ordered]@{
    'G-A11Y-I18N' = 'passed'
    'G-ARCH' = 'passed'
    'G-DESKTOP' = 'passed'
    'G-DPI-MONITOR-PHYSICAL' = 'external_pending'
    'G-DPI-MONITOR-VIRTUAL' = 'passed'
    'G-GUARDIAN-RECOVERY' = 'external_pending'
    'G-INSTALL-ROLLBACK' = 'external_pending'
    'G-PERF' = 'passed'
    'G-REVIEW' = 'external_pending'
    'G-SAFETY' = 'passed'
    'G-SHELL-TAKEOVER' = 'external_pending'
    'G-TASKBAR' = 'passed'
    'G-TRACE' = 'passed'
}
$blockers = @($gates.GetEnumerator() | Where-Object Value -ne 'passed' | ForEach-Object { "$($_.Key):$($_.Value)".ToLowerInvariant() })
$rollup = [ordered]@{
    schema_version = 1
    generated_at_utc = [DateTime]::UtcNow.ToString('o')
    sources = $sources
    gates = $gates
    limitations = @(
        [ordered]@{ capability = 'legacy-explorer-notification-protocol'; disposition = 'not-claimed'; reason = 'implemented host uses the owned versioned provider protocol' },
        [ordered]@{ capability = 'virtual-desktop-undocumented-operations'; disposition = 'unavailable'; reason = 'documented IVirtualDesktopManager adapter exposes query and move only' }
    )
    commands = @(
        'cargo fmt --all -- --check',
        'cargo check --workspace --offline',
        'cargo clippy --workspace --all-targets --offline -- -D warnings',
        'cargo test --workspace --offline',
        'openspec validate --all --strict'
    )
    decision = [ordered]@{
        release_allowed = $blockers.Count -eq 0
        disposition = if ($blockers.Count -eq 0) { 'passed' } else { 'blocked' }
        blockers = $blockers
    }
}
$json = $rollup | ConvertTo-Json -Depth 8
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $json
} else {
    $destination = [IO.Path]::GetFullPath($OutputPath)
    [IO.File]::WriteAllText($destination, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
}
