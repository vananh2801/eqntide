@echo off
setlocal
title Math Editor Add-in Uninstaller (Windows)

echo ==========================================================
echo  Math Editor Add-in Uninstaller (Windows)
echo ==========================================================
echo.

echo [1/3] Removing Word Template Add-in from STARTUP...
set "WORD_STARTUP=%APPDATA%\Microsoft\Word\STARTUP"
if exist "%WORD_STARTUP%\MathEditor.dotm" (
    del /f /q "%WORD_STARTUP%\MathEditor.dotm" >nul 2>&1
    echo [+] MathEditor.dotm removed from STARTUP.
)

echo.
echo [2/3] Removing Word Web Add-in registry entry...
reg delete "HKCU\Software\Microsoft\Office\16.0\WEF\Developer" /v "MathEditor" /f >nul 2>&1
echo [+] Word Web Add-in registry entry removed.

echo.
echo [3/3] Removing WPS Office Add-in files...
set "WPS_DIR=%APPDATA%\Kingsoft\wps\jsaddons\MathEditor"
set "WPS_VER_DIR=%APPDATA%\Kingsoft\wps\jsaddons\MathEditor_1.0.0"
if exist "%WPS_DIR%" rd /s /q "%WPS_DIR%" >nul 2>&1
if exist "%WPS_VER_DIR%" rd /s /q "%WPS_VER_DIR%" >nul 2>&1
echo [+] WPS Add-in files removed.

echo.
echo ==========================================================
echo  UNINSTALL COMPLETED!
echo ==========================================================
echo.
pause
