[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
$changeRoot = Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace'
$artifactRoot = Join-Path $changeRoot 'evidence/artifacts/2.2'
New-Item -ItemType Directory -Force -Path $artifactRoot | Out-Null

function Invoke-And-Capture {
    param([string]$Name, [scriptblock]$Action, [int]$ExpectedExitCode = 0)

    $outputPath = Join-Path $artifactRoot $Name
    $transcript = [System.Collections.Generic.List[string]]::new()
    $transcript.Add("command: $($Action.ToString().Trim())")
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = & $Action 2>&1
        $exitCode = $LASTEXITCODE
        if ($null -eq $exitCode) { $exitCode = 0 }
    } catch {
        $output = $_ | Out-String
        $exitCode = 1
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($null -eq $output) { $output = @() }
    $transcript.Add("exit_status: $exitCode")
    $transcript.Add('output:')
    $transcript.AddRange([string[]]$output)
    Set-Content -Encoding UTF8 -NoNewline -Path $outputPath -Value ($transcript -join [Environment]::NewLine)
    if ($exitCode -ne $ExpectedExitCode) {
        throw "$Name returned $exitCode; expected $ExpectedExitCode."
    }
}

Push-Location $WorkspaceRoot
try {
    Invoke-And-Capture 'license-inventory-generation.txt' { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/generate-license-inventory.ps1" -WorkspaceRoot $WorkspaceRoot }
    Invoke-And-Capture 'source-boundary-audit.txt' { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/audit-source-boundary.ps1" -WorkspaceRoot $WorkspaceRoot }
    Invoke-And-Capture 'superexplorer-path-negative.txt' { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/audit-source-boundary.ps1" -WorkspaceRoot $WorkspaceRoot -Fixture 'fixtures/source-boundary/superexplorer-path-dependency' } 1
    Invoke-And-Capture 'pexplorer-derived-source-negative.txt' { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/audit-source-boundary.ps1" -WorkspaceRoot $WorkspaceRoot -Fixture 'fixtures/source-boundary/pexplorer-derived-source' } 1
    Invoke-And-Capture 'offline-metadata.txt' { cargo metadata --locked --offline --format-version 1 }
    Invoke-And-Capture 'format-check.txt' { cargo fmt --all -- --check }
    Invoke-And-Capture 'strict-openspec-validation.txt' { openspec validate bootstrap-superdesktop-workspace --strict }
    Invoke-And-Capture 'diff-check.txt' { git diff --check }
} finally {
    Pop-Location
}
