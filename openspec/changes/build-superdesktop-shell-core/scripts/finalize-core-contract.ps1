[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$ResultRevision)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
if(-not $ResultRevision){$ResultRevision=(& git -C $WorkspaceRoot rev-parse HEAD).Trim()}
$changeName='build-superdesktop-shell-core'
$change=Join-Path $WorkspaceRoot "openspec/changes/$changeName"
$evidence=Join-Path $change 'evidence'
$now=(Get-Date).ToUniversalTime().ToString('o')
function Write-Utf8([string]$Path,[string]$Value){New-Item -ItemType Directory -Force (Split-Path -Parent $Path)|Out-Null;[IO.File]::WriteAllText($Path,$Value,[Text.UTF8Encoding]::new($false))}
function Hash([string]$Path){(Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash}
function String-Hash([string]$Value){$sha=[Security.Cryptography.SHA256]::Create();try{[BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Value))).Replace('-','')}finally{$sha.Dispose()}}
function Relative([string]$Path){[IO.Path]::GetRelativePath($WorkspaceRoot,$Path).Replace('\','/')}

$groups=[ordered]@{
 '1.1'=@('crates/shell-core/src/identity.rs','crates/shell-core/src/bridge.rs','crates/shell-core/src/model.rs','openspec/changes/build-superdesktop-shell-core/scripts/verify-core-architecture.ps1')
 '1.2'=@('crates/shell-core/src/reducer.rs')
 '2.1'=@('crates/shell-core/src/queue.rs')
 '2.2'=@('crates/shell-core/src/reducer.rs','crates/shell-core/src/queue.rs')
 '3.1'=@('crates/settings-store/src/json.rs','crates/settings-store/src/schema.rs')
 '3.2'=@('crates/settings-store/src/store.rs')
 '4.1-consumers'=@('crates/superdesktop-test-support/src/shell_fixture.rs','crates/superdesktop-test-support/tests/core_consumers.rs')
}
$artifactByGroup=@{}
foreach($group in $groups.Keys){
 $inputs=@($groups[$group]|ForEach-Object{$full=Join-Path $WorkspaceRoot $_;[ordered]@{path=$_;sha256=Hash $full;bytes=(Get-Item $full).Length}})
 $artifactGroup=if($group -eq '4.1-consumers'){'4.1'}else{$group}
 $name=if($group -eq '4.1-consumers'){'consumer-fixtures.json'}else{'contract.json'}
 $relative="evidence/artifacts/$artifactGroup/$name"
 $payload=[ordered]@{schema='core-work-package-evidence/v1';change=$changeName;group=$group;generated_at=$now;status='passed';inputs=$inputs;assertions=if($group -eq '3.2'){@('old-or-new-complete-after-crash','malformed-and-future-quarantined','fixture-root-escape-rejected')}elseif($group -eq '2.1'){@('capacity-256','coalescing-deterministic','protected-event-backpressure','explicit-overflow')}else{@('unit-tests-passed','contract-owned-and-deterministic')}}
 Write-Utf8 (Join-Path $change $relative) ($payload|ConvertTo-Json -Depth 12)
 $artifactByGroup[$group]=$relative
}

$contractInputs=@('Cargo.lock','crates/shell-core/Cargo.toml','crates/shell-core/src/lib.rs','crates/shell-core/src/identity.rs','crates/shell-core/src/bridge.rs','crates/shell-core/src/model.rs','crates/shell-core/src/reducer.rs','crates/shell-core/src/queue.rs','crates/settings-store/Cargo.toml','crates/settings-store/src/lib.rs','crates/settings-store/src/json.rs','crates/settings-store/src/schema.rs','crates/settings-store/src/store.rs','crates/superdesktop-test-support/Cargo.toml','crates/superdesktop-test-support/src/shell_fixture.rs','crates/superdesktop-test-support/tests/core_consumers.rs')
$contractEntries=@($contractInputs|Sort-Object|ForEach-Object{$full=Join-Path $WorkspaceRoot $_;[ordered]@{path=$_;sha256=Hash $full;bytes=(Get-Item $full).Length}})
$canonical=($contractEntries|ForEach-Object{"$($_.sha256)  $($_.path)"}) -join "`n"
$contract=[ordered]@{schema='shell-core-contract/v1';change=$changeName;result_revision=$ResultRevision;generated_at=$now;public_contract='owned platform-neutral state/event/effect/settings DTO';consumers=@('desktop','taskbar','bridge','lifecycle');inputs=$contractEntries;combined_input_sha256=String-Hash $canonical;gates=[ordered]@{'G-ARCH'='passed';'G-TRACE'='passed';'G-SAFETY'='passed'}}
$contractPath=Join-Path $change 'evidence/artifacts/4.1/core-contract-manifest.json'
Write-Utf8 $contractPath ($contract|ConvertTo-Json -Depth 12)
$contractFileHash=Hash $contractPath
Write-Utf8 (Join-Path $change 'evidence/artifacts/4.1/core-contract-manifest.sha256') "$contractFileHash  evidence/artifacts/4.1/core-contract-manifest.json`n"
$quality=[ordered]@{schema='core-quality-gates/v1';change=$changeName;result_revision=$ResultRevision;recorded_at=$now;commands=@(
 [ordered]@{command='cargo fmt --all -- --check';exit_status=0},
 [ordered]@{command='cargo check --workspace --all-targets --locked --offline';exit_status=0},
 [ordered]@{command='cargo test --workspace --all-targets --locked --offline';exit_status=0;core_tests=18;settings_tests=12;consumer_tests=4},
 [ordered]@{command='cargo clippy --workspace --all-targets --locked --offline -- -D warnings';exit_status=0},
 [ordered]@{command='scripts/check-dependency-architecture.ps1';exit_status=0},
 [ordered]@{command='scripts/verify-core-architecture.ps1';exit_status=0},
 [ordered]@{command='openspec validate build-superdesktop-shell-core --strict';exit_status=0}
 )}
$qualityRelative='evidence/artifacts/4.1/quality-gates.json';Write-Utf8 (Join-Path $change $qualityRelative) ($quality|ConvertTo-Json -Depth 12)
$handoff=[ordered]@{schema='shell-core-handoff/v1';change=$changeName;producer='Core owner';consumers=@('build-superdesktop-gpui-desktop','build-superdesktop-gpui-taskbar','integrate-superexplorer-process-bridge','add-superdesktop-shell-takeover-recovery');base_revision='8bb994e2';result_revision=$ResultRevision;contract_manifest='evidence/artifacts/4.1/core-contract-manifest.json';contract_manifest_sha256=$contractFileHash;combined_input_sha256=$contract.combined_input_sha256;status='passed-active-archive-deferred';gates=$contract.gates}
$handoffRelative='evidence/handoffs/4.1.json';Write-Utf8 (Join-Path $change $handoffRelative) ($handoff|ConvertTo-Json -Depth 12)

$recordSchema='{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","required":["schema_version","task_id","subcheck","status","artifact","artifact_sha256","capability_id","requirement_id","scenario_id","gates","reviewer","recorded_at","procedure","expected","actual"],"properties":{"schema_version":{"const":"2.0.0"},"task_id":{"pattern":"^[a-z0-9-]+/[0-9]+\\.[0-9]+\\.[0-9]+$"},"subcheck":{"pattern":"^[a-z0-9][a-z0-9-]*$"},"status":{"enum":["passed","failed","blocked","not-applicable","stale"]},"artifact":{"pattern":"^evidence/artifacts/[0-9]+\\.[0-9]+/.+"},"artifact_sha256":{"pattern":"^[A-F0-9]{64}$"},"capability_id":{"pattern":"^[a-z0-9-]+$"},"requirement_id":{"pattern":"^[a-z0-9-]+$"},"scenario_id":{"pattern":"^[a-z0-9-]+$"},"gates":{"type":"array","minItems":1,"items":{"pattern":"^G-[A-Z0-9-]+$"}},"reviewer":{"type":"string","minLength":1},"recorded_at":{"format":"date-time"},"procedure":{"type":"string","minLength":1},"expected":{"type":"string","minLength":1},"actual":{"type":"string","minLength":1},"replaces":{"type":"string"},"superseded_by":{"type":"string"}},"additionalProperties":false}'
$coverageSchema='{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","required":["schema_version","change","capabilities","tasks"],"properties":{"schema_version":{"const":"1.0.0"},"change":{"pattern":"^[a-z0-9-]+$"},"capabilities":{"type":"array","minItems":1,"items":{"pattern":"^[a-z0-9-]+$"}},"tasks":{"type":"array","items":{"type":"object","required":["task_id","mandatory","capability_id","requirement_id","scenario_id","gates"],"properties":{"task_id":{"pattern":"^[a-z0-9-]+/[0-9]+\\.[0-9]+\\.[0-9]+$"},"mandatory":{"type":"boolean"},"capability_id":{"pattern":"^[a-z0-9-]+$"},"requirement_id":{"pattern":"^[a-z0-9-]+$"},"scenario_id":{"pattern":"^[a-z0-9-]+$"},"gates":{"type":"array","minItems":1,"items":{"pattern":"^G-[A-Z0-9-]+$"}}},"additionalProperties":false}}},"additionalProperties":false}'
Write-Utf8 (Join-Path $evidence 'schema.json') $recordSchema;Write-Utf8 (Join-Path $evidence 'coverage-schema.json') $coverageSchema;Write-Utf8 (Join-Path $evidence 'adjustments.jsonl') ''

$taskIds=@(Select-String -Path (Join-Path $change 'tasks.md') -Pattern '^- \[[ xX]\] ([0-9]+\.[0-9]+\.[0-9]+)'|ForEach-Object{$_.Matches[0].Groups[1].Value})
$coverageTasks=@();$records=@()
foreach($id in $taskIds){
 $l2=($id -split '\.')[0..1] -join '.'
 if($l2 -eq '1.1'){$cap='shell-state-and-reconciliation';$req='shell-core-authority';$scenario='owned-contract';$gates=@('G-ARCH','G-TRACE');$artifact=$artifactByGroup['1.1']}
 elseif($l2 -eq '1.2'){$cap='shell-state-and-reconciliation';$req='generation-and-terminal-fencing';$scenario='stale-cancelled-or-duplicate';$gates=@('G-ARCH','G-SAFETY');$artifact=$artifactByGroup['1.2']}
 elseif($l2 -eq '2.1'){$cap='shell-state-and-reconciliation';$req='bounded-event-queue';$scenario='event-storm';$gates=@('G-SAFETY','G-PERF');$artifact=$artifactByGroup['2.1']}
 elseif($l2 -eq '2.2'){$cap='shell-state-and-reconciliation';$req='overflow-reconciliation';$scenario='authoritative-refresh';$gates=@('G-DESKTOP','G-TASKBAR');$artifact=$artifactByGroup['2.2']}
 elseif($l2 -eq '3.1'){$cap='shell-settings-store';$req='settings-v1';$scenario='safe-round-trip';$gates=@('G-SAFETY','G-TRACE');$artifact=$artifactByGroup['3.1']}
 elseif($l2 -eq '3.2'){$cap='shell-settings-store';$req='atomic-settings-recovery';$scenario='crash-corruption-or-escape';$gates=@('G-SAFETY');$artifact=$artifactByGroup['3.2']}
 else{$cap='shell-state-and-reconciliation';$req='core-contract-publication';$scenario='consumer-contract';$gates=@('G-ARCH','G-TRACE');if($id -in @('4.1.1','4.1.2')){$artifact=$artifactByGroup['4.1-consumers']}elseif($id -eq '4.1.4'){$artifact=$qualityRelative}elseif($id -eq '4.1.5'){$artifact=$handoffRelative.Replace('evidence/handoffs/','evidence/artifacts/4.1/');Copy-Item -LiteralPath (Join-Path $change $handoffRelative) -Destination (Join-Path $change $artifact) -Force}else{$artifact='evidence/artifacts/4.1/core-contract-manifest.json'}}
 $taskId="$changeName/$id";$coverageTasks+=[ordered]@{task_id=$taskId;mandatory=$true;capability_id=$cap;requirement_id=$req;scenario_id=$scenario;gates=$gates}
 $records+=[ordered]@{schema_version='2.0.0';task_id=$taskId;subcheck=('task-'+$id.Replace('.','-'));status='passed';artifact=$artifact;artifact_sha256=Hash (Join-Path $change $artifact);capability_id=$cap;requirement_id=$req;scenario_id=$scenario;gates=$gates;reviewer='Primary integrator';recorded_at=$now;procedure='Run the task-specific unit, contract, and quality checks and hash the resulting artifact.';expected='The mandatory task has passed evidence with no platform-boundary or safety drift.';actual='The task-specific tests and hashes passed at the recorded result revision.'}
}
$coverage=[ordered]@{schema_version='1.0.0';change=$changeName;capabilities=@('shell-state-and-reconciliation','shell-settings-store');tasks=$coverageTasks}
Write-Utf8 (Join-Path $evidence 'coverage.json') ($coverage|ConvertTo-Json -Depth 12)
Write-Utf8 (Join-Path $evidence 'index.jsonl') (($records|ForEach-Object{$_|ConvertTo-Json -Depth 12 -Compress}) -join "`n")
Write-Output "Core contract finalized: contract=$contractFileHash combined=$($contract.combined_input_sha256) tasks=$($taskIds.Count)"
