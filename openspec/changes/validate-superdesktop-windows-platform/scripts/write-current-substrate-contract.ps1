[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$OutputPath,[string]$ProvenanceOutputPath)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
$out=if($OutputPath){$OutputPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-inputs-successor-3.5-manifest-v4.sha256'}
$provenanceOut=if($ProvenanceOutputPath){$ProvenanceOutputPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-v4-vendor-provenance.json'}

function Get-ExpectedPaths {
  @(
    'Cargo.toml','Cargo.lock','.cargo/config.toml','crates/desktop-ui/Cargo.toml','scripts/architecture-allowlist.json',
    'vendor/gpui/.cargo-checksum.json','vendor/gpui/Cargo.toml','vendor/gpui/LICENSE-APACHE','vendor/gpui/build.rs','vendor/gpui/src/app/context.rs','vendor/gpui/resources/windows/gpui.manifest.xml','vendor/gpui/resources/windows/gpui.rc',
    'vendor/gpui_windows/.cargo-checksum.json','vendor/gpui_windows/src/window.rs','vendor/gpui_windows/src/platform.rs','vendor/gpui_windows/src/events.rs','vendor/gpui_windows/src/gpui_windows.rs','vendor/raw-window-handle/.cargo-checksum.json','vendor/windows/.cargo-checksum.json',
    'vendor/embed-resource/.cargo-checksum.json','vendor/embed-resource/Cargo.toml','vendor/embed-resource/LICENSE',
    'vendor/serde_spanned-1.1.1/.cargo-checksum.json','vendor/serde_spanned-1.1.1/Cargo.toml','vendor/serde_spanned-1.1.1/LICENSE-APACHE','vendor/serde_spanned-1.1.1/LICENSE-MIT',
    'vendor/toml-1.1.4/.cargo-checksum.json','vendor/toml-1.1.4/Cargo.toml','vendor/toml-1.1.4/LICENSE-APACHE','vendor/toml-1.1.4/LICENSE-MIT',
    'vendor/toml_datetime-1.1.1/.cargo-checksum.json','vendor/toml_datetime-1.1.1/Cargo.toml','vendor/toml_datetime-1.1.1/LICENSE-APACHE','vendor/toml_datetime-1.1.1/LICENSE-MIT',
    'vendor/toml_parser/.cargo-checksum.json','vendor/toml_parser/Cargo.toml','vendor/toml_parser/LICENSE-APACHE','vendor/toml_parser/LICENSE-MIT',
    'vendor/toml_writer/.cargo-checksum.json','vendor/toml_writer/Cargo.toml','vendor/toml_writer/LICENSE-APACHE','vendor/toml_writer/LICENSE-MIT',
    'vendor/vswhom/.cargo-checksum.json','vendor/vswhom/Cargo.toml','vendor/vswhom/LICENSE',
    'vendor/vswhom-sys/.cargo-checksum.json','vendor/vswhom-sys/Cargo.toml','vendor/vswhom-sys/LICENSE',
    'vendor/winnow-1.0.4/.cargo-checksum.json','vendor/winnow-1.0.4/Cargo.toml','vendor/winnow-1.0.4/LICENSE-MIT',
    'vendor/winreg/.cargo-checksum.json','vendor/winreg/Cargo.toml','vendor/winreg/LICENSE',
    'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-v4-vendor-provenance.json'
  )
}

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
    $relative=($package.vendor_path+'/'+$licenseFile)
    $licenseFiles+=[ordered]@{path=$relative;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot $relative)).Hash}
  }
  $packages+=[ordered]@{name=$package.name;version=$package.version;vendor_path=$package.vendor_path;cargo_toml_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot ($package.vendor_path+'/Cargo.toml'))).Hash;cargo_checksum_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot ($package.vendor_path+'/.cargo-checksum.json'))).Hash;license=$package.license;license_files=$licenseFiles}
}
New-Item -ItemType Directory -Force (Split-Path -Parent $provenanceOut)|Out-Null
$gpuiWindowsPatch=[ordered]@{
  contract='B-W2-3.5-001-no-unwind-terminal'
  upstream_repository='https://github.com/damody/gpui-ce-explorer.git'
  upstream_revision='8945e2981b9fd00ca887e042d8adb9acc241b168'
  license='Apache-2.0'
  rationale='Catch Rust panics at both gpui_windows WndProc boundaries, emit a typed fatal event, and drive backend-owned HWNDs through WM_NCDESTROY and GPUI close terminals.'
  files=@(
    [ordered]@{path='vendor/gpui_windows/src/window.rs';upstream_sha256='4544FD38D971014701502233424713010C465F3071F3BE343591E92A59D537E9';patched_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'vendor/gpui_windows/src/window.rs')).Hash},
    [ordered]@{path='vendor/gpui_windows/src/platform.rs';upstream_sha256='DE4C0F460C637FA432A5DC4C0BDA48CD5EA6E6E46FA3F211724357A315DF3601';patched_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'vendor/gpui_windows/src/platform.rs')).Hash},
    [ordered]@{path='vendor/gpui_windows/src/events.rs';upstream_sha256='CFF9CB8272A6BFC8E4535D6F61919DB65572DF6CEDA774FC20E59D671104934B';patched_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'vendor/gpui_windows/src/events.rs')).Hash},
    [ordered]@{path='vendor/gpui_windows/src/gpui_windows.rs';upstream_sha256='EF1B86DE6B469DAD09F8CE94E5E574091D1A4CC91F01B0703CC72B32FBF000C9';patched_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'vendor/gpui_windows/src/gpui_windows.rs')).Hash}
  )
}
$gpuiCallbackPatch=[ordered]@{path='vendor/gpui/src/app/context.rs';upstream_sha256='3CAA3671D446D384E593283E595C008F03921450319C6A91B9664013744105D8';patched_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'vendor/gpui/src/app/context.rs')).Hash;rationale='Contain public GPUI bounds-observer panics inside the application update and notify the selected platform backend.'}
[ordered]@{schema_version='2.0.0';contract='B-W2-3.5-001-current-substrate-v4-audited-local-patch';predecessor='B-W2-1.2-007-current-substrate-v3-vendor-license-provenance';cargo_lock_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'Cargo.lock')).Hash;gpui=[ordered]@{vendor_path='vendor/gpui';license='Apache-2.0';checksum_path='vendor/gpui/.cargo-checksum.json';build_script_path='vendor/gpui/build.rs';resources=@('vendor/gpui/resources/windows/gpui.manifest.xml','vendor/gpui/resources/windows/gpui.rc');callback_patch=$gpuiCallbackPatch};gpui_windows_patch=$gpuiWindowsPatch;packages=$packages} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $provenanceOut -Encoding utf8

$expected=Get-ExpectedPaths
if(($expected | Select-Object -Unique).Count -ne $expected.Count){throw 'CURRENT_SUBSTRATE_WRITER_EXPECTED_PATH_DUPLICATE'}
$lines=@()
foreach($relative in $expected){
  $full=Join-Path $WorkspaceRoot $relative
  if(-not(Test-Path -LiteralPath $full -PathType Leaf)){throw "CURRENT_SUBSTRATE_INPUT_MISSING: $relative"}
  $lines+="{0}  {1}" -f (Get-FileHash -Algorithm SHA256 -LiteralPath $full).Hash,$relative.Replace('\','/')
}
New-Item -ItemType Directory -Force (Split-Path -Parent $out)|Out-Null
[IO.File]::WriteAllLines($out,$lines,[Text.UTF8Encoding]::new($false))
Write-Output "Current substrate v4 provenance written: $provenanceOut"
Write-Output "Current substrate v4 exact-set manifest written: $out"
