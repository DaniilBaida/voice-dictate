use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VK_CONTROL, VK_SHIFT, VK_V,
};

pub fn paste(shortcut: &str) -> anyhow::Result<bool> {
    let mut inputs = vec![make_key(VK_CONTROL.0, KEYBD_EVENT_FLAGS(0))];
    if shortcut.eq_ignore_ascii_case("Ctrl+Shift+V") {
        inputs.push(make_key(VK_SHIFT.0, KEYBD_EVENT_FLAGS(0)));
    }
    inputs.push(make_key(VK_V.0, KEYBD_EVENT_FLAGS(0)));
    inputs.push(make_key(VK_V.0, KEYEVENTF_KEYUP));
    if shortcut.eq_ignore_ascii_case("Ctrl+Shift+V") {
        inputs.push(make_key(VK_SHIFT.0, KEYEVENTF_KEYUP));
    }
    inputs.push(make_key(VK_CONTROL.0, KEYEVENTF_KEYUP));

    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(true)
}

fn make_key(vk: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
