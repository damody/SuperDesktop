[CmdletBinding()]
param([string]$WorkspaceRoot)
$ErrorActionPreference='Stop';if(-not $WorkspaceRoot){$WorkspaceRoot=Split-Path -Parent $PSScriptRoot};$a=Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace/evidence/artifacts/2.5';New-Item -ItemType Directory -Force $a|Out-Null
$change=Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace'
function LegacyPaths([string]$Artifact) {
  $path=Join-Path $change "evidence/artifacts/$Artifact"
  if(-not(Test-Path $path)){throw "LEGACY_CONTRACT_MISSING: $Artifact"}
  return @(Get-Content $path|?{$_ -match '^[A-F0-9]{64}  (?<p>.+)$'}|%{$Matches.p -replace '\\','/'})
}
$sets=@{
'workspace-current-inputs.sha256'=@('Cargo.toml','scripts/architecture-allowlist.json','scripts/check-dependency-architecture.ps1')+(Get-ChildItem "$WorkspaceRoot/crates" -Recurse -File -Include *.rs,Cargo.toml|%{$_.FullName.Substring($WorkspaceRoot.Length+1)})
'dependency-current-inputs.sha256'=@('Cargo.toml','Cargo.lock','.cargo/config.toml','rust-toolchain.toml','openspec/changes/bootstrap-superdesktop-workspace/evidence/artifacts/1.2/dependency-provenance.json')+(LegacyPaths '1.2/dependency-inputs.sha256')+(LegacyPaths '1.2/workspace-contract-replacement.sha256')
'source-boundary-current-inputs.sha256'=@('Cargo.toml','Cargo.lock','.cargo/config.toml','scripts/audit-source-boundary.ps1','scripts/generate-license-inventory.ps1','scripts/capture-workspace-2.2-evidence.ps1','openspec/changes/bootstrap-superdesktop-workspace/compliance/source-boundary-policy.md','openspec/changes/bootstrap-superdesktop-workspace/compliance/reviewer-disposition.md','openspec/changes/bootstrap-superdesktop-workspace/compliance/third-party-license-inventory.json','fixtures/source-boundary/pexplorer-derived-source/lib.rs','fixtures/source-boundary/superexplorer-path-dependency/Cargo.toml')+(LegacyPaths '2.2/source-boundary-contract-inputs.sha256')
}
foreach($name in $sets.Keys){$lines=$sets[$name]|Sort-Object|%{"$((Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot $_)).Hash)  $_"};Set-Content -Encoding utf8 (Join-Path $a $name) $lines; & "$PSScriptRoot/verify-contract-manifest.ps1" -WorkspaceRoot $WorkspaceRoot -Manifest (Join-Path $a $name)}
