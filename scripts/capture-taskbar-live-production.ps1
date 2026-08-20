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
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint x,uint y,uint d,UIntPtr e);
    public static void RightClick(int x,int y){SetCursorPos(x,y);mouse_event(1,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(150);mouse_event(8,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(50);mouse_event(16,0,0,0,UIntPtr.Zero);}
}
'@
[LiveTaskbarDpi]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

function Find-Named($Root,[string]$Name){$condition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$Name);$Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)}
function Get-Sha256([string]$Path){$stream=[IO.File]::OpenRead($Path);try{$hash=[Security.Cryptography.SHA256]::Create();try{([BitConverter]::ToString($hash.ComputeHash($stream))).Replace('-','')}finally{$hash.Dispose()}}finally{$stream.Dispose()}}

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorMatrix = $env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorLocal = $env:LOCALAPPDATA
$profileRoot=Join-Path $env:TEMP "superdesktop-taskbar-live-$PID";$settingsRoot=Join-Path $profileRoot 'SuperDesktop';New-Item -ItemType Directory -Force $settingsRoot|Out-Null
[IO.File]::WriteAllText((Join-Path $settingsRoot 'settings.json'),'{"schema_version":1,"revision":0,"taskbar":{"rows":1,"locked":true,"combine_groups":true,"previews_enabled":true,"show_labels":true,"pins":[]}}',[Text.UTF8Encoding]::new($false))
$env:LOCALAPPDATA=$profileRoot
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
Remove-Item Env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX -ErrorAction SilentlyContinue
$env:SUPERDESKTOP_ACTION_TRACE = $tracePath
Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue

try {
    $process = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms','9000' -PassThru
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
    $taskMeasurements = @()
    $singleCharacterLabels = 0
    for ($index = 0; $index -lt $buttons.Count; $index++) {
        $button = $buttons.Item($index)
        $name = [string]$button.Current.Name
        if ($name -eq 'SuperExplorer') {
            throw 'SuperExplorer is still rendered as an unconditional fixed taskbar entry.'
        }
        if ($name -notmatch '^(.+) \[(active|minimized|attention|available|unavailable|group:\d+)(?:, .+)?\]$') { continue }
        $visibleLabel = $matches[1].Trim()
        $taskState = $matches[2]
        if ([Globalization.StringInfo]::ParseCombiningCharacters($visibleLabel).Count -le 1) {
            $singleCharacterLabels++
        }
        $bounds = $button.Current.BoundingRectangle
        $taskBounds += [ordered]@{ left=[int]$bounds.Left;top=[int]$bounds.Top;width=[int]$bounds.Width;height=[int]$bounds.Height }
        $taskMeasurements += [ordered]@{ order=$taskMeasurements.Count;name=$visibleLabel;state=$taskState;left=[int]$bounds.Left;top=[int]$bounds.Top;width=[int]$bounds.Width;height=[int]$bounds.Height }
    }
    $rows = @($taskBounds.top | Sort-Object -Unique)
    if ($taskBounds.Count -lt 2 -or $singleCharacterLabels -ne 0 -or $rows.Count -ne 1) {
        throw "Production taskbar parity failed: fixed=false tasks=$($taskBounds.Count) single=$singleCharacterLabels rows=$($rows.Count)"
    }
    $rootBounds=$root.Current.BoundingRectangle
    $rightControlLeft=[double]::PositiveInfinity
    for($index=0;$index-lt$buttons.Count;$index++){
        $button=$buttons.Item($index);$name=[string]$button.Current.Name;$controlBounds=$button.Current.BoundingRectangle
        if($controlBounds.Left-lt($rootBounds.Left+$rootBounds.Width/2)){continue}
        if($name-match '^(.+) \[(active|minimized|attention|available|unavailable|group:\d+)(?:, .+)?\]$'){continue}
        $rightControlLeft=[Math]::Min($rightControlLeft,$controlBounds.Left)
    }
    $maxTaskRight=@($taskBounds|ForEach-Object{$_.left+$_.width}|Measure-Object -Maximum).Maximum
    $taskbarDpi=[LiveTaskbarDpi]::GetDpiForWindow($process.MainWindowHandle);$taskbarScale=[double]$taskbarDpi/96.0
    $logicalTaskWidths=@($taskBounds|ForEach-Object{$_.width/$taskbarScale})
    for($measurementIndex=0;$measurementIndex-lt$taskMeasurements.Count;$measurementIndex++){$taskMeasurements[$measurementIndex]['logical_width_dip']=$logicalTaskWidths[$measurementIndex]}
    for($measurementIndex=1;$measurementIndex-lt$taskMeasurements.Count;$measurementIndex++){if($taskMeasurements[$measurementIndex].left-le$taskMeasurements[$measurementIndex-1].left){throw "Task order is not left-to-right at index $measurementIndex"}}
    if(@($logicalTaskWidths|Where-Object{$_ -lt 43 -or $_ -gt 161}).Count-ne0-or@($logicalTaskWidths|Where-Object{$_ -lt 159}).Count-eq0){throw "Adaptive task widths rejected: tasks=$($logicalTaskWidths-join',')"}
    if([double]::IsPositiveInfinity($rightControlLeft)-or$maxTaskRight-gt$rightControlLeft){throw "One-row task overlap: maxTaskRight=$maxTaskRight reservedLeft=$rightControlLeft"}

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
        app_sha256=(Get-Sha256 $appPath)
        task_count=$taskBounds.Count
        distinct_task_rows=$rows.Count
        single_character_labels=$singleCharacterLabels
        fixed_superexplorer=$false
        fixed_entry_absent=$true
        visible_tasks=$taskMeasurements
        maximum_task_right=$maxTaskRight
        reserved_right_controls_left=$rightControlLeft
        right_control_overlap=$false
        logical_task_widths=$logicalTaskWidths
        adaptive_shrink_observed=$true
        screenshot=(Split-Path -Leaf $ScreenshotPath)
        screenshot_sha256=(Get-Sha256 $ScreenshotPath)
        raw_titles_persisted=$false
        frame_visible=$true
    }
    [IO.File]::WriteAllText($OutputPath,(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface }
    if ($null -eq $priorMatrix) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX=$priorMatrix }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE=$priorTrace }
    if($null-eq$priorLocal){Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue}else{$env:LOCALAPPDATA=$priorLocal};Remove-Item $profileRoot -Recurse -Force -ErrorAction SilentlyContinue
}
