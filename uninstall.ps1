# Removes Voice Dictate from Windows:
#   - stops the running process
#   - removes the Start Menu shortcut
#   - removes the autostart entry (if "Start at login" was enabled)
#   - deletes the installed binary
#
# Usage:  powershell -ExecutionPolicy Bypass -File uninstall.ps1

$ErrorActionPreference = "SilentlyContinue"

$installDir = Join-Path $env:LOCALAPPDATA "Programs\voice-dictate"
$startMenu  = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$lnk        = Join-Path $startMenu "Voice Dictate.lnk"
$startupDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup"

Get-Process voice-dictate -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

Remove-Item $lnk -Force
Remove-Item (Join-Path $startupDir "voice-dictate.cmd") -Force
Remove-Item (Join-Path $startupDir "voice-dictate.vbs") -Force
Remove-Item $installDir -Recurse -Force

Write-Host "Voice Dictate uninstalled."
Write-Host "Config at %APPDATA%\voice-dictate was left in place; delete it manually if you want."
