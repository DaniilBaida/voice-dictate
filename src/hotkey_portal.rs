//! Wayland-native global hotkey via the XDG GlobalShortcuts portal.
//!
//! Unlike the X11 `global-hotkey` backend, this receives activations even when a
//! native Wayland window has focus. The user assigns/confirms the actual key
//! combo through the desktop's portal dialog on first run; the compositor then
//! owns the binding. We propose a preferred trigger derived from the config.

use std::sync::Arc;

const SHORTCUT_ID: &str = "toggle-dictation";

pub async fn run(hotkey: String, toggle: Arc<dyn Fn() + Send + Sync>) {
    if let Err(e) = run_inner(hotkey, toggle).await {
        tracing::error!("global shortcuts portal: {e:#}");
    }
}

async fn run_inner(hotkey: String, toggle: Arc<dyn Fn() + Send + Sync>) -> anyhow::Result<()> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use futures_util::StreamExt;

    let global = GlobalShortcuts::new().await?;
    let session = global.create_session().await?;

    // Subscribe before binding so we never miss an early activation.
    let mut activated = global.receive_activated().await?;

    let trigger = to_portal_trigger(&hotkey);
    let shortcut = NewShortcut::new(SHORTCUT_ID, "Start/stop voice dictation")
        .preferred_trigger(trigger.as_deref());

    global
        .bind_shortcuts(&session, &[shortcut], None)
        .await?
        .response()?;

    tracing::info!("global shortcut bound via portal (waiting for activations)");

    while let Some(act) = activated.next().await {
        if act.shortcut_id() == SHORTCUT_ID {
            toggle();
        }
    }

    Ok(())
}

/// Convert a config hotkey string ("Ctrl+Space") into the portal trigger
/// syntax ("CTRL+space"). This is only a *preferred* hint; the compositor's
/// dialog lets the user pick the final combo.
fn to_portal_trigger(hotkey: &str) -> Option<String> {
    let mut mods: Vec<&str> = Vec::new();
    let mut key: Option<String> = None;

    for part in hotkey.split('+') {
        match part.trim().to_lowercase().as_str() {
            "" => {}
            "ctrl" | "control" | "cmdorctrl" => mods.push("CTRL"),
            "shift" => mods.push("SHIFT"),
            "alt" | "option" => mods.push("ALT"),
            "super" | "meta" | "cmd" | "win" => mods.push("LOGO"),
            other => key = Some(other.to_string()),
        }
    }

    let key = key?;
    let mut out = String::new();
    for m in mods {
        out.push_str(m);
        out.push('+');
    }
    out.push_str(&key);
    Some(out)
}
