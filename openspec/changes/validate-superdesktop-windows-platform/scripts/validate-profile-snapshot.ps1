Set-StrictMode -Version Latest

function Get-ProfileSnapshot {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$AllowlistPath)
    $policy = Get-Content -Raw -Encoding utf8 -LiteralPath $AllowlistPath | ConvertFrom-Json
    $snapshot = [ordered]@{ schema_version=$policy.schema_version; allowlist_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $AllowlistPath).Hash; keys=[ordered]@{} }
    foreach ($entry in $policy.keys.psobject.Properties) {
        $actual = [ordered]@{}
        if (Test-Path -LiteralPath $entry.Name) { $item=Get-Item -LiteralPath $entry.Name; $properties=Get-ItemProperty -LiteralPath $entry.Name; foreach($name in $item.Property){$actual[$name]=[string]$properties.$name} } else { throw "PROFILE_KEY_ABSENT: $($entry.Name)" }
        $null = $snapshot.keys[$entry.Name]=[ordered]@{values=$actual}
    }
    return $snapshot
}

function Assert-ProfileSnapshot {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$AllowlistPath, [Parameter(Mandatory)]$Snapshot)
    $policy = Get-Content -Raw -Encoding utf8 -LiteralPath $AllowlistPath | ConvertFrom-Json
    foreach ($entry in $policy.keys.psobject.Properties) {
        $key=$entry.Name; $rule=$entry.Value; $values=$Snapshot.keys.$key.values
        if ($null -eq $values) { throw "PROFILE_KEY_ABSENT: $key" }
        $expectedProperties=@($rule.expected.psobject.Properties)
        $expectedNames=@($expectedProperties | ForEach-Object { $_.Name })
        $isDictionary=$values -is [Collections.IDictionary]
        $actualNames=if($isDictionary){@($values.Keys)}else{@($values.psobject.Properties | ForEach-Object { $_.Name })}
        foreach($expected in $expectedProperties){ $actualValue=if($isDictionary){$values[$expected.Name]}else{$property=$values.psobject.Properties[$expected.Name];if($null -eq $property){$null}else{$property.Value}}; if($null -eq $actualValue -or [string]$actualValue -ne [string]$expected.Value){throw "PROFILE_VALUE_DRIFT: $key::$($expected.Name)"} }
        if ($rule.reject_unknown_values) { $unknown=@($actualNames | Where-Object { $_ -notin $expectedNames }); if($unknown.Count){throw "PROFILE_UNKNOWN_VALUE: $key::$($unknown -join ',')"} }
        if ($null -ne $rule.PSObject.Properties['important_name_pattern'] -and $rule.important_name_pattern) { $unknownImportant=@($actualNames | Where-Object { $_ -match $rule.important_name_pattern -and $_ -notin $expectedNames }); if($unknownImportant.Count){throw "PROFILE_UNKNOWN_IMPORTANT_VALUE: $key::$($unknownImportant -join ',')"} }
    }
}

function Test-ProfileSnapshotDiagnostic {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$AllowlistPath, [Parameter(Mandatory)]$Snapshot, [Parameter(Mandatory)][string]$ExpectedDiagnostic)
    try { Assert-ProfileSnapshot -AllowlistPath $AllowlistPath -Snapshot $Snapshot; return $false }
    catch { return $_.Exception.Message -match "^$([regex]::Escape($ExpectedDiagnostic))" }
}
