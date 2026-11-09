//! Windows-specific implementation of the ps-windows helpers.
//!
//! Design rules:
//! - No `cmd.exe` / `powershell` string building anywhere: every argument is a
//!   structured Win32/COM call or a fixed executable + argv list.
//! - COM is initialised per call on the calling thread (STA) and balanced.
//! - Caches (icons) are written atomically (temp file + rename).

pub(crate) mod clipboard;
pub(crate) mod icons;
pub(crate) mod shell;
pub(crate) mod util;
pub(crate) mod window;
