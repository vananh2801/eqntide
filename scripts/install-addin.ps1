<#
.SYNOPSIS
  Registers Word & WPS Office Add-ins for the Math Equation Editor & Converter on Windows.
#>

param(
    [switch]$Uninstall
)

$ErrorActionPreference = "SilentlyContinue"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (Test-Path (Join-Path $ScriptDir "word-addin")) {
    $BaseDir = $ScriptDir
} elseif (Test-Path (Join-Path (Split-Path -Parent $ScriptDir) "word-addin")) {
    $BaseDir = Split-Path -Parent $ScriptDir
} else {
    $BaseDir = $ScriptDir
}

$WordAddinDir = Join-Path $BaseDir "word-addin"
$WpsAddinDir = Join-Path $BaseDir "wps-addin"
$ManifestPath = Join-Path $WordAddinDir "manifest.xml"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Math Editor Add-in Installer (Windows)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. WPS Office JS Add-in Installation
$WpsBasePath = Join-Path $env:APPDATA "Kingsoft\wps\jsaddons"
$WpsTargetPath = Join-Path $WpsBasePath "MathEditor"

if ($Uninstall) {
    if (Test-Path $WpsTargetPath) {
        Write-Host "--> Removing WPS Office Add-in from $WpsTargetPath..." -ForegroundColor Yellow
        Remove-Item -Recurse -Force $WpsTargetPath
        Write-Host "[+] WPS Add-in uninstalled." -ForegroundColor Green
    }
} else {
    Write-Host "--> Installing WPS Office JS Add-in..." -ForegroundColor Yellow
    if (!(Test-Path $WpsTargetPath)) {
        New-Item -ItemType Directory -Path $WpsTargetPath -Force | Out-Null
    }
    Copy-Item -Path (Join-Path $WpsAddinDir "*") -Destination $WpsTargetPath -Recurse -Force
    $WpsVersionPath = Join-Path $WpsBasePath "MathEditor_1.0.0"
    Copy-Item -Path (Join-Path $WpsAddinDir "*") -Destination $WpsVersionPath -Recurse -Force

    $PublishXml = Join-Path $WpsBasePath "publish.xml"
    $xmlContent = "<?xml version=""1.0"" encoding=""UTF-8""?>`r`n<jsplugins>`r`n  <jsplugin name=""MathEditor"" type=""wps"" version=""1.0.0"" enable=""enable_dev"" url=""MathEditor_1.0.0/"" />`r`n  <jspluginonline name=""MathEditor"" type=""wps"" enable=""enable_dev"" debug=""true"" url=""http://127.0.0.1:17890/wps-addin/"" />`r`n</jsplugins>"
    Set-Content -Path $PublishXml -Value $xmlContent -Encoding UTF8

    $AuthJson = Join-Path $WpsBasePath "authaddin.json"
    $jsonContent = "{`r`n  ""wps"": {`r`n    ""namelist"": ""MathEditor"",`r`n    ""MathEditor"": {`r`n      ""name"": ""MathEditor"",`r`n      ""enable"": true,`r`n      ""isload"": true,`r`n      ""path"": ""MathEditor_1.0.0/"",`r`n      ""md5"": """",`r`n      ""mode"": 2`r`n    }`r`n  }`r`n}"
    Set-Content -Path $AuthJson -Value $jsonContent -Encoding UTF8

    Write-Host "[+] WPS Add-in successfully installed to $WpsTargetPath" -ForegroundColor Green
}

# 2. Microsoft Word Developer WEF Registry Registration
$RegistryPath = "HKCU:\Software\Microsoft\Office\16.0\WEF\Developer"

if ($Uninstall) {
    if (Test-Path $RegistryPath) {
        Write-Host "--> Removing Word Add-in Developer Catalog..." -ForegroundColor Yellow
        Remove-ItemProperty -Path $RegistryPath -Name "MathEditor" -ErrorAction SilentlyContinue
        Write-Host "[+] Word Add-in registry entry removed." -ForegroundColor Green
    }
} else {
    Write-Host "--> Registering Microsoft Word Add-in Manifest in Developer Catalog..." -ForegroundColor Yellow
    if (!(Test-Path $RegistryPath)) {
        New-Item -Path $RegistryPath -Force | Out-Null
    }
    Set-ItemProperty -Path $RegistryPath -Name "MathEditor" -Value $ManifestPath -Type String
    Write-Host "[+] Word Add-in registered via Developer Catalog." -ForegroundColor Green

    # Enable loopback exemption for local webview (Office JS localhost / 127.0.0.1 bridge access)
    try {
        CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.Win32WebViewHost_cw5n1h2txyewy" 2>$null
        CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.WindowsStore_8wekyb3d8bbwe" 2>$null
    } catch {}
}

Write-Host ""
Write-Host "Done! Please restart Microsoft Word or WPS Office." -ForegroundColor Cyan
Write-Host "Make sure the desktop editor application is running (listening on port 17890)." -ForegroundColor Cyan
