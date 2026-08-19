param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [ValidateSet('light', 'dark', 'high-contrast')][string]$Theme = 'light'
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/examples/taskbar_settings_headful.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing settings headful example: $appPath" }
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SettingsChromePointer {
    [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    public static void UsePhysicalCoordinates() { SetThreadDpiAwarenessContext(new IntPtr(-4)); }
    public static void Drag(int fromX, int fromY, int toX, int toY) {
        SetCursorPos(fromX, fromY);
        mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero);
        for (int step = 1; step <= 12; step++) {
            int x = fromX + ((toX - fromX) * step / 12);
            int y = fromY + ((toY - fromY) * step / 12);
            SetCursorPos(x, y);
            System.Threading.Thread.Sleep(18);
        }
        mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
    }
}
'@
[SettingsChromePointer]::UsePhysicalCoordinates()
$script:closeName = -join @([char]0x95DC, [char]0x9589, [char]0x5DE5, [char]0x4F5C, [char]0x5217, [char]0x8A2D, [char]0x5B9A)

function Find-Elements([int]$ProcessId) {
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
        $ProcessId
    )
    [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition
    )
}

function Wait-Chrome([int]$ProcessId) {
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do {
        $close = $null
        $scrollbar = $null
        $elements = Find-Elements -ProcessId $ProcessId
        for ($index = 0; $index -lt $elements.Count; $index++) {
            $element = $elements.Item($index)
            try {
                if ($element.Current.Name -eq 'Close Taskbar settings' -or $element.Current.Name -eq $script:closeName) { $close = $element }
                if ($element.Current.ControlType -eq [System.Windows.Automation.ControlType]::ScrollBar) { $scrollbar = $element }
            } catch [System.Windows.Automation.ElementNotAvailableException] {}
        }
        if ($null -ne $close -and $null -ne $scrollbar) {
            return [pscustomobject]@{ Close = $close; Scrollbar = $scrollbar; Elements = $elements }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'Close button and scrollbar were not both UIA-reachable.'
}

function Get-RangeValue([System.Windows.Automation.AutomationElement]$Element) {
    $pattern = $null
    if (-not $Element.TryGetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern, [ref]$pattern)) {
        throw 'Scrollbar does not expose RangeValuePattern.'
    }
    ([System.Windows.Automation.RangeValuePattern]$pattern).Current.Value
}

function Get-WindowBounds([object[]]$Elements) {
    foreach ($element in $Elements) {
        try {
            if ($element.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window) {
                $rect = $element.Current.BoundingRectangle
                if ($rect.Width -gt 1 -and $rect.Height -gt 1) { return $rect }
            }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
    }
    throw 'Settings window bounds are unavailable.'
}

function Capture-Window([object]$Rect, [string]$Path) {
    $virtual = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bounds = [Drawing.Rectangle]::Intersect(
        [Drawing.Rectangle]::FromLTRB([int]$Rect.Left, [int]$Rect.Top, [int][Math]::Ceiling($Rect.Right), [int][Math]::Ceiling($Rect.Bottom)),
        $virtual
    )
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

$oldTheme = $env:SUPERDESKTOP_THEME
$oldLocale = $env:SUPERDESKTOP_LOCALE
$env:SUPERDESKTOP_THEME = $Theme
$env:SUPERDESKTOP_LOCALE = 'zh-TW'
$process = Start-Process -FilePath $appPath -ArgumentList '--surface', 'settings', '--hold-ms', '30000' -PassThru
try {
    $chrome = Wait-Chrome -ProcessId ([int]$process.Id)
    $windowBounds = Get-WindowBounds -Elements @($chrome.Elements)
    $closeBoundsBefore = $chrome.Close.Current.BoundingRectangle
    $scrollBounds = $chrome.Scrollbar.Current.BoundingRectangle
    $rangeBefore = Get-RangeValue -Element $chrome.Scrollbar
    Capture-Window -Rect $windowBounds -Path (Join-Path $EvidenceDirectory "$Theme-top.png")

    $dragX = [int]($scrollBounds.Left + $scrollBounds.Width / 2)
    [SettingsChromePointer]::Drag($dragX, [int]($scrollBounds.Top + 12), $dragX, [int]($scrollBounds.Bottom - 12))
    Start-Sleep -Milliseconds 350
    $chromeAfterDrag = Wait-Chrome -ProcessId ([int]$process.Id)
    $rangeAfter = Get-RangeValue -Element $chromeAfterDrag.Scrollbar
    $closeBoundsAfter = $chromeAfterDrag.Close.Current.BoundingRectangle
    if ($rangeAfter -le $rangeBefore + 25) { throw "Scrollbar drag did not materially change range value: before=$rangeBefore after=$rangeAfter" }
    if ($closeBoundsAfter.ToString() -ne $closeBoundsBefore.ToString()) { throw "Close button moved while scrolling: before=$closeBoundsBefore after=$closeBoundsAfter" }
    Capture-Window -Rect $windowBounds -Path (Join-Path $EvidenceDirectory "$Theme-bottom.png")

    $invoke = $null
    if (-not $chromeAfterDrag.Close.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$invoke)) {
        throw 'Close button does not expose InvokePattern.'
    }
    ([System.Windows.Automation.InvokePattern]$invoke).Invoke()
    Start-Sleep -Milliseconds 300
    $remaining = Find-Elements -ProcessId ([int]$process.Id)
    $closeStillPresent = $false
    for ($index = 0; $index -lt $remaining.Count; $index++) {
        try {
            if ($remaining.Item($index).Current.Name -eq 'Close Taskbar settings' -or $remaining.Item($index).Current.Name -eq $script:closeName) { $closeStillPresent = $true }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
    }
    if ($closeStillPresent) { throw 'UIA close invocation did not dismiss the settings window.' }

    $report = [ordered]@{
        schema = 'taskbar-settings-chrome-headful/v1'
        result = 'passed'
        theme = $Theme
        locale = 'zh-TW'
        close = @{ control_type = 'Button'; invoke_pattern = $true; fixed_bounds = $closeBoundsBefore.ToString(); dismissed = $true }
        scrollbar = @{ control_type = 'ScrollBar'; range_pattern = $true; before = $rangeBefore; after_drag = $rangeAfter; bounds = $scrollBounds.ToString() }
        captures = @("$Theme-top.png", "$Theme-bottom.png")
    }
    [IO.File]::WriteAllText((Join-Path $EvidenceDirectory "$Theme-report.json"), (($report | ConvertTo-Json -Depth 6) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 6
} finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    $env:SUPERDESKTOP_THEME = $oldTheme
    $env:SUPERDESKTOP_LOCALE = $oldLocale
}
