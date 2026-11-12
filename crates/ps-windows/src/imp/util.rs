//! Small Windows helpers shared by the imp modules.

use std::path::Path;
use windows::core::PWSTR;
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED};

/// RAII guard for a per-thread COM apartment. `apartment_ok` is true when the
/// thread owns an initialisation that must be balanced with CoUninitialize.
pub(crate) struct ComGuard {
    apartment_ok: bool,
}

impl ComGuard {
    pub(crate) fn new() -> Self {
        // S_OK / S_FALSE => we must call CoUninitialize. RPC_E_CHANGED_MODE
        // means the thread already lives in another apartment: usable anyway.
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        Self {
            apartment_ok: hr.is_ok(),
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.apartment_ok {
            unsafe { CoUninitialize() };
        }
    }
}

/// Convert a lossy string into an owned HSTRING for FFI calls.
pub(crate) fn hstr(value: &str) -> windows::core::HSTRING {
    windows::core::HSTRING::from(value)
}

/// Convert a path to an HSTRING (lossy only for unpaired surrogates).
pub(crate) fn hstr_path(path: &Path) -> windows::core::HSTRING {
    hstr(&path.to_string_lossy())
}

/// Free a PWSTR returned through CoTaskMemAlloc-style APIs.
pub(crate) fn free_pwstr(value: PWSTR) {
    if !value.is_null() {
        unsafe { CoTaskMemFree(Some(value.0 as *const _)) };
    }
}

/// Copy a PWSTR into a String without taking ownership.
pub(crate) fn pwstr_to_string(value: PWSTR) -> String {
    if value.is_null() {
        return String::new();
    }
    unsafe { value.to_string().unwrap_or_default() }
}

/// Convert a UTF-16 buffer (NUL padded) to a String.
pub(crate) fn utf16_buf_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}
