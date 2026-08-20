param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$audioControl = Join-Path $Workspace 'target/release/examples/audio_status_control.exe'
foreach ($required in @($appPath, $audioControl)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Missing release binary: $required" }
}
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$tracePath = Join-Path $EvidenceDirectory 'volume-recovery.log'
$stdoutPath = Join-Path $EvidenceDirectory 'app-stdout.log'
$stderrPath = Join-Path $EvidenceDirectory 'app-stderr.log'
Remove-Item -LiteralPath $tracePath,$stdoutPath,$stderrPath -ErrorAction SilentlyContinue

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class VolumeRecoveryPointer {
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    public static void LeftClick(int x, int y) {
        SetCursorPos(x,y);
        mouse_event(0x0001,0,0,0,UIntPtr.Zero);
        System.Threading.Thread.Sleep(40);
        mouse_event(0x0002,0,0,0,UIntPtr.Zero);
        mouse_event(0x0004,0,0,0,UIntPtr.Zero);
    }
    public static void SendKey(byte key) {
        keybd_event(key,0,0,UIntPtr.Zero);
        System.Threading.Thread.Sleep(40);
        keybd_event(key,0,0x0002,UIntPtr.Zero);
    }
}
'@
[VolumeRecoveryPointer]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

function Get-AudioStatus {
    $output = & $audioControl snapshot 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Audio snapshot failed: $output" }
    ($output | Select-Object -Last 1) | ConvertFrom-Json
}

function Restore-AudioStatus($Status) {
    if ($null -eq $Status) { return }
    $output = & $audioControl restore ([int]$Status.volume_percent) ([bool]$Status.muted).ToString().ToLowerInvariant() 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Audio restore failed: $output" }
}

function Find-Descendant($Root, [scriptblock]$Predicate, [int]$TimeoutMilliseconds = 4000) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        try {
            $all = $Root.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
            for ($index=0; $index -lt $all.Count; $index++) {
                $candidate = $all.Item($index)
                if (& $Predicate $candidate) { return $candidate }
            }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
        Start-Sleep -Milliseconds 75
    } while ([DateTime]::UtcNow -lt $deadline)
    $null
}

function Find-VolumePopup([int]$ProcessId, [IntPtr]$TaskbarHwnd) {
    $deadline = [DateTime]::UtcNow.AddSeconds(4)
    do {
        $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($index=0; $index -lt $windows.Count; $index++) {
            $window = $windows.Item($index)
            try {
                if ($window.Current.ProcessId -ne $ProcessId) { continue }
                if ([IntPtr][int]$window.Current.NativeWindowHandle -eq $TaskbarHwnd) { continue }
                $slider = Find-Descendant $window { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Slider } 150
                if ($null -ne $slider) { return [ordered]@{Root=$window;Slider=$slider} }
            } catch [System.Windows.Automation.ElementNotAvailableException] {}
        }
        Start-Sleep -Milliseconds 75
    } while ([DateTime]::UtcNow -lt $deadline)
    $null
}

function Click-Element($Element) {
    if ($null -eq $Element) { throw 'Pointer target is unavailable.' }
    $bounds = $Element.Current.BoundingRectangle
    [VolumeRecoveryPointer]::LeftClick([int]($bounds.Left+$bounds.Width/2),[int]($bounds.Top+$bounds.Height/2))
    Start-Sleep -Milliseconds 150
}

function Get-SliderValue($Slider) {
    if ($null -eq $Slider) { throw 'Volume slider is unavailable.' }
    $pattern = $Slider.GetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern)
    if ($null -eq $pattern) { throw 'Volume slider range pattern is unavailable.' }
    [int][Math]::Round($pattern.Current.Value)
}

function Get-StatusHosts([int]$ParentProcessId) {
    @(Get-CimInstance Win32_Process -Filter "Name='system-status-host.exe'" |
        Where-Object { $_.ParentProcessId -eq $ParentProcessId } |
        Sort-Object CreationDate)
}

function Restart-ObserverStatusHost([int]$ParentProcessId) {
    $before = @(Get-StatusHosts $ParentProcessId)
    if ($before.Count -lt 2) { throw "Expected command and observer status hosts, found $($before.Count)." }
    $commandHost = $before[0]
    $observerHost = $before[-1]
    Stop-Process -Id ([int]$observerHost.ProcessId) -Force
    $beforeIds = @($before | ForEach-Object { [int]$_.ProcessId })
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    $replacement = $null
    do {
        Start-Sleep -Milliseconds 100
        $replacement = @(Get-StatusHosts $ParentProcessId | Where-Object { [int]$_.ProcessId -notin $beforeIds } | Select-Object -First 1)
    } while ($replacement.Count -eq 0 -and [DateTime]::UtcNow -lt $deadline)
    if ($replacement.Count -eq 0) { throw 'Observer status host did not restart.' }
    Start-Sleep -Milliseconds 900
    [ordered]@{
        command_host_pid=[int]$commandHost.ProcessId
        stopped_observer_pid=[int]$observerHost.ProcessId
        replacement_observer_pid=[int]$replacement[0].ProcessId
    }
}

function Wait-AudioVolume([int]$Expected, [int]$TimeoutMilliseconds = 4000) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        $status = Get-AudioStatus
        if ([Math]::Abs([int]$status.volume_percent-$Expected) -le 1) { return $status }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Endpoint volume did not converge to $Expected percent."
}

function Capture-Screen([string]$Path) {
    $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bitmap = [Drawing.Bitmap]::new($bounds.Width,$bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($bounds.Left,$bounds.Top,0,0,$bitmap.Size)
        $bitmap.Save($Path,[Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
}

function Get-Sha256([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    try {
        $hash = [Security.Cryptography.SHA256]::Create()
        try { ([BitConverter]::ToString($hash.ComputeHash($stream))).Replace('-','').ToLowerInvariant() }
        finally { $hash.Dispose() }
    } finally { $stream.Dispose() }
}

$priorSurface = $env:SUPERDESKTOP_VERIFICATION_SURFACE
$priorTrace = $env:SUPERDESKTOP_ACTION_TRACE
$originalAudio = $null
$process = $null
try {
    $originalAudio = Get-AudioStatus
    $env:SUPERDESKTOP_VERIFICATION_SURFACE = 'taskbar'
    $env:SUPERDESKTOP_ACTION_TRACE = $tracePath
    $process = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms','18000' -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do { Start-Sleep -Milliseconds 100; $process.Refresh() } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Taskbar HWND did not appear.' }
    $taskbarHwnd = [IntPtr]$process.MainWindowHandle
    $taskbar = [System.Windows.Automation.AutomationElement]::FromHandle($taskbarHwnd)
    $volumeButton = Find-Descendant $taskbar { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and ([string]$item.Current.Name).StartsWith('Volume ') }
    if ($null -eq $volumeButton) { throw 'Volume taskbar button is unavailable.' }
    Click-Element $volumeButton
    $popup = Find-VolumePopup $process.Id $taskbarHwnd
    if ($null -eq $popup) { throw 'Volume flyout did not open.' }

    $current = Get-SliderValue $popup.Slider
    $pointerTarget = if ($current -le 80) { [Math]::Min(100,([Math]::Floor($current/10)*10)+10) } else { [Math]::Max(0,([Math]::Ceiling($current/10)*10)-10) }
    if ($pointerTarget -eq $current) { $pointerTarget = if ($current -le 90) {$current+10}else{$current-10} }
    $pointerRestart = Restart-ObserverStatusHost $process.Id
    $popup = Find-VolumePopup $process.Id $taskbarHwnd
    if ($null -eq $popup) { throw 'Volume flyout disappeared before pointer recovery.' }
    $step = Find-Descendant $popup.Root { param($item) [string]$item.Current.Name -eq "Set volume to $pointerTarget percent" }
    Click-Element $step
    $pointerObserved = Wait-AudioVolume $pointerTarget
    Start-Sleep -Milliseconds 700
    Capture-Screen (Join-Path $EvidenceDirectory 'pointer-recovered.png')

    [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    Start-Sleep -Milliseconds 150
    $volumeButton = Find-Descendant $taskbar { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and ([string]$item.Current.Name).StartsWith('Volume ') }
    Click-Element $volumeButton
    $popup = Find-VolumePopup $process.Id $taskbarHwnd
    if ($null -eq $popup) { throw 'Volume flyout did not reopen for keyboard recovery.' }
    $raise = $pointerTarget -le 90
    $keyboardTarget = if ($raise) {$pointerTarget+5}else{$pointerTarget-5}
    $keyboardRestart = Restart-ObserverStatusHost $process.Id
    $popup = Find-VolumePopup $process.Id $taskbarHwnd
    if ($null -eq $popup) { throw 'Volume flyout disappeared before keyboard recovery.' }
    $popupBounds = $popup.Root.Current.BoundingRectangle
    [VolumeRecoveryPointer]::LeftClick([int]($popupBounds.Left+20),[int]($popupBounds.Top+20))
    [VolumeRecoveryPointer]::SetForegroundWindow([IntPtr][int]$popup.Root.Current.NativeWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 100
    [VolumeRecoveryPointer]::SendKey($(if($raise){0x27}else{0x25}))
    $keyboardObserved = Wait-AudioVolume $keyboardTarget
    Start-Sleep -Milliseconds 700
    Capture-Screen (Join-Path $EvidenceDirectory 'keyboard-recovered.png')

    $trace = Get-Content -Raw -LiteralPath $tracePath -Encoding UTF8
    $recoveries = ([regex]::Matches($trace,'status:command-generation-recovered')).Count
    if ($recoveries -lt 2) { throw "Expected two recovered generation races, observed $recoveries." }
    if ($trace -match 'error:status:command:') { throw 'Recovered volume command emitted a status command error.' }
    $stderr = if (Test-Path -LiteralPath $stderrPath) { Get-Content -Raw -LiteralPath $stderrPath } else { '' }
    if ($stderr -match 'SuperDesktop error \[status:command\]') { throw 'Recovered volume command reached console error output.' }

    $report = [ordered]@{
        schema='volume-generation-recovery/v1'
        result='passed'
        app_sha256=Get-Sha256 $appPath
        original_audio=$originalAudio
        pointer=[ordered]@{target=$pointerTarget;observed=$pointerObserved;host_restart=$pointerRestart;physical_pointer=$true}
        keyboard=[ordered]@{target=$keyboardTarget;observed=$keyboardObserved;host_restart=$keyboardRestart;keyboard_arrow=$true}
        recovery_trace_count=$recoveries
        status_command_error_absent=$true
        screenshots=@(
            [ordered]@{name='pointer-recovered.png';sha256=Get-Sha256 (Join-Path $EvidenceDirectory 'pointer-recovered.png')},
            [ordered]@{name='keyboard-recovered.png';sha256=Get-Sha256 (Join-Path $EvidenceDirectory 'keyboard-recovered.png')}
        )
    }
    [IO.File]::WriteAllText((Join-Path $EvidenceDirectory 'headful-report.json'),(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 8
} finally {
    if ($process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    Restore-AudioStatus $originalAudio
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE=$priorTrace }
}
