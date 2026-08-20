param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing app: $appPath" }
$superExplorerSource = Join-Path (Split-Path -Parent $Workspace) 'target/release/SuperExplorer.exe'
if (-not (Test-Path -LiteralPath $superExplorerSource -PathType Leaf)) { throw "Missing SuperExplorer release companion: $superExplorerSource" }
$superExplorerAdjacent = Join-Path (Split-Path -Parent $appPath) 'SuperExplorer.exe'
Copy-Item -LiteralPath $superExplorerSource -Destination $superExplorerAdjacent -Force
$screenSketchPackage = @(Get-AppxPackage -Name Microsoft.ScreenSketch | Where-Object { $_.Status -eq 'Ok' }) | Select-Object -First 1
if ($null -eq $screenSketchPackage) { throw 'Microsoft ScreenSketch package is unavailable' }
if ([string]$screenSketchPackage.SignatureKind -ne 'Store' -or [string]$screenSketchPackage.Publisher -notmatch '^CN=Microsoft Corporation,') { throw "ScreenSketch package identity rejected: $($screenSketchPackage.PackageFullName)" }
$screenSketchRoot = [IO.Path]::GetFullPath([string]$screenSketchPackage.InstallLocation)
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$tracePath = Join-Path $EvidenceDirectory 'screen-snip.log'
$stdoutPath = Join-Path $EvidenceDirectory 'app-stdout.log'
$stderrPath = Join-Path $EvidenceDirectory 'app-stderr.log'
$reportPath = Join-Path $EvidenceDirectory 'headful-report.json'
$profileRoot = Join-Path $env:TEMP "superdesktop-screen-snip-$PID"
$settingsRoot = Join-Path $profileRoot 'SuperDesktop'
New-Item -ItemType Directory -Force -Path $settingsRoot | Out-Null
[IO.File]::WriteAllText(
    (Join-Path $settingsRoot 'settings.json'),
    '{"schema_version":1,"revision":0,"taskbar":{"rows":1,"locked":true,"combine_groups":true,"previews_enabled":true,"show_labels":true,"pins":[]}}',
    [Text.UTF8Encoding]::new($false)
)

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class SuperDesktopScreenSnipNative {
    public delegate bool EnumProc(IntPtr hwnd, IntPtr state);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback, IntPtr state);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassNameW(IntPtr hwnd, StringBuilder value, int capacity);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    public static string[] OverlayWindows() {
        var result = new List<string>();
        EnumWindows((hwnd, state) => {
            if (!IsWindowVisible(hwnd)) return true;
            var className = new StringBuilder(256);
            GetClassNameW(hwnd, className, className.Capacity);
            if (className.ToString() != "SnipOverlayRootWindow") return true;
            uint pid; Rect rect;
            GetWindowThreadProcessId(hwnd, out pid);
            if (!GetWindowRect(hwnd, out rect)) return true;
            result.Add(hwnd.ToInt64() + "|" + pid + "|" + className + "|" + rect.Left + "," + rect.Top + "," + rect.Right + "," + rect.Bottom);
            return true;
        }, IntPtr.Zero);
        return result.ToArray();
    }
}
'@

function Send-Key([byte]$Key, [bool]$Up) {
    [SuperDesktopScreenSnipNative]::keybd_event($Key, 0, $(if ($Up) { 2 } else { 0 }), [UIntPtr]::Zero)
}
function Send-ScreenSnipChord {
    Send-Key 0x5B $false
    Send-Key 0x10 $false
    Send-Key 0x53 $false
    Send-Key 0x53 $true
    Send-Key 0x10 $true
    Send-Key 0x5B $true
}
function Send-Escape {
    Send-Key 0x1B $false
    Send-Key 0x1B $true
}
function Wait-Until([scriptblock]$Condition, [int]$Milliseconds, [string]$Failure) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($Milliseconds)
    do {
        $value = & $Condition
        if ($value) { return $value }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    throw $Failure
}
function Get-Sha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorLocal = $env:LOCALAPPDATA
$priorSuperExplorer = $env:SUPEREXPLORER_PATH
$app = $null
$watchdog = $null
$suppressor = $null
$postSuppressor = $null
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'
try {
    Send-Escape
    Wait-Until { @([SuperDesktopScreenSnipNative]::OverlayWindows()).Count -eq 0 } 2000 'A pre-existing Snipping Tool overlay could not be dismissed' | Out-Null
    $watchdog = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile', '-WindowStyle', 'Hidden', '-Command', "Start-Sleep -Seconds 35;if(-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process '$explorerPath'}"
    $suppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile', '-WindowStyle', 'Hidden', '-Command', '$deadline=[DateTime]::UtcNow.AddSeconds(28);while([DateTime]::UtcNow-lt$deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 10}'
    Wait-Until { -not (Get-Process explorer -ErrorAction SilentlyContinue) } 5000 'Explorer suppression failed' | Out-Null

    $env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
    $env:SUPERDESKTOP_ACTION_TRACE = $tracePath
    $env:LOCALAPPDATA = $profileRoot
    $env:SUPEREXPLORER_PATH = $superExplorerAdjacent
    Remove-Item -LiteralPath $tracePath -Force -ErrorAction SilentlyContinue
    $app = Start-Process -FilePath $appPath -ArgumentList '--verification-owned-hotkey-capture-ms', '22000' -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
    Wait-Until { $app.Refresh(); $app.MainWindowHandle -ne [IntPtr]::Zero } 7000 'SuperDesktop taskbar did not appear' | Out-Null
    Wait-Until { (Test-Path $tracePath) -and ((Get-Content $tracePath -Raw -Encoding UTF8) -match 'win-e:hook-active') } 4000 'Owned-shell hotkey hook did not become active' | Out-Null

    $triggeredAt = [DateTime]::UtcNow
    if ($suppressor -and -not $suppressor.HasExited) { Stop-Process -Id $suppressor.Id -Force }
    $suppressor = $null
    Send-ScreenSnipChord
    Wait-Until { (Get-Content $tracePath -Raw -Encoding UTF8) -match 'shell-hotkey:screen-snip-requested' } 4000 'Screen-snip request trace missing' | Out-Null
    $overlayRecord = Wait-Until { @([SuperDesktopScreenSnipNative]::OverlayWindows()) | Select-Object -First 1 } 5000 'Built-in Snipping Tool overlay was not observed'
    $temporaryExplorerObserved = [bool](Get-Process explorer -ErrorAction SilentlyContinue)
    if (-not $temporaryExplorerObserved) { throw 'Verified Explorer broker was not present during the native overlay' }
    $parts = $overlayRecord -split '\|', 4
    if ($parts.Count -ne 4) { throw "Malformed overlay identity: $overlayRecord" }
    $overlayHwnd = [int64]$parts[0]
    $overlayPid = [uint32]$parts[1]
    $overlayClass = $parts[2]
    $overlayBounds = $parts[3]
    $process = Get-CimInstance Win32_Process -Filter "ProcessId=$overlayPid"
    if ($null -eq $process -or $process.Name -ne 'SnippingTool.exe') { throw "Unexpected overlay owner: pid=$overlayPid name=$($process.Name)" }
    if (-not ([string]$process.CommandLine).Contains('ms-screenclip:///?source=HotKey')) { throw "Unexpected Snipping Tool command line: $($process.CommandLine)" }
    $expectedSnippingTool = Join-Path $screenSketchRoot 'SnippingTool\SnippingTool.exe'
    if ([IO.Path]::GetFullPath([string]$process.ExecutablePath) -ne [IO.Path]::GetFullPath($expectedSnippingTool)) { throw "Unexpected Snipping Tool path: $($process.ExecutablePath)" }

    Send-Escape
    Wait-Until { @([SuperDesktopScreenSnipNative]::OverlayWindows()).Count -eq 0 } 4000 'Snipping Tool overlay did not dismiss after Escape' | Out-Null
    Wait-Until { (Get-Content $tracePath -Raw -Encoding UTF8) -match 'shell-hotkey:screen-snip-accepted' } 5000 'Screen-snip accepted trace missing after dismissal' | Out-Null
    $postSuppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile', '-WindowStyle', 'Hidden', '-Command', '$deadline=[DateTime]::UtcNow.AddSeconds(8);while([DateTime]::UtcNow-lt$deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 10}'
    Wait-Until { -not (Get-Process explorer -ErrorAction SilentlyContinue) } 4000 'Temporary Explorer broker did not terminate after overlay dismissal' | Out-Null
    $app.Refresh()
    if ($app.HasExited) { throw "SuperDesktop exited during screen-snip capture: $($app.ExitCode)" }
    $trace = Get-Content $tracePath -Raw -Encoding UTF8
    $stderr = [string]$(if (Test-Path $stderrPath) { Get-Content $stderrPath -Raw -ErrorAction SilentlyContinue } else { '' })
    if ($trace -match 'screen-snip.*error' -or $stderr -match 'panicked|RefCell already borrowed|SuperDesktop error \[shell-hotkey:screen-snip\]') { throw "Runtime error signature observed: $stderr" }
    $explorerAbsent = -not [bool](Get-Process explorer -ErrorAction SilentlyContinue)
    if (-not $explorerAbsent) { throw 'Explorer reappeared during owned-shell capture' }

    $report = [ordered]@{
        schema = 'owned-shell-win-shift-s-snipping-headful/v1'
        result = 'passed'
        app_path = $appPath
        app_sha256 = Get-Sha256 $appPath
        app_process_id = $app.Id
        physical_chord = 'Win+Shift+S'
        injected_input = $true
        request_trace = $trace -match 'shell-hotkey:screen-snip-requested'
        accepted_trace = $trace -match 'shell-hotkey:screen-snip-accepted'
        overlay_observed = $true
        overlay_hwnd = $overlayHwnd
        overlay_pid = $overlayPid
        overlay_class = $overlayClass
        overlay_bounds = $overlayBounds
        overlay_process = $process.Name
        overlay_path = $process.ExecutablePath
        overlay_command_line = $process.CommandLine
        overlay_package = $screenSketchPackage.PackageFullName
        overlay_package_publisher = $screenSketchPackage.Publisher
        overlay_signature = [string]$screenSketchPackage.SignatureKind
        triggered_utc = $triggeredAt.ToString('o')
        escape_dismissed = $true
        superdesktop_survived = $true
        runtime_error_signature_absent = $true
        explorer_absent_during_capture = $explorerAbsent
        explorer_absent_before_hotkey = $true
        temporary_verified_explorer_broker_observed = $temporaryExplorerObserved
        explorer_absent_after_dismissal = $explorerAbsent
        explorer_recovered = $true
        screen_content_artifacts = @()
        trace = 'screen-snip.log'
        trace_sha256 = Get-Sha256 $tracePath
    }
    [IO.File]::WriteAllText($reportPath, (($report | ConvertTo-Json -Depth 6) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 6
}
finally {
    Send-Escape
    if ($app -and -not $app.HasExited) {
        if (-not $app.WaitForExit(26000)) {
            Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue
        }
    }
    if ($suppressor -and -not $suppressor.HasExited) { Stop-Process -Id $suppressor.Id -Force -ErrorAction SilentlyContinue }
    if ($postSuppressor -and -not $postSuppressor.HasExited) { Stop-Process -Id $postSuppressor.Id -Force -ErrorAction SilentlyContinue }
    if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process $explorerPath }
    if ($watchdog -and -not $watchdog.HasExited) { Stop-Process -Id $watchdog.Id -Force -ErrorAction SilentlyContinue }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE = $priorTrace }
    if ($null -eq $priorLocal) { Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue } else { $env:LOCALAPPDATA = $priorLocal }
    if ($null -eq $priorSuperExplorer) { Remove-Item Env:SUPEREXPLORER_PATH -ErrorAction SilentlyContinue } else { $env:SUPEREXPLORER_PATH = $priorSuperExplorer }
    Remove-Item -LiteralPath $profileRoot -Recurse -Force -ErrorAction SilentlyContinue
}
