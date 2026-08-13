[CmdletBinding()]
param([string]$WorkspaceRoot)
$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }
$manifest = Get-Content -Raw (Join-Path $WorkspaceRoot 'Cargo.toml')
$lock = Get-Content -Raw (Join-Path $WorkspaceRoot 'Cargo.lock')
$platform = Get-Content -Raw (Join-Path $WorkspaceRoot 'crates/platform-win/Cargo.toml')
$changeRoot = Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace'
$inventory = Get-Content -Raw (Join-Path $changeRoot 'compliance/third-party-license-inventory.json') | ConvertFrom-Json
$provenance = Get-Content -Raw (Join-Path $changeRoot 'evidence/artifacts/1.2/dependency-provenance.json') | ConvertFrom-Json
if ($manifest -notmatch 'windows\s*=\s*\{\s*version\s*=\s*"=0\.62\.2"') { throw 'WINDOWS_SUBSTRATE_VERSION_INVALID' }
foreach ($feature in @('Win32_Foundation','Win32_UI_WindowsAndMessaging','Win32_UI_Shell','Win32_System_Threading','Win32_System_ProcessStatus','Win32_System_Console','Win32_System_SystemInformation','Win32_Security','Win32_Storage_FileSystem','Win32_Graphics_Gdi')) { if ($manifest -notmatch [regex]::Escape('"' + $feature + '"')) { throw "WINDOWS_SUBSTRATE_FEATURE_MISSING: $feature" } }
if ($lock -notmatch 'name = "windows"\s+version = "0\.62\.2"') { throw 'WINDOWS_SUBSTRATE_LOCK_INVALID' }
if ($platform -notmatch 'windows\.workspace = true' -or $platform -notmatch 'unsafe_code = "allow"') { throw 'PLATFORM_UNSAFE_EXCEPTION_INVALID' }
$requiredPins = @{
  'windows' = '0.62.2'
  'jsonschema' = '0.37.1'
  'serde_json' = '1.0.151'
  'gpui' = '0.2.2'
}
foreach ($pin in $requiredPins.GetEnumerator()) {
  $inventoryRecord = @($inventory.packages | Where-Object { $_.name -eq $pin.Key -and $_.version -eq $pin.Value })
  if ($inventoryRecord.Count -ne 1 -or [string]::IsNullOrWhiteSpace([string]$inventoryRecord[0].license) -or [string]::IsNullOrWhiteSpace([string]$inventoryRecord[0].vendor_checksum_sha256)) { throw "WINDOWS_SUBSTRATE_LICENSE_COVERAGE: $($pin.Key) $($pin.Value)" }
  $provenanceRecord = @($provenance.dependencies | Where-Object { $_.name -eq $pin.Key -and $_.version -eq $pin.Value })
  if ($provenanceRecord.Count -ne 1) { throw "WINDOWS_SUBSTRATE_PROVENANCE_COVERAGE: $($pin.Key) $($pin.Value)" }
}
$oldWindows = @($inventory.packages | Where-Object { $_.name -eq 'windows' -and $_.version -eq '0.61.3' })
if ($oldWindows.Count -ne 1 -or [string]::IsNullOrWhiteSpace([string]$oldWindows[0].vendor_checksum_sha256)) { throw 'WINDOWS_SUBSTRATE_TRANSITIVE_LICENSE_COVERAGE: windows 0.61.3' }
$otherUnsafe = Get-ChildItem (Join-Path $WorkspaceRoot 'crates') -Recurse -Filter Cargo.toml | Where-Object { $_.FullName -notmatch '[\\/]platform-win[\\/]' } | Where-Object { (Get-Content -Raw $_.FullName) -match 'unsafe_code\s*=\s*"allow"' }
if ($otherUnsafe) { throw "UNSAFE_EXCEPTION_OUTSIDE_PLATFORM: $($otherUnsafe.FullName -join ',')" }
$cargoHome = Join-Path $WorkspaceRoot 'build/offline-cargo-home-2.5'
New-Item -ItemType Directory -Force $cargoHome | Out-Null
$old = $env:CARGO_HOME; $env:CARGO_HOME = $cargoHome
try { & cargo check -p platform-win --locked --offline; if ($LASTEXITCODE -ne 0) { throw 'WINDOWS_SUBSTRATE_OFFLINE_CHECK_FAILED' } } finally { $env:CARGO_HOME = $old }
Write-Output 'Windows 0.62.2 substrate/features, direct-pin provenance/license coverage, 0.61.3 transitive coverage, bounded platform unsafe exception, lock/vendor and isolated offline check passed.'
