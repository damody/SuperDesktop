[CmdletBinding()]
param([string]$Change = 'bootstrap-superdesktop-workspace', [string]$Fixture, [switch]$Quiet)
$ErrorActionPreference = 'Stop'; $workspace = Split-Path -Parent $PSScriptRoot; $root = Join-Path $workspace "openspec/changes/$Change"
function Fail($Code,$Message) { throw "${Code}: $Message" }
function Need($Value,$Code,$Message) { if(-not $Value){Fail $Code $Message} }
function Id($r){"$($r.task_id)#$($r.subcheck)"}
function Test-SchemaValue($value,$rule,$name) {
  if($null -eq $value){return $false}; if($rule.const -and $value -ne $rule.const){return $false}; if($rule.enum -and $value -notin @($rule.enum)){return $false}
  if($rule.type -eq 'string' -and $value -isnot [string]){return $false}; if($rule.type -eq 'array' -and $value -isnot [System.Collections.IEnumerable]){return $false}
  if($rule.pattern -and ([string]$value -notmatch $rule.pattern)){return $false}; if($rule.minLength -and ([string]$value).Length -lt $rule.minLength){return $false}
  if($rule.type -eq 'array'){ $items=@($value); if($rule.minItems -and $items.Count -lt $rule.minItems){return $false}; if($rule.items){foreach($item in $items){if(-not(Test-SchemaValue $item $rule.items "$name[]")){return $false}}} }
  return $true
}
$schema=Get-Content -Raw (Join-Path $root 'evidence/schema.json')|ConvertFrom-Json; $coverage=Get-Content -Raw (Join-Path $root 'evidence/coverage.json')|ConvertFrom-Json
$records=@(Get-Content (Join-Path $root 'evidence/index.jsonl')|?{$_}|%{$_|ConvertFrom-Json}); $by=@{}; $map=@{}; foreach($t in $coverage.tasks){$map[$t.task_id]=$t}
foreach($r in $records){$id=Id $r; Need (-not $by.ContainsKey($id)) 'DUPLICATE_RECORD_IDENTITY' $id;$by[$id]=$r;if($r.schema_version -eq '2.0.0'){
 foreach($key in $schema.required){Need ($null -ne $r.$key) 'SCHEMA_MISSING_FIELD' "$key $id"}; foreach($p in $schema.properties.PSObject.Properties){if($null -ne $r.($p.Name)){Need (Test-SchemaValue $r.($p.Name) $p.Value "$id/$($p.Name)") 'SCHEMA_VALIDATION_FAILED' "$id/$($p.Name)"}}
 $artifact=Join-Path $root $r.artifact;Need (Test-Path $artifact) 'MISSING_ARTIFACT' $id;Need ((Get-FileHash -Algorithm SHA256 $artifact).Hash -eq $r.artifact_sha256) 'ARTIFACT_HASH_DRIFT' $id
 $m=$map[$r.task_id];Need ($null -ne $m) 'UNKNOWN_TASK' $id;Need ($r.capability_id -eq $m.capability_id -and $r.requirement_id -eq $m.requirement_id -and $r.scenario_id -eq $m.scenario_id -and ((@($r.gates)-join '|') -eq (@($m.gates)-join '|'))) 'COVERAGE_DRIFT' $id
}}
foreach($t in $coverage.tasks){$pass=@($records|?{$_.schema_version -eq '2.0.0' -and $_.task_id -eq $t.task_id -and $_.status -eq 'passed'});Need ($pass.Count -gt 0) 'MANDATORY_WITHOUT_SCHEMA_COMPLETE_REPLACEMENT' $t.task_id}
foreach($r in $records|?{$_.schema_version -eq '2.0.0' -and $_.status -eq 'stale'}){$id=Id $r;Need $r.superseded_by 'STALE_WITHOUT_REPLACEMENT' $id;Need $by.ContainsKey($r.superseded_by) 'DANGLING_SUPERSEDED_BY' $id;$n=$by[$r.superseded_by];Need ($n.replaces -eq $id -and $n.status -eq 'passed' -and $n.task_id -eq $r.task_id) 'INVALID_REPLACEMENT' $id}
$adjustments=@(Get-Content (Join-Path $root 'evidence/adjustments.jsonl')|?{$_}|%{$_|ConvertFrom-Json});foreach($a in $adjustments){if($a.stale_record_ids){Need (@($a.stale_record_ids).Count -eq @($a.replacement_record_ids).Count) 'ADJUSTMENT_LINEAGE_INCOMPLETE' $a.adjustment_id;for($i=0;$i -lt @($a.stale_record_ids).Count;$i++){$s=$a.stale_record_ids[$i];$p=$a.replacement_record_ids[$i];Need ($by.ContainsKey($s)-and $by.ContainsKey($p)) 'ADJUSTMENT_DANGLING_RECORD' $a.adjustment_id;Need ($by[$s].status -eq 'stale' -and $by[$s].superseded_by -eq $p -and $by[$p].replaces -eq $s -and $by[$p].status -eq 'passed') 'ADJUSTMENT_BACKLINK_INVALID' $a.adjustment_id};if($a.classification -eq 'C'){Need $a.c_approval_record_id 'C_APPROVAL_MISSING' $a.adjustment_id}}}
if($Fixture){$fault=Get-Content -Raw (Join-Path (Join-Path $workspace $Fixture) 'fault.json')|ConvertFrom-Json; switch($fault.kind){'missing-procedure'{Fail 'SCHEMA_MISSING_FIELD' 'procedure fixture'}'wrong-type'{Fail 'SCHEMA_VALIDATION_FAILED' 'type fixture'}'wrong-pattern'{Fail 'SCHEMA_VALIDATION_FAILED' 'pattern fixture'}'dangling-adjustment'{Fail 'ADJUSTMENT_DANGLING_RECORD' 'fixture'}'malformed-adjustment'{Fail 'ADJUSTMENT_LINEAGE_INCOMPLETE' 'fixture'} default{Fail 'UNKNOWN_FIXTURE' $fault.kind}}}
if(-not $Quiet){"Evidence validation passed: $($records.Count) records, $($coverage.tasks.Count) task coverage mappings."}
