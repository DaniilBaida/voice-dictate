#![cfg_attr(windows, windows_subsystem = "windows")]

mod audio;
mod config;
mod hotkey;
#[cfg(windows)]
mod hotkey_capture;
#[cfg(all(target_os = "linux", feature = "wayland"))]
mod hotkey_portal;
mod notify;
mod paste;
mod sound;
#[cfg(windows)]
mod startup;
mod state;
mod transcribe;
#[cfg(any(windows, target_os = "linux"))]
mod tray;

use arboard::Clipboard;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use state::{AppState, Phase};
use std::sync::{Arc, Mutex};
use tracing::info;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "voice_dictate=info".into()),
        )
        .init();

    let cfg = config::load();
    info!("starting voice-dictate");
    notify::init_icon();

    let app_state = AppState::new();
    let paste_shortcut = Arc::new(Mutex::new(cfg.paste_shortcut.clone()));

    // RecorderHandle is Send - cpal Stream stays on the audio thread
    let recorder = Arc::new(audio::spawn_audio_thread(cfg.samplerate)?);

    let transcriber = Arc::new(transcribe::Transcriber::new(
        &cfg.server_url,
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

    let state_toggle = app_state.clone();
    #[cfg(windows)]
    let state_tray = app_state.clone();
    let rec = Arc::clone(&recorder);
    let tx = Arc::clone(&transcriber);
    let rt2 = Arc::clone(&rt);
    let paste_shortcut_toggle = Arc::clone(&paste_shortcut);

    let toggle = Arc::new(move || {
        let state = &state_toggle;

        if state.transition(Phase::Idle, Phase::Recording) {
            match rec.start() {
                Ok(_) => {
                    info!("recording started");
                    sound::recording_start();
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

            let state2 = state.clone();
            let transcriber = Arc::clone(&tx);
            let paste_shortcut = paste_shortcut_toggle.lock().unwrap().clone();
            rt2.spawn(async move {
                match transcriber.transcribe(wav).await {
                    Ok(text) if !text.is_empty() => {
                        info!("transcription: {text}");
                        if let Err(e) = Clipboard::new().and_then(|mut c| c.set_text(text.clone()))
                        {
                            tracing::error!("clipboard: {e}");
                            state2.set(Phase::Idle);
                            return;
                        }
                        let _ = paste::paste(&paste_shortcut).await;
                    }
                    Ok(_) => info!("nothing recognised"),
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

    #[cfg(windows)]
    let toggle_tray = Arc::clone(&toggle);

    #[cfg(windows)]
    {
        let manager = GlobalHotKeyManager::new()?;
        let current_hotkey = hotkey::parse(&cfg.hotkey)?;
        manager.register(current_hotkey)?;

        let toggle_hk = Arc::clone(&toggle);
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

        tray::run(state_tray, move || toggle_tray(), manager, current_hotkey);
    }

    #[cfg(target_os = "linux")]
    {
        // On Wayland, X11 key grabs never fire while a native Wayland window has
        // focus, so use the GlobalShortcuts portal instead. Fall back to the X11
        // global-hotkey backend on X11 sessions.
        let on_wayland = is_wayland_session();
        let current_shortcut = Arc::new(Mutex::new(cfg.hotkey.clone()));

        #[cfg(feature = "wayland")]
        let portal = if on_wayland {
            // Non-sandboxed apps must register their app id with the host portal
            // Registry, otherwise GlobalShortcuts rejects the session with
            // "An app id is required". This must run on ashpd's shared
            // connection before the shortcuts session is created.
            if let Err(e) = rt.block_on(register_portal_app_id()) {
                tracing::warn!("portal app-id registration failed: {e:#}");
            }
            let toggle_dyn: Arc<dyn Fn() + Send + Sync> = Arc::clone(&toggle) as _;
            rt.spawn(hotkey_portal::run(
                cfg.hotkey.clone(),
                toggle_dyn,
                Arc::clone(&current_shortcut),
            ));
            true
        } else {
            false
        };
        #[cfg(not(feature = "wayland"))]
        let portal = false;

        // Keep the X11 manager alive for the process lifetime when we use it.
        let _x11_manager = if portal {
            None
        } else {
            let manager = GlobalHotKeyManager::new()?;
            let current_hotkey = hotkey::parse(&cfg.hotkey)?;
            manager.register(current_hotkey)?;

            let toggle_hk = Arc::clone(&toggle);
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
            Some(manager)
        };

        info!("ready (hotkey: {})", cfg.hotkey);
        let toggle_tray: Arc<dyn Fn() + Send + Sync> = Arc::clone(&toggle) as _;
        tray::run_linux(app_state, toggle_tray, current_shortcut, paste_shortcut)?;
    }

    Ok(())
}

/// App id that must match the installed `.desktop` file so the host portal
/// Registry accepts our registration.
#[cfg(all(target_os = "linux", feature = "wayland"))]
const PORTAL_APP_ID: &str = "com.daniil.VoiceDictate";

#[cfg(all(target_os = "linux", feature = "wayland"))]
async fn register_portal_app_id() -> anyhow::Result<()> {
    let app_id = ashpd::AppID::try_from(PORTAL_APP_ID)
        .map_err(|e| anyhow::anyhow!("invalid app id: {e}"))?;
    ashpd::register_host_app(app_id).await?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}
