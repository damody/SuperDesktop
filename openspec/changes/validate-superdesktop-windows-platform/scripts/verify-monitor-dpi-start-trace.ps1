[CmdletBinding()]
param([string]$TracePath,[string]$WorkspaceRoot,[string]$ArtifactDirectory)
$ErrorActionPreference='Stop'
if(-not $TracePath -or -not(Test-Path -LiteralPath $TracePath -PathType Leaf)){throw 'TRACE_INPUT_MISSING'}
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
if(-not $ArtifactDirectory){$ArtifactDirectory=Split-Path $TracePath -Parent}
$trace=Get-Content -Raw -LiteralPath $TracePath|ConvertFrom-Json
function RequireField([object]$o,[string]$n){if($null -eq $o -or $null -eq $o.PSObject.Properties[$n]){throw "TRACE_FIELD_MISSING:$n"};$o.$n}
function RequireBool([object]$o,[string]$n,[bool]$want=$true){$v=RequireField $o $n;if($v -isnot [bool]){throw "TRACE_BOOL_TYPE:$n"};if($v -ne $want){throw "TRACE_BOOL_VALUE:$n"}}
function RequireInt([object]$o,[string]$n,[int64]$min=0){$v=RequireField $o $n;if($v -isnot [int] -and $v -isnot [long]){throw "TRACE_INT_TYPE:$n"};if([int64]$v -lt $min){throw "TRACE_INT_RANGE:$n"};[int64]$v}
function RequireHash([object]$o,[string]$n){$v=RequireField $o $n;if($v -isnot [string] -or $v -notmatch '^[A-F0-9]{64}$'){throw "TRACE_HASH_INVALID:$n"}}
function SameResources([object]$a,[object]$b){foreach($n in @('process_handles','user_objects','gdi_objects')){if((RequireInt $a $n) -ne (RequireInt $b $n)){throw "TRACE_RESOURCE_DELTA:$n"}}}
function RequireRect([object]$rect,[string]$label){$left=RequireInt $rect 'left';$top=RequireInt $rect 'top';$right=RequireInt $rect 'right';$bottom=RequireInt $rect 'bottom';if($right -le $left -or $bottom -le $top){throw "TRACE_RECT_INVALID:$label"};[ordered]@{left=$left;top=$top;right=$right;bottom=$bottom}}
function RectContains([object]$outer,[object]$inner){$outer.left -le $inner.left -and $outer.top -le $inner.top -and $outer.right -ge $inner.right -and $outer.bottom -ge $inner.bottom}

if((RequireField $trace 'schema') -ne 'monitor-dpi-start-trace/v3' -or (RequireField $trace 'mode') -ne 'controlled-supported-invocation'){throw 'TRACE_SCHEMA_OR_MODE'}
RequireBool $trace 'explorer_mutations' $false;RequireBool $trace 'shell_takeover' $false;RequireBool $trace 'start_invocation_attempted'
if((RequireField $trace 'typed_disposition') -ne 'go'){throw 'TRACE_DISPOSITION_NOT_GO'}
$awareness=RequireField $trace 'dpi_awareness';RequireBool $awareness 'process_set_per_monitor_v2';RequireBool $awareness 'thread_is_per_monitor_v2';RequireBool $awareness 'geometry_virtualized' $false
$real=RequireField $trace 'real_profile';if((RequireField $real 'origin') -ne 'real-profile'){throw 'TRACE_REAL_ORIGIN'};RequireBool $real 'refresh_stable'
$monitors=@(RequireField $real 'monitors');if($monitors.Count -lt 1){throw 'TRACE_REAL_MONITORS_EMPTY'};if(@($monitors|Where-Object{$_.primary -eq $true}).Count -ne 1){throw 'TRACE_REAL_PRIMARY_INVALID'}
$primary=$null
foreach($monitor in $monitors){if([string](RequireField $monitor 'device_name') -eq ''){throw 'TRACE_DEVICE_EMPTY'};$bounds=RequireRect (RequireField $monitor 'bounds') 'monitor-bounds';$work=RequireRect (RequireField $monitor 'work_area') 'monitor-work-area';if(-not(RectContains $bounds $work)){throw 'TRACE_WORKAREA_OUTSIDE_MONITOR'};if((RequireInt $monitor 'dpi_x' 1)-lt 1 -or (RequireInt $monitor 'dpi_y' 1)-lt 1){throw 'TRACE_REAL_DPI_INVALID'};if($monitor.primary){$primary=$bounds}}
$fixture=RequireField $trace 'virtual_fixture';if((RequireField $fixture 'origin') -ne 'virtual-fixture'){throw 'TRACE_FIXTURE_ORIGIN'};RequireBool $fixture 'physical_mixed_dpi_claimed' $false
$events=@(RequireField $fixture 'events');if($events.Count -ne 4){throw 'TRACE_FIXTURE_EVENT_COUNT'}
$expected=@(
  [ordered]@{kind='added';device_name='VIRTUAL-B'},
  [ordered]@{kind='primary-changed';device_name='VIRTUAL-B'},
  [ordered]@{kind='dpi-changed';device_name='VIRTUAL-B';dpi_x=192;dpi_y=168},
  [ordered]@{kind='removed';device_name='VIRTUAL-A'}
)
for($index=0;$index -lt $expected.Count;$index++){$actual=$events[$index];foreach($key in $expected[$index].Keys){if((RequireField $actual $key) -ne $expected[$index][$key]){throw "TRACE_FIXTURE_TRANSITION_INVALID:$index/$key"}}}
$start=RequireField $trace 'start_host';if((RequireField $start 'status') -ne 'available'){throw 'TRACE_START_STATUS'};if((RequireField $start 'reason') -ne $null){throw 'TRACE_START_REASON_NOT_NULL'};RequireBool $start 'invocation_attempted';RequireBool $start 'foreground_changed';RequireBool $start 'restored';if((RequireField $start 'disposition') -ne 'go'){throw 'TRACE_START_DISPOSITION'}
if((RequireInt $start 'input_events_sent' 2) -ne 2 -or (RequireInt $start 'escape_events_sent' 2) -ne 2){throw 'TRACE_START_INPUT_COUNT'}
$observation=RequireField $start 'host_observation';if($observation.taskbar_class -ne $start.observed_taskbar_class -or $observation.taskbar_class -ne 'Shell_TrayWnd'){throw 'TRACE_START_OBSERVATION_MISMATCH'}
if((RequireField $observation 'host_class') -ne 'Windows.UI.Core.CoreWindow'){throw 'TRACE_START_HOST_CLASS'}
if((RequireInt $observation 'pid' 1) -lt 1){throw 'TRACE_START_HOST_PID'}
$hostPath=[string](RequireField $observation 'path');if($hostPath -notmatch '(?i)^C:\\Windows\\SystemApps\\.*\\(StartMenuExperienceHost|SearchHost)\.exe$'){throw 'TRACE_START_HOST_PATH_UNTRUSTED'}
$fixtures=@(RequireField $start 'fixtures');if($fixtures.Count -ne 2){throw 'TRACE_START_FIXTURE_COUNT'}
$fixtureExpectations=@('taskbar-host-not-found','untrusted-start-host-invocation-refused');foreach($requiredReason in $fixtureExpectations){$matching=@($fixtures|Where-Object{(RequireField $_ 'status') -eq 'unavailable' -and (RequireField $_ 'reason') -eq $requiredReason});if($matching.Count -ne 1){throw "TRACE_START_FIXTURE_MISSING:$requiredReason"}}
SameResources (RequireField $trace 'resources_before') (RequireField $trace 'resources_after')
$external=RequireField $trace 'external_snapshot';RequireBool $external 'capture_process_set_per_monitor_v2';RequireBool $external 'capture_thread_is_per_monitor_v2';RequireBool $external 'equality_passed';$externalBefore=RequireField $external 'before';$externalAfter=RequireField $external 'after';if(($externalBefore|ConvertTo-Json -Depth 8 -Compress) -cne ($externalAfter|ConvertTo-Json -Depth 8 -Compress)){throw 'TRACE_EXTERNAL_SNAPSHOT_DELTA'}
$beforeWork=RequireRect (RequireField $externalBefore 'work_area') 'external-work-area';$afterWork=RequireRect (RequireField $externalAfter 'work_area') 'external-work-area';if(-not(RectContains $primary $beforeWork) -or -not(RectContains $primary $afterWork)){throw 'TRACE_EXTERNAL_WORKAREA_OUTSIDE_PRIMARY'}
$beforeBar=RequireField $externalBefore 'appbar_query';$afterBar=RequireField $externalAfter 'appbar_query';RequireBool $beforeBar 'found';RequireBool $afterBar 'found';$beforeBarRect=RequireRect (RequireField $beforeBar 'rect') 'external-appbar';$afterBarRect=RequireRect (RequireField $afterBar 'rect') 'external-appbar';if(-not(RectContains $primary $beforeBarRect) -or -not(RectContains $primary $afterBarRect)){throw 'TRACE_APPBAR_OUTSIDE_PRIMARY'}
$contract=RequireField $trace 'input_contract';foreach($n in @('current_substrate_manifest_sha256','pre_mutation_admission_trace_sha256','binary_sha256','runner_source_sha256','adapter_source_sha256')){RequireHash $contract $n}
$paths=[ordered]@{current_substrate_manifest_sha256=Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-inputs-successor-3.5-manifest-v4.sha256';pre_mutation_admission_trace_sha256=Join-Path $ArtifactDirectory 'pre-mutation-admission-trace.json';binary_sha256=Join-Path $ArtifactDirectory 'bin/monitor_dpi_start_capability.exe';runner_source_sha256=Join-Path $WorkspaceRoot 'crates/platform-win/examples/monitor_dpi_start_capability.rs';adapter_source_sha256=Join-Path $WorkspaceRoot 'crates/platform-win/src/common/monitor_dpi_start.rs'}
foreach($n in $paths.Keys){if(-not(Test-Path -LiteralPath $paths[$n] -PathType Leaf)){throw "TRACE_HASH_SOURCE_MISSING:$n"};if($contract.$n -ne (Get-FileHash -Algorithm SHA256 -LiteralPath $paths[$n]).Hash){throw "TRACE_HASH_SUBSTITUTION:$n"}}
Write-Output 'Monitor/DPI/Start trace semantic validation passed; supported invocation and taskbar capability are go.'
