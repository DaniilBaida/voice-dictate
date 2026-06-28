use crate::state::AppState;

pub use runner::run;

#[cfg(windows)]
mod runner {
    use super::*;
    use global_hotkey::{hotkey::HotKey, GlobalHotKeyManager};

    pub fn run(
        state: AppState,
        on_toggle: impl Fn() + Send + 'static,
        manager: GlobalHotKeyManager,
        current: HotKey,
    ) {
        windows::run_tray(state, on_toggle, manager, current);
    }
}

#[cfg(target_os = "linux")]
mod runner {
    use super::*;

    pub fn run(
        state: AppState,
        on_toggle: impl Fn() + Send + Sync + 'static,
        on_quit: impl Fn() + Send + Sync + 'static,
    ) {
        linux::run_tray(state, on_toggle, on_quit);
    }
}

#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

const ICON_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon32.png"));

pub fn icon_rgba(recording: bool) -> (Vec<u8>, u32, u32) {
    let img = image::load_from_memory(ICON_PNG)
        .expect("embedded icon PNG is invalid")
        .into_rgba8();

    if recording {
        let mut tinted = img.clone();
        for px in tinted.pixels_mut() {
            px[0] = px[0].saturating_add(80);
            px[1] = px[1].saturating_sub(40);
            px[2] = px[2].saturating_sub(40);
        }
        let (w, h) = tinted.dimensions();
        (tinted.into_raw(), w, h)
    } else {
        let (w, h) = img.dimensions();
        (img.into_raw(), w, h)
    }
}
