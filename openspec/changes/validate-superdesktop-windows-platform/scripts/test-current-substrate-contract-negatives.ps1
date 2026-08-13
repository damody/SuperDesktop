[CmdletBinding()]
param([string]$WorkspaceRoot,[string]$ManifestPath,[string]$OutputPath)
$ErrorActionPreference='Stop'
if(-not $WorkspaceRoot){$WorkspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path}
$manifest=if($ManifestPath){$ManifestPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-inputs-successor-1.2-manifest-v3.sha256'}
$out=if($OutputPath){$OutputPath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/current-substrate-v3-negative-tests.txt'}
$verifier=Join-Path $PSScriptRoot 'verify-current-substrate-contract.ps1'
if(-not(Test-Path -LiteralPath $manifest -PathType Leaf)){throw 'NEGATIVE_MANIFEST_MISSING'}
$scratch=Join-Path $WorkspaceRoot ('build/current-substrate-negative-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force $scratch|Out-Null
function Invoke-Negative([string]$Name,[string[]]$Lines,[string]$Expected){
  $path=Join-Path $scratch ($Name+'.sha256')
  [IO.File]::WriteAllLines($path,$Lines,[Text.UTF8Encoding]::new($false))
  $old=$ErrorActionPreference;$ErrorActionPreference='Continue'
  try{$output=& powershell -NoProfile -ExecutionPolicy Bypass -File $verifier -WorkspaceRoot $WorkspaceRoot -ManifestPath $path 2>&1;$exit=$LASTEXITCODE}finally{$ErrorActionPreference=$old}
  $text=($output|Out-String)
  if($exit -eq 0){throw "NEGATIVE_UNEXPECTED_PASS: $Name"}
  if($text -notmatch [regex]::Escape($Expected)){throw "NEGATIVE_DIAGNOSTIC_MISMATCH: $Name expected $Expected got $text"}
  return "$Name exit=$exit diagnostic=$Expected"
}
try{
  $base=@(Get-Content -LiteralPath $manifest)
  $cargoLine=@($base|Where-Object{$_ -match '  Cargo\.toml$'})[0]
  $gpuiBuildLine=@($base|Where-Object{$_ -match '  vendor/gpui/build\.rs$'})[0]
  if(-not $cargoLine -or -not $gpuiBuildLine){throw 'NEGATIVE_FIXTURE_SOURCE_INCOMPLETE'}
  $results=@()
  $results+=Invoke-Negative 'missing' @($base|Where-Object{$_ -ne $cargoLine}) 'CURRENT_SUBSTRATE_MANIFEST_MISSING_EXPECTED'
  $extraHash=(Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $WorkspaceRoot 'rust-toolchain.toml')).Hash
  $results+=Invoke-Negative 'extra' @($base+($extraHash+'  rust-toolchain.toml')) 'CURRENT_SUBSTRATE_MANIFEST_UNEXPECTED_PATH'
  $results+=Invoke-Negative 'duplicate' @($base+$cargoLine) 'CURRENT_SUBSTRATE_MANIFEST_DUPLICATE_PATH'
  $substitution=@($base|ForEach-Object{if($_ -eq $cargoLine){$gpuiBuildLine}else{$_}})
  $results+=Invoke-Negative 'path-substitution' $substitution 'CURRENT_SUBSTRATE_MANIFEST_PATH_SUBSTITUTION'
  New-Item -ItemType Directory -Force (Split-Path -Parent $out)|Out-Null
  [IO.File]::WriteAllLines($out,$results,[Text.UTF8Encoding]::new($false))
  $results|ForEach-Object{Write-Output $_}
}finally{
  if(Test-Path -LiteralPath $scratch){Remove-Item -LiteralPath $scratch -Recurse -Force}
}
