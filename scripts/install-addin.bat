@echo off
setlocal enabledelayedexpansion
title EqnTide Add-in Installer (Windows)

echo ==========================================================
echo   EqnTide Add-in Installer (Windows)
echo ==========================================================
echo.

set "SCRIPT_DIR=%~dp0"
set "DOTM_SRC=%SCRIPT_DIR%word-template\EqnTide.dotm"
set "WORD_STARTUP=%APPDATA%\Microsoft\Word\STARTUP"

echo [1/4] Installing Word Template Add-in (EqnTide for Word 2010/2013/2016/2019/2021/365)...
if exist "%DOTM_SRC%" (
    if not exist "%WORD_STARTUP%" mkdir "%WORD_STARTUP%" >nul 2>&1
    copy /y "%DOTM_SRC%" "%WORD_STARTUP%\EqnTide.dotm" >nul 2>&1
    if %ERRORLEVEL% EQU 0 (
        echo [+] EqnTide.dotm successfully copied to:
        echo     %WORD_STARTUP%\EqnTide.dotm
    ) else (
        echo [!] Warning: Could not copy to Word STARTUP folder.
    )
) else (
    echo [!] EqnTide.dotm not found, skipping template install.
)

echo.
echo [2/4] Registering Word Web Add-in in Developer Catalog (for Office 2016+)...
set "MANIFEST_PATH=%SCRIPT_DIR%word-addin\manifest.xml"
if exist "%MANIFEST_PATH%" (
    reg add "HKCU\Software\Microsoft\Office\16.0\WEF\Developer" /v "EqnTide" /t REG_SZ /d "%MANIFEST_PATH%" /f >nul 2>&1
    if %ERRORLEVEL% EQU 0 (
        echo [+] Word Web Add-in registered in Developer Catalog.
    )
)

echo.
echo [3/4] Installing WPS Office Add-in (if WPS is used)...
set "WPS_DIR=%APPDATA%\Kingsoft\wps\jsaddons\EqnTide"
set "WPS_VER_DIR=%APPDATA%\Kingsoft\wps\jsaddons\EqnTide_1.0.0"
if exist "%SCRIPT_DIR%wps-addin" (
    if not exist "%APPDATA%\Kingsoft\wps\jsaddons" mkdir "%APPDATA%\Kingsoft\wps\jsaddons" >nul 2>&1
    if not exist "%WPS_DIR%" mkdir "%WPS_DIR%" >nul 2>&1
    if not exist "%WPS_VER_DIR%" mkdir "%WPS_VER_DIR%" >nul 2>&1
    xcopy /y /e /q "%SCRIPT_DIR%wps-addin\*" "%WPS_DIR%\" >nul 2>&1
    xcopy /y /e /q "%SCRIPT_DIR%wps-addin\*" "%WPS_VER_DIR%\" >nul 2>&1

    (
        echo ^<?xml version="1.0" encoding="UTF-8"?^>
        echo ^<jsplugins^>
        echo   ^<jsplugin name="EqnTide" type="wps" version="1.0.0" enable="enable_dev" url="EqnTide_1.0.0/" /^>
        echo   ^<jspluginonline name="EqnTide" type="wps" enable="enable_dev" debug="true" url="http://127.0.0.1:17890/wps-addin/" /^>
        echo ^</jsplugins^>
    ) > "%APPDATA%\Kingsoft\wps\jsaddons\publish.xml"

    (
        echo {
        echo   "wps": {
        echo     "namelist": "EqnTide",
        echo     "EqnTide": {
        echo       "name": "EqnTide",
        echo       "enable": true,
        echo       "isload": true,
        echo       "path": "EqnTide_1.0.0/",
        echo       "md5": "",
        echo       "mode": 2
        echo     }
        echo   }
        echo }
    ) > "%APPDATA%\Kingsoft\wps\jsaddons\authaddin.json"

    echo [+] WPS Office Add-in installed successfully.
)

echo.
echo [4/4] Configuring Windows Loopback Exemption (for Windows 10/11 WebView)...
CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.Win32WebViewHost_cw5n1h2txyewy" >nul 2>&1
CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.WindowsStore_8wekyb3d8bbwe" >nul 2>&1
echo [+] Loopback configuration finished.

echo.
echo ==========================================================
echo  INSTALLATION COMPLETED SUCCESSFULLY!
echo ==========================================================
echo  1. Restart Microsoft Word or WPS Office.
echo  2. Open EqnTide.exe (or start-server.bat).
echo  3. Press Alt + \ or use ribbon buttons to Toggle LaTeX!
echo ==========================================================

:END
echo.
pause
