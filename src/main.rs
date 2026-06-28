#![cfg_attr(windows, windows_subsystem = "windows")]

mod audio;
mod config;
mod hotkey;
#[cfg(windows)]
mod hotkey_capture;
mod notify;
mod paste;
mod sound;
mod startup;
mod state;
mod transcribe;
mod tray;

use arboard::Clipboard;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
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
    let current_hotkey = hotkey::parse(&cfg.hotkey)?;
    manager.register(current_hotkey)?;

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

    // Only one hotkey is ever registered at a time, so any Pressed event is ours.
    std::thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();
        loop {
            if let Ok(ev) = receiver.recv() {
                if ev.state == HotKeyState::Pressed {
                    toggle_hk();
                }
            }
        }
    });

    notify::send("Voice Dictate", &format!("Ready. Hotkey: {}", cfg.hotkey));

    #[cfg(windows)]
    tray::run(state_tray, move || toggle_tray(), manager, current_hotkey);

    #[cfg(target_os = "linux")]
    {
        // Keep the manager alive for the lifetime of the process.
        let _manager = manager;
        let _ = current_hotkey;
        tray::run(state_tray, move || toggle_tray(), || std::process::exit(0));
    }

    Ok(())
}
