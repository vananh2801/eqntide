<#
.SYNOPSIS
  Registers Word & WPS Office Add-ins for the Math Equation Editor & Converter on Windows.
.PARAMETER Uninstall
  Uninstalls the add-in registrations.
#>

param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoDir = Split-Path -Parent $ScriptDir

$WordAddinDir = Join-Path $RepoDir "word-addin"
$WpsAddinDir = Join-Path $RepoDir "wps-addin"
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
        Write-Host "✓ WPS Add-in uninstalled." -ForegroundColor Green
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
    @'
<?xml version="1.0" encoding="UTF-8"?>
<jsplugins>
  <jsplugin name="MathEditor" type="wps" version="1.0.0" enable="enable_dev" url="MathEditor_1.0.0/" />
  <jspluginonline name="MathEditor" type="wps" enable="enable_dev" debug="true" url="http://127.0.0.1:17890/wps-addin/" />
</jsplugins>
'@ | Set-Content -Path $PublishXml -Encoding UTF8

    $AuthJson = Join-Path $WpsBasePath "authaddin.json"
    @'
{
  "wps": {
    "namelist": "MathEditor",
    "MathEditor": {
      "name": "MathEditor",
      "enable": true,
      "isload": true,
      "path": "MathEditor_1.0.0/",
      "md5": "",
      "mode": 2
    }
  }
}
'@ | Set-Content -Path $AuthJson -Encoding UTF8

    Write-Host "✓ WPS Add-in successfully installed to $WpsTargetPath" -ForegroundColor Green
}

# 2. Microsoft Word Developer WEF Registry Registration
$RegistryPath = "HKCU:\Software\Microsoft\Office\16.0\WEF\Developer"

if ($Uninstall) {
    if (Test-Path $RegistryPath) {
        Write-Host "--> Removing Word Add-in Developer Catalog..." -ForegroundColor Yellow
        Remove-ItemProperty -Path $RegistryPath -Name "MathEditor" -ErrorAction SilentlyContinue
        Write-Host "✓ Word Add-in registry entry removed." -ForegroundColor Green
    }
} else {
    Write-Host "--> Registering Microsoft Word Add-in Manifest in Developer Catalog..." -ForegroundColor Yellow
    if (!(Test-Path $RegistryPath)) {
        New-Item -Path $RegistryPath -Force | Out-Null
    }
    Set-ItemProperty -Path $RegistryPath -Name "MathEditor" -Value $ManifestPath -Type String
    Write-Host "✓ Word Add-in registered via Developer Catalog." -ForegroundColor Green
}

Write-Host ""
Write-Host "Done! Please restart Microsoft Word or WPS Office." -ForegroundColor Cyan
Write-Host "Make sure the desktop editor application is running (listening on port 17890)." -ForegroundColor Cyan
