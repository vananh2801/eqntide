<#
.SYNOPSIS
  Registers Word & WPS Office Add-ins for the Math Equation Editor & Converter on Windows.
#>

param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

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
Write-Host " EqnTide Add-in Installer (Windows)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. WPS Office JS Add-in Installation
$WpsBasePath = Join-Path $env:APPDATA "Kingsoft\wps\jsaddons"
$WpsTargetPath = Join-Path $WpsBasePath "EqnTide"
$WpsVersionPath = Join-Path $WpsBasePath "EqnTide_1.0.0"
$LegacyWpsTargetPath = Join-Path $WpsBasePath "MathEditor"
$LegacyWpsVersionPath = Join-Path $WpsBasePath "MathEditor_1.0.0"

if ($Uninstall) {
    foreach ($Path in @($WpsTargetPath, $WpsVersionPath, $LegacyWpsTargetPath, $LegacyWpsVersionPath)) {
        if (Test-Path $Path) {
            Write-Host "--> Removing WPS Office Add-in from $Path..." -ForegroundColor Yellow
            Remove-Item -Recurse -Force $Path
        }
    }
    $PublishXml = Join-Path $WpsBasePath 'publish.xml'
    if (Test-Path $PublishXml) {
        [xml]$config = Get-Content -Raw $PublishXml
        foreach ($Node in @($config.DocumentElement.ChildNodes)) {
            if ($Node.Attributes -and $Node.Attributes['name'] -and $Node.Attributes['name'].Value -in @('EqnTide','MathEditor')) { [void]$config.DocumentElement.RemoveChild($Node) }
        }
        $config.Save($PublishXml)
    }
    $AuthJson = Join-Path $WpsBasePath 'authaddin.json'
    if (Test-Path $AuthJson) {
        $auth = Get-Content -Raw $AuthJson | ConvertFrom-Json
        if ($auth.wps) {
            $auth.wps.PSObject.Properties.Remove('EqnTide')
            $auth.wps.PSObject.Properties.Remove('MathEditor')
            $names = @(([string]$auth.wps.namelist -split '[,; ]+') | Where-Object { $_ -and $_ -notin @('EqnTide','MathEditor') })
            $auth.wps | Add-Member -NotePropertyName namelist -NotePropertyValue ($names -join ',') -Force
            $auth | ConvertTo-Json -Depth 20 | Set-Content $AuthJson -Encoding UTF8
        }
    }
    Write-Host "[+] EqnTide and legacy MathEditor WPS add-ins removed." -ForegroundColor Green
} else {
    Write-Host "--> Installing WPS Office JS Add-in..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $LegacyWpsTargetPath, $LegacyWpsVersionPath -ErrorAction SilentlyContinue
    foreach ($Path in @($WpsTargetPath, $WpsVersionPath)) {
        New-Item -ItemType Directory -Path $Path -Force | Out-Null
    }
    Copy-Item -Path (Join-Path $WpsAddinDir "*") -Destination $WpsTargetPath -Recurse -Force
    Copy-Item -Path (Join-Path $WpsAddinDir "*") -Destination $WpsVersionPath -Recurse -Force

    $PublishXml = Join-Path $WpsBasePath "publish.xml"
    if (Test-Path $PublishXml) { [xml]$config = Get-Content -Raw $PublishXml } else { [xml]$config = '<jsplugins />' }
    foreach ($Node in @($config.DocumentElement.ChildNodes)) {
        if ($Node.Attributes -and $Node.Attributes['name'] -and $Node.Attributes['name'].Value -in @('EqnTide', 'MathEditor')) { [void]$config.DocumentElement.RemoveChild($Node) }
    }
    $entry = $config.CreateElement('jsplugin')
    foreach ($pair in (@{name='EqnTide';type='wps';version='1.0.0';enable='enable_dev';url='EqnTide_1.0.0/'}).GetEnumerator()) { $entry.SetAttribute($pair.Key, $pair.Value) }
    [void]$config.DocumentElement.AppendChild($entry)
    $online = $config.CreateElement('jspluginonline')
    foreach ($pair in (@{name='EqnTide';type='wps';enable='enable_dev';debug='true';url='http://127.0.0.1:17890/wps-addin/'}).GetEnumerator()) { $online.SetAttribute($pair.Key, $pair.Value) }
    [void]$config.DocumentElement.AppendChild($online)
    $config.Save($PublishXml)

    $AuthJson = Join-Path $WpsBasePath "authaddin.json"
    $auth = if (Test-Path $AuthJson) { Get-Content -Raw $AuthJson | ConvertFrom-Json } else { [pscustomobject]@{} }
    if (!$auth.wps) { $auth | Add-Member -NotePropertyName wps -NotePropertyValue ([pscustomobject]@{}) -Force }
    $names = @(([string]$auth.wps.namelist -split '[,; ]+') | Where-Object { $_ -and $_ -notin @('EqnTide','MathEditor') }) + @('EqnTide')
    $auth.wps | Add-Member -NotePropertyName namelist -NotePropertyValue ($names -join ',') -Force
    $auth.wps | Add-Member -NotePropertyName EqnTide -NotePropertyValue ([pscustomobject]@{name='EqnTide';enable=$true;isload=$true;path='EqnTide_1.0.0/';md5='';mode=2}) -Force
    $auth | ConvertTo-Json -Depth 20 | Set-Content $AuthJson -Encoding UTF8

    Write-Host "[+] WPS Add-in successfully installed to $WpsTargetPath" -ForegroundColor Green
}

# 2. Microsoft Word Developer WEF Registry Registration
$RegistryPath = "HKCU:\Software\Microsoft\Office\16.0\WEF\Developer"

if ($Uninstall) {
    if (Test-Path $RegistryPath) {
        Write-Host "--> Removing Word Add-in Developer Catalog..." -ForegroundColor Yellow
        Remove-ItemProperty -Path $RegistryPath -Name "EqnTide" -ErrorAction SilentlyContinue
        Write-Host "[+] Word Add-in registry entry removed." -ForegroundColor Green
    }
} else {
    Write-Host "--> Registering Microsoft Word Add-in Manifest in Developer Catalog..." -ForegroundColor Yellow
    if (!(Test-Path $RegistryPath)) {
        New-Item -Path $RegistryPath -Force | Out-Null
    }
    Set-ItemProperty -Path $RegistryPath -Name "EqnTide" -Value $ManifestPath -Type String
    Write-Host "[+] Word Add-in registered via Developer Catalog." -ForegroundColor Green

    # Enable loopback exemption for local webview (Office JS localhost / 127.0.0.1 bridge access)
    try {
        CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.Win32WebViewHost_cw5n1h2txyewy" 2>$null
        CheckNetIsolation.exe LoopbackExempt -a -n="Microsoft.WindowsStore_8wekyb3d8bbwe" 2>$null
    } catch {}
}

Write-Host ""
$WordStartup = Join-Path $env:APPDATA 'Microsoft\Word\STARTUP'
$Template = Join-Path $WordStartup 'EqnTide.dotm'
if ($Uninstall) {
    Remove-Item $Template -ErrorAction SilentlyContinue
} else {
    New-Item -ItemType Directory $WordStartup -Force | Out-Null
    Copy-Item (Join-Path $BaseDir 'word-template\EqnTide.dotm') $Template -Force -ErrorAction Stop
}

Write-Host "Done! Please restart Microsoft Word or WPS Office." -ForegroundColor Cyan
Write-Host "Make sure the desktop editor application is running (listening on port 17890)." -ForegroundColor Cyan
