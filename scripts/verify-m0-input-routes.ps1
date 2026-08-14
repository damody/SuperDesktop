param(
    [string]$Workspace = (Split-Path -Parent $PSScriptRoot),
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$superExplorerPath = Join-Path (Split-Path -Parent (Split-Path -Parent $Workspace)) 'SuperExplorer/target/release/SuperExplorer.exe'

if (-not (Test-Path -LiteralPath $appPath)) {
    throw "Missing release app: $appPath"
}
if (-not (Test-Path -LiteralPath $superExplorerPath)) {
    throw "Missing SuperExplorer fixture: $superExplorerPath"
}

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class M0PointerInput {
    [DllImport("user32.dll")]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")]
    public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}
'@
[M0PointerInput]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorExplorer = $env:SUPEREXPLORER_PATH

function Get-SuperExplorerIds {
    @(Get-Process SuperExplorer -ErrorAction SilentlyContinue | ForEach-Object Id)
}

function Wait-ForWindowHandle([System.Diagnostics.Process]$Process) {
    $deadline = [DateTime]::UtcNow.AddSeconds(3)
    do {
        Start-Sleep -Milliseconds 50
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero) {
            return $Process.MainWindowHandle
        }
    } while ([DateTime]::UtcNow -lt $deadline -and -not $Process.HasExited)
    throw "No interactive window appeared for PID $($Process.Id)."
}

function Wait-ForFixedControl([IntPtr]$WindowHandle) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($WindowHandle)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::NameProperty,
        'SuperExplorer'
    )
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do {
        $control = $root.FindFirst(
            [System.Windows.Automation.TreeScope]::Descendants,
            $condition
        )
        if ($null -ne $control) {
            return $control
        }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'UI Automation could not find the SuperExplorer fixed entry.'
}

function Invoke-InputRoute([string]$Surface, [string]$Route) {
    $tracePath = Join-Path $env:TEMP ("superdesktop-$Surface-$Route-$PID.log")
    Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
    $beforeIds = Get-SuperExplorerIds
    $env:SUPERDESKTOP_ACTION_TRACE = $tracePath
    $env:SUPERDESKTOP_VERIFICATION_SURFACE = $Surface
    $env:SUPEREXPLORER_PATH = $superExplorerPath
    $process = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms', '4500' -PassThru
    $newIds = @()
    try {
        $windowHandle = Wait-ForWindowHandle $process
        $frameDeadline = [DateTime]::UtcNow.AddSeconds(5)
        do {
            Start-Sleep -Milliseconds 50
            $frameTrace = if (Test-Path -LiteralPath $tracePath) {
                Get-Content -Raw -Encoding UTF8 $tracePath
            } else {
                ''
            }
        } while ($frameTrace -notmatch 'frame-visible' -and [DateTime]::UtcNow -lt $frameDeadline)
        if ($frameTrace -notmatch 'frame-visible') {
            throw "$Surface/$Route did not render a visible frame."
        }
        try {
            $control = Wait-ForFixedControl $windowHandle
        } catch {
            throw "$Surface/$Route failed to expose the SuperExplorer fixed entry: $_"
        }
        $name = $control.Current.Name
        $controlType = $control.Current.ControlType.ProgrammaticName
        $bounds = $control.Current.BoundingRectangle

        switch ($Route) {
            'pointer' {
                $x = [int]($bounds.Left + ($bounds.Width / 2))
                $y = [int]($bounds.Top + ($bounds.Height / 2))
                [M0PointerInput]::SetCursorPos($x, $y) | Out-Null
                [M0PointerInput]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
                [M0PointerInput]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
            }
            'keyboard' {
                [System.Windows.Forms.SendKeys]::SendWait('{ENTER}')
            }
            'uia' {
                $pattern = $control.GetCurrentPattern(
                    [System.Windows.Automation.InvokePattern]::Pattern
                )
                $pattern.Invoke()
            }
            default { throw "Unknown route: $Route" }
        }

        $launchDeadline = [DateTime]::UtcNow.AddSeconds(3)
        do {
            Start-Sleep -Milliseconds 50
            $trace = if (Test-Path -LiteralPath $tracePath) {
                Get-Content -Raw -Encoding UTF8 $tracePath
            } else {
                ''
            }
            $afterIds = Get-SuperExplorerIds
            $newIds = @($afterIds | Where-Object { $_ -notin $beforeIds })
        } while (($trace -notmatch 'superexplorer:launched' -or $newIds.Count -eq 0) -and [DateTime]::UtcNow -lt $launchDeadline)

        if ($trace -notmatch 'superexplorer:launched') {
            throw "$Surface/$Route did not emit superexplorer:launched."
        }
        if ($newIds.Count -eq 0) {
            throw "$Surface/$Route did not create a SuperExplorer process."
        }

        [ordered]@{
            surface = $Surface
            route = $Route
            result = 'passed'
            control_name = $name
            control_type = $controlType
            bounds = [ordered]@{
                left = [int]$bounds.Left
                top = [int]$bounds.Top
                width = [int]$bounds.Width
                height = [int]$bounds.Height
            }
            launched_process_count = $newIds.Count
            trace_contains_launch = $true
        }
    } finally {
        if (-not $process.HasExited) {
            $process.WaitForExit(7000) | Out-Null
        }
        foreach ($newId in $newIds) {
            Stop-Process -Id $newId -Force -ErrorAction SilentlyContinue
            Wait-Process -Id $newId -Timeout 5 -ErrorAction SilentlyContinue
        }
        Remove-Item -LiteralPath $tracePath -ErrorAction SilentlyContinue
    }
}

try {
    $results = @()
    foreach ($surface in @('desktop', 'taskbar')) {
        foreach ($route in @('pointer', 'keyboard', 'uia')) {
            $results += Invoke-InputRoute $surface $route
        }
    }
    $report = [ordered]@{
        schema = 'm0-real-input-routes/v1'
        app_sha256 = (Get-FileHash $appPath -Algorithm SHA256).Hash
        superexplorer_sha256 = (Get-FileHash $superExplorerPath -Algorithm SHA256).Hash
        route_count = $results.Count
        all_passed = @($results | Where-Object result -ne 'passed').Count -eq 0
        routes = $results
    }
    $json = $report | ConvertTo-Json -Depth 12
    if ($OutputPath) {
        $parent = Split-Path -Parent $OutputPath
        if ($parent) {
            New-Item -ItemType Directory -Force $parent | Out-Null
        }
        [IO.File]::WriteAllText($OutputPath, $json + "`n", [Text.UTF8Encoding]::new($false))
    }
    $json
} finally {
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE = $priorTrace }
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface }
    if ($null -eq $priorExplorer) { Remove-Item Env:SUPEREXPLORER_PATH -ErrorAction SilentlyContinue } else { $env:SUPEREXPLORER_PATH = $priorExplorer }
}
