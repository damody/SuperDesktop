[CmdletBinding()]
param(
    [string]$Workspace,
    [Parameter(Mandatory = $true)]
    [string]$ConfirmationPath,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$utf8 = [Text.UTF8Encoding]::new($false)
if (-not $Workspace) { $Workspace = Split-Path -Parent $PSScriptRoot }
if (-not $OutputPath) { $OutputPath = Join-Path $Workspace 'openspec/changes/verify-superdesktop-m0/evidence/artifacts/2.4/physical-mixed-dpi-gate.json' }

function Write-Json([string]$Path, $Value) {
    New-Item -ItemType Directory -Force (Split-Path -Parent $Path) | Out-Null
    [IO.File]::WriteAllText($Path, (($Value | ConvertTo-Json -Depth 30) + "`n"), $utf8)
}
function Invoke-Checked([string]$Name, [scriptblock]$Command) {
    $prior = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    try { $text = (& $Command 2>&1 | Out-String); $exitCode = $LASTEXITCODE } finally { $ErrorActionPreference = $prior }
    if ($exitCode -ne 0) { throw "$Name failed with exit code $exitCode`n$text" }
    return $text.Trim()
}
function Decode-WmiString($Value) {
    return -join @($Value | Where-Object { $_ -ne 0 } | ForEach-Object { [char]$_ })
}
function Monitor-Profile([string]$ProbePath) {
    $text = Invoke-Checked 'monitor/DPI profile' { & $ProbePath }
    $line = $text -split "`r?`n" | Where-Object { $_.StartsWith('{') } | Select-Object -Last 1
    if (-not $line) { throw 'monitor/DPI profile did not emit JSON' }
    return $line | ConvertFrom-Json
}

$Workspace = (Resolve-Path -LiteralPath $Workspace).Path
$ConfirmationPath = (Resolve-Path -LiteralPath $ConfirmationPath).Path
function Get-RepositoryRelativePath([string]$Path, [string]$Label) {
    $full = [IO.Path]::GetFullPath($Path)
    $prefix = $Workspace.TrimEnd('\') + '\'
    if (-not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw "$Label must be stored inside the repository." }
    return $full.Substring($prefix.Length).Replace('\','/')
}
$confirmationRelativePath = Get-RepositoryRelativePath $ConfirmationPath 'Physical confirmation'
$candidatePath = Join-Path $Workspace 'openspec/changes/verify-superdesktop-shell-completion/evidence/release-candidate.json'
$candidate = Get-Content -Raw -Encoding utf8 -LiteralPath $candidatePath | ConvertFrom-Json
$reviewedRevision = [string]$candidate.reviewed_revision
& git -C $Workspace cat-file -e "$reviewedRevision^{commit}"
if ($candidate.schema_version -ne 1 -or $LASTEXITCODE -ne 0) { throw 'Unable to bind frozen release-candidate revision.' }
& git -C $Workspace merge-base --is-ancestor $reviewedRevision HEAD
if ($LASTEXITCODE -ne 0) { throw 'Current checkout does not descend from the frozen release candidate.' }
& git -C $Workspace diff --quiet $reviewedRevision HEAD -- crates Cargo.toml Cargo.lock
if ($LASTEXITCODE -ne 0) { throw 'Committed production source or dependency drift exists relative to the frozen candidate.' }
& git -C $Workspace diff --quiet -- crates Cargo.toml Cargo.lock
if ($LASTEXITCODE -ne 0) { throw 'Uncommitted production source or dependency drift exists relative to the frozen candidate.' }
& git -C $Workspace diff --cached --quiet -- crates Cargo.toml Cargo.lock
if ($LASTEXITCODE -ne 0) { throw 'Staged production source or dependency drift exists relative to the frozen candidate.' }
if (-not [Environment]::UserInteractive -or (Get-Process -Id $PID).SessionId -eq 0) { throw 'An interactive non-session-0 desktop is required' }
$null = Invoke-Checked 'release application build' { cargo build -p superdesktop-app --release --offline --manifest-path (Join-Path $Workspace 'Cargo.toml') }
$null = Invoke-Checked 'release monitor/DPI probe build' { cargo build -p platform-win --release --offline --example monitor_dpi_start_capability --manifest-path (Join-Path $Workspace 'Cargo.toml') }
$wmiMonitors = @(Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID | Where-Object Active)
if ($wmiMonitors.Count -lt 2) { throw "At least two active physical WMI monitors are required; observed $($wmiMonitors.Count)" }

$app = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$probe = Join-Path $Workspace 'target/release/examples/monitor_dpi_start_capability.exe'
foreach ($path in @($app, $probe)) { if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing release artifact: $path" } }
$profileBefore = Monitor-Profile $probe
$realMonitors = @($profileBefore.real_profile.monitors)
if ($realMonitors.Count -lt 2 -or @($realMonitors.dpi_x | Sort-Object -Unique).Count -lt 2) { throw 'Two real monitors with distinct effective DPI values are required' }

$confirmation = Get-Content -Raw -Encoding UTF8 $ConfirmationPath | ConvertFrom-Json
$requiredChecks = @('cross_monitor_pointer','cross_monitor_keyboard_focus','cross_monitor_drag','primary_change','hot_plug_recovery')
if ($confirmation.schema -cne 'm0-physical-mixed-dpi-confirmation/v1' -or $confirmation.reviewed_revision -cne $reviewedRevision) { throw 'Physical confirmation is invalid or stale.' }
try { [DateTimeOffset]::Parse($confirmation.recorded_at_utc) | Out-Null } catch { throw 'recorded_at_utc must be ISO-8601.' }
if ([string]::IsNullOrWhiteSpace($confirmation.reviewer.name) -or [string]::IsNullOrWhiteSpace($confirmation.reviewer.organization) -or
    [string]$confirmation.reviewer.name -like 'REPLACE_WITH_*' -or [string]$confirmation.reviewer.organization -like 'REPLACE_WITH_*') { throw 'Manual confirmation requires an attributable reviewer.' }
foreach ($name in $requiredChecks) { if ($confirmation.$name -ne 'passed') { throw "Manual confirmation check $name is not passed" } }
if (@($confirmation.photos).Count -lt 2) { throw 'At least two physical topology photos are required' }
$photos = @()
foreach ($photo in @($confirmation.photos)) {
    $path = (Resolve-Path -LiteralPath $photo).Path
    Get-RepositoryRelativePath $path 'Physical topology photo' | Out-Null
    $photos += [ordered]@{ path = $path; sha256 = (Get-FileHash $path -Algorithm SHA256).Hash }
}

Add-Type -AssemblyName System.Drawing
$captureRoot = Join-Path (Split-Path -Parent $OutputPath) 'physical-mixed-dpi-captures'
New-Item -ItemType Directory -Force $captureRoot | Out-Null
$process = Start-Process -FilePath $app -ArgumentList '--verification-capture-ms','5000','--shell' -PassThru
Start-Sleep -Milliseconds 1500
$screenshots = @()
try {
    foreach ($monitor in $realMonitors) {
        $width = [int]$monitor.bounds.right - [int]$monitor.bounds.left
        $height = [int]$monitor.bounds.bottom - [int]$monitor.bounds.top
        $safeName = ([string]$monitor.device_name -replace '[^A-Za-z0-9.-]', '_')
        $path = Join-Path $captureRoot "$safeName.png"
        $bitmap = [Drawing.Bitmap]::new($width, $height)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try { $graphics.CopyFromScreen([int]$monitor.bounds.left, [int]$monitor.bounds.top, 0, 0, $bitmap.Size); $bitmap.Save($path, [Drawing.Imaging.ImageFormat]::Png) } finally { $graphics.Dispose(); $bitmap.Dispose() }
        $screenshots += [ordered]@{ monitor = $monitor.device_name; path = $path; sha256 = (Get-FileHash $path -Algorithm SHA256).Hash }
    }
} finally {
    $process.WaitForExit()
}
if ($process.ExitCode -ne 0) { throw "Multi-monitor Shell capture exited with $($process.ExitCode)" }

$inputPath = Join-Path $env:TEMP "superdesktop-physical-input-$PID.json"
try {
    Invoke-Checked 'physical input routes' { powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Workspace 'scripts/verify-m0-input-routes.ps1') -Workspace $Workspace -OutputPath $inputPath } | Out-Null
    $inputRoutes = Get-Content -Raw -Encoding UTF8 $inputPath | ConvertFrom-Json
    if (-not $inputRoutes.all_passed -or $inputRoutes.route_count -ne 6) { throw 'Physical input routes did not pass' }
} finally { Remove-Item -LiteralPath $inputPath -ErrorAction SilentlyContinue }

$profileAfter = Monitor-Profile $probe
if (($profileBefore.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress) -ne ($profileAfter.real_profile.monitors | ConvertTo-Json -Depth 20 -Compress)) { throw 'Monitor/work-area baseline was not restored after capture' }
$drivers = @(Get-CimInstance Win32_PnPSignedDriver | Where-Object DeviceClass -eq 'MONITOR' | ForEach-Object { [ordered]@{ device_name=$_.DeviceName;device_id=$_.DeviceID;driver_version=$_.DriverVersion;driver_provider=$_.DriverProviderName } })
$identities = @($wmiMonitors | ForEach-Object { [ordered]@{ instance_name=$_.InstanceName;manufacturer=(Decode-WmiString $_.ManufacturerName);product=(Decode-WmiString $_.ProductCodeID);serial=(Decode-WmiString $_.SerialNumberID) } })
$artifact = [ordered]@{
    schema = 'm0-physical-mixed-dpi-gate/v1'; status = 'passed'; recorded_at = [DateTime]::UtcNow.ToString('o'); revision = $reviewedRevision.Substring(0, 8); app_sha256 = (Get-FileHash $app -Algorithm SHA256).Hash
    physical_identities = $identities; monitor_geometry = $realMonitors; drivers = $drivers; screenshots = $screenshots; photos = $photos
    interactions = [ordered]@{ pointer=$confirmation.cross_monitor_pointer;keyboard_focus=$confirmation.cross_monitor_keyboard_focus;drag=$confirmation.cross_monitor_drag;input_route_count=$inputRoutes.route_count }
    topology = [ordered]@{ primary_change=$confirmation.primary_change;hot_plug_recovery=$confirmation.hot_plug_recovery;work_area_restored=$true;distinct_dpi_count=@($realMonitors.dpi_x|Sort-Object -Unique).Count }
    reviewer = $confirmation.reviewer; confirmation_path = $confirmationRelativePath; confirmation_sha256 = (Get-FileHash $ConfirmationPath -Algorithm SHA256).Hash; dispositions = @{ 'G-DPI-MONITOR'='passed' }; task_ids = @('2.4.1','2.4.2','2.4.3','2.4.4','2.4.5')
}
Write-Json $OutputPath $artifact
Write-Output "Physical mixed-DPI M0 evidence captured at $OutputPath"
