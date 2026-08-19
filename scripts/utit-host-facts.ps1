$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
$tools = @('cargo.exe','openspec.cmd','powershell.exe','certutil.exe','git.exe') | Where-Object {
    $null -ne (Get-Command $_ -ErrorAction SilentlyContinue)
} | ForEach-Object { $_ -replace '\.(exe|cmd)$','' }
$result = [ordered]@{
    schema = 'superdesktop-utit-host/v1'
    windows_build = [Environment]::OSVersion.Version.Build
    architecture = [Environment]::GetEnvironmentVariable('PROCESSOR_ARCHITECTURE')
    interactive = [Environment]::UserInteractive
    monitor_count = [Windows.Forms.Screen]::AllScreens.Count
    explorer_running = [bool](Get-Process explorer -ErrorAction SilentlyContinue)
    tools = @($tools | Sort-Object -Unique)
}
$result | ConvertTo-Json -Compress
