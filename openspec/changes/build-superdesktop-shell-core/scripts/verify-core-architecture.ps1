[CmdletBinding()]
param([string]$WorkspaceRoot)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
$manifest=Get-Content -Raw -Encoding UTF8 (Join-Path $WorkspaceRoot 'crates/shell-core/Cargo.toml')
if($manifest -match '(?m)^\[dependencies\]' -or $manifest -match '(?m)^(gpui|windows|winapi)\s*='){throw 'CORE_DEPENDENCY_BOUNDARY_VIOLATION'}
$source=(Get-ChildItem (Join-Path $WorkspaceRoot 'crates/shell-core/src') -Filter '*.rs' -File|ForEach-Object{Get-Content -Raw -Encoding UTF8 $_.FullName}) -join "`n"
foreach($pattern in @('\bHWND\b','\bPIDL\b','\bIUnknown\b','\bIShell[A-Za-z]*\b','\bwindows::','\bgpui::')){if($source -match $pattern){throw "CORE_PLATFORM_TYPE_VIOLATION:$pattern"}}
Write-Output 'Core architecture passed: no dependency section and no HWND/PIDL/COM/Windows/GPUI type references.'
