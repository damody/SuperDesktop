[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$TracePath,
    [Parameter(Mandatory)][string]$SuccessorContractPath,
    [Parameter(Mandatory)][string]$AdmissionTracePath,
    [Parameter(Mandatory)][string]$BinaryPath,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$ExpectedTraceSha256
)

$ErrorActionPreference = 'Stop'

function Fail([string]$Code) { throw "NATIVE_TRACE_$Code" }
function Require-Path([string]$Path, [string]$Label) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { Fail "MISSING_$Label" }
}
function Hash([string]$Path) { (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToUpperInvariant() }
function Require-Field($Object, [string]$Name) {
    if ($null -eq $Object.PSObject.Properties[$Name]) { Fail "FIELD_MISSING_$Name" }
    $Object.$Name
}
function Require-True($Value, [string]$Code) {
    if ($Value -isnot [bool] -or -not $Value) { Fail $Code }
}
function Require-False($Value, [string]$Code) {
    if ($Value -isnot [bool] -or $Value) { Fail $Code }
}
function Require-ExactArray($Actual, [string[]]$Expected, [string]$Code) {
    $values = @($Actual)
    if ($values.Count -ne $Expected.Count -or (@($values | Sort-Object) -join '|') -ne (@($Expected | Sort-Object) -join '|')) { Fail $Code }
}
function Require-Resource($Value, [string]$Label) {
    foreach ($name in @('process_handles', 'user_objects', 'gdi_objects')) {
        if ($null -eq $Value.PSObject.Properties[$name] -or [int64]$Value.$name -lt 0) { Fail "RESOURCE_$Label" }
    }
}

Require-Path $TracePath 'TRACE'
Require-Path $SuccessorContractPath 'SUCCESSOR_CONTRACT'
Require-Path $AdmissionTracePath 'ADMISSION_TRACE'
Require-Path $BinaryPath 'BINARY'

$actualTraceHash = Hash $TracePath
if ($actualTraceHash -ne $ExpectedTraceSha256.ToUpperInvariant()) { Fail 'TRACE_HASH_BINDING' }
try { $trace = Get-Content -Raw -Encoding utf8 -LiteralPath $TracePath | ConvertFrom-Json } catch { Fail 'JSON_PARSE' }

foreach ($field in @(
    'schema','gpui_window_opened','hwnd','owner_pid','owner_thread','session_id','gpui_window_id','generation',
    'lifecycle','owned_events','adapter_events','callbacks_before_close','callbacks_after_close',
    'late_event_rejected','wm_ncdestroy_observed','on_window_closed_observed','callback_state_outstanding',
    'fatal_callback','resources_before','resources_after','resource_deadline','resource_thresholds',
    'raw_message_contract','input_contract','appbar_or_hook_mutations','bridge_created_hwnd',
    'bridge_destroyed_hwnd','preview_only'
)) { [void](Require-Field $trace $field) }

if ($trace.schema -ne 'native-window-trace/v2') { Fail 'SCHEMA' }
Require-True $trace.gpui_window_opened 'GPUI_WINDOW_NOT_OPENED'
Require-True ([int64]$trace.hwnd -gt 0 -and [int64]$trace.owner_pid -gt 0 -and [int64]$trace.owner_thread -gt 0 -and [int64]$trace.session_id -ge 0 -and [int64]$trace.gpui_window_id -gt 0 -and [int64]$trace.generation -gt 0) 'IDENTITY'
Require-False $trace.appbar_or_hook_mutations 'APPBAR_OR_HOOK_MUTATION'
Require-False $trace.bridge_created_hwnd 'BRIDGE_CREATED_HWND'
Require-False $trace.bridge_destroyed_hwnd 'BRIDGE_DESTROYED_HWND'
Require-True $trace.preview_only 'PREVIEW_MODE'

$owned = @($trace.owned_events)
if ($owned.Count -ne 3) { Fail 'OWNED_EVENT_COUNT' }
$dpi = @($owned | Where-Object kind -eq 'dpi-changed')
$display = @($owned | Where-Object kind -eq 'display-changed')
$activation = @($owned | Where-Object kind -eq 'activation')
if ($dpi.Count -ne 1 -or [int]$dpi[0].x -ne 96 -or [int]$dpi[0].y -ne 96 -or [int]$dpi[0].suggested_rect.left -ne 0 -or [int]$dpi[0].suggested_rect.top -ne 0 -or [int]$dpi[0].suggested_rect.right -ne 320 -or [int]$dpi[0].suggested_rect.bottom -ne 180) { Fail 'OWNED_DPI_PAYLOAD' }
if ($display.Count -ne 1 -or [int]$display[0].bits_per_pixel -ne 32 -or [int]$display[0].width -ne 320 -or [int]$display[0].height -ne 180) { Fail 'OWNED_DISPLAY_PAYLOAD' }
if ($activation.Count -ne 1 -or [int]$activation[0].state -ne 1) { Fail 'OWNED_ACTIVATION_PAYLOAD' }
if (@($trace.adapter_events).Count -ne 0) { Fail 'PRIVATE_ADAPTER_CLAIM' }
foreach ($field in @('dpi_rect_valid_for_send','display_parameters_valid','activation_parameters_valid')) {
    Require-True (Require-Field $trace.raw_message_contract $field) "RAW_CONTRACT_$field"
}

$lifecycle = @($trace.lifecycle)
if ($lifecycle.Count -ne 5) { Fail 'LIFECYCLE_COUNT' }
$sequence = @{}
foreach ($entry in $lifecycle) {
    foreach ($field in @('state','sequence')) { [void](Require-Field $entry $field) }
    if ($sequence.ContainsKey($entry.state) -or [int64]$entry.sequence -le 0) { Fail 'LIFECYCLE_DUPLICATE_OR_ZERO' }
    $sequence[$entry.state] = [int64]$entry.sequence
}
foreach ($state in @('attached','closing','wm-ncdestroy','on-window-closed','finalized')) {
    if (-not $sequence.ContainsKey($state)) { Fail "LIFECYCLE_MISSING_$state" }
}
if (-not ($sequence.attached -lt $sequence.closing -and $sequence.closing -lt $sequence.'wm-ncdestroy' -and $sequence.closing -lt $sequence.'on-window-closed' -and $sequence.'wm-ncdestroy' -lt $sequence.finalized -and $sequence.'on-window-closed' -lt $sequence.finalized)) { Fail 'LIFECYCLE_ORDER' }
Require-True ([bool]$trace.wm_ncdestroy_observed -and [bool]$trace.on_window_closed_observed -and [bool]$trace.late_event_rejected) 'TERMINAL_OR_FENCE'
if ([int64]$trace.callbacks_before_close -lt 4 -or [int64]$trace.callbacks_after_close -lt 1 -or [int64]$trace.callback_state_outstanding -ne 0 -or $null -ne $trace.fatal_callback) { Fail 'CALLBACK_TERMINAL' }

Require-Resource $trace.resources_before 'BEFORE'
Require-Resource $trace.resources_after 'AFTER'
foreach ($field in @('poll_interval_ms','max_ticks')) { if ([int64](Require-Field $trace.resource_deadline $field) -le 0) { Fail "DEADLINE_$field" } }
foreach ($field in @('process_handle_delta_max','user_object_delta_max','gdi_object_delta_max')) { if ([int64](Require-Field $trace.resource_thresholds $field) -lt 0) { Fail "THRESHOLD_$field" } }
$deltas = @{
    process_handles = [int64]$trace.resources_after.process_handles - [int64]$trace.resources_before.process_handles
    user_objects = [int64]$trace.resources_after.user_objects - [int64]$trace.resources_before.user_objects
    gdi_objects = [int64]$trace.resources_after.gdi_objects - [int64]$trace.resources_before.gdi_objects
}
if ($deltas.process_handles -gt [int64]$trace.resource_thresholds.process_handle_delta_max -or $deltas.user_objects -gt [int64]$trace.resource_thresholds.user_object_delta_max -or $deltas.gdi_objects -gt [int64]$trace.resource_thresholds.gdi_object_delta_max) { Fail 'RESOURCE_CONVERGENCE' }

$input = $trace.input_contract
foreach ($field in @('successor_contract_sha256','admission_trace_sha256','binary_sha256')) {
    $value = [string](Require-Field $input $field)
    if ($value -notmatch '^[A-F0-9]{64}$') { Fail "INPUT_HASH_FORMAT_$field" }
}
if ($input.successor_contract_sha256 -ne (Hash $SuccessorContractPath) -or $input.admission_trace_sha256 -ne (Hash $AdmissionTracePath) -or $input.binary_sha256 -ne (Hash $BinaryPath)) { Fail 'INPUT_HASH_BINDING' }

Write-Output "Native window trace semantic verification passed: $actualTraceHash"
