param(
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
Import-Module PKI -ErrorAction Stop
$output = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $output -Force | Out-Null
$certificate = Get-ChildItem Cert:\CurrentUser\My |
    Where-Object { $_.Subject -eq 'CN=SuperDesktop' -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date).AddDays(30) } |
    Sort-Object NotAfter -Descending |
    Select-Object -First 1
if (-not $certificate) {
    $certificate = New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=SuperDesktop' `
        -FriendlyName 'SuperDesktop Windows identity package' -CertStoreLocation Cert:\CurrentUser\My `
        -NotAfter (Get-Date).AddYears(5)
}
$passwordText = [Guid]::NewGuid().ToString('N')
$password = ConvertTo-SecureString $passwordText -AsPlainText -Force
$pfx = Join-Path $output 'SuperDesktop.WindowsShell.build.pfx'
$msix = Join-Path $output 'SuperDesktop.WindowsShell.msix'
$cer = Join-Path $output 'SuperDesktop.WindowsShell.cer'
try {
    Export-PfxCertificate -Cert $certificate -FilePath $pfx -Password $password -Force | Out-Null
    Export-Certificate -Cert $certificate -FilePath $cer -Force | Out-Null
    & (Join-Path $PSScriptRoot 'build-windows-identity-package.ps1') `
        -PfxPath $pfx -PfxPassword $password -OutputPath $msix | Out-Null
}
finally {
    Remove-Item -LiteralPath $pfx -Force -ErrorAction SilentlyContinue
    $passwordText = $null
}
if (-not (Test-Path -LiteralPath $msix -PathType Leaf) -or -not (Test-Path -LiteralPath $cer -PathType Leaf)) {
    throw 'Windows identity package preparation did not produce both artifacts.'
}
[pscustomobject]@{ package = $msix; certificate = $cer; thumbprint = $certificate.Thumbprint }
