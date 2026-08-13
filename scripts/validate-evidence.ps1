[CmdletBinding()]
param(
    [Parameter()]
    [string]$Change = 'bootstrap-superdesktop-workspace',

    [Parameter()]
    [string]$Fixture,

    [Parameter()]
    [switch]$Quiet
)

$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$changeRoot = Join-Path $workspace "openspec/changes/$Change"

function Fail([string]$Code, [string]$Message) { throw "${Code}: $Message" }
function Read-Json([string]$Path) { Get-Content -Raw -Encoding UTF8 $Path | ConvertFrom-Json }
function Record-Id($Record) { "$($Record.task_id)#$($Record.subcheck)" }
function Require([bool]$Condition, [string]$Code, [string]$Message) { if (-not $Condition) { Fail $Code $Message } }

$coverage = Read-Json (Join-Path $changeRoot 'evidence/coverage.json')
$records = @(Get-Content -Encoding UTF8 (Join-Path $changeRoot 'evidence/index.jsonl') | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json })
$adjustmentsPath = Join-Path $changeRoot 'evidence/adjustments.jsonl'
$adjustments = if (Test-Path $adjustmentsPath) { @(Get-Content -Encoding UTF8 $adjustmentsPath | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json }) } else { @() }
$taskLines = Get-Content -Encoding UTF8 (Join-Path $changeRoot 'tasks.md')
$taskChecks = @{}
foreach ($line in $taskLines) {
    if ($line -match '^\s*- \[(?<checked>[ x])\] (?<id>\d+\.\d+\.\d+) ') { $taskChecks["$Change/$($Matches.id)"] = ($Matches.checked -eq 'x') }
}

if ($Fixture) {
    $fault = Read-Json (Join-Path (Join-Path $workspace $Fixture) 'fault.json')
    switch ($fault.kind) {
        'mandatory-not-applicable' { $records[0].status = 'not-applicable' }
        'mandatory-blocked' { $records[0].status = 'blocked' }
        'stale-without-replacement' { $records[2].superseded_by = $null }
        'missing-artifact' { $records[0].artifact = 'evidence/artifacts/1.1/does-not-exist.txt' }
        'unknown-coverage' { $records[0].requirement_id = 'unknown-requirement' }
        'missing-coverage' { $coverage.tasks = @($coverage.tasks | Where-Object { $_.task_id -ne $records[0].task_id }) }
        'drifted-coverage' { $records[0].gates = @('G-DRIFT') }
        'drifted-scenario' { $records[0].scenario_id = 'drifted-scenario' }
        'dangling-replacement' { $records[2].superseded_by = 'bootstrap-superdesktop-workspace/1.1.3#missing' }
        'cyclic-replacement' { $records[2].superseded_by = 'bootstrap-superdesktop-workspace/1.1.3#replacement'; $records[4] | Add-Member -NotePropertyName superseded_by -NotePropertyValue 'bootstrap-superdesktop-workspace/1.1.3#primary' }
        'nonmandatory-replacement' { $coverage.tasks[2].mandatory = $false }
        'replacement-coverage-drift' {
            $records[4].task_id = 'bootstrap-superdesktop-workspace/1.1.2'
            $records[4].capability_id = 'superdesktop-workspace-foundation'
            $records[4].requirement_id = 'windows-workspace'
            $records[4].scenario_id = 'valid-workspace'
            $records[2].superseded_by = 'bootstrap-superdesktop-workspace/1.1.2#replacement'
        }
        'unpassed-replacement' { $records[4].status = 'failed' }
        'duplicate-identity' { $records += $records[0] }
        'adjustment-stale-not-propagated' { $records[2].status = 'passed'; $records[2].superseded_by = $null }
        default { Fail 'UNKNOWN_FIXTURE' $fault.kind }
    }
}

$coverageByTask = @{}
foreach ($task in $coverage.tasks) {
    Require (-not $coverageByTask.ContainsKey($task.task_id)) 'DUPLICATE_COVERAGE_TASK' $task.task_id
    $coverageByTask[$task.task_id] = $task
    Require ($coverage.capabilities -contains $task.capability_id) 'UNKNOWN_CAPABILITY' "$($task.task_id) -> $($task.capability_id)"
    Require ($task.task_id -match "^$([regex]::Escape($Change))/\d+\.\d+\.\d+$") 'INVALID_TASK_ID' $task.task_id
}
foreach ($taskId in $taskChecks.Keys) { Require ($coverageByTask.ContainsKey($taskId)) 'MISSING_COVERAGE' $taskId }
foreach ($taskId in $coverageByTask.Keys) { Require ($taskChecks.ContainsKey($taskId)) 'UNKNOWN_COVERAGE_TASK' $taskId }

$byId = @{}
foreach ($record in $records) {
    foreach ($property in @('schema_version','task_id','subcheck','status','artifact','artifact_sha256','capability_id','requirement_id','scenario_id','gates','reviewer','recorded_at')) {
        Require ($null -ne $record.$property) 'SCHEMA_MISSING_FIELD' "$property in $(Record-Id $record)"
    }
    Require ($record.schema_version -eq '1.0.0') 'SCHEMA_VERSION' (Record-Id $record)
    Require ($record.task_id -match "^$([regex]::Escape($Change))/\d+\.\d+\.\d+$") 'INVALID_TASK_ID' $record.task_id
    Require ($record.subcheck -match '^[a-z0-9][a-z0-9-]*$') 'INVALID_SUBCHECK' (Record-Id $record)
    Require ($record.status -in @('passed','failed','blocked','not-applicable','stale')) 'INVALID_STATUS' (Record-Id $record)
    $recordId = Record-Id $record
    Require (-not $byId.ContainsKey($recordId)) 'DUPLICATE_RECORD_IDENTITY' $recordId
    $byId[$recordId] = $record
    Require ($coverageByTask.ContainsKey($record.task_id)) 'UNKNOWN_TASK' $record.task_id
    $mapping = $coverageByTask[$record.task_id]
    Require ($record.capability_id -eq $mapping.capability_id) 'COVERAGE_DRIFT_CAPABILITY' $recordId
    Require ($record.requirement_id -eq $mapping.requirement_id) 'COVERAGE_DRIFT_REQUIREMENT' $recordId
    Require ($record.scenario_id -eq $mapping.scenario_id) 'COVERAGE_DRIFT_SCENARIO' $recordId
    Require ((@($record.gates) -join '|') -eq (@($mapping.gates) -join '|')) 'COVERAGE_DRIFT_GATE' $recordId
    $artifact = Join-Path $changeRoot $record.artifact
    Require (Test-Path $artifact) 'MISSING_ARTIFACT' "$recordId -> $($record.artifact)"
    Require ((Get-FileHash -Algorithm SHA256 $artifact).Hash -eq $record.artifact_sha256) 'ARTIFACT_HASH_DRIFT' $recordId
    [DateTimeOffset]::Parse($record.recorded_at) | Out-Null
}

foreach ($record in $records) {
    $recordId = Record-Id $record
    if ($record.replaces) { Require ($byId.ContainsKey($record.replaces)) 'DANGLING_REPLACEMENT' "$recordId -> $($record.replaces)" }
    if ($record.superseded_by) { Require ($byId.ContainsKey($record.superseded_by)) 'DANGLING_SUPERSEDED_BY' "$recordId -> $($record.superseded_by)" }
    if ($record.status -eq 'stale') {
        Require (-not [string]::IsNullOrWhiteSpace([string]$record.superseded_by)) 'STALE_WITHOUT_REPLACEMENT' $recordId
        $replacement = $byId[$record.superseded_by]
        Require ($replacement.replaces -eq $recordId) 'REPLACEMENT_BACKLINK_MISSING' $recordId
        Require ($replacement.status -eq 'passed') 'UNPASSED_REPLACEMENT' $recordId
        $oldMapping = $coverageByTask[$record.task_id]; $newMapping = $coverageByTask[$replacement.task_id]
        Require ($replacement.task_id -eq $record.task_id -and $newMapping.mandatory -and $oldMapping.capability_id -eq $newMapping.capability_id -and $oldMapping.requirement_id -eq $newMapping.requirement_id -and $oldMapping.scenario_id -eq $newMapping.scenario_id -and ((@($oldMapping.gates) -join '|') -eq (@($newMapping.gates) -join '|'))) 'REPLACEMENT_COVERAGE_DRIFT' $recordId
    }
}

function Visit-Replacement([string]$Id, [hashtable]$Visiting, [hashtable]$Visited) {
    if ($Visiting.ContainsKey($Id)) { Fail 'CYCLIC_REPLACEMENT' $Id }
    if ($Visited.ContainsKey($Id)) { return }
    $Visiting[$Id] = $true
    $record = $byId[$Id]
    if ($record.superseded_by) { Visit-Replacement $record.superseded_by $Visiting $Visited }
    $Visiting.Remove($Id); $Visited[$Id] = $true
}
foreach ($recordId in $byId.Keys) { Visit-Replacement $recordId @{} @{} }

foreach ($adjustment in $adjustments) {
    foreach ($taskId in @($adjustment.stale_evidence)) {
        $staleRecords = @($records | Where-Object { $_.task_id -eq $taskId -and $_.status -eq 'stale' })
        Require ($staleRecords.Count -gt 0) 'ADJUSTMENT_STALE_NOT_PROPAGATED' "$($adjustment.adjustment_id) -> $taskId"
    }
}

foreach ($taskId in $coverageByTask.Keys) {
    $mapping = $coverageByTask[$taskId]
    $passed = @($records | Where-Object { $_.task_id -eq $taskId -and $_.status -eq 'passed' })
    $invalidMandatory = @($records | Where-Object { $_.task_id -eq $taskId -and $_.status -in @('not-applicable','blocked') })
    if ($mapping.mandatory) {
        Require ($invalidMandatory.Count -eq 0) 'MANDATORY_NONPASS_STATUS' $taskId
        Require ($passed.Count -gt 0) 'MANDATORY_WITHOUT_PASSED_EVIDENCE' $taskId
    }
    if ($taskChecks[$taskId]) { Require ($passed.Count -gt 0) 'TASK_CHECKBOX_WITHOUT_EVIDENCE' $taskId }
    if (-not $taskChecks[$taskId]) { Require ($passed.Count -eq 0) 'UNCHECKED_TASK_HAS_PASSED_EVIDENCE' $taskId }
}

if (-not $Quiet) { Write-Output "Evidence validation passed: $($records.Count) append-only records cover $($coverageByTask.Count) tasks." }
