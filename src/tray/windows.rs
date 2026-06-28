use crate::{startup, state::AppState, tray::icon_rgba};
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
    on_quit: impl Fn() + Send + 'static,
) {
    let menu = Menu::new();
    let toggle_item = MenuItem::new("Start / Stop dictation", true, None);
    let startup_item = CheckMenuItem::new("Start at login", true, startup::is_installed(), None);
    let quit_item = MenuItem::new("Quit", true, None);
    let _ = menu.append_items(&[
        &toggle_item,
        &PredefinedMenuItem::separator(),
        &startup_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ]);

    let (rgba, w, h) = icon_rgba(false);
    let icon = Icon::from_rgba(rgba, w, h).expect("icon creation failed");

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Voice Dictate")
        .build()
        .expect("tray icon creation failed");

    let toggle_id = toggle_item.id().clone();
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
            } else if ev.id == startup_id {
                if startup::is_installed() {
                    startup::uninstall();
                    startup_item.set_checked(false);
                } else {
                    startup::install();
                    startup_item.set_checked(true);
                }
            } else if ev.id == quit_id {
                on_quit();
                return;
            }
        }

        let recording = state.get() == crate::state::Phase::Recording;
        let (rgba, w, h) = icon_rgba(recording);
        if let Ok(icon) = Icon::from_rgba(rgba, w, h) {
            let _ = tray.set_icon(Some(icon));
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
