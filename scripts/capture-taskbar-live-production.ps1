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
public static class LiveTaskbarDpi {
    [DllImport("user32.dll")]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
}
'@
[LiveTaskbarDpi]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorMatrix = $env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
Remove-Item Env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX -ErrorAction SilentlyContinue
$env:SUPERDESKTOP_ACTION_TRACE = $tracePath
Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue

try {
    $process = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms','5000' -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(4)
    do {
        Start-Sleep -Milliseconds 50
        $process.Refresh()
    } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Production taskbar window did not appear.' }
    Start-Sleep -Milliseconds 1500

    $root = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button
    )
    $buttons = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    $taskBounds = @()
    $singleCharacterLabels = 0
    $fixedFound = $false
    for ($index = 0; $index -lt $buttons.Count; $index++) {
        $button = $buttons.Item($index)
        $name = [string]$button.Current.Name
        if ($name -eq 'SuperExplorer') { $fixedFound = $true; continue }
        if ($name -notmatch '^(.+) \[(active|minimized|attention|available|unavailable|group:\d+)\]$') { continue }
        $visibleLabel = $matches[1].Trim()
        if ([Globalization.StringInfo]::ParseCombiningCharacters($visibleLabel).Count -le 1) {
            $singleCharacterLabels++
        }
        $bounds = $button.Current.BoundingRectangle
        $taskBounds += [ordered]@{ left=[int]$bounds.Left;top=[int]$bounds.Top;width=[int]$bounds.Width;height=[int]$bounds.Height }
    }
    $rows = @($taskBounds.top | Sort-Object -Unique)
    if (-not $fixedFound -or $taskBounds.Count -lt 2 -or $singleCharacterLabels -ne 0 -or $rows.Count -lt 2) {
        throw "Production taskbar parity failed: fixed=$fixedFound tasks=$($taskBounds.Count) single=$singleCharacterLabels rows=$($rows.Count)"
    }

    $bounds = $root.Current.BoundingRectangle
    $bitmap = [Drawing.Bitmap]::new([int]$bounds.Width, [int]$bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$bounds.Left,[int]$bounds.Top,0,0,$bitmap.Size)
    New-Item -ItemType Directory -Force (Split-Path -Parent $ScreenshotPath) | Out-Null
    $bitmap.Save($ScreenshotPath,[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose();$bitmap.Dispose()
    $process.WaitForExit()
    $trace = Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath
    if ($trace -notmatch 'frame-visible') { throw 'Production taskbar trace lacks a visible frame.' }

    $report = [ordered]@{
        schema='taskbar-live-production/v1'
        result='passed'
        app_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $appPath).Hash
        task_count=$taskBounds.Count
        distinct_task_rows=$rows.Count
        single_character_labels=$singleCharacterLabels
        fixed_superexplorer=$fixedFound
        screenshot=(Split-Path -Leaf $ScreenshotPath)
        screenshot_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $ScreenshotPath).Hash
        raw_titles_persisted=$false
        frame_visible=$true
    }
    [IO.File]::WriteAllText($OutputPath,(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface }
    if ($null -eq $priorMatrix) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX=$priorMatrix }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE=$priorTrace }
}
