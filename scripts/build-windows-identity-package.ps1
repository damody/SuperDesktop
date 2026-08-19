param(
    [Parameter(Mandatory = $true)]
    [string]$PfxPath,
    [Parameter(Mandatory = $true)]
    [securestring]$PfxPassword,
    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$manifestDirectory = Join-Path $workspace 'packaging/windows-identity'
$pfx = (Resolve-Path -LiteralPath $PfxPath).Path
$output = [IO.Path]::GetFullPath($OutputPath)
$certificate = Get-PfxCertificate -FilePath $pfx
if ($certificate.Subject -ne 'CN=SuperDesktop') {
    throw 'The signing certificate subject must be CN=SuperDesktop.'
}
$makeAppx = (Get-Command makeappx.exe -ErrorAction Stop).Source
$signTool = (Get-Command signtool.exe -ErrorAction Stop).Source
$parent = Split-Path -Parent $output
if (-not (Test-Path -LiteralPath $parent)) {
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
}
if (Test-Path -LiteralPath $output) {
    throw "Identity package output already exists: $output"
}
& $makeAppx pack /o /d $manifestDirectory /nv /p $output
if ($LASTEXITCODE -ne 0) { throw 'MakeAppx failed.' }
$credential = [pscredential]::new('pfx', $PfxPassword)
$plainPassword = $credential.GetNetworkCredential().Password
try {
    & $signTool sign /fd SHA256 /f $pfx /p $plainPassword $output
    if ($LASTEXITCODE -ne 0) { throw 'SignTool failed.' }
}
finally {
    $plainPassword = $null
}
Write-Output $output
