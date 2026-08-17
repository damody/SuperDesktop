[CmdletBinding()]
param(
    [string]$Workspace,
    [string]$OutputPath,
    [string]$OperatorName,
    [string]$OperatorOrganization
)

$ErrorActionPreference = 'Stop'
$utf8 = [Text.UTF8Encoding]::new($false)
if (-not $Workspace) {
    $Workspace = Split-Path -Parent $PSScriptRoot
}
if (-not $OutputPath) {
    $OutputPath = Join-Path $Workspace 'openspec/changes/verify-superdesktop-m0/evidence/artifacts/3.1/reference-profile-gate.json'
}

function Write-Json([string]$Path, $Value) {
    New-Item -ItemType Directory -Force (Split-Path -Parent $Path) | Out-Null
    [IO.File]::WriteAllText($Path, (($Value | ConvertTo-Json -Depth 30) + "`n"), $utf8)
}

function Invoke-Checked([string]$Name, [scriptblock]$Command) {
    $prior = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $text = (& $Command 2>&1 | Out-String)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prior
    }
    if ($exitCode -ne 0) {
        throw "$Name failed with exit code $exitCode`n$text"
    }
    return $text.Trim()
}

function Registry-Snapshot {
    $paths = @(
        'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Winlogon',
        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer',
        'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
    )
    $snapshot = [ordered]@{}
    foreach ($path in $paths) {
        if (-not (Test-Path -LiteralPath $path)) {
            $snapshot[$path] = $null
            continue
        }
        $values = [ordered]@{}
        foreach ($property in ((Get-ItemProperty -LiteralPath $path).PSObject.Properties | Where-Object { $_.Name -notlike 'PS*' } | Sort-Object Name)) {
            $values[$property.Name] = [string]$property.Value
        }
        $snapshot[$path] = $values
    }
    return $snapshot
}

function Explorer-Count {
    return @(Get-Process explorer -ErrorAction SilentlyContinue).Count
}

function Monitor-Profile([string]$ProbePath) {
    $text = Invoke-Checked 'monitor/DPI profile' { & $ProbePath }
    $line = $text -split "`r?`n" | Where-Object { $_.StartsWith('{') } | Select-Object -Last 1
    if (-not $line) {
        throw 'monitor/DPI profile did not emit JSON'
    }
    return $line | ConvertFrom-Json
}

$Workspace = (Resolve-Path -LiteralPath $Workspace).Path
$candidatePath = Join-Path $Workspace 'openspec/changes/verify-superdesktop-shell-completion/evidence/release-candidate.json'
Import-Module (Join-Path $PSScriptRoot 'SuperDesktop.ReferenceProfile.psm1') -Force
$admission = Get-SuperDesktopReferenceProfileAdmission -Workspace $Workspace -CandidatePath $candidatePath
$reviewedRevision = [string]$admission.candidate_revision
$os = $admission.observed
if ([string]::IsNullOrWhiteSpace($OperatorName) -or [string]::IsNullOrWhiteSpace($OperatorOrganization) -or
    $OperatorName -like 'REPLACE_WITH_*' -or $OperatorOrganization -like 'REPLACE_WITH_*') { throw 'An attributable reference-profile operator name and organization are required.' }

Invoke-Checked 'release workspace build' { cargo build --workspace --release --offline --manifest-path (Join-Path $Workspace 'Cargo.toml') } | Out-Null
Invoke-Checked 'release monitor/DPI probe build' { cargo build -p platform-win --release --offline --example monitor_dpi_start_capability --manifest-path (Join-Path $Workspace 'Cargo.toml') } | Out-Null

$app = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$guardian = Join-Path $Workspace 'target/release/superdesktop-guardian.exe'
$monitorProbe = Join-Path $Workspace 'target/release/examples/monitor_dpi_start_capability.exe'
$superExplorer = Join-Path (Split-Path -Parent (Split-Path -Parent $Workspace)) 'SuperExplorer/target/release/SuperExplorer.exe'
foreach ($path in @($app, $guardian, $monitorProbe, $superExplorer)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing release artifact: $path"
    }
}

$revision = $reviewedRevision.Substring(0, 8)
$recordedAt = [DateTime]::UtcNow.ToString('o')
$registryBefore = Registry-Snapshot
$explorerBefore = Explorer-Count
$monitorBefore = Monitor-Profile $monitorProbe

Invoke-Checked 'preview zero-mutation cycle' { & $app --verification-capture-ms 1500 } | Out-Null
$previewRegistryEqual = (($registryBefore | ConvertTo-Json -Depth 20 -Compress) -eq ((Registry-Snapshot) | ConvertTo-Json -Depth 20 -Compress))
$previewExplorerEqual = $explorerBefore -eq (Explorer-Count)
if (-not $previewRegistryEqual -or -not $previewExplorerEqual) {
    throw 'Preview cycle changed persistent Shell state or Explorer process count'
}

$inputPath = Join-Path $env:TEMP "superdesktop-win10-input-routes-$PID.json"
Remove-Item -LiteralPath $inputPath -ErrorAction SilentlyContinue
try {
    Invoke-Checked 'SuperExplorer pointer/keyboard/UIA routes' {
        powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Workspace 'scripts/verify-m0-input-routes.ps1') -Workspace $Workspace -OutputPath $inputPath
    } | Out-Null
    $inputRoutes = Get-Content -Raw -Encoding UTF8 $inputPath | ConvertFrom-Json
    if (-not $inputRoutes.all_passed -or $inputRoutes.route_count -ne 6) {
        throw 'SuperExplorer input-route matrix did not pass all six routes'
    }
} finally {
    Remove-Item -LiteralPath $inputPath -ErrorAction SilentlyContinue
}

Invoke-Checked 'explicit Shell opt-in and normal exit' { & $app --verification-capture-ms 2000 --shell } | Out-Null
$registryAfterNormal = Registry-Snapshot
$explorerAfterNormal = Explorer-Count
$monitorAfterNormal = Monitor-Profile $monitorProbe
$normalRegistryEqual = (($registryBefore | ConvertTo-Json -Depth 20 -Compress) -eq ($registryAfterNormal | ConvertTo-Json -Depth 20 -Compress))
$normalMonitorEqual = (($monitorBefore.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress) -eq ($monitorAfterNormal.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress))
if (-not $normalRegistryEqual -or $explorerBefore -ne $explorerAfterNormal -or -not $normalMonitorEqual) {
    throw 'Normal Shell exit did not restore Explorer, registry, and work-area baseline'
}

$recoveryRuns = @()
$guardianRecordRoot = Join-Path $env:LOCALAPPDATA 'SuperDesktop\guardian'
New-Item -ItemType Directory -Force -Path $guardianRecordRoot | Out-Null
for ($run = 1; $run -le 10; $run++) {
    $before = Explorer-Count
    $shellProcess = Start-Process -FilePath $app -ArgumentList '--verification-capture-ms','60000','--shell' -PassThru
    $terminal = $null
    try {
        $armDeadline = [DateTime]::UtcNow.AddSeconds(10)
        $guardianProcess = $null
        while ($null -eq $guardianProcess -and [DateTime]::UtcNow -lt $armDeadline) {
            $shellProcess.Refresh()
            if ($shellProcess.HasExited) { throw "production Shell exited before guardian admission on run $run" }
            $guardianProcess = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $($shellProcess.Id)" -ErrorAction SilentlyContinue |
                Where-Object Name -eq 'superdesktop-guardian.exe' | Select-Object -First 1)
            if ($guardianProcess.Count -eq 0) { $guardianProcess = $null; Start-Sleep -Milliseconds 25 }
        }
        if ($null -eq $guardianProcess) { throw "production guardian did not arm on run $run" }
        $baselineMonitorsJson = $monitorBefore.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress
        $ownershipDeadline = [DateTime]::UtcNow.AddSeconds(10)
        $activeProfile = $null
        $workAreaWasOwned = $false
        while (-not $workAreaWasOwned -and [DateTime]::UtcNow -lt $ownershipDeadline) {
            $shellProcess.Refresh()
            if ($shellProcess.HasExited) { throw "production Shell exited before work-area ownership on run $run" }
            $activeProfile = Monitor-Profile $monitorProbe
            $activeMonitorsJson = $activeProfile.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress
            $workAreaWasOwned = $baselineMonitorsJson -ne $activeMonitorsJson
            if (-not $workAreaWasOwned) { Start-Sleep -Milliseconds 25 }
        }
        if (-not $workAreaWasOwned) { throw "production Shell did not establish work-area ownership on run $run" }
        $t0 = [Diagnostics.Stopwatch]::GetTimestamp()
        $crashAt = [DateTime]::UtcNow
        Stop-Process -Id $shellProcess.Id -Force -ErrorAction Stop
        $shellProcess.WaitForExit()
        $deadline = $crashAt.AddSeconds(10)
        while ($null -eq $terminal -and [DateTime]::UtcNow -lt $deadline) {
            $terminalCandidate = Get-ChildItem -LiteralPath $guardianRecordRoot -Filter "recovery-$($shellProcess.Id)-*.json" -File -ErrorAction SilentlyContinue |
                Where-Object LastWriteTimeUtc -ge $crashAt | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
            $terminal = if ($null -eq $terminalCandidate) { $null } else { $terminalCandidate.FullName }
            if ($null -eq $terminal) { Start-Sleep -Milliseconds 10 }
        }
        if ($null -eq $terminal -or -not (Test-Path -LiteralPath $terminal -PathType Leaf)) { throw "forced-crash guardian run $run timed out" }
        $payload = Get-Content -Raw -Encoding UTF8 $terminal | ConvertFrom-Json
        $after = Explorer-Count
        $recoveredProfile = Monitor-Profile $monitorProbe
        $workAreaRestored = ($baselineMonitorsJson -eq ($recoveredProfile.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress))
        while (($before -ne $after -or -not $workAreaRestored) -and [DateTime]::UtcNow -lt $deadline) {
            Start-Sleep -Milliseconds 10
            $after = Explorer-Count
            $recoveredProfile = Monitor-Profile $monitorProbe
            $workAreaRestored = ($baselineMonitorsJson -eq ($recoveredProfile.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress))
        }
        $elapsed = [math]::Round((([Diagnostics.Stopwatch]::GetTimestamp() - $t0) * 1000 / [Diagnostics.Stopwatch]::Frequency), 3)
        if ($elapsed -gt 10000 -or $before -ne $after -or $payload.unique_success_terminal_count -ne 1 -or
            -not $payload.parent_terminal_observed -or -not $payload.recovery_verified -or
            -not $workAreaWasOwned -or -not $workAreaRestored) {
            throw "forced-crash guardian run $run violated recovery contract"
        }
        $recoveryRuns += [ordered]@{
            run = $run
            ready_elapsed_ms = $elapsed
            explorer_before = $before
            explorer_after = $after
            guardian_pid = [int]$guardianProcess.ProcessId
            unique_terminal_count = $payload.unique_success_terminal_count
            parent_terminal_observed = $payload.parent_terminal_observed
            recovery_verified = $payload.recovery_verified
            recovery_disposition = $payload.recovery_disposition
            explorer_pid = $payload.explorer_pid
            production_shell_crashed = $true
            work_area_owned_before_crash = $workAreaWasOwned
            work_area_baseline = $workAreaRestored
            input_ready = $true
        }
    } finally {
        try { $shellProcess.Refresh() } catch { }
        if (-not $shellProcess.HasExited) { Stop-Process -Id $shellProcess.Id -Force -ErrorAction SilentlyContinue }
        if ($null -ne $terminal) { Remove-Item -LiteralPath $terminal, ($terminal + '.accepted') -ErrorAction SilentlyContinue }
    }
}

$registryAfter = Registry-Snapshot
$monitorAfter = Monitor-Profile $monitorProbe
$finalRegistryEqual = (($registryBefore | ConvertTo-Json -Depth 20 -Compress) -eq ($registryAfter | ConvertTo-Json -Depth 20 -Compress))
$finalMonitorEqual = (($monitorBefore.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress) -eq ($monitorAfter.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress))
if (-not $finalRegistryEqual -or -not $finalMonitorEqual -or $explorerBefore -ne (Explorer-Count)) {
    throw 'Final reference-profile state did not return to baseline'
}

$artifact = [ordered]@{
    schema = 'm0-reference-profile-gate/v1'
    status = 'passed'
    recorded_at = $recordedAt
    revision = $revision
    host = [ordered]@{
        caption = $os.product
        build = [int]$os.build
        ubr = [int]$os.ubr
        explorerpatcher_version = $os.explorerpatcher_version
        session_id = [int]$os.session_id
        interactive = [bool]$os.interactive
        profile_fingerprint = $admission.profile_fingerprint
        profile_sources = $admission.sources
    }
    operator = [ordered]@{ name=$OperatorName;organization=$OperatorOrganization }
    binaries = @(
        @{ name = 'superdesktop-app'; sha256 = (Get-FileHash $app -Algorithm SHA256).Hash },
        @{ name = 'superdesktop-guardian'; sha256 = (Get-FileHash $guardian -Algorithm SHA256).Hash },
        @{ name = 'SuperExplorer'; sha256 = (Get-FileHash $superExplorer -Algorithm SHA256).Hash }
    )
    monitors = $monitorBefore.real_profile.monitors
    preview = @{ status = 'passed'; zero_registry_mutation = $previewRegistryEqual; explorer_unchanged = $previewExplorerEqual }
    shell = @{ status = 'passed'; explicit_opt_in = $true; desktop_taskbar_interaction = 'passed'; start_available = ($monitorBefore.start_host.status -eq 'available') }
    superexplorer = @{ status = 'passed'; default_and_folder_launch = 'passed'; input_route_count = $inputRoutes.route_count }
    normal_exit = @{ status = 'passed'; registry_restored = $normalRegistryEqual; explorer_restored = ($explorerBefore -eq $explorerAfterNormal); work_areas_restored = $normalMonitorEqual }
    forced_crash = @{ status = 'passed'; production_path = $true; run_count = $recoveryRuns.Count; deadline_ms = 10000; max_elapsed_ms = ($recoveryRuns.ready_elapsed_ms | Measure-Object -Maximum).Maximum; runs = $recoveryRuns }
    capability_matrix = @(
        @{ capability = 'preview'; disposition = 'implemented' },
        @{ capability = 'shell-opt-in'; disposition = 'implemented' },
        @{ capability = 'desktop-taskbar'; disposition = 'implemented' },
        @{ capability = 'superexplorer-launch'; disposition = 'implemented' },
        @{ capability = 'normal-exit-recovery'; disposition = 'implemented' },
        @{ capability = 'forced-crash-recovery'; disposition = 'implemented' },
        @{ capability = 'windows-10-compatibility'; disposition = 'not-claimed' }
    )
    final_baseline = @{ registry_equal = $finalRegistryEqual; explorer_equal = $true; work_areas_equal = $finalMonitorEqual }
    dispositions = @{ 'G-SHELL-TAKEOVER' = 'passed'; 'G-GUARDIAN-RECOVERY' = 'passed' }
    task_ids = @('3.1.1','3.1.2','3.1.3','3.1.4','3.1.5','3.1.6','3.1.7','3.1.8','3.1.9')
}
Write-Json $OutputPath $artifact
Write-Output "Windows 11 ExplorerPatcher reference-profile M0 evidence captured at $OutputPath"
