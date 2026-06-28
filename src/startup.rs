use std::path::PathBuf;

#[cfg(windows)]
fn startup_cmd() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata)
        .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup")
        .join("voice-dictate.cmd")
}

#[cfg(windows)]
pub fn install() {
    let path = startup_cmd();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(exe) = std::env::current_exe() {
        // Use a VBS wrapper so no console window appears at login
        let vbs = path.with_extension("vbs");
        let content = format!(
            "CreateObject(\"WScript.Shell\").Run \"\"\"{}\"\"\", 0, False\r\n",
            exe.display()
        );
        let _ = std::fs::write(&vbs, content);
        // Write a minimal cmd that delegates to the vbs (keeps the .cmd extension recognisable)
        let cmd = format!("@echo off\r\nwscript.exe //B \"{}\"\r\n", vbs.display());
        let _ = std::fs::write(&path, cmd);
    }
}

#[cfg(windows)]
pub fn uninstall() {
    let path = startup_cmd();
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("vbs"));
}

#[cfg(windows)]
pub fn is_installed() -> bool {
    startup_cmd().exists()
}

// ── Linux ──────────────────────────────────────────────────────────────────────
#[cfg(target_os = "linux")]
fn autostart_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config/autostart/voice-dictate.desktop")
}

#[cfg(target_os = "linux")]
pub fn install() {
    let target = autostart_file();
    if let Some(dir) = target.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(exe) = std::env::current_exe() {
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Voice Dictate\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            exe.display()
        );
        let _ = std::fs::write(target, content);
    }
}

#[cfg(target_os = "linux")]
pub fn uninstall() {
    let _ = std::fs::remove_file(autostart_file());
}

#[cfg(target_os = "linux")]
pub fn is_installed() -> bool {
    autostart_file().exists()
}
