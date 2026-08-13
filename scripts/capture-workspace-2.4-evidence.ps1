[CmdletBinding()]
param([string]$WorkspaceRoot=(Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference='Stop';$a=Join-Path $WorkspaceRoot 'openspec/changes/bootstrap-superdesktop-workspace/evidence/artifacts/2.4'
function Run($name,$action,$expected=0){$old=$ErrorActionPreference;try{$ErrorActionPreference='Continue';$out=& $action 2>&1;$code=$LASTEXITCODE;if($null -eq $code){$code=0}}catch{$out=$_|Out-String;$code=1}finally{$ErrorActionPreference=$old};Set-Content -Encoding UTF8 (Join-Path $a $name) (@("exit_status: $code",'output:')+@($out));if($code -ne $expected){throw "$name expected $expected got $code"}}
Push-Location $WorkspaceRoot;try{
 Run 'evidence-validator-positive.txt' { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/validate-evidence.ps1" }
 Run 'contract-verifier-positive.txt' { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/verify-contract-manifest.ps1" -WorkspaceRoot $WorkspaceRoot -Manifest "$a/wave1-corrective-contract-inputs.sha256" }
 Run 'architecture-positive.txt' { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/check-dependency-architecture.ps1" -WorkspaceRoot $WorkspaceRoot }
 foreach($id in @('missing-procedure','wrong-type','wrong-pattern','dangling-adjustment','malformed-adjustment')){Run "fixture-$id.txt" { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/validate-evidence.ps1" -Fixture "fixtures/evidence-validator/$id" } 1}
 Run 'architecture-nested-pub-use-negative.txt' { powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/check-dependency-architecture.ps1" -WorkspaceRoot $WorkspaceRoot -Fixture 'fixtures/dependency-architecture/ui-public-hwnd-nested' } 1
 Run 'strict-openspec-validation.txt' { openspec validate bootstrap-superdesktop-workspace --strict }
 Run 'diff-check.txt' { git diff --check }
}finally{Pop-Location}
