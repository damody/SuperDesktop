param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [ValidateRange(500,1200)][int]$TargetWidthDip = 820,
    [ValidateRange(420,850)][int]$TargetHeightDip = 620
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) { $Workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path }
$appPath = Join-Path $Workspace 'target/release/superdesktop-app.exe'
if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) { throw "Missing release app: $appPath" }
New-Item -ItemType Directory -Path $EvidenceDirectory -Force | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path
$profileRoot = Join-Path $EvidenceDirectory 'profile'
$settingsRoot = Join-Path $profileRoot 'SuperDesktop'
$settingsPath = Join-Path $settingsRoot 'settings.json'
New-Item -ItemType Directory -Path $settingsRoot -Force | Out-Null
[IO.File]::WriteAllText($settingsPath,'{"schema_version":1,"revision":0,"taskbar":{"rows":1,"alignment":"left"}}',[Text.UTF8Encoding]::new($false))

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class StartResizeNative {
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
  public delegate bool EnumProc(IntPtr hwnd,IntPtr state);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback,IntPtr state);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd,out Rect rect);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
  [DllImport("user32.dll")] static extern IntPtr GetWindowLongPtrW(IntPtr hwnd,int index);
  [DllImport("user32.dll")] static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
  public static IntPtr[] VisibleForProcess(uint wanted) { var result=new List<IntPtr>(); EnumWindows((hwnd,state)=>{uint pid;GetWindowThreadProcessId(hwnd,out pid);if(pid==wanted&&IsWindowVisible(hwnd))result.Add(hwnd);return true;},IntPtr.Zero);return result.ToArray(); }
  public static int[] Bounds(IntPtr hwnd) { Rect r;if(!GetWindowRect(hwnd,out r))throw new System.ComponentModel.Win32Exception();return new[]{r.Left,r.Top,r.Right,r.Bottom}; }
  public static long Style(IntPtr hwnd) { return GetWindowLongPtrW(hwnd,-16).ToInt64(); }
  public static void DragTopRight(int left,int top,int right,int bottom,int targetWidth,int targetHeight) {
    int x0=right-2,y0=top+2,x1=left+targetWidth,y1=bottom-targetHeight;
    SetCursorPos(x0,y0);mouse_event(0x0002,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(120);
    for(int step=1;step<=24;step++){SetCursorPos(x0+(x1-x0)*step/24,y0+(y1-y0)*step/24);System.Threading.Thread.Sleep(20);}
    mouse_event(0x0004,0,0,0,UIntPtr.Zero);
  }
}
'@

function Wait-Until([scriptblock]$Condition,[int]$Milliseconds,[string]$Failure) { $deadline=[DateTime]::UtcNow.AddMilliseconds($Milliseconds);do{$value=&$Condition;if($value){return $value};Start-Sleep -Milliseconds 50}while([DateTime]::UtcNow-lt$deadline);throw $Failure }
function Find-Named($Root,[string]$Name) { $condition=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,$Name);$Root.FindFirst([Windows.Automation.TreeScope]::Descendants,$condition) }
function Start-App([string]$Suffix) {
  $stdout=Join-Path $EvidenceDirectory "$Suffix.stdout.log";$stderr=Join-Path $EvidenceDirectory "$Suffix.stderr.log"
  $process=Start-Process -FilePath $appPath -ArgumentList '--verification-owned-hotkey-capture-ms','55000' -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
  Wait-Until {$process.Refresh();$process.MainWindowHandle-ne[IntPtr]::Zero} 8000 "Taskbar did not appear for $Suffix"|Out-Null
  return $process
}
function Open-Start($Process) {
  $button=Wait-Until {$Process.Refresh();$taskbar=[Windows.Automation.AutomationElement]::FromHandle($Process.MainWindowHandle);if($taskbar){Find-Named $taskbar 'Start'}else{$null}} 5000 'Start button unavailable'
  $button.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
  return Wait-Until {
    foreach($candidate in @([StartResizeNative]::VisibleForProcess([uint32]$Process.Id))|Where-Object{$_-ne$Process.MainWindowHandle}){
      try{$root=[Windows.Automation.AutomationElement]::FromHandle([IntPtr]$candidate)}catch{continue}
      if($root-and((Find-Named $root 'Pinned')-or(Find-Named $root 'All apps'))){return $candidate}
    }
    $null
  } 5000 'Start window did not appear'
}
function Close-Start($Process) {
  $button=Wait-Until {$Process.Refresh();$taskbar=[Windows.Automation.AutomationElement]::FromHandle($Process.MainWindowHandle);if($taskbar){Find-Named $taskbar 'Start'}else{$null}} 5000 'Start button unavailable for close';$button.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke();Start-Sleep -Milliseconds 400
}
function Logical-Bounds([IntPtr]$Handle) { $r=[StartResizeNative]::Bounds($Handle);$dpi=[StartResizeNative]::GetDpiForWindow($Handle);$scale=$dpi/96.0;[ordered]@{left=$r[0];top=$r[1];right=$r[2];bottom=$r[3];width_dip=($r[2]-$r[0])/$scale;height_dip=($r[3]-$r[1])/$scale;dpi=$dpi} }

$priorLocal=$env:LOCALAPPDATA;$priorSurface=$env:SUPERDESKTOP_VERIFICATION_SURFACE;$priorLocale=$env:SUPERDESKTOP_LOCALE;$priorTrace=$env:SUPERDESKTOP_ACTION_TRACE
$tracePath=Join-Path $EvidenceDirectory 'start-resize.log';$env:LOCALAPPDATA=$profileRoot;$env:SUPERDESKTOP_VERIFICATION_SURFACE='taskbar';$env:SUPERDESKTOP_LOCALE='en-US';$env:SUPERDESKTOP_ACTION_TRACE=$tracePath
$first=$null;$second=$null;$watchdog=$null;$suppressor=$null;$explorerPath=Join-Path $env:WINDIR 'explorer.exe';$priorExplorer=[bool](Get-Process explorer -ErrorAction SilentlyContinue)
try {
  if($priorExplorer){$watchdog=Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile','-WindowStyle','Hidden','-Command',"Start-Sleep -Seconds 100;if(-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process '$explorerPath'}"}
  $suppressor=Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile','-WindowStyle','Hidden','-Command','$deadline=[DateTime]::UtcNow.AddSeconds(90);while([DateTime]::UtcNow-lt$deadline){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 10}'
  Wait-Until {-not(Get-Process explorer -ErrorAction SilentlyContinue)} 6000 'Explorer suppression failed'|Out-Null
  $first=Start-App 'first';$handle=Open-Start $first;$before=Logical-Bounds $handle
  $startStyle=[StartResizeNative]::Style($handle);if(($startStyle-band 0x00040000)-eq0){throw "Start window lacks WS_THICKFRAME: style=0x$('{0:X}' -f $startStyle)"}
  $scale=$before.dpi/96.0;[StartResizeNative]::DragTopRight($before.left,$before.top,$before.right,$before.bottom,[int]($TargetWidthDip*$scale),[int]($TargetHeightDip*$scale))
  $resized=Wait-Until {$current=Logical-Bounds $handle;if([Math]::Abs($current.width_dip-$TargetWidthDip)-le20-and[Math]::Abs($current.height_dip-$TargetHeightDip)-le20){$current}else{$null}} 6000 'Start did not reach target size'
  $saved=Wait-Until {try{$value=Get-Content -Raw $settingsPath|ConvertFrom-Json;if($value.start.width_dip-and$value.start.height_dip){$value}else{$null}}catch{$null}} 5000 'Start size was not persisted'
  Close-Start $first;$reopenedHandle=Open-Start $first;$reopened=Logical-Bounds $reopenedHandle
  if([Math]::Abs($reopened.width_dip-$resized.width_dip)-gt4-or[Math]::Abs($reopened.height_dip-$resized.height_dip)-gt4){throw 'Start size did not survive reopen'}
  Close-Start $first;Stop-Process -Id $first.Id -Force;$first=$null
  $second=Start-App 'second';$restartHandle=Open-Start $second;$restarted=Logical-Bounds $restartHandle
  if([Math]::Abs($restarted.width_dip-$reopened.width_dip)-gt4-or[Math]::Abs($restarted.height_dip-$reopened.height_dip)-gt4){throw 'Start size did not survive process restart'}
  $trace=Get-Content -Raw $tracePath;$persistCount=([regex]::Matches($trace,'start:size-persisted')).Count
  if($persistCount-lt1-or$persistCount-gt2){throw "Unexpected persistence write count: $persistCount"}
  $stderr=((Get-Content (Join-Path $EvidenceDirectory 'first.stderr.log') -Raw -ErrorAction SilentlyContinue)+(Get-Content (Join-Path $EvidenceDirectory 'second.stderr.log') -Raw -ErrorAction SilentlyContinue))
  if($stderr-match'panicked|RefCell already borrowed|start:size-persist-failed'){throw "Start resize error signature: $stderr"}
  $report=[ordered]@{schema='owned-start-resize-persistence/v1';result='passed';app_sha256=(Get-FileHash $appPath -Algorithm SHA256).Hash;target=[ordered]@{width_dip=$TargetWidthDip;height_dip=$TargetHeightDip};before=$before;resized=$resized;saved=[ordered]@{width_dip=$saved.start.width_dip;height_dip=$saved.start.height_dip;revision=$saved.revision};reopened=$reopened;restarted=$restarted;native_thickframe=$true;persist_count=$persistCount;contained=$true;sensitive_artifacts=@()}
  [IO.File]::WriteAllText((Join-Path $EvidenceDirectory 'report.json'),(($report|ConvertTo-Json -Depth 8)+"`n"),[Text.UTF8Encoding]::new($false));$report|ConvertTo-Json -Depth 8
} finally {
  foreach($process in @($first,$second)){if($process-and-not$process.HasExited){Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue}}
  if($suppressor-and-not$suppressor.HasExited){Stop-Process -Id $suppressor.Id -Force -ErrorAction SilentlyContinue};if($priorExplorer-and-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process $explorerPath};if($watchdog-and-not$watchdog.HasExited){Stop-Process -Id $watchdog.Id -Force -ErrorAction SilentlyContinue}
  if($null-eq$priorLocal){Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue}else{$env:LOCALAPPDATA=$priorLocal};if($null-eq$priorSurface){Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface};if($null-eq$priorLocale){Remove-Item Env:SUPERDESKTOP_LOCALE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_LOCALE=$priorLocale};if($null-eq$priorTrace){Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_ACTION_TRACE=$priorTrace}
}
