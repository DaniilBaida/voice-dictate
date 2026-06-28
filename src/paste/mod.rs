/// Inject a Ctrl+V keystroke into the currently focused window.
/// On Wayland without portal support, returns Ok(false) to signal
/// "clipboard is ready, paste manually". The caller shows a notification.
pub async fn paste() -> anyhow::Result<bool> {
    #[cfg(windows)]
    return windows::paste();

    #[cfg(target_os = "linux")]
    return linux::paste().await;

    #[cfg(not(any(windows, target_os = "linux")))]
    return Ok(false);
}

#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod linux;
