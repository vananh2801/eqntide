<#
.SYNOPSIS
  Installs the Word VBA template and WPS Office add-in for the Math Equation Editor & Converter on Windows.
#>

param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (Test-Path (Join-Path $ScriptDir "word-template")) {
    $BaseDir = $ScriptDir
} elseif (Test-Path (Join-Path (Split-Path -Parent $ScriptDir) "word-template")) {
    $BaseDir = Split-Path -Parent $ScriptDir
} else {
    $BaseDir = $ScriptDir
}

$WpsAddinDir = Join-Path $BaseDir "wps-addin"

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
        if ($config.DocumentElement.ChildNodes.Count -eq 0) {
            Remove-Item -LiteralPath $PublishXml -Force
        } else {
            $config.Save($PublishXml)
        }
    }
    $AuthJson = Join-Path $WpsBasePath 'authaddin.json'
    if (Test-Path $AuthJson) {
        $auth = Get-Content -Raw $AuthJson | ConvertFrom-Json
        if ($auth.wps) {
            $auth.wps.PSObject.Properties.Remove('EqnTide')
            $auth.wps.PSObject.Properties.Remove('MathEditor')
            $names = @(([string]$auth.wps.namelist -split '[,; ]+') | Where-Object { $_ -and $_ -notin @('EqnTide','MathEditor') })
            $remainingWps = @($auth.wps.PSObject.Properties | Where-Object { $_.Name -ne 'namelist' })
            if ($names.Count -eq 0 -and $remainingWps.Count -eq 0) {
                $auth.PSObject.Properties.Remove('wps')
            } else {
                $auth.wps | Add-Member -NotePropertyName namelist -NotePropertyValue ($names -join ',') -Force
            }
        }
        if (@($auth.PSObject.Properties).Count -eq 0) {
            Remove-Item -LiteralPath $AuthJson -Force
        } else {
            [IO.File]::WriteAllText($AuthJson, ($auth | ConvertTo-Json -Depth 20), [Text.UTF8Encoding]::new($false))
        }
    }
    foreach ($EmptyDir in @($WpsBasePath, (Join-Path $env:APPDATA "Kingsoft\wps"), (Join-Path $env:APPDATA "Kingsoft"))) {
        if ((Test-Path -LiteralPath $EmptyDir) -and @(Get-ChildItem -LiteralPath $EmptyDir -Force -ErrorAction SilentlyContinue).Count -eq 0) {
            Remove-Item -LiteralPath $EmptyDir -Force -ErrorAction SilentlyContinue
        } else {
            break
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

# Retire this application's old Word web catalog entry on install AND uninstall.
# Keep unrelated Office add-ins and shared WebView loopback settings untouched.
$RegistryPath = "HKCU:\Software\Microsoft\Office\16.0\WEF\Developer"
if (Test-Path $RegistryPath) {
    Remove-ItemProperty -Path $RegistryPath -Name 'EqnTide' -ErrorAction SilentlyContinue
}
$LegacyWebAssets = Join-Path $BaseDir 'word-addin'
if (Test-Path -LiteralPath $LegacyWebAssets) {
    Remove-Item -LiteralPath $LegacyWebAssets -Recurse -Force
}

# Word integration is exclusively the VBA global template in STARTUP.
Write-Host ""
$WordStartup = Join-Path $env:APPDATA 'Microsoft\Word\STARTUP'
$Template = Join-Path $WordStartup 'EqnTide.dotm'
if ($Uninstall) {
    if (Test-Path -LiteralPath $Template) {
        Remove-Item -LiteralPath $Template -Force -ErrorAction Stop
    }
} else {
    New-Item -ItemType Directory $WordStartup -Force | Out-Null
    Copy-Item (Join-Path $BaseDir 'word-template\EqnTide.dotm') $Template -Force -ErrorAction Stop
}

Write-Host "Done! Please restart Microsoft Word or WPS Office." -ForegroundColor Cyan
Write-Host "Make sure the desktop editor application is running (listening on port 17890)." -ForegroundColor Cyan
