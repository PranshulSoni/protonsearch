use windows::{
    core::HSTRING,
    Media::SpeechRecognition::{
        SpeechRecognitionResultStatus,
        SpeechRecognitionScenario,
        SpeechRecognitionTopicConstraint,
        SpeechRecognizer,
    },
    Win32::Foundation::{HWND, LPARAM, WPARAM},
    Win32::UI::WindowsAndMessaging::PostMessageW,
};

extern crate windows_core;

pub const WM_VOICE_QUERY_READY: u32 = 0x0400 + 101;

#[derive(Clone, Copy)]
struct HwndPtr(usize);
unsafe impl Send for HwndPtr {}
unsafe impl Sync for HwndPtr {}
impl HwndPtr {
    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut std::ffi::c_void)
    }
}

const QUERY_RETRY_DELAY_MS: u64 = 450;
const QUERY_ATTEMPTS: usize = 2;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn log_voice(msg: String) {
    // Always log next to the exe (cwd varies by how the app was launched).
    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("voice_log.txt")))
        .unwrap_or_else(|| std::path::PathBuf::from("voice_log.txt"));
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 1024 * 1024 { let _ = std::fs::remove_file(&path); }
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = writeln!(file, "[{}] {}", now_ms(), msg);
    }
}

/// Diagnostic line into voice_log.txt (used by main.rs for hotkey registration).
pub fn log(msg: &str) {
    log_voice(msg.to_string());
}

// ── Pre-warmed one-shot dictation, triggered by hotkey or mic button ──────────
//
// A single recognizer is built and compiled ONCE on a persistent worker thread,
// then reused for every trigger. Compiling costs ~0.5–1s; paying it per-press made
// RecognizeAsync start late and miss the front of the phrase (it would hear only the
// trailing "for me"). The mic still opens only during a triggered RecognizeAsync —
// nothing listens in the background.
static TRIGGER: std::sync::OnceLock<std::sync::mpsc::SyncSender<()>> = std::sync::OnceLock::new();

/// Trigger one dictation. First call spawns the persistent (pre-warmed) worker;
/// later calls just wake it.
pub fn start_query_listener(hwnd: HWND) {
    let tx = TRIGGER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::sync_channel::<()>(8);
        let h = HwndPtr(hwnd.0 as usize);
        std::thread::spawn(move || dictation_worker(h, rx));
        tx
    });
    let _ = tx.try_send(());
}

fn dictation_worker(h: HwndPtr, rx: std::sync::mpsc::Receiver<()>) {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
    }

    // We do NOT pre-warm at startup to prevent system-wide cursor flickering in the background.
    // Instead, the recognizer is built on-demand when triggered and dropped immediately after.

    while rx.recv().is_ok() {
        // Collapse any double-press into one dictation.
        while rx.try_recv().is_ok() {}

        log_voice("query: building dictation recognizer on-demand".into());
        let recognizer = build_recognizer();

        let mut text = None;
        for attempt in 1..=QUERY_ATTEMPTS {
            let Some(r) = recognizer.as_ref() else { break };
            log_voice(format!("query: RecognizeAsync (attempt={attempt})"));
            text = run_recognition(r);
            if text.is_some() {
                break;
            }
            if attempt < QUERY_ATTEMPTS {
                log_voice("query: retry after empty/failure".into());
                std::thread::sleep(std::time::Duration::from_millis(QUERY_RETRY_DELAY_MS));
            }
        }

        // Drop the recognizer immediately to release system speech services
        // and prevent cursor flickering/blink issues.
        drop(recognizer);

        post_query(h, text);
    }
}

fn post_query(h: HwndPtr, text: Option<String>) {
    unsafe {
        match text {
            Some(t) if !t.trim().is_empty() => {
                let q = crate::search::clean_prompt(&t).1;
                log_voice(format!("query: '{}' → '{}'", t, q));
                let ptr = Box::into_raw(Box::new(q)) as isize;
                let _ = PostMessageW(h.hwnd(), WM_VOICE_QUERY_READY, WPARAM(1), LPARAM(ptr));
            }
            _ => {
                let _ = PostMessageW(h.hwnd(), WM_VOICE_QUERY_READY, WPARAM(0), LPARAM(0));
            }
        }
    }
}

/// Build + compile a dictation recognizer (the slow part — done once, kept hot).
fn build_recognizer() -> Option<SpeechRecognizer> {
    let recognizer = SpeechRecognizer::new().ok()?;

    // Bound the initial-silence wait so RecognizeAsync always returns (never hangs).
    if let Ok(timeouts) = recognizer.Timeouts() {
        let eight_s = windows::Foundation::TimeSpan { Duration: 8 * 10_000_000 };
        let _ = timeouts.SetInitialSilenceTimeout(eight_s);
    }

    let constraint = SpeechRecognitionTopicConstraint::Create(
        SpeechRecognitionScenario::Dictation,
        &HSTRING::from("dictation"),
    ).ok()?;
    recognizer.Constraints().ok()?.Append(&constraint).ok()?;
    recognizer.CompileConstraintsAsync().ok()?.get().ok()?;
    Some(recognizer)
}

/// One RecognizeAsync on an already-compiled recognizer (starts ~instantly).
fn run_recognition(recognizer: &SpeechRecognizer) -> Option<String> {
    let result = recognizer.RecognizeAsync().ok()?.get().ok()?;
    let status = result.Status().ok()?;
    log_voice(format!("query: result status={:?}", status));
    if status == SpeechRecognitionResultStatus::Success {
        result.Text().ok().map(|s| s.to_string())
    } else {
        None
    }
}

