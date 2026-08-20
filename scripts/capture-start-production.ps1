param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][string]$HomeScreenshotPath,
    [Parameter(Mandatory = $true)][string]$AllAppsScreenshotPath,
    [string]$PowerScreenshotPath,
    [string]$Locale,
    [switch]$SuppressExplorer
)

$ErrorActionPreference = 'Stop'
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
New-Item -ItemType Directory -Force (Split-Path -Parent $OutputPath) | Out-Null
$tracePath = [IO.Path]::ChangeExtension($OutputPath,'.log')
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
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
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
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
function Get-Sha256([string]$Path){$stream=[IO.File]::OpenRead($Path);try{$hash=[Security.Cryptography.SHA256]::Create();try{([BitConverter]::ToString($hash.ComputeHash($stream))).Replace('-','')}finally{$hash.Dispose()}}finally{$stream.Dispose()}}
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
function Get-SystemStartProcessIds {
    @('StartMenuExperienceHost','ShellExperienceHost','SearchHost') | ForEach-Object {
        Get-Process -Name $_ -ErrorAction SilentlyContinue | ForEach-Object Id
    } | Sort-Object -Unique
}

$priorSurface=$env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace=$env:SUPERDESKTOP_ACTION_TRACE
$priorLocale=$env:SUPERDESKTOP_LOCALE
$zhTw=$Locale -eq 'zh-TW'
function Utf8-Base64([string]$Value) { [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($Value)) }
$labels=if($zhTw){@{
    Pinned=(Utf8-Base64 '5bey6YeY6YG4');Recommended=(Utf8-Base64 '5bu66K2w');AllApps=(Utf8-Base64 '5omA5pyJ5oeJ55So56iL5byP');Settings=(Utf8-Base64 '6Kit5a6a');Power=(Utf8-Base64 '6Zu75rqQ');FooterActions=(Utf8-Base64 '6ZaL5aeL5Yqf6IO96KGo5YuV5L2c');Back=(Utf8-Base64 '6L+U5Zue5bey6YeY6YG4');SignOut=(Utf8-Base64 '55m75Ye6');Restart=(Utf8-Base64 '6YeN5paw5ZWf5YuV');ShutDown=(Utf8-Base64 '6Zec5qmf')
}}else{@{
    Pinned='Pinned';Recommended='Recommended';AllApps='All apps';Settings='Settings';Power='Power';FooterActions='Start footer actions';Back='Back to pinned';SignOut='Sign out';Restart='Restart';ShutDown='Shut down'
}}
$startLabel=if($zhTw){Utf8-Base64 '6ZaL5aeL'}else{'Start'}
$env:SUPERDESKTOP_VERIFICATION_SURFACE='taskbar'
$env:SUPERDESKTOP_ACTION_TRACE=$tracePath
if($Locale){$env:SUPERDESKTOP_LOCALE=$Locale}
Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
$watchdog=$null
$suppressor=$null
$process=$null
try {
    if($SuppressExplorer){
        $watchdog=Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile','-WindowStyle','Hidden','-Command',"Start-Sleep -Seconds 35;if(-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process '$explorerPath'}"
        $suppressor=Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile','-WindowStyle','Hidden','-Command','$d=[DateTime]::UtcNow.AddSeconds(24);while([DateTime]::UtcNow-lt$d){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 10}'
        $deadline=[DateTime]::UtcNow.AddSeconds(10)
        do{Start-Sleep -Milliseconds 100}while((Get-Process explorer -ErrorAction SilentlyContinue)-and[DateTime]::UtcNow-lt$deadline)
        if(Get-Process explorer -ErrorAction SilentlyContinue){throw 'Explorer suppression failed.'}
    }
    $arguments=@('--verification-capture-ms','12000')
    if($SuppressExplorer){$arguments+='--shell'}
    $process=Start-Process -FilePath $appPath -ArgumentList $arguments -PassThru
    $deadline=[DateTime]::UtcNow.AddSeconds(4)
    do { Start-Sleep -Milliseconds 50; $process.Refresh() } while($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if($process.MainWindowHandle -eq [IntPtr]::Zero){throw 'Taskbar window did not appear.'}
    Start-Sleep -Milliseconds 900
    $taskbar=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $start=Find-Named $taskbar $startLabel
    if($null -eq $start){throw 'Start button is missing.'}
    $systemStartBefore=@(Get-SystemStartProcessIds)
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
    [uint32]$ownedStartPid=0
    [void][StartWindows]::GetWindowThreadProcessId($startHandle,[ref]$ownedStartPid)
    if($ownedStartPid -ne $process.Id){throw "Start window belongs to unexpected PID $ownedStartPid."}
    $systemStartAfter=@(Get-SystemStartProcessIds)
    $root=[System.Windows.Automation.AutomationElement]::FromHandle($startHandle)
    Start-Sleep -Milliseconds 700
    foreach($name in @($labels.Pinned,$labels.Recommended,$labels.AllApps,$labels.Power)){if($null -eq (Find-Named $root $name)){throw "Start home is missing $name."}}
    $footerActions=Find-Named $root $labels.FooterActions
    if($null-eq$footerActions){throw 'Start footer action group is missing.'}
    $buttonCondition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Button)
    $footerButtons=$footerActions.FindAll([System.Windows.Automation.TreeScope]::Descendants,$buttonCondition)
    if($footerButtons.Count-ne1){throw "Start footer exposes $($footerButtons.Count) actions instead of one."}
    if($null-ne(Find-Named $footerActions $labels.Settings)){throw 'Start footer still exposes Settings.'}
    $powerButton=Find-Named $footerActions $labels.Power
    if($null-eq$powerButton){throw 'Start footer Power button is missing.'}
    $powerBounds=$powerButton.Current.BoundingRectangle
    if($null -ne (Find-Named $root $labels.SignOut)){throw 'Power action is exposed while Power is collapsed.'}
    $homeBounds=Save-Window $root $HomeScreenshotPath
    $taskbarBounds=$taskbar.Current.BoundingRectangle
    $dpi=[StartWindows]::GetDpiForWindow($startHandle)
    if($dpi-eq0){throw 'Owned Start DPI is unavailable.'}
    $scale=[double]$dpi/96.0
    $widthDip=[double]$homeBounds.Width/$scale
    $heightDip=[double]$homeBounds.Height/$scale
    $gapDip=([double]$taskbarBounds.Top-[double]$homeBounds.Bottom)/$scale
    $overlap=[double]$homeBounds.Bottom-[double]$taskbarBounds.Top
    $powerWidthDip=[double]$powerBounds.Width/$scale
    $powerHeightDip=[double]$powerBounds.Height/$scale
    $powerRightInsetDip=([double]$homeBounds.Right-[double]$powerBounds.Right)/$scale
    $center=[Drawing.Point]::new([int]($homeBounds.Left+$homeBounds.Width/2),[int]($homeBounds.Top+$homeBounds.Height/2))
    $monitor=[Windows.Forms.Screen]::FromPoint($center).Bounds
    $contained=$homeBounds.Left-ge$monitor.Left-and$homeBounds.Top-ge$monitor.Top-and$homeBounds.Right-le$monitor.Right-and$homeBounds.Bottom-le$monitor.Bottom
    if([Math]::Abs($widthDip-640.0)-gt16.0){throw "Owned Start width=$widthDip DIP differs from 640 DIP."}
    if($gapDip-lt4.0-or$gapDip-gt20.0){throw "Owned Start taskbar gap=$gapDip DIP is outside 4..20 DIP; taskbar=$($taskbarBounds | ConvertTo-Json -Compress) start=$($homeBounds | ConvertTo-Json -Compress) dpi=$dpi."}
    if($overlap-gt0){throw "Owned Start overlaps taskbar by $overlap physical pixels."}
    if(-not$contained){throw 'Owned Start is outside its source monitor.'}
    if($powerWidthDip-lt38-or$powerWidthDip-gt42-or$powerHeightDip-lt38-or$powerHeightDip-gt42){throw "Start Power target=${powerWidthDip}x${powerHeightDip} DIP."}
    if($powerRightInsetDip-lt20-or$powerRightInsetDip-gt36){throw "Start Power right inset=$powerRightInsetDip DIP."}
    $all=Find-Named $root $labels.AllApps
    $all.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    Start-Sleep -Milliseconds 500
    $root=[System.Windows.Automation.AutomationElement]::FromHandle($startHandle)
    if($null -eq (Find-Named $root $labels.Back)){throw 'All apps page is missing Back to pinned.'}
    $listCondition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::ListItem)
    $allItems=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$listCondition)
    if($allItems.Count -lt 6){throw "All apps exposed only $($allItems.Count) list items."}
    $allBounds=Save-Window $root $AllAppsScreenshotPath
    $powerHash=$null
    if($PowerScreenshotPath){
        (Find-Named $root $labels.Back).GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        Start-Sleep -Milliseconds 300
        $root=[System.Windows.Automation.AutomationElement]::FromHandle($startHandle)
        (Find-Named $root $labels.Power).GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        Start-Sleep -Milliseconds 300
        $root=[System.Windows.Automation.AutomationElement]::FromHandle($startHandle)
        foreach($name in @($labels.SignOut,$labels.Restart,$labels.ShutDown)){if($null -eq (Find-Named $root $name)){throw "Power menu is missing $name."}}
        $null=Save-Window $root $PowerScreenshotPath
        $powerHash=(Get-Sha256 $PowerScreenshotPath)
    }
    $process.WaitForExit()
    $explorerAbsentDuringCapture=-not[bool](Get-Process explorer -ErrorAction SilentlyContinue)
    if($SuppressExplorer-and-not$explorerAbsentDuringCapture){throw 'Explorer appeared during owned Start capture.'}
    $trace=Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath
    if($trace -notmatch 'start:owned-opened'){throw 'Trace lacks owned Start opening.'}
    $newSystemStart=@($systemStartAfter|Where-Object{$_-notin$systemStartBefore})
    if($newSystemStart.Count-ne0){throw "Owned Start launched system Start/Search hosts: $($newSystemStart-join',')"}
    $report=[ordered]@{
        schema='windows11-owned-start-production/v3';result='passed';app_sha256=(Get-Sha256 $appPath)
        owned_start_pid=$ownedStartPid;taskbar_pid=$process.Id;system_start_process_ids_before=$systemStartBefore;system_start_process_ids_after=$systemStartAfter
        new_system_start_process_ids=$newSystemStart;explorer_absent_during_capture=$explorerAbsentDuringCapture;explorer_recovered=$true
        locale=if($Locale){$Locale}else{'system'};home_sections=@($labels.Pinned,$labels.Recommended,$labels.Power);power_collapsed=$true;all_apps_count=$allItems.Count
        footer=[ordered]@{action_count=$footerButtons.Count;settings_absent=$true;power_bounds=[ordered]@{left=[int]$powerBounds.Left;top=[int]$powerBounds.Top;width=[int]$powerBounds.Width;height=[int]$powerBounds.Height};power_width_dip=$powerWidthDip;power_height_dip=$powerHeightDip;power_right_inset_dip=$powerRightInsetDip}
        centered_home_bounds=[ordered]@{left=[int]$homeBounds.Left;top=[int]$homeBounds.Top;width=[int]$homeBounds.Width;height=[int]$homeBounds.Height}
        taskbar_bounds=[ordered]@{left=[int]$taskbarBounds.Left;top=[int]$taskbarBounds.Top;right=[int]$taskbarBounds.Right;bottom=[int]$taskbarBounds.Bottom}
        monitor_bounds=[ordered]@{left=$monitor.Left;top=$monitor.Top;right=$monitor.Right;bottom=$monitor.Bottom}
        geometry=[ordered]@{dpi=$dpi;scale=$scale;width_dip=$widthDip;height_dip=$heightDip;taskbar_gap_dip=$gapDip;overlap_physical_px=$overlap;contained=$contained}
        home_screenshot=(Split-Path -Leaf $HomeScreenshotPath);home_sha256=(Get-Sha256 $HomeScreenshotPath)
        all_apps_screenshot=(Split-Path -Leaf $AllAppsScreenshotPath);all_apps_sha256=(Get-Sha256 $AllAppsScreenshotPath)
        power_screenshot=if($PowerScreenshotPath){Split-Path -Leaf $PowerScreenshotPath}else{$null};power_sha256=$powerHash
    }
    [IO.File]::WriteAllText($OutputPath,(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report|ConvertTo-Json -Depth 8
} finally {
    if($process-and-not$process.HasExited){Stop-Process $process.Id -Force -ErrorAction SilentlyContinue}
    if($suppressor-and-not$suppressor.HasExited){Stop-Process $suppressor.Id -Force -ErrorAction SilentlyContinue}
    if($SuppressExplorer-and-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process $explorerPath}
    if($watchdog-and-not$watchdog.HasExited){Stop-Process $watchdog.Id -Force -ErrorAction SilentlyContinue}
    if($null -eq $priorSurface){Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface}
    if($null -eq $priorTrace){Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_ACTION_TRACE=$priorTrace}
    if($null -eq $priorLocale){Remove-Item Env:SUPERDESKTOP_LOCALE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_LOCALE=$priorLocale}
}
