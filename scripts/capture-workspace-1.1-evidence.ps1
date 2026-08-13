[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
$changeRoot = Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace'
$artifactRoot = Join-Path $changeRoot 'evidence/artifacts/1.1'
New-Item -ItemType Directory -Force -Path $artifactRoot | Out-Null

function Invoke-And-Capture {
    param([string]$Name, [scriptblock]$Action, [int[]]$ExpectedExitCodes = @(0))

    $outputPath = Join-Path $artifactRoot $Name
    $transcript = [System.Collections.Generic.List[string]]::new()
    $transcript.Add("command: $($Action.ToString().Trim())")
    try {
        # cargo emits normal progress on stderr; do not turn that successful
        # native-command output into a terminating PowerShell exception.
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $output = & $Action 2>&1
        $exitCode = $LASTEXITCODE
        $ErrorActionPreference = $previousErrorActionPreference
    } catch {
        $ErrorActionPreference = $previousErrorActionPreference
        $output = $_ | Out-String
        $exitCode = 1
    }
    if ($null -eq $output) {
        $output = @()
    }
    $transcript.Add("exit_status: $exitCode")
    $transcript.Add('output:')
    $transcript.AddRange([string[]]$output)
    Set-Content -Encoding UTF8 -NoNewline -Path $outputPath -Value ($transcript -join [Environment]::NewLine)
    if ($exitCode -notin $ExpectedExitCodes) {
        throw "$Name returned $exitCode; expected $($ExpectedExitCodes -join ', ')."
    }
}

Push-Location $WorkspaceRoot
try {
    Invoke-And-Capture 'cargo-metadata.json' { cargo metadata --format-version 1 --no-deps } @(0)
    Invoke-And-Capture 'architecture-check.txt' { & "$PSScriptRoot/check-dependency-architecture.ps1" -WorkspaceRoot $WorkspaceRoot } @(0)
    Invoke-And-Capture 'core-depends-on-gpui-negative.txt' {
        & "$PSScriptRoot/check-dependency-architecture.ps1" -WorkspaceRoot $WorkspaceRoot -Fixture 'fixtures/dependency-architecture/core-depends-on-gpui'
    } @(1)
    Invoke-And-Capture 'ui-public-hwnd-negative.txt' {
        & "$PSScriptRoot/check-dependency-architecture.ps1" -WorkspaceRoot $WorkspaceRoot -Fixture 'fixtures/dependency-architecture/ui-public-hwnd'
    } @(1)
    Invoke-And-Capture 'cargo-check-windows.txt' { cargo check --workspace } @(0)
    Invoke-And-Capture 'cargo-test-windows.txt' { cargo test --workspace } @(0)
    Invoke-And-Capture 'format-check.txt' { cargo fmt --all -- --check } @(0)
    Invoke-And-Capture 'strict-openspec-validation.txt' { openspec validate bootstrap-superdesktop-workspace --strict } @(0)
    Invoke-And-Capture 'diff-check.txt' { git diff --check } @(0)
    Invoke-And-Capture 'nonwindows-target-availability.txt' { rustup target list --installed } @(0)
} finally {
    Pop-Location
}
