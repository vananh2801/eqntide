@echo off
setlocal
set "BASE_DIR=%~dp0.."
if exist "%~dp0word-template" set "BASE_DIR=%~dp0"
if exist "%BASE_DIR%\register-uri.ps1" (
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%BASE_DIR%\register-uri.ps1"
    if errorlevel 1 exit /b 1
)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-addin.ps1" %*
exit /b %errorlevel%
