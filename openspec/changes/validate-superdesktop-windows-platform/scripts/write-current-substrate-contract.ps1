[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$OutputPath,[string]$ProvenanceOutputPath)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
$out=if($OutputPath){$OutputPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-inputs-successor-1.2-manifest-v2.sha256'}
$provenanceOut=if($ProvenanceOutputPath){$ProvenanceOutputPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-v2-vendor-provenance.json'}
# B-W2-1.2-007: the minimal locked Windows backend pulls these registry crates.
# Bind both vendor checksums and license-bearing package metadata so a later lock
# refresh cannot silently reuse the previous current-substrate disposition.
$paths=@(
  'Cargo.toml','Cargo.lock','.cargo/config.toml','crates/desktop-ui/Cargo.toml','scripts/architecture-allowlist.json',
  'vendor/gpui_windows/.cargo-checksum.json','vendor/raw-window-handle/.cargo-checksum.json','vendor/windows/.cargo-checksum.json',
  'vendor/embed-resource/.cargo-checksum.json','vendor/embed-resource/Cargo.toml','vendor/embed-resource/LICENSE',
  'vendor/serde_spanned-1.1.1/.cargo-checksum.json','vendor/serde_spanned-1.1.1/Cargo.toml','vendor/serde_spanned-1.1.1/LICENSE-APACHE','vendor/serde_spanned-1.1.1/LICENSE-MIT',
  'vendor/toml-1.1.4/.cargo-checksum.json','vendor/toml-1.1.4/Cargo.toml','vendor/toml-1.1.4/LICENSE-APACHE','vendor/toml-1.1.4/LICENSE-MIT',
  'vendor/toml_datetime-1.1.1/.cargo-checksum.json','vendor/toml_datetime-1.1.1/Cargo.toml','vendor/toml_datetime-1.1.1/LICENSE-APACHE','vendor/toml_datetime-1.1.1/LICENSE-MIT',
  'vendor/toml_parser/.cargo-checksum.json','vendor/toml_parser/Cargo.toml','vendor/toml_parser/LICENSE-APACHE','vendor/toml_parser/LICENSE-MIT',
  'vendor/toml_writer/.cargo-checksum.json','vendor/toml_writer/Cargo.toml','vendor/toml_writer/LICENSE-APACHE','vendor/toml_writer/LICENSE-MIT',
  'vendor/vswhom/.cargo-checksum.json','vendor/vswhom/Cargo.toml','vendor/vswhom/LICENSE',
  'vendor/vswhom-sys/.cargo-checksum.json','vendor/vswhom-sys/Cargo.toml','vendor/vswhom-sys/LICENSE',
  'vendor/winnow-1.0.4/.cargo-checksum.json','vendor/winnow-1.0.4/Cargo.toml','vendor/winnow-1.0.4/LICENSE-MIT',
  'vendor/winreg/.cargo-checksum.json','vendor/winreg/Cargo.toml','vendor/winreg/LICENSE'
)
$lines=@()
foreach($relative in $paths){$full=Join-Path $WorkspaceRoot $relative;if(-not(Test-Path -LiteralPath $full -PathType Leaf)){throw "CURRENT_SUBSTRATE_INPUT_MISSING: $relative"};$lines+="{0}  {1}" -f (Get-FileHash -Algorithm SHA256 -LiteralPath $full).Hash,$relative.Replace('\','/')}
New-Item -ItemType Directory -Force (Split-Path -Parent $out)|Out-Null
[IO.File]::WriteAllLines($out,$lines,[Text.UTF8Encoding]::new($false))
$packageMetadata=@(
  [ordered]@{name='embed-resource';version='3.0.11';vendor_path='vendor/embed-resource';license='MIT';license_files=@('LICENSE')},
  [ordered]@{name='serde_spanned';version='1.1.1';vendor_path='vendor/serde_spanned-1.1.1';license='MIT OR Apache-2.0';license_files=@('LICENSE-APACHE','LICENSE-MIT')},
  [ordered]@{name='toml';version='1.1.4+spec-1.1.0';vendor_path='vendor/toml-1.1.4';license='MIT OR Apache-2.0';license_files=@('LICENSE-APACHE','LICENSE-MIT')},
  [ordered]@{name='toml_datetime';version='1.1.1+spec-1.1.0';vendor_path='vendor/toml_datetime-1.1.1';license='MIT OR Apache-2.0';license_files=@('LICENSE-APACHE','LICENSE-MIT')},
  [ordered]@{name='toml_parser';version='1.1.3+spec-1.1.0';vendor_path='vendor/toml_parser';license='MIT OR Apache-2.0';license_files=@('LICENSE-APACHE','LICENSE-MIT')},
  [ordered]@{name='toml_writer';version='1.1.2+spec-1.1.0';vendor_path='vendor/toml_writer';license='MIT OR Apache-2.0';license_files=@('LICENSE-APACHE','LICENSE-MIT')},
  [ordered]@{name='vswhom';version='0.1.0';vendor_path='vendor/vswhom';license='MIT';license_files=@('LICENSE')},
  [ordered]@{name='vswhom-sys';version='0.1.3';vendor_path='vendor/vswhom-sys';license='MIT';license_files=@('LICENSE')},
  [ordered]@{name='winnow';version='1.0.4';vendor_path='vendor/winnow-1.0.4';license='MIT';license_files=@('LICENSE-MIT')},
  [ordered]@{name='winreg';version='0.55.0';vendor_path='vendor/winreg';license='MIT';license_files=@('LICENSE')}
)
$packages=@()
foreach($package in $packageMetadata){
  $licenseFiles=@()
  foreach($licenseFile in $package.license_files){
    $licenseRelative=($package.vendor_path+'/'+$licenseFile)
    $licenseFiles+=[ordered]@{path=$licenseRelative;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot $licenseRelative)).Hash}
  }
  $packages+=[ordered]@{
    name=$package.name
    version=$package.version
    vendor_path=$package.vendor_path
    cargo_toml_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot ($package.vendor_path+'/Cargo.toml'))).Hash
    cargo_checksum_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot ($package.vendor_path+'/.cargo-checksum.json'))).Hash
    license=$package.license
    license_files=$licenseFiles
  }
}
$provenance=[ordered]@{
  schema_version='1.0.0'
  contract='B-W2-1.2-007-current-substrate-v2-vendor-license-provenance'
  cargo_lock_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'Cargo.lock')).Hash
  packages=$packages
}
$provenance | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $provenanceOut -Encoding utf8
Write-Output "Current substrate manifest written: $out"
Write-Output "Current substrate vendor/license provenance written: $provenanceOut"
