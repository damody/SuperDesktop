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
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$tracePath = Join-Path $EvidenceDirectory 'win-key-start.log'
$stdoutPath = Join-Path $EvidenceDirectory 'app-stdout.log'
$stderrPath = Join-Path $EvidenceDirectory 'app-stderr.log'
$screenshotPath = Join-Path $EvidenceDirectory 'start-open.png'
$reportPath = Join-Path $EvidenceDirectory 'headful-report.json'
$profileRoot = Join-Path $EvidenceDirectory 'profile'
$settingsRoot = Join-Path $profileRoot 'SuperDesktop'
New-Item -ItemType Directory -Force -Path $settingsRoot | Out-Null
[IO.File]::WriteAllText(
    (Join-Path $settingsRoot 'settings.json'),
    '{"schema_version":1,"revision":0,"taskbar":{"rows":1,"locked":true,"combine_groups":true,"previews_enabled":true,"show_labels":true,"alignment":"left","pins":[]}}',
    [Text.UTF8Encoding]::new($false)
)

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
public static class SuperDesktopWinKeyNative {
    public delegate bool EnumProc(IntPtr hwnd, IntPtr state);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback, IntPtr state);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    public static long[] VisibleForProcess(uint wanted) {
        var result = new List<long>();
        EnumWindows((hwnd, state) => {
            uint pid;
            GetWindowThreadProcessId(hwnd, out pid);
            if (pid == wanted && IsWindowVisible(hwnd)) result.Add(hwnd.ToInt64());
            return true;
        }, IntPtr.Zero);
        return result.ToArray();
    }
    public static int[] WindowRect(long value) {
        Rect rect;
        if (!GetWindowRect(new IntPtr(value), out rect)) throw new Win32Exception();
        return new [] { rect.Left, rect.Top, rect.Right, rect.Bottom };
    }
}
'@

function Send-WindowsKey {
    [SuperDesktopWinKeyNative]::keybd_event(0x5B, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 45
    [SuperDesktopWinKeyNative]::keybd_event(0x5B, 0, 2, [UIntPtr]::Zero)
}
function Find-OwnedStartHandle([uint32]$ProcessId, [long[]]$BaselineHandles) {
    $labels = @(
        'Pinned',
        'All apps',
        [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('5bey6YeY6YG4')),
        [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('5omA5pyJ5oeJ55So56iL5byP'))
    )
    foreach ($handle in @([SuperDesktopWinKeyNative]::VisibleForProcess($ProcessId) | Where-Object { $_ -notin $BaselineHandles })) {
        try { $root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$handle) }
        catch { continue }
        if ($null -eq $root) { continue }
        foreach ($label in $labels) {
            $condition = [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::NameProperty,
                $label
            )
            if ($null -ne $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)) {
                return [long]$handle
            }
        }
    }
    return [long]0
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
    $stream = [IO.File]::OpenRead($Path)
    try {
        $hash = [Security.Cryptography.SHA256]::Create()
        try { ([BitConverter]::ToString($hash.ComputeHash($stream))).Replace('-', '') }
        finally { $hash.Dispose() }
    }
    finally { $stream.Dispose() }
}
function Save-Window([long]$Handle, [string]$Path) {
    $rect = [SuperDesktopWinKeyNative]::WindowRect($Handle)
    $width = $rect[2] - $rect[0]
    $height = $rect[3] - $rect[1]
    if ($width -le 0 -or $height -le 0) { throw "Start window has invalid bounds: $($rect -join ',')" }
    $bitmap = [Drawing.Bitmap]::new($width, $height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($rect[0], $rect[1], 0, 0, $bitmap.Size)
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
    $rect
}
function Get-ShellSnapshot {
    $key = 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'
    $property = Get-ItemProperty -LiteralPath $key -Name Shell -ErrorAction SilentlyContinue
    [ordered]@{ present = $null -ne $property; value = $(if ($null -ne $property) { [string]$property.Shell } else { $null }) }
}

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorLocal = $env:LOCALAPPDATA
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'
$priorExplorer = [bool](Get-Process explorer -ErrorAction SilentlyContinue)
$shellBefore = Get-ShellSnapshot
$app = $null
$watchdog = $null
$suppressor = $null
$failure = $null
$observed = [ordered]@{}
try {
    if ($priorExplorer) {
        $watchdog = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile', '-WindowStyle', 'Hidden', '-Command', "Start-Sleep -Seconds 35;if(-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process '$explorerPath'}"
    }
    $suppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile', '-WindowStyle', 'Hidden', '-Command', '$deadline=[DateTime]::UtcNow.AddSeconds(24);while([DateTime]::UtcNow-lt$deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 10}'
    Wait-Until { -not (Get-Process explorer -ErrorAction SilentlyContinue) } 6000 'Explorer suppression failed' | Out-Null

    $env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
    $env:SUPERDESKTOP_ACTION_TRACE = $tracePath
    $env:LOCALAPPDATA = $profileRoot
    Remove-Item -LiteralPath $tracePath -Force -ErrorAction SilentlyContinue
    $app = Start-Process -FilePath $appPath -ArgumentList '--verification-owned-hotkey-capture-ms', '18000' -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
    Wait-Until { $app.Refresh(); @([SuperDesktopWinKeyNative]::VisibleForProcess([uint32]$app.Id)).Count -gt 0 } 7000 'SuperDesktop taskbar did not appear' | Out-Null
    Wait-Until { (Test-Path $tracePath) -and ((Get-Content $tracePath -Raw -Encoding UTF8) -match 'win-e:hook-active') } 4000 'Owned-shell hotkey hook did not become active' | Out-Null
    $baselineHandles = @([SuperDesktopWinKeyNative]::VisibleForProcess([uint32]$app.Id))

    Send-WindowsKey
    $startHandle = [long](Wait-Until {
        $candidate = Find-OwnedStartHandle ([uint32]$app.Id) $baselineHandles
        if ($candidate -ne 0) { $candidate } else { $null }
    } 5000 'Standalone Win key did not open an owned Start window')
    Wait-Until {
        $trace = Get-Content $tracePath -Raw -Encoding UTF8
        $trace -match 'start:owned-opened' -and $trace -match 'shell-hotkey:start-toggle'
    } 3000 'Owned Start open/toggle traces are missing' | Out-Null
    $startBounds = Save-Window $startHandle $screenshotPath

    Send-WindowsKey
    Wait-Until { (Find-OwnedStartHandle ([uint32]$app.Id) $baselineHandles) -eq 0 } 5000 'Second standalone Win key did not close owned Start' | Out-Null
    Wait-Until {
        $trace = Get-Content $tracePath -Raw -Encoding UTF8
        $trace -match 'start:closed' -and ([regex]::Matches($trace, 'shell-hotkey:start-toggle')).Count -eq 2
    } 3000 'Owned Start close or exact toggle traces are missing' | Out-Null
    $app.Refresh()
    if ($app.HasExited) { throw "SuperDesktop exited during Win-key Start verification: $($app.ExitCode)" }
    if (Get-Process explorer -ErrorAction SilentlyContinue) { throw 'Explorer reappeared during owned-shell Win-key verification' }
    $stderr = [string]$(if (Test-Path $stderrPath) { Get-Content $stderrPath -Raw -ErrorAction SilentlyContinue } else { '' })
    if ($stderr -match 'panicked|RefCell already borrowed|SuperDesktop error \[shell-hotkey:start-toggle\]') { throw "Runtime error signature observed: $stderr" }
    $observed = [ordered]@{
        app_process_id = $app.Id
        start_handle = $startHandle
        start_bounds = $startBounds
        open_observed = $true
        close_observed = $true
        exact_toggle_trace_count = 2
        superdesktop_survived = $true
        explorer_absent_during_capture = $true
        runtime_error_signature_absent = $true
    }
}
catch {
    $failure = $_
}
finally {
    if ($app -and -not $app.HasExited) { Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue }
    if ($suppressor -and -not $suppressor.HasExited) { Stop-Process -Id $suppressor.Id -Force -ErrorAction SilentlyContinue }
    if ($priorExplorer -and -not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process $explorerPath }
    if ($priorExplorer) {
        try { Wait-Until { [bool](Get-Process explorer -ErrorAction SilentlyContinue) } 7000 'Explorer recovery timed out' | Out-Null }
        catch { if ($null -eq $failure) { $failure = $_ } }
    }
    if ($watchdog -and -not $watchdog.HasExited) { Stop-Process -Id $watchdog.Id -Force -ErrorAction SilentlyContinue }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE = $priorTrace }
    if ($null -eq $priorLocal) { Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue } else { $env:LOCALAPPDATA = $priorLocal }
}

$shellAfter = Get-ShellSnapshot
$shellRestored = $shellBefore.present -eq $shellAfter.present -and $shellBefore.value -eq $shellAfter.value
$explorerRestored = ([bool](Get-Process explorer -ErrorAction SilentlyContinue)) -eq $priorExplorer
if (-not $shellRestored -and $null -eq $failure) { $failure = [Exception]::new('Winlogon Shell changed during verification') }
if (-not $explorerRestored -and $null -eq $failure) { $failure = [Exception]::new('Explorer state was not restored') }
$report = [ordered]@{
    schema = 'owned-win-key-start-toggle-headful/v1'
    result = $(if ($null -eq $failure) { 'passed' } else { 'failed' })
    app_path = $appPath
    app_sha256 = Get-Sha256 $appPath
    physical_gesture = 'Left Windows key down/up twice'
    injected_input = $true
    observed = $observed
    screenshot = $(if (Test-Path $screenshotPath) { 'start-open.png' } else { $null })
    screenshot_sha256 = $(if (Test-Path $screenshotPath) { Get-Sha256 $screenshotPath } else { $null })
    trace = $(if (Test-Path $tracePath) { 'win-key-start.log' } else { $null })
    trace_sha256 = $(if (Test-Path $tracePath) { Get-Sha256 $tracePath } else { $null })
    shell_before = $shellBefore
    shell_after = $shellAfter
    shell_restored = $shellRestored
    explorer_present_before = $priorExplorer
    explorer_restored = $explorerRestored
    failure = $(if ($null -eq $failure) { $null } else { [string]$failure })
}
[IO.File]::WriteAllText($reportPath, (($report | ConvertTo-Json -Depth 8) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
$report | ConvertTo-Json -Depth 8
if ($null -ne $failure) { throw $failure }
