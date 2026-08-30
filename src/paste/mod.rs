/// Inject the configured paste keystroke into the currently focused window.
/// Returns Ok(false) when only the clipboard is updated.
pub async fn paste(shortcut: &str) -> anyhow::Result<bool> {
    #[cfg(windows)]
    return windows::paste(shortcut);

    #[cfg(target_os = "linux")]
    return linux::paste(shortcut).await;

    #[cfg(not(any(windows, target_os = "linux")))]
    return Ok(false);
}

#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod linux;
