[CmdletBinding()]
param([string]$WorkspaceRoot)
$ErrorActionPreference='Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path }
$allowlist=Join-Path $PSScriptRoot 'profile-allowlist.json'
. (Join-Path $PSScriptRoot 'validate-profile-snapshot.ps1')
$isolated=Join-Path $WorkspaceRoot 'build/profile-allowlist-fixture'
New-Item -ItemType Directory -Force $isolated | Out-Null
$policyCopy=Join-Path $isolated 'profile-allowlist.json'; Copy-Item -LiteralPath $allowlist -Destination $policyCopy -Force
$snapshot=Get-ProfileSnapshot -AllowlistPath $allowlist
$snapshotPath=Join-Path $isolated 'actual-snapshot.json'; $snapshot | ConvertTo-Json -Depth 12 | Set-Content -Encoding utf8 $snapshotPath
$baseline=Get-Content -Raw -Encoding utf8 $snapshotPath | ConvertFrom-Json; Assert-ProfileSnapshot -AllowlistPath $policyCopy -Snapshot $baseline
$mutated=Get-Content -Raw -Encoding utf8 $snapshotPath | ConvertFrom-Json; $mutated.keys.'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced'.values.TaskbarAl='99'
if (-not (Test-ProfileSnapshotDiagnostic -AllowlistPath $policyCopy -Snapshot $mutated -ExpectedDiagnostic 'PROFILE_VALUE_DRIFT:')) { throw 'ALLOWLIST_MUTATED_TASKBAR_VALUE_ADMITTED' }
$unknown=Get-Content -Raw -Encoding utf8 $snapshotPath | ConvertFrom-Json; $unknown.keys.'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced'.values | Add-Member -NotePropertyName StartFutureLayout -NotePropertyValue '1'
if (-not (Test-ProfileSnapshotDiagnostic -AllowlistPath $policyCopy -Snapshot $unknown -ExpectedDiagnostic 'PROFILE_UNKNOWN_IMPORTANT_VALUE:')) { throw 'ALLOWLIST_UNKNOWN_START_VALUE_ADMITTED' }
Write-Output 'Profile allowlist negative fixtures passed through production validator diagnostics.'
