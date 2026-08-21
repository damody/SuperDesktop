param(
    [Parameter(Mandatory = $true)]
    [string]$PackagePath,
    [Parameter(Mandatory = $true)]
    [string]$InstallDirectory,
    [Parameter(Mandatory = $true)]
    [string]$CertificatePath
)

$ErrorActionPreference = 'Stop'
Import-Module PKI -ErrorAction Stop
if (-not (Get-PSDrive -Name Cert -ErrorAction SilentlyContinue)) {
    New-PSDrive -Name Cert -PSProvider Microsoft.PowerShell.Security\Certificate -Root '\' | Out-Null
}
$package = (Resolve-Path -LiteralPath $PackagePath).Path
$install = (Resolve-Path -LiteralPath $InstallDirectory).Path
$certificatePath = (Resolve-Path -LiteralPath $CertificatePath).Path
if (-not (Test-Path -LiteralPath (Join-Path $install 'notification-area-host.exe') -PathType Leaf)) {
    throw 'InstallDirectory must contain notification-area-host.exe.'
}
$certificate = Import-Certificate -FilePath $certificatePath -CertStoreLocation Cert:\LocalMachine\TrustedPeople
if (-not $certificate -or $certificate.Subject -ne 'CN=SuperDesktop') {
    throw 'The identity certificate subject must be CN=SuperDesktop.'
}
$signature = Get-AuthenticodeSignature -LiteralPath $package
if ($null -eq $signature.SignerCertificate -or
    $signature.SignerCertificate.Subject -ne 'CN=SuperDesktop' -or
    $signature.SignerCertificate.Thumbprint -ne $certificate.Thumbprint -or
    $signature.Status -notin 'Valid','UnknownError') {
    throw 'The identity package must have a valid CN=SuperDesktop signature.'
}
Get-AppxPackage -Name SuperDesktop.WindowsShell -ErrorAction SilentlyContinue |
    Remove-AppxPackage -ErrorAction Stop
Add-AppxPackage -Path $package -ExternalLocation $install -ForceApplicationShutdown -ForceUpdateFromAnyVersion
$registered = Get-AppxPackage -Name SuperDesktop.WindowsShell -ErrorAction Stop
if ($registered.Status -ne 'Ok') { throw 'SuperDesktop Windows identity package registration is unhealthy.' }
