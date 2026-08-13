[CmdletBinding()]
param([string]$WorkspaceRoot)
$ErrorActionPreference='Stop';if(-not $WorkspaceRoot){$WorkspaceRoot=Split-Path -Parent $PSScriptRoot}
$file=Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace/evidence/adjustments.jsonl'
$records=@(Get-Content ($file -replace 'adjustments\.jsonl','index.jsonl')|?{$_}|%{$_|ConvertFrom-Json})
$old=@(Get-Content $file|?{$_}|%{$_|ConvertFrom-Json}|?{$_.adjustment_id -eq 'B-W1-2.5-SUCCESSOR'})[0]
$fresh=@($records|?{$_.subcheck -eq 'wave25-final-stale'});$stale=@($old.stale_record_ids)+@($fresh|%{"$($_.task_id)#$($_.subcheck)"});$replacement=@($old.replacement_record_ids)+@($fresh|%{$_.superseded_by})
$record=[ordered]@{adjustment_id='B-W1-2.5-FULL-SUCCESSOR-002';classification='B';recorded_at='2026-08-14T12:10:00+08:00';scope='complete effective corrective successor';supersedes_adjustments=@('B-W1-2.5-FULL-SUCCESSOR','B-W1-EXIT-004');stale_record_ids=$stale;replacement_record_ids=$replacement;status='replacement-passed'}
Add-Content -Encoding utf8 $file ($record|ConvertTo-Json -Compress)
