//! Capability probes for the native adapter layer (v10 Quick Settings / Media /
//! Notifications cards). Every probe is bounded (worker thread + timeout) and
//! returns supported/unsupported with an honest reason; no UI or state changes.

use std::sync::mpsc;
use std::time::Duration;

use crate::NativeCapabilities;

const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

fn run_probe<F>(probe: F) -> crate::Capability
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("ps-native-probe".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(probe));
            let _ = sender.send(result);
        });
    if spawned.is_err() {
        return crate::Capability::unsupported("could not start capability probe");
    }
    match receiver.recv_timeout(PROBE_TIMEOUT) {
        Ok(Ok(Ok(()))) => crate::Capability::supported(),
        Ok(Ok(Err(reason))) => crate::Capability::unsupported(reason),
        Ok(Err(_)) => crate::Capability::unsupported("capability probe panicked"),
        Err(_) => crate::Capability::unsupported("capability probe timed out"),
    }
}

fn not_supported(reason: &str) -> Result<(), String> {
    Err(reason.to_string())
}

/// Probe every native integration this build knows about.
pub fn probe_all() -> NativeCapabilities {
    NativeCapabilities {
        media_sessions: crate::Capability::unsupported(
            "GSMTC media enumeration/transport controls are a post-alpha follow-up; capability probe exists but the control API is not shipped in this build",
        ),
        audio_output: run_probe(|| {
            if super::audio::output_supported() {
                Ok(())
            } else {
                not_supported("no active audio render endpoint")
            }
        }),
        audio_volume: run_probe(|| {
            if super::audio::volume_supported() {
                Ok(())
            } else {
                not_supported("default output device does not expose master volume")
            }
        }),
        audio_output_switch: run_probe(|| {
            if super::audio::output_switch_supported() {
                Ok(())
            } else {
                not_supported("IPolicyConfig default-device switching is unavailable; use Windows sound settings")
            }
        }),
        wifi: crate::Capability::unsupported(
            "Wi-Fi network list/connect is a post-alpha follow-up; the UI should open Windows network settings for now",
        ),
        bluetooth: crate::Capability::unsupported(
            "Bluetooth device list/radio toggle is a post-alpha follow-up; the UI should open Windows Bluetooth settings for now",
        ),
        brightness: run_probe(|| {
            if super::brightness::supported() {
                Ok(())
            } else {
                not_supported("no monitor exposes user-level brightness control")
            }
        }),
        local_notifications: crate::Capability::supported(),
        toast_notifications: run_probe(|| {
            let owned = super::winrt::ensure_apartment();
            let result = (|| -> Result<(), String> {
                let app_id = windows::core::HSTRING::from("ProtonSearch");
                windows::UI::Notifications::ToastNotificationManager::CreateToastNotifierWithId(&app_id)
                    .map(|_| ())
                    .map_err(|error| format!("Windows toast notifications unavailable for this app identity: {error}"))
            })();
            super::winrt::release_apartment(owned);
            result
        }),
        airplane_mode: crate::Capability::unsupported(
            "not implemented in this milestone (planned: Windows radio/network APIs, user-level)",
        ),
        energy_saver: crate::Capability::unsupported(
            "not implemented in this milestone (planned: battery/energy APIs)",
        ),
        hotspot: crate::Capability::unsupported(
            "not implemented in this milestone (Windows Mobile Hotspot has no stable user-level API)",
        ),
        cast: crate::Capability::unsupported(
            "not implemented in this milestone (requires MIracast/device picker UX)",
        ),
        night_light: crate::Capability::unsupported(
            "not implemented in this milestone (color temperature is a display-driver/OS setting)",
        ),
    }
}