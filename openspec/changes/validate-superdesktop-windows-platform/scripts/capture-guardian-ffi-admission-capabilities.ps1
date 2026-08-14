[CmdletBinding()]
param([string]$WorkspaceRoot)

$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path }
$artifactRoot = Join-Path $WorkspaceRoot 'openspec/changes/validate-superdesktop-windows-platform/evidence/artifacts'
$profileCapture = Join-Path $PSScriptRoot 'capture-profile-admission.ps1'
$release = Join-Path $WorkspaceRoot 'target/release/examples'

$signature = @'
using System;
using System.Runtime.InteropServices;
public static class FinalCapabilitySnapshot {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct APPBARDATA { public uint cbSize; public IntPtr hWnd; public uint uCallbackMessage; public uint uEdge; public RECT rc; public IntPtr lParam; }
  [DllImport("user32.dll", SetLastError=true)] public static extern bool SystemParametersInfo(uint action,uint parameter, ref RECT rect,uint flags);
  [DllImport("shell32.dll")] public static extern UIntPtr SHAppBarMessage(uint message, ref APPBARDATA data);
}
'@
Add-Type -TypeDefinition $signature

function Get-Snapshot([int]$Sequence) {
    $session = (Get-Process -Id $PID).SessionId
    $work = New-Object FinalCapabilitySnapshot+RECT
    if (-not [FinalCapabilitySnapshot]::SystemParametersInfo(48, 0, [ref]$work, 0)) { throw 'SPI_GETWORKAREA_FAILED' }
    $bar = New-Object FinalCapabilitySnapshot+APPBARDATA
    $bar.cbSize = [Runtime.InteropServices.Marshal]::SizeOf($bar)
    $barResult = [FinalCapabilitySnapshot]::SHAppBarMessage(5, [ref]$bar).ToUInt64()
    if ($barResult -eq 0) { throw 'ABM_GETTASKBARPOS_FAILED' }
    [ordered]@{
        sequence=$Sequence
        captured_at=(Get-Date).ToUniversalTime().ToString('o')
        explorer=@(Get-Process explorer -ErrorAction Stop | Where-Object SessionId -eq $session | Sort-Object Id | ForEach-Object { [ordered]@{pid=$_.Id;session_id=$_.SessionId;creation_time_utc=$_.StartTime.ToUniversalTime().ToString('o');path=$_.Path} })
        appbar_query=[ordered]@{message='ABM_GETTASKBARPOS';result=$barResult;edge=$bar.uEdge;left=$bar.rc.Left;top=$bar.rc.Top;right=$bar.rc.Right;bottom=$bar.rc.Bottom}
        work_area=[ordered]@{action='SPI_GETWORKAREA';left=$work.Left;top=$work.Top;right=$work.Right;bottom=$work.Bottom}
    }
}
function Get-Comparable($Snapshot) {
    [ordered]@{explorer=$Snapshot.explorer;appbar_query=$Snapshot.appbar_query;work_area=$Snapshot.work_area} | ConvertTo-Json -Depth 8 -Compress
}
function Assert-EqualSnapshot($Before, $After, [string]$Name) {
    if ((Get-Comparable $Before) -cne (Get-Comparable $After)) { throw "${Name}_EXTERNAL_MUTATION" }
}
function Copy-Binary([string]$Source, [string]$Destination) {
    New-Item -ItemType Directory -Force (Split-Path -Parent $Destination) | Out-Null
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

Push-Location $WorkspaceRoot
try {
    cargo build -p platform-win --examples --release --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'PLATFORM_EXAMPLE_BUILD_FAILED' }
    cargo build -p desktop-ui --example gpui_callback_panic_capability --release --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'GPUI_PANIC_EXAMPLE_BUILD_FAILED' }

    $profile = Join-Path $release 'capability_profile.exe'
    $guardian = Join-Path $release 'guardian_lease_capability.exe'
    $ffi = Join-Path $release 'ffi_boundary_capability.exe'
    $admission = Join-Path $release 'admission_fixture_capability.exe'
    $gpuiPanic = Join-Path $release 'gpui_callback_panic_capability.exe'
    Copy-Binary $guardian (Join-Path $artifactRoot '3.1/bin/guardian_lease_capability.exe')
    Copy-Binary $ffi (Join-Path $artifactRoot '3.2/bin/ffi_boundary_capability.exe')
    Copy-Binary $gpuiPanic (Join-Path $artifactRoot '3.2/bin/gpui_callback_panic_capability.exe')
    Copy-Binary $admission (Join-Path $artifactRoot '3.3/bin/admission_fixture_capability.exe')

    foreach ($section in @('3.1','3.2','3.3')) {
        & powershell -NoProfile -ExecutionPolicy Bypass -File $profileCapture -WorkspaceRoot $WorkspaceRoot -ProbePath $profile -OutputPath (Join-Path $artifactRoot "$section/pre-mutation-admission-trace.json") | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "${section}_ADMISSION_FAILED" }
    }

    $before = Get-Snapshot 1
    $guardianWork = Join-Path $artifactRoot '3.1/run-valid'
    New-Item -ItemType Directory -Force $guardianWork | Out-Null
    $valid = (& $guardian --controller --work-dir $guardianWork --case valid | ConvertFrom-Json)
    if ($LASTEXITCODE -ne 0) { throw 'GUARDIAN_VALID_FAILED' }
    $after = Get-Snapshot 2
    Assert-EqualSnapshot $before $after 'GUARDIAN'
    $requiredRejects = @('ForgedPid','StaleCreationTime','WrongSession','WrongExecutable','FileIdentityMismatch','BadNonce','DuplicateClaim','UnexpectedInheritedHandle','InsufficientProcessRights')
    $observedRejects = @($valid.negative_fixtures.typed_reject)
    foreach ($reject in $requiredRejects) { if ($reject -notin $observedRejects) { throw "GUARDIAN_REJECT_MISSING_$reject" } }
    if ($valid.controller_resources_before.process_handles -ne $valid.controller_resources_after.process_handles -or $valid.child_terminal.released_inherited_handles -ne 2 -or $valid.parent_report.owned_process_and_thread_handles_closed -ne 3 -or $valid.child_terminal.unique_success_terminal_count -ne 1) { throw 'GUARDIAN_HANDLE_LIFECYCLE_FAILED' }
    [ordered]@{
        schema='guardian-lease-evidence/v1';before=$before;valid=$valid;after=$after
        equality_assertion=[ordered]@{passed=$true;compared_fields=@('explorer','appbar_query','work_area')}
        source_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot 'crates/platform-win/src/common/guardian_lease.rs')).Hash
        binary_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $artifactRoot '3.1/bin/guardian_lease_capability.exe')).Hash
    } | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 (Join-Path $artifactRoot '3.1/guardian-lease-trace.json')

    $before = Get-Snapshot 1
    $ffiResult = (& $ffi | ConvertFrom-Json)
    if ($LASTEXITCODE -ne 0) { throw 'FFI_BOUNDARY_FAILED' }
    $gpuiTracePath = Join-Path $artifactRoot '3.2/gpui-callback-panic-trace.json'
    & $gpuiPanic --controller --output $gpuiTracePath | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'GPUI_PANIC_FIXTURE_FAILED' }
    $gpuiResult = Get-Content -Raw -Encoding utf8 $gpuiTracePath | ConvertFrom-Json
    $after = Get-Snapshot 2
    Assert-EqualSnapshot $before $after 'FFI'
    if ($ffiResult.unwind_crossed_abi -or $ffiResult.panic.typed_fatal -ne 'CallbackPanic' -or $ffiResult.double_callback.typed_fatal -ne 'ReentrantCallback' -or $ffiResult.shutdown_race.typed_fatal -ne 'ShutdownRace') { throw 'SUPERDESKTOP_FFI_SEMANTICS_FAILED' }
    if (-not $gpuiResult.capability_passed -or -not $gpuiResult.typed_fatal_event -or -not $gpuiResult.backend_hwnd_terminal -or -not $gpuiResult.gpui_window_closed_terminal -or $gpuiResult.disposition -ne 'go') { throw 'PATCHED_GPUI_NO_UNWIND_TERMINAL_CONTRACT_FAILED' }
    [ordered]@{
        schema='ffi-capability-evidence/v1';before=$before;superdesktop_boundary=$ffiResult;pinned_gpui_backend=$gpuiResult;after=$after
        equality_assertion=[ordered]@{passed=$true;compared_fields=@('explorer','appbar_query','work_area')}
        abi_source_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot 'crates/platform-win/src/common/ffi_boundary.rs')).Hash
        gpui_wndproc_source_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot 'vendor/gpui_windows/src/window.rs')).Hash
        gpui_platform_source_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot 'vendor/gpui_windows/src/platform.rs')).Hash
        gpui_callback_source_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot 'vendor/gpui/src/app/context.rs')).Hash
        ffi_binary_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $artifactRoot '3.2/bin/ffi_boundary_capability.exe')).Hash
        gpui_binary_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $artifactRoot '3.2/bin/gpui_callback_panic_capability.exe')).Hash
    } | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 (Join-Path $artifactRoot '3.2/ffi-panic-evidence.json')

    $before = Get-Snapshot 1
    $fixtureResult = (& $admission | ConvertFrom-Json)
    if ($LASTEXITCODE -ne 0) { throw 'ADMISSION_FIXTURE_RUN_FAILED' }
    $after = Get-Snapshot 2
    Assert-EqualSnapshot $before $after 'ADMISSION_FIXTURE'
    $fixtureEvidence = @($fixtureResult.fixtures | ForEach-Object { [ordered]@{fixture=$_.fixture;probe_result=$_;before=$before;after=$after;zero_mutation=$true} })
    if ($fixtureEvidence.Count -ne 4 -or @($fixtureResult.fixtures | Where-Object { $_.admitted -or $_.mutations_attempted }).Count -ne 0) { throw 'ADMISSION_FIXTURE_FAIL_CLOSED_FAILED' }
    [ordered]@{
        schema='admission-fixture-evidence/v1';adapter=$fixtureResult.adapter;fixtures=$fixtureEvidence
        source_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $WorkspaceRoot 'crates/platform-win/src/common/admission.rs')).Hash
        binary_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $artifactRoot '3.3/bin/admission_fixture_capability.exe')).Hash
    } | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 (Join-Path $artifactRoot '3.3/admission-fixtures.json')
} finally {
    Pop-Location
}

Write-Output 'Guardian, FFI, pinned GPUI panic, and admission fixture evidence captured.'
