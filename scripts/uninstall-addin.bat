@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-addin.ps1" -Uninstall
if errorlevel 1 exit /b %errorlevel%
reg delete "HKCU\Software\Classes\eqntide" /f >nul 2>&1
exit /b 0
