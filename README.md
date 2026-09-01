# Voice Dictate

Fast, low-overhead push-to-talk voice dictation. Press a global hotkey, speak,
press again: NVIDIA Parakeet TDT transcribes the speech locally through
[NeMo-Speech.cpp](https://github.com/NVIDIA/NeMo-Speech.cpp), then the text is
copied and pasted into the focused window.

The dictation client is a native Rust binary. The local speech service runs as
a separate native process with no cloud transcription account.

## Status

- **Windows**: working (WASAPI capture, Win32 global hotkey, native tray, SendInput paste).
- **Linux**: working on Wayland (GNOME 50) and X11. Wayland uses the
  GlobalShortcuts portal for the hotkey and `ydotool` (kernel uinput) for
  auto-paste; X11 falls back to the `global-hotkey` backend and XTest. A tray
  microphone shows the current state and provides the app menu.

## How it works

| Concern        | Windows                  | Linux                                   |
| -------------- | ------------------------ | --------------------------------------- |
| Audio capture  | WASAPI (`cpal`)          | PipeWire / ALSA (`cpal`)                |
| Global hotkey  | Win32 `RegisterHotKey`   | GlobalShortcuts portal (Wayland) / `global-hotkey` (X11) |
| Paste          | `SendInput`              | `ydotool` uinput (Wayland) / XTest (X11) |
| Tray icon      | `tray-icon` (Win32)      | `tray-icon` (GTK/AppIndicator)             |
| Notifications  | WinRT toast              | D-Bus (`notify-rust`), errors only      |
| Transcription  | NVIDIA Parakeet TDT through local NeMo-Speech.cpp                 |

The icon (`assets/icon.svg`) is rasterised to PNG at build time by `build.rs`
(via `resvg`) and embedded; it never enters the runtime as an SVG.

## Configuration

Config lives at:

- Windows: `%APPDATA%\voice-dictate\config.toml`
- Linux: `~/.config/voice-dictate/config.toml`

Copy `config.example.toml` there and edit. `server_url` points to the local
NeMo-Speech.cpp service at `http://127.0.0.1:8080/v1`. The service loads
Parakeet TDT and exposes it as `model = "default"`. No API key is required.

## Install

Prebuilt client binaries for Windows and Linux are published on the
[Releases page](https://github.com/DaniilBaida/voice-dictate/releases/latest).
The platform installer also installs the local NeMo-Speech.cpp runtime.

## Install (Windows)

```powershell
git clone https://github.com/DaniilBaida/voice-dictate.git
cd voice-dictate
powershell -ExecutionPolicy Bypass -File install.ps1
```

`install.ps1` downloads the prebuilt binary from the latest release (or uses a
locally built one if present), copies it to
`%LOCALAPPDATA%\Programs\voice-dictate`, creates a Start Menu shortcut so it
shows up as an app, installs NeMo-Speech.cpp, starts Parakeet TDT locally, and
launches Voice Dictate. To remove Voice Dictate:

```powershell
powershell -ExecutionPolicy Bypass -File uninstall.ps1
```

The Parakeet model is stored in the local NeMo-Speech.cpp model cache. No cloud
transcription account or API key is required.

## Install (Linux)

Tested on Ubuntu (GNOME, Wayland and X11). The prebuilt binary links against
libraries that ship on a standard desktop (ALSA, D-Bus, XCB). Install the
runtime bits it needs that may be missing:

```sh
sudo apt install pulseaudio-utils ydotool libayatana-appindicator3-1
```

Then clone and run the installer (the binary itself needs no root; sudo is asked
only to set up the `ydotoold` input daemon):

```sh
git clone https://github.com/DaniilBaida/voice-dictate.git
cd voice-dictate
./install.sh
```

`install.sh` downloads the prebuilt binary from the latest release (or uses a
locally built one if present), copies it to `~/.local/bin`, installs a desktop
entry, installs NeMo-Speech.cpp, runs Parakeet TDT as a user service, sets up
the `ydotoold` input daemon, and launches Voice Dictate. The desktop entry is
required: on Wayland the GlobalShortcuts
portal identifies the app by an application id, and the host portal only accepts
an id that matches an installed `.desktop` file.

On the first run under Wayland, GNOME prompts once to set the global shortcut.
Auto-paste uses `ydotool`, which injects the keystroke at the kernel level via
`/dev/uinput`, so it needs no permission dialog.
To remove everything:

```sh
./uninstall.sh
```

## Build from source

Only needed if you want to compile yourself instead of using the prebuilt
binaries above. Requires a Rust toolchain and a C linker.

```sh
cargo build --release
```

The binary is `target/release/voice-dictate`.

### Linux system dependencies

```sh
sudo apt install build-essential pkg-config libasound2-dev libdbus-1-dev \
    libxcb1-dev libxcb-render0-dev libxcb-randr0-dev libssl-dev libgtk-3-dev \
    libayatana-appindicator3-dev pulseaudio-utils
```

## Usage

Two shortcuts, each a press to start and a press again to stop. A short beep
marks both ends.

- **Ctrl+Space** is raw mode: the transcript is pasted exactly as recognised.
- **Ctrl+Alt+Space** is prompt mode: the transcript is restructured into a
  prompt before pasting, through the model named by `cleanup_model`, which reads
  `ANTHROPIC_API_KEY` from the environment. Punctuation, technical terms and
  paragraph structure are corrected; wording, register, language and the
  speaker's purpose are left alone. Dictations longer than `cleanup_max_words`
  paste raw instead.

The tray microphone is white when idle, red while recording, and amber while
transcribing. Its menu switches prompt mode off, copies the last output or the
last raw transcript, lists recent dictations, dictates by mouse, and holds the
shortcut and paste settings. A row appears at the top of the menu only when the
speech server is unreachable or prompt mode has no API key.

Every dictation is appended to `~/.local/share/voice-dictate/history.jsonl`,
which keeps the transcript and the rewritten prompt separately, pruned to
`history_retention_days` at startup.

`install.sh` enables autostart at login on Linux.

## License

MIT
