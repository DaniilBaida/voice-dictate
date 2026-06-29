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

#[cfg(windows)]
mod windows;

const ICON_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon32.png"));

pub fn icon_rgba(recording: bool) -> (Vec<u8>, u32, u32) {
    let img = image::load_from_memory(ICON_PNG)
        .expect("embedded icon PNG is invalid")
        .into_rgba8();

    if recording {
        // Paint the mic clearly red while recording (keep the alpha/shape).
        let mut tinted = img.clone();
        for px in tinted.pixels_mut() {
            if px[3] > 0 {
                px[0] = 230;
                px[1] = 40;
                px[2] = 40;
            }
        }
        let (w, h) = tinted.dimensions();
        (tinted.into_raw(), w, h)
    } else {
        let (w, h) = img.dimensions();
        (img.into_raw(), w, h)
    }
}
