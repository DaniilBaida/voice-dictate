# Voice Dictate

Fast, low-overhead push-to-talk voice dictation. Press a global hotkey, speak,
press again: your speech is transcribed via the OpenAI audio API, copied to the
clipboard, and pasted into the focused window.

Written in Rust for a single self-contained binary with no runtime, no virtualenv,
and a small memory footprint (~22 MB resident on Windows).

## Status

- **Windows**: working (WASAPI capture, Win32 global hotkey, native tray, SendInput paste).
- **Linux**: working on Wayland (tested on GNOME 50) and X11. Wayland uses the
  GlobalShortcuts portal for the hotkey and `ydotool` (kernel uinput) for
  auto-paste; X11 falls back to the `global-hotkey` backend and XTest.

## How it works

| Concern        | Windows                  | Linux                                   |
| -------------- | ------------------------ | --------------------------------------- |
| Audio capture  | WASAPI (`cpal`)          | PipeWire / ALSA (`cpal`)                |
| Global hotkey  | Win32 `RegisterHotKey`   | GlobalShortcuts portal (Wayland) / `global-hotkey` (X11) |
| Paste          | `SendInput`              | `ydotool` uinput (Wayland) / XTest (X11) |
| Tray icon      | `tray-icon` (Win32)      | none (GNOME shows its own mic indicator while recording) |
| Notifications  | WinRT toast              | D-Bus (`notify-rust`), errors only      |
| Transcription  | OpenAI `gpt-4o-transcribe` (`async-openai`)                       |

The icon (`assets/icon.svg`) is rasterised to PNG at build time by `build.rs`
(via `resvg`) and embedded; it never enters the runtime as an SVG.

## Configuration

Config lives at:

- Windows: `%APPDATA%\voice-dictate\config.toml`
- Linux: `~/.config/voice-dictate/config.toml`

Copy `config.example.toml` there and edit. The OpenAI key is read from, in order:

1. `OPENAI_API_KEY` environment variable
2. `openai_api_key` in the config file
3. `~/.config/openai.key`

## Install

Prebuilt binaries for Windows and Linux are published on the
[Releases page](https://github.com/DaniilBaida/voice-dictate/releases/latest).
No Rust toolchain or compiler is needed to install.

## Install (Windows)

```powershell
git clone https://github.com/DaniilBaida/voice-dictate.git
cd voice-dictate
powershell -ExecutionPolicy Bypass -File install.ps1
```

`install.ps1` downloads the prebuilt binary from the latest release (or uses a
locally built one if present), copies it to
`%LOCALAPPDATA%\Programs\voice-dictate`, creates a Start Menu shortcut so it
shows up as an app, and launches it. To remove everything:

```powershell
powershell -ExecutionPolicy Bypass -File uninstall.ps1
```

Set your OpenAI key as a persistent user environment variable (read on every
launch):

```powershell
setx OPENAI_API_KEY "sk-..."
```

Or just download `voice-dictate-windows-x86_64.exe` from the Releases page and
run it directly, after setting `OPENAI_API_KEY`.

## Install (Linux)

Tested on Ubuntu (GNOME, Wayland and X11). The prebuilt binary links against
libraries that ship on a standard desktop (ALSA, D-Bus, XCB). Install the
runtime bits it needs that may be missing:

```sh
sudo apt install pulseaudio-utils ydotool
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
entry, sets up the `ydotoold` input daemon (system service, needs sudo), and
launches it. The desktop entry is required: on Wayland the GlobalShortcuts
portal identifies the app by an application id, and the host portal only accepts
an id that matches an installed `.desktop` file.

On the first run under Wayland, GNOME prompts once to set the global shortcut.
Auto-paste uses `ydotool`, which injects the keystroke at the kernel level via
`/dev/uinput`, so it needs no permission dialog and shows no on-screen indicator.
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
    libxcb1-dev libxcb-render0-dev libxcb-randr0-dev libssl-dev pulseaudio-utils
```

## Usage

Default hotkey is **Ctrl+Space**: press to start recording, press again to stop
and transcribe. A short beep marks start and stop.

On Windows a tray icon appears, with a menu offering "Start at login" and "Quit".

On Linux there is no tray icon: while recording, GNOME shows its own microphone
indicator in the top bar, and the app is otherwise driven entirely by the
hotkey. `install.sh` enables autostart at login. To quit, run
`pkill -x voice-dictate`.

## License

MIT
