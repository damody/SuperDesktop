[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$TracePath)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
if(-not $TracePath){$TracePath=Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/2.2/monitor-dpi-start-trace.json'}
$change=Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform'
$verifier=Join-Path $change 'scripts/verify-monitor-dpi-start-trace.ps1'
$artifact=Split-Path $TracePath -Parent
$base=Get-Content -Raw $TracePath|ConvertFrom-Json
function Clone-Trace { param($Trace) return ($Trace|ConvertTo-Json -Depth 16|ConvertFrom-Json) }
function Assert-Rejected { param([string]$Name,$Trace)
  $temporary=Join-Path ([IO.Path]::GetTempPath()) ("monitor-dpi-negative-$Name-$([guid]::NewGuid()).json")
  try {
    $Trace|ConvertTo-Json -Depth 16 -Compress|Set-Content $temporary -Encoding utf8
    $accepted=$true
    try { & $verifier -TracePath $temporary -WorkspaceRoot $WorkspaceRoot -ArtifactDirectory $artifact | Out-Null } catch { $accepted=$false }
    if($accepted){throw "NEGATIVE_CASE_ACCEPTED:$Name"}
  } finally { Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue }
}

$case=Clone-Trace $base;$case.PSObject.Properties.Remove('dpi_awareness');Assert-Rejected 'missing-awareness' $case
$case=Clone-Trace $base;$case.explorer_mutations=$true;Assert-Rejected 'explorer-mutation-flag' $case
$case=Clone-Trace $base;$case.start_invocation_attempted=$false;Assert-Rejected 'start-invocation-flag' $case
$case=Clone-Trace $base;$case.start_host.host_observation.path='C:\Temp\SearchHost.exe';Assert-Rejected 'untrusted-start-host-path' $case
$case=Clone-Trace $base;$case.start_host.restored=$false;Assert-Rejected 'start-restore-failed' $case
$case=Clone-Trace $base;$case.virtual_fixture.events[2].dpi_x=96;Assert-Rejected 'virtual-dpi-transition' $case
$case=Clone-Trace $base;$case.input_contract.runner_source_sha256=('0' * 64);Assert-Rejected 'hash-substitution' $case
$case=Clone-Trace $base;$case.external_snapshot.equality_passed=$false;Assert-Rejected 'external-snapshot-false-flag' $case
$case=Clone-Trace $base;$case.external_snapshot.capture_thread_is_per_monitor_v2=$false;Assert-Rejected 'capture-awareness-false-flag' $case
Write-Output 'Monitor/DPI/Start semantic verifier negative tests passed.'
