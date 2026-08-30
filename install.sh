#!/usr/bin/env bash
# Linux installer for voice-dictate.
#
# Installs the release binary and a desktop entry, sets up the ydotool input
# daemon (for auto-paste), then launches the app.
#
# The desktop entry is REQUIRED on Wayland: the GlobalShortcuts portal identifies
# the app by its application id, and the host portal Registry only accepts an id
# that matches an installed .desktop file. The app id below must stay in sync
# with PORTAL_APP_ID in src/main.rs.
set -euo pipefail

APP_ID="com.daniil.VoiceDictate"
BIN_NAME="voice-dictate"
REPO="DaniilBaida/voice-dictate"
ASSET="voice-dictate-linux-x86_64"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
APPS_DIR="$HOME/.local/share/applications"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/voice-dictate"
NEMO_BIN="$HOME/.local/bin/nemo-speech"
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
ASR_SERVICE="voice-dictate-asr.service"

# Prefer a locally built binary; otherwise download the prebuilt release asset
# (no Rust toolchain required).
mkdir -p "$BIN_DIR"
if [ -x "$REPO_DIR/target/release/$BIN_NAME" ]; then
    echo "==> Installing locally built binary to $BIN_DIR"
    install -m 0755 "$REPO_DIR/target/release/$BIN_NAME" "$BIN_DIR/$BIN_NAME"
else
    echo "==> Downloading prebuilt binary from latest release"
    curl -fsSL "https://github.com/$REPO/releases/latest/download/$ASSET" -o "$BIN_DIR/$BIN_NAME"
    chmod 0755 "$BIN_DIR/$BIN_NAME"
fi

echo "==> Installing desktop entry ($APP_ID.desktop)"
mkdir -p "$APPS_DIR"
cat > "$APPS_DIR/$APP_ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Voice Dictate
Comment=Push-to-talk voice dictation
Exec=$BIN_DIR/$BIN_NAME
Icon=audio-input-microphone
Terminal=false
Categories=Utility;AudioVideo;
StartupNotify=false
EOF
update-desktop-database "$APPS_DIR" 2>/dev/null || true

echo "==> Ensuring config exists at $CONFIG_DIR/config.toml"
mkdir -p "$CONFIG_DIR"
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    cp "$REPO_DIR/config.example.toml" "$CONFIG_DIR/config.toml"
    echo "    created from config.example.toml"
else
    echo "    already present, left untouched"
fi

echo "==> Installing local NVIDIA Parakeet TDT transcription"
if [ ! -x "$NEMO_BIN" ]; then
    curl -fsSL https://github.com/NVIDIA/NeMo-Speech.cpp/raw/main/scripts/install.sh | sh
fi
mkdir -p "$SYSTEMD_USER_DIR"
cat > "$SYSTEMD_USER_DIR/$ASR_SERVICE" <<EOF
[Unit]
Description=Local speech recognition for Voice Dictate
After=graphical-session.target

[Service]
Type=simple
ExecStart=$NEMO_BIN serve --asr-model parakeet-tdt --host 127.0.0.1 --port 8080 --no-ui
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF
systemctl --user daemon-reload
systemctl --user enable "$ASR_SERVICE"
systemctl --user restart "$ASR_SERVICE"

# Auto-paste on Wayland uses ydotool, which injects keystrokes at the kernel
# level via /dev/uinput. This needs the ydotoold daemon running with a socket
# the user can reach. We run it as a root system service so it works immediately
# (no input-group relogin) and survives reboots. Needs sudo.
if command -v systemctl >/dev/null 2>&1; then
    echo "==> Setting up ydotool input daemon (needs sudo)"
    sudo apt-get install -y ydotool libayatana-appindicator3-1
    # The Debian package ships a user service that requires input-group relogin;
    # disable it in favor of our root system service.
    systemctl --user disable --now ydotool.service 2>/dev/null || true
    sudo tee /etc/systemd/system/ydotoold.service >/dev/null <<EOF
[Unit]
Description=ydotoold virtual input daemon (for voice-dictate)
After=systemd-udevd.service

[Service]
Type=simple
ExecStart=/usr/bin/ydotoold --socket-path=/run/ydotool.socket --socket-own=$(id -u):$(id -g) --socket-perm=0660
Restart=always
RestartSec=1

[Install]
WantedBy=multi-user.target
EOF
    sudo systemctl daemon-reload
    sudo systemctl enable --now ydotoold.service
fi

# Voice Dictate starts automatically at login.
echo "==> Enabling autostart"
AUTOSTART_DIR="$HOME/.config/autostart"
mkdir -p "$AUTOSTART_DIR"
cat > "$AUTOSTART_DIR/voice-dictate.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Voice Dictate
Exec=$BIN_DIR/$BIN_NAME
Terminal=false
X-GNOME-Autostart-enabled=true
EOF

# Stop any running instance so the new binary takes over.
pkill -x "$BIN_NAME" 2>/dev/null || true
sleep 1

echo "==> Launching"
setsid "$BIN_DIR/$BIN_NAME" >/dev/null 2>&1 < /dev/null &

cat <<EOF

Voice Dictate is installed and running. It also starts automatically at login. The tray
microphone stays visible and its menu controls dictation, shortcut selection,
and quitting.

Transcription runs locally with NVIDIA Parakeet TDT through NeMo-Speech.cpp.
No cloud transcription account or API key is used.

First run on Wayland prompts once to set the global shortcut (default proposes
Ctrl+Space). Auto-paste via ydotool needs no prompt.

To quit:    pkill -x voice-dictate
To disable autostart:  rm ~/.config/autostart/voice-dictate.desktop
EOF
