[CmdletBinding()]
param([string]$WorkspaceRoot)
$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }
$verifier = Join-Path $PSScriptRoot 'verify-archived-bootstrap-contract.ps1'

$old = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
try { $revisionOutput = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $verifier -WorkspaceRoot $WorkspaceRoot -ArchiveRevision HEAD 2>&1; $revisionExit = $LASTEXITCODE }
finally { $ErrorActionPreference = $old }
if ($revisionExit -ne 1 -or (($revisionOutput | Out-String) -notmatch 'parameter.*ArchiveRevision|ArchiveRevision.*parameter')) { throw 'ARCHIVE_REVISION_OVERRIDE_NEGATIVE_FAILED' }

$programHandoffPath = Join-Path $WorkspaceRoot 'openspec/changes/build-superdesktop-shell-foundation/evidence/handoffs/2.1.json'
$stage = Join-Path $WorkspaceRoot ('build/archive-contract-negative-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $stage | Out-Null
$fixtureWorkspace = Join-Path $stage 'workspace'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent (Join-Path $fixtureWorkspace 'openspec/changes/build-superdesktop-shell-foundation/evidence/handoffs/2.1.json')) | Out-Null
Copy-Item -LiteralPath $programHandoffPath -Destination (Join-Path $fixtureWorkspace 'openspec/changes/build-superdesktop-shell-foundation/evidence/handoffs/2.1.json')
$handoffPath = Join-Path $fixtureWorkspace 'openspec/changes/build-superdesktop-shell-foundation/evidence/handoffs/2.1.json'
$handoff = Get-Content -Raw -LiteralPath $handoffPath | ConvertFrom-Json
$handoff.archive_path = '..\SuperDesktop-escape'
[IO.File]::WriteAllText($handoffPath, ($handoff | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
$old = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
try { $pathOutput = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $verifier -WorkspaceRoot $fixtureWorkspace 2>&1; $pathExit = $LASTEXITCODE }
finally { $ErrorActionPreference = $old }
if ($pathExit -ne 1 -or (($pathOutput | Out-String) -notmatch 'BOOTSTRAP_ARCHIVE_PATH_INVALID')) { throw 'ARCHIVE_PATH_ESCAPE_NEGATIVE_FAILED' }

Write-Output 'Archived bootstrap negative fixtures passed: revision override rejected; separator-boundary path escape rejected.'
