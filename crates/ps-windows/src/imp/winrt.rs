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
        windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED)
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

/// Run WinRT work on one persistent MTA thread, bounded by a timeout.
///
/// WinRT cannot be safely torn down and re-initialised on short-lived threads
/// (see radios.rs), and a blocking `.get()` on an async operation must never run
/// on a UI thread, so the media and notification adapters all funnel through here.
pub(crate) fn run<T: Send + 'static>(
    job: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> anyhow::Result<T> {
    use std::sync::mpsc;
    type Job = Box<dyn FnOnce() + Send>;
    static WORKER: std::sync::OnceLock<Option<mpsc::Sender<Job>>> = std::sync::OnceLock::new();
    let worker = WORKER
        .get_or_init(|| {
            let (sender, receiver) = mpsc::channel::<Job>();
            std::thread::Builder::new()
                .name("ps-winrt".into())
                .spawn(move || {
                    // Persistent MTA apartment for the whole process lifetime.
                    let _owned = ensure_apartment();
                    while let Ok(job) = receiver.recv() {
                        job();
                    }
                })
                .ok()
                .map(|_| sender)
        })
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("could not start the WinRT worker"))?;
    let (reply, outcome) = mpsc::channel();
    worker
        .send(Box::new(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
                .unwrap_or_else(|_| Err(anyhow::anyhow!("WinRT operation panicked")));
            let _ = reply.send(result);
        }))
        .map_err(|_| anyhow::anyhow!("the WinRT worker stopped"))?;
    outcome
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| anyhow::anyhow!("the WinRT operation timed out"))?
}
