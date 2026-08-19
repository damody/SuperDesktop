@echo off
setlocal
chcp 65001 >nul

set "WORKSPACE=%~dp0."
set "RUNNER=%~dp0target\release\superdesktop-utit.exe"

if not exist "%RUNNER%" (
    cargo build --manifest-path "%WORKSPACE%Cargo.toml" -p superdesktop-utit --release --locked --offline
    if errorlevel 1 exit /b %errorlevel%
)

"%RUNNER%" %* --workspace "%WORKSPACE%"
exit /b %errorlevel%
