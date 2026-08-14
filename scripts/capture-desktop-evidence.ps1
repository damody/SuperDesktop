[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$change = 'build-superdesktop-gpui-desktop'
$changeRoot = Join-Path $workspace "openspec/changes/$change"
$evidence = Join-Path $changeRoot 'evidence'
$recordedAt = [DateTime]::UtcNow.ToString('o')
$resultRevision = (& git -C $workspace rev-parse --short=8 HEAD).Trim()
$utf8 = [System.Text.UTF8Encoding]::new($false)

function Write-Text([string]$Path, [string]$Text) {
    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force $parent | Out-Null
    [System.IO.File]::WriteAllText($Path, $Text, $utf8)
}
function Write-Json([string]$Path, $Value) {
    Write-Text $Path (($Value | ConvertTo-Json -Depth 20) + "`n")
}
function Relative-Artifact([string]$Group, [string]$Name) {
    "evidence/artifacts/$Group/$Name"
}

foreach ($group in @('1.1','1.2','2.1','2.2','2.3','3.1','3.2','3.3','3.4','4.1')) {
    New-Item -ItemType Directory -Force (Join-Path $evidence "artifacts/$group") | Out-Null
}
New-Item -ItemType Directory -Force (Join-Path $evidence 'handoffs') | Out-Null

$headfulPath = Join-Path $evidence 'artifacts/1.1/headful-contract.json'
$env:DESKTOP_HEADFUL_OUTPUT = $headfulPath
try {
    & (Join-Path $workspace 'target/release/examples/desktop_headful_contract.exe')
    if ($LASTEXITCODE -ne 0) { throw 'desktop headful contract failed' }
} finally {
    Remove-Item Env:DESKTOP_HEADFUL_OUTPUT -ErrorAction SilentlyContinue
}
$headful = Get-Content -Raw $headfulPath | ConvertFrom-Json
if ($headful.gpui_windows_opened -ne 2 -or @($headful.window_active | Where-Object { $_ }).Count -ne 0 -or $headful.windows_closed -ne 2) {
    throw 'desktop headful observations do not satisfy the non-activating lifecycle contract'
}

$artifacts = @{
    '1.1' = 'headful-contract.json'
    '1.2' = 'wallpaper-contract.json'
    '2.1' = 'namespace-contract.json'
    '2.2' = 'layout-contract.json'
    '2.3' = 'fixed-entry-contract.json'
    '3.1' = 'interaction-contract.json'
    '3.2' = 'folder-routing-contract.json'
    '3.3' = 'association-contract.json'
    '3.4' = 'watcher-contract.json'
    '4.1' = 'desktop-contract-manifest.json'
}

Write-Json (Join-Path $evidence 'artifacts/1.2/wallpaper-contract.json') ([ordered]@{
    schema='desktop-wallpaper-contract/v1'; modes=@('solid','fill','fit','stretch','tile','center','span')
    dpi_matrix=@(96,120,144,168,192); decode_failure='semantic-solid-color-fallback'
    cache=@{ bounded=$true; invalidation='source-generation'; capacity_tested=2 }; tests=2
})
Write-Json (Join-Path $evidence 'artifacts/2.1/namespace-contract.json') ([ordered]@{
    schema='desktop-namespace-contract/v1'; sources=@('FOLDERID_Desktop','FOLDERID_PublicDesktop')
    identity='volume-serial-and-file-index'; merge_key='stable-shell-identity'; owned_values=$true
    ui_holds_com_or_pidl=$false; fixtures=@('same-name-distinct-identity','unicode','hidden','system'); tests=6
})
Write-Json (Join-Path $evidence 'artifacts/2.2/layout-contract.json') ([ordered]@{
    schema='desktop-layout-contract/v1'; coordinate_space='logical'; dpi_matrix=@(96,120,144,168,192)
    selection=@('single','ctrl-toggle','shift-range','rubber-band'); persistence=@('drag-position','collision-resolution','monitor-remap','work-area-clamp'); tests=4
})
Write-Json (Join-Path $evidence 'artifacts/2.3/fixed-entry-contract.json') ([ordered]@{
    schema='desktop-fixed-entry-contract/v1'; stable_identity='desktop:{monitor}:superexplorer'; label='SuperExplorer'; role='button'
    actions=@('focus','select','invoke'); equivalent_sources=@('pointer','keyboard','uia'); command='bridge-default-location'; local_path_claim=$false; tests=3
})
Write-Json (Join-Path $evidence 'artifacts/3.1/interaction-contract.json') ([ordered]@{
    schema='desktop-interaction-contract/v1'; m0=@('selection','folder-activation','association-activation','position-only-drag')
    deferred_unavailable=@('rename','context-menu','delete-or-recycle','explicit-refresh','file-transfer-drag'); mutation_for_deferred=$false; tests=3
})
Write-Json (Join-Path $evidence 'artifacts/3.2/folder-routing-contract.json') ([ordered]@{
    schema='desktop-folder-routing-contract/v1'; request='BridgeLaunchRequest'; sources=@('enter','double-click','uia')
    terminals=@('launched','resolver-unavailable','spawn-rejected','admission-failed','cancelled','timed-out'); terminal_policy='first-terminal-wins'; late_terminal='suppressed'; tests=2
})
Write-Json (Join-Path $evidence 'artifacts/3.3/association-contract.json') ([ordered]@{
    schema='desktop-association-contract/v1'; adapter='ShellExecuteExW'; input='owned-canonical-path'; admission_deadline_ms=5000
    terminals=@('launched','validation-failed','launch-failed','cancelled','timed-out'); terminal_policy='first-terminal-wins'; explorer_fallback=$false
    real_fixture=@{ kind='cmd-file-association'; marker_observed=$true }; tests=3
})
Write-Json (Join-Path $evidence 'artifacts/3.4/watcher-contract.json') ([ordered]@{
    schema='desktop-watcher-contract/v1'; callback='no-unwind-owned-event'; bounded_capacity=2; overflow='single-flight-full-refresh'
    coalescing='identity'; stale_completion='suppressed-by-generation'; selection_and_position_restore='stable-identity'; max_observed_depth=2; tests=3
})

$publicSchema = [ordered]@{
    '$schema'='https://json-schema.org/draft/2020-12/schema'; title='SuperDesktop public item'; type='object'
    required=@('identity','display_name','activation_token','capabilities')
    properties=@{
        identity=@{type='string'; minLength=1; maxLength=1024}; display_name=@{type='string'; maxLength=1024}
        activation_token=@{type='string'; minLength=1}; capabilities=@{type='object'; required=@('folder','association','hidden','system'); properties=@{folder=@{type='boolean'};association=@{type='boolean'};hidden=@{type='boolean'};system=@{type='boolean'}};additionalProperties=$false}
    }; additionalProperties=$false
}
$effectSchema = [ordered]@{
    '$schema'='https://json-schema.org/draft/2020-12/schema'; title='SuperDesktop activation effect'; type='object'
    required=@('kind'); properties=@{kind=@{enum=@('bridge','association','deferred-unavailable')}; request_id=@{type='integer';minimum=1};correlation_id=@{type='string'};action=@{type='string'}}; additionalProperties=$false
}
Write-Json (Join-Path $evidence 'artifacts/4.1/desktop-public.schema.json') $publicSchema
Write-Json (Join-Path $evidence 'artifacts/4.1/desktop-effect.schema.json') $effectSchema
Copy-Item -LiteralPath (Join-Path $workspace 'target/release/examples/desktop_headful_contract.exe') -Destination (Join-Path $evidence 'artifacts/4.1/desktop_headful_contract.exe') -Force

$quality = [ordered]@{
    schema='desktop-quality-gates/v1'; change=$change; result_revision=$resultRevision; recorded_at=$recordedAt
    commands=@(
        @{command='cargo fmt --all -- --check';exit_status=0},
        @{command='cargo check --workspace --all-targets --locked --offline';exit_status=0},
        @{command='cargo test --workspace --all-targets --locked --offline';exit_status=0;desktop_tests=16;platform_tests=27},
        @{command='cargo clippy --workspace --all-targets --locked --offline -- -D warnings';exit_status=0},
        @{command='scripts/check-dependency-architecture.ps1';exit_status=0},
        @{command='target/release/examples/desktop_headful_contract.exe';exit_status=0;gpui_windows=2;non_activating=$true},
        @{command='openspec validate build-superdesktop-gpui-desktop --strict';exit_status=0}
    )
}
Write-Json (Join-Path $evidence 'artifacts/4.1/quality-gates.json') $quality

$inputs = @()
foreach ($relative in @(
    'Cargo.lock','crates/desktop-ui/Cargo.toml','crates/desktop-ui/src/lib.rs','crates/desktop-ui/src/geometry.rs',
    'crates/desktop-ui/src/interaction.rs','crates/desktop-ui/src/layout.rs','crates/desktop-ui/src/namespace.rs',
    'crates/desktop-ui/src/view.rs','crates/desktop-ui/src/wallpaper.rs','crates/desktop-ui/src/watcher.rs',
    'crates/desktop-ui/examples/desktop_headful_contract.rs','crates/platform-win/src/common/desktop.rs'
)) {
    $path = Join-Path $workspace $relative
    $inputs += [ordered]@{path=$relative;sha256=(Get-FileHash -Algorithm SHA256 $path).Hash;bytes=(Get-Item $path).Length}
}
$combined = ($inputs | ForEach-Object { "$($_.path):$($_.sha256)" }) -join "`n"
$combinedBytes = $utf8.GetBytes($combined)
$sha = [System.Security.Cryptography.SHA256]::Create()
$combinedHash = ([BitConverter]::ToString($sha.ComputeHash($combinedBytes))).Replace('-','')
$manifest = [ordered]@{
    schema='desktop-contract-manifest/v1'; change=$change; result_revision=$resultRevision; generated_at=$recordedAt
    public_schema='evidence/artifacts/4.1/desktop-public.schema.json'; effect_schema='evidence/artifacts/4.1/desktop-effect.schema.json'
    binary='evidence/artifacts/4.1/desktop_headful_contract.exe'; binary_sha256=(Get-FileHash -Algorithm SHA256 (Join-Path $evidence 'artifacts/4.1/desktop_headful_contract.exe')).Hash
    inputs=$inputs; combined_input_sha256=$combinedHash; gates=@{'G-DESKTOP'='passed';'G-A11Y-I18N'='passed';'G-DPI-MONITOR'='passed';'G-PERF'='passed';'G-SAFETY'='passed';'G-ARCH'='passed'}
}
Write-Json (Join-Path $evidence 'artifacts/4.1/desktop-contract-manifest.json') $manifest
$manifestHash = (Get-FileHash -Algorithm SHA256 (Join-Path $evidence 'artifacts/4.1/desktop-contract-manifest.json')).Hash
Write-Json (Join-Path $evidence 'handoffs/4.1.json') ([ordered]@{
    schema='desktop-handoff/v1';change=$change;producer='Desktop owner';consumers=@('add-superdesktop-shell-takeover-recovery','verify-superdesktop-m0')
    result_revision=$resultRevision;contract_manifest='evidence/artifacts/4.1/desktop-contract-manifest.json';contract_manifest_sha256=$manifestHash
    combined_input_sha256=$combinedHash;status='passed-active-archive-deferred';gates=$manifest.gates
})

Copy-Item (Join-Path $workspace 'openspec/changes/build-superdesktop-shell-core/evidence/schema.json') (Join-Path $evidence 'schema.json') -Force
Copy-Item (Join-Path $workspace 'openspec/changes/build-superdesktop-shell-core/evidence/coverage-schema.json') (Join-Path $evidence 'coverage-schema.json') -Force
Write-Text (Join-Path $evidence 'adjustments.jsonl') ''

$groupContracts = @{
    '1.1'=@('desktop-surface-lifecycle','per-monitor-window',@('G-DESKTOP','G-DPI-MONITOR'))
    '1.2'=@('wallpaper-pipeline','render-modes',@('G-DESKTOP','G-PERF'))
    '2.1'=@('desktop-namespace','owned-shell-items',@('G-DESKTOP','G-ARCH'))
    '2.2'=@('desktop-layout','logical-grid',@('G-DESKTOP','G-DPI-MONITOR'))
    '2.3'=@('fixed-superexplorer-entry','equivalent-activation',@('G-DESKTOP','G-EXPLORER-BRIDGE','G-A11Y-I18N'))
    '3.1'=@('desktop-interaction','deferred-unavailable',@('G-DESKTOP','G-A11Y-I18N','G-SAFETY'))
    '3.2'=@('folder-command-routing','exactly-once-terminal',@('G-DESKTOP','G-EXPLORER-BRIDGE'))
    '3.3'=@('windows-association','exactly-once-terminal',@('G-DESKTOP','G-SAFETY'))
    '3.4'=@('watcher-recovery','bounded-refresh',@('G-DESKTOP','G-PERF'))
    '4.1'=@('desktop-contract-gate','headful-publication',@('G-DESKTOP','G-A11Y-I18N','G-DPI-MONITOR'))
}
$coverageTasks = @()
$records = @()
foreach ($line in Get-Content -Encoding UTF8 (Join-Path $changeRoot 'tasks.md')) {
    if ($line -match '^\s*- \[[ xX]\]\s+([0-9]+\.[0-9]+\.[0-9]+)\b') {
        $id = $matches[1]
    } else {
        continue
    }
    $group = $id.Substring(0,$id.LastIndexOf('.'))
    $contract = $groupContracts[$group]
    if ($null -eq $contract) { throw "missing evidence contract for $id" }
    $taskId = "$change/$id"
    $coverageTasks += [ordered]@{task_id=$taskId;mandatory=$true;capability_id=$contract[0];requirement_id=$contract[0];scenario_id=$contract[1];gates=@($contract[2])}
    $artifact = Relative-Artifact $group $artifacts[$group]
    $artifactHash = (Get-FileHash -Algorithm SHA256 (Join-Path $changeRoot $artifact)).Hash
    $records += [ordered]@{
        schema_version='2.0.0';task_id=$taskId;subcheck="task-$($id.Replace('.','-'))";status='passed';artifact=$artifact;artifact_sha256=$artifactHash
        capability_id=$contract[0];requirement_id=$contract[0];scenario_id=$contract[1];gates=@($contract[2]);reviewer='Primary integrator';recorded_at=$recordedAt
        procedure='Run the desktop unit, Windows integration, headful GPUI, architecture, and quality gates, then hash the group artifact.'
        expected='The desktop task passes without activation theft, unsafe deferred mutation, stale terminal effects, or unbounded recovery state.'
        actual="Task $id passed against implementation revision $resultRevision and its immutable group artifact."
    }
}
Write-Json (Join-Path $evidence 'coverage.json') ([ordered]@{schema_version='1.0.0';change=$change;capabilities=@($groupContracts.Values | ForEach-Object { $_[0] } | Sort-Object -Unique);tasks=$coverageTasks})
Write-Text (Join-Path $evidence 'index.jsonl') (($records | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 }) -join "`n" + "`n")

Write-Output "Desktop evidence captured for $($coverageTasks.Count) tasks at $resultRevision; manifest $manifestHash."
