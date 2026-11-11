//! Windows-specific implementation of the ps-windows helpers.
//!
//! Design rules:
//! - No `cmd.exe` / `powershell` string building anywhere: every argument is a
//!   structured Win32/COM call or a fixed executable + argv list.
//! - COM is initialised per call on the calling thread (STA) and balanced.
//! - Caches (icons) are written atomically (temp file + rename).

pub(crate) mod audio;
pub(crate) mod bluetooth;
pub(crate) mod brightness;
pub(crate) mod capabilities;
pub(crate) mod clipboard;
pub(crate) mod icons;
pub(crate) mod media;
pub(crate) mod native;
pub(crate) mod notifications;
pub(crate) mod radios;
pub(crate) mod settings_pages;
pub(crate) mod shell;
pub(crate) mod toast;
pub(crate) mod util;
pub(crate) mod wifi;
pub(crate) mod window;
pub(crate) mod winrt;
