[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$OutputPath)

$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
if(-not $OutputPath){$OutputPath=Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/2.1/appbar-shell-hook-trace.json'}
$change=Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform'
$manifest=Join-Path $change 'evidence/artifacts/1.1/current-substrate-inputs-successor-1.2-manifest-v3.sha256'
$probe=Join-Path $change 'evidence/artifacts/1.1/bin/capability_profile-successor-1.2-manifest-v3.exe'
$admission=Join-Path (Split-Path $OutputPath -Parent) 'pre-mutation-admission-trace.json'

& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $change 'scripts/verify-current-substrate-contract.ps1') -WorkspaceRoot $WorkspaceRoot -ManifestPath $manifest
if($LASTEXITCODE -ne 0){throw 'CURRENT_SUBSTRATE_GATE_FAILED'}
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $change 'scripts/capture-profile-admission.ps1') -WorkspaceRoot $WorkspaceRoot -ProbePath $probe -OutputPath $admission
if($LASTEXITCODE -ne 0){throw 'PRE_MUTATION_ADMISSION_GATE_FAILED'}
Push-Location $WorkspaceRoot
try {
  cargo build -p platform-win --example appbar_shell_hook_capability --locked --offline
  if($LASTEXITCODE -ne 0){throw 'APPBAR_SHELL_HOOK_BUILD_FAILED'}
  $binary=Join-Path $WorkspaceRoot 'target/debug/examples/appbar_shell_hook_capability.exe'
  if(-not(Test-Path -LiteralPath $binary -PathType Leaf)){throw 'APPBAR_SHELL_HOOK_BINARY_MISSING'}
  $raw=& $binary
  $exit=$LASTEXITCODE
  if($exit -ne 0){throw "APPBAR_SHELL_HOOK_RUN_FAILED:$raw"}
  New-Item -ItemType Directory -Force (Split-Path $OutputPath -Parent) | Out-Null
  New-Item -ItemType Directory -Force (Join-Path (Split-Path $OutputPath -Parent) 'bin') | Out-Null
  Copy-Item -LiteralPath $binary -Destination (Join-Path (Split-Path $OutputPath -Parent) 'bin/appbar_shell_hook_capability.exe') -Force
  $temporaryTrace="$OutputPath.pending"
  $trace=$raw|ConvertFrom-Json
  $trace|Add-Member -NotePropertyName explorer_mutations -NotePropertyValue $false
  $trace|Add-Member -NotePropertyName shell_takeover -NotePropertyValue $false
  $trace|Add-Member -NotePropertyName input_contract -NotePropertyValue ([ordered]@{
    current_substrate_manifest_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $manifest).Hash
    pre_mutation_admission_trace_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $admission).Hash
    binary_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $binary).Hash
    runner_source_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'crates/platform-win/examples/appbar_shell_hook_capability.rs')).Hash
    adapter_source_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'crates/platform-win/src/common/appbar_shell_hook.rs')).Hash
  })
  $trace|ConvertTo-Json -Depth 12 -Compress | Set-Content -LiteralPath $temporaryTrace -Encoding utf8
  & (Join-Path $change 'scripts/verify-appbar-shell-hook-trace.ps1') -TracePath $temporaryTrace -WorkspaceRoot $WorkspaceRoot
  if($LASTEXITCODE -ne 0){throw 'APPBAR_SHELL_HOOK_TRACE_INVALID'}
  Move-Item -LiteralPath $temporaryTrace -Destination $OutputPath -Force
  Write-Output "AppBar/Shell Hook controlled capability trace passed: $OutputPath"
} finally { Pop-Location }
