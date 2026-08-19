$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$violations = [Collections.Generic.List[string]]::new()
$sources = Get-ChildItem (Join-Path $workspace 'crates') -Recurse -Filter '*.rs' | Where-Object {
    $_.FullName -notmatch '[\\/]tests[\\/]' -and
    $_.FullName -notmatch '[\\/]examples[\\/]' -and
    $_.Name -ne 'build.rs' -and
    $_.FullName -notmatch '[\\/]superdesktop-test-support[\\/]'
}
foreach ($source in $sources) {
    $text = Get-Content -LiteralPath $source.FullName -Raw
    $production = ($text -split '#\[cfg\(test\)\]', 2)[0]
    $lines = $production -split "`r?`n"
    for ($index = 0; $index -lt $lines.Length; $index++) {
        if ($lines[$index] -match '\.(unwrap\(\)|expect\()') {
            $relative = [IO.Path]::GetRelativePath($workspace, $source.FullName)
            $violations.Add("${relative}:$($index + 1):$($lines[$index].Trim())")
        }
    }
}
if ($violations.Count -ne 0) {
    $violations | ForEach-Object { [Console]::Error.WriteLine("production panic primitive: $_") }
    exit 1
}
Write-Output 'production unwrap/expect scan passed'
