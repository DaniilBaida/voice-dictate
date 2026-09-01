use crate::{
    config, history,
    hotkey_portal::{PROMPT_SHORTCUT_ID, SHORTCUT_ID},
    state::{AppState, Mode, Phase},
    tray::icon_rgba,
};
use gtk::prelude::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu},
    Icon, TrayIconBuilder,
};

/// How many past dictations the history submenu lists.
const HISTORY_SHOWN: usize = 12;
/// Characters of each entry shown in the menu before it is elided.
const HISTORY_PREVIEW: usize = 60;

/// Which of the two dictation shortcuts a dialog is rebinding.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Which {
    Raw,
    Prompt,
}

pub fn run(
    state: AppState,
    on_toggle: Arc<dyn Fn(Mode) + Send + Sync>,
    shortcut: Arc<Mutex<String>>,
    prompt_shortcut: Arc<Mutex<String>>,
    paste_shortcut: Arc<Mutex<String>>,
    prompt_enabled: Arc<AtomicBool>,
    have_api_key: bool,
    server_reachable: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    gtk::init().map_err(|e| anyhow::anyhow!("GTK initialization failed: {e}"))?;

    let menu = Menu::new();

    // Only ever in the menu while something is wrong, so a visible row always
    // means there is something to fix.
    let status_item = MenuItem::new("", false, None);
    let status_separator = PredefinedMenuItem::separator();
    let mut status_shown = false;
    let mut displayed_problem: Option<String> = None;

    let prompt_switch = CheckMenuItem::new(
        "Prompt mode",
        true,
        prompt_enabled.load(Ordering::Relaxed),
        None,
    );

    // ── Recover the last dictation ────────────────────────────────────────────
    let copy_output_item = MenuItem::new("Copy last output", false, None);
    let copy_raw_item = MenuItem::new("Copy last raw transcript", false, None);

    // ── Dictate ───────────────────────────────────────────────────────────────
    let raw_item = MenuItem::new("Dictate raw", true, None);
    let prompt_item = MenuItem::new("Dictate prompt", true, None);

    // ── History ───────────────────────────────────────────────────────────────
    let history_menu = Submenu::new("History", true);
    let history_open_item = MenuItem::new("Open history file", true, None);
    let history_clear_item = MenuItem::new("Clear history", true, None);
    let mut history_ids = rebuild_history(
        &history_menu,
        &history_open_item,
        &history_clear_item,
    )?;

    refresh_copy_items(&copy_output_item, &copy_raw_item);

    // ── Shortcuts ─────────────────────────────────────────────────────────────
    let raw_shortcut_item = MenuItem::new(
        format!("Raw: {}", shortcut.lock().unwrap()),
        true,
        None,
    );
    let prompt_shortcut_item = MenuItem::new(
        format!("Prompt: {}", prompt_shortcut.lock().unwrap()),
        true,
        None,
    );
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
    let shortcuts_menu = Submenu::with_items(
        "Settings",
        true,
        &[
            &raw_shortcut_item,
            &prompt_shortcut_item,
            &PredefinedMenuItem::separator(),
            &paste_menu,
        ],
    )?;

    let quit_item = MenuItem::new("Quit", true, None);

    // Ordered by how often each row is actually reached: the switch, then
    // recovering what just came out, then the mouse-only fallback for dictating,
    // then setup.
    menu.append_items(&[
        &prompt_switch,
        &PredefinedMenuItem::separator(),
        &copy_output_item,
        &copy_raw_item,
        &history_menu,
        &PredefinedMenuItem::separator(),
        &raw_item,
        &prompt_item,
        &PredefinedMenuItem::separator(),
        &shortcuts_menu,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ])?;

    let (rgba, width, height) = icon_rgba(Phase::Idle);
    let icon = Icon::from_rgba(rgba, width, height)?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_icon(icon)
        .with_tooltip("Voice Dictate")
        .build()?;

    let raw_id = raw_item.id().clone();
    let prompt_id = prompt_item.id().clone();
    let prompt_switch_id = prompt_switch.id().clone();
    let copy_output_id = copy_output_item.id().clone();
    let copy_raw_id = copy_raw_item.id().clone();
    let raw_shortcut_id = raw_shortcut_item.id().clone();
    let prompt_shortcut_id = prompt_shortcut_item.id().clone();
    let history_open_id = history_open_item.id().clone();
    let history_clear_id = history_clear_item.id().clone();
    let paste_ctrl_v_id = paste_ctrl_v_item.id().clone();
    let paste_ctrl_shift_v_id = paste_ctrl_shift_v_item.id().clone();
    let quit_id = quit_item.id().clone();

    let menu_rx = MenuEvent::receiver();
    let mut displayed_phase = Phase::Idle;
    let mut displayed_shortcut = shortcut.lock().unwrap().clone();
    let mut displayed_prompt_shortcut = prompt_shortcut.lock().unwrap().clone();

    loop {
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        while let Ok(event) = menu_rx.try_recv() {
            if event.id == raw_id {
                on_toggle(Mode::Raw);
            } else if event.id == prompt_id {
                on_toggle(Mode::Prompt);
            } else if event.id == prompt_switch_id {
                let on = prompt_switch.is_checked();
                prompt_enabled.store(on, Ordering::Relaxed);
                tracing::info!("prompt mode: {on}");
            } else if event.id == copy_output_id {
                if let Some(entry) = history::load().first() {
                    copy_to_clipboard(entry.pasted(), "Copied the last output");
                }
            } else if event.id == copy_raw_id {
                if let Some(entry) = history::load().first() {
                    copy_to_clipboard(&entry.transcript, "Copied the raw transcript");
                }
            } else if event.id == raw_shortcut_id {
                show_shortcut_dialog(
                    Which::Raw,
                    Arc::clone(&shortcut),
                    Arc::clone(&prompt_shortcut),
                );
            } else if event.id == prompt_shortcut_id {
                show_shortcut_dialog(
                    Which::Prompt,
                    Arc::clone(&shortcut),
                    Arc::clone(&prompt_shortcut),
                );
            } else if event.id == history_open_id {
                open_history_file();
            } else if event.id == history_clear_id {
                match history::clear() {
                    Ok(()) => {
                        history_ids = rebuild_history(
                            &history_menu,
                            &history_open_item,
                            &history_clear_item,
                        )
                        .unwrap_or_default();
                        refresh_copy_items(&copy_output_item, &copy_raw_item);
                    }
                    Err(error) => tracing::error!("clear history: {error}"),
                }
            } else if let Some((_, text)) = history_ids.iter().find(|(id, _)| *id == event.id) {
                copy_to_clipboard(text, "Copied a history entry");
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
            // A dictation that just finished has written a history entry.
            let finished = displayed_phase == Phase::Transcribing && phase == Phase::Idle;
            displayed_phase = phase;

            let (raw_text, prompt_text, enabled) = match phase {
                Phase::Idle => ("Dictate raw", "Dictate prompt", true),
                Phase::Recording => ("Stop", "Stop", true),
                Phase::Transcribing => ("Transcribing...", "Transcribing...", false),
            };
            raw_item.set_text(raw_text);
            prompt_item.set_text(prompt_text);
            raw_item.set_enabled(enabled);
            prompt_item.set_enabled(enabled);

            let (rgba, width, height) = icon_rgba(phase);
            if let Ok(icon) = Icon::from_rgba(rgba, width, height) {
                let _ = tray.set_icon(Some(icon));
            }

            if finished {
                history_ids = rebuild_history(
                    &history_menu,
                    &history_open_item,
                    &history_clear_item,
                )
                .unwrap_or_default();
                refresh_copy_items(&copy_output_item, &copy_raw_item);
            }
        }

        let problem = if !server_reachable.load(Ordering::Relaxed) {
            Some("Speech server offline".to_string())
        } else if prompt_enabled.load(Ordering::Relaxed) && !have_api_key {
            Some("Prompt mode needs ANTHROPIC_API_KEY".to_string())
        } else {
            None
        };
        if problem != displayed_problem {
            displayed_problem = problem.clone();
            match &displayed_problem {
                Some(text) => {
                    status_item.set_text(text);
                    if !status_shown {
                        menu.insert(&status_item, 0)?;
                        menu.insert(&status_separator, 1)?;
                        status_shown = true;
                    }
                }
                None => {
                    if status_shown {
                        let _ = menu.remove(&status_item);
                        let _ = menu.remove(&status_separator);
                        status_shown = false;
                    }
                }
            }
        }

        let current_shortcut = shortcut.lock().unwrap().clone();
        if current_shortcut != displayed_shortcut {
            displayed_shortcut = current_shortcut;
            raw_shortcut_item.set_text(format!("Raw: {displayed_shortcut}"));
        }

        let current_prompt_shortcut = prompt_shortcut.lock().unwrap().clone();
        if current_prompt_shortcut != displayed_prompt_shortcut {
            displayed_prompt_shortcut = current_prompt_shortcut;
            prompt_shortcut_item.set_text(format!("Prompt: {displayed_prompt_shortcut}"));
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

/// Refills the history submenu from disk. Returns the id of each entry item
/// paired with the text that clicking it copies.
fn rebuild_history(
    menu: &Submenu,
    open_item: &MenuItem,
    clear_item: &MenuItem,
) -> anyhow::Result<Vec<(MenuId, String)>> {
    while menu.remove_at(0).is_some() {}

    let entries = history::load();
    let mut ids = Vec::new();

    if entries.is_empty() {
        menu.append(&MenuItem::new("No dictations yet", false, None))?;
    } else {
        for entry in entries.iter().take(HISTORY_SHOWN) {
            let marker = if entry.mode == "prompt" { "P" } else { "R" };
            let item = MenuItem::new(
                format!(
                    "[{marker}] {} · {}",
                    history::preview(entry.pasted(), HISTORY_PREVIEW),
                    history::age(entry.at)
                ),
                true,
                None,
            );
            ids.push((item.id().clone(), entry.pasted().to_string()));
            menu.append(&item)?;
        }
    }

    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(open_item)?;
    menu.append(clear_item)?;
    Ok(ids)
}

/// Enables the copy shortcuts against whatever the newest entry actually holds,
/// so a disabled row means there is nothing of that kind to copy.
fn refresh_copy_items(copy_output: &MenuItem, copy_raw: &MenuItem) {
    match history::load().first() {
        Some(entry) => {
            copy_output.set_enabled(true);
            // Only worth offering when it differs from the output.
            copy_raw.set_enabled(entry.prompt.is_some());
        }
        None => {
            copy_output.set_enabled(false);
            copy_raw.set_enabled(false);
        }
    }
}

fn copy_to_clipboard(text: &str, confirmation: &str) {
    match arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_string())) {
        Ok(()) => crate::notify::send("Voice Dictate", confirmation),
        Err(error) => tracing::error!("clipboard: {error}"),
    }
}

fn open_history_file() {
    let path = history::file();
    if let Err(error) = std::process::Command::new("xdg-open").arg(&path).spawn() {
        tracing::error!("xdg-open {}: {error}", path.display());
    }
}

fn show_shortcut_dialog(
    which: Which,
    shortcut: Arc<Mutex<String>>,
    prompt_shortcut: Arc<Mutex<String>>,
) {
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title(match which {
        Which::Raw => "Voice Dictate: raw shortcut",
        Which::Prompt => "Voice Dictate: prompt shortcut",
    });
    window.set_default_size(420, 140);
    window.set_resizable(false);
    window.set_position(gtk::WindowPosition::Center);
    window.set_keep_above(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let title = gtk::Label::new(Some(match which {
        Which::Raw => "Press a new shortcut for raw dictation",
        Which::Prompt => "Press a new shortcut for prompt dictation",
    }));
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

        let Some((display, _)) = shortcut_from_event(event) else {
            feedback.set_text(
                "That combination is not supported. Use a modifier with a letter or Space, or use F1 to F12.",
            );
            return gtk::glib::Propagation::Stop;
        };

        // Both bindings live in one gsettings array, so the untouched one has to
        // be written back alongside the new one or the compositor drops it.
        let (raw_display, prompt_display) = match which {
            Which::Raw => (display.clone(), prompt_shortcut.lock().unwrap().clone()),
            Which::Prompt => (shortcut.lock().unwrap().clone(), display.clone()),
        };

        match save_shortcuts(&raw_display, &prompt_display, which) {
            Ok(()) => {
                match which {
                    Which::Raw => *shortcut.lock().unwrap() = display,
                    Which::Prompt => *prompt_shortcut.lock().unwrap() = display,
                }
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

/// Turns a display combo ("Ctrl+Alt+Space") into the GNOME accelerator syntax
/// ("<Control><Alt>space").
fn to_accelerator(display: &str) -> String {
    let mut mods = String::new();
    let mut key = String::new();
    for part in display.split('+') {
        match part.trim().to_lowercase().as_str() {
            "" => {}
            "ctrl" | "control" | "cmdorctrl" => mods.push_str("<Control>"),
            "alt" | "option" => mods.push_str("<Alt>"),
            "shift" => mods.push_str("<Shift>"),
            "super" | "meta" | "cmd" | "win" => mods.push_str("<Super>"),
            other => key = other.to_string(),
        }
    }
    format!("{mods}{key}")
}

fn save_shortcuts(raw_display: &str, prompt_display: &str, changed: Which) -> anyhow::Result<()> {
    let settings = "org.gnome.settings-daemon.global-shortcuts.application:/org/gnome/settings-daemon/global-shortcuts/com.daniil.VoiceDictate/";
    let value = format!(
        "[('{SHORTCUT_ID}', {{'shortcuts': <['{}']>, 'description': <'Start/stop voice dictation'>}}), \
          ('{PROMPT_SHORTCUT_ID}', {{'shortcuts': <['{}']>, 'description': <'Start/stop dictation in prompt mode'>}})]",
        to_accelerator(raw_display),
        to_accelerator(prompt_display),
    );
    let status = std::process::Command::new("gsettings")
        .args(["set", settings, "shortcuts", &value])
        .status()?;
    if !status.success() {
        anyhow::bail!("gsettings exited with {status}");
    }
    match changed {
        Which::Raw => config::save_hotkey(raw_display)?,
        Which::Prompt => config::save_prompt_hotkey(prompt_display)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn builds_gnome_accelerators() {
        assert_eq!(super::to_accelerator("Ctrl+Space"), "<Control>space");
        assert_eq!(
            super::to_accelerator("Ctrl+Alt+Space"),
            "<Control><Alt>space"
        );
        assert_eq!(super::to_accelerator("F9"), "f9");
    }
}
