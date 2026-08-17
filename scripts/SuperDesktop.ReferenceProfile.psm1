Set-StrictMode -Version Latest

$script:ExpectedContractSha256 = '8d4855cab9549efb9687ebcc7b6aefa9394a86eae379c70f76034fee27040974'
$script:ReferenceChangeRelative = 'openspec/changes/validate-superdesktop-windows-platform'

function Resolve-ContainedFile {
    param(
        [Parameter(Mandatory)][string]$Root,
        [Parameter(Mandatory)][string]$RelativePath,
        [Parameter(Mandatory)][string]$Label
    )
    if ([IO.Path]::IsPathRooted($RelativePath)) { throw "REFERENCE_PATH_ROOTED: $Label" }
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    $full = [IO.Path]::GetFullPath((Join-Path $rootFull ($RelativePath -replace '/', '\')))
    if (-not $full.StartsWith($rootFull + '\', [StringComparison]::OrdinalIgnoreCase) -or
        -not (Test-Path -LiteralPath $full -PathType Leaf)) {
        throw "REFERENCE_PATH_INVALID: $Label"
    }
    return $full
}

function Assert-FileHash {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Expected,
        [Parameter(Mandatory)][string]$Label
    )
    if ($Expected -notmatch '^[0-9a-fA-F]{64}$') { throw "REFERENCE_HASH_INVALID: $Label" }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($actual -cne $Expected.ToLowerInvariant()) { throw "REFERENCE_HASH_DRIFT: $Label" }
    return $actual
}

function Get-ReferenceProfileContract {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Workspace)

    $workspaceFull = (Resolve-Path -LiteralPath $Workspace).Path
    $contractPath = Resolve-ContainedFile -Root $workspaceFull -RelativePath "$script:ReferenceChangeRelative/evidence/artifacts/1.1/frozen-profile-contract.json" -Label 'contract'
    $contractHash = Assert-FileHash -Path $contractPath -Expected $script:ExpectedContractSha256 -Label 'contract'
    $contract = Get-Content -Raw -Encoding utf8 -LiteralPath $contractPath | ConvertFrom-Json
    if ($contract.schema_version -ne '1.0.0' -or $contract.contract -cne 'frozen-win11-explorerpatcher-profile-and-readonly-admission-probe') {
        throw 'REFERENCE_CONTRACT_INVALID'
    }
    if ([int]$contract.os_session_display_monitor.values.os.build -ne 26200 -or [int]$contract.os_session_display_monitor.values.os.ubr -ne 8875) {
        throw 'REFERENCE_CONTRACT_OS_INVALID'
    }
    if ([string]$contract.explorerpatcher.expected_version -cne '26100.8457.70.3') {
        throw 'REFERENCE_CONTRACT_EXPLORERPATCHER_INVALID'
    }

    $referenceRoot = Join-Path $workspaceFull ($script:ReferenceChangeRelative -replace '/', '\')
    $settingsPath = Resolve-ContainedFile -Root $referenceRoot -RelativePath ([string]$contract.explorerpatcher.settings_snapshot_path) -Label 'settings'
    $allowlistPath = Resolve-ContainedFile -Root $referenceRoot -RelativePath ([string]$contract.explorerpatcher.allowlist_path) -Label 'allowlist'
    $imagePath = Resolve-ContainedFile -Root $referenceRoot -RelativePath ([string]$contract.reference_image.path) -Label 'reference-image'
    $settingsHash = Assert-FileHash -Path $settingsPath -Expected ([string]$contract.explorerpatcher.settings_snapshot_sha256) -Label 'settings'
    $allowlistHash = Assert-FileHash -Path $allowlistPath -Expected ([string]$contract.explorerpatcher.allowlist_sha256) -Label 'allowlist'
    $imageHash = Assert-FileHash -Path $imagePath -Expected ([string]$contract.reference_image.sha256) -Label 'reference-image'

    return [ordered]@{
        contract = $contract
        paths = [ordered]@{
            contract = $contractPath
            settings = $settingsPath
            allowlist = $allowlistPath
            reference_image = $imagePath
        }
        hashes = [ordered]@{
            contract = $contractHash
            settings = $settingsHash
            allowlist = $allowlistHash
            reference_image = $imageHash
        }
    }
}

function Assert-ReferenceProfileValues {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]$Expected,
        [Parameter(Mandatory)]$Observed
    )
    if ([string]$Observed.product -notmatch 'Windows 11') { throw 'REFERENCE_OS_PRODUCT_DRIFT' }
    if ([int]$Observed.build -ne [int]$Expected.build) { throw 'REFERENCE_OS_BUILD_DRIFT' }
    if ([int]$Observed.ubr -ne [int]$Expected.ubr) { throw 'REFERENCE_OS_UBR_DRIFT' }
    if (-not [bool]$Observed.interactive -or [int]$Observed.session_id -eq 0 -or [int]$Observed.product_type -ne 1) {
        throw 'REFERENCE_SESSION_UNSUPPORTED'
    }
    if ([string]$Observed.explorerpatcher_version -cne [string]$Expected.explorerpatcher_version) {
        throw 'REFERENCE_EXPLORERPATCHER_VERSION_DRIFT'
    }
    $expectedBinaries = @($Expected.binaries)
    $observedBinaries = @($Observed.binaries)
    if ($expectedBinaries.Count -ne 3 -or $observedBinaries.Count -ne $expectedBinaries.Count) {
        throw 'REFERENCE_BINARY_SET_DRIFT'
    }
    for ($index = 0; $index -lt $expectedBinaries.Count; $index++) {
        $expectedBinary = $expectedBinaries[$index]
        $observedBinary = $observedBinaries[$index]
        if ([string]$observedBinary.path -cne [string]$expectedBinary.path -or
            [long]$observedBinary.length -ne [long]$expectedBinary.length -or
            [string]$observedBinary.file_version -cne [string]$expectedBinary.file_version -or
            [string]$observedBinary.product_version -cne [string]$expectedBinary.product_version -or
            [string]$observedBinary.sha256 -cne ([string]$expectedBinary.sha256).ToLowerInvariant()) {
            throw "REFERENCE_BINARY_DRIFT: $index"
        }
    }
}

function Assert-ReleaseCandidateLineage {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Workspace,
        [Parameter(Mandatory)][string]$Revision
    )
    if ($Revision -notmatch '^[0-9a-f]{40}$') { throw 'REFERENCE_CANDIDATE_INVALID' }
    & git -C $Workspace cat-file -e "$Revision^{commit}" 2>$null
    if ($LASTEXITCODE -ne 0) { throw 'REFERENCE_CANDIDATE_MISSING' }
    & git -C $Workspace merge-base --is-ancestor $Revision HEAD 2>$null
    if ($LASTEXITCODE -ne 0) { throw 'REFERENCE_CANDIDATE_NOT_ANCESTOR' }
    foreach ($arguments in @(
        @('diff', '--quiet', $Revision, 'HEAD', '--', 'crates', 'Cargo.toml', 'Cargo.lock'),
        @('diff', '--quiet', '--', 'crates', 'Cargo.toml', 'Cargo.lock'),
        @('diff', '--cached', '--quiet', '--', 'crates', 'Cargo.toml', 'Cargo.lock')
    )) {
        & git -C $Workspace @arguments
        if ($LASTEXITCODE -ne 0) { throw 'REFERENCE_PRODUCTION_DRIFT' }
    }
}

function Assert-SuperDesktopExternalEvidenceKind {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Kind)
    if ($Kind -ceq 'windows10-lifecycle-installer' -or $Kind -match '(?i)windows10|windows_10') {
        throw 'REFERENCE_OBSOLETE_WINDOWS10_KIND'
    }
    if ($Kind -cne 'reference-profile-lifecycle-installer') {
        throw 'REFERENCE_EXTERNAL_KIND_INVALID'
    }
}

function Assert-SuperDesktopInstallerHostSet {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][Collections.IDictionary]$Hosts,
        [Parameter(Mandatory)][string]$Revision,
        [Parameter(Mandatory)][string]$ProfileFingerprint
    )
    $required = @('DryRun', 'Enable', 'AfterReboot', 'Rollback')
    foreach ($phase in $required) {
        if (-not $Hosts.Contains($phase)) { throw "REFERENCE_INSTALLER_PHASE_MISSING: $phase" }
    }
    if ($Hosts.Count -ne $required.Count) { throw 'REFERENCE_INSTALLER_PHASE_UNEXPECTED' }
    $baseline = $Hosts['DryRun']
    if ([string]::IsNullOrWhiteSpace($baseline.operator.name) -or [string]::IsNullOrWhiteSpace($baseline.operator.organization) -or
        [string]$baseline.operator.name -like 'REPLACE_WITH_*' -or [string]$baseline.operator.organization -like 'REPLACE_WITH_*') {
        throw 'REFERENCE_INSTALLER_OPERATOR_INVALID'
    }
    $baselineBinaries = @($baseline.binaries)
    if ($baselineBinaries.Count -ne 6) { throw 'REFERENCE_INSTALLER_BINARY_SET_INVALID' }
    $baselineNames = @($baselineBinaries | ForEach-Object { [string]$_.name } | Sort-Object -Unique)
    if ($baselineNames.Count -ne 6) { throw 'REFERENCE_INSTALLER_BINARY_SET_INVALID' }
    foreach ($phase in $required) {
        $host = $Hosts[$phase]
        if ([string]$host.revision -cne $Revision) { throw "REFERENCE_INSTALLER_REVISION_DRIFT: $phase" }
        if ([int]$host.build -ne 26200 -or [int]$host.ubr -ne 8875 -or
            [string]$host.explorerPatcherVersion -cne '26100.8457.70.3' -or
            [string]$host.profileFingerprint -cne $ProfileFingerprint) {
            throw "REFERENCE_INSTALLER_PROFILE_DRIFT: $phase"
        }
        if ([string]$host.operator.name -cne [string]$baseline.operator.name -or
            [string]$host.operator.organization -cne [string]$baseline.operator.organization) {
            throw "REFERENCE_INSTALLER_OPERATOR_DRIFT: $phase"
        }
        if ([IO.Path]::GetFullPath([string]$host.rollbackRecordPath) -cne [IO.Path]::GetFullPath([string]$baseline.rollbackRecordPath)) {
            throw "REFERENCE_INSTALLER_ROLLBACK_PATH_DRIFT: $phase"
        }
        $binaries = @($host.binaries)
        if ($binaries.Count -ne 6) { throw "REFERENCE_INSTALLER_BINARY_SET_DRIFT: $phase" }
        foreach ($name in $baselineNames) {
            $expectedRecords = @($baselineBinaries | Where-Object name -CEQ $name)
            $records = @($binaries | Where-Object name -CEQ $name)
            if ($expectedRecords.Count -ne 1 -or $records.Count -ne 1 -or
                [string]$records[0].sha256 -notmatch '^[0-9a-f]{64}$' -or
                [string]$records[0].sha256 -cne [string]$expectedRecords[0].sha256) {
                throw "REFERENCE_INSTALLER_BINARY_DRIFT: $phase/$name"
            }
        }
    }
}

function Get-SuperDesktopReferenceProfileAdmission {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Workspace,
        [string]$CandidatePath
    )
    $workspaceFull = (Resolve-Path -LiteralPath $Workspace).Path
    if ([string]::IsNullOrWhiteSpace($CandidatePath)) {
        $CandidatePath = Join-Path $workspaceFull 'openspec/changes/verify-superdesktop-shell-completion/evidence/release-candidate.json'
    }
    $candidateFull = (Resolve-Path -LiteralPath $CandidatePath).Path
    $workspacePrefix = $workspaceFull.TrimEnd('\') + '\'
    if (-not $candidateFull.StartsWith($workspacePrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'REFERENCE_CANDIDATE_PATH_INVALID'
    }
    $candidate = Get-Content -Raw -Encoding utf8 -LiteralPath $candidateFull | ConvertFrom-Json
    $revision = [string]$candidate.reviewed_revision
    if ($candidate.schema_version -ne 1) { throw 'REFERENCE_CANDIDATE_SCHEMA_INVALID' }
    Assert-ReleaseCandidateLineage -Workspace $workspaceFull -Revision $revision

    $bound = Get-ReferenceProfileContract -Workspace $workspaceFull
    $contract = $bound.contract
    $os = Get-CimInstance Win32_OperatingSystem
    $windows = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
    $observedBinaries = @()
    foreach ($binary in @($contract.explorerpatcher.binaries)) {
        $path = (Resolve-Path -LiteralPath ([string]$binary.path)).Path
        $item = Get-Item -LiteralPath $path
        $observedBinaries += [ordered]@{
            path = $path
            length = [long]$item.Length
            file_version = [string]$item.VersionInfo.FileVersion
            product_version = [string]$item.VersionInfo.ProductVersion
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        }
    }
    $observed = [ordered]@{
        product = [string]$os.Caption
        build = [int]$os.BuildNumber
        ubr = [int]$windows.UBR
        product_type = [int]$os.ProductType
        interactive = [bool][Environment]::UserInteractive
        session_id = [int](Get-Process -Id $PID).SessionId
        explorerpatcher_version = [string]$observedBinaries[0].file_version
        binaries = $observedBinaries
    }
    $expected = [ordered]@{
        build = [int]$contract.os_session_display_monitor.values.os.build
        ubr = [int]$contract.os_session_display_monitor.values.os.ubr
        explorerpatcher_version = [string]$contract.explorerpatcher.expected_version
        binaries = @($contract.explorerpatcher.binaries | ForEach-Object {
            [ordered]@{
                path = (Resolve-Path -LiteralPath ([string]$_.path)).Path
                length = [long]$_.length
                file_version = [string]$_.file_version
                product_version = [string]$_.product_version
                sha256 = ([string]$_.sha256).ToLowerInvariant()
            }
        })
    }
    Assert-ReferenceProfileValues -Expected $expected -Observed $observed

    $validatorPath = Join-Path $workspaceFull 'openspec/changes/validate-superdesktop-windows-platform/scripts/validate-profile-snapshot.ps1'
    . $validatorPath
    $settingsObservation = Get-ProfileSnapshot -AllowlistPath $bound.paths.allowlist
    Assert-ProfileSnapshot -AllowlistPath $bound.paths.allowlist -Snapshot $settingsObservation

    $sourceRecords = @(
        [ordered]@{ kind='contract';path=$bound.paths.contract.Substring($workspacePrefix.Length).Replace('\','/');sha256=$bound.hashes.contract },
        [ordered]@{ kind='settings';path=$bound.paths.settings.Substring($workspacePrefix.Length).Replace('\','/');sha256=$bound.hashes.settings },
        [ordered]@{ kind='allowlist';path=$bound.paths.allowlist.Substring($workspacePrefix.Length).Replace('\','/');sha256=$bound.hashes.allowlist },
        [ordered]@{ kind='reference-image';path=$bound.paths.reference_image.Substring($workspacePrefix.Length).Replace('\','/');sha256=$bound.hashes.reference_image },
        [ordered]@{ kind='candidate';path=$candidateFull.Substring($workspacePrefix.Length).Replace('\','/');sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $candidateFull).Hash.ToLowerInvariant() }
    )
    $fingerprintInput = [ordered]@{
        revision = $revision
        build = $observed.build
        ubr = $observed.ubr
        explorerpatcher_version = $observed.explorerpatcher_version
        binaries = @($observed.binaries | ForEach-Object { $_.sha256 })
        sources = @($sourceRecords | ForEach-Object { $_.sha256 })
    } | ConvertTo-Json -Depth 8 -Compress
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $fingerprint = ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($fingerprintInput)))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
    return [ordered]@{
        schema = 'superdesktop-reference-profile-admission/v1'
        result = 'passed'
        mutation_performed = $false
        candidate_revision = $revision
        profile_fingerprint = "sha256:$fingerprint"
        observed = $observed
        settings_allowlist_sha256 = ([string]$settingsObservation.allowlist_sha256).ToLowerInvariant()
        sources = $sourceRecords
    }
}

Export-ModuleMember -Function Get-ReferenceProfileContract, Assert-ReferenceProfileValues, Assert-ReleaseCandidateLineage, Assert-SuperDesktopExternalEvidenceKind, Assert-SuperDesktopInstallerHostSet, Get-SuperDesktopReferenceProfileAdmission
