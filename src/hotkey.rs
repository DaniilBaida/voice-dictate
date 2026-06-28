//! Shared hotkey string parsing. Accepts combos like "Ctrl+Alt+D" or "F9".

use global_hotkey::hotkey::{Code, HotKey, Modifiers};

pub fn parse(s: &str) -> anyhow::Result<HotKey> {
    let mut mods = Modifiers::empty();
    let mut key_code = None;

    for part in s.split('+') {
        match part.trim().to_lowercase().as_str() {
            "" => {}
            "cmdorctrl" | "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            "super" | "meta" | "cmd" | "win" => mods |= Modifiers::SUPER,
            other => key_code = Some(parse_key_code(other)?),
        }
    }

    let code = key_code.ok_or_else(|| anyhow::anyhow!("no key in hotkey: {s}"))?;
    Ok(HotKey::new(Some(mods), code))
}

fn parse_key_code(s: &str) -> anyhow::Result<Code> {
    match s.to_lowercase().as_str() {
        "space" => Ok(Code::Space),
        "a" => Ok(Code::KeyA), "b" => Ok(Code::KeyB), "c" => Ok(Code::KeyC),
        "d" => Ok(Code::KeyD), "e" => Ok(Code::KeyE), "f" => Ok(Code::KeyF),
        "g" => Ok(Code::KeyG), "h" => Ok(Code::KeyH), "i" => Ok(Code::KeyI),
        "j" => Ok(Code::KeyJ), "k" => Ok(Code::KeyK), "l" => Ok(Code::KeyL),
        "m" => Ok(Code::KeyM), "n" => Ok(Code::KeyN), "o" => Ok(Code::KeyO),
        "p" => Ok(Code::KeyP), "q" => Ok(Code::KeyQ), "r" => Ok(Code::KeyR),
        "s" => Ok(Code::KeyS), "t" => Ok(Code::KeyT), "u" => Ok(Code::KeyU),
        "v" => Ok(Code::KeyV), "w" => Ok(Code::KeyW), "x" => Ok(Code::KeyX),
        "y" => Ok(Code::KeyY), "z" => Ok(Code::KeyZ),
        "f1"  => Ok(Code::F1),  "f2"  => Ok(Code::F2),  "f3"  => Ok(Code::F3),
        "f4"  => Ok(Code::F4),  "f5"  => Ok(Code::F5),  "f6"  => Ok(Code::F6),
        "f7"  => Ok(Code::F7),  "f8"  => Ok(Code::F8),  "f9"  => Ok(Code::F9),
        "f10" => Ok(Code::F10), "f11" => Ok(Code::F11), "f12" => Ok(Code::F12),
        other => anyhow::bail!("unknown key: {other}"),
    }
}
