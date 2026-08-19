param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [switch]$Locked,
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
$lockedJson = if ($Locked) { 'true' } else { 'false' }
$settingsJson = ('{{"schema_version":1,"revision":0,"taskbar":{{"rows":2,"locked":{0},"pins":[]}}}}' -f $lockedJson)
[IO.File]::WriteAllText($settingsPath, $settingsJson, [Text.UTF8Encoding]::new($false))
$tracePath = Join-Path $EvidenceDirectory 'taskbar-resize-lock.log'
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SuperDesktopTaskbarPointer {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
    [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtrW(IntPtr hwnd, int index);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    public static void RightClick(int x, int y) { SetCursorPos(x,y); mouse_event(0x0008,0,0,0,UIntPtr.Zero); mouse_event(0x0010,0,0,0,UIntPtr.Zero); }
    public static void LeftClick(int x, int y) { SetCursorPos(x,y); mouse_event(0x0002,0,0,0,UIntPtr.Zero); mouse_event(0x0004,0,0,0,UIntPtr.Zero); }
    public static void Enter() { keybd_event(0x0D,0,0,UIntPtr.Zero); keybd_event(0x0D,0,2,UIntPtr.Zero); }
    public static void Down() { keybd_event(0x28,0,0,UIntPtr.Zero); keybd_event(0x28,0,2,UIntPtr.Zero); }
    public static void Space() { keybd_event(0x20,0,0,UIntPtr.Zero); keybd_event(0x20,0,2,UIntPtr.Zero); }
    public static void Drag(int x, int fromY, int toY) {
        SetCursorPos(x,fromY); mouse_event(0x0002,0,0,0,UIntPtr.Zero);
        System.Threading.Thread.Sleep(150); SetCursorPos(x,toY);
        System.Threading.Thread.Sleep(250); mouse_event(0x0004,0,0,0,UIntPtr.Zero);
    }
}
'@

function Get-Rect([IntPtr]$Hwnd) {
    $rect = [SuperDesktopTaskbarPointer+Rect]::new()
    if (-not [SuperDesktopTaskbarPointer]::GetWindowRect($Hwnd, [ref]$rect)) {
        throw 'GetWindowRect failed.'
    }
    [pscustomobject]@{left=$rect.Left;top=$rect.Top;right=$rect.Right;bottom=$rect.Bottom;width=$rect.Right-$rect.Left;height=$rect.Bottom-$rect.Top}
}

function Get-Style([IntPtr]$Hwnd) {
    [SuperDesktopTaskbarPointer]::GetWindowLongPtrW($Hwnd, -16).ToInt64()
}

function Get-ClientTop([IntPtr]$Hwnd) {
    $point = [SuperDesktopTaskbarPointer+Point]::new()
    if (-not [SuperDesktopTaskbarPointer]::ClientToScreen($Hwnd, [ref]$point)) {
        throw 'ClientToScreen failed.'
    }
    $point.Y
}

function Get-ClientGeometry([IntPtr]$Hwnd) {
    $rect = [SuperDesktopTaskbarPointer+Rect]::new()
    $point = [SuperDesktopTaskbarPointer+Point]::new()
    if (
        -not [SuperDesktopTaskbarPointer]::GetClientRect($Hwnd, [ref]$rect) -or
        -not [SuperDesktopTaskbarPointer]::ClientToScreen($Hwnd, [ref]$point)
    ) {
        throw 'Client geometry query failed.'
    }
    [pscustomobject]@{left=$point.X;top=$point.Y;width=$rect.Right-$rect.Left;height=$rect.Bottom-$rect.Top;right=$point.X+$rect.Right-$rect.Left;bottom=$point.Y+$rect.Bottom-$rect.Top}
}

function Find-TaskbarHwnd([int]$ProcessId) {
    $deadline = [DateTime]::UtcNow.AddSeconds(6)
    do {
        $items = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($index = 0; $index -lt $items.Count; $index++) {
            try {
                $item = $items.Item($index)
                if ($item.Current.ProcessId -ne $ProcessId -or $item.Current.Name -ne 'SuperTaskbar') {
                    continue
                }
                $owner = $item
                while ($null -ne $owner -and [int]$owner.Current.NativeWindowHandle -eq 0) {
                    $owner = [System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($owner)
                }
                if ($null -ne $owner) {
                    return [IntPtr][int]$owner.Current.NativeWindowHandle
                }
            } catch [System.Windows.Automation.ElementNotAvailableException] {
                continue
            }
        }
        $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($index = 0; $index -lt $windows.Count; $index++) {
            try {
                $window = $windows.Item($index)
                $bounds = $window.Current.BoundingRectangle
                if (
                    $window.Current.ProcessId -eq $ProcessId -and
                    $bounds.Width -gt 500 -and
                    $bounds.Height -gt 30 -and
                    $bounds.Height -lt 400
                ) {
                    return [IntPtr][int]$window.Current.NativeWindowHandle
                }
            } catch [System.Windows.Automation.ElementNotAvailableException] {
                continue
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    return [IntPtr]::Zero
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

function Find-LockMenuItem([int]$ProcessId, [IntPtr]$TaskbarHwnd) {
    $script:lastPopupNames = @()
    $script:lastPopupWindows = @()
    $deadline = [DateTime]::UtcNow.AddSeconds(4)
    do {
        $rootItems = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($rootIndex = 0; $rootIndex -lt $rootItems.Count; $rootIndex++) {
            try {
                $item = $rootItems.Item($rootIndex)
                if (
                    $item.Current.ProcessId -eq $ProcessId -and
                    (
                        $item.Current.Name.EndsWith(', checked') -or
                        $item.Current.Name.EndsWith(', not checked')
                    )
                ) {
                    $owner = $item
                    while ($null -ne $owner -and [int]$owner.Current.NativeWindowHandle -eq 0) {
                        $owner = [System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($owner)
                    }
                    if ($null -ne $owner) {
                        [SuperDesktopTaskbarPointer]::SetForegroundWindow(
                            [IntPtr][int]$owner.Current.NativeWindowHandle
                        ) | Out-Null
                        Start-Sleep -Milliseconds 100
                    }
                    $item.SetFocus()
                    Start-Sleep -Milliseconds 75
                    return $item
                }
            } catch [System.Windows.Automation.ElementNotAvailableException] {
                continue
            }
        }
        $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($windowIndex = 0; $windowIndex -lt $windows.Count; $windowIndex++) {
            $window = $windows.Item($windowIndex)
            try {
                if ($window.Current.ProcessId -ne $ProcessId) { continue }
                $native = $window.Current.NativeWindowHandle
                if ($null -eq $native) { continue }
                $hwnd = [IntPtr][int]$native
                if ($hwnd -eq $TaskbarHwnd) { continue }
                $bounds = $window.Current.BoundingRectangle
                $script:lastPopupWindows += "hwnd=$($hwnd.ToInt64()) $([int]$bounds.Width)x$([int]$bounds.Height)"
                if ($bounds.Width -gt 700 -or $bounds.Height -gt 500) { continue }
                [SuperDesktopTaskbarPointer]::SetForegroundWindow($hwnd) | Out-Null
                Start-Sleep -Milliseconds 75
                $items = $window.FindAll(
                    [System.Windows.Automation.TreeScope]::Descendants,
                    [System.Windows.Automation.Condition]::TrueCondition
                )
                for ($index = 0; $index -lt $items.Count; $index++) {
                    $item = $items.Item($index)
                    if ($item.Current.Name) { $script:lastPopupNames += [string]$item.Current.Name }
                    if (
                        $item.Current.Name.EndsWith(', checked') -or
                        $item.Current.Name.EndsWith(', not checked')
                    ) {
                        return $item
                    }
                }
            } catch [System.Windows.Automation.ElementNotAvailableException] {
                continue
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    return $null
}

function Open-LockMenuItem([IntPtr]$TaskbarHwnd, [int]$ProcessId) {
    $rect = Get-Rect $TaskbarHwnd
    [SuperDesktopTaskbarPointer]::SetForegroundWindow($TaskbarHwnd) | Out-Null
    [SuperDesktopTaskbarPointer]::RightClick($rect.Left + 68, $rect.Top + [Math]::Min(20, [Math]::Max(4, $rect.Height - 4)))
    $item = Find-LockMenuItem $ProcessId $TaskbarHwnd
    if ($null -eq $item) { throw "Owned Lock the taskbar menu item did not appear. Windows: $($script:lastPopupWindows -join ' | ') UIA: $($script:lastPopupNames -join ' | ')" }
    return $item
}

function Click-Element([System.Windows.Automation.AutomationElement]$Element) {
    $bounds = $Element.Current.BoundingRectangle
    if ($bounds.Width -le 0 -or $bounds.Height -le 0) { throw 'UIA element has empty bounds.' }
    [SuperDesktopTaskbarPointer]::LeftClick(
        [int]($bounds.Left + $bounds.Width / 2),
        [int]($bounds.Top + $bounds.Height / 2)
    )
}

$priorLocalAppData = $env:LOCALAPPDATA
$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorTheme = $env:SUPERDESKTOP_THEME
$priorLocale = $env:SUPERDESKTOP_LOCALE
$env:LOCALAPPDATA = $profileRoot
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
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
            '$deadline=[DateTime]::UtcNow.AddSeconds(27); while([DateTime]::UtcNow -lt $deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 10}'
        )
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        do { Start-Sleep -Milliseconds 100 } while ((Get-Process explorer -ErrorAction SilentlyContinue) -and [DateTime]::UtcNow -lt $deadline)
        if (Get-Process explorer -ErrorAction SilentlyContinue) { throw 'Explorer suppression failed.' }
    }
    Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
    $arguments = @('--verification-capture-ms','20000')
    if ($SuppressExplorer) { $arguments += '--shell' }
    $process = Start-Process -FilePath $appPath -ArgumentList $arguments -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(6)
    do { Start-Sleep -Milliseconds 100; $process.Refresh() } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and -not $process.HasExited -and [DateTime]::UtcNow -lt $deadline)
    if ($process.HasExited -or $process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'SuperDesktop taskbar HWND did not appear.' }
    $taskbarHwnd = Find-TaskbarHwnd $process.Id
    if ($taskbarHwnd -eq [IntPtr]::Zero) { throw 'Owned SuperTaskbar UIA root did not resolve to an HWND.' }
    $initialRect = Get-Rect $taskbarHwnd
    $initialClient = Get-ClientGeometry $taskbarHwnd
    $initialStyle = Get-Style $taskbarHwnd
    $hasThickFrame = ($initialStyle -band 0x00040000) -ne 0
    if ($Locked -and $hasThickFrame) { throw 'Locked taskbar unexpectedly has WS_THICKFRAME.' }
    if (-not $Locked -and -not $hasThickFrame) { throw 'Unlocked taskbar is missing WS_THICKFRAME.' }

    $threeRows = $null
    $oneRow = $null
    $afterLockedDrag = $null
    if ($Locked) {
        $clientTop = Get-ClientTop $taskbarHwnd
        [SuperDesktopTaskbarPointer]::Drag([int](($initialRect.Left + $initialRect.Right) / 2), $clientTop + 1, $clientTop - 100)
        Start-Sleep -Milliseconds 600
        $afterLockedDrag = Get-Rect $taskbarHwnd
        $lockedClient = Get-ClientGeometry $taskbarHwnd
        if ($lockedClient.Height -ne $initialClient.Height -or $lockedClient.Bottom -ne $initialClient.Bottom) {
            throw 'Locked taskbar changed geometry.'
        }
        Capture-Screen (Join-Path $EvidenceDirectory 'taskbar-locked-two-rows.png')
    } else {
        $clientTop = Get-ClientTop $taskbarHwnd
        [SuperDesktopTaskbarPointer]::Drag([int](($initialRect.Left + $initialRect.Right) / 2), $clientTop + 1, $clientTop - 100)
        Start-Sleep -Milliseconds 900
        $threeRows = Get-Rect $taskbarHwnd
        $threeRowsClient = Get-ClientGeometry $taskbarHwnd
        if ($threeRowsClient.Height -lt 200 -or $threeRowsClient.Height -gt 220 -or $threeRowsClient.Bottom -ne $initialClient.Bottom) {
            throw "Three-row resize did not snap or preserve bottom: outer=$($threeRows | ConvertTo-Json -Compress) client=$($threeRowsClient | ConvertTo-Json -Compress) initial=$($initialClient | ConvertTo-Json -Compress)"
        }
        Capture-Screen (Join-Path $EvidenceDirectory 'taskbar-three-rows.png')
        $clientTop = Get-ClientTop $taskbarHwnd
        [SuperDesktopTaskbarPointer]::Drag([int](($threeRows.Left + $threeRows.Right) / 2), $clientTop + 1, $clientTop + 150)
        Start-Sleep -Milliseconds 900
        $oneRow = Get-Rect $taskbarHwnd
        $oneRowClient = Get-ClientGeometry $taskbarHwnd
        if ($oneRowClient.Height -lt 60 -or $oneRowClient.Height -gt 80 -or $oneRowClient.Bottom -ne $initialClient.Bottom) {
            throw "One-row resize did not snap or preserve bottom: $($oneRow | ConvertTo-Json -Compress)"
        }
        Capture-Screen (Join-Path $EvidenceDirectory 'taskbar-one-row.png')
    }

    $lockItem = Open-LockMenuItem $taskbarHwnd $process.Id
    $menuOwner = $lockItem
    while ($null -ne $menuOwner -and [int]$menuOwner.Current.NativeWindowHandle -eq 0) {
        $menuOwner = [System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($menuOwner)
    }
    if ($null -eq $menuOwner) { throw 'Owned taskbar context menu HWND was not found.' }
    $menuHwnd = [IntPtr][int]$menuOwner.Current.NativeWindowHandle
    $menuBounds = $menuOwner.Current.BoundingRectangle
    $menuDpi = [SuperDesktopTaskbarPointer]::GetDpiForWindow($menuHwnd)
    $menuScale = [double]$menuDpi / 96.0
    $menuItemCondition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::MenuItem
    )
    $menuElements = $menuOwner.FindAll([System.Windows.Automation.TreeScope]::Descendants, $menuItemCondition)
    $menuItems = @()
    for ($menuIndex = 0; $menuIndex -lt $menuElements.Count; $menuIndex++) {
        $menuElement = $menuElements.Item($menuIndex)
        $bounds = $menuElement.Current.BoundingRectangle
        $menuItems += [ordered]@{order=$menuIndex;name=[string]$menuElement.Current.Name;left=[int]$bounds.Left;top=[int]$bounds.Top;width=[int]$bounds.Width;height=[int]$bounds.Height}
    }
    $expectedMenuNames = @(
        'Search: Hidden',
        'Show Task View button, checked',
        'Show the desktop',
        'Task Manager',
        ('Lock the taskbar, ' + $(if ($Locked) { 'checked' } else { 'not checked' })),
        'Taskbar settings'
    )
    if ($menuItems.Count -ne $expectedMenuNames.Count) { throw "Expected six context commands, found $($menuItems.Count): $($menuItems.name -join ' | ')" }
    for ($menuIndex = 0; $menuIndex -lt $expectedMenuNames.Count; $menuIndex++) {
        if ($menuItems[$menuIndex].name -ne $expectedMenuNames[$menuIndex]) { throw "Context command order mismatch at ${menuIndex}: $($menuItems[$menuIndex].name)" }
        if ($menuItems[$menuIndex].width -le 0 -or $menuItems[$menuIndex].height -le 0) { throw "Context command has empty bounds at $menuIndex" }
        if ($menuIndex -gt 0 -and ($menuItems[$menuIndex].top -le $menuItems[$menuIndex-1].top -or $menuItems[$menuIndex-1].top + $menuItems[$menuIndex-1].height -gt $menuItems[$menuIndex].top)) { throw "Context command rows overlap or are out of visual order at $menuIndex" }
        if ($menuItems[$menuIndex].left -lt $menuBounds.Left -or $menuItems[$menuIndex].top -lt $menuBounds.Top -or $menuItems[$menuIndex].left + $menuItems[$menuIndex].width -gt $menuBounds.Right -or $menuItems[$menuIndex].top + $menuItems[$menuIndex].height -gt $menuBounds.Bottom) { throw "Context command is clipped at $menuIndex" }
    }
    $menuWidthDip = [double]$menuBounds.Width / $menuScale
    $menuHeightDip = [double]$menuBounds.Height / $menuScale
    if ($menuWidthDip -lt 200 -or $menuWidthDip -gt 240 -or $menuHeightDip -lt 195 -or $menuHeightDip -gt 225) { throw "Context popup geometry rejected: ${menuWidthDip}x${menuHeightDip} DIP" }
    $virtual = [System.Windows.Forms.SystemInformation]::VirtualScreen
    foreach ($menuItem in $menuItems) {
        if ($menuItem.left -lt $virtual.Left -or $menuItem.top -lt $virtual.Top -or $menuItem.left + $menuItem.width -gt $virtual.Right -or $menuItem.top + $menuItem.height -gt $virtual.Bottom) { throw "Interactive context row left the virtual monitor bounds: $($menuItem.name)" }
    }
    $shadowOverflow = [Math]::Max([Math]::Max($virtual.Left - $menuBounds.Left, $virtual.Top - $menuBounds.Top), [Math]::Max($menuBounds.Right - $virtual.Right, $menuBounds.Bottom - $virtual.Bottom))
    if ($shadowOverflow -gt 16) { throw "Context popup non-client overflow exceeds DWM shadow tolerance: $shadowOverflow px" }
    Capture-Screen (Join-Path $EvidenceDirectory 'taskbar-lock-context.png')
    $lockItem.SetFocus()
    for ($downIndex = 0; $downIndex -lt 4; $downIndex++) { [SuperDesktopTaskbarPointer]::Down(); Start-Sleep -Milliseconds 50 }
    [SuperDesktopTaskbarPointer]::Enter()
    Start-Sleep -Milliseconds 500

    $settings = Get-Content -Raw -Encoding UTF8 -LiteralPath $settingsPath | ConvertFrom-Json
    $expectedRows = if ($Locked) { 2 } else { 1 }
    if ($settings.taskbar.rows -ne $expectedRows -or [bool]$settings.taskbar.locked -eq [bool]$Locked) {
        throw 'Final persisted rows/lock state is incorrect.'
    }
    $process.WaitForExit()
    $trace = Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath
    $requiredTraces = @('taskbar:context-opened','taskbar:lock-toggled')
    if (-not $Locked) { $requiredTraces += @('taskbar:resize-saved','taskbar:resize-applied') }
    foreach ($required in $requiredTraces) {
        if ($trace -notmatch [regex]::Escape($required)) { throw "Missing trace: $required" }
    }
    if (
        $SuppressExplorer -and
        $trace -notmatch 'taskbar:resize-appbar-synced' -and
        $trace -notmatch 'taskbar:resize-owned-workarea-synced'
    ) {
        throw 'Shell resize did not synchronize AppBar or the explicit owned-workarea fallback.'
    }
    $explorerAbsent = -not [bool](Get-Process explorer -ErrorAction SilentlyContinue)
    if ($SuppressExplorer -and -not $explorerAbsent) { throw 'Explorer appeared during Shell capture.' }
    $report = [ordered]@{
        schema='taskbar-resize-lock-headful/v1'
        result='passed'
        shell=[bool]$SuppressExplorer
        locked=[bool]$Locked
        app_sha256=(Get-FileHash $appPath -Algorithm SHA256).Hash.ToLowerInvariant()
        explorer_before=$explorerBefore
        explorer_absent_during_capture=$explorerAbsent
        appbar_disposition=if(-not $SuppressExplorer){'not-applicable-preview'}elseif($trace -match 'taskbar:resize-appbar-synced'){'registered'}else{'unavailable-owned-shell'}
        initial_rect=$initialRect
        initial_client=$initialClient
        three_rows_rect=$threeRows
        three_rows_client=$threeRowsClient
        one_row_rect=$oneRow
        one_row_client=$oneRowClient
        locked_drag_rect=$afterLockedDrag
        initial_style=$initialStyle
        context_menu=[ordered]@{width_dip=$menuWidthDip;height_dip=$menuHeightDip;dpi=$menuDpi;interactive_content_contained=$true;non_client_shadow_overflow_px=$shadowOverflow;items=$menuItems}
        final_settings=@{rows=$settings.taskbar.rows;locked=$settings.taskbar.locked;lock_action_persisted=$true}
        screenshots=Get-ChildItem -LiteralPath $EvidenceDirectory -Filter 'taskbar-*.png' | ForEach-Object { [ordered]@{name=$_.Name;bytes=$_.Length;sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()} }
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
