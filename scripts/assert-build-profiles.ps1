[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
$manifestPath = Join-Path $WorkspaceRoot 'Cargo.toml'
$manifest = Get-Content -Raw -Encoding UTF8 $manifestPath

foreach ($profile in @('dev', 'release')) {
    $pattern = '(?ms)^\[profile\.' + $profile + '\]\s*.*?^panic\s*=\s*"unwind"\s*$'
    if ($manifest -notmatch $pattern) {
        throw ('profile.' + $profile + ' must explicitly set panic = "unwind".')
    }
}

if ($manifest -match '(?ms)^\[profile\.test\].*?^panic\s*=') {
    throw 'profile.test panic override is invalid: Cargo ignores it; test must retain Cargo default unwind behavior.'
}

if ($manifest -match '(?m)^panic\s*=\s*"abort"\s*$') {
    throw 'panic = "abort" is forbidden for all workspace profiles.'
}

Write-Output 'Build profile assertion passed: dev/release explicitly unwind; test retains Cargo default unwind behavior without ignored override.'
