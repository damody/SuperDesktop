[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
$changeRoot = Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace'
$artifactRoot = Join-Path $changeRoot 'evidence/artifacts/1.2'
New-Item -ItemType Directory -Force -Path $artifactRoot | Out-Null

function Invoke-And-Capture {
    param([string]$Name, [scriptblock]$Action)

    $outputPath = Join-Path $artifactRoot $Name
    $transcript = [System.Collections.Generic.List[string]]::new()
    $transcript.Add("command: $($Action.ToString().Trim())")
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = & $Action 2>&1
        $exitCode = $LASTEXITCODE
    } catch {
        $output = $_ | Out-String
        $exitCode = 1
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($null -eq $output) {
        $output = @()
    }
    $transcript.Add("exit_status: $exitCode")
    $transcript.Add('output:')
    $transcript.AddRange([string[]]$output)
    Set-Content -Encoding UTF8 -NoNewline -Path $outputPath -Value ($transcript -join [Environment]::NewLine)
    if ($exitCode -ne 0) {
        throw "$Name returned $exitCode."
    }
}

Push-Location $WorkspaceRoot
try {
    Invoke-And-Capture 'toolchain.txt' { rustup run 1.97.1 cargo --version; rustup run 1.97.1 rustc --version; rustup target list --installed }
    Invoke-And-Capture 'profile-assertion.txt' { & "$PSScriptRoot/assert-build-profiles.ps1" -WorkspaceRoot $WorkspaceRoot }
    Invoke-And-Capture 'dependency-provenance-assertion.txt' { & "$PSScriptRoot/verify-dependency-provenance.ps1" -WorkspaceRoot $WorkspaceRoot }
    Invoke-And-Capture 'online-locked-check.txt' { cargo check --workspace --locked }
    Invoke-And-Capture 'offline-isolated-locked-check.txt' {
        $env:CARGO_HOME = (Join-Path $WorkspaceRoot 'build/isolated-cargo-home')
        $env:CARGO_TARGET_DIR = (Join-Path $WorkspaceRoot 'build/isolated-target')
        $env:CARGO_NET_OFFLINE = 'true'
        New-Item -ItemType Directory -Force -Path $env:CARGO_HOME, $env:CARGO_TARGET_DIR | Out-Null
        cargo check --workspace --locked --offline
    }
} finally {
    Pop-Location
}
