param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [switch]$SkipStartFocusVerification,
    [switch]$SuppressExplorer
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$tracePath = Join-Path $EvidenceDirectory 'system-status-headful.log'
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SuperDesktopFlyoutFocus {
    [DllImport("user32.dll")]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    public static extern uint GetDpiForWindow(IntPtr hwnd);
}
'@
[SuperDesktopFlyoutFocus]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

function Find-Element {
    param(
        [System.Windows.Automation.AutomationElement]$Root,
        [scriptblock]$Predicate,
        [int]$TimeoutMilliseconds = 3000
    )
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        try {
            $all = $Root.FindAll(
                [System.Windows.Automation.TreeScope]::Descendants,
                [System.Windows.Automation.Condition]::TrueCondition
            )
        } catch [System.Windows.Automation.ElementNotAvailableException] {
            Start-Sleep -Milliseconds 100
            continue
        }
        for ($index = 0; $index -lt $all.Count; $index++) {
            $candidate = $all.Item($index)
            try {
                if (& $Predicate $candidate) { return $candidate }
            } catch [System.Windows.Automation.ElementNotAvailableException] {
                continue
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    return $null
}

function Invoke-Element([System.Windows.Automation.AutomationElement]$Element) {
    if ($null -eq $Element) { throw 'Required UI Automation element was not found.' }
    $pattern = $Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
}

function Find-OwnedPopupElement {
    param(
        [int]$ProcessId,
        [IntPtr]$TaskbarHwnd,
        [scriptblock]$Predicate,
        [int]$TimeoutMilliseconds = 3000
    )
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        $ownedWindows = @()
        $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($windowIndex = 0; $windowIndex -lt $windows.Count; $windowIndex++) {
            $candidateWindow = $windows.Item($windowIndex)
            try {
                if ($candidateWindow.Current.ProcessId -ne $ProcessId) {
                    continue
                }
                $nativeHandle = $candidateWindow.Current.NativeWindowHandle
                if ($null -eq $nativeHandle) {
                    continue
                }
                $hwnd = [IntPtr][int]$nativeHandle
                if ($hwnd -eq $TaskbarHwnd) {
                    continue
                }
                $bounds = $candidateWindow.Current.BoundingRectangle
                $virtualScreen = [System.Windows.Forms.SystemInformation]::VirtualScreen
                if (
                    $bounds.Width -le 0 -or
                    $bounds.Height -le 0 -or
                    (
                        $bounds.Width -ge $virtualScreen.Width * 0.75 -and
                        $bounds.Height -ge $virtualScreen.Height * 0.75
                    )
                ) {
                    continue
                }
                $ownedWindows += [pscustomobject]@{ Element=$candidateWindow; Hwnd=$hwnd }
                $all = $candidateWindow.FindAll(
                    [System.Windows.Automation.TreeScope]::Descendants,
                    [System.Windows.Automation.Condition]::TrueCondition
                )
                for ($index = 0; $index -lt $all.Count; $index++) {
                    $candidate = $all.Item($index)
                    if (& $Predicate $candidate) { return $candidate }
                }
            } catch [System.Windows.Automation.ElementNotAvailableException] {
                continue
            }
        }
        foreach ($ownedWindow in $ownedWindows) {
            try {
                [SuperDesktopFlyoutFocus]::SetForegroundWindow($ownedWindow.Hwnd) | Out-Null
                Start-Sleep -Milliseconds 75
                $all = $ownedWindow.Element.FindAll(
                    [System.Windows.Automation.TreeScope]::Descendants,
                    [System.Windows.Automation.Condition]::TrueCondition
                )
                for ($index = 0; $index -lt $all.Count; $index++) {
                    $candidate = $all.Item($index)
                    if (& $Predicate $candidate) { return $candidate }
                }
            } catch [System.Windows.Automation.ElementNotAvailableException] {
                continue
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    return $null
}

function Get-OwnedPopupWindows([int]$ProcessId, [IntPtr]$TaskbarHwnd) {
    $result = @()
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    for ($index = 0; $index -lt $windows.Count; $index++) {
        $candidate = $windows.Item($index)
        try {
            if ($candidate.Current.ProcessId -ne $ProcessId) { continue }
            if ($candidate.Current.IsOffscreen) { continue }
            $handle = [IntPtr][int]$candidate.Current.NativeWindowHandle
            if ($handle -eq [IntPtr]::Zero -or $handle -eq $TaskbarHwnd) { continue }
            $bounds = $candidate.Current.BoundingRectangle
            if ($bounds.Width -gt 100 -and $bounds.Height -gt 100) {
                $result += $candidate
            }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
    }
    @($result)
}

function Measure-OwnedFlyout(
    [string]$Kind,
    [int]$ProcessId,
    [System.Windows.Automation.AutomationElement]$Taskbar,
    [IntPtr]$TaskbarHwnd,
    [double]$ExpectedWidthDip
) {
    $deadline = [DateTime]::UtcNow.AddSeconds(3)
    do {
        $candidates = @(Get-OwnedPopupWindows $ProcessId $TaskbarHwnd)
        $maximumRight = @($candidates | ForEach-Object { $_.Current.BoundingRectangle.Right } | Measure-Object -Maximum).Maximum
        $windows = if ($null -eq $maximumRight) { @() } else { @($candidates | Where-Object { $maximumRight - $_.Current.BoundingRectangle.Right -le 64.0 }) }
        if ($windows.Count -eq 1) { break }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($windows.Count -ne 1) {
        $details = @($windows | ForEach-Object {
            $bounds = $_.Current.BoundingRectangle
            "hwnd=$($_.Current.NativeWindowHandle),name=$($_.Current.Name),bounds=$($bounds.Left),$($bounds.Top),$($bounds.Right),$($bounds.Bottom)"
        })
        throw "$Kind owned popup count=$($windows.Count), expected 1: $($details -join '; ')"
    }
    $popup = $windows[0]
    $popupBounds = $popup.Current.BoundingRectangle
    $taskbarBounds = $Taskbar.Current.BoundingRectangle
    $popupHwnd = [IntPtr][int]$popup.Current.NativeWindowHandle
    $dpi = [SuperDesktopFlyoutFocus]::GetDpiForWindow($popupHwnd)
    if ($dpi -eq 0) { throw "$Kind popup DPI unavailable" }
    $scale = [double]$dpi / 96.0
    $widthDip = [double]$popupBounds.Width / $scale
    $heightDip = [double]$popupBounds.Height / $scale
    $gapDip = ([double]$taskbarBounds.Top - [double]$popupBounds.Bottom) / $scale
    $center = [Drawing.Point]::new(
        [int]($popupBounds.Left + $popupBounds.Width / 2),
        [int]($popupBounds.Top + $popupBounds.Height / 2)
    )
    $monitor = [Windows.Forms.Screen]::FromPoint($center).Bounds
    $contained = (
        $popupBounds.Left -ge $monitor.Left -and
        $popupBounds.Top -ge $monitor.Top -and
        $popupBounds.Right -le $monitor.Right -and
        $popupBounds.Bottom -le $monitor.Bottom
    )
    if (-not $contained) { throw "$Kind popup is outside its monitor" }
    if ([Math]::Abs($widthDip - $ExpectedWidthDip) -gt 16.0) {
        throw "$Kind width=$widthDip DIP differs from expected=$ExpectedWidthDip DIP; hwnd=$popupHwnd name=$($popup.Current.Name) dpi=$dpi popup=$($popupBounds.Left),$($popupBounds.Top),$($popupBounds.Right),$($popupBounds.Bottom) taskbar=$($taskbarBounds.Left),$($taskbarBounds.Top),$($taskbarBounds.Right),$($taskbarBounds.Bottom) gapDip=$gapDip"
    }
    if ($gapDip -lt 2.0 -or $gapDip -gt 16.0) {
        throw "$Kind taskbar gap=$gapDip DIP is outside 2..16 DIP"
    }
    [ordered]@{
        kind = $Kind
        hwnd = [int64]$popupHwnd
        owned_popup_count = $windows.Count
        dpi = $dpi
        scale = $scale
        expected_width_dip = $ExpectedWidthDip
        width_dip = $widthDip
        height_dip = $heightDip
        taskbar_gap_dip = $gapDip
        contained = $contained
        popup_bounds = [ordered]@{left=$popupBounds.Left;top=$popupBounds.Top;right=$popupBounds.Right;bottom=$popupBounds.Bottom}
        taskbar_bounds = [ordered]@{left=$taskbarBounds.Left;top=$taskbarBounds.Top;right=$taskbarBounds.Right;bottom=$taskbarBounds.Bottom}
        monitor_bounds = [ordered]@{left=$monitor.Left;top=$monitor.Top;right=$monitor.Right;bottom=$monitor.Bottom}
    }
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

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
$env:SUPERDESKTOP_ACTION_TRACE = $tracePath
Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
$watchdog = $null
$suppressor = $null
$explorerBefore = @(Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
$geometryRecords = @()

try {
    if ($SuppressExplorer) {
        if (-not (Test-Path -LiteralPath $explorerPath -PathType Leaf)) {
            throw "Missing Explorer recovery binary: $explorerPath"
        }
        $watchdog = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile','-WindowStyle','Hidden','-Command',
            "Start-Sleep -Seconds 50; if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath '$explorerPath' }"
        )
        $suppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile','-WindowStyle','Hidden','-Command',
            '$deadline=[DateTime]::UtcNow.AddSeconds(37); while([DateTime]::UtcNow -lt $deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 10}'
        )
        $suppressionDeadline = [DateTime]::UtcNow.AddSeconds(12)
        do {
            Start-Sleep -Milliseconds 100
        } while (
            (Get-Process explorer -ErrorAction SilentlyContinue) -and
            [DateTime]::UtcNow -lt $suppressionDeadline
        )
        if (Get-Process explorer -ErrorAction SilentlyContinue) {
            throw 'Explorer suppression did not reach an absent state.'
        }
    }
    $arguments = @('--verification-capture-ms','20000')
    if ($SuppressExplorer) { $arguments += '--shell' }
    $process = Start-Process -FilePath $appPath -ArgumentList $arguments -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 100
        $process.Refresh()
    } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'SuperDesktop taskbar HWND did not appear.' }
    $taskbar = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $button = [System.Windows.Automation.ControlType]::Button

    $input = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith('Input language ') }
    $network = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith('Network ') }
    $volume = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith('Volume ') }
    $calendar = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name -match '^\d{2}:\d{2} ' }
    $traditionalStart = -join @([char]0x958B, [char]0x59CB)
    $start = Find-Element $taskbar {
        param($item)
        $item.Current.ControlType -eq $button -and
            ($item.Current.Name -eq 'Start' -or $item.Current.Name -eq $traditionalStart)
    }
    $startMissing = -not $SkipStartFocusVerification -and $null -eq $start
    if ($null -eq $input -or $null -eq $network -or $null -eq $volume -or $null -eq $calendar -or $startMissing) {
        $missing = @(
            if ($null -eq $input) { 'input' }
            if ($null -eq $network) { 'network' }
            if ($null -eq $volume) { 'volume' }
            if ($null -eq $calendar) { 'calendar' }
            if ($startMissing) { 'start' }
        )
        $names = $taskbar.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        ) | ForEach-Object { [string]$_.Current.Name } | Where-Object { $_ }
        throw "Owned taskbar controls missing: $($missing -join ', '). Visible names: $($names -join ' | ')"
    }
    $originalLanguage = ([string]$input.Current.Name).Substring('Input language '.Length)

    Invoke-Element $network
    $networkDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $networkDialog) { throw 'Owned network and power flyout did not appear.' }
    $geometryRecords += Measure-OwnedFlyout 'network-power' $process.Id $taskbar $process.MainWindowHandle 360.0
    Capture-Screen (Join-Path $EvidenceDirectory 'network-power-flyout.png')

    Invoke-Element $input
    $inputDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $inputDialog) { throw 'Owned input flyout did not appear.' }
    $geometryRecords += Measure-OwnedFlyout 'input' $process.Id $taskbar $process.MainWindowHandle 360.0
    Capture-Screen (Join-Path $EvidenceDirectory 'input-flyout.png')

    Invoke-Element $volume
    $volumeDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $volumeDialog) { throw 'Owned volume flyout did not replace the input flyout.' }
    $slider = Find-Element $volumeDialog { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Slider }
    if ($null -eq $slider) { throw 'Owned volume slider is missing.' }
    $geometryRecords += Measure-OwnedFlyout 'volume' $process.Id $taskbar $process.MainWindowHandle 360.0
    Capture-Screen (Join-Path $EvidenceDirectory 'volume-flyout.png')

    Invoke-Element $calendar
    $calendarDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $calendarDialog) { throw 'Owned calendar flyout did not replace the volume flyout.' }
    $geometryRecords += Measure-OwnedFlyout 'calendar' $process.Id $taskbar $process.MainWindowHandle 380.0
    Capture-Screen (Join-Path $EvidenceDirectory 'calendar-flyout.png')

    if (-not $SkipStartFocusVerification) {
        Invoke-Element $start
        $startDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
        if ($null -eq $startDialog) { throw 'Owned Start did not appear before the input switch.' }
    }
    Invoke-Element $input
    $inputDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $inputDialog) { throw 'Owned input flyout did not open for the controlled switch.' }
    $buttonCondition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button
    )
    $profiles = $inputDialog.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        $buttonCondition
    )
    $alternate = $null
    $original = $null
    for ($index = 0; $index -lt $profiles.Count; $index++) {
        $profile = $profiles.Item($index)
        $name = [string]$profile.Current.Name
        if ($name.StartsWith($originalLanguage)) { $original = $profile }
        elseif ($null -eq $alternate) { $alternate = $profile }
    }
    if ($null -eq $alternate -or $null -eq $original) { throw 'Two real input profiles are required for the controlled switch.' }
    $alternateName = [string]$alternate.Current.Name
    Invoke-Element $alternate
    Start-Sleep -Milliseconds 1000
    if (-not $SkipStartFocusVerification) {
        $startAfterSwitch = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
        if ($null -eq $startAfterSwitch) { throw 'Owned Start was lost during the input profile switch.' }
        Capture-Screen (Join-Path $EvidenceDirectory 'start-after-input-switch.png')
    }

    [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    Start-Sleep -Milliseconds 150
    $input = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith('Input language ') }
    Invoke-Element $input
    $inputDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $inputDialog) { throw 'Owned input flyout did not remain available for profile restoration.' }
    $restore = Find-Element $inputDialog { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith($originalLanguage) }
    Invoke-Element $restore
    Start-Sleep -Milliseconds 750

    $process.WaitForExit()
    $explorerAbsentDuringCapture = -not [bool](Get-Process explorer -ErrorAction SilentlyContinue)
    if ($SuppressExplorer -and -not $explorerAbsentDuringCapture) {
        throw 'Explorer appeared during the owned system-flyout capture.'
    }
    $trace = Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath
    $missingStartFocus = -not $SkipStartFocusVerification -and $trace -notmatch 'start:ime-focus-restored'
    if ($trace -notmatch 'status:owned-flyout-opened' -or $missingStartFocus) {
        throw 'Headful trace does not prove owned flyout composition and Start focus restoration.'
    }
    $screenshots = Get-ChildItem -LiteralPath $EvidenceDirectory -Filter '*-flyout.png' | ForEach-Object {
        [ordered]@{ name=$_.Name;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant();bytes=$_.Length }
    }
    $report = [ordered]@{
        schema='system-status-headful/v2'
        result='passed'
        app_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $appPath).Hash.ToLowerInvariant()
        original_input_profile=$originalLanguage
        switched_input_profile=$alternateName
        original_profile_restored=$true
        start_survived_switch=if($SkipStartFocusVerification){$null}else{$true}
        start_focus_restored_trace=if($SkipStartFocusVerification){$null}else{$true}
        owned_flyouts=@('network-power','input','volume','calendar')
        geometry_thresholds=[ordered]@{width_tolerance_dip=16.0;taskbar_gap_min_dip=2.0;taskbar_gap_max_dip=16.0}
        geometry_records=$geometryRecords
        explorer_suppressed=[bool]$SuppressExplorer
        explorer_before=$explorerBefore
        explorer_absent_during_capture=$explorerAbsentDuringCapture
        screenshots=$screenshots
    }
    [IO.File]::WriteAllText(
        (Join-Path $EvidenceDirectory 'headful-report.json'),
        (($report | ConvertTo-Json -Depth 8) + "`n"),
        [Text.UTF8Encoding]::new($false)
    )
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($null -ne $process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force }
    if ($suppressor -and -not $suppressor.HasExited) { Stop-Process -Id $suppressor.Id -Force -ErrorAction SilentlyContinue }
    if ($SuppressExplorer -and -not (Get-Process explorer -ErrorAction SilentlyContinue)) {
        Start-Process -FilePath $explorerPath
    }
    if ($watchdog -and -not $watchdog.HasExited) { Stop-Process -Id $watchdog.Id -Force -ErrorAction SilentlyContinue }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE = $priorTrace }
}
