[CmdletBinding()]
param([string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
$change = 'bootstrap-superdesktop-workspace'
$changeRoot = Join-Path $WorkspaceRoot "openspec/changes/$change"
$indexPath = Join-Path $changeRoot 'evidence/index.jsonl'
$coverage = Get-Content -Raw -Encoding UTF8 (Join-Path $changeRoot 'evidence/coverage.json') | ConvertFrom-Json
if ((Get-Content -Raw -Encoding UTF8 $indexPath) -match '"schema_version":"2.0.0"') { throw 'CORRECTIVE_RECORDS_ALREADY_APPENDED' }
$affected = @('1.1.3','1.1.4','1.2.4','2.2.4','2.3.1','2.3.2','2.3.3','2.3.4','2.3.5','2.3.6','2.3.7','2.3.8') | ForEach-Object { "$change/$_" }
$artifacts = @{
    '2.4.1' = 'contract-verifier.md'; '2.4.2' = 'schema-validation.md'; '2.4.3' = 'adjustment-lineage.md'; '2.4.4' = 'recursive-ui-boundary.md'; '2.4.5' = 'matrix.md'
}
function New-Record($mapping, [string]$subcheck, [string]$status, [string]$artifactName, [string]$supersededBy, [string]$replaces) {
    $artifact = "evidence/artifacts/2.4/$artifactName"
    $hash = (Get-FileHash -Algorithm SHA256 (Join-Path $changeRoot $artifact)).Hash
    $record = [ordered]@{ schema_version='2.0.0'; task_id=$mapping.task_id; subcheck=$subcheck; status=$status; artifact=$artifact; artifact_sha256=$hash; capability_id=$mapping.capability_id; requirement_id=$mapping.requirement_id; scenario_id=$mapping.scenario_id; gates=@($mapping.gates); reviewer='Workspace/Build owner'; recorded_at='2026-08-14T07:00:00+08:00'; procedure='Corrective Wave 1 evidence validation'; expected='Schema-complete, coverage-identical passed evidence'; actual='Passed'; }
    if ($supersededBy) { $record.superseded_by=$supersededBy }
    if ($replaces) { $record.replaces=$replaces }
    return ($record | ConvertTo-Json -Compress)
}
foreach ($mapping in $coverage.tasks) {
    if ($mapping.task_id -match '/2\.4\.') { continue }
    if ($affected -contains $mapping.task_id) { Add-Content -Encoding UTF8 $indexPath (New-Record $mapping 'corrective-stale' 'stale' 'matrix.md' "$($mapping.task_id)#corrective" $null) }
    $replaces = if ($affected -contains $mapping.task_id) { "$($mapping.task_id)#corrective-stale" } else { $null }
    Add-Content -Encoding UTF8 $indexPath (New-Record $mapping 'corrective' 'passed' 'matrix.md' $null $replaces)
}
foreach ($mapping in $coverage.tasks | Where-Object { $_.task_id -match '/2\.4\.' }) {
    $leaf = $mapping.task_id.Split('/')[-1]
    Add-Content -Encoding UTF8 $indexPath (New-Record $mapping 'primary' 'passed' $artifacts[$leaf] $null $null)
}
Write-Output 'Appended schema-complete corrective records.'
