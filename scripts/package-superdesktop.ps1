param(
    [Parameter(Mandatory = $true)]
    [string]$SuperExplorerPath,
    [string]$OutputDirectory = ""
)

$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$superExplorer = (Resolve-Path -LiteralPath $SuperExplorerPath).Path
if (-not (Test-Path -LiteralPath $superExplorer -PathType Leaf)) {
    throw 'SuperExplorerPath must identify a regular file.'
}
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $workspace 'build/SuperDesktop-package'
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $output) {
    throw "Package output already exists: $output"
}

& cargo build --workspace --all-targets --release --locked --offline --manifest-path (Join-Path $workspace 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'Release product build failed.' }

$staging = Join-Path ([IO.Path]::GetTempPath()) ("superdesktop-package-{0}" -f [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $staging | Out-Null
try {
    $inputs = [ordered]@{
        'superdesktop-app.exe' = Join-Path $workspace 'target/release/superdesktop-app.exe'
        'superdesktop-guardian.exe' = Join-Path $workspace 'target/release/superdesktop-guardian.exe'
        'shell-installer.exe' = Join-Path $workspace 'target/release/shell-installer.exe'
        'shell-provider-host.exe' = Join-Path $workspace 'target/release/shell-provider-host.exe'
        'notification-area-host.exe' = Join-Path $workspace 'target/release/notification-area-host.exe'
        'system-status-host.exe' = Join-Path $workspace 'target/release/system-status-host.exe'
        'SuperExplorer.exe' = $superExplorer
    }
    $binaries = @()
    foreach ($entry in $inputs.GetEnumerator()) {
        if (-not (Test-Path -LiteralPath $entry.Value -PathType Leaf)) {
            throw "Required binary missing: $($entry.Value)"
        }
        $destination = Join-Path $staging $entry.Key
        Copy-Item -LiteralPath $entry.Value -Destination $destination
        $binaries += [ordered]@{
            name = $entry.Key
            bytes = (Get-Item -LiteralPath $destination).Length
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $destination).Hash.ToLowerInvariant()
        }
    }
    $manifest = [ordered]@{
        schema = 'superdesktop-package/v1'
        generated_at_utc = [DateTime]::UtcNow.ToString('o')
        binaries = $binaries
    }
    [IO.File]::WriteAllText(
        (Join-Path $staging 'package-manifest.json'),
        (($manifest | ConvertTo-Json -Depth 8) + "`n"),
        [Text.UTF8Encoding]::new($false)
    )
    $parent = Split-Path -Parent $output
    if (-not (Test-Path -LiteralPath $parent)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    Move-Item -LiteralPath $staging -Destination $output
    $staging = $null
    Write-Output $output
}
finally {
    if ($staging -and (Test-Path -LiteralPath $staging)) {
        Remove-Item -LiteralPath $staging -Recurse -Force
    }
}
