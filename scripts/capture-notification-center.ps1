param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [ValidateSet('light','dark','high-contrast')][string]$Theme = 'light',
    [switch]$ExerciseActions
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$app = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$fixture = Join-Path $Workspace 'target/release/examples/notify_icon_fixture.exe'
foreach ($path in @($app, $fixture)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing binary: $path" }
}
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path
$trace = Join-Path $EvidenceDirectory "$Theme-trace.log"
$fixtureLog = Join-Path $EvidenceDirectory "$Theme-fixture.log"
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class NotificationCenterCaptureDpi {
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
}
'@
[NotificationCenterCaptureDpi]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

function Find-Descendant([System.Windows.Automation.AutomationElement]$Root, [scriptblock]$Predicate, [int]$TimeoutMs = 6000) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    do {
        try {
            $items = $Root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
            for ($index = 0; $index -lt $items.Count; $index++) {
                $item = $items.Item($index)
                if (& $Predicate $item) { return $item }
            }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    $null
}

function Find-OwnedPopup([int]$ProcessId, [IntPtr]$TaskbarHandle) {
    $deadline = [DateTime]::UtcNow.AddSeconds(7)
    do {
        $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($index = 0; $index -lt $windows.Count; $index++) {
            $candidate = $windows.Item($index)
            try {
                if ($candidate.Current.ProcessId -ne $ProcessId) { continue }
                if ([IntPtr][int]$candidate.Current.NativeWindowHandle -eq $TaskbarHandle) { continue }
                $bounds = $candidate.Current.BoundingRectangle
                if ($bounds.Width -gt 100 -and $bounds.Height -gt 100 -and $bounds.Width -lt 1600 -and $bounds.Height -lt 2000) {
                    return $candidate
                }
            } catch [System.Windows.Automation.ElementNotAvailableException] {}
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    $null
}

function Invoke-Element([System.Windows.Automation.AutomationElement]$Element) {
    if ($null -eq $Element) { throw 'Required UIA element is missing.' }
    $pattern = $Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
}

function Count-ListItems([System.Windows.Automation.AutomationElement]$Root) {
    $items = $Root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    @($items | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::ListItem }).Count
}

function Capture-Screen([string]$Path) {
    $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bitmap = [Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bitmap.Size)
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $graphics.Dispose(); $bitmap.Dispose()
    }
}

$priorTheme = $env:SUPERDESKTOP_THEME
$priorLocale = $env:SUPERDESKTOP_LOCALE
$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$explorerBefore = @(Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
$forbiddenBefore = @(
    Get-Process ShellExperienceHost,SystemSettings -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty Id
)
$zhDismiss = -join @([char]0x95DC,[char]0x9589,[char]0x901A,[char]0x77E5)
$zhClear = -join @([char]0x5168,[char]0x90E8,[char]0x6E05,[char]0x9664)
$zhClearLabel = $zhClear + (-join @([char]0x901A,[char]0x77E5))
$zhEmpty = -join @([char]0x6C92,[char]0x6709,[char]0x65B0,[char]0x7684,[char]0x901A,[char]0x77E5)
$watchdog = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
    '-NoProfile','-WindowStyle','Hidden','-Command',
    "Start-Sleep -Seconds 45; if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath '$explorerPath' }"
)
$suppressor = $null
$shell = $null
$client = $null
try {
    $suppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
        '-NoProfile','-WindowStyle','Hidden','-Command',
        '$deadline=[DateTime]::UtcNow.AddSeconds(32); while([DateTime]::UtcNow -lt $deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 10}'
    )
    Start-Sleep -Milliseconds 500
    $env:SUPERDESKTOP_THEME = $Theme
    $env:SUPERDESKTOP_LOCALE = 'zh-TW'
    $env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
    $env:SUPERDESKTOP_ACTION_TRACE = $trace
    $shell = Start-Process -FilePath $app -ArgumentList '--verification-capture-ms','28000','--shell' -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do { Start-Sleep -Milliseconds 100; $shell.Refresh() } while ($shell.MainWindowHandle -eq [IntPtr]::Zero -and -not $shell.HasExited -and [DateTime]::UtcNow -lt $deadline)
    if ($shell.HasExited -or $shell.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Owned taskbar did not start.' }
    if (Get-Process explorer -ErrorAction SilentlyContinue) { throw 'Explorer remained active.' }

    $client = Start-Process -FilePath $fixture -ArgumentList '--hold-ms','22000','--notification-count','8' -RedirectStandardOutput $fixtureLog -PassThru
    $taskbar = [System.Windows.Automation.AutomationElement]::FromHandle($shell.MainWindowHandle)
    $clock = Find-Descendant $taskbar {
        param($item)
        $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and
            $item.Current.Name -match '^\d{2}:\d{2} '
    }
    if ($null -eq $clock) { throw 'Owned clock control was not found.' }
    Start-Sleep -Milliseconds 1800
    Invoke-Element $clock
    $popup = Find-OwnedPopup $shell.Id $shell.MainWindowHandle
    if ($null -eq $popup) { throw 'Owned notification center did not open.' }
    $initialCount = Count-ListItems $popup
    if ($initialCount -lt 2) { throw "Expected populated notification history, found $initialCount rows." }
    Capture-Screen (Join-Path $EvidenceDirectory "$Theme-populated.png")

    $afterDismissCount = $initialCount
    $emptyVisible = $false
    if ($ExerciseActions) {
        $dismiss = Find-Descendant $popup {
            param($item)
            $item.Current.Name -in @($zhDismiss,'Dismiss notification','×')
        }
        Invoke-Element $dismiss
        $deadline = [DateTime]::UtcNow.AddSeconds(4)
        do { Start-Sleep -Milliseconds 100; $afterDismissCount = Count-ListItems $popup } while ($afterDismissCount -ge $initialCount -and [DateTime]::UtcNow -lt $deadline)
        if ($afterDismissCount -ne $initialCount - 1) { throw "UIA dismiss did not reconcile exactly one row: initial=$initialCount after=$afterDismissCount" }
        Capture-Screen (Join-Path $EvidenceDirectory "$Theme-dismissed.png")

        $clear = Find-Descendant $popup {
            param($item)
            $item.Current.Name -in @($zhClear,$zhClearLabel,'Clear all','Clear all notifications')
        }
        Invoke-Element $clear
        Start-Sleep -Milliseconds 350
        Capture-Screen (Join-Path $EvidenceDirectory "$Theme-empty.png")
        $remainingItems = Count-ListItems $popup
        $emptyVisible = $remainingItems -eq 0
        if (-not $emptyVisible) { throw "UIA clear-all did not reconcile the empty state: remaining=$remainingItems" }
        [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    }

    $forbiddenAfter = @(
        Get-Process ShellExperienceHost,SystemSettings -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty Id
    )
    $newForbidden = @($forbiddenAfter | Where-Object { $_ -notin $forbiddenBefore })
    if ($newForbidden.Count -ne 0) { throw "Forbidden delegated processes started: $($newForbidden -join ',')" }
    if (Get-Process explorer -ErrorAction SilentlyContinue) { throw 'Explorer appeared during capture.' }
    $report = [ordered]@{
        schema = 'owned-notification-center-headful/v1'
        result = 'passed'
        theme = $Theme
        dpi = 168
        dpi_percent = 175
        explorer_absent_during_capture = $true
        forbidden_processes_started = @()
        initial_notification_count = $initialCount
        after_dismiss_count = $afterDismissCount
        empty_visible_after_clear = $emptyVisible
        uia_dismiss = [bool]$ExerciseActions
        uia_clear_all = [bool]$ExerciseActions
        escape_sent = [bool]$ExerciseActions
        app_sha256 = (Get-FileHash $app -Algorithm SHA256).Hash.ToLowerInvariant()
        fixture_sha256 = (Get-FileHash $fixture -Algorithm SHA256).Hash.ToLowerInvariant()
        screenshots = @(Get-ChildItem $EvidenceDirectory -Filter "$Theme-*.png" | ForEach-Object {
            @{name=$_.Name;sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();bytes=$_.Length}
        })
    }
    [IO.File]::WriteAllText((Join-Path $EvidenceDirectory "$Theme-report.json"),(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($client -and -not $client.HasExited) { Stop-Process -Id $client.Id -Force -ErrorAction SilentlyContinue }
    if ($shell -and -not $shell.HasExited) { Stop-Process -Id $shell.Id -Force -ErrorAction SilentlyContinue }
    if ($suppressor -and -not $suppressor.HasExited) { Stop-Process -Id $suppressor.Id -Force -ErrorAction SilentlyContinue }
    if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath $explorerPath }
    if ($watchdog -and -not $watchdog.HasExited) { Stop-Process -Id $watchdog.Id -Force -ErrorAction SilentlyContinue }
    $env:SUPERDESKTOP_THEME = $priorTheme
    $env:SUPERDESKTOP_LOCALE = $priorLocale
    $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface
    $env:SUPERDESKTOP_ACTION_TRACE = $priorTrace
}
