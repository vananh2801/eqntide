!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "EqnTide"
OutFile "..\release\EqnTide-Setup.exe"
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
    SetOutPath "$INSTDIR"
    File "..\dist\windows-stage\eqntide-desktop.exe"
    File "..\dist\windows-stage\eqntide-cli.exe"
    File "..\dist\windows-stage\start-server.bat"
    File "..\dist\windows-stage\README.md"
    File "..\dist\windows-stage\LICENSE"
    File "..\dist\windows-stage\NOTICE"

    SetOutPath "$INSTDIR\fonts"
    File /r "..\dist\windows-stage\fonts\*.*"
    SetOutPath "$INSTDIR\icons"
    File /r "..\dist\windows-stage\icons\*.*"

    SetOutPath "$INSTDIR\word-addin"
    File /r "..\dist\windows-stage\word-addin\*.*"

    SetOutPath "$INSTDIR\wps-addin"
    File /r "..\dist\windows-stage\wps-addin\*.*"

    SetOutPath "$INSTDIR\word-template"
    File "..\dist\windows-stage\word-template\EqnTide.dotm"

    ; 1. Auto-install Word Template to Word STARTUP folder
    CreateDirectory "$APPDATA\Microsoft\Word\STARTUP"
    CopyFiles /SILENT "$INSTDIR\word-template\EqnTide.dotm" "$APPDATA\Microsoft\Word\STARTUP\EqnTide.dotm"

    ; 2. Auto-install Word Web Add-in Developer Catalog registry
    WriteRegStr HKCU "Software\Microsoft\Office\16.0\WEF\Developer" "EqnTide" "$INSTDIR\word-addin\manifest.xml"

    ; 3. Auto-install WPS Office JS Add-in
    CreateDirectory "$APPDATA\Kingsoft\wps\jsaddons\EqnTide"
    CreateDirectory "$APPDATA\Kingsoft\wps\jsaddons\EqnTide_1.0.0"
    CopyFiles /SILENT "$INSTDIR\wps-addin\*.*" "$APPDATA\Kingsoft\wps\jsaddons\EqnTide"
    CopyFiles /SILENT "$INSTDIR\wps-addin\*.*" "$APPDATA\Kingsoft\wps\jsaddons\EqnTide_1.0.0"

    FileOpen $0 "$APPDATA\Kingsoft\wps\jsaddons\publish.xml" w
    FileWrite $0 "<?xml version=$\"1.0$\" encoding=$\"UTF-8$\"?>$\r$\n<jsplugins>$\r$\n  <jsplugin name=$\"EqnTide$\" type=$\"wps$\" version=$\"1.0.0$\" enable=$\"enable_dev$\" url=$\"EqnTide_1.0.0/$\" />$\r$\n  <jspluginonline name=$\"EqnTide$\" type=$\"wps$\" enable=$\"enable_dev$\" debug=$\"true$\" url=$\"http://127.0.0.1:17890/wps-addin/$\" />$\r$\n</jsplugins>"
    FileClose $0

    FileOpen $0 "$APPDATA\Kingsoft\wps\jsaddons\authaddin.json" w
    FileWrite $0 "{$\r$\n  $\"wps$\": {$\r$\n    $\"namelist$\": $\"EqnTide$\",$\r$\n    $\"EqnTide$\": {$\r$\n      $\"name$\": $\"EqnTide$\",$\r$\n      $\"enable$\": true,$\r$\n      $\"isload$\": true,$\r$\n      $\"path$\": $\"EqnTide_1.0.0/$\",$\r$\n      $\"md5$\": $\"$\",$\r$\n      $\"mode$\": 2$\r$\n    }$\r$\n  }$\r$\n}"
    FileClose $0

    ; 4. Windows loopback exemption for WebViews
    ExecWait 'CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.Win32WebViewHost_cw5n1h2txyewy"'
    ExecWait 'CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.WindowsStore_8wekyb3d8bbwe"'

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
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "DisplayVersion" "0.3.0"
SectionEnd

Section "Uninstall"
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

    ; Remove files
    RMDir /r "$INSTDIR\word-addin"
    RMDir /r "$INSTDIR\wps-addin"
    RMDir /r "$INSTDIR\word-template"
    Delete "$INSTDIR\eqntide-desktop.exe"
    Delete "$INSTDIR\eqntide-cli.exe"
    Delete "$INSTDIR\start-server.bat"
    Delete "$INSTDIR\README.md"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\NOTICE"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir "$INSTDIR"

    DeleteRegKey HKCU "Software\Classes\eqntide"

    ; Remove uninstaller registry
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide"
SectionEnd
