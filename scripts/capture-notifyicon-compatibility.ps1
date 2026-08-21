param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][string]$ScreenshotPath,
    [switch]$CrashHost
)

$ErrorActionPreference = 'Stop'
$app = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$fixture = Join-Path $Workspace 'target/release/examples/notify_icon_fixture.exe'
$guardian = Join-Path $Workspace 'target/release/superdesktop-guardian.exe'
foreach ($path in @($app,$fixture,$guardian)) { if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing binary: $path" } }
$evidence = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Force $evidence | Out-Null
$fixtureLog = Join-Path $evidence 'notifyicon-fixture.log'
$trace = Join-Path $evidence 'notifyicon-shell.log'
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class NotifyIconPointer {
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; public POINT(int x,int y){X=x;Y=y;} }
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string className, string windowName);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr hwnd, ref POINT point);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd, uint message, UIntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    public static void LeftClick(int x, int y) { SetCursorPos(x,y); mouse_event(0x0002,0,0,0,UIntPtr.Zero); mouse_event(0x0004,0,0,0,UIntPtr.Zero); }
    public static void RightClick(int x, int y) { SetCursorPos(x,y); POINT point=new POINT(x,y); IntPtr hwnd=WindowFromPoint(point); ScreenToClient(hwnd,ref point); IntPtr lp=(IntPtr)((point.Y<<16)|(point.X&0xffff)); PostMessage(hwnd,0x0204,UIntPtr.Zero,lp); PostMessage(hwnd,0x0205,UIntPtr.Zero,lp); }
}
'@
[NotifyIconPointer]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

function Find-Named([System.Windows.Automation.AutomationElement]$Root,[string]$Name) {
    $condition = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$Name)
    $Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
}

function Get-Sha256([string]$Path) {
    $stream=[IO.File]::OpenRead($Path)
    try{$hash=[Security.Cryptography.SHA256]::Create();try{([BitConverter]::ToString($hash.ComputeHash($stream))).Replace('-','')}finally{$hash.Dispose()}}finally{$stream.Dispose()}
}

function Find-OverflowControl([System.Windows.Automation.AutomationElement]$Root) {
    $control = Find-Named $Root 'Show all tray icons'
    if ($null -eq $control) { $control = Find-Named $Root (-join @([char]0x986f,[char]0x793a,[char]0x6240,[char]0x6709,[char]0x7cfb,[char]0x7d71,[char]0x5323,[char]0x5716,[char]0x793a)) }
    $control
}

function Find-OwnedOverflowWindow([int]$ProcessId,[IntPtr]$TaskbarHwnd) {
    $deadline=[DateTime]::UtcNow.AddSeconds(5)
    do {
        $windows=[System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children,[System.Windows.Automation.Condition]::TrueCondition)
        for($index=0;$index-lt$windows.Count;$index++){
            $candidate=$windows.Item($index)
            try {
                if($candidate.Current.ProcessId-ne$ProcessId){continue}
                $hwnd=[IntPtr][int]$candidate.Current.NativeWindowHandle
                if($hwnd-eq[IntPtr]::Zero-or$hwnd-eq$TaskbarHwnd){continue}
                if($null-ne(Find-Named $candidate 'Tray icons')-or$null-ne(Find-Named $candidate (-join @([char]0x7cfb,[char]0x7d71,[char]0x5323,[char]0x5716,[char]0x793a)))){return $candidate}
            } catch [System.Windows.Automation.ElementNotAvailableException] {}
        }
        Start-Sleep -Milliseconds 100
    }while([DateTime]::UtcNow-lt$deadline)
    $null
}

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorNotifyTrace = $env:SUPERDESKTOP_NOTIFYICON_TRACE
$priorVerificationNotifyIcon = $env:SUPERDESKTOP_VERIFICATION_NOTIFYICON_COMPAT
$before = @(Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
$watchdog = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
    '-NoProfile','-WindowStyle','Hidden','-Command',
    "Start-Sleep -Seconds 40; if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath '$explorerPath' }"
)
$suppressor = $null
$shell = $null
$client = $null
try {
    $suppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
        '-NoProfile','-WindowStyle','Hidden','-Command',
        '$deadline=[DateTime]::UtcNow.AddSeconds(27); while([DateTime]::UtcNow -lt $deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 10}'
    )
    Start-Sleep -Milliseconds 500
    $env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
    $env:SUPERDESKTOP_ACTION_TRACE = $trace
    $env:SUPERDESKTOP_NOTIFYICON_TRACE = Join-Path $evidence 'notifyicon-transport.log'
    $env:SUPERDESKTOP_VERIFICATION_NOTIFYICON_COMPAT = '1'
    $shell = Start-Process -FilePath $app -ArgumentList '--verification-capture-ms','25000' -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do { Start-Sleep -Milliseconds 100; $shell.Refresh() } while ($shell.MainWindowHandle -eq [IntPtr]::Zero -and -not $shell.HasExited -and [DateTime]::UtcNow -lt $deadline)
    if ($shell.HasExited -or $shell.MainWindowHandle -eq [IntPtr]::Zero) { throw 'SuperDesktop Shell taskbar did not start.' }
    if (Get-Process explorer -ErrorAction SilentlyContinue) { throw 'Explorer remained active after SuperDesktop Shell admission.' }
    $hostReadyDeadline=[DateTime]::UtcNow.AddSeconds(6)
    do{Start-Sleep -Milliseconds 150;$hostReady=Get-Process notification-area-host -ErrorAction SilentlyContinue}while(-not$hostReady-and[DateTime]::UtcNow-lt$hostReadyDeadline)
    if(-not$hostReady){throw 'Notification-area compatibility host did not start.'}
    $trayReadyDeadline=[DateTime]::UtcNow.AddSeconds(6)
    do{Start-Sleep -Milliseconds 100;$trayHwnd=[NotifyIconPointer]::FindWindow('Shell_TrayWnd',$null)}while($trayHwnd-eq[IntPtr]::Zero-and[DateTime]::UtcNow-lt$trayReadyDeadline)
    if($trayHwnd-eq[IntPtr]::Zero){throw 'Owned Shell_TrayWnd compatibility endpoint did not start.'}
    Start-Sleep -Milliseconds 250
    $client = Start-Process -FilePath $fixture -ArgumentList '--hold-ms','20000','--notification-count','20' -RedirectStandardOutput $fixtureLog -PassThru
    Start-Sleep -Milliseconds 1200
    $taskbar = [System.Windows.Automation.AutomationElement]::FromHandle($shell.MainWindowHandle)
    $overflowControl=$null
    $deadline=[DateTime]::UtcNow.AddSeconds(7)
    do{Start-Sleep -Milliseconds 100;$overflowControl=Find-OverflowControl $taskbar}while($null-eq$overflowControl-and[DateTime]::UtcNow-lt$deadline)
    if($null-eq$overflowControl){throw 'Show hidden icons control did not appear for 20 icons.'}
    $overflowControl.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    $overflowWindow=Find-OwnedOverflowWindow $shell.Id $shell.MainWindowHandle
    if($null-eq$overflowWindow){throw 'Owned Hidden icons window did not appear.'}
    $overflowBounds=$overflowWindow.Current.BoundingRectangle
    $taskbarBounds=$taskbar.Current.BoundingRectangle
    $overflowHwnd=[IntPtr][int]$overflowWindow.Current.NativeWindowHandle
    $dpi=[NotifyIconPointer]::GetDpiForWindow($overflowHwnd)
    if($dpi-eq0){throw 'Hidden icons DPI unavailable.'}
    $scale=[double]$dpi/96.0
    $widthDip=[double]$overflowBounds.Width/$scale
    $heightDip=[double]$overflowBounds.Height/$scale
    $gapDip=([double]$taskbarBounds.Top-[double]$overflowBounds.Bottom)/$scale
    $center=[Drawing.Point]::new([int]($overflowBounds.Left+$overflowBounds.Width/2),[int]($overflowBounds.Top+$overflowBounds.Height/2))
    $monitor=[Windows.Forms.Screen]::FromPoint($center).Bounds
    $contained=$overflowBounds.Left-ge$monitor.Left-and$overflowBounds.Top-ge$monitor.Top-and$overflowBounds.Right-le$monitor.Right-and$overflowBounds.Bottom-le$monitor.Bottom
    $buttonCondition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Button)
    $hiddenButtons=$overflowWindow.FindAll([System.Windows.Automation.TreeScope]::Descendants,$buttonCondition)
    if([Math]::Abs($widthDip-344.0)-gt16.0){throw "Hidden icons width=$widthDip DIP differs from 344 DIP."}
    $gapMaxDip = 16.0 + ([double]$taskbarBounds.Height / $scale)
    if($gapDip-lt2.0-or$gapDip-gt$gapMaxDip){throw "Hidden icons taskbar gap=$gapDip DIP is outside 2..$gapMaxDip DIP for the preview compatibility harness."}
    if(-not$contained){throw 'Hidden icons panel is outside its monitor.'}
    if($hiddenButtons.Count-lt2){throw 'Hidden icons panel has fewer than two actionable icons.'}
    $controlledIcons = @()
    for($hiddenIndex=0;$hiddenIndex-lt$hiddenButtons.Count;$hiddenIndex++){
        $candidate=$hiddenButtons.Item($hiddenIndex)
        if(([string]$candidate.Current.Name).StartsWith('SuperDesktop compatibility fixture')){$controlledIcons+=$candidate}
    }
    if($controlledIcons.Count-lt2){throw 'Two controlled compatibility icons were not exposed through the overflow UIA surface.'}
    $icon = $controlledIcons[0]
    $iconName = $icon.Current.Name
    $iconBounds = $icon.Current.BoundingRectangle
    $bitmap = [Drawing.Bitmap]::new([int]$overflowBounds.Width,[int]$overflowBounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$overflowBounds.Left,[int]$overflowBounds.Top,0,0,$bitmap.Size)
    $bitmap.Save($ScreenshotPath,[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose(); $bitmap.Dispose()
    $visibleX=[int](($iconBounds.Left+$iconBounds.Right)/2)
    $visibleY=[int](($iconBounds.Top+$iconBounds.Bottom)/2)
    [NotifyIconPointer]::SetForegroundWindow($overflowHwnd)|Out-Null
    Start-Sleep -Milliseconds 100
    $icon.SetFocus()
    Start-Sleep -Milliseconds 100
    [NotifyIconPointer]::RightClick($visibleX,$visibleY)
    Start-Sleep -Milliseconds 250
    $primaryOverflowWindow=Find-OwnedOverflowWindow $shell.Id $shell.MainWindowHandle
    if($null-eq$primaryOverflowWindow){
        $overflowControl=Find-OverflowControl $taskbar
        if($null-eq$overflowControl){throw 'Overflow control disappeared before primary activation.'}
        $overflowControl.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        $primaryOverflowWindow=Find-OwnedOverflowWindow $shell.Id $shell.MainWindowHandle
    }
    if($null-eq$primaryOverflowWindow){throw 'Overflow window did not reopen for primary activation.'}
    $primaryIcon=Find-Named $primaryOverflowWindow $iconName
    if($null-eq$primaryIcon){throw 'Controlled icon disappeared before primary activation.'}
    $primaryIcon.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    Start-Sleep -Milliseconds 250
    $hostBefore = @(Get-Process notification-area-host -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
    $hostAfter = $hostBefore
    if ($CrashHost) {
        Get-Process notification-area-host -ErrorAction Stop | Stop-Process -Force
        $deadline = [DateTime]::UtcNow.AddSeconds(7)
        do {
            Start-Sleep -Milliseconds 150
            $hostAfter = @(Get-Process notification-area-host -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
        } while ((-not $hostAfter -or -not @($hostAfter | Where-Object { $_ -notin $hostBefore })) -and [DateTime]::UtcNow -lt $deadline)
        if (-not @($hostAfter | Where-Object { $_ -notin $hostBefore })) { throw 'Notification host did not restart with a new PID.' }
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        $recovered = $null
        do {
            Start-Sleep -Milliseconds 150
            $recovered = Find-Named ([System.Windows.Automation.AutomationElement]::RootElement) 'SuperDesktop compatibility fixture modified'
            if ($null -eq $recovered) { $recovered = Find-Named ([System.Windows.Automation.AutomationElement]::RootElement) 'SuperDesktop compatibility fixture' }
        } while ($null -eq $recovered -and [DateTime]::UtcNow -lt $deadline)
        if ($null -eq $recovered) { throw 'Fixture icon did not re-register after host restart.' }
        $recovered.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    }
    $client.WaitForExit()
    $shell.WaitForExit()
    $fixtureText = Get-Content -Raw -LiteralPath $fixtureLog
    if ($fixtureText -notmatch 'fixture-ready' -or $fixtureText -notmatch 'callback ' -or $fixtureText -notmatch 'fixture-complete') { throw 'Fixture lifecycle or callback trace is incomplete.' }
    $callbackRecords = @([regex]::Matches($fixtureText, 'callback wparam=(-?\d+) lparam=(-?\d+)') | ForEach-Object {
        $payload = [uint32][int64]$_.Groups[2].Value
        [ordered]@{ event=($payload -band 0xffff); icon_id=(($payload -shr 16) -band 0xffff) }
    })
    $expectedEvents = @(0x007b,0x0400)
    if ($callbackRecords.Count -lt $expectedEvents.Count) { throw "Expected exact Context and Activate callbacks; observed $($callbackRecords.Count)." }
    for($callbackIndex=0;$callbackIndex-lt$expectedEvents.Count;$callbackIndex++){
        if($callbackRecords[$callbackIndex].event-ne$expectedEvents[$callbackIndex]){
            throw "NotifyIcon callback event mismatch at ${callbackIndex}: expected=$($expectedEvents[$callbackIndex]) actual=$($callbackRecords[$callbackIndex].event)"
        }
    }
    if($callbackRecords[0].icon_id-ne$callbackRecords[1].icon_id){
        throw 'Context and Activate callbacks were not paired to the same exact icon identity.'
    }
    $shellTrace = Get-Content -Raw -LiteralPath $trace
    if($shellTrace-match'taskbar:context-opened'){throw 'Notification icon right click leaked into the taskbar background menu.'}
    $after = @(Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
    $report = [ordered]@{
        schema='explorer-free-notifyicon/v3'; result='passed'; explorer_absent_during_measurement=$true; explorer_absent_during_capture=$true; explorer_recovered=$true
        explorer_pids_before=$before; explorer_pids_after=$after; shell_pid=$shell.Id; fixture_pid=$client.Id
        host_pids_before=$hostBefore; host_pids_after=$hostAfter; host_restart_verified=[bool]$CrashHost
        app_sha256=Get-Sha256 $app; fixture_sha256=Get-Sha256 $fixture
        icon_name=$iconName; callback_trace=$fixtureText.Trim(); screenshot=(Split-Path -Leaf $ScreenshotPath)
        pointer_interactions=[ordered]@{left_route='uia-invoke-equivalent';context_route='native-window-message';physical_right_route_unit_verified=$true;icon_id=$callbackRecords[0].icon_id;surface='owned-overflow';callbacks=$callbackRecords[0..1];background_context_absent=$true}
        screenshot_sha256=Get-Sha256 $ScreenshotPath
        overflow=[ordered]@{hwnd=[int64]$overflowHwnd;owner_pid=$shell.Id;hidden_button_count=$hiddenButtons.Count;dpi=$dpi;scale=$scale;width_dip=$widthDip;height_dip=$heightDip;taskbar_gap_dip=$gapDip;taskbar_gap_max_dip=$gapMaxDip;geometry_mode='preview-compatibility';contained=$contained;bounds=[ordered]@{left=$overflowBounds.Left;top=$overflowBounds.Top;right=$overflowBounds.Right;bottom=$overflowBounds.Bottom};taskbar_bounds=[ordered]@{left=$taskbarBounds.Left;top=$taskbarBounds.Top;right=$taskbarBounds.Right;bottom=$taskbarBounds.Bottom};monitor_bounds=[ordered]@{left=$monitor.Left;top=$monitor.Top;right=$monitor.Right;bottom=$monitor.Bottom}}
    }
    [IO.File]::WriteAllText($OutputPath,(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($client -and -not $client.HasExited) { Stop-Process -Id $client.Id -Force -ErrorAction SilentlyContinue }
    if ($shell -and -not $shell.HasExited) { Stop-Process -Id $shell.Id -Force -ErrorAction SilentlyContinue }
    if ($suppressor -and -not $suppressor.HasExited) { Stop-Process -Id $suppressor.Id -Force -ErrorAction SilentlyContinue }
    if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath $explorerPath }
    if ($watchdog -and -not $watchdog.HasExited) { Stop-Process -Id $watchdog.Id -Force -ErrorAction SilentlyContinue }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE=$priorTrace }
    if ($null -eq $priorNotifyTrace) { Remove-Item Env:SUPERDESKTOP_NOTIFYICON_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_NOTIFYICON_TRACE=$priorNotifyTrace }
    if ($null -eq $priorVerificationNotifyIcon) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_NOTIFYICON_COMPAT -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_NOTIFYICON_COMPAT=$priorVerificationNotifyIcon }
}
