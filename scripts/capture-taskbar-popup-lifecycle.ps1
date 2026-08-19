param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/examples/taskbar_settings_headful.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) {
    throw "Missing popup headful example: $appPath"
}
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path
$contextDirectory = Join-Path $EvidenceDirectory 'context-deactivation'
$previewDirectory = Join-Path $EvidenceDirectory 'preview-topmost'
New-Item -ItemType Directory -Force -Path $contextDirectory, $previewDirectory | Out-Null

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class PopupLifecycleNative {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern IntPtr CreateWindowExW(uint exStyle, string className, string title, uint style, int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool DestroyWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr window);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern void SwitchToThisWindow(IntPtr window, bool altTab);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
    public static IntPtr CreateOrdinaryWindow(int x, int y, int width, int height) {
        const uint WS_OVERLAPPEDWINDOW = 0x00CF0000;
        const uint WS_VISIBLE = 0x10000000;
        return CreateWindowExW(0, "STATIC", "Ordinary overlap window", WS_OVERLAPPEDWINDOW | WS_VISIBLE, x, y, width, height, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
    }
    public static bool ActivateOrdinaryWindow(IntPtr window) {
        ShowWindow(window, 5);
        BringWindowToTop(window);
        SetForegroundWindow(window);
        SwitchToThisWindow(window, true);
        return GetForegroundWindow() == window;
    }
    public static bool MoveOrdinaryWindow(IntPtr window, int x, int y, int width, int height) {
        return SetWindowPos(window, IntPtr.Zero, x, y, width, height, 0x0040);
    }
}
'@

function Find-ProcessWindow([int]$ProcessId) {
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
        $ProcessId
    )
    $elements = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition
    )
    foreach ($element in $elements) {
        try {
            $native = [IntPtr]$element.Current.NativeWindowHandle
            $rect = $element.Current.BoundingRectangle
            if ($native -ne [IntPtr]::Zero -and $rect.Width -gt 1 -and $rect.Height -gt 1) {
                return [pscustomobject]@{ Element = $element; Handle = $native; Rect = $rect }
            }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
    }
    $null
}

function Start-OverlapProcess {
    $process = Start-Process -FilePath $appPath -ArgumentList '--surface', 'settings', '--hold-ms', '30000' -PassThru
    try {
        $window = Wait-ProcessWindow -ProcessId ([int]$process.Id) -Present $true
        return [pscustomobject]@{ Process = $process; Handle = $window.Handle }
    } catch {
        if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
        throw
    }
}

function Wait-ProcessWindow([int]$ProcessId, [bool]$Present) {
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do {
        $window = Find-ProcessWindow -ProcessId $ProcessId
        if (($Present -and $null -ne $window) -or (-not $Present -and $null -eq $window)) { return $window }
        Start-Sleep -Milliseconds 80
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Process window presence did not become $Present for PID $ProcessId"
}

function Capture-Rect([object]$Rect, [string]$Path) {
    $bounds = [Drawing.Rectangle]::FromLTRB([int]$Rect.Left, [int]$Rect.Top, [int][Math]::Ceiling($Rect.Right), [int][Math]::Ceiling($Rect.Bottom))
    $bitmap = [Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bitmap.Size)
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

$contextProcess = Start-Process -FilePath $appPath -ArgumentList '--surface', 'context', '--hold-ms', '30000' -PassThru
$contextOverlapProcess = $null
try {
    $contextWindow = Wait-ProcessWindow -ProcessId ([int]$contextProcess.Id) -Present $true
    Capture-Rect -Rect $contextWindow.Rect -Path (Join-Path $contextDirectory 'before-deactivation.png')
    $contextOverlapProcess = Start-OverlapProcess
    $ordinaryContext = [IntPtr]$contextOverlapProcess.Handle
    if ([PopupLifecycleNative]::GetForegroundWindow() -ne $ordinaryContext -and -not [PopupLifecycleNative]::ActivateOrdinaryWindow($ordinaryContext)) {
        throw 'Context overlap window did not become foreground.'
    }
    Start-Sleep -Milliseconds 250
    [void](Wait-ProcessWindow -ProcessId ([int]$contextProcess.Id) -Present $false)
    Capture-Rect -Rect $contextWindow.Rect -Path (Join-Path $contextDirectory 'after-deactivation.png')
    $contextReport = [ordered]@{
        schema = 'taskbar-context-deactivation-headful/v1'
        result = 'passed'
        popup_hwnd = $contextWindow.Handle.ToInt64()
        ordinary_hwnd = $ordinaryContext.ToInt64()
        foreground_after = [PopupLifecycleNative]::GetForegroundWindow().ToInt64()
        foreground_switched_to_ordinary = ([PopupLifecycleNative]::GetForegroundWindow() -eq $ordinaryContext)
        popup_removed_after_deactivation = $true
        screenshots = @('before-deactivation.png', 'after-deactivation.png')
    }
    [IO.File]::WriteAllText((Join-Path $contextDirectory 'report.json'), (($contextReport | ConvertTo-Json -Depth 5) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllLines((Join-Path $contextDirectory 'trace.log'), @(
        "context-popup-observed hwnd=$($contextWindow.Handle.ToInt64())",
        "foreground-switched hwnd=$($ordinaryContext.ToInt64())",
        'context-popup-removed result=passed'
    ), [Text.UTF8Encoding]::new($false))
} finally {
    if ($null -ne $contextOverlapProcess -and -not $contextOverlapProcess.Process.HasExited) { Stop-Process -Id $contextOverlapProcess.Process.Id -Force -ErrorAction SilentlyContinue }
    if (-not $contextProcess.HasExited) { Stop-Process -Id $contextProcess.Id -Force -ErrorAction SilentlyContinue }
}

$previewOverlapProcess = Start-OverlapProcess
$ordinaryPreview = [IntPtr]$previewOverlapProcess.Handle
[void][PopupLifecycleNative]::MoveOrdinaryWindow($ordinaryPreview, 0, 0, 480, 360)
$previewProcess = $null
try {
    if ([PopupLifecycleNative]::GetForegroundWindow() -ne $ordinaryPreview -and -not [PopupLifecycleNative]::ActivateOrdinaryWindow($ordinaryPreview)) {
        throw 'Preview overlap window did not become foreground.'
    }
    Start-Sleep -Milliseconds 100
    $foregroundBefore = [PopupLifecycleNative]::GetForegroundWindow()
    $previewProcess = Start-Process -FilePath $appPath -ArgumentList '--surface', 'preview', '--hold-ms', '30000' -PassThru
    $previewWindow = Wait-ProcessWindow -ProcessId ([int]$previewProcess.Id) -Present $true
    Start-Sleep -Milliseconds 200
    $foregroundAfter = [PopupLifecycleNative]::GetForegroundWindow()
    $extendedStyle = [PopupLifecycleNative]::GetWindowLongPtrW($previewWindow.Handle, -20).ToInt64()
    $isTopmost = (($extendedStyle -band 0x8) -ne 0)
    if (-not $isTopmost) { throw 'Preview HWND did not acquire WS_EX_TOPMOST.' }
    if ($foregroundAfter -ne $foregroundBefore) { throw "Passive preview changed foreground HWND from $foregroundBefore to $foregroundAfter" }
    Capture-Rect -Rect $previewWindow.Rect -Path (Join-Path $previewDirectory 'overlap-topmost.png')
    $previewReport = [ordered]@{
        schema = 'taskbar-preview-topmost-headful/v1'
        result = 'passed'
        popup_hwnd = $previewWindow.Handle.ToInt64()
        ordinary_hwnd = $ordinaryPreview.ToInt64()
        ws_ex_topmost = $isTopmost
        foreground_before = $foregroundBefore.ToInt64()
        foreground_after = $foregroundAfter.ToInt64()
        foreground_unchanged = ($foregroundBefore -eq $foregroundAfter)
        screenshot = 'overlap-topmost.png'
    }
    [IO.File]::WriteAllText((Join-Path $previewDirectory 'report.json'), (($previewReport | ConvertTo-Json -Depth 5) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllLines((Join-Path $previewDirectory 'trace.log'), @(
        "preview-popup-observed hwnd=$($previewWindow.Handle.ToInt64())",
        "topmost-style ws_ex_topmost=$isTopmost",
        "foreground-preserved hwnd=$($foregroundAfter.ToInt64()) result=passed"
    ), [Text.UTF8Encoding]::new($false))
} finally {
    if ($null -ne $previewProcess -and -not $previewProcess.HasExited) { Stop-Process -Id $previewProcess.Id -Force -ErrorAction SilentlyContinue }
    if (-not $previewOverlapProcess.Process.HasExited) { Stop-Process -Id $previewOverlapProcess.Process.Id -Force -ErrorAction SilentlyContinue }
}

@{ context = $contextReport; preview = $previewReport } | ConvertTo-Json -Depth 8
