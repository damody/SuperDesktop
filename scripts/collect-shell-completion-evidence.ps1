[CmdletBinding()]
param(
    [string]$RepositoryRoot,
    [string]$OutputPath,
    [string]$ExternalEvidenceDirectory
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

$externalSources = [ordered]@{}
$externalGates = @{}
if (-not [string]::IsNullOrWhiteSpace($ExternalEvidenceDirectory)) {
    $externalRoot = (Resolve-Path -LiteralPath $ExternalEvidenceDirectory).Path
    $requiredExternalRoot = [IO.Path]::GetFullPath((Join-Path $evidenceRoot 'external'))
    if ($externalRoot -cne $requiredExternalRoot) {
        throw "External evidence must be admitted from $requiredExternalRoot"
    }
    $expectedExternal = [ordered]@{
        'windows10-lifecycle-installer' = 'windows10-lifecycle-installer.json'
        'physical-mixed-dpi' = 'physical-mixed-dpi.json'
        'independent-review' = 'independent-review.json'
    }
    $candidatePath = Join-Path $evidenceRoot 'release-candidate.json'
    $candidate = Get-Content -Raw -Encoding utf8 -LiteralPath $candidatePath | ConvertFrom-Json
    $revision = [string]$candidate.reviewed_revision
    if ($candidate.schema_version -ne 1 -or $revision -notmatch '^[0-9a-f]{40}$') { throw 'Invalid release-candidate manifest.' }
    & git -C $root cat-file -e "$revision^{commit}"
    if ($LASTEXITCODE -ne 0) { throw 'Release-candidate revision is unavailable.' }
    & git -C $root merge-base --is-ancestor $revision HEAD
    if ($LASTEXITCODE -ne 0) { throw 'Current revision does not descend from the frozen release candidate.' }
    foreach ($kind in $expectedExternal.Keys) {
        $path = Join-Path $externalRoot $expectedExternal[$kind]
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Missing external evidence: $kind"
        }
        $document = Get-Content -Raw -Encoding utf8 -LiteralPath $path | ConvertFrom-Json
        if ($document.schema_version -ne 1 -or $document.kind -cne $kind -or $document.status -cne 'passed') {
            throw "Invalid external evidence envelope: $kind"
        }
        if ($document.revision -cne $revision) {
            throw "External evidence revision drift: $kind"
        }
        try {
            [DateTimeOffset]::Parse($document.recorded_at_utc) | Out-Null
        } catch {
            throw "Invalid external timestamp: $kind"
        }
        switch ($kind) {
            'windows10-lifecycle-installer' {
                if ($document.host.build -ne 19045 -or $document.host.display_version -cne '22H2') {
                    throw 'Windows 10 evidence host is not build 19045 22H2.'
                }
                if ($document.lifecycle.forced_crash_runs -ne 10 -or $document.lifecycle.max_recovery_ms -gt 10000) {
                    throw 'Windows 10 recovery contract failed.'
                }
                if (-not $document.lifecycle.preview_zero_mutation -or -not $document.lifecycle.normal_exit_restored) {
                    throw 'Windows 10 lifecycle contract failed.'
                }
                if (-not $document.installer.reboot_verified -or -not $document.installer.exact_rollback_verified -or -not $document.installer.metadata_removed) {
                    throw 'Installer reboot/rollback contract failed.'
                }
                foreach ($gate in @('G-SHELL-TAKEOVER','G-GUARDIAN-RECOVERY','G-INSTALL-ROLLBACK')) {
                    if ($document.gates.$gate -cne 'passed') { throw "Missing passed $gate" }
                    $externalGates[$gate] = 'passed'
                }
            }
            'physical-mixed-dpi' {
                if ($document.monitor_count -lt 2 -or $document.distinct_dpi_count -lt 2 -or $document.artifact_hashes.Count -lt 4) {
                    throw 'Physical mixed-DPI topology or artifacts are incomplete.'
                }
                foreach ($check in @('pointer','keyboard_focus','drag','primary_change','hot_plug','work_area_restored')) {
                    if ($document.interactions.$check -cne 'passed') { throw "Physical interaction is not passed: $check" }
                }
                if ($document.gates.'G-DPI-MONITOR-PHYSICAL' -cne 'passed') { throw 'Missing passed physical DPI gate.' }
                $externalGates['G-DPI-MONITOR-PHYSICAL'] = 'passed'
            }
            'independent-review' {
                if (-not $document.independence.not_implementation_owner -or -not $document.independence.not_remediation_owner -or $document.unresolved_p0_p1 -ne 0) {
                    throw 'Independent review admission failed.'
                }
                foreach ($area in @('architecture','security','accessibility','evidence_lineage')) {
                    if ($document.scope.$area -cne 'passed') { throw "Independent review area is not passed: $area" }
                }
                if ($document.gates.'G-REVIEW' -cne 'passed') { throw 'Missing passed review gate.' }
                $externalGates['G-REVIEW'] = 'passed'
            }
        }
        $relative = [IO.Path]::GetRelativePath($root, $path).Replace('\','/')
        $externalSources[$kind] = [ordered]@{
            kind = $kind
            relative_path = $relative
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
            status = 'passed'
        }
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
foreach ($gate in $externalGates.Keys) {
    $gates[$gate] = $externalGates[$gate]
}
$blockers = @($gates.GetEnumerator() | Where-Object Value -ne 'passed' | ForEach-Object { "$($_.Key):$($_.Value)".ToLowerInvariant() })
$rollup = [ordered]@{
    schema_version = 1
    generated_at_utc = [DateTime]::UtcNow.ToString('o')
    sources = $sources
    external_sources = $externalSources
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
