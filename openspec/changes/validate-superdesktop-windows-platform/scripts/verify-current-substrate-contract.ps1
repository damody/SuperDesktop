[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$ManifestPath)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
$manifest=if($ManifestPath){$ManifestPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-inputs-successor-1.2-manifest-v2.sha256'}
if(-not(Test-Path -LiteralPath $manifest -PathType Leaf)){throw 'CURRENT_SUBSTRATE_MANIFEST_MISSING'}
foreach($line in Get-Content -LiteralPath $manifest){if($line -notmatch '^([A-F0-9]{64})  ([a-zA-Z0-9._/-]+)$'){throw "CURRENT_SUBSTRATE_MANIFEST_MALFORMED: $line"};$path=$matches[2];if($path.Contains('..') -or $path.StartsWith('/')){throw "CURRENT_SUBSTRATE_PATH_REJECTED: $path"};$full=Join-Path $WorkspaceRoot $path;if(-not(Test-Path -LiteralPath $full -PathType Leaf)){throw "CURRENT_SUBSTRATE_INPUT_MISSING: $path"};if((Get-FileHash -Algorithm SHA256 -LiteralPath $full).Hash -ne $matches[1]){throw "CURRENT_SUBSTRATE_HASH_DRIFT: $path"}}
$lock=Get-Content -Raw -Encoding utf8 (Join-Path $WorkspaceRoot 'Cargo.lock')
if($lock -notmatch 'name = "gpui_windows"' -or $lock -notmatch 'name = "raw-window-handle"\r?\nversion = "0\.6\.2"'){throw 'CURRENT_SUBSTRATE_PROVENANCE_MISSING'}
$requiredPackages=@(
  @('embed-resource','3.0.11'),@('serde_spanned','1.1.1'),@('toml','1.1.4\+spec-1.1.0'),@('toml_datetime','1.1.1\+spec-1.1.0'),@('toml_parser','1.1.3\+spec-1.1.0'),@('toml_writer','1.1.2\+spec-1.1.0'),@('vswhom','0.1.0'),@('vswhom-sys','0.1.3'),@('winnow','1.0.4'),@('winreg','0.55.0')
)
foreach($package in $requiredPackages){if($lock -notmatch ('name = "'+$package[0]+'"\r?\nversion = "'+$package[1]+'"')){throw "CURRENT_SUBSTRATE_PACKAGE_PROVENANCE_MISSING: $($package[0])"}}
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $WorkspaceRoot 'scripts/check-dependency-architecture.ps1') -WorkspaceRoot $WorkspaceRoot | Out-Null
if($LASTEXITCODE -ne 0){throw 'CURRENT_SUBSTRATE_ARCHITECTURE_FAILED'}
Write-Output 'Current substrate v2 contract passed: root manifest/config/lock, desktop composition, allowlist, and all pinned vendor/license provenance match.'
