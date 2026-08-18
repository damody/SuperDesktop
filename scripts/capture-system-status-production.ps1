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
    public static extern bool SetForegroundWindow(IntPtr hwnd);
}
'@

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

try {
    if ($SuppressExplorer) {
        if (-not (Test-Path -LiteralPath $explorerPath -PathType Leaf)) {
            throw "Missing Explorer recovery binary: $explorerPath"
        }
        $watchdog = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile','-WindowStyle','Hidden','-Command',
            "Start-Sleep -Seconds 40; if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath '$explorerPath' }"
        )
        $suppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile','-WindowStyle','Hidden','-Command',
            '$deadline=[DateTime]::UtcNow.AddSeconds(27); while([DateTime]::UtcNow -lt $deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 10}'
        )
        $suppressionDeadline = [DateTime]::UtcNow.AddSeconds(5)
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
    $start = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name -eq 'Start' }
    $startMissing = -not $SkipStartFocusVerification -and $null -eq $start
    if ($null -eq $input -or $null -eq $network -or $null -eq $volume -or $null -eq $calendar -or $startMissing) {
        $names = $taskbar.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        ) | ForEach-Object { [string]$_.Current.Name } | Where-Object { $_ }
        throw "One or more owned taskbar status controls are missing. Visible names: $($names -join ' | ')"
    }
    $originalLanguage = ([string]$input.Current.Name).Substring('Input language '.Length)

    Invoke-Element $network
    $networkDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $networkDialog) { throw 'Owned network and power flyout did not appear.' }
    Capture-Screen (Join-Path $EvidenceDirectory 'network-power-flyout.png')

    Invoke-Element $input
    $inputDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $inputDialog) { throw 'Owned input flyout did not appear.' }
    Capture-Screen (Join-Path $EvidenceDirectory 'input-flyout.png')

    Invoke-Element $volume
    $volumeDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $volumeDialog) { throw 'Owned volume flyout did not replace the input flyout.' }
    $slider = Find-Element $volumeDialog { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Slider }
    if ($null -eq $slider) { throw 'Owned volume slider is missing.' }
    Capture-Screen (Join-Path $EvidenceDirectory 'volume-flyout.png')

    Invoke-Element $calendar
    $calendarDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    if ($null -eq $calendarDialog) { throw 'Owned calendar flyout did not replace the volume flyout.' }
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

    $inputDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window } 500
    if ($null -eq $inputDialog) {
        $input = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith('Input language ') }
        Invoke-Element $input
        $inputDialog = Find-OwnedPopupElement $process.Id $process.MainWindowHandle { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window }
    }
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
        schema='system-status-headful/v1'
        result='passed'
        app_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $appPath).Hash.ToLowerInvariant()
        original_input_profile=$originalLanguage
        switched_input_profile=$alternateName
        original_profile_restored=$true
        start_survived_switch=if($SkipStartFocusVerification){$null}else{$true}
        start_focus_restored_trace=if($SkipStartFocusVerification){$null}else{$true}
        owned_flyouts=@('network-power','input','volume','calendar')
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
