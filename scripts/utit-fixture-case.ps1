param(
    [ValidateSet('pass','fail','timeout','malformed')][string]$Mode = 'pass',
    [Parameter(Mandatory = $true)][string]$Artifact
)
$ErrorActionPreference = 'Stop'
if ($Mode -eq 'timeout') { Start-Sleep -Seconds 5; exit 0 }
if ($Mode -eq 'fail') { [Console]::Error.WriteLine('controlled failure'); exit 7 }
if ($Mode -eq 'pass') {
    New-Item -ItemType Directory -Force (Split-Path -Parent $Artifact) | Out-Null
    [IO.File]::WriteAllText($Artifact, '{"result":"passed"}', [Text.UTF8Encoding]::new($false))
}
Write-Output "fixture:$Mode"
