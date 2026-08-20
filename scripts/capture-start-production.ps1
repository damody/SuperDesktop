param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][string]$HomeScreenshotPath,
    [Parameter(Mandatory = $true)][string]$CenteredScreenshotPath,
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
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left; public int Top; public int Right; public int Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X; public int Y; }
    public delegate bool EnumProc(IntPtr hwnd, IntPtr value);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr value);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);
    [DllImport("user32.dll")] public static extern void keybd_event(byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);
    public static void SendWinS() {
        const uint KEYUP = 0x0002;
        keybd_event(0x5B,0,0,UIntPtr.Zero);
        keybd_event(0x53,0,0,UIntPtr.Zero);
        keybd_event(0x53,0,KEYUP,UIntPtr.Zero);
        keybd_event(0x5B,0,KEYUP,UIntPtr.Zero);
    }
    public static int[] WindowRect(IntPtr hwnd) {
        Rect rect;
        if (!GetWindowRect(hwnd,out rect)) throw new System.ComponentModel.Win32Exception();
        return new [] { rect.Left, rect.Top, rect.Right, rect.Bottom };
    }
    public static int[] ClientBounds(IntPtr hwnd) {
        Rect rect;
        Point origin = new Point();
        if (!GetClientRect(hwnd,out rect) || !ClientToScreen(hwnd,ref origin)) throw new System.ComponentModel.Win32Exception();
        return new [] { origin.X, origin.Y, rect.Right - rect.Left, rect.Bottom - rect.Top };
    }
    public static void ClickPoint(int x, int y) {
        if (!SetCursorPos(x,y)) throw new System.ComponentModel.Win32Exception();
        mouse_event(0x0002,0,0,0,UIntPtr.Zero);
        mouse_event(0x0004,0,0,0,UIntPtr.Zero);
    }
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
function Root-FromHandle($Handle,[string]$Label) {
    if($null-eq$Handle-or[IntPtr]$Handle-eq[IntPtr]::Zero){throw "$Label HWND is unavailable."}
    $root=[System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$Handle)
    if($null-eq$root){throw "$Label UI Automation root is unavailable."}
    $root
}
function Click-Element($Element,[string]$Label) {
    if($null-eq$Element){throw "$Label element is unavailable."}
    $bounds=$Element.Current.BoundingRectangle
    if($bounds.Width -le 0 -or $bounds.Height -le 0){throw "$Label has no clickable bounds."}
    [StartWindows]::ClickPoint([int]($bounds.Left+$bounds.Width/2),[int]($bounds.Top+$bounds.Height/2))
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
$priorLocalAppData=$env:LOCALAPPDATA
$winlogonKey='HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'
$priorShellProperty=Get-ItemProperty -LiteralPath $winlogonKey -Name Shell -ErrorAction SilentlyContinue
$priorShellPresent=$null -ne $priorShellProperty
$priorShellValue=if($priorShellPresent){[string]$priorShellProperty.Shell}else{$null}
$preexistingWorkspacePids=@(Get-Process superdesktop-app,superdesktop-guardian -ErrorAction SilentlyContinue | Where-Object {
    try { $_.Path -like (Join-Path $Workspace 'target\release\*') } catch { $false }
} | ForEach-Object Id)
$zhTw=$Locale -eq 'zh-TW'
function Utf8-Base64([string]$Value) { [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($Value)) }
$labels=if($zhTw){@{
    Pinned=(Utf8-Base64 '5bey6YeY6YG4');Recommended=(Utf8-Base64 '5bu66K2w');AllApps=(Utf8-Base64 '5omA5pyJ5oeJ55So56iL5byP');Settings=(Utf8-Base64 '6Kit5a6a');Power=(Utf8-Base64 '6Zu75rqQ');FooterActions=(Utf8-Base64 '6ZaL5aeL5Yqf6IO96KGo5YuV5L2c');Back=(Utf8-Base64 '6L+U5Zue5bey6YeY6YG4');SignOut=(Utf8-Base64 '55m75Ye6');Restart=(Utf8-Base64 '6YeN5paw5ZWf5YuV');ShutDown=(Utf8-Base64 '6Zec5qmf');TaskbarSettings=(Utf8-Base64 '5bel5L2c5YiX6Kit5a6a');AlignmentLeft=(Utf8-Base64 '5bel5L2c5YiX5bCN6b2KLCDpnaDlt6Y=');AlignmentCenter=(Utf8-Base64 '5bel5L2c5YiX5bCN6b2KLCDnva7kuK0=');CloseTaskbarSettings=(Utf8-Base64 '6Zec6ZaJ5bel5L2c5YiX6Kit5a6a')
}}else{@{
    Pinned='Pinned';Recommended='Recommended';AllApps='All apps';Settings='Settings';Power='Power';FooterActions='Start footer actions';Back='Back to pinned';SignOut='Sign out';Restart='Restart';ShutDown='Shut down';TaskbarSettings='Taskbar settings';AlignmentLeft='Taskbar alignment, Left';AlignmentCenter='Taskbar alignment, Center';CloseTaskbarSettings='Close Taskbar settings'
}}
$startLabel=if($zhTw){Utf8-Base64 '6ZaL5aeL'}else{'Start'}
$env:SUPERDESKTOP_VERIFICATION_SURFACE='taskbar'
$env:SUPERDESKTOP_ACTION_TRACE=$tracePath
$env:LOCALAPPDATA=Join-Path (Split-Path -Parent $OutputPath) 'profile'
New-Item -ItemType Directory -Force $env:LOCALAPPDATA | Out-Null
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
    $launchProcess=Start-Process -FilePath $appPath -ArgumentList $arguments -PassThru
    $process=$launchProcess
    $taskbar=$null
    $taskbarHandle=[IntPtr]::Zero
    $start=$null
    $deadline=[DateTime]::UtcNow.AddSeconds(4)
    do {
        Start-Sleep -Milliseconds 50
        foreach($candidateProcess in @(Get-Process superdesktop-app -ErrorAction SilentlyContinue | Where-Object {
            try { $_.Path -eq $appPath } catch { $false }
        } | Sort-Object StartTime -Descending)){
            foreach($handle in @([StartWindows]::VisibleForProcess([uint32]$candidateProcess.Id))){
                if($null-eq$handle-or[IntPtr]$handle-eq[IntPtr]::Zero){continue}
                try {$candidateRoot=Root-FromHandle $handle 'Taskbar candidate'}catch{continue}
                $candidateStart=Find-Named $candidateRoot $startLabel
                if($candidateStart){$process=$candidateProcess;$taskbar=$candidateRoot;$taskbarHandle=$handle;$start=$candidateStart;break}
            }
            if($start){break}
        }
        if($start){break}
    } while([DateTime]::UtcNow-lt$deadline)
    if($null-eq$start){throw 'Taskbar window with Start did not appear.'}
    Start-Sleep -Milliseconds 900
    $taskbar=Root-FromHandle $taskbarHandle 'Taskbar'
    $start=Find-Named $taskbar $startLabel
    if($null-eq$start){throw 'Start button disappeared before invocation.'}
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
    $root=Root-FromHandle $startHandle 'Start home'
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
    $homeWindowRect=[StartWindows]::WindowRect($startHandle)
    $homeClientBounds=[StartWindows]::ClientBounds($startHandle)
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
    $screen=[Windows.Forms.Screen]::FromPoint($center)
    $monitor=$screen.Bounds
    $workArea=$screen.WorkingArea
    $contained=$homeBounds.Left-ge$monitor.Left-and$homeBounds.Top-ge$monitor.Top-and$homeBounds.Right-le$monitor.Right-and$homeBounds.Bottom-le$monitor.Bottom
    $leftInsetDip=([double]$homeClientBounds[0]-[double]$workArea.Left)/$scale
    if([Math]::Abs($widthDip-640.0)-gt16.0){throw "Owned Start width=$widthDip DIP differs from 640 DIP."}
    if($gapDip-lt4.0-or$gapDip-gt20.0){throw "Owned Start taskbar gap=$gapDip DIP is outside 4..20 DIP; taskbar=$($taskbarBounds | ConvertTo-Json -Compress) start=$($homeBounds | ConvertTo-Json -Compress) dpi=$dpi."}
    if($overlap-gt0){throw "Owned Start overlaps taskbar by $overlap physical pixels."}
    if(-not$contained){throw 'Owned Start is outside its source monitor.'}
    if($leftInsetDip-lt8.0-or$leftInsetDip-gt16.0){throw "Default Start left inset=$leftInsetDip DIP is outside 8..16 DIP."}
    if($powerWidthDip-lt38-or$powerWidthDip-gt42-or$powerHeightDip-lt38-or$powerHeightDip-gt42){throw "Start Power target=${powerWidthDip}x${powerHeightDip} DIP."}
    if($powerRightInsetDip-lt20-or$powerRightInsetDip-gt36){throw "Start Power right inset=$powerRightInsetDip DIP."}
    $all=Find-Named $root $labels.AllApps
    $all.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    Start-Sleep -Milliseconds 500
    $root=Root-FromHandle $startHandle 'Start all apps'
    if($null -eq (Find-Named $root $labels.Back)){throw 'All apps page is missing Back to pinned.'}
    $listCondition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::ListItem)
    $allItems=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$listCondition)
    if($allItems.Count -lt 6){throw "All apps exposed only $($allItems.Count) list items."}
    $allBounds=Save-Window $root $AllAppsScreenshotPath
    $powerHash=$null
    if($PowerScreenshotPath){
        (Find-Named $root $labels.Back).GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        Start-Sleep -Milliseconds 300
        $root=Root-FromHandle $startHandle 'Start pinned return'
        (Find-Named $root $labels.Power).GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        Start-Sleep -Milliseconds 300
        $root=Root-FromHandle $startHandle 'Start power menu'
        foreach($name in @($labels.SignOut,$labels.Restart,$labels.ShutDown)){if($null -eq (Find-Named $root $name)){throw "Power menu is missing $name."}}
        $null=Save-Window $root $PowerScreenshotPath
        $powerHash=(Get-Sha256 $PowerScreenshotPath)
    }
    [Windows.Forms.SendKeys]::SendWait('{ESC}')
    Start-Sleep -Milliseconds 150
    [Windows.Forms.SendKeys]::SendWait('{ESC}')
    Start-Sleep -Milliseconds 350
    $taskbar=Root-FromHandle $taskbarHandle 'Taskbar before settings'
    $start=Find-Named $taskbar $startLabel
    if($null-eq$start){throw 'Start button is missing before keyboard context invocation.'}
    $start.SetFocus()
    [Windows.Forms.SendKeys]::SendWait('+{F10}')
    $contextRoot=$null
    $deadline=[DateTime]::UtcNow.AddSeconds(4)
    do {
        Start-Sleep -Milliseconds 75
        foreach($handle in [StartWindows]::VisibleForProcess([uint32]$process.Id)){
            if($null-eq$handle-or[IntPtr]$handle-eq[IntPtr]::Zero){continue}
            $candidate=Root-FromHandle $handle 'Taskbar context candidate'
            if($null-ne(Find-Named $candidate $labels.TaskbarSettings)){$contextRoot=$candidate;break}
        }
    } while($null-eq$contextRoot-and[DateTime]::UtcNow-lt$deadline)
    if($null-eq$contextRoot){throw 'Taskbar context menu did not appear from Shift+F10.'}
    Click-Element (Find-Named $contextRoot $labels.TaskbarSettings) 'Taskbar settings command'
    $settingsRoot=$null
    $settingsHandle=[IntPtr]::Zero
    $deadline=[DateTime]::UtcNow.AddSeconds(4)
    do {
        Start-Sleep -Milliseconds 75
        foreach($handle in [StartWindows]::VisibleForProcess([uint32]$process.Id)){
            if($null-eq$handle-or[IntPtr]$handle-eq[IntPtr]::Zero){continue}
            $candidate=Root-FromHandle $handle 'Taskbar settings candidate'
            if($null-ne(Find-Named $candidate $labels.AlignmentLeft)){$settingsRoot=$candidate;$settingsHandle=$handle;break}
        }
    } while($null-eq$settingsRoot-and[DateTime]::UtcNow-lt$deadline)
    if($null-eq$settingsRoot){throw 'Taskbar settings did not expose the default Left alignment.'}
    for($index=0;$index-lt6;$index++){[Windows.Forms.SendKeys]::SendWait('{DOWN}');Start-Sleep -Milliseconds 40}
    [Windows.Forms.SendKeys]::SendWait(' ')
    $deadline=[DateTime]::UtcNow.AddSeconds(4)
    do {
        Start-Sleep -Milliseconds 75
        $settingsRoot=Root-FromHandle $settingsHandle 'Taskbar settings refresh'
        $centeredSetting=Find-Named $settingsRoot $labels.AlignmentCenter
    } while($null-eq$centeredSetting-and[DateTime]::UtcNow-lt$deadline)
    if($null-eq$centeredSetting){throw 'Taskbar alignment did not save Center.'}
    [Windows.Forms.SendKeys]::SendWait('{ESC}')
    Start-Sleep -Milliseconds 350
    [StartWindows]::SendWinS()
    $centeredRoot=$null
    $centeredHandle=[IntPtr]::Zero
    $deadline=[DateTime]::UtcNow.AddSeconds(4)
    do {
        Start-Sleep -Milliseconds 75
        foreach($handle in [StartWindows]::VisibleForProcess([uint32]$process.Id)){
            if($handle-eq$taskbarHandle){continue}
            if($null-eq$handle-or[IntPtr]$handle-eq[IntPtr]::Zero){continue}
            $candidate=Root-FromHandle $handle 'Centered Start candidate'
            if($null-ne(Find-Named $candidate $labels.Pinned)){$centeredRoot=$candidate;$centeredHandle=$handle;break}
        }
    } while($null-eq$centeredRoot-and[DateTime]::UtcNow-lt$deadline)
    if($null-eq$centeredRoot){throw 'Win+S did not reopen owned Start.'}
    $centeredBounds=Save-Window $centeredRoot $CenteredScreenshotPath
    $centeredWindowRect=[StartWindows]::WindowRect($centeredHandle)
    $centeredClientBounds=[StartWindows]::ClientBounds($centeredHandle)
    $centeredDpi=[StartWindows]::GetDpiForWindow($centeredHandle)
    $centeredScale=[double]$centeredDpi/96.0
    $centerOffsetDip=[Math]::Abs((([double]$centeredClientBounds[0]+[double]$centeredClientBounds[2]/2.0)-([double]$workArea.Left+[double]$workArea.Width/2.0))/$centeredScale)
    if($centerOffsetDip-gt4.0){throw "Centered Start offset=$centerOffsetDip DIP exceeds 4 DIP."}
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
        default_left_home_bounds=[ordered]@{left=[int]$homeWindowRect[0];top=[int]$homeWindowRect[1];right=[int]$homeWindowRect[2];bottom=[int]$homeWindowRect[3];client_left=[int]$homeClientBounds[0];client_top=[int]$homeClientBounds[1];client_width=[int]$homeClientBounds[2];client_height=[int]$homeClientBounds[3];left_inset_dip=$leftInsetDip;uia_left=[int]$homeBounds.Left;uia_top=[int]$homeBounds.Top;uia_width=[int]$homeBounds.Width;uia_height=[int]$homeBounds.Height}
        centered_keyboard_bounds=[ordered]@{left=[int]$centeredWindowRect[0];top=[int]$centeredWindowRect[1];right=[int]$centeredWindowRect[2];bottom=[int]$centeredWindowRect[3];client_left=[int]$centeredClientBounds[0];client_top=[int]$centeredClientBounds[1];client_width=[int]$centeredClientBounds[2];client_height=[int]$centeredClientBounds[3];center_offset_dip=$centerOffsetDip;uia_left=[int]$centeredBounds.Left;uia_top=[int]$centeredBounds.Top;uia_width=[int]$centeredBounds.Width;uia_height=[int]$centeredBounds.Height}
        taskbar_bounds=[ordered]@{left=[int]$taskbarBounds.Left;top=[int]$taskbarBounds.Top;right=[int]$taskbarBounds.Right;bottom=[int]$taskbarBounds.Bottom}
        monitor_bounds=[ordered]@{left=$monitor.Left;top=$monitor.Top;right=$monitor.Right;bottom=$monitor.Bottom}
        geometry=[ordered]@{dpi=$dpi;scale=$scale;width_dip=$widthDip;height_dip=$heightDip;taskbar_gap_dip=$gapDip;overlap_physical_px=$overlap;contained=$contained}
        home_screenshot=(Split-Path -Leaf $HomeScreenshotPath);home_sha256=(Get-Sha256 $HomeScreenshotPath)
        centered_screenshot=(Split-Path -Leaf $CenteredScreenshotPath);centered_sha256=(Get-Sha256 $CenteredScreenshotPath)
        all_apps_screenshot=(Split-Path -Leaf $AllAppsScreenshotPath);all_apps_sha256=(Get-Sha256 $AllAppsScreenshotPath)
        power_screenshot=if($PowerScreenshotPath){Split-Path -Leaf $PowerScreenshotPath}else{$null};power_sha256=$powerHash
    }
    [IO.File]::WriteAllText($OutputPath,(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report|ConvertTo-Json -Depth 8
} finally {
    if($priorShellPresent){Set-ItemProperty -LiteralPath $winlogonKey -Name Shell -Value $priorShellValue -Type String}else{Remove-ItemProperty -LiteralPath $winlogonKey -Name Shell -ErrorAction SilentlyContinue}
    if($process-and-not$process.HasExited){Stop-Process $process.Id -Force -ErrorAction SilentlyContinue}
    if($launchProcess-and-not$launchProcess.HasExited){Stop-Process $launchProcess.Id -Force -ErrorAction SilentlyContinue}
    Get-Process superdesktop-app,superdesktop-guardian -ErrorAction SilentlyContinue | Where-Object {
        try { $_.Id -notin $preexistingWorkspacePids -and $_.Path -like (Join-Path $Workspace 'target\release\*') } catch { $false }
    } | Stop-Process -Force -ErrorAction SilentlyContinue
    if($suppressor-and-not$suppressor.HasExited){Stop-Process $suppressor.Id -Force -ErrorAction SilentlyContinue}
    if($SuppressExplorer-and-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process $explorerPath}
    if($watchdog-and-not$watchdog.HasExited){Stop-Process $watchdog.Id -Force -ErrorAction SilentlyContinue}
    if($null -eq $priorSurface){Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface}
    if($null -eq $priorTrace){Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_ACTION_TRACE=$priorTrace}
    if($null -eq $priorLocale){Remove-Item Env:SUPERDESKTOP_LOCALE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_LOCALE=$priorLocale}
    if($null -eq $priorLocalAppData){Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue}else{$env:LOCALAPPDATA=$priorLocalAppData}
}
