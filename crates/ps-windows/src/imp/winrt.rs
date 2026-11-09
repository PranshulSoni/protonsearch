//! Minimal WinRT apartment helper shared by the native adapters.

thread_local! {
    static APARTMENT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Initialise WinRT on this thread (MTA). Returns true when this thread owns the
/// initialisation (caller must balance with RoUninitialize when done).
pub(crate) fn ensure_apartment() -> bool {
    if APARTMENT.with(|cell| cell.get()) {
        return false;
    }
    match unsafe {
        windows::Win32::System::WinRT::RoInitialize(
            windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
        )
    } {
        Ok(()) => {
            APARTMENT.with(|cell| cell.set(true));
            true
        }
        // RPC_E_CHANGED_MODE: already initialised by someone else.
        Err(_) => false,
    }
}

pub(crate) fn release_apartment(owned: bool) {
    if owned {
        unsafe { windows::Win32::System::WinRT::RoUninitialize() };
        APARTMENT.with(|cell| cell.set(false));
    }
}
