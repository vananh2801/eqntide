!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "Math Equation Editor"
OutFile "MathEditor-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\MathEditor"
RequestExecutionLevel user

; Interface Settings
!define MUI_ABORTWARNING
!define MUI_ICON "src-tauri/icons/icon.ico"
!define MUI_UNICON "src-tauri/icons/icon.ico"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\omml-editor-desktop.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Math Equation Editor now"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "MainSection" SEC01
    SetOutPath "$INSTDIR"
    File "omml-converter-windows-x64\omml-editor-desktop.exe"
    File "omml-converter-windows-x64\omml-converter.exe"
    File "omml-converter-windows-x64\start-server.bat"
    File "omml-converter-windows-x64\HUONG-DAN-SU-DUNG.txt"

    SetOutPath "$INSTDIR\word-addin"
    File /r "omml-converter-windows-x64\word-addin\*.*"

    SetOutPath "$INSTDIR\wps-addin"
    File /r "omml-converter-windows-x64\wps-addin\*.*"

    SetOutPath "$INSTDIR\word-template"
    File "omml-converter-windows-x64\word-template\MathEditor.dotm"

    ; 1. Auto-install Word Template to Word STARTUP folder
    CreateDirectory "$APPDATA\Microsoft\Word\STARTUP"
    CopyFiles /SILENT "$INSTDIR\word-template\MathEditor.dotm" "$APPDATA\Microsoft\Word\STARTUP\MathEditor.dotm"

    ; 2. Auto-install Word Web Add-in Developer Catalog registry
    WriteRegStr HKCU "Software\Microsoft\Office\16.0\WEF\Developer" "MathEditor" "$INSTDIR\word-addin\manifest.xml"

    ; 3. Auto-install WPS Office JS Add-in
    CreateDirectory "$APPDATA\Kingsoft\wps\jsaddons\MathEditor"
    CreateDirectory "$APPDATA\Kingsoft\wps\jsaddons\MathEditor_1.0.0"
    CopyFiles /SILENT "$INSTDIR\wps-addin\*.*" "$APPDATA\Kingsoft\wps\jsaddons\MathEditor"
    CopyFiles /SILENT "$INSTDIR\wps-addin\*.*" "$APPDATA\Kingsoft\wps\jsaddons\MathEditor_1.0.0"

    FileOpen $0 "$APPDATA\Kingsoft\wps\jsaddons\publish.xml" w
    FileWrite $0 "<?xml version=$\"1.0$\" encoding=$\"UTF-8$\"?>$\r$\n<jsplugins>$\r$\n  <jsplugin name=$\"MathEditor$\" type=$\"wps$\" version=$\"1.0.0$\" enable=$\"enable_dev$\" url=$\"MathEditor_1.0.0/$\" />$\r$\n  <jspluginonline name=$\"MathEditor$\" type=$\"wps$\" enable=$\"enable_dev$\" debug=$\"true$\" url=$\"http://127.0.0.1:17890/wps-addin/$\" />$\r$\n</jsplugins>"
    FileClose $0

    FileOpen $0 "$APPDATA\Kingsoft\wps\jsaddons\authaddin.json" w
    FileWrite $0 "{$\r$\n  $\"wps$\": {$\r$\n    $\"namelist$\": $\"MathEditor$\",$\r$\n    $\"MathEditor$\": {$\r$\n      $\"name$\": $\"MathEditor$\",$\r$\n      $\"enable$\": true,$\r$\n      $\"isload$\": true,$\r$\n      $\"path$\": $\"MathEditor_1.0.0/$\",$\r$\n      $\"md5$\": $\"$\",$\r$\n      $\"mode$\": 2$\r$\n    }$\r$\n  }$\r$\n}"
    FileClose $0

    ; 4. Windows loopback exemption for WebViews
    ExecWait 'CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.Win32WebViewHost_cw5n1h2txyewy"'
    ExecWait 'CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.WindowsStore_8wekyb3d8bbwe"'

    ; 5. Shortcuts
    CreateDirectory "$SMPROGRAMS\Math Editor"
    CreateShortcut "$SMPROGRAMS\Math Editor\Math Equation Editor.lnk" "$INSTDIR\omml-editor-desktop.exe"
    CreateShortcut "$SMPROGRAMS\Math Editor\Start Bridge Server.lnk" "$INSTDIR\start-server.bat"
    CreateShortcut "$SMPROGRAMS\Math Editor\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
    CreateShortcut "$DESKTOP\Math Equation Editor.lnk" "$INSTDIR\omml-editor-desktop.exe"

    ; 6. Uninstaller registration
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\MathEditor" "DisplayName" "Math Equation Editor & Converter"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\MathEditor" "UninstallString" '"$INSTDIR\Uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\MathEditor" "DisplayIcon" "$INSTDIR\omml-editor-desktop.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\MathEditor" "Publisher" "MathEditor"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\MathEditor" "DisplayVersion" "0.3.0"
SectionEnd

Section "Uninstall"
    ; Remove Word Template Add-in
    Delete "$APPDATA\Microsoft\Word\STARTUP\MathEditor.dotm"

    ; Remove Word Web Add-in registry
    DeleteRegValue HKCU "Software\Microsoft\Office\16.0\WEF\Developer" "MathEditor"

    ; Remove WPS Office Add-in
    RMDir /r "$APPDATA\Kingsoft\wps\jsaddons\MathEditor"
    RMDir /r "$APPDATA\Kingsoft\wps\jsaddons\MathEditor_1.0.0"

    ; Remove shortcuts
    Delete "$DESKTOP\Math Equation Editor.lnk"
    RMDir /r "$SMPROGRAMS\Math Editor"

    ; Remove files
    RMDir /r "$INSTDIR\word-addin"
    RMDir /r "$INSTDIR\wps-addin"
    RMDir /r "$INSTDIR\word-template"
    Delete "$INSTDIR\omml-editor-desktop.exe"
    Delete "$INSTDIR\omml-converter.exe"
    Delete "$INSTDIR\start-server.bat"
    Delete "$INSTDIR\HUONG-DAN-SU-DUNG.txt"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir "$INSTDIR"

    ; Remove uninstaller registry
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\MathEditor"
SectionEnd
