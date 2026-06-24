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
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = writeln!(file, "[{}] {}", now_ms(), msg);
    }
}

/// Diagnostic line into voice_log.txt (used by main.rs for hotkey registration).
pub fn log(msg: &str) {
    log_voice(msg.to_string());
}

// ── One-shot dictation, triggered by hotkey or mic button ─────────────────────

/// Run one-shot dictation and post the recognized (normalized) query to the launcher.
/// The recognizer is built FRESH inside this thread (which stays alive for the whole
/// RecognizeAsync) — reusing a recognizer built in an already-exited thread hangs.
/// WPARAM=1 + Box<String> on success, WPARAM=0 on empty/failure.
pub fn start_query_listener(hwnd: HWND) {
    let h = HwndPtr(hwnd.0 as usize);
    std::thread::spawn(move || {
        let _ = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };

        // ponytail: no always-on session owns the mic anymore, so dictation can start
        // immediately; one retry still covers a cold WinRT recognizer returning empty.
        let mut text = None;
        for attempt in 1..=QUERY_ATTEMPTS {
            log_voice(format!("query: building dictation recognizer (attempt={attempt})"));
            text = run_dictation();
            if text.is_some() {
                break;
            }
            if attempt < QUERY_ATTEMPTS {
                log_voice("query: retry after empty/failure".into());
                std::thread::sleep(std::time::Duration::from_millis(QUERY_RETRY_DELAY_MS));
            }
        }
        log_voice(format!("query: dictation done (got_text={})", text.is_some()));

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
            windows::Win32::System::Com::CoUninitialize();
        }
    });
}

fn run_dictation() -> Option<String> {
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

    log_voice("query: RecognizeAsync (listening)".into());
    let result = recognizer.RecognizeAsync().ok()?.get().ok()?;
    let status = result.Status().ok()?;
    log_voice(format!("query: result status={:?}", status));
    if status == SpeechRecognitionResultStatus::Success {
        result.Text().ok().map(|s| s.to_string())
    } else {
        None
    }
}

