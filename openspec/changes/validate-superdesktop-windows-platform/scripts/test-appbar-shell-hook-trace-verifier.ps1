[CmdletBinding()]
param([string]$TracePath,[string]$VerifierPath)

$ErrorActionPreference='Stop'
if(-not $TracePath){$TracePath=Join-Path $PSScriptRoot '../evidence/artifacts/2.1/appbar-shell-hook-trace.json'}
if(-not $VerifierPath){$VerifierPath=Join-Path $PSScriptRoot 'verify-appbar-shell-hook-trace.ps1'}
if(-not(Test-Path -LiteralPath $TracePath) -or -not(Test-Path -LiteralPath $VerifierPath)){throw 'TRACE_VERIFIER_TEST_INPUT_MISSING'}
function Assert-Rejected([string]$Name,[scriptblock]$Mutate){
  $trace=Get-Content -Raw -LiteralPath $TracePath|ConvertFrom-Json
  & $Mutate $trace
  $temporary=New-TemporaryFile
  $trace|ConvertTo-Json -Depth 12 -Compress|Set-Content -LiteralPath $temporary -Encoding utf8
  $rejected=$false
  try { & $VerifierPath -TracePath $temporary.FullName -ArtifactDirectory (Split-Path $TracePath -Parent) | Out-Null } catch { $rejected=$true }
  if(-not $rejected){throw "TRACE_VERIFIER_ACCEPTED_NEGATIVE:$Name"}
}
Assert-Rejected 'missing-field' { param($t) $t.PSObject.Properties.Remove('shell_hook_events') }
Assert-Rejected 'boolean-string' { param($t) $t.controlled_only='true' }
Assert-Rejected 'hash-substitution' { param($t) $t.input_contract.binary_sha256=('A'*64) }
Assert-Rejected 'mid-failure-stage' { param($t) $t.mid_failure.typed_failure='invalid-hwnd-only' }
Assert-Rejected 'unregister-event-mutation' { param($t) $t.unregister_event_fence.after_helper=[int]$t.unregister_event_fence.before_helper+1 }
Write-Output 'AppBar/Shell Hook trace verifier negative matrix passed.'
