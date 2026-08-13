[CmdletBinding()]
param([string]$WorkspaceRoot,[Parameter(Mandatory)][string]$Schema,[Parameter(Mandatory)][string]$Instance)
$ErrorActionPreference='Stop';if(-not $WorkspaceRoot){$WorkspaceRoot=Split-Path -Parent $PSScriptRoot};$exe=Join-Path $WorkspaceRoot 'target/debug/superdesktop-test-support.exe'
if(-not(Test-Path $exe)){& cargo build -p superdesktop-test-support --locked --offline;if($LASTEXITCODE){throw 'JSON_SCHEMA_ENGINE_BUILD_FAILED'}}
& $exe validate-json-schema (Join-Path $WorkspaceRoot $Schema) (Join-Path $WorkspaceRoot $Instance);if($LASTEXITCODE){throw 'JSON_SCHEMA_ENGINE_REJECTED'}
