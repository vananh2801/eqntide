!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "EqnTide"
OutFile "..\release\EqnTide-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\EqnTide"
RequestExecutionLevel user

; Interface Settings
!define MUI_ABORTWARNING
!define MUI_ICON "..\src-tauri\icons\icon.ico"
!define MUI_UNICON "..\src-tauri\icons\icon.ico"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\EqnTide.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch EqnTide now"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "MainSection" SEC01
    SetOutPath "$INSTDIR"
    File "..\dist-package\EqnTide-windows-x64\EqnTide.exe"
    File "..\dist-package\EqnTide-windows-x64\eqntide-cli.exe"
    File "..\dist-package\EqnTide-windows-x64\start-server.bat"
    File "..\dist-package\EqnTide-windows-x64\HUONG-DAN-SU-DUNG.txt"

    SetOutPath "$INSTDIR\word-addin"
    File /r "..\dist-package\EqnTide-windows-x64\word-addin\*.*"

    SetOutPath "$INSTDIR\wps-addin"
    File /r "..\dist-package\EqnTide-windows-x64\wps-addin\*.*"

    SetOutPath "$INSTDIR\word-template"
    File "..\dist-package\EqnTide-windows-x64\word-template\EqnTide.dotm"

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

    ; 5. Shortcuts
    CreateDirectory "$SMPROGRAMS\EqnTide"
    CreateShortcut "$SMPROGRAMS\EqnTide\EqnTide.lnk" "$INSTDIR\EqnTide.exe"
    CreateShortcut "$SMPROGRAMS\EqnTide\Start Bridge Server.lnk" "$INSTDIR\start-server.bat"
    CreateShortcut "$SMPROGRAMS\EqnTide\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
    CreateShortcut "$DESKTOP\EqnTide.lnk" "$INSTDIR\EqnTide.exe"

    ; 6. Uninstaller registration
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "DisplayName" "EqnTide - Equation Editor & Converter"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "UninstallString" '"$INSTDIR\Uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide" "DisplayIcon" "$INSTDIR\EqnTide.exe"
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

    ; Remove files
    RMDir /r "$INSTDIR\word-addin"
    RMDir /r "$INSTDIR\wps-addin"
    RMDir /r "$INSTDIR\word-template"
    Delete "$INSTDIR\EqnTide.exe"
    Delete "$INSTDIR\eqntide-cli.exe"
    Delete "$INSTDIR\start-server.bat"
    Delete "$INSTDIR\HUONG-DAN-SU-DUNG.txt"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir "$INSTDIR"

    ; Remove uninstaller registry
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EqnTide"
SectionEnd
