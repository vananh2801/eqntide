<# Per-user Windows bridge startup; never registers a machine-wide service. #>
param(
    [switch]$Uninstall,
    [switch]$Stop,
    [switch]$Launch,
    [string]$BaseDir = (Split-Path -Parent $PSScriptRoot)
)
$ErrorActionPreference = 'Stop'
$Exe = [IO.Path]::GetFullPath((Join-Path $BaseDir 'eqntide-cli.exe'))
$Desktop = Join-Path $BaseDir 'eqntide-desktop.exe'
$LaunchScript = Join-Path $BaseDir 'scripts\install-bridge-startup.ps1'
$Startup = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Startup\EqnTide Bridge.vbs'

function Get-OwnedBridge {
    # Match the full executable path and server mode, never unrelated CLI jobs.
    @(Get-CimInstance Win32_Process -Filter "Name = 'eqntide-cli.exe'" | Where-Object {
        $_.ExecutablePath -and [IO.Path]::GetFullPath($_.ExecutablePath) -eq $Exe -and
        $_.CommandLine -match '(?:^|\s)--server(?:\s|$)'
    })
}

if ($Uninstall -or $Stop) {
    if ($Uninstall) { Remove-Item -LiteralPath $Startup -Force -ErrorAction SilentlyContinue }
    # Stop our hidden supervisor first so it cannot restart the child.
    foreach ($Runner in @(Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe'" | Where-Object {
        $_.ProcessId -ne $PID -and $_.CommandLine -and
        $_.CommandLine.IndexOf($LaunchScript, [StringComparison]::OrdinalIgnoreCase) -ge 0 -and
        $_.CommandLine -match '(?:^|\s)-Launch(?:\s|$)'
    })) {
        Stop-Process -Id $Runner.ProcessId -Force -ErrorAction Stop
        Wait-Process -Id $Runner.ProcessId -Timeout 10 -ErrorAction SilentlyContinue
    }
    foreach ($Process in (Get-OwnedBridge)) {
        Stop-Process -Id $Process.ProcessId -Force -ErrorAction Stop
        Wait-Process -Id $Process.ProcessId -Timeout 10 -ErrorAction SilentlyContinue
    }
    exit 0
}

if (!(Test-Path -LiteralPath $Exe)) { throw "Bridge executable missing: $Exe" }
if (!$Launch) {
    $Command = 'powershell.exe -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "' + $PSCommandPath + '" -Launch'
    $Vbs = 'CreateObject("WScript.Shell").Run "' + $Command.Replace('"', '""') + '", 0, False'
    New-Item -ItemType Directory -Path (Split-Path -Parent $Startup) -Force | Out-Null
    Set-Content -LiteralPath $Startup -Value $Vbs -Encoding Unicode
}
if (!$Launch) {
    Start-Process -FilePath 'wscript.exe' -ArgumentList ('"' + $Startup + '"') | Out-Null
    for ($Attempt = 0; $Attempt -lt 30; $Attempt++) {
        Start-Sleep -Milliseconds 250
        try {
            $Health = Invoke-RestMethod 'http://127.0.0.1:17890/health' -TimeoutSec 1
            if ($Health.server -eq 'eqntide-bridge') { exit 0 }
        } catch {}
    }
    throw 'Background bridge did not become ready. Inspect %LOCALAPPDATA%\EqnTide\logs.'
}

# One supervisor per installation and login session. It also waits while an
# already-open desktop owns the port, then takes over after that desktop exits.
$Hash = [Security.Cryptography.SHA256]::Create()
$Key = [BitConverter]::ToString($Hash.ComputeHash([Text.Encoding]::UTF8.GetBytes($Exe.ToLowerInvariant()))).Replace('-', '')
$Mutex = New-Object Threading.Mutex($false, ('Local\EqnTideBridge-' + $Key))
$Owned = $false
try {
    try { $Owned = $Mutex.WaitOne(0) } catch [Threading.AbandonedMutexException] { $Owned = $true }
    if (!$Owned) { exit 0 }
    $env:EQNTIDE_DESKTOP_BIN = $Desktop
    $Logs = Join-Path $env:LOCALAPPDATA 'EqnTide\logs'
    New-Item -ItemType Directory -Path $Logs -Force | Out-Null
    while ($true) {
        try {
            $Running = @(Get-OwnedBridge)
            if ($Running.Count -gt 0) {
                Wait-Process -Id $Running[0].ProcessId -ErrorAction SilentlyContinue
            } else {
                $Occupied = $false
                try {
                    $Health = Invoke-RestMethod 'http://127.0.0.1:17890/health' -TimeoutSec 2
                    $Occupied = $Health.server -eq 'eqntide-bridge'
                } catch {}
                if (!$Occupied) {
                    $Child = Start-Process -FilePath $Exe -ArgumentList '--server' -WorkingDirectory $BaseDir -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $Logs 'bridge.out.log') -RedirectStandardError (Join-Path $Logs 'bridge.err.log')
                    Wait-Process -Id $Child.Id -ErrorAction SilentlyContinue
                }
            }
        } catch {
            Add-Content -LiteralPath (Join-Path $Logs 'startup.log') -Value $_
        }
        Start-Sleep -Seconds 3
    }
} finally {
    if ($Owned) { $Mutex.ReleaseMutex() }
    $Mutex.Dispose()
    $Hash.Dispose()
}
