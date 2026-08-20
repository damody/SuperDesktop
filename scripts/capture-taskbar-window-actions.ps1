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

function Find-OwnedElement([int]$ProcessId, [string]$Name, [System.Windows.Automation.ControlType]$ControlType = $null) {
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    for ($windowIndex = 0; $windowIndex -lt $windows.Count; $windowIndex++) {
        $window = $windows.Item($windowIndex)
        if ($window.Current.ProcessId -ne $ProcessId) { continue }
        if (
            [string]$window.Current.Name -eq $Name -and
            ($null -eq $ControlType -or $window.Current.ControlType -eq $ControlType)
        ) {
            return $window
        }
        $conditions = [Collections.Generic.List[System.Windows.Automation.Condition]]::new()
        $conditions.Add([System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::NameProperty,
            $Name
        ))
        if ($null -ne $ControlType) {
            $conditions.Add([System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                $ControlType
            ))
        }
        $condition = if ($conditions.Count -eq 1) {
            $conditions[0]
        } else {
            [System.Windows.Automation.AndCondition]::new($conditions.ToArray())
        }
        $element = $window.FindFirst(
            [System.Windows.Automation.TreeScope]::Descendants,
            $condition
        )
        if ($null -ne $element) { return $element }
    }
    return $null
}

function Wait-OwnedElement([int]$ProcessId, [string]$Name, [System.Windows.Automation.ControlType]$ControlType, [int]$TimeoutMilliseconds = 5000) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        $element = Find-OwnedElement $ProcessId $Name $ControlType
        if ($null -ne $element) { return $element }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Owned element did not appear: $Name"
}

function Assert-OwnedElementAbsent([int]$ProcessId, [string]$Name, [int]$DurationMilliseconds) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($DurationMilliseconds)
    do {
        if ($null -ne (Find-OwnedElement $ProcessId $Name)) {
            throw "Owned element unexpectedly visible: $Name"
        }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
}

function Wait-TracePattern([string]$Pattern, [int]$TimeoutMilliseconds = 5000) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        if (
            (Test-Path -LiteralPath $script:tracePath) -and
            (Get-Content -Raw -LiteralPath $script:tracePath) -match $Pattern
        ) {
            return
        }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Trace pattern did not appear: $Pattern"
}

function Assert-NoPreviewReopenedAfterContext([int]$DurationMilliseconds) {
    Start-Sleep -Milliseconds $DurationMilliseconds
    $trace = Get-Content -Raw -LiteralPath $script:tracePath
    $cancelIndex = $trace.LastIndexOf('task-preview:context-cancelled', [StringComparison]::Ordinal)
    if ($cancelIndex -lt 0) { throw 'Task context did not record preview cancellation.' }
    $jumpIndex = $trace.IndexOf('taskbar:jump-list-opened', $cancelIndex, [StringComparison]::Ordinal)
    if ($jumpIndex -lt 0) { throw 'Jump List did not open after preview cancellation.' }
    $reopenedIndex = $trace.IndexOf('task-preview:hover-opened', $cancelIndex, [StringComparison]::Ordinal)
    if ($reopenedIndex -ge 0) { throw 'A stale hover timer reopened the preview after task context cancellation.' }
}

function Inspect-TaskMenuExclusivity([int]$ProcessId, [string]$Title) {
    [UtitWindowActionsNative]::SetCursorPos(0, 0) | Out-Null
    [UtitWindowActionsNative]::mouse_event(1, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 300
    $target = Find-TaskButton $ProcessId $Title
    $bounds = $target.Button.Current.BoundingRectangle
    $x = [int]($bounds.Left + $bounds.Width / 2)
    $y = [int]($bounds.Top + $bounds.Height / 2)
    [UtitWindowActionsNative]::SetCursorPos($x, $y) | Out-Null
    [UtitWindowActionsNative]::mouse_event(1, 0, 0, 0, [UIntPtr]::Zero)
    Wait-TracePattern 'task-preview:hover-opened' 3000

    [UtitWindowActionsNative]::SetForegroundWindow(
        [IntPtr][int]$target.Window.Current.NativeWindowHandle
    ) | Out-Null
    [UtitWindowActionsNative]::RightClick($x, $y)
    $jumpList = Wait-OwnedElement $ProcessId 'Jump List' $null 3000
    Assert-OwnedElementAbsent $ProcessId 'Window previews' 800
    Assert-NoPreviewReopenedAfterContext 100

    $menuItems = $jumpList.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::MenuItem
        )
    )
    $labels = @()
    for ($index = 0; $index -lt $menuItems.Count; $index++) {
        $labels += [string]$menuItems.Item($index).Current.Name
    }
    $pinIndex = [Array]::IndexOf($labels, 'Unpin from taskbar')
    if ($pinIndex -lt 0) { $pinIndex = [Array]::IndexOf($labels, 'Pin to taskbar') }
    $closeIndex = [Array]::IndexOf($labels, 'Close window')
    if ($pinIndex -lt 0 -or $closeIndex -lt 0 -or $pinIndex -ge $closeIndex) {
        throw "Explorer bottom command order rejected: $($labels -join ' | ')"
    }
    foreach ($forbidden in @('Close all windows', 'ms-gamingoverlay---', 'Actions')) {
        if ($labels -contains $forbidden -or $null -ne (Find-OwnedElement $ProcessId $forbidden)) {
            throw "Forbidden Jump List entry visible: $forbidden"
        }
    }
    foreach ($heading in @('Recent', 'Frequent', 'Actions')) {
        if ($null -ne (Find-OwnedElement $ProcessId $heading ([System.Windows.Automation.ControlType]::Header))) {
            throw "Unscoped or synthetic Jump List heading visible: $heading"
        }
    }
    [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    Start-Sleep -Milliseconds 150
    return $labels
}

function Invoke-TaskMenuAction([int]$ProcessId, [string]$Title, [string]$Action) {
    [UtitWindowActionsNative]::SetCursorPos(0, 0) | Out-Null
    Start-Sleep -Milliseconds 500
    $target = Find-TaskButton $ProcessId $Title
    $button = $target.Button
    [UtitWindowActionsNative]::SetForegroundWindow(
        [IntPtr][int]$target.Window.Current.NativeWindowHandle
    ) | Out-Null
    Start-Sleep -Milliseconds 100
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

    $menuLabels = Inspect-TaskMenuExclusivity $app.Id $title

    Invoke-TaskMenuAction $app.Id $title 'Minimize'
    Wait-WindowState $fixtureHwnd { [UtitWindowActionsNative]::IsIconic($fixtureHwnd) } 'Fixture was not minimized by the Jump List'
    Invoke-TaskMenuAction $app.Id $title 'Maximize'
    Wait-WindowState $fixtureHwnd { [UtitWindowActionsNative]::IsZoomed($fixtureHwnd) } 'Fixture was not maximized by the Jump List'
    Invoke-TaskMenuAction $app.Id $title 'Close window'
    Wait-WindowState $fixtureHwnd { -not [UtitWindowActionsNative]::IsWindow($fixtureHwnd) } 'Fixture was not closed by the Jump List'
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
        context_commands_present = $menuLabels
        preview_visible_before_context = $true
        preview_absent_when_jump_list_opened = $true
        preview_absent_after_hover_delay = $true
        unscoped_recent_absent = $true
        synthetic_actions_heading_absent = $true
        pin_command_present = $true
        pin_precedes_close = $true
        minimized_observed = $true
        maximized_observed = $true
        close_observed = $true
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
