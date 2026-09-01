#[cfg(windows)]
use crate::state::AppState;

#[cfg(windows)]
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

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::run as run_linux;

const ICON_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon32.png"));

pub fn icon_rgba(phase: crate::state::Phase) -> (Vec<u8>, u32, u32) {
    let mut img = image::load_from_memory(ICON_PNG)
        .expect("embedded icon PNG is invalid")
        .into_rgba8();

    // Transcribing needs its own colour: it can run for seconds in prompt mode
    // and the hotkey is inert throughout, so an idle-looking icon reads as a
    // hang.
    let color = match phase {
        crate::state::Phase::Recording => [230, 40, 40],
        crate::state::Phase::Transcribing => [235, 165, 40],
        crate::state::Phase::Idle => [255, 255, 255],
    };
    for pixel in img.pixels_mut() {
        if pixel[3] > 0 {
            pixel[0] = color[0];
            pixel[1] = color[1];
            pixel[2] = color[2];
        }
    }
    let (width, height) = img.dimensions();
    (img.into_raw(), width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Phase;

    #[test]
    fn each_phase_has_its_own_colour() {
        let (idle, _, _) = icon_rgba(Phase::Idle);
        let (transcribing, _, _) = icon_rgba(Phase::Transcribing);
        let (recording, _, _) = icon_rgba(Phase::Recording);

        assert!(has_only_color(&idle, [255, 255, 255]));
        assert!(has_only_color(&transcribing, [235, 165, 40]));
        assert!(has_only_color(&recording, [230, 40, 40]));
        assert_ne!(idle, transcribing);
        assert_ne!(transcribing, recording);
    }

    fn has_only_color(rgba: &[u8], color: [u8; 3]) -> bool {
        rgba.chunks_exact(4)
            .filter(|pixel| pixel[3] > 0)
            .all(|pixel| pixel[..3] == color)
    }
}
