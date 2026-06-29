static ICON_PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Write the embedded icon PNG to a temp file so notify-rust can reference it.
/// Called once at startup.
pub fn init_icon() {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/icon64.png"));
    let path = std::env::temp_dir().join("voice-dictate-icon.png");
    if std::fs::write(&path, bytes).is_ok() {
        let _ = ICON_PATH.set(path);
    }
}

pub fn send(title: &str, body: &str) {
    let title = title.to_string();
    let body = body.to_string();
    let icon = ICON_PATH.get().cloned();

    std::thread::spawn(move || {
        let mut n = notify_rust::Notification::new();
        n.appname("Voice Dictate").summary(&title).body(&body);
        if let Some(p) = icon {
            n.image_path(p.to_str().unwrap_or(""));
        }
        if let Err(e) = n.show() {
            tracing::warn!("notification error: {e}");
        }
    });
}

/// Show the transcription result. Clicking the notification re-copies the text
/// to the clipboard (the "default" action fires when the body is clicked).
#[cfg(target_os = "linux")]
pub fn send_result(text: String) {
    let icon = ICON_PATH.get().cloned();

    std::thread::spawn(move || {
        let mut n = notify_rust::Notification::new();
        n.appname("Voice Dictate").summary("Voice Dictate").body(&text);
        if let Some(p) = &icon {
            n.image_path(p.to_str().unwrap_or(""));
        }
        n.action("default", "Copy again");

        match n.show() {
            Ok(handle) => handle.wait_for_action(|action| {
                if action == "default" {
                    if let Err(e) =
                        arboard::Clipboard::new().and_then(|mut c| c.set_text(text.clone()))
                    {
                        tracing::warn!("re-copy failed: {e}");
                    }
                }
            }),
            Err(e) => tracing::warn!("notification error: {e}"),
        }
    });
}
