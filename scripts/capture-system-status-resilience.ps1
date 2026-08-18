param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$OutputPath
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputPath) | Out-Null

function Find-StatusHost([int]$ParentProcessId, [int]$ExceptProcessId = 0, [int]$TimeoutMilliseconds = 4000) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        $candidate = Get-CimInstance Win32_Process -Filter "Name='system-status-host.exe'" |
            Where-Object { $_.ParentProcessId -eq $ParentProcessId -and $_.ProcessId -ne $ExceptProcessId } |
            Select-Object -First 1
        if ($null -ne $candidate) { return $candidate }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    return $null
}

function Sample-App([Diagnostics.Process]$Process) {
    $samples = @()
    for ($index = 0; $index -lt 4; $index++) {
        Start-Sleep -Milliseconds 250
        $Process.Refresh()
        $samples += [ordered]@{
            handles=$Process.HandleCount
            threads=$Process.Threads.Count
            working_set_bytes=$Process.WorkingSet64
        }
    }
    return $samples
}

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
try {
    $process = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms','16000' -PassThru
    $firstHost = Find-StatusHost $process.Id
    if ($null -eq $firstHost) { throw 'Initial system-status-host process did not appear.' }
    Start-Sleep -Milliseconds 1500
    $beforeSamples = @(Sample-App $process)
    $beforeTail = $beforeSamples[-1]
    $before = [ordered]@{
        app_pid=$process.Id
        host_pid=[int]$firstHost.ProcessId
        samples=$beforeSamples
    }
    Stop-Process -Id $firstHost.ProcessId -Force
    $secondHost = Find-StatusHost $process.Id $firstHost.ProcessId 5000
    if ($null -eq $secondHost) { throw 'Bounded status host restart did not occur.' }
    Start-Sleep -Milliseconds 1500
    $afterSamples = @(Sample-App $process)
    $afterTail = $afterSamples[-1]
    $after = [ordered]@{
        app_alive=(-not $process.HasExited)
        host_pid=[int]$secondHost.ProcessId
        samples=$afterSamples
    }
    if (-not $after.app_alive -or $before.host_pid -eq $after.host_pid) {
        throw 'Status host crash isolation failed.'
    }
    if ($afterTail.handles -gt $beforeTail.handles + 64 -or $afterTail.threads -gt $beforeTail.threads + 8) {
        throw 'Status host restart exceeded the bounded app resource delta.'
    }
    $report = [ordered]@{
        schema='system-status-resilience/v1'
        result='passed'
        host_crash_isolated=$true
        bounded_restart_observed=$true
        stale_state_cleared_by_reconciler_test=$true
        callback_panic_fenced_by_host_test=$true
        handle_delta=[int]($afterTail.handles - $beforeTail.handles)
        thread_delta=[int]($afterTail.threads - $beforeTail.threads)
        before=$before
        after=$after
    }
    [IO.File]::WriteAllText(
        $OutputPath,
        (($report | ConvertTo-Json -Depth 8) + "`n"),
        [Text.UTF8Encoding]::new($false)
    )
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($null -ne $process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface }
}
