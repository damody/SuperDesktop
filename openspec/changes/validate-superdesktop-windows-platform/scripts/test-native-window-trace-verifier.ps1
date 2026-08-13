[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$verifier = Join-Path $PSScriptRoot 'verify-native-window-trace.ps1'
$scratch = Join-Path ([System.IO.Path]::GetTempPath()) ("superdesktop-native-trace-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null

try {
    $contract = Join-Path $scratch 'successor-contract.json'
    $admission = Join-Path $scratch 'admission-trace.json'
    $binary = Join-Path $PSHOME 'powershell.exe'
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { $binary = (Get-Process -Id $PID).Path }
    Set-Content -Encoding utf8 -NoNewline -LiteralPath $contract -Value '{"contract":"fixture"}'
    Set-Content -Encoding utf8 -NoNewline -LiteralPath $admission -Value '{"admission":"fixture"}'
    $hash = { param([string]$path) (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToUpperInvariant() }
    $base = [ordered]@{
        schema = 'native-window-trace/v2'; gpui_window_opened = $true; hwnd = 100; owner_pid = 101; owner_thread = 102; session_id = 1; gpui_window_id = 10; generation = 1
        lifecycle = @(
            [ordered]@{state='attached';sequence=1}, [ordered]@{state='closing';sequence=2}, [ordered]@{state='wm-ncdestroy';sequence=3}, [ordered]@{state='on-window-closed';sequence=4}, [ordered]@{state='finalized';sequence=5}
        )
        owned_events = @(
            [ordered]@{kind='dpi-changed';x=96;y=96;suggested_rect=[ordered]@{left=0;top=0;right=320;bottom=180}},
            [ordered]@{kind='display-changed';bits_per_pixel=32;width=320;height=180},
            [ordered]@{kind='activation';state=1}
        ); adapter_events = @(); callbacks_before_close = 4; callbacks_after_close = 1; late_event_rejected = $true; wm_ncdestroy_observed = $true; on_window_closed_observed = $true; callback_state_outstanding = 0; fatal_callback = $null
        resources_before = [ordered]@{process_handles=10;user_objects=5;gdi_objects=2}; resources_after = [ordered]@{process_handles=11;user_objects=5;gdi_objects=2}
        resource_deadline = [ordered]@{poll_interval_ms=50;max_ticks=40}; resource_thresholds = [ordered]@{process_handle_delta_max=2;user_object_delta_max=2;gdi_object_delta_max=2}
        raw_message_contract = [ordered]@{dpi_rect_valid_for_send=$true;display_parameters_valid=$true;activation_parameters_valid=$true}
        input_contract = [ordered]@{successor_contract_sha256=& $hash $contract;admission_trace_sha256=& $hash $admission;binary_sha256=& $hash $binary}
        appbar_or_hook_mutations = $false; bridge_created_hwnd = $false; bridge_destroyed_hwnd = $false; preview_only = $true
    }
    $positive = Join-Path $scratch 'positive.json'
    $base | ConvertTo-Json -Depth 8 -Compress | Set-Content -Encoding utf8 -NoNewline -LiteralPath $positive
    $positiveHash = & $hash $positive
    & $verifier -TracePath $positive -SuccessorContractPath $contract -AdmissionTracePath $admission -BinaryPath $binary -ExpectedTraceSha256 $positiveHash | Out-Null

    $copied = Join-Path $scratch 'copied-valid-but-unbound.json'
    $copy = Get-Content -Raw -Encoding utf8 -LiteralPath $positive | ConvertFrom-Json; $copy.gpui_window_id = 11
    $copy | ConvertTo-Json -Depth 8 -Compress | Set-Content -Encoding utf8 -NoNewline -LiteralPath $copied
    try { & $verifier -TracePath $copied -SuccessorContractPath $contract -AdmissionTracePath $admission -BinaryPath $binary -ExpectedTraceSha256 $positiveHash | Out-Null; throw 'copy substitution unexpectedly passed' } catch { if ($_.Exception.Message -notmatch 'NATIVE_TRACE_TRACE_HASH_BINDING') { throw } }

    $mutated = Join-Path $scratch 'mutated.json'
    $mutation = Get-Content -Raw -Encoding utf8 -LiteralPath $positive | ConvertFrom-Json; $mutation.owned_events[0].suggested_rect.right = 319
    $mutation | ConvertTo-Json -Depth 8 -Compress | Set-Content -Encoding utf8 -NoNewline -LiteralPath $mutated
    $mutationHash = & $hash $mutated
    try { & $verifier -TracePath $mutated -SuccessorContractPath $contract -AdmissionTracePath $admission -BinaryPath $binary -ExpectedTraceSha256 $mutationHash | Out-Null; throw 'semantic mutation unexpectedly passed' } catch { if ($_.Exception.Message -notmatch 'NATIVE_TRACE_OWNED_DPI_PAYLOAD') { throw } }
    $terminalOrder = Join-Path $scratch 'terminal-before-closing.json'
    $orderMutation = Get-Content -Raw -Encoding utf8 -LiteralPath $positive | ConvertFrom-Json; $orderMutation.lifecycle[1].sequence = 4; $orderMutation.lifecycle[2].sequence = 2; $orderMutation.lifecycle[3].sequence = 3
    $orderMutation | ConvertTo-Json -Depth 8 -Compress | Set-Content -Encoding utf8 -NoNewline -LiteralPath $terminalOrder
    $orderHash = & $hash $terminalOrder
    try { & $verifier -TracePath $terminalOrder -SuccessorContractPath $contract -AdmissionTracePath $admission -BinaryPath $binary -ExpectedTraceSha256 $orderHash | Out-Null; throw 'terminal-order mutation unexpectedly passed' } catch { if ($_.Exception.Message -notmatch 'NATIVE_TRACE_LIFECYCLE_ORDER') { throw } }
    $overThreshold = Join-Path $scratch 'resource-over-threshold.json'
    $resourceMutation = Get-Content -Raw -Encoding utf8 -LiteralPath $positive | ConvertFrom-Json; $resourceMutation.resources_after.gdi_objects = 99
    $resourceMutation | ConvertTo-Json -Depth 8 -Compress | Set-Content -Encoding utf8 -NoNewline -LiteralPath $overThreshold
    $resourceHash = & $hash $overThreshold
    try { & $verifier -TracePath $overThreshold -SuccessorContractPath $contract -AdmissionTracePath $admission -BinaryPath $binary -ExpectedTraceSha256 $resourceHash | Out-Null; throw 'resource mutation unexpectedly passed' } catch { if ($_.Exception.Message -notmatch 'NATIVE_TRACE_RESOURCE_CONVERGENCE') { throw } }
    $booleanString = Join-Path $scratch 'boolean-string-substitution.json'
    $booleanMutation = Get-Content -Raw -Encoding utf8 -LiteralPath $positive | ConvertFrom-Json; $booleanMutation.preview_only = 'false'
    $booleanMutation | ConvertTo-Json -Depth 8 -Compress | Set-Content -Encoding utf8 -NoNewline -LiteralPath $booleanString
    $booleanHash = & $hash $booleanString
    try { & $verifier -TracePath $booleanString -SuccessorContractPath $contract -AdmissionTracePath $admission -BinaryPath $binary -ExpectedTraceSha256 $booleanHash | Out-Null; throw 'boolean-string mutation unexpectedly passed' } catch { if ($_.Exception.Message -notmatch 'NATIVE_TRACE_PREVIEW_MODE') { throw } }
    Write-Output 'Native window trace verifier positive plus copy, raw-event, terminal-order, resource-threshold, and boolean-type negative fixtures passed.'
} finally {
    if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
}
