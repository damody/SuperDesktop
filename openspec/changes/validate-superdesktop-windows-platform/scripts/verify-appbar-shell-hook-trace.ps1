[CmdletBinding()]
param([string]$TracePath,[string]$WorkspaceRoot,[string]$ArtifactDirectory)

$ErrorActionPreference='Stop'
if(-not $TracePath -or -not(Test-Path -LiteralPath $TracePath -PathType Leaf)){throw 'TRACE_INPUT_MISSING'}
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
if(-not $ArtifactDirectory){$ArtifactDirectory=Split-Path $TracePath -Parent}
$trace=Get-Content -Raw -LiteralPath $TracePath|ConvertFrom-Json
function Require-Property([object]$Object,[string]$Name){if($null -eq $Object.PSObject.Properties[$Name]){throw "TRACE_FIELD_MISSING:$Name"};return $Object.$Name}
function Require-Bool([object]$Object,[string]$Name,[bool]$Expected=$true){$value=Require-Property $Object $Name;if($value -isnot [bool]){throw "TRACE_BOOLEAN_TYPE_INVALID:$Name"};if($value -ne $Expected){throw "TRACE_BOOLEAN_VALUE_INVALID:$Name"}}
function Require-Int([object]$Object,[string]$Name,[int]$Minimum=0){$value=Require-Property $Object $Name;if($value -isnot [long] -and $value -isnot [int]){throw "TRACE_INTEGER_TYPE_INVALID:$Name"};if([int64]$value -lt $Minimum){throw "TRACE_INTEGER_RANGE_INVALID:$Name"};return [int64]$value}
function Require-Hash([object]$Object,[string]$Name){$value=Require-Property $Object $Name;if($value -isnot [string] -or $value -notmatch '^[A-F0-9]{64}$'){throw "TRACE_HASH_INVALID:$Name"}}
function Assert-RectEqual([object]$Left,[object]$Right,[string]$Prefix){foreach($edge in @('left','top','right','bottom')){if([int](Require-Property $Left $edge) -ne [int](Require-Property $Right $edge)){throw "TRACE_RECT_NOT_EQUAL:$Prefix.$edge"}}}
function Assert-ResourcesEqual([object]$Left,[object]$Right,[string]$Prefix){foreach($field in @('process_handles','user_objects','gdi_objects')){if((Require-Int $Left $field) -ne (Require-Int $Right $field)){throw "TRACE_RESOURCE_DELTA:$Prefix.$field"}}}

if((Require-Property $trace 'schema') -ne 'appbar-shell-hook-trace/v2'){throw 'TRACE_SCHEMA_INVALID'}
foreach($name in @('controlled_only','warmup_unaccepted','appbar_registered','failure_injection_rejected','explorer_mutations','shell_takeover')){Require-Bool $trace $name ($name -notin @('explorer_mutations','shell_takeover'))}
if((Require-Int $trace 'shell_hook_message' 1) -le 0 -or (Require-Int $trace 'shell_hook_events' 1) -le 0){throw 'TRACE_SHELL_HOOK_DELIVERY_MISSING'}
Require-Bool (Require-Property $trace 'first_teardown') 'appbar_removed'; Require-Bool (Require-Property $trace 'first_teardown') 'shell_hook_unregistered'
Require-Bool (Require-Property $trace 'second_teardown') 'appbar_removed' $false; Require-Bool (Require-Property $trace 'second_teardown') 'shell_hook_unregistered' $false
$mid=Require-Property $trace 'mid_failure'; if((Require-Property $mid 'typed_failure') -ne 'injected-after-reserve-before-shell-hook'){throw 'TRACE_MID_FAILURE_STAGE_INVALID'}; Require-Bool $mid 'appbar_removed'; Assert-RectEqual (Require-Property $trace 'work_area_before') (Require-Property $mid 'work_area_after') 'mid-failure-work-area'; Assert-ResourcesEqual (Require-Property $mid 'resources_before') (Require-Property $mid 'resources_after') 'mid-failure-resources'
$fence=Require-Property $trace 'unregister_event_fence'; if((Require-Int $fence 'before_helper') -ne (Require-Int $fence 'after_helper')){throw 'TRACE_UNREGISTER_EVENT_FENCE_FAILED'}
Assert-RectEqual (Require-Property $trace 'work_area_before') (Require-Property $trace 'work_area_after') 'measured-work-area'
Assert-ResourcesEqual (Require-Property $trace 'resources_before_first') (Require-Property $trace 'resources_after_first') 'first-measured-resources'
Assert-ResourcesEqual (Require-Property $trace 'resources_before_second') (Require-Property $trace 'resources_after_second') 'second-measured-resources'
$contract=Require-Property $trace 'input_contract'; foreach($name in @('current_substrate_manifest_sha256','pre_mutation_admission_trace_sha256','binary_sha256','runner_source_sha256','adapter_source_sha256')){Require-Hash $contract $name}
$expectedHashes=[ordered]@{
  current_substrate_manifest_sha256=Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-inputs-successor-1.2-manifest-v3.sha256'
  pre_mutation_admission_trace_sha256=Join-Path $ArtifactDirectory 'pre-mutation-admission-trace.json'
  binary_sha256=Join-Path $ArtifactDirectory 'bin/appbar_shell_hook_capability.exe'
  runner_source_sha256=Join-Path $WorkspaceRoot 'crates/platform-win/examples/appbar_shell_hook_capability.rs'
  adapter_source_sha256=Join-Path $WorkspaceRoot 'crates/platform-win/src/common/appbar_shell_hook.rs'
}
foreach($name in $expectedHashes.Keys){$path=$expectedHashes[$name];if(-not(Test-Path -LiteralPath $path -PathType Leaf)){throw "TRACE_HASH_SOURCE_MISSING:$name"};if($contract.$name -ne (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash){throw "TRACE_HASH_SUBSTITUTION:$name"}}
Write-Output 'AppBar/Shell Hook v2 trace semantic validation passed.'
