use crate::{
    config,
    state::{AppState, Phase},
    tray::icon_rgba,
};
use gtk::prelude::*;
use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    Icon, TrayIconBuilder,
};

pub fn run(
    state: AppState,
    on_toggle: Arc<dyn Fn() + Send + Sync>,
    shortcut: Arc<Mutex<String>>,
    paste_shortcut: Arc<Mutex<String>>,
) -> anyhow::Result<()> {
    gtk::init().map_err(|e| anyhow::anyhow!("GTK initialization failed: {e}"))?;

    let menu = Menu::new();
    let toggle_item = MenuItem::new("Start dictation", true, None);
    let status_item = MenuItem::new("Status: Ready", false, None);
    let shortcut_item = MenuItem::new(
        format!("Shortcut: {}", shortcut.lock().unwrap()),
        false,
        None,
    );
    let configure_item = MenuItem::new("Change shortcut...", true, None);
    let paste_with_shift = paste_shortcut
        .lock()
        .unwrap()
        .eq_ignore_ascii_case("Ctrl+Shift+V");
    let paste_ctrl_v_item = CheckMenuItem::new("Ctrl+V", true, !paste_with_shift, None);
    let paste_ctrl_shift_v_item = CheckMenuItem::new("Ctrl+Shift+V", true, paste_with_shift, None);
    let paste_menu = Submenu::with_items(
        format!(
            "Paste with: {}",
            if paste_with_shift {
                "Ctrl+Shift+V"
            } else {
                "Ctrl+V"
            }
        ),
        true,
        &[&paste_ctrl_v_item, &paste_ctrl_shift_v_item],
    )?;
    let quit_item = MenuItem::new("Quit", true, None);
    menu.append_items(&[
        &toggle_item,
        &status_item,
        &PredefinedMenuItem::separator(),
        &shortcut_item,
        &configure_item,
        &paste_menu,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ])?;

    let (rgba, width, height) = icon_rgba(Phase::Idle);
    let icon = Icon::from_rgba(rgba, width, height)?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Voice Dictate")
        .build()?;

    let toggle_id = toggle_item.id().clone();
    let configure_id = configure_item.id().clone();
    let paste_ctrl_v_id = paste_ctrl_v_item.id().clone();
    let paste_ctrl_shift_v_id = paste_ctrl_shift_v_item.id().clone();
    let quit_id = quit_item.id().clone();
    let menu_rx = MenuEvent::receiver();
    let mut displayed_phase = Phase::Idle;
    let mut displayed_shortcut = shortcut.lock().unwrap().clone();

    loop {
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        while let Ok(event) = menu_rx.try_recv() {
            if event.id == toggle_id {
                on_toggle();
            } else if event.id == configure_id {
                show_shortcut_dialog(Arc::clone(&shortcut));
            } else if event.id == paste_ctrl_v_id {
                match config::save_paste_shortcut("Ctrl+V") {
                    Ok(()) => {
                        *paste_shortcut.lock().unwrap() = "Ctrl+V".to_string();
                        paste_ctrl_v_item.set_checked(true);
                        paste_ctrl_shift_v_item.set_checked(false);
                        paste_menu.set_text("Paste with: Ctrl+V");
                    }
                    Err(error) => tracing::error!("paste shortcut: {error}"),
                }
            } else if event.id == paste_ctrl_shift_v_id {
                match config::save_paste_shortcut("Ctrl+Shift+V") {
                    Ok(()) => {
                        *paste_shortcut.lock().unwrap() = "Ctrl+Shift+V".to_string();
                        paste_ctrl_v_item.set_checked(false);
                        paste_ctrl_shift_v_item.set_checked(true);
                        paste_menu.set_text("Paste with: Ctrl+Shift+V");
                    }
                    Err(error) => tracing::error!("paste shortcut: {error}"),
                }
            } else if event.id == quit_id {
                std::process::exit(0);
            }
        }

        let phase = state.get();
        if phase != displayed_phase {
            displayed_phase = phase;
            let (toggle_text, status_text, toggle_enabled) = match phase {
                Phase::Idle => ("Start dictation", "Status: Ready", true),
                Phase::Recording => ("Stop dictation", "Status: Recording", true),
                Phase::Transcribing => ("Transcribing...", "Status: Transcribing", false),
            };
            toggle_item.set_text(toggle_text);
            toggle_item.set_enabled(toggle_enabled);
            status_item.set_text(status_text);

            let (rgba, width, height) = icon_rgba(phase);
            if let Ok(icon) = Icon::from_rgba(rgba, width, height) {
                let _ = tray.set_icon(Some(icon));
            }
        }

        let current_shortcut = shortcut.lock().unwrap().clone();
        if current_shortcut != displayed_shortcut {
            displayed_shortcut = current_shortcut;
            shortcut_item.set_text(format!("Shortcut: {displayed_shortcut}"));
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

fn show_shortcut_dialog(shortcut: Arc<Mutex<String>>) {
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Voice Dictate shortcut");
    window.set_default_size(420, 140);
    window.set_resizable(false);
    window.set_position(gtk::WindowPosition::Center);
    window.set_keep_above(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let title = gtk::Label::new(Some("Press a new keyboard shortcut"));
    title.set_xalign(0.0);
    let feedback = gtk::Label::new(Some(
        "Use Ctrl, Alt, Shift, or Super with a letter, Space, or F1 to F12.\nPress Esc to cancel.",
    ));
    feedback.set_xalign(0.0);
    feedback.set_line_wrap(true);
    content.pack_start(&title, false, false, 0);
    content.pack_start(&feedback, false, false, 0);
    window.add(&content);

    window.connect_key_press_event(move |window, event| {
        if event.keyval() == gtk::gdk::keys::constants::Escape {
            window.close();
            return gtk::glib::Propagation::Stop;
        }
        if event.is_modifier() {
            return gtk::glib::Propagation::Stop;
        }

        let Some((display, accelerator)) = shortcut_from_event(event) else {
            feedback.set_text(
                "That combination is not supported. Use a modifier with a letter or Space, or use F1 to F12.",
            );
            return gtk::glib::Propagation::Stop;
        };

        match save_shortcut(&display, &accelerator) {
            Ok(()) => {
                *shortcut.lock().unwrap() = display;
                window.close();
            }
            Err(error) => feedback.set_text(&format!("Could not save the shortcut: {error}")),
        }
        gtk::glib::Propagation::Stop
    });

    window.show_all();
    window.present();
}

fn shortcut_from_event(event: &gtk::gdk::EventKey) -> Option<(String, String)> {
    let key_name = event.keyval().name()?.to_string();
    let (display_key, accelerator_key, function_key) = if key_name.len() == 1
        && key_name
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        (key_name.to_uppercase(), key_name.to_lowercase(), false)
    } else if key_name.eq_ignore_ascii_case("space") {
        ("Space".to_string(), "space".to_string(), false)
    } else if matches!(
        key_name.as_str(),
        "F1" | "F2" | "F3" | "F4" | "F5" | "F6" | "F7" | "F8" | "F9" | "F10" | "F11" | "F12"
    ) {
        (key_name.clone(), key_name, true)
    } else {
        return None;
    };

    let state = event.state();
    let mut display_parts = Vec::new();
    let mut accelerator = String::new();
    if state.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
        display_parts.push("Ctrl");
        accelerator.push_str("<Control>");
    }
    if state.contains(gtk::gdk::ModifierType::MOD1_MASK) {
        display_parts.push("Alt");
        accelerator.push_str("<Alt>");
    }
    if state.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
        display_parts.push("Shift");
        accelerator.push_str("<Shift>");
    }
    if state.intersects(gtk::gdk::ModifierType::SUPER_MASK | gtk::gdk::ModifierType::META_MASK) {
        display_parts.push("Super");
        accelerator.push_str("<Super>");
    }
    if display_parts.is_empty() && !function_key {
        return None;
    }

    display_parts.push(&display_key);
    accelerator.push_str(&accelerator_key);
    Some((display_parts.join("+"), accelerator))
}

fn save_shortcut(display: &str, accelerator: &str) -> anyhow::Result<()> {
    let settings = "org.gnome.settings-daemon.global-shortcuts.application:/org/gnome/settings-daemon/global-shortcuts/com.daniil.VoiceDictate/";
    let value = format!(
        "[('toggle-dictation', {{'shortcuts': <['{accelerator}']>, 'description': <'Start/stop voice dictation'>}})]"
    );
    let status = std::process::Command::new("gsettings")
        .args(["set", settings, "shortcuts", &value])
        .status()?;
    if !status.success() {
        anyhow::bail!("gsettings exited with {status}");
    }
    config::save_hotkey(display)?;
    Ok(())
}
