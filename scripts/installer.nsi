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
!define MUI_ICON "..\assets\brand\icon.ico"
!define MUI_UNICON "..\assets\brand\icon.ico"

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
    ; Stop only this installation's bridge and desktop before replacing executables.
    InitPluginsDir
    SetOutPath "$PLUGINSDIR"
    File "${PACKAGE_DIR}\windows-stage\scripts\install-bridge-startup.ps1"
    nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "$PLUGINSDIR\install-bridge-startup.ps1" -BaseDir "$INSTDIR" -Stop -IncludeDesktop'
    Pop $0
    ${If} $0 != 0
        MessageBox MB_ICONEXCLAMATION "Could not stop running EqnTide processes. Close EqnTide and run Setup again."
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

    nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-bridge-startup.ps1"'
    Pop $0
    ${If} $0 != 0
        MessageBox MB_ICONEXCLAMATION "Background bridge setup failed. Run scripts\install-bridge-startup.ps1 to retry."
    ${EndIf}

    ; Share add-in installation policy with the portable package.
    nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-addin.ps1"'
    Pop $0
    ${If} $0 != 0
        MessageBox MB_ICONEXCLAMATION "EqnTide desktop was installed. Add-in setup failed; close Office and rerun scripts\install-addin.ps1."
    ${EndIf}

    WriteRegStr HKCU "Software\Classes\eqntide" "" "URL:EqnTide Protocol"
    WriteRegStr HKCU "Software\Classes\eqntide" "URL Protocol" ""
    WriteRegStr HKCU "Software\Classes\eqntide\shell\open\command" "" '$\"$INSTDIR\eqntide-desktop.exe$\" $\"%1$\"'

    ; 5. Shortcuts (recreate cleanly and flush Start Menu + Explorer icon cache)
    Delete "$SMPROGRAMS\EqnTide\*.lnk"
    RMDir /r "$SMPROGRAMS\EqnTide"
    Delete "$DESKTOP\EqnTide.lnk"
    System::Call 'shell32::SHChangeNotify(i 0x0001000, i 0x0005, w "$SMPROGRAMS", p 0)'

    CreateDirectory "$SMPROGRAMS\EqnTide"
    CreateShortcut "$SMPROGRAMS\EqnTide\EqnTide.lnk" "$INSTDIR\eqntide-desktop.exe" "" "$INSTDIR\icons\icon.ico" 0
    CreateShortcut "$SMPROGRAMS\EqnTide\Start Bridge Server.lnk" "$INSTDIR\start-server.bat" "" "$INSTDIR\icons\icon.ico" 0
    CreateShortcut "$SMPROGRAMS\EqnTide\Uninstall.lnk" "$INSTDIR\Uninstall.exe" "" "$INSTDIR\icons\icon.ico" 0
    CreateShortcut "$DESKTOP\EqnTide.lnk" "$INSTDIR\eqntide-desktop.exe" "" "$INSTDIR\eqntide-desktop.exe" 0
    System::Call 'shell32::SHChangeNotify(i 0x0001000, i 0x0005, w "$SMPROGRAMS\EqnTide", p 0)'
    System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, p 0, p 0)'
    IfFileExists "$SYSDIR\ie4uinit.exe" 0 +2
        ExecWait '"$SYSDIR\ie4uinit.exe" -show'

    ; 6. Uninstaller registration
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "DisplayName" "EqnTide - Equation Editor & Converter"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "UninstallString" '"$INSTDIR\Uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "DisplayIcon" "$INSTDIR\eqntide-desktop.exe,0"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "Publisher" "EqnTide"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "DisplayVersion" "${APP_VERSION}"
SectionEnd

Section "Uninstall"
    IfFileExists "$INSTDIR\scripts\install-bridge-startup.ps1" 0 bridge_uninstall_done
        nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-bridge-startup.ps1" -Uninstall'
        Pop $0
        ${If} $0 != 0
            MessageBox MB_ICONEXCLAMATION "Could not stop EqnTide processes. Close EqnTide and retry uninstall."
            Abort
        ${EndIf}
bridge_uninstall_done:
    Delete "$APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\EqnTide Bridge.vbs"

    IfFileExists "$INSTDIR\scripts\install-addin.ps1" 0 addin_uninstall_done
addin_uninstall_retry:
        nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-addin.ps1" -Uninstall'
        Pop $0
        ${If} $0 != 0
            MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Could not remove Word or WPS Office add-in files. Close Microsoft Word and WPS Office and retry uninstall." IDRETRY addin_uninstall_retry
            Abort
        ${EndIf}
addin_uninstall_done:

    ; Fallback cleanup for Word Template Add-in and registry
    Delete "$APPDATA\Microsoft\Word\STARTUP\EqnTide.dotm"
    DeleteRegValue HKCU "Software\Microsoft\Office\16.0\WEF\Developer" "EqnTide"

    ; Fallback cleanup for WPS Office Add-in
    RMDir /r "$APPDATA\Kingsoft\wps\jsaddons\EqnTide"
    RMDir /r "$APPDATA\Kingsoft\wps\jsaddons\EqnTide_1.0.0"
    RMDir "$APPDATA\Kingsoft\wps\jsaddons"
    RMDir "$APPDATA\Kingsoft\wps"
    RMDir "$APPDATA\Kingsoft"

    ; Remove runtime logs and lock/endpoint data
    RMDir /r "$LOCALAPPDATA\EqnTide"
    RMDir /r "$LOCALAPPDATA\eqntide\EqnTide"
    RMDir "$LOCALAPPDATA\eqntide"

    ; Remove shortcuts and refresh shell
    Delete "$DESKTOP\EqnTide.lnk"
    RMDir /r "$SMPROGRAMS\EqnTide"
    System::Call 'shell32::SHChangeNotify(i 0x0001000, i 0x0005, w "$SMPROGRAMS", p 0)'
    System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, p 0, p 0)'

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
