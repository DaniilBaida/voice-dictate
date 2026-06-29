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
    echo "    created from config.example.toml - add your OpenAI key"
else
    echo "    already present, left untouched"
fi

# Auto-paste on Wayland uses ydotool, which injects keystrokes at the kernel
# level via /dev/uinput. This needs the ydotoold daemon running with a socket
# the user can reach. We run it as a root system service so it works immediately
# (no input-group relogin) and survives reboots. Needs sudo.
if command -v systemctl >/dev/null 2>&1; then
    echo "==> Setting up ydotool input daemon (needs sudo)"
    if ! command -v ydotoold >/dev/null 2>&1; then
        sudo apt-get install -y ydotool
    fi
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

# Start automatically at login (there is no tray menu to toggle this).
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

Installed and started; it also starts automatically at login. There is no tray
icon: recording is shown by GNOME's own microphone indicator (top bar), and the
app is driven entirely by the global hotkey.

First run on Wayland prompts once to set the global shortcut (default proposes
Ctrl+Space). Auto-paste via ydotool needs no prompt and no on-screen indicator.

To quit:    pkill -x voice-dictate
To disable autostart:  rm ~/.config/autostart/voice-dictate.desktop

Set your OpenAI key (one of):
  - export OPENAI_API_KEY=sk-...      (then relaunch)
  - openai_api_key = "sk-..."         in $CONFIG_DIR/config.toml
  - $HOME/.config/openai.key          (key on a single line)
EOF
