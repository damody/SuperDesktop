param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][string]$ScreenshotPath
)

$ErrorActionPreference = 'Stop'
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
$parent = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Force $parent | Out-Null
$tracePath = [IO.Path]::ChangeExtension($OutputPath, '.log')

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class MarqueeInput {
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    public const uint LEFTDOWN = 0x0002;
    public const uint LEFTUP = 0x0004;
    public const uint MOVE = 0x0001;
}
'@
[MarqueeInput]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'desktop'
$env:SUPERDESKTOP_ACTION_TRACE = $tracePath
Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
$held = $false

try {
    $process = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms','6000' -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(4)
    do { Start-Sleep -Milliseconds 50; $process.Refresh() }
    while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Desktop window did not appear.' }
    Start-Sleep -Milliseconds 1000
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $bounds = $root.Current.BoundingRectangle
    $startX = [int]($bounds.Left + [Math]::Min($bounds.Width - 40, 760))
    $startY = [int]($bounds.Top + [Math]::Min($bounds.Height - 40, 650))
    $endX = [int]($bounds.Left + 24)
    $endY = [int]($bounds.Top + 24)
    [MarqueeInput]::SetCursorPos($startX,$startY) | Out-Null
    [MarqueeInput]::mouse_event([MarqueeInput]::LEFTDOWN,0,0,0,[UIntPtr]::Zero)
    $held = $true
    for ($step=1; $step -le 12; $step++) {
        $x = [int]($startX + ($endX-$startX)*$step/12)
        $y = [int]($startY + ($endY-$startY)*$step/12)
        [MarqueeInput]::SetCursorPos($x,$y) | Out-Null
        Start-Sleep -Milliseconds 35
    }
    Start-Sleep -Milliseconds 350

    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button
    )
    $buttons = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)
    $selected = @()
    for ($index=0; $index -lt $buttons.Count; $index++) {
        $name = [string]$buttons.Item($index).Current.Name
        if ($name -like '*[selected]') { $selected += $name }
    }
    if ($selected.Count -lt 2) { throw "Marquee selected only $($selected.Count) desktop items." }

    $bitmap = [Drawing.Bitmap]::new([int]$bounds.Width,[int]$bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$bounds.Left,[int]$bounds.Top,0,0,$bitmap.Size)
    New-Item -ItemType Directory -Force (Split-Path -Parent $ScreenshotPath) | Out-Null
    $bitmap.Save($ScreenshotPath,[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose(); $bitmap.Dispose()
    [MarqueeInput]::mouse_event([MarqueeInput]::LEFTUP,0,0,0,[UIntPtr]::Zero)
    $held = $false
    $process.WaitForExit()
    $trace = Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath
    if ($trace -notmatch 'frame-visible') { throw 'Desktop trace lacks a visible frame.' }
    $report = [ordered]@{
        schema='desktop-marquee-production/v1';result='passed';app_sha256=(Get-FileHash -Algorithm SHA256 $appPath).Hash
        reverse_drag=$true;selected_count=$selected.Count;raw_names_persisted=$false
        screenshot=(Split-Path -Leaf $ScreenshotPath);screenshot_sha256=(Get-FileHash -Algorithm SHA256 $ScreenshotPath).Hash
        frame_visible=$true
    }
    [IO.File]::WriteAllText($OutputPath,(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($held) { [MarqueeInput]::mouse_event([MarqueeInput]::LEFTUP,0,0,0,[UIntPtr]::Zero) }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE=$priorTrace }
}
