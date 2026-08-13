[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot),

    [Parameter()]
    [string]$Fixture
)

$ErrorActionPreference = 'Stop'

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

    foreach ($pattern in $allowlist.forbidden_public_type_patterns) {
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
    $sourceFiles = @(Get-ChildItem -Path $sourceDirectory -Filter '*.rs' -File -ErrorAction SilentlyContinue)
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

        if (($crateName -in @('desktop-ui', 'taskbar-ui')) -and ($source -match $allowlist.forbidden_public_type_patterns[0])) {
            Add-Diagnostic $diagnostics 'UI_PUBLIC_WINDOWS_OR_COM_TYPE' "$crateName exports a Windows/COM type."
        }
    }

    if (-not $hasGuard) {
        Add-Diagnostic $diagnostics 'MISSING_WINDOWS_GUARD' "$crateName lacks cfg(not(windows)) compile_error! refusal."
    }
}

if ($diagnostics.Count -gt 0) {
    $diagnostics | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Output 'Architecture check passed: nine approved crates, allowlisted graph, Windows guards, and UI type boundary.'
