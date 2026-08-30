use crate::{config, hotkey, hotkey_capture, notify, startup, state::AppState, tray::icon_rgba};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyManager};
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

pub fn run_tray(
    state: AppState,
    on_toggle: impl Fn() + Send + 'static,
    manager: GlobalHotKeyManager,
    mut current: HotKey,
) {
    let menu = Menu::new();
    let toggle_item = MenuItem::new("Start / Stop dictation", true, None);
    let set_hotkey_item = MenuItem::new("Set hotkey...", true, None);
    let startup_item = CheckMenuItem::new("Start at login", true, startup::is_installed(), None);
    let quit_item = MenuItem::new("Quit", true, None);
    let _ = menu.append_items(&[
        &toggle_item,
        &PredefinedMenuItem::separator(),
        &set_hotkey_item,
        &startup_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ]);

    let (rgba, w, h) = icon_rgba(crate::state::Phase::Idle);
    let icon = Icon::from_rgba(rgba, w, h).expect("icon creation failed");

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Voice Dictate")
        .build()
        .expect("tray icon creation failed");

    let toggle_id = toggle_item.id().clone();
    let set_hotkey_id = set_hotkey_item.id().clone();
    let startup_id = startup_item.id().clone();
    let quit_id = quit_item.id().clone();
    let menu_rx = MenuEvent::receiver();

    loop {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        while let Ok(ev) = menu_rx.try_recv() {
            if ev.id == toggle_id {
                on_toggle();
            } else if ev.id == set_hotkey_id {
                hotkey_capture::begin();
                notify::send("Voice Dictate", "Press the key combination now (Esc to cancel)");
            } else if ev.id == startup_id {
                if startup::is_installed() {
                    startup::uninstall();
                } else {
                    startup::install();
                }
            } else if ev.id == quit_id {
                std::process::exit(0);
            }
        }

        // Hotkey capture result (if any)
        match hotkey_capture::poll() {
            hotkey_capture::Poll::Captured(combo) => {
                apply_hotkey(&manager, &mut current, &combo);
            }
            hotkey_capture::Poll::Cancelled => {
                notify::send("Voice Dictate", "Hotkey change cancelled");
            }
            _ => {}
        }

        // Update icon colour to reflect recording state
        let (rgba, w, h) = icon_rgba(state.get());
        if let Ok(icon) = Icon::from_rgba(rgba, w, h) {
            let _ = tray.set_icon(Some(icon));
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

fn apply_hotkey(manager: &GlobalHotKeyManager, current: &mut HotKey, combo: &str) {
    let new = match hotkey::parse(combo) {
        Ok(h) => h,
        Err(e) => {
            notify::send("Voice Dictate", &format!("Invalid combination: {e}"));
            return;
        }
    };

    let _ = manager.unregister(*current);
    match manager.register(new) {
        Ok(_) => {
            *current = new;
            if let Err(e) = config::save_hotkey(combo) {
                tracing::warn!("could not persist hotkey: {e}");
            }
            notify::send("Voice Dictate", &format!("Hotkey set to {combo}"));
        }
        Err(e) => {
            // Roll back to the previous hotkey so the app stays usable
            let _ = manager.register(*current);
            notify::send("Voice Dictate", &format!("Could not set {combo}: {e}"));
        }
    }
}
