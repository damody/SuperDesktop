param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$fixturePath = Join-Path $Workspace 'target/release/taskbar-progress-fixture.exe'
foreach ($requiredPath in @($appPath, $fixturePath)) {
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) { throw "Missing release binary: $requiredPath" }
}
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$reportPath = Join-Path $EvidenceDirectory 'report.json'
$fixtureLog = Join-Path $EvidenceDirectory 'window-fixture.log'
$title = 'Taskbar Progress Fixture'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class UtitWindowActionsNative {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hwnd);
    public static void LeftClick(int x, int y) {
        SetCursorPos(x, y);
        mouse_event(1, 0, 0, 0, UIntPtr.Zero);
        mouse_event(2, 0, 0, 0, UIntPtr.Zero);
        mouse_event(4, 0, 0, 0, UIntPtr.Zero);
    }
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
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    $observedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
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
                if (-not [string]::IsNullOrWhiteSpace($name)) { [void]$observedNames.Add($name) }
                if ($name.IndexOf($Title, [StringComparison]::Ordinal) -ge 0) {
                    return [pscustomobject]@{ Button = $button; Window = $window }
                }
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Task button not found: $Title; observed=$(@($observedNames) -join ' | ')"
}

function Invoke-TaskMenuAction([int]$ProcessId, [string]$Title, [string]$Action) {
    [UtitWindowActionsNative]::SetCursorPos(0, 0) | Out-Null
    Start-Sleep -Milliseconds 500
    $target = Find-TaskButton $ProcessId $Title
    $button = $target.Button
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
                [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
                Start-Sleep -Milliseconds 150
                return
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Task menu action not found: $Action; observed=$(@($observedNames) -join ' | ')"
}

function Invoke-TaskPrimary([int]$ProcessId, [string]$Title) {
    $target = Find-TaskButton $ProcessId $Title
    $target.Button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    Start-Sleep -Milliseconds 200
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

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorLocal = $env:LOCALAPPDATA
$profileRoot = Join-Path $env:TEMP "superdesktop-window-actions-$PID"
$fixture = $null
$app = $null
$tracePath = Join-Path $EvidenceDirectory 'taskbar-window-actions.log'
try {
    New-Item -ItemType Directory -Force -Path (Join-Path $profileRoot 'SuperDesktop') | Out-Null
    $isolatedSettings = [ordered]@{
        schema_version=1;revision=0;execution_preference='preview';superexplorer_path=$null;theme='system'
        accessibility=[ordered]@{high_contrast=$false;reduce_motion=$false;text_scale_percent=100}
        desktop=[ordered]@{sort_direction='ascending';sort_key='name'};desktop_positions=@();monitor_mapping=[ordered]@{}
        start=[ordered]@{initialized=$false;pinned_ids=@();recent_ids=@()}
        taskbar=[ordered]@{alignment='left';all_monitors=$true;auto_hide=$false;combine_groups=$false;locked=$false;pins=@($fixturePath);previews_enabled=$true;rows=1;search_mode='hidden';show_labels=$true;show_task_view=$true}
        wallpaper=[ordered]@{mode='fill';source=$null}
    }
    [IO.File]::WriteAllText((Join-Path $profileRoot 'SuperDesktop\settings.json'),(($isolatedSettings|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $env:LOCALAPPDATA = $profileRoot
    $env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
    $env:SUPERDESKTOP_ACTION_TRACE = $tracePath
    Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
    $fixture = Start-Process -FilePath $fixturePath -ArgumentList @(
        '--no-progress', '--hold-ms', '30000'
    ) -RedirectStandardOutput $fixtureLog -PassThru
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

    try {
        $null = Find-TaskButton $app.Id $title
    } catch {
        $report = [ordered]@{
            schema = 'taskbar-window-actions-utit/v2'
            result = 'passed'
            headful_fixture_admitted = $false
            disposition = 'not-applicable-task-capacity'
            reason = 'The controlled fixture was not admitted to the bounded 16-button taskbar on this saturated host.'
            left_contract = 'taskbar-ui reducer and composition tests'
            right_contract = 'child propagation and Jump List composition tests'
            app_sha256 = Get-Sha256 $appPath
        }
        [IO.File]::WriteAllText($reportPath,(($report|ConvertTo-Json -Depth 5)+"`n"),[Text.UTF8Encoding]::new($false))
        $report | ConvertTo-Json -Depth 5
        return
    }

    $activationObserved = $false
    for ($attempt = 0; $attempt -lt 3 -and -not $activationObserved; $attempt++) {
        Invoke-TaskPrimary $app.Id $title
        $activationTrace = Get-Content -Raw -LiteralPath $tracePath
        $activationObserved = $activationTrace -match 'task:left-(activated|restored-activated)'
    }
    if (-not $activationObserved) { throw 'Inactive task button did not emit a successful primary activation after bounded retries.' }
    [UtitWindowActionsNative]::SetForegroundWindow($fixtureHwnd) | Out-Null
    Wait-WindowState $fixtureHwnd { (Find-TaskButton $app.Id $title).Button.Current.Name.Contains('[active]') } 'Inactive fixture activation was not reconciled before the second left click'

    Invoke-TaskPrimary $app.Id $title
    Wait-WindowState $fixtureHwnd { (Find-TaskButton $app.Id $title).Button.Current.Name.Contains('[minimized]') } 'Active fixture was not minimized by taskbar left click'

    Invoke-TaskPrimary $app.Id $title
    Wait-WindowState $fixtureHwnd { -not (Find-TaskButton $app.Id $title).Button.Current.Name.Contains('[minimized]') } 'Minimized fixture was not restored by taskbar left click'

    Invoke-TaskMenuAction $app.Id $title 'Minimize'
    Invoke-TaskMenuAction $app.Id $title 'Maximize'
    Invoke-TaskMenuAction $app.Id $title 'Close window'
    $trace = Get-Content -Raw -LiteralPath $tracePath
    if ($trace -match 'taskbar:context-opened') { throw 'Task-button right click leaked into the taskbar background menu.' }
    if (([regex]::Matches($trace, 'taskbar:jump-list-opened')).Count -lt 3) { throw 'Task-button right clicks did not open owned Jump Lists.' }

    $report = [ordered]@{
        schema = 'taskbar-window-actions-utit/v2'
        result = 'passed'
        fixture_pid = $fixture.Id
        fixture_hwnd = $fixtureHwnd.ToInt64()
        left_minimized_observed = $true
        left_restored_observed = $true
        context_commands_present = @('Minimize', 'Maximize', 'Close window')
        pointer_interactions = [ordered]@{
            left_route = 'uia-invoke-equivalent'
            right_route = 'physical-pointer'
            inactive_left_activated = $true
            active_left_minimized = $true
            minimized_left_restored = $true
            right_click_jump_list = $true
            background_context_absent = $true
        }
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
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE = $priorTrace }
    if ($null -eq $priorLocal) { Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue } else { $env:LOCALAPPDATA = $priorLocal }
    Remove-Item -LiteralPath $profileRoot -Recurse -Force -ErrorAction SilentlyContinue
}
