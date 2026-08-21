param(
    [string]$Workspace = '',
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Workspace)) {
    $Workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
$app = Join-Path $Workspace 'target/release/superdesktop-app.exe'
$guardian = Join-Path $Workspace 'target/release/superdesktop-guardian.exe'
foreach ($path in @($app, $guardian)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing release binary: $path" }
}
New-Item -ItemType Directory -Path $EvidenceDirectory -Force | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path
$registryPath = 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'
$shellBefore = (Get-ItemProperty -LiteralPath $registryPath -Name Shell -ErrorAction SilentlyContinue).Shell
$explorerBefore = @(Get-Process explorer -ErrorAction SilentlyContinue).Id
$forbidden = 'guardian rejected|guardian-lease-validation|child-acceptance-timeout|child-exited-before-acceptance|SuperDesktop warning \[taskbar:appbar\]|rollback record is unavailable|panicked|RefCell already borrowed'
$runs = @()
foreach ($run in 1..2) {
    $terminal = Join-Path $EvidenceDirectory "guardian-terminal-$run.json"
    $accepted = "$terminal.accepted"
    $stdout = Join-Path $EvidenceDirectory "guardian-parent-$run.stdout.log"
    $stderr = Join-Path $EvidenceDirectory "guardian-parent-$run.stderr.log"
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $parent = Start-Process -FilePath $app -ArgumentList '--guardian-parent-fixture', $terminal `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru -Wait
    if ($parent.ExitCode -ne 0) { throw "Guardian parent fixture $run exited $($parent.ExitCode)" }
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while (-not (Test-Path -LiteralPath $terminal -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 25
    }
    $timer.Stop()
    if (-not (Test-Path -LiteralPath $terminal -PathType Leaf)) { throw "Guardian terminal $run was not published" }
    if (-not (Test-Path -LiteralPath $accepted -PathType Leaf)) { throw "Guardian acceptance $run was not published" }
    $record = Get-Content -LiteralPath $terminal -Raw -Encoding UTF8 | ConvertFrom-Json
    $acceptance = Get-Content -LiteralPath $accepted -Raw -Encoding UTF8
    $stderrText = [string](Get-Content -LiteralPath $stderr -Raw -ErrorAction SilentlyContinue)
    $stdoutText = [string](Get-Content -LiteralPath $stdout -Raw -ErrorAction SilentlyContinue)
    if (-not $record.parent_terminal_observed -or -not $record.recovery_verified -or
        $record.unique_success_terminal_count -ne 1 -or $acceptance -notmatch '^guardian-lease-accepted:[0-9a-f]{32}$') {
        throw "Guardian lifecycle $run did not satisfy the exact terminal contract"
    }
    if ("$stderrText`n$stdoutText" -match $forbidden) { throw "Guardian lifecycle $run emitted a forbidden signature" }
    $runs += [ordered]@{
        run = $run
        elapsed_ms = $timer.ElapsedMilliseconds
        recovery_disposition = $record.recovery_disposition
        explorer_pid = $record.explorer_pid
        unique_success_terminal_count = $record.unique_success_terminal_count
        explicit_allowlist_exact = $record.explicit_allowlist_exact
        acceptance_nonce_bound = $true
        stdout_sha256 = (Get-FileHash -LiteralPath $stdout -Algorithm SHA256).Hash
        stderr_sha256 = (Get-FileHash -LiteralPath $stderr -Algorithm SHA256).Hash
        terminal_sha256 = (Get-FileHash -LiteralPath $terminal -Algorithm SHA256).Hash
    }
}
$shellAfter = (Get-ItemProperty -LiteralPath $registryPath -Name Shell -ErrorAction SilentlyContinue).Shell
$report = [ordered]@{
    schema = 'guardian-shell-lifecycle/v1'
    result = 'passed'
    app_sha256 = (Get-FileHash -LiteralPath $app -Algorithm SHA256).Hash
    guardian_sha256 = (Get-FileHash -LiteralPath $guardian -Algorithm SHA256).Hash
    registry_unchanged = $shellBefore -ceq $shellAfter
    explorer_before = $explorerBefore
    explorer_after = @(Get-Process explorer -ErrorAction SilentlyContinue).Id
    runs = $runs
    forbidden_signatures = @()
}
if (-not $report.registry_unchanged -or $report.explorer_after.Count -eq 0) {
    throw 'Guardian lifecycle did not preserve registry state and Explorer recovery'
}
[IO.File]::WriteAllText(
    (Join-Path $EvidenceDirectory 'report.json'),
    (($report | ConvertTo-Json -Depth 8) + [Environment]::NewLine),
    [Text.UTF8Encoding]::new($false)
)
$report | ConvertTo-Json -Depth 8
