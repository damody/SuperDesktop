[CmdletBinding()]
param([string]$Change = 'bootstrap-superdesktop-workspace', [string]$EvidenceRoot, [switch]$Quiet)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$root = if ($EvidenceRoot) { (Resolve-Path $EvidenceRoot).Path } else { Join-Path $workspace "openspec/changes/$Change" }
function Fail([string]$Code, [string]$Message) { throw "$Code`: $Message" }
function Need($Value, [string]$Code, [string]$Message) { if (-not $Value) { Fail $Code $Message } }
function Id($Record) { "$($Record.task_id)#$($Record.subcheck)" }
function Values($Object, [string]$Plural, [string]$Singular) {
  if ($null -ne $Object.$Plural) { return @($Object.$Plural) }
  if ($null -ne $Object.$Singular) { return @($Object.$Singular) }
  return @()
}
function Write-Utf8NoBom([string]$Path, [string]$Text) { [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false)) }
function Invoke-Engine([string]$Schema, [string]$Instance, [string]$Code, [string]$Name) {
  $oldPreference = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
  try { $ignored = & $engine validate-json-schema $Schema $Instance 2>&1; $engineExit = $LASTEXITCODE } finally { $ErrorActionPreference = $oldPreference }
  if ($engineExit -ne 0) { Fail $Code $Name }
}

$schemaPath = Join-Path $root 'evidence/schema.json'
$coverageSchemaPath = Join-Path $root 'evidence/coverage-schema.json'
$coveragePath = Join-Path $root 'evidence/coverage.json'
$indexPath = Join-Path $root 'evidence/index.jsonl'
$adjustmentPath = Join-Path $root 'evidence/adjustments.jsonl'
$engine = Join-Path $workspace 'target/debug/superdesktop-test-support.exe'
if (-not (Test-Path $engine)) {
  & cargo build -p superdesktop-test-support --locked --offline
  if ($LASTEXITCODE -ne 0) { Fail 'JSON_SCHEMA_ENGINE_BUILD_FAILED' $engine }
}
Invoke-Engine $coverageSchemaPath $coveragePath 'JSON_SCHEMA_COVERAGE_INVALID' $coveragePath
$coverage = Get-Content -Raw $coveragePath | ConvertFrom-Json
$records = @(Get-Content $indexPath | Where-Object { $_ } | ForEach-Object { $_ | ConvertFrom-Json })
$by = @{}; $coverageByTask = @{}
foreach ($task in @($coverage.tasks)) { $coverageByTask[$task.task_id] = $task }
foreach ($record in $records) {
  $id = Id $record
  Need (-not $by.ContainsKey($id)) 'DUPLICATE_RECORD_IDENTITY' $id
  $by[$id] = $record
  if ($record.schema_version -ne '2.0.0') { continue }
  $stage = Join-Path $workspace ('build/validator-stage/' + [guid]::NewGuid().ToString('N') + '.json')
  New-Item -ItemType Directory -Force (Split-Path -Parent $stage) | Out-Null
  Write-Utf8NoBom $stage ($record | ConvertTo-Json -Depth 16)
  try { Invoke-Engine $schemaPath $stage 'JSON_SCHEMA_RECORD_INVALID' $id } finally { if (Test-Path $stage) { Remove-Item -LiteralPath $stage -Force } }
  $artifact = Join-Path $root $record.artifact
  Need (Test-Path $artifact) 'MISSING_ARTIFACT' $id
  Need ((Get-FileHash -Algorithm SHA256 $artifact).Hash -eq $record.artifact_sha256) 'ARTIFACT_HASH_DRIFT' $id
  $covered = $coverageByTask[$record.task_id]
  Need ($null -ne $covered) 'UNKNOWN_TASK' $id
  Need ($record.capability_id -eq $covered.capability_id -and $record.requirement_id -eq $covered.requirement_id -and $record.scenario_id -eq $covered.scenario_id -and ((@($record.gates) -join '|') -eq (@($covered.gates) -join '|'))) 'COVERAGE_DRIFT' $id
}
foreach ($task in @($coverage.tasks)) {
  if (-not $task.mandatory) { continue }
  Need (@($records | Where-Object { $_.schema_version -eq '2.0.0' -and $_.task_id -eq $task.task_id -and $_.status -eq 'passed' }).Count -gt 0) 'MANDATORY_WITHOUT_SCHEMA_COMPLETE_REPLACEMENT' $task.task_id
}
$replacementTargets = @{}
foreach ($record in @($records | Where-Object { $_.schema_version -eq '2.0.0' })) {
  $id = Id $record
  if ($record.status -eq 'stale') {
    Need $record.superseded_by 'STALE_WITHOUT_REPLACEMENT' $id
    Need $by.ContainsKey($record.superseded_by) 'DANGLING_SUPERSEDED_BY' $id
    $successor = $by[$record.superseded_by]
    Need ($successor.replaces -eq $id -and $successor.status -eq 'passed' -and $successor.task_id -eq $record.task_id) 'INVALID_REPLACEMENT' $id
    Need (-not $replacementTargets.ContainsKey($id)) 'REPLACEMENT_MAPPING_DUPLICATE' $id
    $replacementTargets[$id] = $successor
  }
  if ($record.replaces) {
    Need $by.ContainsKey($record.replaces) 'DANGLING_REPLACES' $id
    $predecessor = $by[$record.replaces]
    Need ($predecessor.status -eq 'stale' -and $predecessor.superseded_by -eq $id -and $record.status -eq 'passed') 'INVALID_REPLACEMENT' $id
  }
}

$adjustments = @(Get-Content $adjustmentPath | Where-Object { $_ } | ForEach-Object { $_ | ConvertFrom-Json })
$adjustmentById = @{}
foreach ($adjustment in $adjustments) {
  Need $adjustment.adjustment_id 'ADJUSTMENT_MALFORMED' 'missing adjustment_id'
  Need (-not $adjustmentById.ContainsKey($adjustment.adjustment_id)) 'ADJUSTMENT_DUPLICATE_ID' $adjustment.adjustment_id
  Need ($adjustment.classification -in @('A', 'B', 'C')) 'ADJUSTMENT_MALFORMED' $adjustment.adjustment_id
  Need $adjustment.status 'ADJUSTMENT_MALFORMED' $adjustment.adjustment_id
  if ($adjustment.classification -eq 'C') { Need $adjustment.c_approval_record_id 'C_APPROVAL_MISSING' $adjustment.adjustment_id }
  $stale = @(Values $adjustment 'stale_record_ids' '__none__'); $replacement = @(Values $adjustment 'replacement_record_ids' '__none__')
  if ($stale.Count -gt 0 -or $replacement.Count -gt 0) {
    Need ($stale.Count -eq $replacement.Count -and $stale.Count -gt 0) 'ADJUSTMENT_LINEAGE_INCOMPLETE' $adjustment.adjustment_id
    for ($i = 0; $i -lt $stale.Count; $i++) {
      Need ($by.ContainsKey($stale[$i]) -and $by.ContainsKey($replacement[$i])) 'ADJUSTMENT_DANGLING_RECORD' $adjustment.adjustment_id
      Need ($by[$stale[$i]].status -eq 'stale' -and $by[$stale[$i]].superseded_by -eq $replacement[$i] -and $by[$replacement[$i]].replaces -eq $stale[$i] -and $by[$replacement[$i]].status -eq 'passed') 'ADJUSTMENT_BACKLINK_INVALID' $adjustment.adjustment_id
    }
  }
  $adjustmentById[$adjustment.adjustment_id] = $adjustment
}
$parents = @{}
foreach ($adjustment in $adjustments) {
  $list = @(Values $adjustment 'supersedes_adjustments' 'supersedes_adjustment')
  $parents[$adjustment.adjustment_id] = $list
  foreach ($parent in @($list | Where-Object { $_ })) { Need $adjustmentById.ContainsKey($parent) 'ADJUSTMENT_SUPERSESSION_DANGLING' "$($adjustment.adjustment_id)->$parent" }
}
function Visit([string]$Node, $Seen, $Done) {
  if ($Seen.ContainsKey($Node)) { Fail 'ADJUSTMENT_SUPERSESSION_CYCLE' $Node }
  if ($Done.ContainsKey($Node)) { return }
  $Seen[$Node] = $true
  foreach ($parent in @($parents[$Node] | Where-Object { $_ })) { Visit $parent $Seen $Done }
  $Seen.Remove($Node) | Out-Null; $Done[$Node] = $true
}
$done = @{}; foreach ($id in $adjustmentById.Keys) { Visit $id @{} $done }
$legacy = @('A-W1-1.2-001','A-W1-1.2-003','B-W1-EXIT-001','B-W1-EXIT-001-lineage','B-W1-EXIT-002','B-W1-EXIT-003')
$children = @{}; foreach ($child in $parents.Keys) { foreach ($parent in @($parents[$child] | Where-Object { $_ })) { if (-not $children.ContainsKey($parent)) { $children[$parent]=@() }; $children[$parent] += $child } }
foreach ($legacyId in $legacy) {
  Need $adjustmentById.ContainsKey($legacyId) 'ADJUSTMENT_LEGACY_MISSING' $legacyId
  $queue = [System.Collections.Generic.Queue[string]]::new(); $queue.Enqueue($legacyId); $visited=@{}; $closed=$false
  while ($queue.Count -gt 0) { $node=$queue.Dequeue(); if ($visited.ContainsKey($node)) { continue }; $visited[$node]=$true; foreach ($child in @($children[$node] | Where-Object { $_ })) { if ($adjustmentById[$child].status -eq 'replacement-passed') { $closed=$true }; $queue.Enqueue($child) } }
  Need $closed 'ADJUSTMENT_LEGACY_UNCLOSED' $legacyId
}
if (-not $Quiet) { "Evidence validation passed: $($records.Count) records, $($coverage.tasks.Count) task coverage mappings." }
