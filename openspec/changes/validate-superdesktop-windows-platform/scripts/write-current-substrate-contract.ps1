[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$OutputPath)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
$out=if($OutputPath){$OutputPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-inputs-successor-1.2.sha256'}
$paths=@('Cargo.toml','Cargo.lock','.cargo/config.toml','crates/desktop-ui/Cargo.toml','scripts/architecture-allowlist.json','vendor/gpui_windows/.cargo-checksum.json','vendor/raw-window-handle/.cargo-checksum.json','vendor/windows/.cargo-checksum.json')
$lines=@()
foreach($relative in $paths){$full=Join-Path $WorkspaceRoot $relative;if(-not(Test-Path -LiteralPath $full -PathType Leaf)){throw "CURRENT_SUBSTRATE_INPUT_MISSING: $relative"};$lines+="{0}  {1}" -f (Get-FileHash -Algorithm SHA256 -LiteralPath $full).Hash,$relative.Replace('\','/')}
New-Item -ItemType Directory -Force (Split-Path -Parent $out)|Out-Null
[IO.File]::WriteAllLines($out,$lines,[Text.UTF8Encoding]::new($false))
Write-Output "Current substrate manifest written: $out"
