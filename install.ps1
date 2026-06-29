# Installs Voice Dictate on Windows:
#   - uses a locally built binary if present, otherwise downloads the prebuilt
#     binary from the latest GitHub release (no Rust toolchain required)
#   - copies it to %LOCALAPPDATA%\Programs\voice-dictate
#   - creates a Start Menu shortcut so it shows up as an app
#   - launches it
#
# Usage:  powershell -ExecutionPolicy Bypass -File install.ps1

$ErrorActionPreference = "Stop"

$repo      = "DaniilBaida/voice-dictate"
$asset     = "voice-dictate-windows-x86_64.exe"
$root      = $PSScriptRoot
$binary    = Join-Path $root "target\release\voice-dictate.exe"
$installDir = Join-Path $env:LOCALAPPDATA "Programs\voice-dictate"
$exe       = Join-Path $installDir "voice-dictate.exe"
$startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$lnk       = Join-Path $startMenu "Voice Dictate.lnk"

# Prefer a locally built binary; otherwise download the prebuilt release asset.
if (Test-Path $binary) {
    $source = $binary
    Write-Host "Using locally built binary: $binary"
} else {
    $url      = "https://github.com/$repo/releases/latest/download/$asset"
    $source   = Join-Path $env:TEMP $asset
    Write-Host "Downloading prebuilt binary from latest release..."
    Invoke-WebRequest -Uri $url -OutFile $source -UseBasicParsing
}

# Stop any running instance so the file is not locked
Get-Process voice-dictate -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# Install binary
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item $source $exe -Force
Write-Host "Installed binary: $exe"

# Start Menu shortcut
$ws = New-Object -ComObject WScript.Shell
$s  = $ws.CreateShortcut($lnk)
$s.TargetPath        = $exe
$s.WorkingDirectory  = $installDir
$s.Description        = "Voice Dictate"
$s.Save()
Write-Host "Created Start Menu shortcut: $lnk"

# Launch
Start-Process $exe
Write-Host "Voice Dictate is running. Look for the microphone icon in the system tray."
