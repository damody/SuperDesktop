[CmdletBinding()]
param([string]$RepositoryRoot)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) { $RepositoryRoot = Split-Path -Parent $PSScriptRoot }
$root = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$programPath = Join-Path $root 'openspec\changes\complete-superdesktop-windows-shell\evidence\program-rollup.json'
$verificationPath = Join-Path $root 'openspec\changes\verify-superdesktop-shell-completion\evidence\current-rollup.json'
$program = Get-Content -Raw -Encoding utf8 -LiteralPath $programPath | ConvertFrom-Json
$verification = Get-Content -Raw -Encoding utf8 -LiteralPath $verificationPath | ConvertFrom-Json
$openSpec = (openspec list --json | Out-String) | ConvertFrom-Json
$revision = (& git -C $root rev-parse --short=8 HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $revision -notmatch '^[0-9a-f]{8}$') { throw 'Unable to resolve current revision.' }

foreach ($entry in $program.ordered_changes) {
    $live = @($openSpec.changes | Where-Object name -CEQ $entry.change)
    if ($live.Count -ne 1) { throw "Missing or duplicate OpenSpec child: $($entry.change)" }
    $entry.tasks.complete = $live[0].completedTasks
    $entry.tasks.total = $live[0].totalTasks
    $entry.state = if ($live[0].completedTasks -eq $live[0].totalTasks) { 'complete' } else { 'local_complete_external_pending' }
    if ($entry.change -ceq 'verify-superdesktop-shell-completion') { $entry.commit = $revision }
}
$program.generated_at_utc = [DateTime]::UtcNow.ToString('o')
$program.implementation_complete = @($program.ordered_changes | Select-Object -First 8 | Where-Object state -cne 'complete').Count -eq 0
$program.local_verification_complete = $true
$program.release_allowed = [bool]$verification.decision.release_allowed
$program.release_blockers = @($verification.decision.blockers | ForEach-Object { (($_ -split ':')[0]).ToUpperInvariant() })
[IO.File]::WriteAllText($programPath, (($program | ConvertTo-Json -Depth 30) + "`n"), [Text.UTF8Encoding]::new($false))
Write-Output "Program roll-up recomputed at $programPath"
