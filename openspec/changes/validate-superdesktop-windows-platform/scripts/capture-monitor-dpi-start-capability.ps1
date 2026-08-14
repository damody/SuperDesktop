[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$OutputPath)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
if(-not $OutputPath){$OutputPath=Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/2.2/monitor-dpi-start-trace.json'}
$change=Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform'
$manifest=Join-Path $change 'evidence/artifacts/1.1/current-substrate-inputs-successor-3.5-manifest-v4.sha256'
$probe=Join-Path $change 'evidence/artifacts/1.1/bin/capability_profile-successor-1.2-manifest-v3.exe'
$artifact=Split-Path $OutputPath -Parent
$admission=Join-Path $artifact 'pre-mutation-admission-trace.json'

if(-not ('Wave2MonitorExternalSnapshot' -as [type])){
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Wave2MonitorExternalSnapshot {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int left, top, right, bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct APPBARDATA { public uint cbSize; public IntPtr hWnd; public uint uCallbackMessage; public uint uEdge; public RECT rc; public IntPtr lParam; }
  [DllImport("user32.dll", SetLastError=true)] public static extern bool SystemParametersInfo(uint action, uint param, ref RECT rect, uint flags);
  [DllImport("user32.dll", SetLastError=true)] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
  [DllImport("user32.dll")] public static extern IntPtr GetThreadDpiAwarenessContext();
  [DllImport("user32.dll")] public static extern bool AreDpiAwarenessContextsEqual(IntPtr first, IntPtr second);
  [DllImport("shell32.dll", SetLastError=true)] public static extern UIntPtr SHAppBarMessage(uint message, ref APPBARDATA data);
}
'@
}
$pmv2=[IntPtr](-4)
if(-not [Wave2MonitorExternalSnapshot]::SetProcessDpiAwarenessContext($pmv2)){throw 'CAPTURE_SET_PROCESS_DPI_AWARENESS_CONTEXT_PER_MONITOR_V2_FAILED'}
$captureThreadPmV2=[Wave2MonitorExternalSnapshot]::AreDpiAwarenessContextsEqual([Wave2MonitorExternalSnapshot]::GetThreadDpiAwarenessContext(),$pmv2)
if(-not $captureThreadPmV2){throw 'CAPTURE_THREAD_DPI_AWARENESS_CONTEXT_PER_MONITOR_V2_FAILED'}
function Get-ExternalSnapshot {
  $work=[Wave2MonitorExternalSnapshot+RECT]::new()
  if(-not [Wave2MonitorExternalSnapshot]::SystemParametersInfo(48,0,[ref]$work,0)){throw 'EXTERNAL_WORKAREA_QUERY_FAILED'}
  $appbar=[Wave2MonitorExternalSnapshot+APPBARDATA]::new();$appbar.cbSize=[Runtime.InteropServices.Marshal]::SizeOf($appbar)
  $appbarResult=[Wave2MonitorExternalSnapshot]::SHAppBarMessage(5,[ref]$appbar)
  $explorers=@(Get-Process -Name explorer -ErrorAction SilentlyContinue|Sort-Object Id|ForEach-Object{[ordered]@{pid=$_.Id;session_id=$_.SessionId;path=$_.Path}})
  [ordered]@{
    explorer=$explorers
    work_area=[ordered]@{left=$work.left;top=$work.top;right=$work.right;bottom=$work.bottom}
    appbar_query=[ordered]@{found=([uint64]$appbarResult -ne 0);edge=$appbar.uEdge;rect=[ordered]@{left=$appbar.rc.left;top=$appbar.rc.top;right=$appbar.rc.right;bottom=$appbar.rc.bottom}}
  }
}

& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $change 'scripts/verify-current-substrate-contract.ps1') -WorkspaceRoot $WorkspaceRoot -ManifestPath $manifest
if($LASTEXITCODE -ne 0){throw 'CURRENT_SUBSTRATE_GATE_FAILED'}
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $change 'scripts/capture-profile-admission.ps1') -WorkspaceRoot $WorkspaceRoot -ProbePath $probe -OutputPath $admission
if($LASTEXITCODE -ne 0){throw 'PRE_MUTATION_ADMISSION_GATE_FAILED'}
Push-Location $WorkspaceRoot
try{
  cargo build -p platform-win --example monitor_dpi_start_capability --locked --offline
  if($LASTEXITCODE -ne 0){throw 'MONITOR_DPI_START_BUILD_FAILED'}
  $binary=Join-Path $WorkspaceRoot 'target/debug/examples/monitor_dpi_start_capability.exe'
  $externalBefore=Get-ExternalSnapshot
  $raw=&$binary
  $runnerExit=$LASTEXITCODE
  $externalAfter=Get-ExternalSnapshot
  if($runnerExit -ne 0){throw "MONITOR_DPI_START_RUN_FAILED:$raw"}
  $beforeCanonical=$externalBefore|ConvertTo-Json -Depth 8 -Compress
  $afterCanonical=$externalAfter|ConvertTo-Json -Depth 8 -Compress
  if($beforeCanonical -cne $afterCanonical){throw 'EXTERNAL_SNAPSHOT_MUTATION_DETECTED'}
  New-Item -ItemType Directory -Force $artifact,(Join-Path $artifact 'bin')|Out-Null
  Copy-Item $binary (Join-Path $artifact 'bin/monitor_dpi_start_capability.exe') -Force
  $trace=$raw|ConvertFrom-Json
  if($trace.explorer_mutations -isnot [bool] -or $trace.explorer_mutations){throw 'RUNNER_EXPLORER_MUTATION_FLAG_INVALID'}
  if($trace.shell_takeover -isnot [bool] -or $trace.shell_takeover){throw 'RUNNER_SHELL_TAKEOVER_FLAG_INVALID'}
  $trace|Add-Member external_snapshot ([ordered]@{capture_process_set_per_monitor_v2=$true;capture_thread_is_per_monitor_v2=$true;before=$externalBefore;after=$externalAfter;equality_passed=$true})
  $trace|Add-Member input_contract ([ordered]@{
    current_substrate_manifest_sha256=(Get-FileHash -Algorithm SHA256 $manifest).Hash
    pre_mutation_admission_trace_sha256=(Get-FileHash -Algorithm SHA256 $admission).Hash
    binary_sha256=(Get-FileHash -Algorithm SHA256 $binary).Hash
    runner_source_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot 'crates/platform-win/examples/monitor_dpi_start_capability.rs')).Hash
    adapter_source_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot 'crates/platform-win/src/common/monitor_dpi_start.rs')).Hash
  })
  $pending="$OutputPath.pending"
  $trace|ConvertTo-Json -Depth 16 -Compress|Set-Content $pending -Encoding utf8
  & (Join-Path $change 'scripts/verify-monitor-dpi-start-trace.ps1') -TracePath $pending -WorkspaceRoot $WorkspaceRoot -ArtifactDirectory $artifact
  if($LASTEXITCODE -ne 0){throw 'MONITOR_DPI_START_TRACE_INVALID'}
  Move-Item $pending $OutputPath -Force
  Write-Output "Monitor/DPI/Start trace captured: $OutputPath"
}finally{Pop-Location}
