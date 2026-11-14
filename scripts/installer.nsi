!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!ifndef PACKAGE_DIR
    !error "Pass /DPACKAGE_DIR=<absolute dist/windows/version directory> after packaging Windows."
!endif
!include "${PACKAGE_DIR}\installer-config.nsh"

Name "EqnTide"
OutFile "${PACKAGE_DIR}\EqnTide-${APP_VERSION}-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\EqnTide"
RequestExecutionLevel user

; Interface Settings
!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\icons\icon.ico"
!define MUI_UNICON "..\assets\icons\icon.ico"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\eqntide-desktop.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch EqnTide now"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "MainSection" SEC01
    ; Stop only this installation's bridge before replacing its executable.
    InitPluginsDir
    SetOutPath "$PLUGINSDIR"
    File "${PACKAGE_DIR}\windows-stage\scripts\install-bridge-startup.ps1"
    ExecWait 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\install-bridge-startup.ps1" -BaseDir "$INSTDIR" -Stop' $0
    ${If} $0 != 0
        MessageBox MB_ICONEXCLAMATION "Could not stop the old bridge. Close EqnTide bridge processes and run Setup again."
        Abort
    ${EndIf}
    SetOutPath "$INSTDIR"
    ; Retire the legacy shell only after the user has closed it.
legacy_retry:
    IfFileExists "$INSTDIR\EqnTide.exe" 0 legacy_done
    ClearErrors
    Delete "$INSTDIR\EqnTide.exe"
    IfErrors 0 legacy_done
    MessageBox MB_RETRYCANCEL "Close the old EqnTide application before installing the Rust desktop." IDRETRY legacy_retry
    Abort
legacy_done:
    File "${PACKAGE_DIR}\windows-stage\eqntide-desktop.exe"
    File "${PACKAGE_DIR}\windows-stage\eqntide-cli.exe"
    File "${PACKAGE_DIR}\windows-stage\start-server.bat"
    File "${PACKAGE_DIR}\windows-stage\install-addins.bat"
    File "${PACKAGE_DIR}\windows-stage\register-uri.ps1"
    File "${PACKAGE_DIR}\windows-stage\README.md"
    File "${PACKAGE_DIR}\windows-stage\LICENSE"
    File "${PACKAGE_DIR}\windows-stage\NOTICE"

    SetOutPath "$INSTDIR\scripts"
    File /r "${PACKAGE_DIR}\windows-stage\scripts\*.*"

    SetOutPath "$INSTDIR\fonts"
    File /r "${PACKAGE_DIR}\windows-stage\fonts\*.*"
    SetOutPath "$INSTDIR\icons"
    File /r "${PACKAGE_DIR}\windows-stage\icons\*.*"

    ; Clean assets shipped by older Office.js-based releases.
    RMDir /r "$INSTDIR\word-addin"

    SetOutPath "$INSTDIR\wps-addin"
    File /r "${PACKAGE_DIR}\windows-stage\wps-addin\*.*"

    SetOutPath "$INSTDIR\word-template"
    File "${PACKAGE_DIR}\windows-stage\word-template\EqnTide.dotm"

    ExecWait 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-bridge-startup.ps1"' $0
    ${If} $0 != 0
        MessageBox MB_ICONEXCLAMATION "Background bridge setup failed. Run scripts\install-bridge-startup.ps1 to retry."
    ${EndIf}

    ; Share add-in installation policy with the portable package.
    ExecWait 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-addin.ps1"' $0
    ${If} $0 != 0
        MessageBox MB_ICONEXCLAMATION "EqnTide desktop was installed. Add-in setup failed; close Office and rerun scripts\install-addin.ps1."
    ${EndIf}

    WriteRegStr HKCU "Software\Classes\eqntide" "" "URL:EqnTide Protocol"
    WriteRegStr HKCU "Software\Classes\eqntide" "URL Protocol" ""
    WriteRegStr HKCU "Software\Classes\eqntide\shell\open\command" "" '$\"$INSTDIR\eqntide-desktop.exe$\" $\"%1$\"'

    ; 5. Shortcuts
    CreateDirectory "$SMPROGRAMS\EqnTide"
    CreateShortcut "$SMPROGRAMS\EqnTide\EqnTide.lnk" "$INSTDIR\eqntide-desktop.exe"
    CreateShortcut "$SMPROGRAMS\EqnTide\Start Bridge Server.lnk" "$INSTDIR\start-server.bat"
    CreateShortcut "$SMPROGRAMS\EqnTide\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
    CreateShortcut "$DESKTOP\EqnTide.lnk" "$INSTDIR\eqntide-desktop.exe"

    ; 6. Uninstaller registration
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "DisplayName" "EqnTide - Equation Editor & Converter"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "UninstallString" '"$INSTDIR\Uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "DisplayIcon" "$INSTDIR\eqntide-desktop.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "Publisher" "EqnTide"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "DisplayVersion" "${APP_VERSION}"
SectionEnd

Section "Uninstall"
    ExecWait 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-bridge-startup.ps1" -Uninstall' $0
    ${If} $0 != 0
        MessageBox MB_ICONEXCLAMATION "Could not stop the bridge. Close EqnTide bridge processes and retry uninstall."
        Abort
    ${EndIf}
    ExecWait 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-addin.ps1" -Uninstall' $0
    ; Remove Word Template Add-in
    Delete "$APPDATA\Microsoft\Word\STARTUP\EqnTide.dotm"

    ; Remove Word Web Add-in registry
    DeleteRegValue HKCU "Software\Microsoft\Office\16.0\WEF\Developer" "EqnTide"

    ; Remove WPS Office Add-in
    RMDir /r "$APPDATA\Kingsoft\wps\jsaddons\EqnTide"
    RMDir /r "$APPDATA\Kingsoft\wps\jsaddons\EqnTide_1.0.0"

    ; Remove shortcuts
    Delete "$DESKTOP\EqnTide.lnk"
    RMDir /r "$SMPROGRAMS\EqnTide"

    RMDir /r "$INSTDIR\fonts"
    RMDir /r "$INSTDIR\icons"

    RMDir /r "$INSTDIR\scripts"

    ; Remove files
    RMDir /r "$INSTDIR\word-addin"
    RMDir /r "$INSTDIR\wps-addin"
    RMDir /r "$INSTDIR\word-template"
    Delete "$INSTDIR\eqntide-desktop.exe"
    Delete "$INSTDIR\eqntide-cli.exe"
    Delete "$INSTDIR\start-server.bat"
    Delete "$INSTDIR\install-addins.bat"
    Delete "$INSTDIR\register-uri.ps1"
    Delete "$INSTDIR\README.md"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\NOTICE"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir "$INSTDIR"

    DeleteRegKey HKCU "Software\Classes\eqntide"

    ; Remove uninstaller registry
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide"
SectionEnd
