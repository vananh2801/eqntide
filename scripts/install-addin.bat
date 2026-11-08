@echo off
setlocal

echo ==========================================================
echo  Math Editor Add-in Installer (Windows)
echo ==========================================================

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-addin.ps1" %*

echo.
pause
