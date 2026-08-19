param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [ValidateSet('empty', 'populated')][string]$Mode = 'empty',
    [ValidateSet('light', 'dark', 'high-contrast')][string]$Theme = 'light'
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/examples/taskbar_show_all_headful.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing show-all headful example: $appPath" }
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

function Find-Elements([int]$ProcessId) {
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
        $ProcessId
    )
    [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition
    )
}

function Wait-NamedElement([int]$ProcessId, [string]$Name) {
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do {
        $elements = Find-Elements -ProcessId $ProcessId
        for ($index = 0; $index -lt $elements.Count; $index++) {
            try {
                if ($elements.Item($index).Current.Name -eq $Name) { return $elements.Item($index) }
            } catch [System.Windows.Automation.ElementNotAvailableException] {}
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "UIA element was not found: $Name"
}

function Capture-ProcessWindows([int]$ProcessId, [string]$Path) {
    $elements = Find-Elements -ProcessId $ProcessId
    $rectangles = @()
    for ($index = 0; $index -lt $elements.Count; $index++) {
        try {
            $element = $elements.Item($index)
            if ($element.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window) {
                $rect = $element.Current.BoundingRectangle
                if ($rect.Width -gt 1 -and $rect.Height -gt 1) { $rectangles += $rect }
            }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
    }
    if ($rectangles.Count -eq 0) { throw 'No process window bounds were available.' }
    $left = ($rectangles | Measure-Object Left -Minimum).Minimum
    $top = ($rectangles | Measure-Object Top -Minimum).Minimum
    $right = ($rectangles | ForEach-Object { $_.Right } | Measure-Object -Maximum).Maximum
    $bottom = ($rectangles | ForEach-Object { $_.Bottom } | Measure-Object -Maximum).Maximum
    $virtual = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bounds = [Drawing.Rectangle]::Intersect(
        [Drawing.Rectangle]::FromLTRB([int]$left, [int]$top, [int][Math]::Ceiling($right), [int][Math]::Ceiling($bottom)),
        $virtual
    )
    $bitmap = [Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bitmap.Size)
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
    $bounds
}

$oldTheme = $env:SUPERDESKTOP_THEME
$oldLocale = $env:SUPERDESKTOP_LOCALE
$env:SUPERDESKTOP_THEME = $Theme
$env:SUPERDESKTOP_LOCALE = 'en-US'
$process = Start-Process -FilePath $appPath -ArgumentList '--mode', $Mode, '--hold-ms', '30000' -PassThru
try {
    $chevron = Wait-NamedElement -ProcessId ([int]$process.Id) -Name 'Show all tray icons'
    $chevronBounds = $chevron.Current.BoundingRectangle
    $invoke = $null
    if (-not $chevron.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$invoke)) {
        throw 'Show-all chevron does not expose InvokePattern.'
    }
    ([System.Windows.Automation.InvokePattern]$invoke).Invoke()
    $popup = Wait-NamedElement -ProcessId ([int]$process.Id) -Name 'Tray icons'
    Start-Sleep -Milliseconds 200
    $elements = Find-Elements -ProcessId ([int]$process.Id)
    $fixtureNames = @()
    $emptyState = $false
    for ($index = 0; $index -lt $elements.Count; $index++) {
        try {
            $name = [string]$elements.Item($index).Current.Name
            if ($name -like 'Fixture tray icon *') { $fixtureNames += $name }
            if ($name -eq 'No tray icons are currently registered') { $emptyState = $true }
        } catch [System.Windows.Automation.ElementNotAvailableException] {}
    }
    if ($Mode -eq 'empty' -and -not $emptyState) { throw 'Empty show-all popup did not expose its truthful empty state.' }
    if ($Mode -eq 'populated' -and (@($fixtureNames | Sort-Object -Unique).Count -ne 3)) {
        throw "Populated show-all popup did not expose all three icons: $($fixtureNames -join ', ')"
    }
    $captureName = "$Theme-$Mode.png"
    $captureBounds = Capture-ProcessWindows -ProcessId ([int]$process.Id) -Path (Join-Path $EvidenceDirectory $captureName)
    $report = [ordered]@{
        schema = 'taskbar-show-all-tray-icons-headful/v1'
        result = 'passed'
        mode = $Mode
        theme = $Theme
        chevron = @{ control_type = $chevron.Current.ControlType.ProgrammaticName; invoke_pattern = $true; bounds = $chevronBounds.ToString() }
        popup = @{ name = $popup.Current.Name; empty_state = $emptyState; fixture_icons = @($fixtureNames | Sort-Object -Unique) }
        capture = $captureName
        capture_bounds = $captureBounds.ToString()
    }
    [IO.File]::WriteAllText((Join-Path $EvidenceDirectory "$Theme-$Mode-report.json"), (($report | ConvertTo-Json -Depth 6) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 6
} finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    $env:SUPERDESKTOP_THEME = $oldTheme
    $env:SUPERDESKTOP_LOCALE = $oldLocale
}
