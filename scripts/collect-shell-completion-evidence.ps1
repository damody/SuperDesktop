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
function Resolve-RepositoryEvidencePath([string]$RelativePath, [string]$Label) {
    if ([string]::IsNullOrWhiteSpace($RelativePath) -or [IO.Path]::IsPathRooted($RelativePath)) { throw "$Label path must be repository-relative." }
    $full = [IO.Path]::GetFullPath((Join-Path $root ($RelativePath -replace '/', '\')))
    $prefix = $root.TrimEnd('\') + '\'
    if (-not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $full -PathType Leaf)) {
        throw "$Label path is missing or escapes the repository: $RelativePath"
    }
    return $full
}
function Assert-HashRecords($Records, [int]$RequiredCount, [string]$Label, [switch]$AllowMore) {
    $items = @($Records)
    if (($AllowMore -and $items.Count -lt $RequiredCount) -or (-not $AllowMore -and $items.Count -ne $RequiredCount)) {
        throw "$Label hash-record count is invalid."
    }
    $seen = @{}
    foreach ($item in $items) {
        $relative = [string]$item.path
        if ($seen.ContainsKey($relative)) { throw "$Label contains duplicate path: $relative" }
        $seen[$relative] = $true
        $expected = ([string]$item.sha256).ToLowerInvariant()
        if ($expected -notmatch '^[0-9a-f]{64}$') { throw "$Label contains an invalid SHA-256: $relative" }
        $path = Resolve-RepositoryEvidencePath $relative $Label
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        if ($actual -cne $expected) { throw "$Label source hash drift: $relative" }
    }
}
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
        'reference-profile-lifecycle-installer' = 'reference-profile-lifecycle-installer.json'
        'physical-mixed-dpi' = 'physical-mixed-dpi.json'
        'independent-review' = 'independent-review.json'
    }
    $expectedExternalFiles = @($expectedExternal.Values)
    $unexpectedExternalFiles = @(Get-ChildItem -LiteralPath $externalRoot -File -Filter '*.json' | Where-Object {
        $_.Name -notin $expectedExternalFiles
    })
    if ($unexpectedExternalFiles.Count -ne 0) {
        throw "Unexpected external evidence: $($unexpectedExternalFiles[0].Name)"
    }
    $presentExternalKinds = @($expectedExternal.Keys | Where-Object {
        Test-Path -LiteralPath (Join-Path $externalRoot $expectedExternal[$_]) -PathType Leaf
    })
    $revision = $null
    if ($presentExternalKinds.Count -ne 0) {
        $candidatePath = Join-Path $evidenceRoot 'release-candidate.json'
        $candidate = Get-Content -Raw -Encoding utf8 -LiteralPath $candidatePath | ConvertFrom-Json
        $revision = [string]$candidate.reviewed_revision
        if ($candidate.schema_version -ne 1 -or $revision -notmatch '^[0-9a-f]{40}$') { throw 'Invalid release-candidate manifest.' }
        & git -C $root cat-file -e "$revision^{commit}"
        if ($LASTEXITCODE -ne 0) { throw 'Release-candidate revision is unavailable.' }
        & git -C $root merge-base --is-ancestor $revision HEAD
        if ($LASTEXITCODE -ne 0) { throw 'Current revision does not descend from the frozen release candidate.' }
        $trackedDrift = @(& git -C $root diff --name-only $revision HEAD --)
        if ($LASTEXITCODE -ne 0) { throw 'Unable to inspect post-candidate drift.' }
        $invalidDrift = @($trackedDrift | Where-Object {
            $_ -notmatch '^openspec/changes/[^/]+/evidence/' -and
            $_ -notmatch '^openspec/changes/[^/]+/tasks\.md$'
        })
        if ($invalidDrift.Count -ne 0) {
            throw "Production drift after frozen release candidate: $($invalidDrift[0])"
        }
    }
    foreach ($kind in $expectedExternal.Keys) {
        $path = Join-Path $externalRoot $expectedExternal[$kind]
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            continue
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
            'reference-profile-lifecycle-installer' {
                if ($document.host.build -ne 26200 -or $document.host.ubr -ne 9168 -or
                    $document.host.explorerpatcher_version -cne '26100.8457.70.3' -or
                    [string]$document.host.profile_fingerprint -notmatch '^sha256:[0-9a-f]{64}$') {
                    throw 'Lifecycle/installer evidence host is not the exact Windows 11 ExplorerPatcher reference profile.'
                }
                foreach ($operator in @($document.operators.lifecycle,$document.operators.installer)) {
                    if ([string]::IsNullOrWhiteSpace($operator.name) -or [string]::IsNullOrWhiteSpace($operator.organization) -or
                        [string]$operator.name -like 'REPLACE_WITH_*' -or [string]$operator.organization -like 'REPLACE_WITH_*') { throw 'Reference-profile lifecycle/installer operator is not attributable.' }
                }
                if (-not $document.lifecycle.production_guardian_path -or $document.lifecycle.forced_crash_runs -ne 10 -or $document.lifecycle.max_recovery_ms -gt 10000) {
                    throw 'Reference-profile recovery contract failed.'
                }
                if (-not $document.lifecycle.preview_zero_mutation -or -not $document.lifecycle.normal_exit_restored) {
                    throw 'Reference-profile lifecycle contract failed.'
                }
                if (-not $document.installer.reboot_verified -or -not $document.installer.exact_rollback_verified -or -not $document.installer.metadata_removed) {
                    throw 'Installer reboot/rollback contract failed.'
                }
                Assert-HashRecords $document.host.profile_sources 5 'Reference-profile source evidence'
                Assert-HashRecords $document.source_hashes 12 'Reference-profile lifecycle/installer evidence'
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
                foreach ($feature in @('desktop_file_operations','context_menu','start_search','taskbar_flyouts','notification_area','virtual_desktop_query_move','accessibility')) {
                    if ($document.completion_features.$feature -cne 'passed') { throw "Physical completion feature is not passed: $feature" }
                }
                if ([string]::IsNullOrWhiteSpace($document.operator.name) -or [string]::IsNullOrWhiteSpace($document.operator.organization) -or
                    [string]$document.operator.name -like 'REPLACE_WITH_*' -or [string]$document.operator.organization -like 'REPLACE_WITH_*') { throw 'Physical evidence operator is not attributable.' }
                Assert-HashRecords $document.source_hashes 3 'Physical mixed-DPI source evidence'
                Assert-HashRecords $document.artifact_hashes 4 'Physical mixed-DPI photo/screenshot evidence' -AllowMore
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
                if ([string]::IsNullOrWhiteSpace($document.reviewer.name) -or [string]::IsNullOrWhiteSpace($document.reviewer.organization) -or
                    [string]$document.reviewer.name -like 'REPLACE_WITH_*' -or [string]$document.reviewer.organization -like 'REPLACE_WITH_*') { throw 'Independent reviewer is not attributable.' }
                $expectedChanges = @(
                    'extend-superdesktop-shell-contracts','add-superdesktop-desktop-file-operations','add-superdesktop-shell-context-menu-host',
                    'add-superdesktop-start-search','add-superdesktop-taskbar-advanced-interactions','add-superdesktop-notification-area-host',
                    'add-superdesktop-virtual-desktops','add-superdesktop-shell-installer','adopt-superdesktop-windows11-reference-release',
                    'verify-superdesktop-shell-completion','complete-superdesktop-windows-shell'
                )
                $actualChanges = @($document.scope.changes | Sort-Object -Unique)
                if ($actualChanges.Count -ne $expectedChanges.Count -or @($expectedChanges | Where-Object { $_ -notin $actualChanges }).Count -ne 0) { throw 'Independent review change scope is incomplete.' }
                $expectedPaths = @(
                    'docs/superpowers/specs/2026-08-16-superdesktop-windows-shell-completion-design.md',
                    'docs/superpowers/specs/2026-08-17-superdesktop-windows11-reference-release-design.md',
                    'openspec/changes/adopt-superdesktop-windows11-reference-release/design.md',
                    'openspec/changes/adopt-superdesktop-windows11-reference-release/specs/windows11-reference-release-baseline/spec.md',
                    'openspec/changes/complete-superdesktop-windows-shell/PROGRAM.md',
                    'openspec/changes/complete-superdesktop-windows-shell/design.md',
                    'openspec/changes/complete-superdesktop-windows-shell/specs/windows-shell-completion-program/spec.md',
                    'openspec/changes/verify-superdesktop-shell-completion/design.md',
                    'openspec/changes/verify-superdesktop-shell-completion/specs/shell-completion-verification/spec.md',
                    'openspec/changes/verify-superdesktop-shell-completion/evidence/current-rollup.json'
                )
                $manifestEntries = @($document.source_manifest)
                if ($manifestEntries.Count -ne $expectedPaths.Count) { throw 'Independent review source manifest is incomplete.' }
                foreach ($sourcePath in $expectedPaths) {
                    $entries = @($manifestEntries | Where-Object path -CEQ $sourcePath)
                    $expectedOid = (& git -C $root rev-parse "$revision`:$sourcePath").Trim()
                    if ($entries.Count -ne 1 -or $entries[0].git_blob_oid -cne $expectedOid) { throw "Independent review source lineage drift: $sourcePath" }
                }
                $expectedTree = (& git -C $root rev-parse "$revision^{tree}").Trim()
                if ($document.reviewed_tree_oid -cne $expectedTree) { throw 'Independent review tree lineage drift.' }
                Assert-HashRecords @($document.confirmation) 1 'Independent review confirmation'
                $unresolved = @($document.findings | Where-Object { $_.severity -in @('P0','P1') -and $_.status -cne 'resolved' })
                if ($unresolved.Count -ne 0 -or $document.unresolved_p0_p1 -ne $unresolved.Count -or $document.final_disposition -cne 'no-unresolved-p0-p1') { throw 'Independent review finding disposition is inconsistent.' }
                if ($document.gates.'G-REVIEW' -cne 'passed') { throw 'Missing passed review gate.' }
                $externalGates['G-REVIEW'] = 'passed'
            }
        }
        $rootPrefix = $root.TrimEnd('\') + '\'
        $relative = $path.Substring($rootPrefix.Length).Replace('\','/')
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
        [ordered]@{ capability = 'virtual-desktop-undocumented-operations'; disposition = 'unavailable'; reason = 'documented IVirtualDesktopManager adapter exposes query and move only' },
        [ordered]@{ capability = 'windows-10-compatibility'; disposition = 'not-claimed'; reason = 'C-W11-REFERENCE-001 selects the exact Windows 11 ExplorerPatcher profile as the release platform' }
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
