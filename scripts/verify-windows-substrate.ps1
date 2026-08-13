[CmdletBinding()]
param([string]$WorkspaceRoot)
$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }
$manifest = Get-Content -Raw (Join-Path $WorkspaceRoot 'Cargo.toml')
$lock = Get-Content -Raw (Join-Path $WorkspaceRoot 'Cargo.lock')
$platform = Get-Content -Raw (Join-Path $WorkspaceRoot 'crates/platform-win/Cargo.toml')
if ($manifest -notmatch 'windows\s*=\s*\{\s*version\s*=\s*"=0\.62\.2"') { throw 'WINDOWS_SUBSTRATE_VERSION_INVALID' }
foreach ($feature in @('Win32_Foundation','Win32_UI_WindowsAndMessaging','Win32_UI_Shell','Win32_System_Threading','Win32_System_ProcessStatus','Win32_System_Console','Win32_System_SystemInformation','Win32_Security','Win32_Storage_FileSystem','Win32_Graphics_Gdi')) { if ($manifest -notmatch [regex]::Escape('"' + $feature + '"')) { throw "WINDOWS_SUBSTRATE_FEATURE_MISSING: $feature" } }
if ($lock -notmatch 'name = "windows"\s+version = "0\.62\.2"') { throw 'WINDOWS_SUBSTRATE_LOCK_INVALID' }
if ($platform -notmatch 'windows\.workspace = true' -or $platform -notmatch 'unsafe_code = "allow"') { throw 'PLATFORM_UNSAFE_EXCEPTION_INVALID' }
$otherUnsafe = Get-ChildItem (Join-Path $WorkspaceRoot 'crates') -Recurse -Filter Cargo.toml | Where-Object { $_.FullName -notmatch '[\\/]platform-win[\\/]' } | Where-Object { (Get-Content -Raw $_.FullName) -match 'unsafe_code\s*=\s*"allow"' }
if ($otherUnsafe) { throw "UNSAFE_EXCEPTION_OUTSIDE_PLATFORM: $($otherUnsafe.FullName -join ',')" }
$cargoHome = Join-Path $WorkspaceRoot 'build/offline-cargo-home-2.5'
New-Item -ItemType Directory -Force $cargoHome | Out-Null
$old = $env:CARGO_HOME; $env:CARGO_HOME = $cargoHome
try { & cargo check -p platform-win --locked --offline; if ($LASTEXITCODE -ne 0) { throw 'WINDOWS_SUBSTRATE_OFFLINE_CHECK_FAILED' } } finally { $env:CARGO_HOME = $old }
Write-Output 'Windows 0.62.2 substrate/features, bounded platform unsafe exception, lock/vendor and isolated offline check passed.'
