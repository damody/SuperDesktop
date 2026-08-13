[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$ManifestPath)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
$manifest=if($ManifestPath){$ManifestPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-inputs-successor-1.2.sha256'}
if(-not(Test-Path -LiteralPath $manifest -PathType Leaf)){throw 'CURRENT_SUBSTRATE_MANIFEST_MISSING'}
foreach($line in Get-Content -LiteralPath $manifest){if($line -notmatch '^([A-F0-9]{64})  ([a-zA-Z0-9._/-]+)$'){throw "CURRENT_SUBSTRATE_MANIFEST_MALFORMED: $line"};$path=$matches[2];if($path.Contains('..') -or $path.StartsWith('/')){throw "CURRENT_SUBSTRATE_PATH_REJECTED: $path"};$full=Join-Path $WorkspaceRoot $path;if(-not(Test-Path -LiteralPath $full -PathType Leaf)){throw "CURRENT_SUBSTRATE_INPUT_MISSING: $path"};if((Get-FileHash -Algorithm SHA256 -LiteralPath $full).Hash -ne $matches[1]){throw "CURRENT_SUBSTRATE_HASH_DRIFT: $path"}}
$lock=Get-Content -Raw -Encoding utf8 (Join-Path $WorkspaceRoot 'Cargo.lock')
if($lock -notmatch 'name = "gpui_windows"' -or $lock -notmatch 'name = "raw-window-handle"\r?\nversion = "0\.6\.2"'){throw 'CURRENT_SUBSTRATE_PROVENANCE_MISSING'}
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $WorkspaceRoot 'scripts/check-dependency-architecture.ps1') -WorkspaceRoot $WorkspaceRoot | Out-Null
if($LASTEXITCODE -ne 0){throw 'CURRENT_SUBSTRATE_ARCHITECTURE_FAILED'}
Write-Output 'Current substrate contract passed: root manifest/config/lock, desktop composition, allowlist, and vendor provenance match.'
