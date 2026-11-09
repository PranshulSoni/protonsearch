//! Bounded teardown/stress probe for the Win32 clipboard listener.
//! Starts and stops the listener several times in one process, then exits
//! normally; used to check listener teardown for leaks/heap corruption.

fn main() {
    let db = std::path::PathBuf::from("target/clipboard-listener-cycles/index.db");
    let images = std::path::PathBuf::from("target/clipboard-listener-cycles/images");
    let _ = std::fs::remove_dir_all("target/clipboard-listener-cycles");
    let mut cycles = 0;
    for cycle in 0..5 {
        let callback_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = callback_count.clone();
        let listener = ps_windows::ClipboardListener::start(move |_snapshot| {
            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        })
        .expect("start listener");
        ps_windows::set_clipboard_text(&format!("listener cycle {cycle}"))
            .expect("set clipboard");
        std::thread::sleep(std::time::Duration::from_millis(80));
        listener.stop();
        cycles += 1;
        println!("cycle {cycle}: ok (callbacks seen: {})", callback_count.load(std::sync::atomic::Ordering::Relaxed));
    }
    // Also exercise double-drop safety: start, drop without stop().
    {
        let listener = ps_windows::ClipboardListener::start(|_| {}).expect("start listener");
        drop(listener);
        println!("drop-without-stop: ok");
    }
    println!("completed {cycles} start/stop cycles + drop path; process exiting normally");
    let _ = std::fs::remove_dir_all("target/clipboard-listener-cycles");
    let _ = db;
    let _ = images;
}
