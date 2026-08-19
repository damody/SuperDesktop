param(
    [Parameter(Mandatory = $true)]
    [string]$PackagePath,
    [Parameter(Mandatory = $true)]
    [string]$InstallDirectory
)

$ErrorActionPreference = 'Stop'
$package = (Resolve-Path -LiteralPath $PackagePath).Path
$install = (Resolve-Path -LiteralPath $InstallDirectory).Path
if (-not (Test-Path -LiteralPath (Join-Path $install 'notification-area-host.exe') -PathType Leaf)) {
    throw 'InstallDirectory must contain notification-area-host.exe.'
}
$signature = Get-AuthenticodeSignature -LiteralPath $package
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -ne 'CN=SuperDesktop') {
    throw 'The identity package must have a valid CN=SuperDesktop signature.'
}
Add-AppxPackage -Path $package -ExternalLocation $install
