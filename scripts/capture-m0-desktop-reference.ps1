param(
    [Parameter(Mandatory = $true)]
    [string]$Workspace,
    [Parameter(Mandatory = $true)]
    [string]$OutputPath,
    [Parameter(Mandatory = $true)]
    [string]$ScreenshotPath
)

$ErrorActionPreference = 'Stop'
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class M0DesktopDpi {
    [DllImport("user32.dll")]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
}
'@
[M0DesktopDpi]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorSelected = $env:SUPERDESKTOP_VERIFICATION_DESKTOP_SELECTED
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$tracePath = [IO.Path]::ChangeExtension($OutputPath, '.log')
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'desktop'
$env:SUPERDESKTOP_VERIFICATION_DESKTOP_SELECTED = '1'
$env:SUPERDESKTOP_ACTION_TRACE = $tracePath
Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue

try {
    $process = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms', '4000' -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 50
        $process.Refresh()
    } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) {
        throw 'Desktop reference surface did not appear.'
    }
    Start-Sleep -Milliseconds 1200
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::NameProperty,
        'SuperExplorer [selected]'
    )
    $control = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
    if ($null -eq $control) {
        throw 'Selected desktop icon is missing from the UI Automation tree.'
    }
    $patterns = @($control.GetSupportedPatterns() | ForEach-Object ProgrammaticName)
    if ('InvokePatternIdentifiers.Pattern' -notin $patterns) {
        throw 'Selected desktop icon is missing InvokePattern.'
    }
    $selectedName = $control.Current.Name
    $iconBounds = $control.Current.BoundingRectangle
    if ($iconBounds.Width -lt 80 -or $iconBounds.Width -gt 300 -or $iconBounds.Height -lt 80 -or $iconBounds.Height -gt 300) {
        throw "Desktop icon bounds are not grid-sized: $iconBounds"
    }

    $bounds = $root.Current.BoundingRectangle
    $bitmap = [Drawing.Bitmap]::new([int]$bounds.Width, [int]$bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$bounds.Left, [int]$bounds.Top, 0, 0, $bitmap.Size)
    New-Item -ItemType Directory -Force (Split-Path -Parent $ScreenshotPath) | Out-Null
    $bitmap.Save($ScreenshotPath, [Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    $process.WaitForExit()

    $trace = Get-Content -Raw -Encoding UTF8 $tracePath
    if ($trace -notmatch 'frame-visible' -or $trace -notmatch 'wallpaper:loaded') {
        throw 'Desktop reference trace is missing visible-frame or wallpaper evidence.'
    }
    $report = [ordered]@{
        schema = 'm0-desktop-reference/v1'
        result = 'passed'
        app_sha256 = (Get-FileHash $appPath -Algorithm SHA256).Hash
        screenshot = Split-Path -Leaf $ScreenshotPath
        screenshot_sha256 = (Get-FileHash $ScreenshotPath -Algorithm SHA256).Hash
        wallpaper = 'system-current'
        icon_grid = $true
        selected_control = $selectedName
        selected_bounds = [ordered]@{
            left = [int]$iconBounds.Left
            top = [int]$iconBounds.Top
            width = [int]$iconBounds.Width
            height = [int]$iconBounds.Height
        }
        invoke_available = $true
        frame_visible = $true
    }
    $json = $report | ConvertTo-Json -Depth 8
    New-Item -ItemType Directory -Force (Split-Path -Parent $OutputPath) | Out-Null
    [IO.File]::WriteAllText($OutputPath, $json + "`n", [Text.UTF8Encoding]::new($false))
    $json
} finally {
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface }
    if ($null -eq $priorSelected) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_DESKTOP_SELECTED -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_DESKTOP_SELECTED = $priorSelected }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE = $priorTrace }
}
