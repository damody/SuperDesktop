param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][string]$ScreenshotPath,
    [Parameter(Mandatory = $true)][string]$JumpListScreenshotPath
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
    [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint x,uint y,uint d,UIntPtr e);
    public static void RightClick(int x,int y){SetCursorPos(x,y);mouse_event(8,0,0,0,UIntPtr.Zero);mouse_event(16,0,0,0,UIntPtr.Zero);}
}
'@
[LiveTaskbarDpi]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

function Find-Named($Root,[string]$Name){$condition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$Name);$Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)}

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
    $singleCharacterLabels = 0
    $fixedFound = $false
    $taskButton = $null
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
        if($null-eq$taskButton){$taskButton=$button}
        $taskBounds += [ordered]@{ left=[int]$bounds.Left;top=[int]$bounds.Top;width=[int]$bounds.Width;height=[int]$bounds.Height }
    }
    $rows = @($taskBounds.top | Sort-Object -Unique)
    if (-not $fixedFound -or $taskBounds.Count -lt 2 -or $singleCharacterLabels -ne 0 -or $rows.Count -ne 1) {
        throw "Production taskbar parity failed: fixed=$fixedFound tasks=$($taskBounds.Count) single=$singleCharacterLabels rows=$($rows.Count)"
    }
    $rootBounds=$root.Current.BoundingRectangle
    $rightControlLeft=[double]::PositiveInfinity
    for($index=0;$index-lt$buttons.Count;$index++){
        $button=$buttons.Item($index);$name=[string]$button.Current.Name;$controlBounds=$button.Current.BoundingRectangle
        if($controlBounds.Left-lt($rootBounds.Left+$rootBounds.Width/2)){continue}
        if($name-match '^(.+) \[(active|minimized|attention|available|unavailable|group:\d+)\]$'){continue}
        $rightControlLeft=[Math]::Min($rightControlLeft,$controlBounds.Left)
    }
    $maxTaskRight=@($taskBounds|ForEach-Object{$_.left+$_.width}|Measure-Object -Maximum).Maximum
    $taskbarDpi=[LiveTaskbarDpi]::GetDpiForWindow($process.MainWindowHandle);$taskbarScale=[double]$taskbarDpi/96.0
    $logicalTaskWidths=@($taskBounds|ForEach-Object{$_.width/$taskbarScale})
    if(@($logicalTaskWidths|Where-Object{$_ -lt 43 -or $_ -gt 161}).Count -ne 0 -or @($logicalTaskWidths|Where-Object{$_ -lt 159}).Count -eq 0){throw "Adaptive task widths rejected: $($logicalTaskWidths-join',')"}
    if([double]::IsPositiveInfinity($rightControlLeft)-or$maxTaskRight-gt$rightControlLeft){throw "One-row task overlap: maxTaskRight=$maxTaskRight reservedLeft=$rightControlLeft"}

    $sourceBounds=$taskButton.Current.BoundingRectangle
    [LiveTaskbarDpi]::RightClick([int]($sourceBounds.Left+$sourceBounds.Width/2),[int]($sourceBounds.Top+$sourceBounds.Height/2))
    $jumpWindow=$null
    $deadline=[DateTime]::UtcNow.AddSeconds(4)
    do{
        Start-Sleep -Milliseconds 100
        $windows=[System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children,[System.Windows.Automation.Condition]::TrueCondition)
        for($wi=0;$wi-lt$windows.Count;$wi++){
            $candidate=$windows.Item($wi)
            if($candidate.Current.ProcessId-ne$process.Id-or$candidate.Current.NativeWindowHandle-eq$process.MainWindowHandle){continue}
            if($null-ne(Find-Named $candidate 'Jump List')){$jumpWindow=$candidate;break}
        }
    }while($null-eq$jumpWindow-and[DateTime]::UtcNow-lt$deadline)
    if($null-eq$jumpWindow){throw 'Owned Jump List did not appear.'}
    $jumpBounds=$jumpWindow.Current.BoundingRectangle
    $jumpHwnd=[IntPtr][int]$jumpWindow.Current.NativeWindowHandle
    $dpi=[LiveTaskbarDpi]::GetDpiForWindow($jumpHwnd);$scale=[double]$dpi/96.0
    $widthDip=[double]$jumpBounds.Width/$scale;$heightDip=[double]$jumpBounds.Height/$scale
    $gapDip=([double]$root.Current.BoundingRectangle.Top-[double]$jumpBounds.Bottom)/$scale
    $anchorDelta=[Math]::Abs(($jumpBounds.Left+$jumpBounds.Width/2)-($sourceBounds.Left+$sourceBounds.Width/2))/$scale
    $menuItemCondition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::MenuItem)
    $menuItems=$jumpWindow.FindAll([System.Windows.Automation.TreeScope]::Descendants,$menuItemCondition)
    $headingNames=@();foreach($headingName in @('Recent','Frequent','Tasks','Actions')){if($null-ne(Find-Named $jumpWindow $headingName)){$headingNames+=$headingName}}
    if([Math]::Abs($widthDip-360)-gt16-or$heightDip-gt496-or$gapDip-lt2-or$gapDip-gt16-or$anchorDelta-gt24-or$menuItems.Count-lt2-or$headingNames-notcontains'Actions'){throw "Jump List rejected width=$widthDip height=$heightDip gap=$gapDip anchor=$anchorDelta items=$($menuItems.Count) headings=$($headingNames-join',')"}
    $jumpBitmap=[Drawing.Bitmap]::new([int]$jumpBounds.Width,[int]$jumpBounds.Height);$jumpGraphics=[Drawing.Graphics]::FromImage($jumpBitmap);$jumpGraphics.CopyFromScreen([int]$jumpBounds.Left,[int]$jumpBounds.Top,0,0,$jumpBitmap.Size);New-Item -ItemType Directory -Force (Split-Path -Parent $JumpListScreenshotPath)|Out-Null;$jumpBitmap.Save($JumpListScreenshotPath,[Drawing.Imaging.ImageFormat]::Png);$jumpGraphics.Dispose();$jumpBitmap.Dispose()

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
        maximum_task_right=$maxTaskRight
        reserved_right_controls_left=$rightControlLeft
        right_control_overlap=$false
        logical_task_widths=$logicalTaskWidths
        adaptive_shrink_observed=$true
        screenshot=(Split-Path -Leaf $ScreenshotPath)
        screenshot_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $ScreenshotPath).Hash
        raw_titles_persisted=$false
        frame_visible=$true
        jump_list=[ordered]@{width_dip=$widthDip;height_dip=$heightDip;taskbar_gap_dip=$gapDip;source_anchor_delta_dip=$anchorDelta;menu_item_count=$menuItems.Count;headings=$headingNames;screenshot=(Split-Path -Leaf $JumpListScreenshotPath);screenshot_sha256=(Get-FileHash -Algorithm SHA256 $JumpListScreenshotPath).Hash}
    }
    [IO.File]::WriteAllText($OutputPath,(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface }
    if ($null -eq $priorMatrix) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_STATE_MATRIX=$priorMatrix }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE=$priorTrace }
    if($null-eq$priorLocal){Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue}else{$env:LOCALAPPDATA=$priorLocal};Remove-Item $profileRoot -Recurse -Force -ErrorAction SilentlyContinue
}
