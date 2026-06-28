# Installs Voice Dictate on Windows:
#   - builds the release binary (if cargo is available and no binary exists)
#   - copies it to %LOCALAPPDATA%\Programs\voice-dictate
#   - creates a Start Menu shortcut so it shows up as an app
#   - launches it
#
# Usage:  powershell -ExecutionPolicy Bypass -File install.ps1

$ErrorActionPreference = "Stop"

$root      = $PSScriptRoot
$binary    = Join-Path $root "target\release\voice-dictate.exe"
$installDir = Join-Path $env:LOCALAPPDATA "Programs\voice-dictate"
$exe       = Join-Path $installDir "voice-dictate.exe"
$startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$lnk       = Join-Path $startMenu "Voice Dictate.lnk"

# Build if needed
if (-not (Test-Path $binary)) {
    Write-Host "No release binary found, building..."
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo not found on PATH and no prebuilt binary at $binary"
    }
    Push-Location $root
    cargo build --release
    Pop-Location
}

# Stop any running instance so the file is not locked
Get-Process voice-dictate -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# Install binary
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item $binary $exe -Force
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
