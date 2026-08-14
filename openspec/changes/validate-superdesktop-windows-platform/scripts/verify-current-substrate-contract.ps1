[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$ManifestPath)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
$manifest=if($ManifestPath){$ManifestPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-inputs-successor-3.5-manifest-v4.sha256'}
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
if(-not(Test-Path -LiteralPath $manifest -PathType Leaf)){throw 'CURRENT_SUBSTRATE_MANIFEST_MISSING'}
$expected=Get-ExpectedPaths
if(($expected|Select-Object -Unique).Count -ne $expected.Count){throw 'CURRENT_SUBSTRATE_VERIFIER_EXPECTED_PATH_DUPLICATE'}
$entries=@();$seen=@{}
foreach($line in Get-Content -LiteralPath $manifest){
  if($line -notmatch '^([A-F0-9]{64})  ([a-zA-Z0-9._/-]+)$'){throw "CURRENT_SUBSTRATE_MANIFEST_MALFORMED: $line"}
  $path=$matches[2]
  if($path.Contains('..') -or $path.StartsWith('/')){throw "CURRENT_SUBSTRATE_PATH_REJECTED: $path"}
  if($seen.ContainsKey($path)){$seen[$path]++;continue};$seen[$path]=1
  $entries+=[ordered]@{hash=$matches[1];path=$path}
}
$duplicate=@($seen.Keys|Where-Object{$seen[$_] -gt 1})
$observed=@($seen.Keys)
$unexpected=@($observed|Where-Object{$_ -notin $expected})
$missing=@($expected|Where-Object{$_ -notin $observed})
if($unexpected.Count -gt 0){throw "CURRENT_SUBSTRATE_MANIFEST_UNEXPECTED_PATH: $($unexpected[0])"}
if($duplicate.Count -gt 0 -and $missing.Count -gt 0){throw "CURRENT_SUBSTRATE_MANIFEST_PATH_SUBSTITUTION: $($duplicate[0])"}
if($duplicate.Count -gt 0){throw "CURRENT_SUBSTRATE_MANIFEST_DUPLICATE_PATH: $($duplicate[0])"}
if($missing.Count -gt 0){throw "CURRENT_SUBSTRATE_MANIFEST_MISSING_EXPECTED: $($missing[0])"}
foreach($entry in $entries){
  $full=Join-Path $WorkspaceRoot $entry.path
  if(-not(Test-Path -LiteralPath $full -PathType Leaf)){throw "CURRENT_SUBSTRATE_INPUT_MISSING: $($entry.path)"}
  if((Get-FileHash -Algorithm SHA256 -LiteralPath $full).Hash -ne $entry.hash){throw "CURRENT_SUBSTRATE_HASH_DRIFT: $($entry.path)"}
}
$gpuiChecksum=Get-Content -Raw -Encoding utf8 (Join-Path $WorkspaceRoot 'vendor/gpui/.cargo-checksum.json')|ConvertFrom-Json
foreach($path in @('build.rs','resources/windows/gpui.manifest.xml','resources/windows/gpui.rc')){if($null -eq $gpuiChecksum.files.$path){throw "CURRENT_SUBSTRATE_GPUI_CHECKSUM_COVERAGE_MISSING: $path"}}
$provenance=Get-Content -Raw -Encoding utf8 (Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-v4-vendor-provenance.json')|ConvertFrom-Json
if($provenance.contract -ne 'B-W2-3.5-001-current-substrate-v4-audited-local-patch' -or $provenance.gpui_windows_patch.upstream_revision -ne '8945e2981b9fd00ca887e042d8adb9acc241b168' -or $provenance.gpui_windows_patch.license -ne 'Apache-2.0'){throw 'CURRENT_SUBSTRATE_GPUI_WINDOWS_PATCH_PROVENANCE_INVALID'}
if($provenance.gpui.callback_patch.upstream_sha256 -ne '3CAA3671D446D384E593283E595C008F03921450319C6A91B9664013744105D8' -or (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot $provenance.gpui.callback_patch.path)).Hash -ne $provenance.gpui.callback_patch.patched_sha256){throw 'CURRENT_SUBSTRATE_GPUI_CALLBACK_PATCH_PROVENANCE_INVALID'}
foreach($file in $provenance.gpui_windows_patch.files){if((Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot $file.path)).Hash -ne $file.patched_sha256){throw "CURRENT_SUBSTRATE_GPUI_WINDOWS_PATCH_HASH_DRIFT: $($file.path)"}}
$lock=Get-Content -Raw -Encoding utf8 (Join-Path $WorkspaceRoot 'Cargo.lock')
if($lock -notmatch 'name = "gpui_windows"' -or $lock -notmatch 'name = "raw-window-handle"\r?\nversion = "0\.6\.2"'){throw 'CURRENT_SUBSTRATE_PROVENANCE_MISSING'}
$requiredPackages=@(@('embed-resource','3.0.11'),@('serde_spanned','1.1.1'),@('toml','1.1.4\+spec-1.1.0'),@('toml_datetime','1.1.1\+spec-1.1.0'),@('toml_parser','1.1.3\+spec-1.1.0'),@('toml_writer','1.1.2\+spec-1.1.0'),@('vswhom','0.1.0'),@('vswhom-sys','0.1.3'),@('winnow','1.0.4'),@('winreg','0.55.0'))
foreach($package in $requiredPackages){if($lock -notmatch ('name = "'+$package[0]+'"\r?\nversion = "'+$package[1]+'"')){throw "CURRENT_SUBSTRATE_PACKAGE_PROVENANCE_MISSING: $($package[0])"}}
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $WorkspaceRoot 'scripts/check-dependency-architecture.ps1') -WorkspaceRoot $WorkspaceRoot | Out-Null
if($LASTEXITCODE -ne 0){throw 'CURRENT_SUBSTRATE_ARCHITECTURE_FAILED'}
Write-Output 'Current substrate v4 exact-set contract passed.'
