[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$OutputPath,[string]$CurrentSubstratePath,[string]$ProbeBinaryPath,[string]$AdmissionTracePath,[string]$VendorProvenancePath)

$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path }
$change = Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform'
$artifacts = Join-Path $change 'evidence/artifacts/1.1'
$allowlist = Join-Path $PSScriptRoot 'profile-allowlist.json'
$programHandoffPath = Join-Path $WorkspaceRoot 'openspec/changes/build-superdesktop-shell-foundation/evidence/handoffs/2.1.json'
$programHandoff = Get-Content -Raw -Encoding utf8 $programHandoffPath | ConvertFrom-Json
$profile = Get-Content -Raw -Encoding utf8 (Join-Path $artifacts 'profile-snapshot.json') | ConvertFrom-Json
$ep = Get-Content -Raw -Encoding utf8 (Join-Path $artifacts 'explorerpatcher-profile.json') | ConvertFrom-Json
$currentSubstrate = if($CurrentSubstratePath){$CurrentSubstratePath}else{Join-Path $artifacts 'current-substrate-inputs-successor-1.2-manifest-v3.sha256'}
$probeBinary = if($ProbeBinaryPath){$ProbeBinaryPath}else{Join-Path $artifacts 'bin/capability_profile-successor-1.2-manifest-v3.exe'}
$admissionTrace = if($AdmissionTracePath){$AdmissionTracePath}else{Join-Path $artifacts 'admission-zero-mutation-trace-successor-1.2-manifest-v3.json'}
$vendorProvenance=if($VendorProvenancePath){$VendorProvenancePath}else{Join-Path $artifacts 'current-substrate-v3-vendor-provenance.json'}
if (-not (Test-Path -LiteralPath $currentSubstrate -PathType Leaf)) { throw 'CURRENT_SUBSTRATE_MANIFEST_MISSING' }
$sha = { param($path) (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash }
$output = [ordered]@{
  schema_version = '1.0.0'
  contract = 'frozen-win11-explorerpatcher-profile-and-readonly-admission-probe'
  captured_at = (Get-Date).ToUniversalTime().ToString('o')
  os_session_display_monitor = [ordered]@{ snapshot_path='evidence/artifacts/1.1/profile-snapshot.json'; snapshot_sha256=&$sha (Join-Path $artifacts 'profile-snapshot.json'); values=[ordered]@{ captured_at=$profile.captured_at; os=$profile.os; session=$profile.session; display_adapters=$profile.display_adapters; monitors=$profile.monitors } }
  explorerpatcher = [ordered]@{ expected_version=$ep.expected_version; binaries=$ep.binaries; settings_snapshot_path='evidence/artifacts/1.1/explorerpatcher-profile.json'; settings_snapshot_sha256=&$sha (Join-Path $artifacts 'explorerpatcher-profile.json'); allowlist_path='scripts/profile-allowlist.json'; allowlist_sha256=&$sha $allowlist }
  reference_image = [ordered]@{ path='evidence/artifacts/1.1/reference-taskbar.jpg'; sha256=&$sha (Join-Path $artifacts 'reference-taskbar.jpg') }
  build = [ordered]@{ gpui_revision='8945e2981b9fd00ca887e042d8adb9acc241b168'; rust_toolchain_path='rust-toolchain.toml'; rust_toolchain_sha256=&$sha (Join-Path $WorkspaceRoot 'rust-toolchain.toml'); cargo_lock_path='Cargo.lock'; cargo_lock_sha256=&$sha (Join-Path $WorkspaceRoot 'Cargo.lock'); probe_source_path='crates/platform-win/examples/capability_profile.rs'; probe_source_sha256=&$sha (Join-Path $WorkspaceRoot 'crates/platform-win/examples/capability_profile.rs'); probe_binary_path='evidence/artifacts/1.1/bin/capability_profile-successor-1.2-manifest-v3.exe'; probe_binary_sha256=&$sha $probeBinary }
  current_substrate_contract = [ordered]@{ path='evidence/artifacts/1.1/current-substrate-inputs-successor-1.2-manifest-v3.sha256'; sha256=&$sha $currentSubstrate; vendor_provenance_path='evidence/artifacts/1.1/current-substrate-v3-vendor-provenance.json'; vendor_provenance_sha256=&$sha $vendorProvenance; verifier_path='scripts/verify-current-substrate-contract.ps1'; verifier_sha256=&$sha (Join-Path $PSScriptRoot 'verify-current-substrate-contract.ps1'); predecessor_archive_contract_sha256=[string]$programHandoff.child_contract_sha256 }
  archive_contract = [ordered]@{ program_handoff_path='openspec/changes/build-superdesktop-shell-foundation/evidence/handoffs/2.1.json'; program_handoff_sha256=&$sha $programHandoffPath; archive_path=[string]$programHandoff.archive_path; archive_revision=[string]$programHandoff.archive_revision; child_contract_sha256=[string]$programHandoff.child_contract_sha256; inputs_manifest='openspec/changes/archive/2026-08-13-bootstrap-superdesktop-workspace/evidence/artifacts/2.5/aggregate-contract-inputs.sha256' }
  admission_zero_mutation_trace = [ordered]@{ path='evidence/artifacts/1.1/admission-zero-mutation-trace-successor-1.2-manifest-v3.json'; sha256=&$sha $admissionTrace }
}
$destination=if($OutputPath){$OutputPath}else{Join-Path $artifacts 'frozen-profile-contract-successor-1.2-manifest-v3.json'}
$output | ConvertTo-Json -Depth 12 | Set-Content -Encoding utf8 -LiteralPath $destination
Write-Output 'Frozen profile contract written.'
