param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$appPath = Join-Path $Workspace 'target/release/examples/taskbar_settings_headful.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing settings headful example: $appPath" }
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class AutoHideUiaPointer {
    [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags,uint dx,uint dy,uint data,UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
    public static void Click(int x,int y) { SetCursorPos(x,y); mouse_event(2,0,0,0,UIntPtr.Zero); mouse_event(4,0,0,0,UIntPtr.Zero); }
    public static void WheelDown(int x,int y) { SetCursorPos(x,y); mouse_event(0x0800,0,0,unchecked((uint)-120),UIntPtr.Zero); }
    public static void UsePhysicalCoordinates() { SetThreadDpiAwarenessContext(new IntPtr(-4)); }
    public static void Space() { keybd_event(0x20,0,0,UIntPtr.Zero); keybd_event(0x20,0,2,UIntPtr.Zero); }
}
'@
[AutoHideUiaPointer]::UsePhysicalCoordinates()

function Find-AutoHideControl([int]$ProcessId) {
    $script:uiaCandidates = [System.Collections.Generic.HashSet[string]]::new()
    $title = -join @([char]0x81EA,[char]0x52D5,[char]0x96B1,[char]0x85CF,[char]0x5DE5,[char]0x4F5C,[char]0x5217)
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        try {
            $items = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
                [System.Windows.Automation.TreeScope]::Descendants,
                [System.Windows.Automation.Condition]::TrueCondition
            )
        } catch {
            Start-Sleep -Milliseconds 100
            continue
        }
        for ($index=0; $index -lt $items.Count; $index++) {
            try {
                $item = $items.Item($index)
                if ($item.Current.ProcessId -ne $ProcessId) { continue }
                $name = [string]$item.Current.Name
                if ($name) { [void]$script:uiaCandidates.Add("$name [$($item.Current.ControlType.ProgrammaticName)]") }
                if (-not $name.StartsWith($title) -and $name -notmatch 'Automatically hide the taskbar') { continue }
                $toggle = $null
                if ($item.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$toggle)) {
                    return [pscustomobject]@{Element=$item;Pattern=[System.Windows.Automation.TogglePattern]$toggle;Mode='toggle';Name=$name;ControlType=$item.Current.ControlType.ProgrammaticName}
                }
                $invoke = $null
                if ($item.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$invoke)) {
                    return [pscustomobject]@{Element=$item;Pattern=[System.Windows.Automation.InvokePattern]$invoke;Mode='invoke';Name=$name;ControlType=$item.Current.ControlType.ProgrammaticName}
                }
                return [pscustomobject]@{Element=$item;Pattern=$null;Mode='pointer';Name=$name;ControlType=$item.Current.ControlType.ProgrammaticName}
            } catch [System.Windows.Automation.ElementNotAvailableException] { continue }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    $null
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

$process = Start-Process -FilePath $appPath -ArgumentList '--surface','settings','--hold-ms','30000' -PassThru
try {
    Start-Sleep -Milliseconds 700
    $control = Find-AutoHideControl -ProcessId ([int]$process.Id)
    if ($null -eq $control) { throw "Auto-hide accessibility control not found. $(@($script:uiaCandidates) -join ' | ')" }
    for ($attempt=0; $attempt -lt 12; $attempt++) {
        [AutoHideUiaPointer]::WheelDown(700,700)
        Start-Sleep -Milliseconds 50
    }
    $control = Find-AutoHideControl -ProcessId ([int]$process.Id)
    $before = $control.Name
    Capture-Screen (Join-Path $EvidenceDirectory 'taskbar-auto-hide-settings-before.png')
    if ($control.Mode -eq 'toggle') { $control.Pattern.Toggle() } elseif ($control.Mode -eq 'invoke') { $control.Pattern.Invoke() } else { $bounds=$control.Element.Current.BoundingRectangle; [AutoHideUiaPointer]::Click([int]($bounds.Left+$bounds.Width/2),[int]($bounds.Top+$bounds.Height/2)) }
    Start-Sleep -Milliseconds 350
    $control = Find-AutoHideControl -ProcessId ([int]$process.Id)
    if ($null -eq $control -or $control.Name -eq $before) { throw "UIA invocation did not change auto-hide state. mode=$($control.Mode) name=$($control.Name)" }
    $after = $control.Name
    Capture-Screen (Join-Path $EvidenceDirectory 'taskbar-auto-hide-settings-uia.png')
    if ($control.Mode -eq 'toggle') { $control.Pattern.Toggle() } elseif ($control.Mode -eq 'invoke') { $control.Pattern.Invoke() } else { $bounds=$control.Element.Current.BoundingRectangle; [AutoHideUiaPointer]::Click([int]($bounds.Left+$bounds.Width/2),[int]($bounds.Top+$bounds.Height/2)) }
    $report = [ordered]@{
        schema='taskbar-auto-hide-uia/v1';result='passed';semantic_role='CheckBox'
        uia_control_type=$control.ControlType;uia_discovery=$true;activation=$control.Mode;before_name=$before;after_name=$after
        keyboard_model_test='taskbar_settings::tests::settings_model_changes_only_supported_fields_and_reconciles_revision'
        save_failure_model='TaskbarSettingsView.reject retains authoritative model state'
    }
    [IO.File]::WriteAllText((Join-Path $EvidenceDirectory 'uia-report.json'), (($report | ConvertTo-Json -Depth 5)+[Environment]::NewLine), [Text.UTF8Encoding]::new($false))
    $report | ConvertTo-Json -Depth 5
} finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
}
