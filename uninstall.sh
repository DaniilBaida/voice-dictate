#!/usr/bin/env bash
# Linux uninstaller for voice-dictate. Removes the binary, desktop entry and
# autostart entry. Leaves your config and the saved portal token in place;
# remove ~/.config/voice-dictate yourself if you want a clean slate.
set -euo pipefail

APP_ID="com.daniil.VoiceDictate"
BIN_NAME="voice-dictate"
BIN_DIR="$HOME/.local/bin"
APPS_DIR="$HOME/.local/share/applications"
ASR_SERVICE="$HOME/.config/systemd/user/voice-dictate-asr.service"

pkill -x "$BIN_NAME" 2>/dev/null || true

rm -f "$BIN_DIR/$BIN_NAME"
rm -f "$APPS_DIR/$APP_ID.desktop"
rm -f "$HOME/.config/autostart/voice-dictate.desktop"
update-desktop-database "$APPS_DIR" 2>/dev/null || true

if [ -f "$ASR_SERVICE" ]; then
    systemctl --user disable --now voice-dictate-asr.service 2>/dev/null || true
    rm -f "$ASR_SERVICE"
    systemctl --user daemon-reload
fi

# Remove the ydotoold system service we installed (needs sudo). The ydotool
# package itself is left installed.
if [ -f /etc/systemd/system/ydotoold.service ]; then
    sudo systemctl disable --now ydotoold.service 2>/dev/null || true
    sudo rm -f /etc/systemd/system/ydotoold.service
    sudo systemctl daemon-reload 2>/dev/null || true
fi

echo "Removed voice-dictate. Config left at ~/.config/voice-dictate (delete manually if desired)."
