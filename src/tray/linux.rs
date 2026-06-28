use crate::{state::AppState, tray::icon_rgba};
use ksni::{MenuItem, Tray, TrayService};
use std::sync::{Arc, Mutex};

struct VoiceTray {
    state: AppState,
    on_toggle: Arc<dyn Fn() + Send + Sync>,
    on_quit: Arc<dyn Fn() + Send + Sync>,
}

impl Tray for VoiceTray {
    fn title(&self) -> String {
        "Voice Dictate".into()
    }

    fn icon_name(&self) -> String {
        // Use XDG theme icon as fallback; the PNG pixmap is set via icon_pixmap
        "audio-input-microphone".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let recording = self.state.get() == crate::state::Phase::Recording;
        let (rgba, w, h) = icon_rgba(recording);

        // ksni expects ARGB32 in network byte order
        let argb: Vec<u8> = rgba
            .chunks_exact(4)
            .flat_map(|px| {
                let [r, g, b, a] = [px[0], px[1], px[2], px[3]];
                [a, r, g, b]
            })
            .collect();

        vec![ksni::Icon {
            width: w as i32,
            height: h as i32,
            data: argb,
        }]
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            MenuItem::Standard(ksni::menu::StandardItem {
                label: "Start / Stop dictation".into(),
                activate: Box::new(|this: &mut Self| (this.on_toggle)()),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(ksni::menu::StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| (this.on_quit)()),
                ..Default::default()
            }),
        ]
    }
}

pub fn run_tray(
    state: AppState,
    on_toggle: impl Fn() + Send + Sync + 'static,
    on_quit: impl Fn() + Send + Sync + 'static,
) {
    let tray = VoiceTray {
        state,
        on_toggle: Arc::new(on_toggle),
        on_quit: Arc::new(on_quit),
    };

    let service = TrayService::new(tray);
    // handle() lets us update the icon; run() blocks the thread
    let _handle = service.handle();
    service.run();
}
