//! The legacy search process owns dock child lifetime without sharing UI event loops.
use proton_core::dock_model::LayoutStore;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::Win32::System::Threading::{CreateEventW, ResetEvent, SetEvent};

pub fn layout_path() -> PathBuf {
    std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .map(PathBuf::from)
        .map(|path| path.join("protonsearch").join("dock-layout.json"))
        .unwrap_or_else(|| {
            crate::settings::AppSettings::get_settings_path().with_file_name("dock-layout.json")
        })
}

fn retry_delay(failures: u32) -> Duration {
    Duration::from_secs((1u64 << failures.min(6)).min(60))
}

pub struct DockSupervisor {
    child: Option<Child>,
    controller: HWND,
    allowed: bool,
    next_attempt: Instant,
    failures: u32,
    started: Option<Instant>,
    stop_event: Option<HANDLE>,
}

impl DockSupervisor {
    pub fn new(controller: HWND, allowed: bool) -> Self {
        let name: Vec<u16> = format!("Local\\ProtonDockStop-{}\0", std::process::id())
            .encode_utf16()
            .collect();
        let stop_event = unsafe { CreateEventW(None, true, false, PCWSTR(name.as_ptr())) }.ok();
        if stop_event.is_none() {
            crate::applog::log("cannot create dock shutdown event; dock startup disabled");
        }
        let mut supervisor = Self {
            child: None,
            controller,
            allowed: allowed && stop_event.is_some(),
            next_attempt: Instant::now(),
            failures: 0,
            started: None,
            stop_event,
        };
        supervisor.tick();
        supervisor
    }

    pub fn tick(&mut self) {
        let enabled = self.allowed
            && LayoutStore::new(layout_path())
                .load()
                .map(|config| config.enabled)
                .unwrap_or(true);
        if !enabled {
            self.stop();
            return;
        }
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(None) => {
                    if self
                        .started
                        .is_some_and(|time| time.elapsed() > Duration::from_secs(30))
                    {
                        self.failures = 0;
                    }
                    return;
                }
                Ok(Some(status)) => {
                    crate::applog::log(&format!("dock process exited: {status}"));
                }
                Err(error) => {
                    crate::applog::log(&format!("cannot inspect dock process: {error}"));
                    return;
                }
            }
            self.child = None;
            self.started = None;
            self.failures = self.failures.saturating_add(1);
            self.next_attempt = Instant::now() + retry_delay(self.failures);
        }
        if Instant::now() < self.next_attempt {
            return;
        }
        if let Some(event) = self.stop_event {
            let _ = unsafe { ResetEvent(event) };
        }
        let result = std::env::current_exe().and_then(|exe| {
            Command::new(exe)
                .arg("--dock")
                .arg("--controller-pid")
                .arg(std::process::id().to_string())
                .arg("--controller-hwnd")
                .arg((self.controller.0 as isize).to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(0x0800_0000)
                .spawn()
        });
        match result {
            Ok(child) => {
                self.child = Some(child);
                self.started = Some(Instant::now());
            }
            Err(error) => {
                crate::applog::log(&format!("cannot start native dock: {error}"));
                self.failures = self.failures.saturating_add(1);
                self.next_attempt = Instant::now() + retry_delay(self.failures);
            }
        }
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Let Slint drop the AppBar and release the reserved work area first.
            if let Some(event) = self.stop_event {
                let _ = unsafe { SetEvent(event) };
            }
            let deadline = Instant::now() + Duration::from_millis(1500);
            while child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            if child.try_wait().ok().flatten().is_none() {
                crate::applog::log("dock graceful shutdown timed out; terminating owned child");
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        self.started = None;
    }
}

impl Drop for DockSupervisor {
    fn drop(&mut self) {
        self.stop();
        if let Some(event) = self.stop_event.take() {
            let _ = unsafe { CloseHandle(event) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crash_restart_backoff_is_bounded() {
        assert_eq!(retry_delay(0), Duration::from_secs(1));
        assert_eq!(retry_delay(3), Duration::from_secs(8));
        assert_eq!(retry_delay(u32::MAX), Duration::from_secs(60));
    }
    #[test]
    fn layout_is_separate_from_search_settings() {
        assert_eq!(layout_path().file_name().unwrap(), "dock-layout.json");
    }
}
