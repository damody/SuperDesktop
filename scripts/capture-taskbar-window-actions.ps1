param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$reportPath = Join-Path $EvidenceDirectory 'report.json'
$fixtureScript = Join-Path $EvidenceDirectory 'window-fixture.ps1'
$title = 'SuperDesktop UTIT Window Actions'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class UtitWindowActionsNative {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hwnd);
    public static void RightClick(int x, int y) {
        SetCursorPos(x, y);
        mouse_event(1, 0, 0, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(150);
        mouse_event(8, 0, 0, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(50);
        mouse_event(16, 0, 0, 0, UIntPtr.Zero);
    }
}
'@

function Find-TaskButton([int]$ProcessId, [string]$Title) {
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($windowIndex = 0; $windowIndex -lt $windows.Count; $windowIndex++) {
            $window = $windows.Item($windowIndex)
            if ($window.Current.ProcessId -ne $ProcessId) { continue }
            $buttons = $window.FindAll(
                [System.Windows.Automation.TreeScope]::Descendants,
                [System.Windows.Automation.PropertyCondition]::new(
                    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                    [System.Windows.Automation.ControlType]::Button
                )
            )
            for ($buttonIndex = 0; $buttonIndex -lt $buttons.Count; $buttonIndex++) {
                $button = $buttons.Item($buttonIndex)
                $name = [string]$button.Current.Name
                if ($name.StartsWith($Title + ' [', [StringComparison]::Ordinal)) {
                    return [pscustomobject]@{ Button = $button; Window = $window }
                }
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Task button not found: $Title"
}

function Invoke-TaskMenuAction([int]$ProcessId, [string]$Title, [string]$Action) {
    $target = Find-TaskButton $ProcessId $Title
    $button = $target.Button
    [UtitWindowActionsNative]::SetForegroundWindow(
        [IntPtr][int]$target.Window.Current.NativeWindowHandle
    ) | Out-Null
    Start-Sleep -Milliseconds 150
    $bounds = $button.Current.BoundingRectangle
    [UtitWindowActionsNative]::RightClick(
        [int]($bounds.Left + $bounds.Width / 2),
        [int]($bounds.Top + $bounds.Height / 2)
    )
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    $observedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    do {
        $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($windowIndex = 0; $windowIndex -lt $windows.Count; $windowIndex++) {
            $window = $windows.Item($windowIndex)
            if ($window.Current.ProcessId -ne $ProcessId) { continue }
            $elements = $window.FindAll(
                [System.Windows.Automation.TreeScope]::Descendants,
                [System.Windows.Automation.Condition]::TrueCondition
            )
            for ($elementIndex = 0; $elementIndex -lt $elements.Count; $elementIndex++) {
                $observedName = [string]$elements.Item($elementIndex).Current.Name
                if (-not [string]::IsNullOrWhiteSpace($observedName)) { [void]$observedNames.Add($observedName) }
            }
            $item = $window.FindFirst(
                [System.Windows.Automation.TreeScope]::Descendants,
                [System.Windows.Automation.AndCondition]::new(@(
                    [System.Windows.Automation.PropertyCondition]::new(
                        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                        [System.Windows.Automation.ControlType]::MenuItem
                    ),
                    [System.Windows.Automation.PropertyCondition]::new(
                        [System.Windows.Automation.AutomationElement]::NameProperty,
                        $Action
                    )
                ))
            )
            if ($null -ne $item) {
                $invoke = [System.Windows.Automation.InvokePattern]$item.GetCurrentPattern(
                    [System.Windows.Automation.InvokePattern]::Pattern
                )
                $invoke.Invoke()
                return
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Task menu action not found: $Action; observed=$(@($observedNames) -join ' | ')"
}

function Wait-WindowState([IntPtr]$Hwnd, [scriptblock]$Predicate, [string]$Failure) {
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        if (& $Predicate) { return }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw $Failure
}

function Get-Sha256([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    try {
        $sha256 = [Security.Cryptography.SHA256]::Create()
        try {
            return ([BitConverter]::ToString($sha256.ComputeHash($stream))).Replace('-', '')
        } finally {
            $sha256.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

$fixtureSource = @'
Add-Type -AssemblyName System.Windows.Forms
$form = [System.Windows.Forms.Form]::new()
$form.Text = 'SuperDesktop UTIT Window Actions'
$form.Width = 720
$form.Height = 480
$form.StartPosition = 'CenterScreen'
$form.ShowInTaskbar = $true
[void]$form.ShowDialog()
'@
[IO.File]::WriteAllText($fixtureScript, $fixtureSource, [Text.UTF8Encoding]::new($false))

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorLocal = $env:LOCALAPPDATA
$profileRoot = Join-Path $env:TEMP "superdesktop-window-actions-$PID"
$fixture = $null
$app = $null
try {
    New-Item -ItemType Directory -Force -Path (Join-Path $profileRoot 'SuperDesktop') | Out-Null
    $env:LOCALAPPDATA = $profileRoot
    $env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
    $fixture = Start-Process -FilePath 'powershell.exe' -ArgumentList @(
        '-NoLogo', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $fixtureScript
    ) -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do { Start-Sleep -Milliseconds 100; $fixture.Refresh() } while (
        $fixture.MainWindowHandle -eq [IntPtr]::Zero -and
        -not $fixture.HasExited -and
        [DateTime]::UtcNow -lt $deadline
    )
    if ($fixture.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Fixture window did not appear' }
    $fixtureHwnd = [IntPtr]$fixture.MainWindowHandle

    $app = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms','18000' -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do { Start-Sleep -Milliseconds 100; $app.Refresh() } while (
        $app.MainWindowHandle -eq [IntPtr]::Zero -and
        -not $app.HasExited -and
        [DateTime]::UtcNow -lt $deadline
    )
    if ($app.MainWindowHandle -eq [IntPtr]::Zero) { throw 'SuperDesktop taskbar did not appear' }
    Start-Sleep -Milliseconds 1000

    Invoke-TaskMenuAction $app.Id $title 'Minimize'
    Wait-WindowState $fixtureHwnd { [UtitWindowActionsNative]::IsIconic($fixtureHwnd) } 'Fixture was not minimized'

    Invoke-TaskMenuAction $app.Id $title 'Maximize'
    Wait-WindowState $fixtureHwnd { [UtitWindowActionsNative]::IsZoomed($fixtureHwnd) } 'Fixture was not maximized'

    Invoke-TaskMenuAction $app.Id $title 'Close window'
    Wait-WindowState $fixtureHwnd { -not [UtitWindowActionsNative]::IsWindow($fixtureHwnd) } 'Fixture was not closed'
    $fixture.WaitForExit(5000) | Out-Null

    $report = [ordered]@{
        schema = 'taskbar-window-actions-utit/v1'
        result = 'passed'
        fixture_pid = $fixture.Id
        fixture_hwnd = $fixtureHwnd.ToInt64()
        minimized_observed = $true
        maximized_observed = $true
        close_observed = $true
        exact_actions = @('Minimize', 'Maximize', 'Close window')
        app_sha256 = Get-Sha256 $appPath
    }
    [IO.File]::WriteAllText(
        $reportPath,
        (($report | ConvertTo-Json -Depth 5) + "`n"),
        [Text.UTF8Encoding]::new($false)
    )
    $report | ConvertTo-Json -Depth 5
} finally {
    if ($null -ne $fixture -and -not $fixture.HasExited) { Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue }
    if ($null -ne $app -and -not $app.HasExited) { Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface }
    if ($null -eq $priorLocal) { Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue } else { $env:LOCALAPPDATA = $priorLocal }
    Remove-Item -LiteralPath $profileRoot -Recurse -Force -ErrorAction SilentlyContinue
}
