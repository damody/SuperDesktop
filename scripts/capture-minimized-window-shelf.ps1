param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$fixturePath = Join-Path $Workspace 'target/release/taskbar-progress-fixture.exe'
$superExplorerSource = Join-Path (Split-Path -Parent $Workspace) 'target/release/SuperExplorer.exe'
foreach ($required in @($appPath, $fixturePath, $superExplorerSource)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Missing release binary: $required" }
}
$superExplorerAdjacent = Join-Path (Split-Path -Parent $appPath) 'SuperExplorer.exe'
Copy-Item -LiteralPath $superExplorerSource -Destination $superExplorerAdjacent -Force
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$tracePath = Join-Path $EvidenceDirectory 'minimized-window-shelf.log'
$reportPath = Join-Path $EvidenceDirectory 'headful-report.json'
$screenshotPath = Join-Path $EvidenceDirectory 'minimized-desktop.png'
$stdoutPath = Join-Path $EvidenceDirectory 'app-stdout.log'
$stderrPath = Join-Path $EvidenceDirectory 'app-stderr.log'
$fixtureStdout = Join-Path $EvidenceDirectory 'fixture-stdout.log'
$fixtureStderr = Join-Path $EvidenceDirectory 'fixture-stderr.log'
$profileRoot = Join-Path $EvidenceDirectory 'profile'
$settingsRoot = Join-Path $profileRoot 'SuperDesktop'
New-Item -ItemType Directory -Force -Path $settingsRoot | Out-Null
[IO.File]::WriteAllText(
    (Join-Path $settingsRoot 'settings.json'),
    '{"schema_version":1,"revision":0,"taskbar":{"rows":1,"locked":true,"combine_groups":true,"previews_enabled":true,"show_labels":true,"alignment":"left","pins":[]}}',
    [Text.UTF8Encoding]::new($false)
)

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SuperDesktopMinimizedShelfNative {
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct WindowPlacement {
        public uint Length, Flags, ShowCmd;
        public Point MinPosition, MaxPosition;
        public Rect NormalPosition;
    }
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool GetWindowPlacement(IntPtr hwnd, ref WindowPlacement placement);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd, uint message, UIntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern void SwitchToThisWindow(IntPtr hwnd, bool altTab);
    public static Rect RectFor(IntPtr hwnd) {
        Rect rect;
        if (!GetWindowRect(hwnd, out rect)) throw new InvalidOperationException("GetWindowRect");
        return rect;
    }
    public static WindowPlacement PlacementFor(IntPtr hwnd) {
        WindowPlacement placement = new WindowPlacement();
        placement.Length = (uint)Marshal.SizeOf<WindowPlacement>();
        if (!GetWindowPlacement(hwnd, ref placement)) throw new InvalidOperationException("GetWindowPlacement");
        return placement;
    }
}
'@
[SuperDesktopMinimizedShelfNative]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

function Wait-Until([scriptblock]$Condition, [int]$Milliseconds, [string]$Failure) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($Milliseconds)
    do {
        $value = & $Condition
        if ($value) { return $value }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    throw $Failure
}
function Find-TaskButton([int]$ProcessId, [string]$Title) {
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    for ($windowIndex = 0; $windowIndex -lt $windows.Count; $windowIndex++) {
        $window = $windows.Item($windowIndex)
        if ($window.Current.ProcessId -ne $ProcessId) { continue }
        $buttons = $window.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::Button
            )
        )
        for ($buttonIndex = 0; $buttonIndex -lt $buttons.Count; $buttonIndex++) {
            $button = $buttons.Item($buttonIndex)
            if ([string]$button.Current.Name -like "*$Title*") { return $button }
        }
    }
    return $null
}
function Wait-TaskButton([int]$ProcessId, [string]$Title) {
    Wait-Until { Find-TaskButton $ProcessId $Title } 8000 "Task button not found: $Title"
}
function Invoke-TaskButton([int]$ProcessId, [string]$Title) {
    $button = Wait-TaskButton $ProcessId $Title
    $pattern = [System.Windows.Automation.InvokePattern]$button.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern
    )
    $pattern.Invoke()
}
function Rect-Array($Rect) { @($Rect.Left, $Rect.Top, $Rect.Right, $Rect.Bottom) }
function Assert-RectNear($Actual, $Expected, [int]$Tolerance, [string]$Label) {
    $actualValues = Rect-Array $Actual
    $expectedValues = Rect-Array $Expected
    for ($index = 0; $index -lt 4; $index++) {
        if ([Math]::Abs($actualValues[$index] - $expectedValues[$index]) -gt $Tolerance) {
            throw "$Label bounds changed: actual=$($actualValues -join ',') expected=$($expectedValues -join ',')"
        }
    }
}
function Save-Desktop([string]$Path) {
    $bounds = [Windows.Forms.SystemInformation]::VirtualScreen
    $bitmap = [Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bitmap.Size)
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    }
    finally { $graphics.Dispose(); $bitmap.Dispose() }
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
function Get-ShellSnapshot {
    $key = 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'
    $property = Get-ItemProperty -LiteralPath $key -Name Shell -ErrorAction SilentlyContinue
    [ordered]@{ present = $null -ne $property; value = $(if ($null -ne $property) { [string]$property.Shell } else { $null }) }
}

$title = 'Taskbar Progress Fixture'
$fixtureMinimizeMessage = 0x8000 + 41
$fixtureRestoreMessage = 0x8000 + 42
$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorLocal = $env:LOCALAPPDATA
$priorSuperExplorer = $env:SUPEREXPLORER_PATH
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'
$priorExplorer = [bool](Get-Process explorer -ErrorAction SilentlyContinue)
$shellBefore = Get-ShellSnapshot
$app = $null
$fixture = $null
$watchdog = $null
$suppressor = $null
$failure = $null
$observed = [ordered]@{}
try {
    if ($priorExplorer) {
        $watchdog = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile', '-WindowStyle', 'Hidden', '-Command', "Start-Sleep -Seconds 45;if(-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process '$explorerPath'}"
    }
    $suppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile', '-WindowStyle', 'Hidden', '-Command', '$deadline=[DateTime]::UtcNow.AddSeconds(35);while([DateTime]::UtcNow-lt$deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 10}'
    Wait-Until { -not (Get-Process explorer -ErrorAction SilentlyContinue) } 7000 'Explorer suppression failed' | Out-Null

    $env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
    $env:SUPERDESKTOP_ACTION_TRACE = $tracePath
    $env:LOCALAPPDATA = $profileRoot
    $env:SUPEREXPLORER_PATH = $superExplorerAdjacent
    Remove-Item -LiteralPath $tracePath -Force -ErrorAction SilentlyContinue
    $app = Start-Process -FilePath $appPath -ArgumentList '--verification-owned-hotkey-capture-ms', '30000' -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
    Wait-Until { $app.Refresh(); $app.MainWindowHandle -ne [IntPtr]::Zero } 7000 'SuperDesktop taskbar did not appear' | Out-Null
    Wait-Until { (Test-Path $tracePath) -and ((Get-Content $tracePath -Raw) -match 'win-e:hook-active') } 4000 'Owned shell did not become active' | Out-Null

    $fixture = Start-Process -FilePath $fixturePath -ArgumentList '--no-progress', '--hold-ms', '26000' -RedirectStandardOutput $fixtureStdout -RedirectStandardError $fixtureStderr -PassThru
    Wait-Until { $fixture.Refresh(); $fixture.MainWindowHandle -ne [IntPtr]::Zero } 5000 'Fixture window did not appear' | Out-Null
    $fixtureHwnd = [IntPtr]$fixture.MainWindowHandle
    $normalRect = [SuperDesktopMinimizedShelfNative]::RectFor($fixtureHwnd)
    $normalPlacement = [SuperDesktopMinimizedShelfNative]::PlacementFor($fixtureHwnd)
    [SuperDesktopMinimizedShelfNative]::SetForegroundWindow($fixtureHwnd) | Out-Null
    [SuperDesktopMinimizedShelfNative]::SwitchToThisWindow($fixtureHwnd, $true)
    Wait-TaskButton $app.Id $title | Out-Null
    Invoke-TaskButton $app.Id $title
    Wait-Until { [SuperDesktopMinimizedShelfNative]::GetForegroundWindow() -eq $fixtureHwnd } 5000 'Taskbar did not activate fixture before minimize' | Out-Null
    Start-Sleep -Milliseconds 250

    Invoke-TaskButton $app.Id $title
    Wait-Until { [SuperDesktopMinimizedShelfNative]::IsIconic($fixtureHwnd) } 5000 'Taskbar minimize did not make fixture iconic' | Out-Null
    try {
        Wait-Until {
            -not [SuperDesktopMinimizedShelfNative]::IsWindowVisible($fixtureHwnd)
        } 5000 'Taskbar-minimized fixture remained visible on the desktop' | Out-Null
    }
    catch {
        $failedRect = [SuperDesktopMinimizedShelfNative]::RectFor($fixtureHwnd)
        $failedPlacement = [SuperDesktopMinimizedShelfNative]::PlacementFor($fixtureHwnd)
        throw "Taskbar shelf geometry failed: rect=$((Rect-Array $failedRect) -join ',') min=$($failedPlacement.MinPosition.X),$($failedPlacement.MinPosition.Y) show=$($failedPlacement.ShowCmd)"
    }
    $taskbarMinimizedRect = [SuperDesktopMinimizedShelfNative]::RectFor($fixtureHwnd)
    $taskbarMinimizedPlacement = [SuperDesktopMinimizedShelfNative]::PlacementFor($fixtureHwnd)
    Wait-TaskButton $app.Id $title | Out-Null
    Save-Desktop $screenshotPath

    Invoke-TaskButton $app.Id $title
    Wait-Until {
        -not [SuperDesktopMinimizedShelfNative]::IsIconic($fixtureHwnd) -and
        [SuperDesktopMinimizedShelfNative]::IsWindowVisible($fixtureHwnd)
    } 5000 'Taskbar activation did not restore and show fixture' | Out-Null
    $taskbarRestoredRect = [SuperDesktopMinimizedShelfNative]::RectFor($fixtureHwnd)
    $taskbarRestoredPlacement = [SuperDesktopMinimizedShelfNative]::PlacementFor($fixtureHwnd)
    Assert-RectNear $taskbarRestoredRect $normalRect 2 'Taskbar restore'
    Assert-RectNear $taskbarRestoredPlacement.NormalPosition $normalPlacement.NormalPosition 2 'Taskbar normal placement'
    Start-Sleep -Milliseconds 250

    if (-not [SuperDesktopMinimizedShelfNative]::PostMessageW($fixtureHwnd, $fixtureMinimizeMessage, [UIntPtr]::Zero, [IntPtr]::Zero)) { throw 'Fixture self-minimize message failed' }
    Wait-Until { [SuperDesktopMinimizedShelfNative]::IsIconic($fixtureHwnd) } 5000 'Fixture self-minimize did not become iconic' | Out-Null
    try {
        Wait-Until {
            -not [SuperDesktopMinimizedShelfNative]::IsWindowVisible($fixtureHwnd)
        } 5000 'Application-minimized fixture remained visible on the desktop' | Out-Null
    }
    catch {
        $failedRect = [SuperDesktopMinimizedShelfNative]::RectFor($fixtureHwnd)
        $failedPlacement = [SuperDesktopMinimizedShelfNative]::PlacementFor($fixtureHwnd)
        throw "Application shelf geometry failed: rect=$((Rect-Array $failedRect) -join ',') min=$($failedPlacement.MinPosition.X),$($failedPlacement.MinPosition.Y) show=$($failedPlacement.ShowCmd)"
    }
    $applicationMinimizedRect = [SuperDesktopMinimizedShelfNative]::RectFor($fixtureHwnd)
    $applicationMinimizedPlacement = [SuperDesktopMinimizedShelfNative]::PlacementFor($fixtureHwnd)
    Wait-TaskButton $app.Id $title | Out-Null

    if (-not [SuperDesktopMinimizedShelfNative]::PostMessageW($fixtureHwnd, $fixtureRestoreMessage, [UIntPtr]::Zero, [IntPtr]::Zero)) { throw 'Fixture self-restore message failed' }
    Wait-Until {
        -not [SuperDesktopMinimizedShelfNative]::IsIconic($fixtureHwnd) -and
        [SuperDesktopMinimizedShelfNative]::IsWindowVisible($fixtureHwnd)
    } 5000 'Fixture self-restore did not restore and show' | Out-Null
    $applicationRestoredRect = [SuperDesktopMinimizedShelfNative]::RectFor($fixtureHwnd)
    $applicationRestoredPlacement = [SuperDesktopMinimizedShelfNative]::PlacementFor($fixtureHwnd)
    Assert-RectNear $applicationRestoredRect $normalRect 2 'Application restore'
    Assert-RectNear $applicationRestoredPlacement.NormalPosition $normalPlacement.NormalPosition 2 'Application normal placement'

    $trace = Get-Content -Raw -LiteralPath $tracePath
    $fixtureTracePattern = "task:minimized-shelved:win:$($fixture.Id):"
    $shelfTraceCount = ([regex]::Matches($trace, [regex]::Escape($fixtureTracePattern))).Count
    if ($shelfTraceCount -ne 2) { throw "Expected two shelf episodes, observed $shelfTraceCount" }
    $app.Refresh(); $fixture.Refresh()
    if ($app.HasExited -or $fixture.HasExited) { throw 'SuperDesktop or fixture exited during shelf verification' }
    if (Get-Process explorer -ErrorAction SilentlyContinue) { throw 'Explorer reappeared during shelf verification' }
    $stderr = [string]$(if (Test-Path $stderrPath) { Get-Content -Raw $stderrPath -ErrorAction SilentlyContinue } else { '' })
    if ($stderr -match 'panicked|RefCell already borrowed|SuperDesktop error \[task:minimized-shelf\]') { throw "Runtime error signature observed: $stderr" }
    $observed = [ordered]@{
        fixture_pid = $fixture.Id
        fixture_hwnd = $fixtureHwnd.ToInt64()
        normal_rect = Rect-Array $normalRect
        normal_placement = Rect-Array $normalPlacement.NormalPosition
        taskbar_minimized_iconic = $true
        taskbar_minimized_hidden = $true
        taskbar_minimized_rect = Rect-Array $taskbarMinimizedRect
        taskbar_min_position = @($taskbarMinimizedPlacement.MinPosition.X, $taskbarMinimizedPlacement.MinPosition.Y)
        taskbar_entry_retained = $true
        taskbar_restore_rect = Rect-Array $taskbarRestoredRect
        taskbar_restore_exact = $true
        application_minimized_iconic = $true
        application_minimized_hidden = $true
        application_minimized_rect = Rect-Array $applicationMinimizedRect
        application_min_position = @($applicationMinimizedPlacement.MinPosition.X, $applicationMinimizedPlacement.MinPosition.Y)
        application_taskbar_entry_retained = $true
        application_restore_rect = Rect-Array $applicationRestoredRect
        application_restore_exact = $true
        shelf_trace_count = $shelfTraceCount
        superdesktop_survived = $true
        fixture_survived = $true
        explorer_absent_during_capture = $true
        runtime_error_signature_absent = $true
    }
}
catch { $failure = $_ }
finally {
    if ($fixture -and -not $fixture.HasExited) { Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue }
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
    if ($null -eq $priorSuperExplorer) { Remove-Item Env:SUPEREXPLORER_PATH -ErrorAction SilentlyContinue } else { $env:SUPEREXPLORER_PATH = $priorSuperExplorer }
}

$shellAfter = Get-ShellSnapshot
$shellRestored = $shellBefore.present -eq $shellAfter.present -and $shellBefore.value -eq $shellAfter.value
$explorerRestored = ([bool](Get-Process explorer -ErrorAction SilentlyContinue)) -eq $priorExplorer
if (-not $shellRestored -and $null -eq $failure) { $failure = [Exception]::new('Winlogon Shell changed during verification') }
if (-not $explorerRestored -and $null -eq $failure) { $failure = [Exception]::new('Explorer state was not restored') }
$report = [ordered]@{
    schema = 'owned-minimized-window-shelf-headful/v1'
    result = $(if ($null -eq $failure) { 'passed' } else { 'failed' })
    app_path = $appPath
    app_sha256 = Get-Sha256 $appPath
    fixture_path = $fixturePath
    fixture_sha256 = Get-Sha256 $fixturePath
    observed = $observed
    screenshot = $(if (Test-Path $screenshotPath) { 'minimized-desktop.png' } else { $null })
    screenshot_sha256 = $(if (Test-Path $screenshotPath) { Get-Sha256 $screenshotPath } else { $null })
    trace = $(if (Test-Path $tracePath) { 'minimized-window-shelf.log' } else { $null })
    trace_sha256 = $(if (Test-Path $tracePath) { Get-Sha256 $tracePath } else { $null })
    shell_before = $shellBefore
    shell_after = $shellAfter
    shell_restored = $shellRestored
    explorer_present_before = $priorExplorer
    explorer_restored = $explorerRestored
    failure = $(if ($null -eq $failure) { $null } else { [string]$failure })
}
[IO.File]::WriteAllText($reportPath, (($report | ConvertTo-Json -Depth 10) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
$report | ConvertTo-Json -Depth 10
if ($null -ne $failure) { throw $failure }
