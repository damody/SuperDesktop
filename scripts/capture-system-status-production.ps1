param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$tracePath = Join-Path $EvidenceDirectory 'system-status-headful.log'

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

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

try {
    $process = Start-Process -FilePath $appPath -ArgumentList '--verification-capture-ms','20000' -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 100
        $process.Refresh()
    } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'SuperDesktop taskbar HWND did not appear.' }
    $taskbar = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $desktopRoot = [System.Windows.Automation.AutomationElement]::RootElement
    $button = [System.Windows.Automation.ControlType]::Button

    $input = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith('Input language ') }
    $network = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith('Network ') }
    $volume = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith('Volume ') }
    $calendar = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name -match '^\d{2}:\d{2} ' }
    $start = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name -eq 'Start' }
    if ($null -eq $input -or $null -eq $network -or $null -eq $volume -or $null -eq $calendar -or $null -eq $start) {
        throw 'One or more owned taskbar status controls are missing.'
    }
    $originalLanguage = ([string]$input.Current.Name).Substring('Input language '.Length)

    Invoke-Element $network
    $networkDialog = Find-Element $desktopRoot { param($item) $item.Current.ProcessId -eq $process.Id -and $item.Current.Name -eq 'Network and power' }
    if ($null -eq $networkDialog) { throw 'Owned network and power flyout did not appear.' }
    Capture-Screen (Join-Path $EvidenceDirectory 'network-power-flyout.png')

    Invoke-Element $input
    $inputDialog = Find-Element $desktopRoot { param($item) $item.Current.ProcessId -eq $process.Id -and $item.Current.Name -eq 'Input languages' }
    if ($null -eq $inputDialog) { throw 'Owned input flyout did not appear.' }
    Capture-Screen (Join-Path $EvidenceDirectory 'input-flyout.png')

    Invoke-Element $volume
    $volumeDialog = Find-Element $desktopRoot { param($item) $item.Current.ProcessId -eq $process.Id -and $item.Current.Name -eq 'Volume' }
    if ($null -eq $volumeDialog) { throw 'Owned volume flyout did not replace the input flyout.' }
    $slider = Find-Element $volumeDialog { param($item) $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::Slider }
    if ($null -eq $slider) { throw 'Owned volume slider is missing.' }
    Capture-Screen (Join-Path $EvidenceDirectory 'volume-flyout.png')

    Invoke-Element $calendar
    $calendarDialog = Find-Element $desktopRoot { param($item) $item.Current.ProcessId -eq $process.Id -and $item.Current.Name -eq 'Calendar' }
    if ($null -eq $calendarDialog) { throw 'Owned calendar flyout did not replace the volume flyout.' }
    Capture-Screen (Join-Path $EvidenceDirectory 'calendar-flyout.png')

    Invoke-Element $start
    $startDialog = Find-Element $desktopRoot { param($item) $item.Current.ProcessId -eq $process.Id -and $item.Current.Name -eq 'Start' }
    if ($null -eq $startDialog) { throw 'Owned Start did not appear before the input switch.' }
    Invoke-Element $input
    $inputDialog = Find-Element $desktopRoot { param($item) $item.Current.ProcessId -eq $process.Id -and $item.Current.Name -eq 'Input languages' }
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
    $startAfterSwitch = Find-Element $desktopRoot { param($item) $item.Current.ProcessId -eq $process.Id -and $item.Current.Name -eq 'Start' }
    if ($null -eq $startAfterSwitch) { throw 'Owned Start was lost during the input profile switch.' }
    Capture-Screen (Join-Path $EvidenceDirectory 'start-after-input-switch.png')

    $input = Find-Element $taskbar { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith('Input language ') }
    Invoke-Element $input
    $inputDialog = Find-Element $desktopRoot { param($item) $item.Current.ProcessId -eq $process.Id -and $item.Current.Name -eq 'Input languages' }
    $restore = Find-Element $inputDialog { param($item) $item.Current.ControlType -eq $button -and $item.Current.Name.StartsWith($originalLanguage) }
    Invoke-Element $restore
    Start-Sleep -Milliseconds 750

    $process.WaitForExit()
    $trace = Get-Content -Raw -Encoding UTF8 -LiteralPath $tracePath
    if ($trace -notmatch 'status:owned-flyout-opened' -or $trace -notmatch 'start:ime-focus-restored') {
        throw 'Headful trace does not prove owned flyout composition and Start focus restoration.'
    }
    $screenshots = Get-ChildItem -LiteralPath $EvidenceDirectory -Filter '*.png' | ForEach-Object {
        [ordered]@{ name=$_.Name;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant();bytes=$_.Length }
    }
    $report = [ordered]@{
        schema='system-status-headful/v1'
        result='passed'
        app_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $appPath).Hash.ToLowerInvariant()
        original_input_profile=$originalLanguage
        switched_input_profile=$alternateName
        original_profile_restored=$true
        start_survived_switch=$true
        start_focus_restored_trace=$true
        owned_flyouts=@('network-power','input','volume','calendar')
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
    if ($null -eq $priorSurface) { Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_VERIFICATION_SURFACE = $priorSurface }
    if ($null -eq $priorTrace) { Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue } else { $env:SUPERDESKTOP_ACTION_TRACE = $priorTrace }
}
