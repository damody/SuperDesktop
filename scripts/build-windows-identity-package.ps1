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
$certificate = (Get-PfxData -FilePath $pfx -Password $PfxPassword).EndEntityCertificates | Select-Object -First 1
if ($certificate.Subject -ne 'CN=SuperDesktop') {
    throw 'The signing certificate subject must be CN=SuperDesktop.'
}
function Find-WindowsSdkTool([string]$Name) {
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $candidate = Get-ChildItem -LiteralPath $kits -Recurse -Filter $Name -ErrorAction SilentlyContinue |
        Where-Object FullName -Match '\\x64\\' |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $candidate) { throw "Windows SDK tool is unavailable: $Name" }
    return $candidate.FullName
}
$makeAppx = Find-WindowsSdkTool 'makeappx.exe'
$signTool = Find-WindowsSdkTool 'signtool.exe'
$parent = Split-Path -Parent $output
if (-not (Test-Path -LiteralPath $parent)) {
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
}
$staging = Join-Path $parent 'windows-identity-staging'
if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $staging 'Assets') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $manifestDirectory 'AppxManifest.xml') -Destination $staging
$pixel = [Convert]::FromBase64String('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScL9WQAAAABJRU5ErkJggg==')
foreach ($name in 'StoreLogo.png','Square150x150Logo.png','Square44x44Logo.png') {
    [IO.File]::WriteAllBytes((Join-Path $staging "Assets\$name"), $pixel)
}
if (Test-Path -LiteralPath $output) { Remove-Item -LiteralPath $output -Force }
& $makeAppx pack /o /d $staging /nv /p $output
if ($LASTEXITCODE -ne 0) { throw 'MakeAppx failed.' }
$credential = [pscredential]::new('pfx', $PfxPassword)
$plainPassword = $credential.GetNetworkCredential().Password
try {
    & $signTool sign /fd SHA256 /f $pfx /p $plainPassword $output
    if ($LASTEXITCODE -ne 0) { throw 'SignTool failed.' }
}
finally {
    $plainPassword = $null
    Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Output $output
