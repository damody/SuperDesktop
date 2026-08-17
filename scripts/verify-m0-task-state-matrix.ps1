param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [Parameter(Mandatory = $true)]
    [string]$OutputPath,
    [Parameter(Mandatory = $true)]
    [string]$ScreenshotPath
)

$ErrorActionPreference = 'Stop'
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath)) {
    throw "Missing release app: $appPath"
}

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class M0StateDpi {
    [DllImport("user32.dll")]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
}
'@
[M0StateDpi]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorMatrix = $env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$tracePath = [IO.Path]::ChangeExtension($OutputPath, '.log')
$traceParent = Split-Path -Parent $tracePath
New-Item -ItemType Directory -Force $traceParent | Out-Null
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
$env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX = '1'
$env:SUPERDESKTOP_ACTION_TRACE = $tracePath
Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue

function Wait-ForWindowHandle([System.Diagnostics.Process]$Process) {
    $deadline = [DateTime]::UtcNow.AddSeconds(3)
    do {
        Start-Sleep -Milliseconds 50
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero) {
            return $Process.MainWindowHandle
        }
    } while ([DateTime]::UtcNow -lt $deadline -and -not $Process.HasExited)
    throw 'The task-state verification window did not appear.'
}

try {
    $process = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms', '4000' -PassThru
    $windowHandle = Wait-ForWindowHandle $process
    Start-Sleep -Milliseconds 1200
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($windowHandle)
    $buttonCondition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button
    )
    $buttons = $root.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        $buttonCondition
    )
    $controls = @()
    for ($index = 0; $index -lt $buttons.Count; $index++) {
        $button = $buttons.Item($index)
        if ($button.Current.Name -notlike 'State *') {
            continue
        }
        $patterns = @($button.GetSupportedPatterns() | ForEach-Object ProgrammaticName)
        $bounds = $button.Current.BoundingRectangle
        $controls += [ordered]@{
            name = $button.Current.Name
            invoke_available = 'InvokePatternIdentifiers.Pattern' -in $patterns
            bounds = [ordered]@{
                left = [int]$bounds.Left
                top = [int]$bounds.Top
                width = [int]$bounds.Width
                height = [int]$bounds.Height
            }
        }
    }

    $expected = @(
        'State active [active]',
        'State minimized [minimized]',
        'State attention [attention]',
        'State group [group:3]',
        'State unavailable [unavailable]'
    )
    foreach ($name in $expected) {
        if ($name -notin @($controls.name)) {
            throw "Missing UI Automation task state: $name"
        }
    }
    $availableControls = @($controls | Where-Object name -ne 'State unavailable [unavailable]')
    if ($availableControls.Count -ne 4 -or @($availableControls | Where-Object { -not $_.invoke_available }).Count -ne 0) {
        throw 'An available task state is missing InvokePattern.'
    }
    $unavailable = @($controls | Where-Object name -eq 'State unavailable [unavailable]')
    if ($unavailable.Count -ne 1 -or $unavailable[0].invoke_available) {
        throw 'Unavailable state incorrectly exposes InvokePattern.'
    }
    $active = @($controls | Where-Object name -eq 'State active [active]')[0]
    $minimized = @($controls | Where-Object name -eq 'State minimized [minimized]')[0]
    $attention = @($controls | Where-Object name -eq 'State attention [attention]')[0]
    $group = @($controls | Where-Object name -eq 'State group [group:3]')[0]
    $taskRows = @($controls.bounds.top | Sort-Object -Unique)
    if ($taskRows.Count -ne 2 -or
        $active.bounds.top -le $minimized.bounds.top -or
        $active.bounds.left -ge $minimized.bounds.left -or
        $minimized.bounds.left -ne $attention.bounds.left -or
        $minimized.bounds.top -ge $attention.bounds.top -or
        $group.bounds.left -ne $unavailable[0].bounds.left -or
        $group.bounds.top -ge $unavailable[0].bounds.top) {
        throw 'Two-row tasks are not packed top-to-bottom before advancing columns.'
    }

    $windowBounds = $root.Current.BoundingRectangle
    $width = [math]::Max(1, [int]$windowBounds.Width)
    $height = [math]::Max(1, [int]$windowBounds.Height)
    $bitmap = [Drawing.Bitmap]::new($width, $height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen(
        [int]$windowBounds.Left,
        [int]$windowBounds.Top,
        0,
        0,
        $bitmap.Size
    )
    $screenshotParent = Split-Path -Parent $ScreenshotPath
    New-Item -ItemType Directory -Force $screenshotParent | Out-Null
    $bitmap.Save($ScreenshotPath, [Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    $process.WaitForExit()

    $trace = Get-Content -Raw -Encoding UTF8 $tracePath
    if ($trace -notmatch 'frame-visible') {
        throw 'The task-state surface did not produce a visible frame trace.'
    }
    $report = [ordered]@{
        schema = 'm0-task-state-matrix/v1'
        result = 'passed'
        app_sha256 = (Get-FileHash $appPath -Algorithm SHA256).Hash
        screenshot = Split-Path -Leaf $ScreenshotPath
        screenshot_sha256 = (Get-FileHash $ScreenshotPath -Algorithm SHA256).Hash
        state_count = $controls.Count
        controls = $controls
        unavailable_invoke_suppressed = $true
        distinct_task_rows = $taskRows.Count
        column_major_packing = $true
        frame_visible = $true
    }
    $parent = Split-Path -Parent $OutputPath
    New-Item -ItemType Directory -Force $parent | Out-Null
    $json = $report | ConvertTo-Json -Depth 10
    [IO.File]::WriteAllText($OutputPath, $json + "`n", [Text.UTF8Encoding]::new($false))
    $json
} finally {
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface }
    if ($null -eq $priorMatrix) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX = $priorMatrix }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE = $priorTrace }
}
