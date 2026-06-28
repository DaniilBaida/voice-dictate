#![cfg_attr(windows, windows_subsystem = "windows")]

mod audio;
mod config;
mod notify;
mod paste;
mod sound;
mod startup;
mod state;
mod transcribe;
mod tray;

use arboard::Clipboard;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use state::{AppState, Phase};
use std::sync::Arc;
use tracing::info;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "voice_dictate=info".into()),
        )
        .init();

    let cfg = config::load();
    let api_key = match config::resolve_api_key(&cfg) {
        Some(k) => k,
        None => {
            eprintln!(
                "No OpenAI API key found.\n\
                 Set OPENAI_API_KEY env var or add openai_api_key = \"sk-...\" to\n\
                 {:?}",
                config::config_file()
            );
            std::process::exit(1);
        }
    };

    info!("starting voice-dictate");
    notify::init_icon();

    let app_state = AppState::new();

    // RecorderHandle is Send - cpal Stream stays on the audio thread
    let recorder = Arc::new(audio::spawn_audio_thread(cfg.samplerate)?);

    let transcriber = Arc::new(transcribe::Transcriber::new(
        &api_key,
        &cfg.model,
        &cfg.language,
        &cfg.prompt,
    ));

    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?,
    );

    let manager = GlobalHotKeyManager::new()?;
    let hotkey = parse_hotkey(&cfg.hotkey)?;
    manager.register(hotkey)?;
    let hotkey_id = hotkey.id();

    let state_toggle = app_state.clone();
    let state_tray = app_state.clone();
    let rec = Arc::clone(&recorder);
    let tx = Arc::clone(&transcriber);
    let rt2 = Arc::clone(&rt);

    let toggle = Arc::new(move || {
        let state = &state_toggle;

        if state.transition(Phase::Idle, Phase::Recording) {
            match rec.start() {
                Ok(_) => {
                    info!("recording started");
                    sound::recording_start();
                    notify::send("Voice Dictate", "Recording...");
                }
                Err(e) => {
                    tracing::error!("recorder start: {e}");
                    state.set(Phase::Idle);
                }
            }
        } else if state.transition(Phase::Recording, Phase::Transcribing) {
            sound::recording_stop();
            let wav = match rec.stop() {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("recorder stop: {e}");
                    state.set(Phase::Idle);
                    return;
                }
            };

            if wav.is_empty() {
                info!("no audio captured");
                state.set(Phase::Idle);
                return;
            }

            info!("transcribing {} bytes", wav.len());
            notify::send("Voice Dictate", "Transcribing...");

            let state2 = state.clone();
            let transcriber = Arc::clone(&tx);
            rt2.spawn(async move {
                match transcriber.transcribe(wav).await {
                    Ok(text) if !text.is_empty() => {
                        info!("transcription: {text}");
                        if let Err(e) = Clipboard::new().and_then(|mut c| c.set_text(text.clone())) {
                            tracing::error!("clipboard: {e}");
                            state2.set(Phase::Idle);
                            return;
                        }
                        match paste::paste().await {
                            Ok(true) => {
                                notify::send("Voice Dictate", &text[..text.len().min(120)]);
                            }
                            _ => {
                                notify::send(
                                    "Voice Dictate",
                                    "Text copied to clipboard - press Ctrl+V to paste",
                                );
                            }
                        }
                    }
                    Ok(_) => notify::send("Voice Dictate", "Nothing recognised."),
                    Err(e) => {
                        tracing::error!("transcription: {e}");
                        notify::send("Voice Dictate", &format!("Error: {e}"));
                    }
                }
                state2.set(Phase::Idle);
            });
        }
        // Phase::Transcribing: hotkey press is ignored
    });

    let toggle_hk = Arc::clone(&toggle);
    let toggle_tray = Arc::clone(&toggle);

    std::thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();
        loop {
            if let Ok(ev) = receiver.recv() {
                if ev.id == hotkey_id && ev.state == HotKeyState::Pressed {
                    toggle_hk();
                }
            }
        }
    });

    notify::send("Voice Dictate", &format!("Ready. Hotkey: {}", cfg.hotkey));

    tray::run(
        state_tray,
        move || toggle_tray(),
        || std::process::exit(0),
    );

    Ok(())
}

fn parse_hotkey(s: &str) -> anyhow::Result<HotKey> {
    let mut mods = Modifiers::empty();
    let mut key_code = None;

    for part in s.split('+') {
        match part.trim().to_lowercase().as_str() {
            "cmdorctrl" | "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            "super" | "meta" | "cmd" | "win" => mods |= Modifiers::SUPER,
            other => key_code = Some(parse_key_code(other)?),
        }
    }

    let code = key_code.ok_or_else(|| anyhow::anyhow!("no key in hotkey: {s}"))?;
    Ok(HotKey::new(Some(mods), code))
}

fn parse_key_code(s: &str) -> anyhow::Result<Code> {
    match s.to_lowercase().as_str() {
        "space" => Ok(Code::Space),
        "a" => Ok(Code::KeyA), "b" => Ok(Code::KeyB), "c" => Ok(Code::KeyC),
        "d" => Ok(Code::KeyD), "e" => Ok(Code::KeyE), "f" => Ok(Code::KeyF),
        "g" => Ok(Code::KeyG), "h" => Ok(Code::KeyH), "i" => Ok(Code::KeyI),
        "j" => Ok(Code::KeyJ), "k" => Ok(Code::KeyK), "l" => Ok(Code::KeyL),
        "m" => Ok(Code::KeyM), "n" => Ok(Code::KeyN), "o" => Ok(Code::KeyO),
        "p" => Ok(Code::KeyP), "q" => Ok(Code::KeyQ), "r" => Ok(Code::KeyR),
        "s" => Ok(Code::KeyS), "t" => Ok(Code::KeyT), "u" => Ok(Code::KeyU),
        "v" => Ok(Code::KeyV), "w" => Ok(Code::KeyW), "x" => Ok(Code::KeyX),
        "y" => Ok(Code::KeyY), "z" => Ok(Code::KeyZ),
        "f1"  => Ok(Code::F1),  "f2"  => Ok(Code::F2),  "f3"  => Ok(Code::F3),
        "f4"  => Ok(Code::F4),  "f5"  => Ok(Code::F5),  "f6"  => Ok(Code::F6),
        "f7"  => Ok(Code::F7),  "f8"  => Ok(Code::F8),  "f9"  => Ok(Code::F9),
        "f10" => Ok(Code::F10), "f11" => Ok(Code::F11), "f12" => Ok(Code::F12),
        other => anyhow::bail!("unknown key: {other}"),
    }
}
