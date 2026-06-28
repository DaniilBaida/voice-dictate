#[cfg(windows)]
pub fn recording_start() {
    // High tone = start
    std::thread::spawn(|| unsafe {
        let _ = windows::Win32::System::Diagnostics::Debug::Beep(880, 80);
    });
}

#[cfg(windows)]
pub fn recording_stop() {
    // Lower tone = stop
    std::thread::spawn(|| unsafe {
        let _ = windows::Win32::System::Diagnostics::Debug::Beep(660, 80);
    });
}

#[cfg(target_os = "linux")]
pub fn recording_start() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("paplay")
            .arg("/usr/share/sounds/freedesktop/stereo/message.oga")
            .output();
    });
}

#[cfg(target_os = "linux")]
pub fn recording_stop() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("paplay")
            .arg("/usr/share/sounds/freedesktop/stereo/message-new-instant.oga")
            .output();
    });
}
