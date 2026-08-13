[CmdletBinding()]
param([string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
$changeRoot = Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace'
$artifactRoot = Join-Path $changeRoot 'evidence/artifacts/2.3'
New-Item -ItemType Directory -Force -Path $artifactRoot | Out-Null

function Capture([string]$Name, [scriptblock]$Action, [int]$ExpectedExit = 0, [string]$ExpectedText = '') {
    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = & $Action 2>&1
        $exit = $LASTEXITCODE; if ($null -eq $exit) { $exit = 0 }
    } catch {
        $output = $_ | Out-String
        $exit = 1
    } finally { $ErrorActionPreference = $previous }
    $text = @("command: $($Action.ToString().Trim())", "exit_status: $exit", 'output:') + @($output | ForEach-Object { [string]$_ })
    Set-Content -Encoding UTF8 -Path (Join-Path $artifactRoot $Name) -Value $text
    if ($exit -ne $ExpectedExit) { throw "$Name exit $exit, expected $ExpectedExit" }
    if ($ExpectedText -and -not (($text -join "`n").Contains($ExpectedText))) { throw "$Name did not contain $ExpectedText" }
}

Push-Location $WorkspaceRoot
try {
    Capture 'validator-positive.txt' { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/validate-evidence.ps1" }
    $matrix = Get-Content -Raw -Encoding UTF8 (Join-Path $WorkspaceRoot 'fixtures/evidence-validator/fixtures.json') | ConvertFrom-Json
    foreach ($case in $matrix.cases) {
        Capture ("fixture-$($case.id).txt") { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/validate-evidence.ps1" -Fixture $case.fixture } $case.expected_exit $case.expected_code
    }
    Capture 'strict-openspec-validation.txt' { openspec validate bootstrap-superdesktop-workspace --strict }
    Capture 'diff-check.txt' { git diff --check }
} finally { Pop-Location }
