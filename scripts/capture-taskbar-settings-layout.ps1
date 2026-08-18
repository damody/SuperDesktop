param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [ValidateSet('light', 'dark')][string]$Theme = 'light'
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/examples/taskbar_settings_headful.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) {
    throw "Missing settings headful example: $appPath"
}
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class TaskbarSettingsLayoutPointer {
    [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    public static void WheelDown(int x, int y) {
        SetCursorPos(x, y);
        mouse_event(0x0800, 0, 0, unchecked((uint)-120), UIntPtr.Zero);
    }
    public static void UsePhysicalCoordinates() { SetThreadDpiAwarenessContext(new IntPtr(-4)); }
}
'@
[TaskbarSettingsLayoutPointer]::UsePhysicalCoordinates()

function Get-ProcessIds([string[]]$Names) {
    @(
        foreach ($name in $Names) {
            Get-Process -Name $name -ErrorAction SilentlyContinue | ForEach-Object { [int]$_.Id }
        }
    ) | Sort-Object -Unique
}

function Find-ProcessElements([int]$ProcessId) {
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
        $ProcessId
    )
    [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition
    )
}

function Wait-ForProcessElements([int]$ProcessId) {
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do {
        try {
            $elements = Find-ProcessElements -ProcessId $ProcessId
            if ($elements.Count -gt 3) { return @($elements) }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'The owned settings surface did not expose its UIA tree.'
}

function Get-ProcessBounds([object[]]$Elements) {
    foreach ($element in $Elements) {
        try {
            if ($element.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window) {
                $windowRect = $element.Current.BoundingRectangle
                if ($windowRect.Width -gt 1 -and $windowRect.Height -gt 1) {
                    return [Drawing.Rectangle]::FromLTRB(
                        [int]$windowRect.Left,
                        [int]$windowRect.Top,
                        [int][Math]::Ceiling($windowRect.Right),
                        [int][Math]::Ceiling($windowRect.Bottom)
                    )
                }
            }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
    }
    $rectangles = foreach ($element in $Elements) {
        try {
            $rect = $element.Current.BoundingRectangle
            if ($rect.Width -gt 1 -and $rect.Height -gt 1) { $rect }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
    }
    if (-not $rectangles) { throw 'The owned settings surface has no visible UIA bounds.' }
    $left = ($rectangles | Measure-Object Left -Minimum).Minimum
    $top = ($rectangles | Measure-Object Top -Minimum).Minimum
    $right = ($rectangles | ForEach-Object { $_.Right } | Measure-Object -Maximum).Maximum
    $bottom = ($rectangles | ForEach-Object { $_.Bottom } | Measure-Object -Maximum).Maximum
    [Drawing.Rectangle]::FromLTRB([int]$left, [int]$top, [int][Math]::Ceiling($right), [int][Math]::Ceiling($bottom))
}

function Capture-Region([Drawing.Rectangle]$Bounds, [string]$Path) {
    $bitmap = [Drawing.Bitmap]::new($Bounds.Width, $Bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($Bounds.Left, $Bounds.Top, 0, 0, $bitmap.Size)
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

function Find-AutoHide([object[]]$Elements) {
    $traditional = -join @([char]0x81EA, [char]0x52D5, [char]0x96B1, [char]0x85CF, [char]0x5DE5, [char]0x4F5C, [char]0x5217)
    foreach ($element in $Elements) {
        try {
            $name = [string]$element.Current.Name
            if (-not $name.StartsWith($traditional) -and $name -notmatch 'Automatically hide the taskbar') { continue }
            $toggle = $null
            if ($element.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$toggle)) {
                return [pscustomobject]@{ Element = $element; Pattern = [System.Windows.Automation.TogglePattern]$toggle; Name = $name }
            }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
    }
    $null
}

$delegatedNames = @('explorer', 'SystemSettings', 'ApplicationFrameHost')
$processesBefore = Get-ProcessIds -Names $delegatedNames
$oldTheme = $env:SUPERDESKTOP_THEME
$oldLocale = $env:SUPERDESKTOP_LOCALE
$env:SUPERDESKTOP_THEME = $Theme
$env:SUPERDESKTOP_LOCALE = 'zh-TW'
$process = Start-Process -FilePath $appPath -ArgumentList '--surface', 'settings', '--hold-ms', '30000' -PassThru
try {
    $elements = @(Wait-ForProcessElements -ProcessId ([int]$process.Id))
    $bounds = Get-ProcessBounds -Elements $elements
    $virtual = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $captureBounds = [Drawing.Rectangle]::Intersect($bounds, $virtual)
    if ($captureBounds.Width -lt 640 -or $captureBounds.Height -lt 480) {
        throw "Settings surface has insufficient visible bounds: window=$bounds capture=$captureBounds"
    }
    Capture-Region -Bounds $captureBounds -Path (Join-Path $EvidenceDirectory "$Theme-top.png")

    $centerX = [int]($captureBounds.Left + ($captureBounds.Width / 2))
    $centerY = [int]($captureBounds.Top + ($captureBounds.Height / 2))
    for ($attempt = 0; $attempt -lt 18; $attempt++) {
        [TaskbarSettingsLayoutPointer]::WheelDown($centerX, $centerY)
        Start-Sleep -Milliseconds 40
    }
    $elements = @(Wait-ForProcessElements -ProcessId ([int]$process.Id))
    $autoHide = Find-AutoHide -Elements $elements
    if ($null -eq $autoHide) { throw 'Auto-hide control was not UIA-reachable after scrolling.' }
    $autoHideRect = $autoHide.Element.Current.BoundingRectangle
    if ($autoHideRect.Width -le 1 -or $autoHideRect.Height -le 1) { throw 'Auto-hide UIA control is clipped after scrolling.' }
    Capture-Region -Bounds $captureBounds -Path (Join-Path $EvidenceDirectory "$Theme-bottom.png")

    $beforeState = $autoHide.Pattern.Current.ToggleState.ToString()
    $autoHide.Pattern.Toggle()
    Start-Sleep -Milliseconds 250
    $elements = @(Wait-ForProcessElements -ProcessId ([int]$process.Id))
    $autoHide = Find-AutoHide -Elements $elements
    $afterState = $autoHide.Pattern.Current.ToggleState.ToString()
    if ($afterState -eq $beforeState) { throw 'TogglePattern did not change the owned setting.' }
    $autoHide.Pattern.Toggle()

    $processesAfter = Get-ProcessIds -Names $delegatedNames
    $newDelegated = @($processesAfter | Where-Object { $_ -notin $processesBefore })
    if ($newDelegated.Count -ne 0) { throw "Owned settings launched delegated shell processes: $($newDelegated -join ', ')" }

    $windowElement = @($elements | Where-Object {
        try { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window } catch { $false }
    })[0]
    $windowHandle = [IntPtr]$windowElement.Current.NativeWindowHandle
    $windowDpi = [TaskbarSettingsLayoutPointer]::GetDpiForWindow($windowHandle)
    $monitorScale = [Math]::Round($windowDpi / 96.0, 3)
    $report = [ordered]@{
        schema = 'taskbar-settings-layout-headful/v1'
        result = 'passed'
        theme = $Theme
        locale = 'zh-TW'
        physical_bounds = @{ left = $bounds.Left; top = $bounds.Top; width = $bounds.Width; height = $bounds.Height }
        capture_bounds = @{ left = $captureBounds.Left; top = $captureBounds.Top; width = $captureBounds.Width; height = $captureBounds.Height }
        window_dpi = $windowDpi
        inferred_dpi_scale = $monitorScale
        expected_logical_width = 1100
        expected_logical_height = 860
        top_capture = "$Theme-top.png"
        bottom_capture = "$Theme-bottom.png"
        auto_hide_uia = @{ control_type = $autoHide.Element.Current.ControlType.ProgrammaticName; pattern = 'TogglePattern'; before = $beforeState; after = $afterState; visible_after_scroll = $true }
        keyboard_focus_contract = 'focus_visible styling and GPUI focusable rows are covered by taskbar-ui source-contract tests'
        save_failure_contract = 'TaskbarSettingsView retains authoritative state on rejected save; covered by taskbar-ui model tests'
        delegated_processes_started = @()
        explorer_ui_invoked = $false
    }
    [IO.File]::WriteAllText(
        (Join-Path $EvidenceDirectory "$Theme-report.json"),
        (($report | ConvertTo-Json -Depth 8) + [Environment]::NewLine),
        [Text.UTF8Encoding]::new($false)
    )
    $report | ConvertTo-Json -Depth 8
} finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    $env:SUPERDESKTOP_THEME = $oldTheme
    $env:SUPERDESKTOP_LOCALE = $oldLocale
}
