# Voice Dictate

Fast, low-overhead push-to-talk voice dictation. Press a global hotkey, speak,
press again: your speech is transcribed via the OpenAI audio API, copied to the
clipboard, and pasted into the focused window.

Written in Rust for a single self-contained binary with no runtime, no virtualenv,
and a small memory footprint (~22 MB resident on Windows).

## Status

- **Windows**: working (WASAPI capture, Win32 global hotkey, native tray, SendInput paste).
- **Linux**: code paths exist (PipeWire/ALSA capture, ksni tray, X11/Wayland paste)
  but are **not yet built or tested**. Wayland global hotkeys still need the
  `ashpd` GlobalShortcuts portal; the current `global-hotkey` backend is X11-only.

## How it works

| Concern        | Windows                  | Linux                                   |
| -------------- | ------------------------ | --------------------------------------- |
| Audio capture  | WASAPI (`cpal`)          | PipeWire / ALSA (`cpal`)                |
| Global hotkey  | Win32 `RegisterHotKey`   | X11 (Wayland portal: TODO)              |
| Paste          | `SendInput`              | XTest (X11) / RemoteDesktop portal (Wayland) |
| Tray icon      | `tray-icon` (Win32)      | `ksni` (D-Bus StatusNotifierItem)       |
| Notifications  | WinRT toast              | D-Bus (`notify-rust`)                   |
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

## Install (Windows)

```powershell
powershell -ExecutionPolicy Bypass -File install.ps1
```

This builds the release binary (if needed), copies it to
`%LOCALAPPDATA%\Programs\voice-dictate`, creates a Start Menu shortcut so it
shows up as an app, and launches it. To remove everything:

```powershell
powershell -ExecutionPolicy Bypass -File uninstall.ps1
```

## Build from source

Requires a Rust toolchain and a C linker.

```sh
cargo build --release
```

The binary is `target/release/voice-dictate`.

### Linux system dependencies

```sh
sudo apt install libasound2-dev libdbus-1-dev libxcb1-dev pkg-config
```

## Usage

A microphone icon appears in the tray. Default hotkey is **Ctrl+Shift+Space**:
press to start recording, press again to stop and transcribe. A short beep marks
start and stop. The tray menu offers "Start at login" and "Quit".

## License

MIT
