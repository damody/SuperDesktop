[CmdletBinding()]
param([string]$WorkspaceRoot)

$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path }
$allowlist = Join-Path $PSScriptRoot 'profile-allowlist.json'
$policy = Get-Content -Raw -Encoding utf8 $allowlist | ConvertFrom-Json

function Test-Rule([object]$Rule, [hashtable]$Actual) {
    foreach ($expected in $Rule.expected.psobject.Properties) {
        if (-not $Actual.ContainsKey($expected.Name) -or $Actual[$expected.Name] -ne [string]$expected.Value) { return $false }
    }
    if ($Rule.reject_unknown_values) {
        foreach ($name in $Actual.Keys) { if (-not $Rule.expected.psobject.Properties.Name.Contains($name)) { return $false } }
    }
    if ($Rule.important_name_pattern) {
        foreach ($name in $Actual.Keys) {
            if ($name -match $Rule.important_name_pattern -and -not $Rule.expected.psobject.Properties.Name.Contains($name)) { return $false }
        }
    }
    return $true
}

$advanced = $policy.keys.'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced'
$actual = @{}
foreach ($expected in $advanced.expected.psobject.Properties) { $actual[$expected.Name] = [string]$expected.Value }
if (-not (Test-Rule $advanced $actual)) { throw 'ALLOWLIST_BASELINE_FIXTURE_FAILED' }
$mutated = @{} + $actual; $mutated['TaskbarAl'] = '99'
if (Test-Rule $advanced $mutated) { throw 'ALLOWLIST_MUTATED_TASKBAR_VALUE_ADMITTED' }
$unknown = @{} + $actual; $unknown['StartFutureLayout'] = '1'
if (Test-Rule $advanced $unknown) { throw 'ALLOWLIST_UNKNOWN_START_VALUE_ADMITTED' }
Write-Output 'Profile allowlist negative fixtures passed: mutated TaskbarAl and unknown StartFutureLayout rejected.'
