@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-addin.ps1" -Uninstall
exit /b %errorlevel%
