//! Wayland-native global hotkey via the XDG GlobalShortcuts portal.
//!
//! Unlike the X11 `global-hotkey` backend, this receives activations even when a
//! native Wayland window has focus. The user assigns/confirms the actual key
//! combo through the desktop's portal dialog on first run; the compositor then
//! owns the binding. We propose a preferred trigger derived from the config.

use crate::state::Mode;
use std::sync::Arc;
use std::sync::Mutex;

pub const SHORTCUT_ID: &str = "toggle-dictation";
pub const PROMPT_SHORTCUT_ID: &str = "toggle-dictation-prompt";

pub async fn run(
    hotkey: String,
    prompt_hotkey: String,
    toggle: Arc<dyn Fn(Mode) + Send + Sync>,
    current_shortcut: Arc<Mutex<String>>,
    current_prompt_shortcut: Arc<Mutex<String>>,
) {
    if let Err(e) = run_inner(
        hotkey,
        prompt_hotkey,
        toggle,
        current_shortcut,
        current_prompt_shortcut,
    )
    .await
    {
        tracing::error!("global shortcuts portal: {e:#}");
    }
}

async fn run_inner(
    hotkey: String,
    prompt_hotkey: String,
    toggle: Arc<dyn Fn(Mode) + Send + Sync>,
    current_shortcut: Arc<Mutex<String>>,
    current_prompt_shortcut: Arc<Mutex<String>>,
) -> anyhow::Result<()> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use futures_util::StreamExt;

    let global = GlobalShortcuts::new().await?;
    let session = global.create_session().await?;

    // Subscribe before binding so we never miss an early activation.
    let mut activated = global.receive_activated().await?;
    let mut changed = global.receive_shortcuts_changed().await?;

    let trigger = to_portal_trigger(&hotkey);
    let shortcut = NewShortcut::new(SHORTCUT_ID, "Start/stop voice dictation")
        .preferred_trigger(trigger.as_deref());
    let prompt_trigger = to_portal_trigger(&prompt_hotkey);
    let prompt_shortcut = NewShortcut::new(PROMPT_SHORTCUT_ID, "Start/stop dictation in prompt mode")
        .preferred_trigger(prompt_trigger.as_deref());

    let response = global
        .bind_shortcuts(&session, &[shortcut, prompt_shortcut], None)
        .await?
        .response()?;
    update_current_shortcut(response.shortcuts(), SHORTCUT_ID, &current_shortcut);
    update_current_shortcut(
        response.shortcuts(),
        PROMPT_SHORTCUT_ID,
        &current_prompt_shortcut,
    );

    tracing::info!("global shortcut bound via portal (waiting for activations)");

    loop {
        tokio::select! {
            Some(activation) = activated.next() => {
                match activation.shortcut_id() {
                    SHORTCUT_ID => toggle(Mode::Raw),
                    PROMPT_SHORTCUT_ID => toggle(Mode::Prompt),
                    other => tracing::warn!("unknown portal shortcut: {other}"),
                }
            }
            Some(event) = changed.next() => {
                update_current_shortcut(event.shortcuts(), SHORTCUT_ID, &current_shortcut);
                update_current_shortcut(
                    event.shortcuts(),
                    PROMPT_SHORTCUT_ID,
                    &current_prompt_shortcut,
                );
            }
        }
    }
}

fn update_current_shortcut(
    shortcuts: &[ashpd::desktop::global_shortcuts::Shortcut],
    id: &str,
    current_shortcut: &Mutex<String>,
) {
    if let Some(shortcut) = shortcuts.iter().find(|item| item.id() == id) {
        *current_shortcut.lock().unwrap() = display_shortcut(shortcut.trigger_description());
    }
}

fn display_shortcut(trigger: &str) -> String {
    let mut display = trigger
        .strip_prefix("Press ")
        .unwrap_or(trigger)
        .replace("<Control>", "Ctrl+")
        .replace("<Alt>", "Alt+")
        .replace("<Shift>", "Shift+")
        .replace("<Super>", "Super+");

    if let Some((prefix, key)) = display.rsplit_once('+') {
        display = format!("{prefix}+{}", display_key(key));
    } else {
        display = display_key(&display);
    }
    display
}

fn display_key(key: &str) -> String {
    if key.eq_ignore_ascii_case("space") {
        "Space".to_string()
    } else if key.len() == 1 {
        key.to_uppercase()
    } else {
        key.to_string()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats_portal_shortcut_for_the_menu() {
        assert_eq!(super::display_shortcut("Press <Control>space"), "Ctrl+Space");
        assert_eq!(
            super::display_shortcut("Press <Control><Shift>v"),
            "Ctrl+Shift+V"
        );
    }
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
