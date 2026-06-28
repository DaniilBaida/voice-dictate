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
