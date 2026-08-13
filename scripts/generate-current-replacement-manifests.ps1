[CmdletBinding()]
param([string]$WorkspaceRoot)
$ErrorActionPreference='Stop';if(-not $WorkspaceRoot){$WorkspaceRoot=Split-Path -Parent $PSScriptRoot};$a=Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace/evidence/artifacts/2.5';New-Item -ItemType Directory -Force $a|Out-Null
$sets=@{
'workspace-current-inputs.sha256'=@('Cargo.toml','scripts/architecture-allowlist.json','scripts/check-dependency-architecture.ps1')+(Get-ChildItem "$WorkspaceRoot/crates" -Recurse -File -Include *.rs,Cargo.toml|%{$_.FullName.Substring($WorkspaceRoot.Length+1)})
'dependency-current-inputs.sha256'=@('Cargo.toml','Cargo.lock','.cargo/config.toml','rust-toolchain.toml')
'source-boundary-current-inputs.sha256'=@('Cargo.toml','Cargo.lock','.cargo/config.toml','scripts/audit-source-boundary.ps1','scripts/generate-license-inventory.ps1','openspec/changes/bootstrap-superdesktop-workspace/compliance/source-boundary-policy.md','openspec/changes/bootstrap-superdesktop-workspace/compliance/third-party-license-inventory.json')
}
foreach($name in $sets.Keys){$lines=$sets[$name]|Sort-Object|%{"$((Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot $_)).Hash)  $_"};Set-Content -Encoding utf8 (Join-Path $a $name) $lines; & "$PSScriptRoot/verify-contract-manifest.ps1" -WorkspaceRoot $WorkspaceRoot -Manifest (Join-Path $a $name)}
