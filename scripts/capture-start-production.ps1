param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][string]$HomeScreenshotPath,
    [Parameter(Mandatory = $true)][string]$AllAppsScreenshotPath
)

$ErrorActionPreference = 'Stop'
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
New-Item -ItemType Directory -Force (Split-Path -Parent $OutputPath) | Out-Null
$tracePath = [IO.Path]::ChangeExtension($OutputPath,'.log')
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class StartWindows {
    public delegate bool EnumProc(IntPtr hwnd, IntPtr value);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr value);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    public static IntPtr[] VisibleForProcess(uint wanted) {
        var result = new List<IntPtr>();
        EnumWindows((hwnd, value) => { uint pid; GetWindowThreadProcessId(hwnd,out pid); if (pid==wanted && IsWindowVisible(hwnd)) result.Add(hwnd); return true; },IntPtr.Zero);
        return result.ToArray();
    }
}
'@
[StartWindows]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

function Find-Named($Root,[string]$Name) {
    $condition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,$Name)
    $Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
}
function Save-Window($Root,[string]$Path) {
    $bounds=$Root.Current.BoundingRectangle
    $bitmap=[Drawing.Bitmap]::new([int]$bounds.Width,[int]$bounds.Height)
    $graphics=[Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$bounds.Left,[int]$bounds.Top,0,0,$bitmap.Size)
    New-Item -ItemType Directory -Force (Split-Path -Parent $Path) | Out-Null
    $bitmap.Save($Path,[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose();$bitmap.Dispose()
    $bounds
}

$priorSurface=$env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorOwned=$env:SUPERDESKTOP_VERIFICATION_OWNED_START
$priorTrace=$env:SUPERDESKTOP_ACTION_TRACE
$env:SUPERDESKTOP_VERIFICATION_SURFACE='taskbar'
$env:SUPERDESKTOP_VERIFICATION_OWNED_START='1'
$env:SUPERDESKTOP_ACTION_TRACE=$tracePath
Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
try {
    $process=Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms','8000' -PassThru
    $deadline=[DateTime]::UtcNow.AddSeconds(4)
    do { Start-Sleep -Milliseconds 50; $process.Refresh() } while($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if($process.MainWindowHandle -eq [IntPtr]::Zero){throw 'Taskbar window did not appear.'}
    Start-Sleep -Milliseconds 900
    $taskbar=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $start=Find-Named $taskbar 'Start'
    if($null -eq $start){throw 'Start button is missing.'}
    $invoke=$start.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $invoke.Invoke()
    $startHandle=[IntPtr]::Zero
    $deadline=[DateTime]::UtcNow.AddSeconds(4)
    do {
        Start-Sleep -Milliseconds 75
        foreach($handle in [StartWindows]::VisibleForProcess([uint32]$process.Id)){
            if($handle -ne $process.MainWindowHandle){$startHandle=$handle;break}
        }
    } while($startHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if($startHandle -eq [IntPtr]::Zero){throw 'Owned Start window did not appear.'}
    $root=[System.Windows.Automation.AutomationElement]::FromHandle($startHandle)
    Start-Sleep -Milliseconds 700
    foreach($name in @('Pinned','Recommended','All apps','Settings','Power')){if($null -eq (Find-Named $root $name)){throw "Start home is missing $name."}}
    if($null -ne (Find-Named $root 'Sign out')){throw 'Power action is exposed while Power is collapsed.'}
    $homeBounds=Save-Window $root $HomeScreenshotPath
    $all=Find-Named $root 'All apps'
    $all.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    Start-Sleep -Milliseconds 500
    $root=[System.Windows.Automation.AutomationElement]::FromHandle($startHandle)
    if($null -eq (Find-Named $root 'Back to pinned')){throw 'All apps page is missing Back to pinned.'}
    $listCondition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::ListItem)
    $allItems=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$listCondition)
    if($allItems.Count -lt 6){throw "All apps exposed only $($allItems.Count) list items."}
    $allBounds=Save-Window $root $AllAppsScreenshotPath
    $process.WaitForExit()
    $trace=Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath
    if($trace -notmatch 'start:owned-opened'){throw 'Trace lacks owned Start opening.'}
    $report=[ordered]@{
        schema='windows11-start-production/v1';result='passed';app_sha256=(Get-FileHash -Algorithm SHA256 $appPath).Hash
        home_sections=@('Search','Pinned','Recommended','Account','Settings','Power');power_collapsed=$true;all_apps_count=$allItems.Count
        centered_home_bounds=[ordered]@{left=[int]$homeBounds.Left;top=[int]$homeBounds.Top;width=[int]$homeBounds.Width;height=[int]$homeBounds.Height}
        home_screenshot=(Split-Path -Leaf $HomeScreenshotPath);home_sha256=(Get-FileHash -Algorithm SHA256 $HomeScreenshotPath).Hash
        all_apps_screenshot=(Split-Path -Leaf $AllAppsScreenshotPath);all_apps_sha256=(Get-FileHash -Algorithm SHA256 $AllAppsScreenshotPath).Hash
    }
    [IO.File]::WriteAllText($OutputPath,(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report|ConvertTo-Json -Depth 8
} finally {
    if($null -eq $priorSurface){Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface}
    if($null -eq $priorOwned){Remove-Item Env:SUPERDESKTOP_VERIFICATION_OWNED_START -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_VERIFICATION_OWNED_START=$priorOwned}
    if($null -eq $priorTrace){Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_ACTION_TRACE=$priorTrace}
}
