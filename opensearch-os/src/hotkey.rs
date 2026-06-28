use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use once_cell::sync::Lazy;
use std::thread;

// Represents a parsed hotkey
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyConfig {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub vkey: u32,
}

impl HotkeyConfig {
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() { return None; }
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut win = false;
        let mut vkey = 0;
        
        let parts: Vec<String> = s.split('+').map(|p| p.trim().to_lowercase()).collect();
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            match part.as_str() {
                "ctrl" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                "win" => win = true,
                "space" => if is_last { vkey = VK_SPACE.0 as u32; },
                "enter" => if is_last { vkey = VK_RETURN.0 as u32; },
                "esc" | "escape" => if is_last { vkey = VK_ESCAPE.0 as u32; },
                "tab" => if is_last { vkey = VK_TAB.0 as u32; },
                "up" => if is_last { vkey = VK_UP.0 as u32; },
                "down" => if is_last { vkey = VK_DOWN.0 as u32; },
                "left" => if is_last { vkey = VK_LEFT.0 as u32; },
                "right" => if is_last { vkey = VK_RIGHT.0 as u32; },
                k if is_last && k.starts_with('f') => {
                    if let Ok(n) = k[1..].parse::<u32>() {
                        if (1..=24).contains(&n) {
                            vkey = VK_F1.0 as u32 + n - 1;
                        }
                    }
                }
                k if k.len() == 1 => {
                    let c = k.chars().next().unwrap();
                    if c.is_ascii_alphabetic() {
                        vkey = c.to_ascii_uppercase() as u32;
                    } else if c.is_ascii_digit() {
                        vkey = c as u32;
                    }
                }
                _ => {} // Ignore unknown
            }
        }
        if vkey == 0 { return None; }
        Some(Self { ctrl, alt, shift, win, vkey })
    }

    pub fn modifiers(&self) -> HOT_KEY_MODIFIERS {
        let mut modifiers = HOT_KEY_MODIFIERS(0);
        if self.alt { modifiers |= MOD_ALT; }
        if self.ctrl { modifiers |= MOD_CONTROL; }
        if self.shift { modifiers |= MOD_SHIFT; }
        if self.win { modifiers |= MOD_WIN; }
        modifiers | MOD_NOREPEAT
    }
}

static IS_HOOKED: AtomicBool = AtomicBool::new(false);
static HOOK_HANDLE: Lazy<Mutex<Option<isize>>> = Lazy::new(|| Mutex::new(None));

// For the Settings UI to record a hotkey - use channel for instant notification
static RECORDING_MODE: AtomicBool = AtomicBool::new(false);
static RECORDING_SENDER: Lazy<Mutex<Option<mpsc::SyncSender<String>>>> = Lazy::new(|| Mutex::new(None));

pub fn register_hotkey(hwnd: HWND, id: i32, config_str: &str) -> bool {
    unsafe {
        let _ = UnregisterHotKey(hwnd, id);
        let Some(cfg) = HotkeyConfig::parse(config_str) else {
            return false;
        };
        RegisterHotKey(hwnd, id, cfg.modifiers(), cfg.vkey).is_ok()
    }
}

pub fn start_hook() {
    if IS_HOOKED.swap(true, Ordering::SeqCst) {
        return; // already hooked
    }
    thread::spawn(|| {
        unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                None,
                0,
            );
            if let Ok(hook) = hook {
                if let Ok(mut g) = HOOK_HANDLE.lock() {
                    *g = Some(hook.0 as isize);
                }
                
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).into() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            IS_HOOKED.store(false, Ordering::SeqCst);
        }
    });
}

unsafe extern "system" fn keyboard_hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode == HC_ACTION as i32 {
        let msg = wparam.0 as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vkey = kb.vkCode;
            
            // Ignore bare modifier presses
            if vkey != VK_LCONTROL.0 as u32 && vkey != VK_RCONTROL.0 as u32 
                && vkey != VK_LSHIFT.0 as u32 && vkey != VK_RSHIFT.0 as u32 
                && vkey != VK_LMENU.0 as u32 && vkey != VK_RMENU.0 as u32 
                && vkey != VK_LWIN.0 as u32 && vkey != VK_RWIN.0 as u32 {
                
                let ctrl = (GetAsyncKeyState(VK_CONTROL.0 as i32) as i16) < 0;
                let alt = (GetAsyncKeyState(VK_MENU.0 as i32) as i16) < 0;
                let shift = (GetAsyncKeyState(VK_SHIFT.0 as i32) as i16) < 0;
                let win = (GetAsyncKeyState(VK_LWIN.0 as i32) as i16) < 0 || (GetAsyncKeyState(VK_RWIN.0 as i32) as i16) < 0;

                // 1. Are we in recording mode for the settings UI?
                if RECORDING_MODE.load(Ordering::SeqCst) {
                    let mut parts = Vec::new();
                    if ctrl { parts.push("Ctrl"); }
                    if alt { parts.push("Alt"); }
                    if shift { parts.push("Shift"); }
                    if win { parts.push("Win"); }
                    
                    let key_str = match vkey {
                        0x20 => "Space".to_string(),
                        0x0D => "Enter".to_string(),
                        0x1B => "Esc".to_string(),
                        0x09 => "Tab".to_string(),
                        k if (0x30..=0x39).contains(&k) || (0x41..=0x5A).contains(&k) => {
                            (k as u8 as char).to_string()
                        }
                        _ => format!("Key{}", vkey),
                    };
                    parts.push(&key_str);
                    
                    let hotkey_str = parts.join("+");
                    RECORDING_MODE.store(false, Ordering::SeqCst);
                    // Send instantly via channel - no spinloop needed
                    if let Ok(guard) = RECORDING_SENDER.lock() {
                        if let Some(sender) = guard.as_ref() {
                            let _ = sender.try_send(hotkey_str);
                        }
                    }
                    
                    // Consume the keystroke so it doesn't trigger anything else!
                    return LRESULT(1);
                }
                
            }
        }
    }
    
    CallNextHookEx(None, ncode, wparam, lparam)
}

pub fn record_hotkey_blocking() -> Option<String> {
    // Create a sync channel - receives instantly when hook fires
    let (tx, rx) = mpsc::sync_channel::<String>(1);
    {
        if let Ok(mut guard) = RECORDING_SENDER.lock() {
            *guard = Some(tx);
        }
    }
    RECORDING_MODE.store(true, Ordering::SeqCst);
    
    // Block until the key is pressed (channel recv is instant, no spinloop)
    let key_str = match rx.recv_timeout(std::time::Duration::from_secs(10)) {
        Ok(s) => s,
        Err(_) => {
            // Timed out or cancelled
            RECORDING_MODE.store(false, Ordering::SeqCst);
            return None;
        }
    };
    
    // Clean up the sender
    if let Ok(mut guard) = RECORDING_SENDER.lock() {
        *guard = None;
    }
    
    // Check for hotkey conflicts via RegisterHotKey
    if let Some(cfg) = HotkeyConfig::parse(&key_str) {
        let mut modifiers = HOT_KEY_MODIFIERS(0);
        if cfg.alt { modifiers |= MOD_ALT; }
        if cfg.ctrl { modifiers |= MOD_CONTROL; }
        if cfg.shift { modifiers |= MOD_SHIFT; }
        if cfg.win { modifiers |= MOD_WIN; }
        modifiers |= MOD_NOREPEAT;
        
        unsafe {
            let test_id = 9999i32;
            if RegisterHotKey(HWND(std::ptr::null_mut()), test_id, modifiers, cfg.vkey).is_err() {
                // Conflict - show dialog
                let msg: Vec<u16> = "This hotkey is already in use by Windows or another app.\n\nDo you want OmniSearch to forcefully override it?\n(Choosing Yes will hijack the hotkey globally)"
                    .encode_utf16().chain(std::iter::once(0)).collect();
                let title: Vec<u16> = "Hotkey Conflict\0".encode_utf16().collect();
                let res = MessageBoxW(
                    HWND(std::ptr::null_mut()),
                    windows::core::PCWSTR(msg.as_ptr()),
                    windows::core::PCWSTR(title.as_ptr()),
                    MB_YESNO | MB_ICONWARNING
                );
                if res.0 == 6 /* IDYES */ {
                    return Some(key_str);
                } else {
                    return None;
                }
            } else {
                let _ = UnregisterHotKey(HWND(std::ptr::null_mut()), test_id);
            }
        }
    }
    Some(key_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_launcher_hotkeys() {
        let alt_space = HotkeyConfig::parse("Alt+Space").unwrap();
        assert!(alt_space.alt);
        assert!(!alt_space.ctrl);
        assert_eq!(alt_space.vkey, VK_SPACE.0 as u32);

        let ctrl_shift_f12 = HotkeyConfig::parse("Ctrl + Shift + F12").unwrap();
        assert!(ctrl_shift_f12.ctrl);
        assert!(ctrl_shift_f12.shift);
        assert_eq!(ctrl_shift_f12.vkey, VK_F12.0 as u32);
    }

    #[test]
    fn builds_register_hotkey_modifiers() {
        let cfg = HotkeyConfig::parse("Ctrl+Alt+K").unwrap();
        let modifiers = cfg.modifiers();
        assert_ne!(modifiers & MOD_CONTROL, HOT_KEY_MODIFIERS(0));
        assert_ne!(modifiers & MOD_ALT, HOT_KEY_MODIFIERS(0));
        assert_ne!(modifiers & MOD_NOREPEAT, HOT_KEY_MODIFIERS(0));
        assert_eq!(cfg.vkey, b'K' as u32);
    }
}
