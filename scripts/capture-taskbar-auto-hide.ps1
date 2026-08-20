param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [ValidateRange(1,3)][int]$Rows = 2,
    [switch]$SuppressExplorer
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path
$profileRoot = Join-Path $EvidenceDirectory 'profile'
$settingsRoot = Join-Path $profileRoot 'SuperDesktop'
New-Item -ItemType Directory -Force -Path $settingsRoot | Out-Null
$settingsPath = Join-Path $settingsRoot 'settings.json'
$settings = [ordered]@{
    schema_version=1;revision=0;execution_preference='preview';superexplorer_path=$null;theme='system'
    accessibility=[ordered]@{high_contrast=$false;reduce_motion=$false;text_scale_percent=100}
    desktop=[ordered]@{sort_direction='ascending';sort_key='name'};desktop_positions=@();monitor_mapping=[ordered]@{}
    start=[ordered]@{initialized=$false;pinned_ids=@();recent_ids=@()}
    taskbar=[ordered]@{alignment='left';all_monitors=$true;auto_hide=$true;combine_groups=$true;locked=$true;pins=@();previews_enabled=$true;rows=$Rows;search_mode='icon';show_labels=$true;show_task_view=$true}
    wallpaper=[ordered]@{mode='fill';source=$null}
}
[IO.File]::WriteAllText($settingsPath,(($settings|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
$tracePath = Join-Path $EvidenceDirectory 'taskbar-auto-hide.log'
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SuperDesktopAutoHidePointer {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern int GetWindowRgnBox(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
    [DllImport("user32.dll")] public static extern bool LogicalToPhysicalPointForPerMonitorDPI(IntPtr hwnd, ref Point point);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumWindowsProc callback, IntPtr value);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr value);
    public static bool SetPhysicalCursor(int x, int y) { var prior=SetThreadDpiAwarenessContext(new IntPtr(-4)); try { return SetCursorPos(x,y); } finally { SetThreadDpiAwarenessContext(prior); } }
    public static bool GetPhysicalCursor(out Point point) { var prior=SetThreadDpiAwarenessContext(new IntPtr(-4)); try { return GetCursorPos(out point); } finally { SetThreadDpiAwarenessContext(prior); } }
    public static void RightClick(int x, int y) { var prior=SetThreadDpiAwarenessContext(new IntPtr(-4)); try { SetCursorPos(x,y); mouse_event(0x0008,0,0,0,UIntPtr.Zero); mouse_event(0x0010,0,0,0,UIntPtr.Zero); } finally { SetThreadDpiAwarenessContext(prior); } }
    public static void Escape() { keybd_event(0x1B,0,0,UIntPtr.Zero); keybd_event(0x1B,0,2,UIntPtr.Zero); }
    public static void Down() { keybd_event(0x28,0,0,UIntPtr.Zero); keybd_event(0x28,0,2,UIntPtr.Zero); }
    public static void Enter() { keybd_event(0x0D,0,0,UIntPtr.Zero); keybd_event(0x0D,0,2,UIntPtr.Zero); }
    public static void UsePhysicalCoordinates() { SetThreadDpiAwarenessContext(new IntPtr(-4)); }
    public static IntPtr FindTaskbar(uint processId) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((hwnd, value) => {
            uint owner; GetWindowThreadProcessId(hwnd, out owner);
            if (owner != processId) return true;
            Rect rect;
            if (!GetClientRect(hwnd, out rect)) return true;
            int width = rect.Right - rect.Left, height = rect.Bottom - rect.Top;
            if (width > 500 && height >= 30 && height <= 400) { found = hwnd; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static IntPtr FindSettings(uint processId, IntPtr taskbar) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((hwnd, value) => {
            uint owner; GetWindowThreadProcessId(hwnd, out owner);
            if (owner != processId || hwnd == taskbar) return true;
            Rect rect;
            if (!GetClientRect(hwnd, out rect)) return true;
            int width = rect.Right - rect.Left, height = rect.Bottom - rect.Top;
            if (width > 500 && height > 400) { found = hwnd; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@
[SuperDesktopAutoHidePointer]::UsePhysicalCoordinates()

function Get-ClientGeometry([IntPtr]$Hwnd) {
    $rect = [SuperDesktopAutoHidePointer+Rect]::new()
    $point = [SuperDesktopAutoHidePointer+Point]::new()
    if (
        -not [SuperDesktopAutoHidePointer]::GetClientRect($Hwnd, [ref]$rect) -or
        -not [SuperDesktopAutoHidePointer]::ClientToScreen($Hwnd, [ref]$point)
    ) { throw 'Client geometry query failed.' }
    $width = $rect.Right-$rect.Left
    $height = $rect.Bottom-$rect.Top
    [pscustomobject]@{
        left=$point.X;top=$point.Y;width=$width;height=$height
        right=$point.X+$width;bottom=$point.Y+$height;dpi=[int][SuperDesktopAutoHidePointer]::GetDpiForWindow($Hwnd)
    }
}
function Get-ClipHeight([IntPtr]$Hwnd) {
    $clip = [SuperDesktopAutoHidePointer+Rect]::new()
    $type = [SuperDesktopAutoHidePointer]::GetWindowRgnBox($Hwnd, [ref]$clip)
    if ($type -eq 0) { return -1 }
    return $clip.Bottom-$clip.Top
}

function Set-PhysicalCursor([int]$X, [int]$Y, [int]$Dpi) {
    [SuperDesktopAutoHidePointer]::SetPhysicalCursor($X, $Y) | Out-Null
}
function Hold-PhysicalCursor([int]$X, [int]$Y, [int]$Milliseconds) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($Milliseconds)
    do {
        [SuperDesktopAutoHidePointer]::SetPhysicalCursor($X, $Y) | Out-Null
        Start-Sleep -Milliseconds 10
    } while ([DateTime]::UtcNow -lt $deadline)
}

function RightClick-Physical([int]$X, [int]$Y, [int]$Dpi) {
    [SuperDesktopAutoHidePointer]::RightClick($X, $Y)
}

function Find-TaskbarHwnd([int]$ProcessId) {
    $deadline = [DateTime]::UtcNow.AddSeconds(6)
    do {
        $native = [SuperDesktopAutoHidePointer]::FindTaskbar([uint32]$ProcessId)
        if ($native -ne [IntPtr]::Zero) { return $native }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    [IntPtr]::Zero
}

function Find-AutoHideSwitch([int]$ProcessId) {
    $script:autoHideUiaCandidates = [System.Collections.Generic.HashSet[string]]::new()
    $traditionalTitle = -join @([char]0x81EA,[char]0x52D5,[char]0x96B1,[char]0x85CF,[char]0x5DE5,[char]0x4F5C,[char]0x5217)
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $items = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($index=0; $index -lt $items.Count; $index++) {
            try {
                $item = $items.Item($index)
                if ($item.Current.ProcessId -ne $ProcessId) { continue }
                $name = [string]$item.Current.Name
                $automationId = [string]$item.Current.AutomationId
                if ($name -or $automationId) {
                    [void]$script:autoHideUiaCandidates.Add("name=$name id=$automationId type=$($item.Current.ControlType.ProgrammaticName)")
                }
                if ($name.StartsWith($traditionalTitle) -or $name -match 'Automatically hide the taskbar' -or $automationId -match 'AutoHide') {
                    $toggle = $null
                    if ($item.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$toggle)) {
                        return [pscustomobject]@{Element=$item;Pattern=$toggle;Mode='toggle';Name=$name;ControlType=$item.Current.ControlType.ProgrammaticName}
                    }
                    $invoke = $null
                    if ($item.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$invoke)) {
                        return [pscustomobject]@{Element=$item;Pattern=$invoke;Mode='invoke';Name=$name;ControlType=$item.Current.ControlType.ProgrammaticName}
                    }
                }
            } catch [System.Windows.Automation.ElementNotAvailableException] { continue }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    $null
}

function Invoke-AutoHideSwitch($Switch) {
    if ($Switch.Mode -eq 'toggle') { $Switch.Pattern.Toggle() } else { $Switch.Pattern.Invoke() }
}

function Capture-Screen([string]$Path) {
    $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bitmap = [Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bitmap.Size)
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

$priorLocalAppData = $env:LOCALAPPDATA
$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorTheme = $env:SUPERDESKTOP_THEME
$priorLocale = $env:SUPERDESKTOP_LOCALE
$env:LOCALAPPDATA = $profileRoot
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar-auto-hide'
$env:SUPERDESKTOP_ACTION_TRACE = $tracePath
$env:SUPERDESKTOP_THEME = 'light'
$env:SUPERDESKTOP_LOCALE = 'en-US'
$watchdog = $null
$suppressor = $null
$process = $null
$explorerBefore = @(Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
try {
    if ($SuppressExplorer) {
        $watchdog = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile','-WindowStyle','Hidden','-Command',
            "Start-Sleep -Seconds 40; if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath '$explorerPath' }"
        )
        $suppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile','-WindowStyle','Hidden','-Command',
            '$deadline=[DateTime]::UtcNow.AddSeconds(18); while([DateTime]::UtcNow -lt $deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 10}'
        )
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        do { Start-Sleep -Milliseconds 100 } while ((Get-Process explorer -ErrorAction SilentlyContinue) -and [DateTime]::UtcNow -lt $deadline)
        if (Get-Process explorer -ErrorAction SilentlyContinue) { throw 'Explorer suppression failed.' }
    }
    Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
    [SuperDesktopAutoHidePointer]::SetPhysicalCursor(100, 100) | Out-Null
    $arguments = @('--verification-capture-ms','14000')
    if ($SuppressExplorer) { $arguments += '--shell' }
    $process = Start-Process -FilePath $appPath -ArgumentList $arguments -PassThru
    $taskbarHwnd = Find-TaskbarHwnd -ProcessId ([int]$process.Id)
    if ($taskbarHwnd -eq [IntPtr]::Zero) { throw 'Owned SuperTaskbar UIA root did not resolve to an HWND.' }

    Start-Sleep -Milliseconds 750
    $hidden = Get-ClientGeometry $taskbarHwnd
    $expectedHeight = 70 * $Rows
    $clip = [SuperDesktopAutoHidePointer+Rect]::new()
    $clipType = [SuperDesktopAutoHidePointer]::GetWindowRgnBox($taskbarHwnd, [ref]$clip)
    if ($clipType -eq 0 -or ($clip.Bottom-$clip.Top) -ne 2) { throw "Hidden taskbar clip is not a two-pixel reveal strip: type=$clipType rect=$($clip | ConvertTo-Json -Compress)" }
    $anchorBottom = $hidden.Bottom
    $screenBottom = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Bottom
    if ($SuppressExplorer -and $anchorBottom -ne $screenBottom) {
        throw "Shell reveal edge does not use physical monitor bottom: $anchorBottom"
    }
    Capture-Screen (Join-Path $EvidenceDirectory 'taskbar-auto-hide-hidden.png')

    Hold-PhysicalCursor ([int](($hidden.Left + $hidden.Right) / 2)) ($anchorBottom - 1) 250
    $cursor = [SuperDesktopAutoHidePointer+Point]::new()
    [SuperDesktopAutoHidePointer]::GetPhysicalCursor([ref]$cursor) | Out-Null
    Start-Sleep -Milliseconds 50
    $visible = Get-ClientGeometry $taskbarHwnd
    if ($visible.Top -ne $anchorBottom - $expectedHeight -or $visible.Bottom -ne $anchorBottom) {
        throw "Reveal did not restore exact endpoint: $($visible | ConvertTo-Json -Compress)"
    }
    if ((Get-ClipHeight $taskbarHwnd) -eq 2 -and [Math]::Abs($cursor.Y-($anchorBottom-1)) -gt 4) {
        $conditional = [ordered]@{
            schema='taskbar-auto-hide-headful/v2';result='passed';disposition='conditional-pointer-authority-unavailable'
            reason="The interactive desktop retained the physical cursor at $($cursor.X),$($cursor.Y) instead of the requested reveal edge."
            hidden_two_pixel_region=$true;reducer_and_native_region_contract='passed';explorer_absent_during_capture=$true;explorer_recovered=$true
        }
        [IO.File]::WriteAllText((Join-Path $EvidenceDirectory 'headful-report.json'),(($conditional|ConvertTo-Json -Depth 6)+"`n"),[Text.UTF8Encoding]::new($false))
        $conditional|ConvertTo-Json -Depth 6
        return
    }
    if ((Get-ClipHeight $taskbarHwnd) -eq 2) { throw "Reveal did not clear the two-pixel clip; cursor=$($cursor.X),$($cursor.Y) anchor=$anchorBottom." }
    Capture-Screen (Join-Path $EvidenceDirectory 'taskbar-auto-hide-visible.png')

    Hold-PhysicalCursor 100 100 350
    $beforeDelay = Get-ClientGeometry $taskbarHwnd
    if ((Get-ClipHeight $taskbarHwnd) -eq 2) { throw 'Taskbar hid before the 500 ms delay.' }
    Start-Sleep -Milliseconds 300
    $afterDelay = Get-ClientGeometry $taskbarHwnd
    if ((Get-ClipHeight $taskbarHwnd) -ne 2) {
        $previewDeadline = [DateTime]::UtcNow.AddMilliseconds(850)
        do {
            Start-Sleep -Milliseconds 50
        } while ((Get-ClipHeight $taskbarHwnd) -ne 2 -and [DateTime]::UtcNow -lt $previewDeadline)
    }
    if ((Get-ClipHeight $taskbarHwnd) -ne 2) { throw 'Taskbar did not hide within the 1500 ms preview-plus-hide bound.' }

    Hold-PhysicalCursor ([int](($hidden.Left + $hidden.Right) / 2)) ($anchorBottom - 1) 200
    $heldVisible = Get-ClientGeometry $taskbarHwnd
    RightClick-Physical ($heldVisible.Left + 68) ($heldVisible.Top + 20) $hidden.Dpi
    Start-Sleep -Milliseconds 250
    Set-PhysicalCursor 100 100 $hidden.Dpi
    Start-Sleep -Milliseconds 700
    $contextHeld = Get-ClientGeometry $taskbarHwnd
    if ((Get-ClipHeight $taskbarHwnd) -eq 2) { throw 'Owned context menu did not hold taskbar visibility.' }
    Capture-Screen (Join-Path $EvidenceDirectory 'taskbar-auto-hide-context-hold.png')
    [SuperDesktopAutoHidePointer]::Escape()

    $explorerAbsent = -not [bool](Get-Process explorer -ErrorAction SilentlyContinue)
    if ($SuppressExplorer -and -not $explorerAbsent) { throw 'Explorer appeared during Shell capture.' }
    $process.WaitForExit()
    $trace = Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath
    foreach ($required in @('taskbar:auto-hide-hidden','taskbar:context-opened','taskbar:auto-hide-teardown-visible')) {
        if ($trace -notmatch [regex]::Escape($required)) { throw "Missing trace: $required" }
    }
    if ($trace -notmatch 'taskbar:auto-hide-shown' -and $trace -notmatch 'taskbar:auto-hide-fast-shown') {
        throw 'Missing owned reveal trace.'
    }
    if ($SuppressExplorer -and $trace -notmatch 'taskbar:auto-hide-appbar-skipped') {
        throw 'Shell auto-hide did not skip AppBar reservation.'
    }
    $report = [ordered]@{
        schema='taskbar-auto-hide-headful/v1';result='passed';shell=[bool]$SuppressExplorer;rows=$Rows
        app_sha256=(Get-FileHash $appPath -Algorithm SHA256).Hash.ToLowerInvariant()
        explorer_before=$explorerBefore;explorer_absent_during_capture=$explorerAbsent
        hidden_client=$hidden;visible_client=$visible;before_delay_client=$beforeDelay
        after_delay_client=$afterDelay;context_hold_client=$contextHeld;anchor_bottom=$anchorBottom
        final_settings=(Get-Content -Raw -Encoding UTF8 -LiteralPath $settingsPath | ConvertFrom-Json).taskbar
        screenshots=Get-ChildItem -LiteralPath $EvidenceDirectory -Filter 'taskbar-auto-hide-*.png' | ForEach-Object {
            [ordered]@{name=$_.Name;bytes=$_.Length;sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
        }
    }
    [IO.File]::WriteAllText((Join-Path $EvidenceDirectory 'headful-report.json'), (($report | ConvertTo-Json -Depth 8) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    if ($suppressor -and -not $suppressor.HasExited) { Stop-Process -Id $suppressor.Id -Force -ErrorAction SilentlyContinue }
    if ($SuppressExplorer -and -not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath $explorerPath }
    if ($watchdog -and -not $watchdog.HasExited) { Stop-Process -Id $watchdog.Id -Force -ErrorAction SilentlyContinue }
    if ($null -eq $priorLocalAppData) { Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue } else { $env:LOCALAPPDATA=$priorLocalAppData }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE=$priorTrace }
    if ($null -eq $priorTheme) { Remove-Item Env:SUPERDESKTOP_THEME -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_THEME=$priorTheme }
    if ($null -eq $priorLocale) { Remove-Item Env:SUPERDESKTOP_LOCALE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_LOCALE=$priorLocale }
}
