use windows_implement::implement;

use windows::{
    core::HSTRING,
    Foundation::Collections::{
        IIterable, IIterable_Impl, IIterator, IIterator_Impl,
    },
    Media::SpeechRecognition::{
        SpeechContinuousRecognitionResultGeneratedEventArgs,
        SpeechContinuousRecognitionSession,
        SpeechRecognitionListConstraint,
        SpeechRecognitionResultStatus,
        SpeechRecognitionScenario,
        SpeechRecognitionTopicConstraint,
        SpeechRecognizer,
    },
    Win32::Foundation::{HWND, LPARAM, WPARAM},
    Win32::UI::WindowsAndMessaging::PostMessageW,
};

extern crate windows_core;

// Must match WM_USER constants in main.rs (WM_USER = 0x0400)
pub const WM_VOICE_WAKEWORD: u32 = 0x0400 + 100;
pub const WM_VOICE_QUERY_READY: u32 = 0x0400 + 101;

// Wrap HWND as usize so it can cross thread boundaries safely
#[derive(Clone, Copy)]
struct HwndPtr(usize);
unsafe impl Send for HwndPtr {}
unsafe impl Sync for HwndPtr {}

impl HwndPtr {
    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut std::ffi::c_void)
    }
}

// ── Minimal IIterable<HSTRING> + IIterator<HSTRING> impl ─────────────────────

#[implement(IIterable<HSTRING>, IIterator<HSTRING>)]
struct HStringIter {
    items: Vec<HSTRING>,
    pos: std::cell::Cell<usize>,
}

// Safety: HStringIter is only used on the thread that creates it (STA).
unsafe impl Send for HStringIter_Impl {}
unsafe impl Sync for HStringIter_Impl {}

impl IIterable_Impl<HSTRING> for HStringIter_Impl {
    fn First(&self) -> windows_core::Result<IIterator<HSTRING>> {
        let iter: IIterator<HSTRING> = HStringIter {
            items: self.items.clone(),
            pos: std::cell::Cell::new(0),
        }
        .into();
        Ok(iter)
    }
}

impl IIterator_Impl<HSTRING> for HStringIter_Impl {
    fn Current(&self) -> windows_core::Result<HSTRING> {
        let p = self.pos.get();
        if p < self.items.len() {
            Ok(self.items[p].clone())
        } else {
            Err(windows_core::Error::from_win32())
        }
    }
    fn HasCurrent(&self) -> windows_core::Result<bool> {
        Ok(self.pos.get() < self.items.len())
    }
    fn MoveNext(&self) -> windows_core::Result<bool> {
        let p = self.pos.get();
        if p < self.items.len() {
            self.pos.set(p + 1);
        }
        Ok(self.pos.get() < self.items.len())
    }
    fn GetMany(&self, _items: &mut [HSTRING]) -> windows_core::Result<u32> {
        Ok(0)
    }
}

fn make_phrase_iterable(phrases: &[&str]) -> IIterable<HSTRING> {
    HStringIter {
        items: phrases.iter().map(|s| HSTRING::from(*s)).collect(),
        pos: std::cell::Cell::new(0),
    }
    .into()
}

// ── Wake word listener ────────────────────────────────────────────────────────

/// Spawn a background STA thread that listens continuously for wake phrases.
/// On match, posts WM_VOICE_WAKEWORD to the launcher window.
pub fn start_wake_word_listener(hwnd: HWND) {
    let h = HwndPtr(hwnd.0 as usize);
    std::thread::spawn(move || {
        let _ = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };
        if let Err(e) = run_wake_word_loop(h) {
            eprintln!("[Voice] Wake word error: {:?}", e);
        }
        unsafe { windows::Win32::System::Com::CoUninitialize() };
    });
}

fn run_wake_word_loop(h: HwndPtr) -> windows_core::Result<()> {
    let recognizer = SpeechRecognizer::new()?;

    let phrases = make_phrase_iterable(&["hey search", "open search", "hey open search"]);
    let constraint = SpeechRecognitionListConstraint::CreateWithTag(
        &phrases,
        &HSTRING::from("wakeword"),
    )?;

    recognizer.Constraints()?.Append(&constraint)?;
    recognizer.CompileConstraintsAsync()?.get()?;

    let session: SpeechContinuousRecognitionSession =
        recognizer.ContinuousRecognitionSession()?;

    let handler = windows::Foundation::TypedEventHandler::<
        SpeechContinuousRecognitionSession,
        SpeechContinuousRecognitionResultGeneratedEventArgs,
    >::new(move |_session, args| {
        if let Some(args) = args {
            let result = args.Result()?;
            if result.Status()? == SpeechRecognitionResultStatus::Success
                && result.RawConfidence()? > 0.3
            {
                unsafe {
                    let _ = PostMessageW(h.hwnd(), WM_VOICE_WAKEWORD, WPARAM(0), LPARAM(0));
                }
            }
        }
        Ok(())
    });

    session.ResultGenerated(&handler)?;
    session.StartAsync()?.get()?;

    // Keep session alive forever
    loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
}

// ── Query dictation ───────────────────────────────────────────────────────────

/// Spawn a one-shot STA thread for dictation. Posts WM_VOICE_QUERY_READY when done.
/// WPARAM=1, LPARAM=Box<String> raw ptr → success text.
/// WPARAM=0, LPARAM=0 → failure/empty.
pub fn start_query_listener(hwnd: HWND) {
    let h = HwndPtr(hwnd.0 as usize);
    std::thread::spawn(move || {
        let _ = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };

        unsafe {
            match run_dictation() {
                Ok(text) if !text.trim().is_empty() => {
                    let ptr = Box::into_raw(Box::new(text)) as isize;
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

fn run_dictation() -> windows_core::Result<String> {
    let recognizer = SpeechRecognizer::new()?;
    let constraint = SpeechRecognitionTopicConstraint::Create(
        SpeechRecognitionScenario::Dictation,
        &HSTRING::from("dictation"),
    )?;
    recognizer.Constraints()?.Append(&constraint)?;
    recognizer.CompileConstraintsAsync()?.get()?;

    let result = recognizer.RecognizeAsync()?.get()?;
    if result.Status()? == SpeechRecognitionResultStatus::Success {
        Ok(result.Text()?.to_string())
    } else {
        Ok(String::new())
    }
}
