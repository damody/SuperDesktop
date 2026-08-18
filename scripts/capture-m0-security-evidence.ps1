[CmdletBinding()]
param(
    [string]$Workspace,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $Workspace 'openspec/changes/verify-superdesktop-m0/evidence/artifacts/5.2/security-procedures.json'
}
$Workspace = (Resolve-Path -LiteralPath $Workspace).Path
$logRoot = Join-Path (Split-Path -Parent $OutputPath) 'security-logs'
New-Item -ItemType Directory -Force -Path $logRoot | Out-Null
$utf8 = [Text.UTF8Encoding]::new($false)
$results = @()

function Invoke-PassingGate([string]$Name, [scriptblock]$Command) {
    $prior = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = (& $Command 2>&1 | Out-String)
        $exit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prior
    }
    if ($exit -ne 0) { throw "$Name failed with exit $exit`n$output" }
    $path = Join-Path $logRoot "$Name.log"
    [IO.File]::WriteAllText($path, ($output.TrimEnd() + [Environment]::NewLine), $utf8)
    $script:results += [ordered]@{
        name = $Name
        result = 'passed'
        exit_status = $exit
        artifact = $path.Substring($Workspace.Length + 1).Replace('\', '/')
        sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Invoke-RejectedFixture([string]$Name, [scriptblock]$Command, [string]$Expected) {
    $prior = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = (& $Command 2>&1 | Out-String)
        $exit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prior
    }
    if ($exit -ne 1 -or $output -notmatch [regex]::Escape($Expected)) {
        throw "$Name did not fail closed with $Expected (exit=$exit)`n$output"
    }
    $path = Join-Path $logRoot "$Name.log"
    [IO.File]::WriteAllText($path, ($output.TrimEnd() + [Environment]::NewLine), $utf8)
    $script:results += [ordered]@{
        name = $Name
        result = 'passed-rejected'
        exit_status = $exit
        expected_diagnostic = $Expected
        artifact = $path.Substring($Workspace.Length + 1).Replace('\', '/')
        sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

$parent = Split-Path -Parent $Workspace
$parentStatusBefore = @(& git -C $parent status --porcelain=v1 --untracked-files=no | Where-Object { $_ -notmatch 'SuperDesktop$' })
if ($LASTEXITCODE -ne 0 -or $parentStatusBefore.Count -ne 0) {
    throw 'The external SuperExplorer tracked worktree is not clean before the security procedures.'
}

Push-Location $Workspace
try {
    Invoke-PassingGate 'canonical-reparse' {
        & cargo test -p settings-store -p platform-win --locked --offline reparse -- --nocapture
    }
    Invoke-PassingGate 'bridge-injection-redaction' {
        & cargo test -p explorer-bridge --locked --offline -- --nocapture
    }
    Invoke-PassingGate 'installer-drift-rollback' {
        & cargo test -p shell-installer --locked --offline -- --nocapture
    }
    Invoke-PassingGate 'guardian-anti-spoof' {
        & cargo test -p superdesktop-guardian --locked --offline recovery::tests::forged_wrong_session_token_and_ambiguous_timing_are_zero_effect -- --exact --nocapture
    }
    Invoke-PassingGate 'architecture-positive' {
        & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-dependency-architecture.ps1
    }
    Invoke-PassingGate 'source-boundary-positive' {
        & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/audit-source-boundary.ps1
    }
    Invoke-RejectedFixture 'source-boundary-superexplorer-negative' {
        & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/audit-source-boundary.ps1 -Fixture fixtures/source-boundary/superexplorer-path-dependency
    } 'SUPEREXPLORER_PATH_DEPENDENCY'
    Invoke-RejectedFixture 'source-boundary-derived-negative' {
        & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/audit-source-boundary.ps1 -Fixture fixtures/source-boundary/pexplorer-derived-source
    } 'PEXPLORER_DERIVED_SOURCE'

    $sensitivePattern = '(?i)\b(?:clipboard|password|CredRead|CredWrite|Credential Manager)\b'
    $sensitiveHits = @(& rg -n $sensitivePattern crates -g 'src/*.rs' -g 'src/**/*.rs')
    if ($LASTEXITCODE -notin @(0, 1)) { throw 'Sensitive-source scan failed.' }
    if ($sensitiveHits.Count -ne 0) {
        throw "Production shell sources unexpectedly access sensitive clipboard/credential material: $($sensitiveHits -join '; ')"
    }
    $sensitivePath = Join-Path $logRoot 'sensitive-source-absence.log'
    [IO.File]::WriteAllText(
        $sensitivePath,
        "passed: no production clipboard, password, CredRead, CredWrite, or Credential Manager access`n",
        $utf8
    )
    $results += [ordered]@{
        name = 'sensitive-source-absence'
        result = 'passed'
        exit_status = 0
        artifact = $sensitivePath.Substring($Workspace.Length + 1).Replace('\', '/')
        sha256 = (Get-FileHash -LiteralPath $sensitivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
} finally {
    Pop-Location
}

$parentStatusAfter = @(& git -C $parent status --porcelain=v1 --untracked-files=no | Where-Object { $_ -notmatch 'SuperDesktop$' })
if ($LASTEXITCODE -ne 0 -or (Compare-Object $parentStatusBefore $parentStatusAfter)) {
    throw 'Security procedures changed tracked SuperExplorer state outside the SuperDesktop gitlink.'
}

$report = [ordered]@{
    schema = 'm0-security-procedures/v1'
    result = 'passed'
    recorded_at = [DateTimeOffset]::Now.ToString('o')
    revision = (& git -C $Workspace rev-parse HEAD).Trim()
    procedures = $results
    assertions = [ordered]@{
        fixture_root_canonical_reparse = 'passed-by-tests'
        path_argument_environment_executable_substitution = 'passed-by-bridge-and-installer-tests'
        credential_clipboard_environment_log_redaction = 'passed-by-redaction-tests-and-sensitive-source-absence'
        dependency_license_inventory = 'passed-by-source-boundary-audit'
        pexplorer_and_superexplorer_source_boundary = 'passed-by-positive-and-negative-fixtures'
        superexplorer_repository_unchanged = 'passed-by-before-after-tracked-status'
    }
}
[IO.File]::WriteAllText(
    $OutputPath,
    (($report | ConvertTo-Json -Depth 12) + [Environment]::NewLine),
    $utf8
)
$report | ConvertTo-Json -Depth 12
