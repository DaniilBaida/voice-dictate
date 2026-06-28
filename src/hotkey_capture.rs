//! Windows-only: capture an arbitrary key combination via a low-level keyboard
//! hook. While capturing, the next valid chord (one or more modifiers + a key,
//! or a function key alone) is recorded and returned as a config string like
//! "Ctrl+Alt+D". Esc cancels; a timeout protects against keyboard lockout.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

static CAPTURING: AtomicBool = AtomicBool::new(false);
static CANCELLED: AtomicBool = AtomicBool::new(false);
static HOOK: AtomicIsize = AtomicIsize::new(0);
static RESULT: Mutex<Option<String>> = Mutex::new(None);
static START: Mutex<Option<Instant>> = Mutex::new(None);

const TIMEOUT: Duration = Duration::from_secs(5);

pub enum Poll {
    Idle,
    Pending,
    Captured(String),
    Cancelled,
}

pub fn begin() {
    if CAPTURING.load(Ordering::SeqCst) {
        return;
    }
    *RESULT.lock().unwrap() = None;
    CANCELLED.store(false, Ordering::SeqCst);
    *START.lock().unwrap() = Some(Instant::now());

    unsafe {
        match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0) {
            Ok(h) => {
                HOOK.store(h.0 as isize, Ordering::SeqCst);
                CAPTURING.store(true, Ordering::SeqCst);
            }
            Err(e) => tracing::error!("failed to install keyboard hook: {e}"),
        }
    }
}

pub fn poll() -> Poll {
    if !CAPTURING.load(Ordering::SeqCst) {
        return Poll::Idle;
    }

    if let Some(combo) = RESULT.lock().unwrap().take() {
        end();
        return Poll::Captured(combo);
    }
    if CANCELLED.load(Ordering::SeqCst) {
        end();
        return Poll::Cancelled;
    }
    let started = START.lock().unwrap().unwrap_or_else(Instant::now);
    if started.elapsed() > TIMEOUT {
        end();
        return Poll::Cancelled;
    }
    Poll::Pending
}

fn end() {
    CAPTURING.store(false, Ordering::SeqCst);
    let raw = HOOK.swap(0, Ordering::SeqCst);
    if raw != 0 {
        unsafe {
            let _ = UnhookWindowsHookEx(HHOOK(raw as *mut _));
        }
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && CAPTURING.load(Ordering::SeqCst) {
        let msg = wparam.0 as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk = kb.vkCode as u16;

            if vk == 0x1B {
                // Escape cancels
                CANCELLED.store(true, Ordering::SeqCst);
                return LRESULT(1);
            }

            if is_modifier(vk) {
                // Swallow modifiers, keep waiting for the main key
                return LRESULT(1);
            }

            if let Some(key) = vk_to_name(vk) {
                let mods = held_modifiers();
                let is_fn = key.starts_with('F') && key.len() > 1;
                if !mods.is_empty() || is_fn {
                    let mut combo = mods;
                    combo.push_str(&key);
                    *RESULT.lock().unwrap() = Some(combo);
                    return LRESULT(1);
                }
            }
            // Unsupported or bare key: swallow and keep waiting
            return LRESULT(1);
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn is_modifier(vk: u16) -> bool {
    matches!(
        vk,
        0x10 | 0xA0 | 0xA1 | // Shift, LShift, RShift
        0x11 | 0xA2 | 0xA3 | // Ctrl, LCtrl, RCtrl
        0x12 | 0xA4 | 0xA5 | // Alt, LAlt, RAlt
        0x5B | 0x5C          // LWin, RWin
    )
}

/// Builds the modifier prefix ("Ctrl+Alt+") from currently-held keys.
fn held_modifiers() -> String {
    let mut s = String::new();
    unsafe {
        if down(0x11) {
            s.push_str("Ctrl+");
        }
        if down(0x12) {
            s.push_str("Alt+");
        }
        if down(0x10) {
            s.push_str("Shift+");
        }
        if down(0x5B) || down(0x5C) {
            s.push_str("Super+");
        }
    }
    s
}

unsafe fn down(vk: i32) -> bool {
    (GetAsyncKeyState(vk) as u16 & 0x8000) != 0
}

fn vk_to_name(vk: u16) -> Option<String> {
    match vk {
        0x41..=0x5A => Some(((vk as u8) as char).to_string()), // A-Z
        0x20 => Some("Space".to_string()),
        0x70..=0x7B => Some(format!("F{}", vk - 0x6F)),        // F1-F12
        _ => None,
    }
}
