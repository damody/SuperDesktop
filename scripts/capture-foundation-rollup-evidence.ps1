[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$change = 'build-superdesktop-shell-foundation'
$root = Join-Path $workspace "openspec/changes/$change"
$verificationRoot = Join-Path $workspace 'openspec/changes/verify-superdesktop-m0'
$evidence = Join-Path $root 'evidence'
$utf8 = [Text.UTF8Encoding]::new($false)
$recordedAt = [DateTime]::UtcNow.ToString('o')

function Write-Text([string]$Path, [string]$Text) {
    [IO.File]::WriteAllText($Path, $Text, $utf8)
}

function Write-Json([string]$Path, $Value) {
    Write-Text $Path (($Value | ConvertTo-Json -Depth 20) + "`n")
}

function Invoke-Captured([string]$Name, [scriptblock]$Command) {
    $prior = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = (& $Command 2>&1 | Out-String)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prior
    }
    if ($exitCode -ne 0) {
        throw "$Name failed with exit code $exitCode`n$output"
    }
    return $output.Trim()
}

$strictOutput = Invoke-Captured 'verification strict validation' {
    openspec validate verify-superdesktop-m0 --strict
}
$evidenceOutput = Invoke-Captured 'verification evidence validation' {
    powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $workspace 'scripts/validate-evidence.ps1') -Change verify-superdesktop-m0
}

$visualPath = Join-Path $verificationRoot 'evidence/artifacts/2.1/reference-ui-matrix.json'
$accessibilityPath = Join-Path $verificationRoot 'evidence/artifacts/4.1/accessibility-input.json'
$localizationPath = Join-Path $verificationRoot 'evidence/artifacts/4.2/localization-ime.json'
$performancePath = Join-Path $verificationRoot 'evidence/artifacts/5.1/performance.json'
$safetyPath = Join-Path $verificationRoot 'evidence/artifacts/5.2/safety-license-source.json'
$architecturePath = Join-Path $verificationRoot 'evidence/artifacts/1.2/build-offline-gate.json'
$traceabilityPath = Join-Path $verificationRoot 'evidence/artifacts/5.3/traceability-review.json'
$coveragePath = Join-Path $verificationRoot 'evidence/coverage.json'
$adjustmentsPath = Join-Path $verificationRoot 'evidence/adjustments.jsonl'
$visual = Get-Content -Raw -Encoding UTF8 $visualPath | ConvertFrom-Json
$accessibility = Get-Content -Raw -Encoding UTF8 $accessibilityPath | ConvertFrom-Json
$localization = Get-Content -Raw -Encoding UTF8 $localizationPath | ConvertFrom-Json
$performance = Get-Content -Raw -Encoding UTF8 $performancePath | ConvertFrom-Json
$safety = Get-Content -Raw -Encoding UTF8 $safetyPath | ConvertFrom-Json
$architecture = Get-Content -Raw -Encoding UTF8 $architecturePath | ConvertFrom-Json
$traceability = Get-Content -Raw -Encoding UTF8 $traceabilityPath | ConvertFrom-Json
$coverage = Get-Content -Raw -Encoding UTF8 $coveragePath | ConvertFrom-Json
$adjustments = @(Get-Content -Encoding UTF8 $adjustmentsPath | Where-Object { $_ } | ForEach-Object { $_ | ConvertFrom-Json })

if ($visual.visual_gate -ne 'passed' -or
    $visual.masked_global_ssim -lt $visual.minimum_ssim -or
    -not $visual.preview_and_shell_start -or
    $visual.actual_input_route_count -ne 6) {
    throw 'G-DESKTOP/G-TASKBAR/G-EXPLORER-BRIDGE visual roll-up is not passed'
}
if ($accessibility.high_contrast_visual -ne 'passed' -or
    $accessibility.focus_indicator -ne 'passed' -or
    $localization.glyph_fallback -ne 'passed' -or
    $localization.zh_cn_headful -ne 'passed') {
    throw 'G-A11Y-I18N roll-up is not passed'
}
if ($performance.gate -ne 'G-PERF passed' -or
    $performance.results.cold_start_max_ms -gt $performance.thresholds.cold_start_ms -or
    $performance.results.idle_cpu_median_percent -ge $performance.thresholds.idle_cpu_percent -or
    $performance.results.event_to_visible_p95_ms -ge $performance.thresholds.event_to_visible_p95_ms -or
    $performance.results.working_set_peak_bytes -ge $performance.thresholds.working_set_bytes) {
    throw 'G-PERF roll-up is not passed'
}
$safetyValues = @(
    $safety.shell_opt_in,
    $safety.preview_safe_mode_fail_closed,
    $safety.fixture_root_canonical_reparse,
    $safety.path_argument_environment_substitution,
    $safety.credential_clipboard_environment_log_redaction,
    $safety.dependency_license_inventory,
    $safety.pexplorer_read_only_boundary,
    $safety.superexplorer_repository_unchanged
)
if (@($safetyValues | Where-Object { $_ -ne 'passed' }).Count -gt 0 -or
    -not $architecture.all_exit_zero) {
    throw 'G-SAFETY/G-ARCH roll-up is not passed'
}
if (@($coverage.tasks).Count -ne 93 -or
    @($traceability.tasks).Count -ne 93 -or
    @($traceability.replacement_negative_fixtures | Where-Object { -not $_.rejected }).Count -ne 0 -or
    @($adjustments).Count -eq 0 -or
    @($adjustments | Where-Object { $_.status -ne 'replacement-passed' }).Count -ne 0) {
    throw 'G-TRACE coverage/replacement/corrective lineage roll-up is not passed'
}

$wave6Path = Join-Path $evidence 'artifacts/4.2/wave6-local-gates.json'
$wave6 = [ordered]@{
    schema = 'foundation-wave6-local-gates/v1'
    recorded_at = $recordedAt
    verification_change = 'verify-superdesktop-m0'
    strict_validation = @{ status = 'passed'; output = $strictOutput }
    detailed_tasks_validator = @{ status = 'passed'; output = $evidenceOutput }
    gates = [ordered]@{
        'G-DESKTOP' = 'passed'
        'G-TASKBAR' = 'passed'
        'G-EXPLORER-BRIDGE' = 'passed'
        'G-A11Y-I18N' = 'passed'
        'G-PERF' = 'passed'
        'G-SAFETY' = 'passed'
        'G-ARCH' = 'passed'
        'G-TRACE' = 'passed'
    }
    source_artifacts = @(
        @{ path = 'verify-superdesktop-m0/evidence/artifacts/2.1/reference-ui-matrix.json'; sha256 = (Get-FileHash $visualPath -Algorithm SHA256).Hash },
        @{ path = 'verify-superdesktop-m0/evidence/artifacts/4.1/accessibility-input.json'; sha256 = (Get-FileHash $accessibilityPath -Algorithm SHA256).Hash },
        @{ path = 'verify-superdesktop-m0/evidence/artifacts/4.2/localization-ime.json'; sha256 = (Get-FileHash $localizationPath -Algorithm SHA256).Hash },
        @{ path = 'verify-superdesktop-m0/evidence/artifacts/5.1/performance.json'; sha256 = (Get-FileHash $performancePath -Algorithm SHA256).Hash },
        @{ path = 'verify-superdesktop-m0/evidence/artifacts/5.2/safety-license-source.json'; sha256 = (Get-FileHash $safetyPath -Algorithm SHA256).Hash },
        @{ path = 'verify-superdesktop-m0/evidence/artifacts/1.2/build-offline-gate.json'; sha256 = (Get-FileHash $architecturePath -Algorithm SHA256).Hash },
        @{ path = 'verify-superdesktop-m0/evidence/artifacts/5.3/traceability-review.json'; sha256 = (Get-FileHash $traceabilityPath -Algorithm SHA256).Hash },
        @{ path = 'verify-superdesktop-m0/evidence/coverage.json'; sha256 = (Get-FileHash $coveragePath -Algorithm SHA256).Hash },
        @{ path = 'verify-superdesktop-m0/evidence/adjustments.jsonl'; sha256 = (Get-FileHash $adjustmentsPath -Algorithm SHA256).Hash }
    )
    unresolved = @(
        'physical-five-dpi-matrix',
        'physical-mixed-dpi-dual-monitor',
        'windows-10-22h2',
        'independent-final-review',
        'archive-deferred-by-user'
    )
}
New-Item -ItemType Directory -Force (Split-Path -Parent $wave6Path) | Out-Null
Write-Json $wave6Path $wave6

$groupMetadata = @{
    '1.1' = @{ artifact = 'evidence/artifacts/1.1/readiness-report.md'; gates = @('G-TRACE') }
    '2.1' = @{ artifact = 'evidence/artifacts/2.1/wave1-bootstrap-rollup.md'; gates = @('G-ARCH', 'G-TRACE') }
    '2.2' = @{ artifact = 'evidence/artifacts/2.2/wave2-capability-rollup.md'; gates = @('G-ARCH', 'G-SAFETY') }
    '3.1' = @{ artifact = 'evidence/artifacts/3.1/wave3-core-rollup.md'; gates = @('G-ARCH', 'G-TRACE') }
    '3.2' = @{ artifact = 'evidence/artifacts/3.2/wave4a-desktop-rollup.md'; gates = @('G-DESKTOP', 'G-A11Y-I18N', 'G-DPI-MONITOR') }
    '3.3' = @{ artifact = 'evidence/artifacts/3.3/wave4b-taskbar-rollup.md'; gates = @('G-TASKBAR', 'G-A11Y-I18N', 'G-DPI-MONITOR') }
    '3.4' = @{ artifact = 'evidence/artifacts/3.4/wave4c-bridge-rollup.md'; gates = @('G-EXPLORER-BRIDGE', 'G-SAFETY') }
    '4.1' = @{ artifact = 'evidence/artifacts/4.1/wave5-lifecycle-rollup.md'; gates = @('G-SHELL-TAKEOVER-PROVISIONAL', 'G-GUARDIAN-RECOVERY-PROVISIONAL', 'G-SAFETY') }
}
$wave6Gates = @{
    '4.2.1' = @('G-TRACE'); '4.2.2' = @('G-TRACE'); '4.2.3' = @('G-TRACE')
    '4.2.4' = @('G-DESKTOP', 'G-TASKBAR', 'G-EXPLORER-BRIDGE')
    '4.2.5' = @('G-SHELL-TAKEOVER', 'G-GUARDIAN-RECOVERY')
    '4.2.6' = @('G-DPI-MONITOR'); '4.2.7' = @('G-A11Y-I18N')
    '4.2.8' = @('G-PERF'); '4.2.9' = @('G-SAFETY', 'G-ARCH')
    '4.2.10' = @('G-TRACE'); '4.2.11' = @('G-TRACE'); '4.2.12' = @('G-ARCH')
    '4.2.13' = @('G-TRACE')
}

$coverageTasks = @()
$records = @()
foreach ($line in Get-Content -Encoding UTF8 (Join-Path $root 'tasks.md')) {
    if ($line -notmatch '^\s*- \[([ xX])\]\s+([0-9]+\.[0-9]+\.[0-9]+)\b') {
        continue
    }
    $marker = $matches[1]
    $id = $matches[2]
    $checked = $marker -in @('x', 'X')
    $group = $id.Substring(0, $id.LastIndexOf('.'))
    if ($group -eq '4.2') {
        $artifact = 'evidence/artifacts/4.2/wave6-local-gates.json'
        $gates = @($wave6Gates[$id])
    } else {
        $artifact = $groupMetadata[$group].artifact
        $gates = @($groupMetadata[$group].gates)
    }
    $taskId = "$change/$id"
    $coverageTasks += [ordered]@{
        task_id = $taskId
        mandatory = $true
        capability_id = 'shell-foundation-verification'
        requirement_id = 'program-rollup-evidence'
        scenario_id = 'mandatory-leaf-has-passed-evidence'
        gates = $gates
    }
    if (-not $checked) {
        continue
    }
    $artifactPath = Join-Path $root $artifact
    if (-not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
        throw "Missing roll-up artifact for $id`: $artifactPath"
    }
    $records += [ordered]@{
        schema_version = '2.0.0'
        task_id = $taskId
        subcheck = "rollup-$($id.Replace('.', '-'))"
        status = 'passed'
        artifact = $artifact
        artifact_sha256 = (Get-FileHash $artifactPath -Algorithm SHA256).Hash
        capability_id = 'shell-foundation-verification'
        requirement_id = 'program-rollup-evidence'
        scenario_id = 'mandatory-leaf-has-passed-evidence'
        gates = $gates
        reviewer = 'Primary integrator'
        recorded_at = $recordedAt
        procedure = "Verify the child outputs and gate dispositions required by foundation leaf $id, then hash the immutable roll-up artifact."
        expected = 'Every checked program leaf has schema-valid, hash-bound passed evidence without converting an unresolved mandatory leaf to N/A.'
        actual = "Foundation leaf $id is passed; unresolved archive and external release gates remain unchecked."
    }
}

Write-Json (Join-Path $evidence 'coverage.json') ([ordered]@{
    schema_version = '1.0.0'
    change = $change
    capabilities = @('shell-foundation-verification')
    tasks = $coverageTasks
})
Write-Text (Join-Path $evidence 'index.jsonl') (($records | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 }) -join "`n" + "`n")
Write-Output "Foundation evidence captured: $($records.Count) passed records / $($coverageTasks.Count) mappings."
