[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot,

    [Parameter()]
    [string]$Fixture
)

$ErrorActionPreference = 'Stop'
if (-not $WorkspaceRoot) { $WorkspaceRoot = Split-Path -Parent $PSScriptRoot }

function Add-Diagnostic {
    param(
        [System.Collections.Generic.List[string]]$Diagnostics,
        [string]$Code,
        [string]$Message
    )

    $Diagnostics.Add("${Code}: ${Message}")
}

$allowlistPath = Join-Path $WorkspaceRoot 'scripts/architecture-allowlist.json'
$allowlist = Get-Content -Raw -Encoding UTF8 $allowlistPath | ConvertFrom-Json
$diagnostics = [System.Collections.Generic.List[string]]::new()

if ($Fixture) {
    $fixturePath = Join-Path $WorkspaceRoot $Fixture
    if (-not (Test-Path $fixturePath)) {
        throw "Fixture does not exist: $Fixture"
    }

    $fixtureManifest = Get-Content -Raw -Encoding UTF8 (Join-Path $fixturePath 'Cargo.toml')
    $fixtureSource = Get-ChildItem -Path (Join-Path $fixturePath 'src') -Filter '*.rs' -Recurse |
        ForEach-Object { Get-Content -Raw -Encoding UTF8 $_.FullName } | Out-String

    if ($fixtureManifest -match '(?m)^gpui\s*=') {
        Add-Diagnostic $diagnostics 'CORE_FORBIDDEN_DEPENDENCY' 'shell-core fixture declares gpui.'
    }
    if ($fixtureManifest -match '(?m)^windows\s*=') {
        Add-Diagnostic $diagnostics 'WINDOWS_BINDING_OUTSIDE_PLATFORM' 'non-platform fixture directly declares windows.'
    }
    if ($fixtureManifest -match '(?m)^\s*unsafe_code\s*=\s*"allow"' -or $fixtureSource -match '(?m)^\s*#!\[allow\(unsafe_code\)\]' -or $fixtureSource -match '(?m)^\s*unsafe\s*\{') {
        if ($Fixture -notmatch 'platform-unsafe-missing-safety') { Add-Diagnostic $diagnostics 'UNSAFE_OVERRIDE_OUTSIDE_PLATFORM' 'non-platform fixture permits or uses unsafe code.' }
    }
    if ($Fixture -match 'platform-unsafe-missing-safety' -and $fixtureSource -match '(?m)^\s*unsafe\s*\{' -and $fixtureSource -notmatch '(?m)^\s*//\s*SAFETY:') {
        Add-Diagnostic $diagnostics 'UNSAFE_WITHOUT_SAFETY_INVARIANT' 'platform fixture unsafe block lacks an adjacent SAFETY invariant.'
    }

    foreach ($pattern in @($allowlist.forbidden_public_type_patterns) + @('(?ms)pub\s+use\s+[^;]*(?:HWND|HANDLE|IUnknown|IDesktop|IShell|COM)', '(?ms)pub\s+trait\s+.*?(?:HWND|HANDLE|IUnknown|IDesktop|IShell|COM)', '(?ms)pub\s+fn\s+.*?(?:HWND|HANDLE|IUnknown|IDesktop|IShell|COM)')) {
        if ($fixtureSource -match $pattern) {
            Add-Diagnostic $diagnostics 'UI_PUBLIC_WINDOWS_OR_COM_TYPE' 'UI fixture exports a Windows/COM type.'
        }
    }

    if ($diagnostics.Count -eq 0) {
        throw "Fixture did not trigger an architecture violation: $Fixture"
    }

    $diagnostics | ForEach-Object { Write-Output $_ }
    exit 1
}

$metadataJson = & cargo metadata --format-version 1 --no-deps --manifest-path (Join-Path $WorkspaceRoot 'Cargo.toml')
if ($LASTEXITCODE -ne 0) {
    throw 'cargo metadata failed.'
}

$metadata = $metadataJson | ConvertFrom-Json
$packagesByName = @{}
foreach ($package in $metadata.packages) {
    $packagesByName[$package.name] = $package
}

$actualNames = @($packagesByName.Keys | Sort-Object)
$expectedNames = @($allowlist.approved_crates | Sort-Object)
if (Compare-Object $expectedNames $actualNames) {
    Add-Diagnostic $diagnostics 'WORKSPACE_MEMBERS_MISMATCH' "Expected: $($expectedNames -join ', '); actual: $($actualNames -join ', ')."
}

foreach ($crateName in $expectedNames) {
    if (-not $packagesByName.ContainsKey($crateName)) {
        continue
    }

    $package = $packagesByName[$crateName]
    $allowed = @($allowlist.allowed_direct_dependencies.PSObject.Properties[$crateName].Value)
    foreach ($dependency in $package.dependencies) {
        $dependencyName = $dependency.rename
        if ([string]::IsNullOrWhiteSpace($dependencyName)) {
            $dependencyName = $dependency.name
        }

        if ($dependencyName -notin $allowed) {
            Add-Diagnostic $diagnostics 'DEPENDENCY_DIRECTION' "$crateName -> $dependencyName is not allowlisted."
        }
        if ($dependencyName -eq 'windows' -and $crateName -ne 'platform-win') { Add-Diagnostic $diagnostics 'WINDOWS_BINDING_OUTSIDE_PLATFORM' "$crateName directly depends on windows." }

        $forbiddenProperty = $allowlist.forbidden_dependency_substrings.PSObject.Properties[$crateName]
        if ($null -ne $forbiddenProperty) {
            foreach ($forbidden in @($forbiddenProperty.Value)) {
                if ($dependencyName -like "*$forbidden*") {
                    Add-Diagnostic $diagnostics 'CORE_FORBIDDEN_DEPENDENCY' "$crateName -> $dependencyName is forbidden."
                }
            }
        }
    }

    $sourceDirectory = Join-Path (Split-Path -Parent $package.manifest_path) 'src'
    $sourceFiles = @(Get-ChildItem -Path $sourceDirectory -Filter '*.rs' -File -Recurse -ErrorAction SilentlyContinue)
    if ($sourceFiles.Count -eq 0) {
        Add-Diagnostic $diagnostics 'MISSING_WINDOWS_GUARD' "$crateName has no Rust source file."
        continue
    }

    $hasGuard = $false
    foreach ($sourceFile in $sourceFiles) {
        $source = Get-Content -Raw -Encoding UTF8 $sourceFile.FullName
        if ($source -match '\#\[cfg\(not\(windows\)\)\]\s*\r?\ncompile_error!') {
            $hasGuard = $true
        }

        if (($crateName -in @('desktop-ui', 'taskbar-ui')) -and ($source -match '(?mi)^\s*pub\s+(?:use\s+[^;]*|trait\s+[^\r\n{;]*|fn\s+[^\r\n{;]*|type\s+[^\r\n{;]*|struct\s+[^\r\n{;]*|enum\s+[^\r\n{;]*)(?:HWND|HANDLE|IUnknown|IDesktop|IShell|COM)(?:\b|_)')) {
            Add-Diagnostic $diagnostics 'UI_PUBLIC_WINDOWS_OR_COM_TYPE' "$crateName exports a Windows/COM type."
        }
        if ($crateName -ne 'platform-win' -and $source -match '(?m)^\s*#!\[allow\(unsafe_code\)\]') { Add-Diagnostic $diagnostics 'UNSAFE_OVERRIDE_OUTSIDE_PLATFORM' $crateName }
        if ($crateName -eq 'platform-win' -and $source -match '(?m)^\s*unsafe\s*\{' -and $source -notmatch '(?m)^\s*//\s*SAFETY:') { Add-Diagnostic $diagnostics 'UNSAFE_WITHOUT_SAFETY_INVARIANT' $sourceFile.FullName }
    }

    if (-not $hasGuard -and $crateName -notin @($allowlist.windows_guard_exempt_crates)) {
        Add-Diagnostic $diagnostics 'MISSING_WINDOWS_GUARD' "$crateName lacks cfg(not(windows)) compile_error! refusal."
    }
}

if ($diagnostics.Count -gt 0) {
    $diagnostics | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Output 'Architecture check passed: 13 approved crates, allowlisted graph, declared platform-neutral exemptions, Windows guards, and UI type boundary.'
