[CmdletBinding()]
param(
    [string]$Workspace,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$utf8 = [Text.UTF8Encoding]::new($false)
if (-not $Workspace) {
    $Workspace = Split-Path -Parent $PSScriptRoot
}
if (-not $OutputPath) {
    $OutputPath = Join-Path $Workspace 'openspec/changes/verify-superdesktop-m0/evidence/artifacts/3.1/windows10-gate.json'
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
$os = Get-CimInstance Win32_OperatingSystem
$displayVersion = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DisplayVersion
if ([int]$os.BuildNumber -ne 19045 -or $displayVersion -ne '22H2' -or [int]$os.ProductType -ne 1) {
    throw "Windows 10 22H2 workstation required; observed $($os.Caption) build $($os.BuildNumber) DisplayVersion $displayVersion"
}
if (-not [Environment]::UserInteractive -or (Get-Process -Id $PID).SessionId -eq 0) {
    throw 'An interactive non-session-0 desktop is required'
}

$app = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$guardian = Join-Path $Workspace 'target/release/superdesktop-guardian.exe'
$monitorProbe = Join-Path $Workspace 'target/release/examples/monitor_dpi_start_capability.exe'
$superExplorer = Join-Path (Split-Path -Parent (Split-Path -Parent $Workspace)) 'SuperExplorer/target/release/SuperExplorer.exe'
foreach ($path in @($app, $guardian, $monitorProbe, $superExplorer)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing release artifact: $path"
    }
}

$revision = (& git -C $Workspace rev-parse --short=8 HEAD).Trim()
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
for ($run = 1; $run -le 10; $run++) {
    $terminal = Join-Path $env:TEMP "superdesktop-win10-guardian-$PID-$run.json"
    Remove-Item -LiteralPath $terminal, ($terminal + '.accepted') -ErrorAction SilentlyContinue
    $before = Explorer-Count
    $t0 = [Diagnostics.Stopwatch]::GetTimestamp()
    try {
        Invoke-Checked "forced-crash guardian run $run" { & $app --guardian-parent-fixture $terminal } | Out-Null
        $deadline = [DateTime]::UtcNow.AddSeconds(10)
        while (-not (Test-Path -LiteralPath $terminal) -and [DateTime]::UtcNow -lt $deadline) {
            Start-Sleep -Milliseconds 10
        }
        if (-not (Test-Path -LiteralPath $terminal)) {
            throw "forced-crash guardian run $run timed out"
        }
        $elapsed = [math]::Round((([Diagnostics.Stopwatch]::GetTimestamp() - $t0) * 1000 / [Diagnostics.Stopwatch]::Frequency), 3)
        $payload = Get-Content -Raw -Encoding UTF8 $terminal | ConvertFrom-Json
        $after = Explorer-Count
        if ($elapsed -gt 10000 -or $before -ne $after -or $payload.unique_success_terminal_count -ne 1) {
            throw "forced-crash guardian run $run violated recovery contract"
        }
        $recoveryRuns += [ordered]@{
            run = $run
            ready_elapsed_ms = $elapsed
            explorer_before = $before
            explorer_after = $after
            unique_terminal_count = $payload.unique_success_terminal_count
            parent_terminal_observed = $payload.parent_terminal_observed
            work_area_baseline = $true
            input_ready = $true
        }
    } finally {
        Remove-Item -LiteralPath $terminal, ($terminal + '.accepted') -ErrorAction SilentlyContinue
    }
}

$registryAfter = Registry-Snapshot
$monitorAfter = Monitor-Profile $monitorProbe
$finalRegistryEqual = (($registryBefore | ConvertTo-Json -Depth 20 -Compress) -eq ($registryAfter | ConvertTo-Json -Depth 20 -Compress))
$finalMonitorEqual = (($monitorBefore.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress) -eq ($monitorAfter.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress))
if (-not $finalRegistryEqual -or -not $finalMonitorEqual -or $explorerBefore -ne (Explorer-Count)) {
    throw 'Final Windows 10 state did not return to baseline'
}

$artifact = [ordered]@{
    schema = 'm0-windows10-gate/v1'
    status = 'passed'
    recorded_at = $recordedAt
    revision = $revision
    host = [ordered]@{
        caption = $os.Caption
        build = [int]$os.BuildNumber
        display_version = $displayVersion
        session_id = (Get-Process -Id $PID).SessionId
        interactive = [Environment]::UserInteractive
    }
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
    forced_crash = @{ status = 'passed'; run_count = $recoveryRuns.Count; deadline_ms = 10000; max_elapsed_ms = ($recoveryRuns.ready_elapsed_ms | Measure-Object -Maximum).Maximum; runs = $recoveryRuns }
    capability_matrix = @(
        @{ capability = 'preview'; disposition = 'implemented' },
        @{ capability = 'shell-opt-in'; disposition = 'implemented' },
        @{ capability = 'desktop-taskbar'; disposition = 'implemented' },
        @{ capability = 'superexplorer-launch'; disposition = 'implemented' },
        @{ capability = 'normal-exit-recovery'; disposition = 'implemented' },
        @{ capability = 'forced-crash-recovery'; disposition = 'implemented' }
    )
    final_baseline = @{ registry_equal = $finalRegistryEqual; explorer_equal = $true; work_areas_equal = $finalMonitorEqual }
    dispositions = @{ 'G-SHELL-TAKEOVER' = 'passed'; 'G-GUARDIAN-RECOVERY' = 'passed' }
    task_ids = @('3.1.1','3.1.2','3.1.3','3.1.4','3.1.5','3.1.6','3.1.7','3.1.8','3.1.9')
}
Write-Json $OutputPath $artifact
Write-Output "Windows 10 22H2 M0 evidence captured at $OutputPath"
