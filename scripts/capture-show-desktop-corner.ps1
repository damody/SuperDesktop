param(
    [string]$Workspace,
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [ValidateSet('light','dark','high-contrast')][string]$Theme = 'light',
    [ValidateRange(1,3)][int]$Rows = 2,
    [switch]$SuppressExplorer,
    [switch]$ExerciseCycle
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = Split-Path -Parent $PSScriptRoot }
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
New-Item -ItemType Directory -Force $EvidenceDirectory | Out-Null
$captureKey = "$Theme-row$Rows"
$tracePath = Join-Path $EvidenceDirectory "$captureKey-action.log"
$explorerPath = Join-Path $env:WINDIR 'explorer.exe'
$profileRoot = Join-Path $env:TEMP "superdesktop-show-desktop-$PID-$captureKey"
$settingsRoot = Join-Path $profileRoot 'SuperDesktop'
New-Item -ItemType Directory -Force $settingsRoot | Out-Null
$settingsJson = '{{"schema_version":1,"revision":0,"taskbar":{{"rows":{0},"locked":true,"pins":[]}}}}' -f $Rows
[IO.File]::WriteAllText((Join-Path $settingsRoot 'settings.json'),$settingsJson,[Text.UTF8Encoding]::new($false))

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ShowDesktopCaptureNative {
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}
'@
[ShowDesktopCaptureNative]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

function New-Fixture([string]$Title, [bool]$Minimized) {
    $state = if ($Minimized) { 'Minimized' } else { 'Normal' }
    $script = @"
Add-Type -AssemblyName System.Windows.Forms
`$form = [Windows.Forms.Form]::new()
`$form.Text = '$Title'
`$form.Width = 520
`$form.Height = 320
`$form.StartPosition = 'CenterScreen'
`$form.Controls.Add([Windows.Forms.Label]@{ Text='$Title'; AutoSize=`$true; Left=24; Top=24 })
`$form.Add_Shown({ `$form.WindowState = [Windows.Forms.FormWindowState]::$state })
[void]`$form.ShowDialog()
"@
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($script))
    $process = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile','-STA','-WindowStyle','Hidden','-EncodedCommand',$encoded
    $deadline = [DateTime]::UtcNow.AddSeconds(6)
    do { Start-Sleep -Milliseconds 100; $process.Refresh() } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and -not $process.HasExited -and [DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw "Fixture did not expose HWND: $Title" }
    return $process
}

function Wait-Iconic([IntPtr]$Hwnd, [bool]$Expected, [string]$Label) {
    $deadline = [DateTime]::UtcNow.AddSeconds(4)
    do {
        $actual = [ShowDesktopCaptureNative]::IsIconic($Hwnd)
        if ($actual -eq $Expected) { return }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "$Label iconic state did not become $Expected"
}

function Capture-Bounds([object]$Bounds, [string]$Path) {
    $bitmap = [Drawing.Bitmap]::new([int]$Bounds.Width, [int]$Bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$Bounds.Left,[int]$Bounds.Top,0,0,$bitmap.Size)
    $bitmap.Save($Path,[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose(); $bitmap.Dispose()
}

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorLocalAppData = $env:LOCALAPPDATA
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorTheme = $env:SUPERDESKTOP_THEME
$priorLocale = $env:SUPERDESKTOP_LOCALE
$env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
$env:LOCALAPPDATA = $profileRoot
$env:SUPERDESKTOP_ACTION_TRACE = $tracePath
$env:SUPERDESKTOP_THEME = $Theme
$env:SUPERDESKTOP_LOCALE = 'en-US'
$watchdog = $null
$suppressor = $null
$app = $null
$fixtures = @()
$explorerBefore = @(Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
$forbiddenBefore = @('ShellExperienceHost','SystemSettings') | ForEach-Object { Get-Process $_ -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id }
try {
    if ($SuppressExplorer) {
        $watchdog = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile','-WindowStyle','Hidden','-Command',
            "Start-Sleep -Seconds 35; if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath '$explorerPath' }"
        )
        $suppressor = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile','-WindowStyle','Hidden','-Command',
            '$deadline=[DateTime]::UtcNow.AddSeconds(24); while([DateTime]::UtcNow -lt $deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 10}'
        )
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        do { Start-Sleep -Milliseconds 100 } while ((Get-Process explorer -ErrorAction SilentlyContinue) -and [DateTime]::UtcNow -lt $deadline)
        if (Get-Process explorer -ErrorAction SilentlyContinue) { throw 'Explorer suppression failed.' }
    }

    if ($ExerciseCycle) {
        $fixtures += New-Fixture 'SuperDesktop Show Desktop A' $false
        $fixtures += New-Fixture 'SuperDesktop Show Desktop B' $false
        $fixtures += New-Fixture 'SuperDesktop Preserved Minimized' $true
        Wait-Iconic $fixtures[0].MainWindowHandle $false 'fixture A initial'
        Wait-Iconic $fixtures[1].MainWindowHandle $false 'fixture B initial'
        Wait-Iconic $fixtures[2].MainWindowHandle $true 'fixture preserved initial'
    }

    Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
    $arguments = @('--verification-capture-ms','14000')
    if ($SuppressExplorer) { $arguments += '--shell' }
    $app = Start-Process -FilePath $appPath -ArgumentList $arguments -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(6)
    do { Start-Sleep -Milliseconds 100; $app.Refresh() } while ($app.MainWindowHandle -eq [IntPtr]::Zero -and -not $app.HasExited -and [DateTime]::UtcNow -lt $deadline)
    if ($app.MainWindowHandle -eq [IntPtr]::Zero) { throw 'SuperDesktop taskbar did not appear.' }
    Start-Sleep -Milliseconds 700
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($app.MainWindowHandle)
    $allElements = $root.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    $corner = $null
    for ($index = 0; $index -lt $allElements.Count; $index++) {
        $candidate = $allElements.Item($index)
        if ([string]$candidate.Current.Name -in @('顯示桌面','Show desktop')) { $corner = $candidate; break }
    }
    if ($null -eq $corner) {
        $diagnostics = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition) |
            ForEach-Object { "$($_.Current.Name) [$($_.Current.ControlType.ProgrammaticName)] id=$($_.Current.AutomationId)" }
        throw "Show desktop UIA button not found. $($diagnostics -join ' | ')"
    }
    $cornerBounds = $corner.Current.BoundingRectangle
    $rootBounds = $root.Current.BoundingRectangle
    $monitorBounds = [Windows.Forms.Screen]::FromHandle($app.MainWindowHandle).Bounds
    $rightGap = [Math]::Abs(($cornerBounds.Left + $cornerBounds.Width) - $monitorBounds.Right)
    if ($corner.Current.ControlType -ne [System.Windows.Automation.ControlType]::Button -or $cornerBounds.Width -lt 12 -or $cornerBounds.Width -gt 16 -or $rightGap -gt 1) {
        throw "Far-edge geometry mismatch: corner=$cornerBounds root=$rootBounds gap=$rightGap"
    }
    $beforePath = Join-Path $EvidenceDirectory "$captureKey-before.png"
    [ShowDesktopCaptureNative]::SetCursorPos([int]($rootBounds.Left + 30),[int]($rootBounds.Top + 20)) | Out-Null
    Start-Sleep -Milliseconds 150
    Capture-Bounds $rootBounds $beforePath

    $invoke = $corner.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    if ($ExerciseCycle) {
        $invoke.Invoke()
        Wait-Iconic $fixtures[0].MainWindowHandle $true 'fixture A minimized'
        Wait-Iconic $fixtures[1].MainWindowHandle $true 'fixture B minimized'
        Wait-Iconic $fixtures[2].MainWindowHandle $true 'fixture preserved after minimize'
        Start-Sleep -Milliseconds 250
        Capture-Bounds $rootBounds (Join-Path $EvidenceDirectory "$captureKey-desktop-shown.png")
        $invoke.Invoke()
        Wait-Iconic $fixtures[0].MainWindowHandle $false 'fixture A restored'
        Wait-Iconic $fixtures[1].MainWindowHandle $false 'fixture B restored'
        Wait-Iconic $fixtures[2].MainWindowHandle $true 'fixture preserved after restore'
    }
    $corner.SetFocus()
    [ShowDesktopCaptureNative]::SetCursorPos([int]($cornerBounds.Left + $cornerBounds.Width / 2),[int]($cornerBounds.Top + $cornerBounds.Height / 2)) | Out-Null
    Start-Sleep -Milliseconds 200
    $focusPath = Join-Path $EvidenceDirectory "$captureKey-focus-hover.png"
    Capture-Bounds $rootBounds $focusPath

    $trace = Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath
    if ($ExerciseCycle -and ($trace -notmatch 'show-desktop:minimized' -or $trace -notmatch 'show-desktop:restored')) { throw 'Show desktop cycle trace incomplete.' }
    $explorerAbsent = -not [bool](Get-Process explorer -ErrorAction SilentlyContinue)
    if ($SuppressExplorer -and -not $explorerAbsent) { throw 'Explorer appeared during capture.' }
    $forbiddenAfter = @('ShellExperienceHost','SystemSettings') | ForEach-Object { Get-Process $_ -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id }
    $forbiddenLaunched = @($forbiddenAfter | Where-Object { $_ -notin $forbiddenBefore })
    if ($forbiddenLaunched) { throw 'A forbidden shell UI process was launched during capture.' }
    $report = [ordered]@{
        schema='owned-show-desktop-corner-headful/v1'; result='passed'; theme=$Theme; rows=$Rows
        app_sha256=(Get-FileHash $appPath -Algorithm SHA256).Hash.ToLowerInvariant()
        explorer_before=$explorerBefore; explorer_absent_during_capture=$explorerAbsent
        uia=@{name=$corner.Current.Name;control_type=$corner.Current.ControlType.ProgrammaticName;invoke_pattern=$true}
        root_bounds=@{left=[int]$rootBounds.Left;top=[int]$rootBounds.Top;width=[int]$rootBounds.Width;height=[int]$rootBounds.Height}
        monitor_bounds=@{left=$monitorBounds.Left;top=$monitorBounds.Top;width=$monitorBounds.Width;height=$monitorBounds.Height}
        corner_bounds=@{left=[int]$cornerBounds.Left;top=[int]$cornerBounds.Top;width=[int]$cornerBounds.Width;height=[int]$cornerBounds.Height;right_gap=[double]$rightGap}
        cycle_exercised=[bool]$ExerciseCycle; visible_minimized=if($ExerciseCycle){2}else{0}; visible_restored=if($ExerciseCycle){2}else{0}; pre_minimized_preserved=[bool]$ExerciseCycle
        screenshots=@($beforePath,$focusPath) | ForEach-Object { @{name=(Split-Path -Leaf $_);sha256=(Get-FileHash $_ -Algorithm SHA256).Hash.ToLowerInvariant()} }
        forbidden_processes_launched=$forbiddenLaunched; shell_delegation=$false
    }
    $reportPath = Join-Path $EvidenceDirectory "$captureKey-report.json"
    [IO.File]::WriteAllText($reportPath,(($report|ConvertTo-Json -Depth 8)+[Environment]::NewLine),[Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($app -and -not $app.HasExited) { Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue }
    foreach ($fixture in $fixtures) { if ($fixture -and -not $fixture.HasExited) { Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue } }
    if ($suppressor -and -not $suppressor.HasExited) { Stop-Process -Id $suppressor.Id -Force -ErrorAction SilentlyContinue }
    if ($SuppressExplorer -and -not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process -FilePath $explorerPath }
    if ($watchdog -and -not $watchdog.HasExited) { Stop-Process -Id $watchdog.Id -Force -ErrorAction SilentlyContinue }
    if ($null -eq $priorLocalAppData) { Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue } else { $env:LOCALAPPDATA=$priorLocalAppData }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE=$priorTrace }
    if ($null -eq $priorTheme) { Remove-Item Env:SUPERDESKTOP_THEME -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_THEME=$priorTheme }
    if ($null -eq $priorLocale) { Remove-Item Env:SUPERDESKTOP_LOCALE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_LOCALE=$priorLocale }
    Remove-Item -LiteralPath $profileRoot -Recurse -Force -ErrorAction SilentlyContinue
}
