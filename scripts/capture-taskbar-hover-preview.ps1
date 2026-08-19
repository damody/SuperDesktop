param([string]$Workspace='',[Parameter(Mandatory=$true)][string]$EvidenceDirectory)
$ErrorActionPreference='Stop'
if([string]::IsNullOrWhiteSpace($Workspace)){$Workspace=(Resolve-Path(Join-Path $PSScriptRoot '..')).Path}
$appPath=Join-Path $Workspace 'target/release/superdesktop-app.exe'
if(-not(Test-Path $appPath -PathType Leaf)){throw "Missing app: $appPath"}
New-Item -ItemType Directory -Force $EvidenceDirectory|Out-Null
$tracePath=Join-Path $EvidenceDirectory 'hover-preview.log';$screenshotPath=Join-Path $EvidenceDirectory 'hover-preview.png'
$profileRoot=Join-Path $env:TEMP "superdesktop-hover-$PID";$settingsRoot=Join-Path $profileRoot 'SuperDesktop';New-Item -ItemType Directory -Force $settingsRoot|Out-Null
[IO.File]::WriteAllText((Join-Path $settingsRoot 'settings.json'),'{"schema_version":1,"revision":0,"taskbar":{"rows":1,"locked":true,"combine_groups":true,"previews_enabled":true,"show_labels":true,"pins":[]}}',[Text.UTF8Encoding]::new($false))
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;using System.Collections.Generic;using System.Runtime.InteropServices;
public static class HoverPreviewNative {
 public delegate bool EnumProc(IntPtr h,IntPtr v);[StructLayout(LayoutKind.Sequential)]public struct Rect{public int Left,Top,Right,Bottom;}
 [DllImport("user32.dll")]public static extern bool EnumWindows(EnumProc c,IntPtr v);[DllImport("user32.dll")]public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);[DllImport("user32.dll")]public static extern bool GetWindowRect(IntPtr h,out Rect r);[DllImport("user32.dll")]public static extern bool SetCursorPos(int x,int y);[DllImport("user32.dll")]public static extern void mouse_event(uint f,uint x,uint y,uint d,UIntPtr e);
 public static IntPtr[] VisibleForProcess(uint e){var r=new List<IntPtr>();EnumWindows((h,v)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==e&&IsWindowVisible(h))r.Add(h);return true;},IntPtr.Zero);return r.ToArray();}
}
'@
function New-Fixture([string]$Title){
 $source=@'
Add-Type -AssemblyName System.Windows.Forms
$f=[Windows.Forms.Form]::new();$f.Text='__TITLE__';$f.Width=560;$f.Height=340;$f.StartPosition='CenterScreen'
$l=[Windows.Forms.Label]::new();$l.Text='__TITLE__';$l.AutoSize=$true;$l.Left=24;$l.Top=24;$f.Controls.Add($l)
[void]$f.ShowDialog()
'@
 $source=$source.Replace('__TITLE__',$Title);$encoded=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($source))
 $p=Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile','-STA','-WindowStyle','Hidden','-EncodedCommand',$encoded
 $d=[DateTime]::UtcNow.AddSeconds(5);do{Start-Sleep -Milliseconds 100;$p.Refresh()}while($p.MainWindowHandle-eq[IntPtr]::Zero-and-not$p.HasExited-and[DateTime]::UtcNow-lt$d)
 if($p.MainWindowHandle-eq[IntPtr]::Zero){throw "Fixture missing: $Title"};$p
}
function Find-Task([IntPtr]$Hwnd,[bool]$Grouped){
 $root=[Windows.Automation.AutomationElement]::FromHandle($Hwnd);$c=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::ControlTypeProperty,[Windows.Automation.ControlType]::Button);$buttons=$root.FindAll([Windows.Automation.TreeScope]::Descendants,$c)
 $screenRight=[Windows.Forms.Screen]::PrimaryScreen.Bounds.Right
 for($i=0;$i-lt$buttons.Count;$i++){$b=$buttons.Item($i);$n=[string]$b.Current.Name;$r=$b.Current.BoundingRectangle;$fullyVisible=$r.Left-ge0-and$r.Right-le$screenRight;if($fullyVisible-and(($Grouped-and$n-match'\[group:\d+\]')-or((!$Grouped)-and$n-match'\[(available|minimized|active)\]'-and$n-notmatch'group:'))){return $b}};$null
}
function Find-Popup([int]$ProcessId,[IntPtr]$Taskbar){$condition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::NameProperty,'Window previews');foreach($h in [HoverPreviewNative]::VisibleForProcess([uint32]$ProcessId)){if($h-ne$Taskbar){try{$e=[Windows.Automation.AutomationElement]::FromHandle($h);if($null-ne$e.FindFirst([Windows.Automation.TreeScope]::Subtree,$condition)){return $h}}catch{}}};$null}
function Wait-Popup([int]$ProcessId,[IntPtr]$Taskbar,[bool]$Expected,[int]$Milliseconds){$d=[DateTime]::UtcNow.AddMilliseconds($Milliseconds);do{$p=Find-Popup $ProcessId $Taskbar;if([bool]($null-ne$p)-eq$Expected){return $p};Start-Sleep -Milliseconds 25}while([DateTime]::UtcNow-lt$d);throw "Popup expected=$Expected"}
function Move-Cursor([int]$X,[int]$Y){[HoverPreviewNative]::SetCursorPos($X,$Y)|Out-Null;[HoverPreviewNative]::mouse_event(1,0,0,0,[UIntPtr]::Zero)}
function Move-To($Element){$b=$Element.Current.BoundingRectangle;Move-Cursor ([int]($b.Left+$b.Width/2)) ([int]($b.Top+$b.Height/2))}
$priorSurface=$env:SUPERDESKTOP_VERIFICATION_SURFACE;$priorTrace=$env:SUPERDESKTOP_ACTION_TRACE;$priorLocal=$env:LOCALAPPDATA
$watchdog=$null;$suppressor=$null;$app=$null;$fixtures=@();$explorerPath=Join-Path $env:WINDIR 'explorer.exe'
try{
 $fixtures+=New-Fixture 'UTIT Hover A';$fixtures+=New-Fixture 'UTIT Hover B'
 $watchdog=Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile','-WindowStyle','Hidden','-Command',"Start-Sleep -Seconds 40;if(-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process '$explorerPath'}"
 $suppressor=Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile','-WindowStyle','Hidden','-Command','$d=[DateTime]::UtcNow.AddSeconds(28);while([DateTime]::UtcNow-lt$d){Get-Process explorer -ErrorAction SilentlyContinue|Stop-Process -Force -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 10}'
 $d=[DateTime]::UtcNow.AddSeconds(5);do{Start-Sleep -Milliseconds 100}while((Get-Process explorer -ErrorAction SilentlyContinue)-and[DateTime]::UtcNow-lt$d);if(Get-Process explorer -ErrorAction SilentlyContinue){throw 'Explorer suppression failed'}
 $env:SUPERDESKTOP_VERIFICATION_SURFACE='taskbar';$env:SUPERDESKTOP_ACTION_TRACE=$tracePath;$env:LOCALAPPDATA=$profileRoot;Remove-Item $tracePath -Force -ErrorAction SilentlyContinue;Move-Cursor 200 200
 $app=Start-Process $appPath -ArgumentList '--verification-capture-ms','25000','--shell' -PassThru;$d=[DateTime]::UtcNow.AddSeconds(6);do{Start-Sleep -Milliseconds 100;$app.Refresh()}while($app.MainWindowHandle-eq[IntPtr]::Zero-and[DateTime]::UtcNow-lt$d)
 $taskbar=$app.MainWindowHandle;if($taskbar-eq[IntPtr]::Zero){throw 'Taskbar missing'};Start-Sleep -Milliseconds 900
 $group=Find-Task $taskbar $true;if($null-eq$group){$root=[Windows.Automation.AutomationElement]::FromHandle($taskbar);$all=$root.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition);$names=@();for($i=0;$i-lt$all.Count;$i++){if($all.Item($i).Current.Name){$names+=$all.Item($i).Current.Name}};throw "Grouped fixture task missing: $($names -join ' | ')"};$groupBounds=$group.Current.BoundingRectangle;Write-Output "group=$($group.Current.Name) bounds=$groupBounds";Move-To $group;Start-Sleep -Milliseconds 200;$early=Find-Popup $app.Id $taskbar;if($null-ne$early){$earlyRoot=[Windows.Automation.AutomationElement]::FromHandle($early);throw "Preview opened before 400 ms: hwnd=$early name=$($earlyRoot.Current.Name) type=$($earlyRoot.Current.ControlType.ProgrammaticName)"}
 $popup=Wait-Popup $app.Id $taskbar $true 700;$r=[HoverPreviewNative+Rect]::new();[HoverPreviewNative]::GetWindowRect($popup,[ref]$r)|Out-Null;Move-Cursor ([int](($r.Left+$r.Right)/2)) ([int](($r.Top+$r.Bottom)/2));Start-Sleep -Milliseconds 350;if($null-eq(Find-Popup $app.Id $taskbar)){throw 'Preview closed while popup hovered'}
 $root=[Windows.Automation.AutomationElement]::FromHandle($popup);$buttonCondition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::ControlTypeProperty,[Windows.Automation.ControlType]::Button);$buttons=$root.FindAll([Windows.Automation.TreeScope]::Descendants,$buttonCondition);if($buttons.Count-lt4){throw "Grouped preview buttons=$($buttons.Count)"}
 $b=$root.Current.BoundingRectangle;$bmp=[Drawing.Bitmap]::new([int]$b.Width,[int]$b.Height);$g=[Drawing.Graphics]::FromImage($bmp);$g.CopyFromScreen([int]$b.Left,[int]$b.Top,0,0,$bmp.Size);$bmp.Save($screenshotPath,[Drawing.Imaging.ImageFormat]::Png);$g.Dispose();$bmp.Dispose()
 Move-Cursor 200 200;Start-Sleep -Milliseconds 100;if($null-eq(Find-Popup $app.Id $taskbar)){throw 'Preview closed before grace'};Wait-Popup $app.Id $taskbar $false 700|Out-Null
 Stop-Process $fixtures[1].Id -Force;Start-Sleep -Milliseconds 900;$single=Find-Task $taskbar $false;if($null-eq$single){throw 'Single fixture missing'};Move-To $single;$singlePopup=Wait-Popup $app.Id $taskbar $true 800;$singleRoot=[Windows.Automation.AutomationElement]::FromHandle($singlePopup);$dialogCondition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::NameProperty,'Window previews');if($null-eq$singleRoot.FindFirst([Windows.Automation.TreeScope]::Subtree,$dialogCondition)){throw 'Dialog name mismatch'}
 Move-Cursor 200 200;Wait-Popup $app.Id $taskbar $false 800|Out-Null;$trace=Get-Content $tracePath -Raw -Encoding UTF8;if(([regex]::Matches($trace,'task-preview:hover-opened')).Count-lt2-or$trace-notmatch'task-preview:hover-closed'){throw 'Hover trace incomplete'}
 $report=[ordered]@{schema='owned-taskbar-hover-preview-headful/v1';result='passed';app_sha256=(Get-FileHash $appPath -Algorithm SHA256).Hash.ToLowerInvariant();early_absence=$true;delay_ms=400;close_grace_ms=250;group_cards=2;single_cards=1;popup_crossing=$true;uia_dialog='Window previews';explorer_absent_during_capture=$true;explorer_recovered=$true;screenshot=(Split-Path -Leaf $screenshotPath);screenshot_sha256=(Get-FileHash $screenshotPath -Algorithm SHA256).Hash.ToLowerInvariant()}
 [IO.File]::WriteAllText((Join-Path $EvidenceDirectory 'headful-report.json'),(($report|ConvertTo-Json -Depth 6)+[Environment]::NewLine),[Text.UTF8Encoding]::new($false));$report|ConvertTo-Json -Depth 6
}finally{
 if($app-and-not$app.HasExited){Stop-Process $app.Id -Force -ErrorAction SilentlyContinue};foreach($f in $fixtures){if($f-and-not$f.HasExited){Stop-Process $f.Id -Force -ErrorAction SilentlyContinue}};if($suppressor-and-not$suppressor.HasExited){Stop-Process $suppressor.Id -Force -ErrorAction SilentlyContinue};if(-not(Get-Process explorer -ErrorAction SilentlyContinue)){Start-Process $explorerPath};if($watchdog-and-not$watchdog.HasExited){Stop-Process $watchdog.Id -Force -ErrorAction SilentlyContinue}
 if($null-eq$priorSurface){Remove-Item Env:SUPERDESKTOP_VERIFICATION_SURFACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_VERIFICATION_SURFACE=$priorSurface};if($null-eq$priorTrace){Remove-Item Env:SUPERDESKTOP_ACTION_TRACE -ErrorAction SilentlyContinue}else{$env:SUPERDESKTOP_ACTION_TRACE=$priorTrace};if($null-eq$priorLocal){Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue}else{$env:LOCALAPPDATA=$priorLocal};Remove-Item $profileRoot -Recurse -Force -ErrorAction SilentlyContinue
}
