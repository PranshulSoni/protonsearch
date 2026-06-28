use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

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
        if s.is_empty() {
            return None;
        }
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut win = false;
        let mut vkey = 0;

        let parts: Vec<String> = s.split('+').map(|p| p.trim().to_lowercase()).collect();
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            match part.as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                "win" | "meta" => win = true,
                key if is_last => vkey = key_to_vkey(key)?,
                _ => {}
            }
        }
        (vkey != 0).then_some(Self { ctrl, alt, shift, win, vkey })
    }

    pub fn modifiers(&self) -> HOT_KEY_MODIFIERS {
        let mut modifiers = HOT_KEY_MODIFIERS(0);
        if self.alt {
            modifiers |= MOD_ALT;
        }
        if self.ctrl {
            modifiers |= MOD_CONTROL;
        }
        if self.shift {
            modifiers |= MOD_SHIFT;
        }
        if self.win {
            modifiers |= MOD_WIN;
        }
        modifiers | MOD_NOREPEAT
    }
}

pub fn register_hotkey(hwnd: HWND, id: i32, config_str: &str) -> bool {
    unsafe {
        let _ = UnregisterHotKey(hwnd, id);
        let Some(cfg) = HotkeyConfig::parse(config_str) else {
            return false;
        };
        RegisterHotKey(hwnd, id, cfg.modifiers(), cfg.vkey).is_ok()
    }
}

pub fn hotkey_available(config_str: &str) -> bool {
    unsafe {
        let Some(cfg) = HotkeyConfig::parse(config_str) else {
            return false;
        };
        if RegisterHotKey(HWND(std::ptr::null_mut()), 9999, cfg.modifiers(), cfg.vkey).is_err() {
            return false;
        }
        let _ = UnregisterHotKey(HWND(std::ptr::null_mut()), 9999);
        true
    }
}

pub fn format_recorded_hotkey(
    key_text: &str,
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
) -> Option<String> {
    let key = normalize_slint_key(key_text)?;
    let lower = key.to_lowercase();
    if matches!(lower.as_str(), "ctrl" | "control" | "alt" | "shift" | "win" | "meta") {
        return None;
    }

    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    if win {
        parts.push("Win");
    }
    parts.push(key.as_str());
    Some(parts.join("+"))
}

fn normalize_slint_key(key_text: &str) -> Option<String> {
    let key = match key_text {
        " " => "Space".to_string(),
        "\n" | "\r" => "Enter".to_string(),
        "\t" => "Tab".to_string(),
        other => {
            let trimmed = other.trim();
            if trimmed.is_empty() {
                return None;
            }
            match trimmed {
                "Esc" => "Escape".to_string(),
                key if key.len() == 1 => key.to_ascii_uppercase(),
                key => key.to_string(),
            }
        }
    };
    key_to_vkey(&key.to_lowercase()).map(|_| key)
}

fn key_to_vkey(key: &str) -> Option<u32> {
    Some(match key {
        "space" => VK_SPACE.0 as u32,
        "enter" | "return" => VK_RETURN.0 as u32,
        "esc" | "escape" => VK_ESCAPE.0 as u32,
        "tab" => VK_TAB.0 as u32,
        "up" => VK_UP.0 as u32,
        "down" => VK_DOWN.0 as u32,
        "left" => VK_LEFT.0 as u32,
        "right" => VK_RIGHT.0 as u32,
        key if key.starts_with('f') => {
            let n = key[1..].parse::<u32>().ok()?;
            if !(1..=24).contains(&n) {
                return None;
            }
            VK_F1.0 as u32 + n - 1
        }
        key if key.len() == 1 => {
            let c = key.chars().next()?;
            if c.is_ascii_alphabetic() {
                c.to_ascii_uppercase() as u32
            } else if c.is_ascii_digit() {
                c as u32
            } else {
                return None;
            }
        }
        _ => return None,
    })
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

    #[test]
    fn formats_slint_key_capture() {
        assert_eq!(
            format_recorded_hotkey("k", true, true, false, false).as_deref(),
            Some("Ctrl+Alt+K")
        );
        assert_eq!(
            format_recorded_hotkey(" ", false, true, false, false).as_deref(),
            Some("Alt+Space")
        );
        assert!(format_recorded_hotkey("Shift", false, false, true, false).is_none());
    }
}
