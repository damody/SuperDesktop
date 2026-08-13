[CmdletBinding()]
param(
    [string]$WorkspaceRoot,
    [string]$OutputPath,
    [string]$ProbePath
)

$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path }
if (-not $OutputPath) { $OutputPath = Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/admission-zero-mutation-trace.json' }

$probe = if($ProbePath){$ProbePath}else{Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts/1.1/bin/capability_profile.exe'}
$currentSubstrateVerifier = Join-Path $PSScriptRoot 'verify-current-substrate-contract.ps1'
$probeSource = Join-Path $WorkspaceRoot 'crates/platform-win/examples/capability_profile.rs'
$allowlist = Join-Path $PSScriptRoot 'profile-allowlist.json'
$profileValidator = Join-Path $PSScriptRoot 'validate-profile-snapshot.ps1'
$programHandoff = Join-Path $WorkspaceRoot 'openspec/changes/build-superdesktop-shell-foundation/evidence/handoffs/2.1.json'
foreach ($input in @($probe, $currentSubstrateVerifier, $probeSource, $allowlist, $profileValidator, $programHandoff)) { if (-not (Test-Path -LiteralPath $input -PathType Leaf)) { throw "PROFILE_INPUT_MISSING: $input" } }
. $profileValidator

$signature = @'
using System;
using System.Runtime.InteropServices;
public static class Wave2ReadOnlySnapshot {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct APPBARDATA { public uint cbSize; public IntPtr hWnd; public uint uCallbackMessage; public uint uEdge; public RECT rc; public IntPtr lParam; }
  [DllImport("user32.dll", SetLastError=true)] public static extern bool SystemParametersInfo(uint action,uint parameter, ref RECT rect,uint flags);
  [DllImport("shell32.dll")] public static extern UIntPtr SHAppBarMessage(uint message, ref APPBARDATA data);
}
'@
Add-Type -TypeDefinition $signature

function Get-ReadOnlySnapshot([int]$Sequence, [int]$ExpectedSessionId) {
    $workArea = New-Object Wave2ReadOnlySnapshot+RECT
    $spi = [Wave2ReadOnlySnapshot]::SystemParametersInfo(48, 0, [ref]$workArea, 0)
    if (-not $spi) { throw 'SPI_GETWORKAREA_FAILED' }
    $appBar = New-Object Wave2ReadOnlySnapshot+APPBARDATA
    $appBar.cbSize = [Runtime.InteropServices.Marshal]::SizeOf($appBar)
    $appBarResult = [Wave2ReadOnlySnapshot]::SHAppBarMessage(5, [ref]$appBar).ToUInt64()
    if ($appBarResult -eq 0) { throw 'ABM_GETTASKBARPOS_FAILED' }
    [ordered]@{
        sequence = $Sequence
        captured_at = (Get-Date).ToUniversalTime().ToString('o')
        explorer = @(Get-Process explorer -ErrorAction SilentlyContinue | Where-Object { $_.SessionId -eq $ExpectedSessionId } | Sort-Object Id | ForEach-Object { [ordered]@{ pid=$_.Id; session_id=$_.SessionId; creation_time_utc=$_.StartTime.ToUniversalTime().ToString('o'); path=$_.Path } })
        appbar_query = [ordered]@{ message='ABM_GETTASKBARPOS'; result=$appBarResult; edge=$appBar.uEdge; left=$appBar.rc.Left; top=$appBar.rc.Top; right=$appBar.rc.Right; bottom=$appBar.rc.Bottom }
        work_area = [ordered]@{ action='SPI_GETWORKAREA'; left=$workArea.Left; top=$workArea.Top; right=$workArea.Right; bottom=$workArea.Bottom }
    }
}

function Get-StringSha256([string]$Value) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Value)))).Replace('-', '') }
    finally { $sha.Dispose() }
}

& powershell -NoProfile -ExecutionPolicy Bypass -File $currentSubstrateVerifier -WorkspaceRoot $WorkspaceRoot | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'CURRENT_SUBSTRATE_CONTRACT_FAILED' }
$currentSessionId = (Get-Process -Id $PID).SessionId
$programArchive = Get-Content -Raw -Encoding utf8 $programHandoff | ConvertFrom-Json
if (-not $programArchive.archive_path -or -not $programArchive.archive_revision -or -not $programArchive.child_contract_sha256) { throw 'PROGRAM_ARCHIVE_HANDOFF_INVALID' }
$before = Get-ReadOnlySnapshot 1 $currentSessionId
$profile = Get-ProfileSnapshot -AllowlistPath $allowlist
Assert-ProfileSnapshot -AllowlistPath $allowlist -Snapshot $profile
$admissionCapturedAt = (Get-Date).ToUniversalTime().ToString('o')
$probeResult = (& $probe | ConvertFrom-Json)
$probeExit = $LASTEXITCODE
$after = Get-ReadOnlySnapshot 3 $currentSessionId
$beforePayload = [ordered]@{explorer=$before.explorer;appbar_query=$before.appbar_query;work_area=$before.work_area}
$afterPayload = [ordered]@{explorer=$after.explorer;appbar_query=$after.appbar_query;work_area=$after.work_area}
$beforeCanonical = $beforePayload | ConvertTo-Json -Depth 8 -Compress
$afterCanonical = $afterPayload | ConvertTo-Json -Depth 8 -Compress
$trace = [ordered]@{
    schema_version = '1.0.0'
    procedure = 'read-only profile admission probe; no AppBar/Hook/Explorer mutation APIs are invoked'
    inputs = [ordered]@{
        current_substrate_verifier_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $currentSubstrateVerifier).Hash
        program_archive_handoff_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $programHandoff).Hash
        program_archive_handoff_path = 'openspec/changes/build-superdesktop-shell-foundation/evidence/handoffs/2.1.json'
        archive_path = [string]$programArchive.archive_path
        archive_revision = [string]$programArchive.archive_revision
        archive_child_contract_sha256 = [string]$programArchive.child_contract_sha256
        probe_source_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $probeSource).Hash
        probe_binary_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $probe).Hash
        profile_allowlist_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $allowlist).Hash
    }
    before = $before
    profile = $profile
    admission = [ordered]@{ sequence=2; captured_at=$admissionCapturedAt; probe_exit_status=$probeExit; result=$probeResult }
    after = $after
    equality_assertion = [ordered]@{
        compared_fields = @('explorer.pid', 'explorer.creation_time_utc', 'explorer.path', 'appbar_query', 'work_area')
        before_sha256 = (Get-StringSha256 $beforeCanonical)
        after_sha256 = (Get-StringSha256 $afterCanonical)
        passed = ($beforeCanonical -eq $afterCanonical)
    }
}
if ($trace.before.explorer.Count -eq 0 -or $trace.after.explorer.Count -eq 0 -or $trace.admission.probe_exit_status -ne 0 -or -not $trace.admission.result.admitted -or -not $trace.equality_assertion.passed) { throw 'ADMISSION_OR_ZERO_MUTATION_ASSERTION_FAILED' }
$directory = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Force $directory | Out-Null
$trace | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
Write-Output "Read-only admission and zero-mutation trace passed: $OutputPath"
