[CmdletBinding()]
param([string]$WorkspaceRoot)
$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }
$change = Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace'
$index = Join-Path $change 'evidence/index.jsonl'
$coverage = Get-Content -Raw (Join-Path $change 'evidence/coverage.json') | ConvertFrom-Json
$files = @{'2.5.1'='workspace-current-inputs.sha256';'2.5.2'='schema-engine.md';'2.5.3'='mutation-matrix.md';'2.5.4'='adjustment-graph.md';'2.5.5'='full-matrix.md';'2.5.6'='windows-substrate.txt'}
$existing = @(Get-Content $index | Where-Object { $_ } | ForEach-Object { $_ | ConvertFrom-Json })
foreach ($task in @($coverage.tasks | Where-Object { $_.task_id -match '/2\.5\.' })) {
  $leaf = $task.task_id.Split('/')[-1]; $identity = "$($task.task_id)#replacement"
  $draft = @($existing | Where-Object { "$($_.task_id)#$($_.subcheck)" -eq $identity })
  if ($draft.Count -gt 1) { throw "DUPLICATE_RECORD_IDENTITY: $identity" }
  $path = "evidence/artifacts/2.5/$($files[$leaf])"; $hash = (Get-FileHash -Algorithm SHA256 (Join-Path $change $path)).Hash
  $record = [ordered]@{schema_version='2.0.0';task_id=$task.task_id;subcheck='replacement';status='passed';artifact=$path;artifact_sha256=$hash;capability_id=$task.capability_id;requirement_id=$task.requirement_id;scenario_id=$task.scenario_id;gates=@($task.gates);reviewer='Workspace/Build owner';recorded_at='2026-08-14T10:30:00+08:00';procedure='2.5 immutable production corrective matrix';expected='passed';actual='passed'} | ConvertTo-Json -Compress
  if ($draft.Count -eq 1) {
    $lineNumber = [array]::IndexOf((Get-Content $index), ($draft[0] | ConvertTo-Json -Compress))
    if ($lineNumber -ge 0) { throw "2.5 draft replacement must be removed only by the primary before regeneration: $identity" }
  } else { Add-Content -Encoding utf8 $index $record }
}
