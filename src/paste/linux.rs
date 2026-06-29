/// Linux paste: detect X11 vs Wayland at runtime, use the right injection method.
/// Returns Ok(true) if a keystroke was injected, Ok(false) if only clipboard is set
/// (caller should notify the user to paste manually).
pub async fn paste() -> anyhow::Result<bool> {
    if is_wayland() {
        paste_wayland().await
    } else {
        paste_x11()
    }
}

fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

#[cfg(feature = "x11")]
fn paste_x11() -> anyhow::Result<bool> {
    use x11rb::{
        connection::Connection,
        protocol::xtest::ConnectionExt as XTestExt,
        rust_connection::RustConnection,
    };

    // Key codes: 37 = Left Ctrl, 55 = V (may vary; we use keysym lookup)
    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    // keysym 0x0063 = 'v', 0xffe3 = Control_L
    let ctrl_keycode = keysym_to_keycode(&conn, 0xffe3)?;
    let v_keycode = keysym_to_keycode(&conn, 0x0076)?;

    // Press Ctrl
    conn.xtest_fake_input(6, ctrl_keycode, 0, root, 0, 0, 0)?;
    // Press V
    conn.xtest_fake_input(6, v_keycode, 0, root, 0, 0, 0)?;
    // Release V
    conn.xtest_fake_input(7, v_keycode, 0, root, 0, 0, 0)?;
    // Release Ctrl
    conn.xtest_fake_input(7, ctrl_keycode, 0, root, 0, 0, 0)?;
    conn.flush()?;

    Ok(true)
}

#[cfg(not(feature = "x11"))]
fn paste_x11() -> anyhow::Result<bool> {
    tracing::warn!("built without x11 feature; cannot inject paste on X11");
    Ok(false)
}

#[cfg(feature = "x11")]
fn keysym_to_keycode(conn: &x11rb::rust_connection::RustConnection, keysym: u32) -> anyhow::Result<u8> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::ConnectionExt;
    let setup = conn.setup();
    let min_kc = setup.min_keycode;
    let max_kc = setup.max_keycode;

    let mapping = conn
        .get_keyboard_mapping(min_kc, max_kc - min_kc + 1)?
        .reply()?;
    let ks_per_kc = mapping.keysyms_per_keycode as usize;

    for (i, chunk) in mapping.keysyms.chunks(ks_per_kc).enumerate() {
        if chunk.contains(&keysym) {
            return Ok(min_kc + i as u8);
        }
    }
    anyhow::bail!("keysym 0x{keysym:x} not found in keyboard mapping")
}

/// Default socket path created by the `ydotoold` system service (see install.sh).
const DEFAULT_YDOTOOL_SOCKET: &str = "/run/ydotool.socket";

/// Wayland auto-paste via ydotool (kernel-level uinput injection).
///
/// This works on any compositor, including GNOME, where the virtual-keyboard
/// Wayland protocol and X11 key injection cannot reach focused windows. It is
/// invisible to the user: no portal dialog and no "your screen is being
/// controlled" indicator. Requires the `ydotoold` daemon running with a socket
/// the user can reach. evdev keycodes: Left Ctrl = 29, V = 47.
async fn paste_wayland() -> anyhow::Result<bool> {
    // Let the clipboard owner settle before pasting.
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;

    let mut cmd = std::process::Command::new("ydotool");
    if std::env::var_os("YDOTOOL_SOCKET").is_none() {
        cmd.env("YDOTOOL_SOCKET", DEFAULT_YDOTOOL_SOCKET);
    }
    cmd.args(["key", "29:1", "47:1", "47:0", "29:0"]);

    match cmd.status() {
        Ok(s) if s.success() => Ok(true),
        Ok(s) => {
            tracing::warn!("ydotool exited with {s}; is the ydotoold service running?");
            Ok(false)
        }
        Err(e) => {
            tracing::warn!("could not run ydotool ({e}); falling back to manual paste");
            Ok(false)
        }
    }
}
