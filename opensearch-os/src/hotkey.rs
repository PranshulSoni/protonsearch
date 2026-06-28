use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM, HWND};
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
                "esc" => if is_last { vkey = VK_ESCAPE.0 as u32; },
                "tab" => if is_last { vkey = VK_TAB.0 as u32; },
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
}

static CURRENT_HOTKEY: Lazy<Mutex<Option<HotkeyConfig>>> = Lazy::new(|| Mutex::new(None));
static TARGET_HWND: Lazy<Mutex<Option<isize>>> = Lazy::new(|| Mutex::new(None));
static IS_HOOKED: AtomicBool = AtomicBool::new(false);
static HOOK_HANDLE: Lazy<Mutex<Option<isize>>> = Lazy::new(|| Mutex::new(None));

// For the Settings UI to record a hotkey
static RECORDING_MODE: AtomicBool = AtomicBool::new(false);
static RECORDED_HOTKEY: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub fn set_hotkey_target(hwnd: HWND, config_str: &str) {
    if let Ok(mut g) = TARGET_HWND.lock() {
        *g = Some(hwnd.0 as isize);
    }
    if let Some(parsed) = HotkeyConfig::parse(config_str) {
        if let Ok(mut g) = CURRENT_HOTKEY.lock() {
            *g = Some(parsed);
        }
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
                    if let Ok(mut g) = RECORDED_HOTKEY.lock() {
                        *g = Some(hotkey_str);
                    }
                    RECORDING_MODE.store(false, Ordering::SeqCst);
                    
                    // Consume the keystroke so it doesn't trigger anything else!
                    return LRESULT(1);
                }
                
                // 2. Are we checking for the global launcher hotkey?
                if let Ok(g) = CURRENT_HOTKEY.lock() {
                    if let Some(cfg) = &*g {
                        if cfg.ctrl == ctrl && cfg.alt == alt && cfg.shift == shift && cfg.win == win && cfg.vkey == vkey {
                            // Match! Send message to main window
                            if let Ok(hwnd_g) = TARGET_HWND.lock() {
                                if let Some(hwnd_isize) = *hwnd_g {
                                    let hwnd = HWND(hwnd_isize as *mut std::ffi::c_void);
                                    PostMessageW(hwnd, WM_USER + 4, WPARAM(0), LPARAM(0));
                                    // Block keystroke so windows doesn't process it!
                                    return LRESULT(1); 
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    CallNextHookEx(None, ncode, wparam, lparam)
}

pub fn record_hotkey_blocking() -> String {
    // Spinlock waiting for recording to finish
    RECORDING_MODE.store(true, Ordering::SeqCst);
    if let Ok(mut g) = RECORDED_HOTKEY.lock() {
        *g = None;
    }
    
    loop {
        if !RECORDING_MODE.load(Ordering::SeqCst) {
            if let Ok(g) = RECORDED_HOTKEY.lock() {
                if let Some(s) = &*g {
                    return s.clone();
                }
            }
            break;
        }
        thread::sleep(std::time::Duration::from_millis(50));
    }
    "Alt+Space".to_string()
}
