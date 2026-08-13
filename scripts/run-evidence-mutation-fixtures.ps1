[CmdletBinding()]
param([string]$WorkspaceRoot, [string]$RunId=('run-' + (Get-Date -Format 'yyyyMMdd-HHmmss')))
$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }
function Write-Utf8NoBom([string]$Path, [string]$Text) { [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false)) }
$source = Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace'
$resultRoot = Join-Path $WorkspaceRoot "build/fixture-results/$RunId"
New-Item -ItemType Directory -Force $resultRoot | Out-Null
# The seed is copied once and never mutated. Each case receives a fresh copy; no production artifact is touched.
$seed = Join-Path $resultRoot 'immutable-seed'
Copy-Item (Join-Path $source 'evidence') $seed -Recurse -Force
$expected = [ordered]@{
  'mandatory-not-applicable'='MANDATORY_WITHOUT_SCHEMA_COMPLETE_REPLACEMENT'; 'mandatory-blocked'='MANDATORY_WITHOUT_SCHEMA_COMPLETE_REPLACEMENT';
  'missing-artifact'='MISSING_ARTIFACT'; 'missing-coverage'='UNKNOWN_TASK'; 'unknown-coverage'='COVERAGE_DRIFT'; 'drifted-coverage'='COVERAGE_DRIFT'; 'drifted-scenario'='COVERAGE_DRIFT'; 'stale-without-replacement'='STALE_WITHOUT_REPLACEMENT';
  'dangling-replacement'='DANGLING_SUPERSEDED_BY'; 'cyclic-replacement'='REPLACEMENT_CYCLE'; 'nonmandatory-replacement'='NONMANDATORY_REPLACEMENT'; 'replacement-coverage-drift'='COVERAGE_DRIFT'; 'unpassed-replacement'='UNPASSED_REPLACEMENT';
  'duplicate-identity'='DUPLICATE_RECORD_IDENTITY'; 'adjustment-stale-not-propagated'='ADJUSTMENT_BACKLINK_INVALID'; 'missing-procedure'='JSON_SCHEMA_RECORD_INVALID'; 'wrong-type'='JSON_SCHEMA_RECORD_INVALID'; 'wrong-pattern'='JSON_SCHEMA_RECORD_INVALID';
  'date-time-format'='JSON_SCHEMA_RECORD_INVALID'; 'dangling-adjustment'='ADJUSTMENT_DANGLING_RECORD'; 'malformed-adjustment'='ADJUSTMENT_LINEAGE_INCOMPLETE';
  'effective-stale-unmapped'='EFFECTIVE_STALE_MAPPING_MISSING'
}
foreach ($case in $expected.Keys) {
  $stage = Join-Path $resultRoot "cases/$case"; New-Item -ItemType Directory -Force $stage | Out-Null
  Copy-Item $seed (Join-Path $stage 'evidence') -Recurse -Force
  $index = Join-Path $stage 'evidence/index.jsonl'; $lines = [System.Collections.Generic.List[string]](Get-Content $index)
  # Add an isolated, current-hash record that is not a source or target in the
  # production adjustment graph, so each mutation tests only its intended rule.
  $fixtureBase = @($lines | Where-Object { $_ -match '"task_id":"bootstrap-superdesktop-workspace/2\.5\.6"' -and $_ -match '"subcheck":"effective-25"' })[0]
  if (-not $fixtureBase) { throw 'FIXTURE_BASE_RECORD_MISSING' }
  $lines.Add(($fixtureBase -replace '"subcheck":"effective-25"','"subcheck":"fixture-base"'))
  $n = $lines.Count - 1
  switch ($case) {
    'mandatory-not-applicable' { $lines[$n]=$lines[$n] -replace '"status":"passed"','"status":"not-applicable"' }
    'mandatory-blocked' { $lines[$n]=$lines[$n] -replace '"status":"passed"','"status":"blocked"' }
    'stale-without-replacement' { $lines[$n]=$lines[$n] -replace '"status":"passed"','"status":"stale"' }
    'missing-artifact' { $lines[$n]=$lines[$n] -replace 'evidence/artifacts/2.5/[^"\\]+','evidence/artifacts/2.5/not-found.txt' }
    'missing-coverage' { $c=Get-Content -Raw (Join-Path $stage 'evidence/coverage.json')|ConvertFrom-Json; $c.tasks=@($c.tasks|Where-Object {$_.task_id -ne 'bootstrap-superdesktop-workspace/2.5.6'}); Write-Utf8NoBom (Join-Path $stage 'evidence/coverage.json') ($c|ConvertTo-Json -Depth 8) }
    'unknown-coverage' { $lines[$n]=$lines[$n] -replace '"requirement_id":"windows-platform-substrate"','"requirement_id":"unknown"' }
    'drifted-coverage' { $lines[$n]=$lines[$n] -replace '"gates":\["G-ARCH","G-SAFETY"\]','"gates":["G-DRIFT"]' }
    'drifted-scenario' { $lines[$n]=$lines[$n] -replace '"scenario_id":"offline-windows-substrate"','"scenario_id":"drift"' }
    'dangling-replacement' { $lines[$n]=$lines[$n] -replace '"status":"passed"','"status":"stale"'; $lines[$n]=$lines[$n] -replace '"actual":"passed"','"actual":"passed","superseded_by":"missing#record"' }
    'cyclic-replacement' { $lines[$n]=$lines[$n] -replace '"status":"passed"','"status":"stale"'; $lines[$n]=$lines[$n] -replace '"actual":"passed"','"actual":"passed","superseded_by":"bootstrap-superdesktop-workspace/2.5.6#fixture-cycle-b"'; $lines[$n]=$lines[$n] -replace '"subcheck":"fixture-base"','"subcheck":"fixture-cycle-a"'; $lines.Add(($lines[$n] -replace '"subcheck":"fixture-cycle-a"','"subcheck":"fixture-cycle-b"' -replace '"superseded_by":"bootstrap-superdesktop-workspace/2.5.6#fixture-cycle-b"','"superseded_by":"bootstrap-superdesktop-workspace/2.5.6#fixture-cycle-a"')); }
    'nonmandatory-replacement' { $lines[$n]=$lines[$n] -replace '"status":"passed"','"status":"stale"'; $lines[$n]=$lines[$n] -replace '"actual":"passed"','"actual":"passed","superseded_by":"bootstrap-superdesktop-workspace/2.4.1#primary"'; $lines[$n]=$lines[$n] -replace '"subcheck":"fixture-base"','"subcheck":"nonmandatory-a"' }
    'replacement-coverage-drift' { $lines[$n]=$lines[$n] -replace '"scenario_id":"offline-windows-substrate"','"scenario_id":"drift"' }
    'unpassed-replacement' { $original=$lines[$n]; $lines[$n]=$lines[$n] -replace '"status":"passed"','"status":"stale"'; $lines[$n]=$lines[$n] -replace '"actual":"passed"','"actual":"passed","superseded_by":"bootstrap-superdesktop-workspace/2.5.6#unpassed-b"'; $lines[$n]=$lines[$n] -replace '"subcheck":"fixture-base"','"subcheck":"unpassed-a"'; $lines.Add(($original -replace '"subcheck":"fixture-base"','"subcheck":"unpassed-b"' -replace '"status":"passed"','"status":"failed"' -replace '"actual":"passed"','"actual":"failed","replaces":"bootstrap-superdesktop-workspace/2.5.6#unpassed-a"')); }
    'duplicate-identity' { $lines.Add($lines[$n]) }
    'adjustment-stale-not-propagated' { Add-Content -Encoding utf8 (Join-Path $stage 'evidence/adjustments.jsonl') '{"adjustment_id":"fixture-propagation","classification":"B","status":"replacement-passed","stale_record_ids":["bootstrap-superdesktop-workspace/2.3.1#corrective-stale"],"replacement_record_ids":["bootstrap-superdesktop-workspace/2.3.2#corrective"]}' }
    'missing-procedure' { $lines[$n]=$lines[$n] -replace ',"procedure":"[^"]+"','' }
    'wrong-type' { $lines[$n]=$lines[$n] -replace '"gates":\[[^\]]+\]','"gates":"bad"' }
    'wrong-pattern' { $lines[$n]=$lines[$n] -replace '"subcheck":"[^"]+"','"subcheck":"BAD!"' }
    'date-time-format' { $lines[$n]=$lines[$n] -replace '"recorded_at":"[^"]+"','"recorded_at":"not-a-date"' }
    'dangling-adjustment' { Add-Content -Encoding utf8 (Join-Path $stage 'evidence/adjustments.jsonl') '{"adjustment_id":"fixture-dangling","classification":"B","status":"replacement-passed","stale_record_ids":["missing#x"],"replacement_record_ids":["missing#y"]}' }
    'malformed-adjustment' { Add-Content -Encoding utf8 (Join-Path $stage 'evidence/adjustments.jsonl') '{"adjustment_id":"fixture-malformed","classification":"B","status":"replacement-passed","stale_record_ids":["bootstrap-superdesktop-workspace/2.3.1#corrective-stale"],"replacement_record_ids":[]}' }
    'effective-stale-unmapped' { Add-Content -Encoding utf8 (Join-Path $stage 'evidence/adjustments.jsonl') '{"adjustment_id":"fixture-effective-stale-unmapped","classification":"B","status":"replacement-passed","stale_evidence":["bootstrap-superdesktop-workspace/2.5.6#fixture-base"]}' }
  }
  if ($case -in @('mandatory-not-applicable','mandatory-blocked')) {
    for ($i=0; $i -lt $lines.Count; $i++) { if ($lines[$i] -match '"task_id":"bootstrap-superdesktop-workspace/2\.5\.6"' -and $lines[$i] -match '"status":"passed"') { $lines[$i]=$lines[$i] -replace '"status":"passed"', ('"status":"' + $(if($case -eq 'mandatory-blocked'){'blocked'}else{'not-applicable'}) + '"') } }
  }
  Write-Utf8NoBom $index ($lines -join [Environment]::NewLine)
  $old = $ErrorActionPreference; $ErrorActionPreference='Continue'
  try { $output = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'validate-evidence.ps1') -EvidenceRoot $stage 2>&1; $exitCode=$LASTEXITCODE } finally { $ErrorActionPreference=$old }
  $text = $output | Out-String
  $log = Join-Path $resultRoot "$case.txt"; Set-Content -Encoding utf8 $log @("case: $case", "expected_code: $($expected[$case])", "exit_status: $exitCode", 'actual_output:', $text)
  if ($exitCode -ne 1 -or -not $text.Contains($expected[$case])) { throw "MUTATION_FIXTURE_MISMATCH: $case expected $($expected[$case]) got $text" }
}
Write-Output "$($expected.Count) production-path mutation fixtures matched expected semantic diagnostics: $resultRoot"
exit 0
