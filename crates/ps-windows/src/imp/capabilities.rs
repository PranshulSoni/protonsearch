//! Capability probes for the native adapter layer (v10 Quick Settings / Media /
//! Notifications cards). Every probe is bounded (worker thread + timeout) and
//! returns supported/unsupported with an honest reason and — where Windows owns
//! the feature — an allow-listed ms-settings fallback URI for the UI button.

use std::sync::mpsc;
use std::time::Duration;

use crate::{Capability, NativeCapabilities, SettingsPage};

const PROBE_TIMEOUT: Duration = Duration::from_secs(4);

fn run_probe<F>(probe: F, fallback: Option<SettingsPage>) -> Capability
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
        return Capability::unsupported("could not start capability probe");
    }
    let capability = match receiver.recv_timeout(PROBE_TIMEOUT) {
        Ok(Ok(Ok(()))) => Capability::supported(),
        Ok(Ok(Err(reason))) => Capability::unsupported(reason),
        Ok(Err(_)) => Capability::unsupported("capability probe panicked"),
        Err(_) => Capability::unsupported("capability probe timed out"),
    };
    // Keep the settings fallback available even when supported: Windows may still
    // deny a specific action (e.g. radio consent / location permission), and the
    // UI should be able to offer "open Windows settings".
    match fallback {
        Some(page) => capability.with_fallback(page),
        None => capability,
    }
}

fn not_supported(reason: &str) -> Result<(), String> {
    Err(reason.to_string())
}

/// Probe every native integration this build ships.
pub fn probe_all() -> NativeCapabilities {
    let media_sessions = run_probe(
        || super::media::now_playing().map(|_| ()).map_err(|error| format!("{error:#}")),
        None,
    );
    let audio_output = run_probe(
        || {
            if super::audio::output_supported() {
                Ok(())
            } else {
                not_supported("no active audio render endpoint")
            }
        },
        Some(SettingsPage::Sound),
    );
    let audio_volume = run_probe(
        || {
            if super::audio::volume_supported() {
                Ok(())
            } else {
                not_supported("default output device does not expose master volume")
            }
        },
        Some(SettingsPage::Sound),
    );
    let audio_output_switch = run_probe(
        || {
            if super::audio::output_switch_supported() {
                Ok(())
            } else {
                not_supported("default-device switching is unavailable on this build")
            }
        },
        Some(SettingsPage::Sound),
    );
    let wifi = run_probe(
        || {
            if super::wifi::supported() {
                Ok(())
            } else {
                not_supported("no Wi-Fi adapter or radio is available on this device")
            }
        },
        Some(SettingsPage::Wifi),
    );
    let bluetooth = run_probe(
        || {
            if super::bluetooth::supported() {
                Ok(())
            } else {
                not_supported("no Bluetooth radio is available on this device")
            }
        },
        Some(SettingsPage::Bluetooth),
    );
    let brightness = run_probe(
        || {
            if super::brightness::supported() {
                Ok(())
            } else {
                not_supported(
                    "no monitor exposes brightness control (internal panel or DDC/CI) on this device",
                )
            }
        },
        Some(SettingsPage::Display),
    );
    let local_notifications = Capability::supported();
    let toast_notifications = run_probe(
        || {
            let owned = super::winrt::ensure_apartment();
            let result = (|| -> Result<(), String> {
                let app_id = windows::core::HSTRING::from("ProtonSearch");
                windows::UI::Notifications::ToastNotificationManager::CreateToastNotifierWithId(
                    &app_id,
                )
                .map(|_| ())
                .map_err(|error| {
                    format!("Windows toast notifications unavailable for this app identity: {error}")
                })
            })();
            super::winrt::release_apartment(owned);
            result
        },
        Some(SettingsPage::Notifications),
    );
    NativeCapabilities {
        media_sessions,
        audio_output,
        audio_volume,
        audio_output_switch,
        wifi,
        bluetooth,
        brightness,
        local_notifications,
        toast_notifications,
        airplane_mode: Capability::unsupported_with_fallback(
            "airplane mode has no stable user-level desktop API; use Windows settings",
            SettingsPage::AirplaneMode,
        ),
        energy_saver: Capability::unsupported_with_fallback(
            "battery saver has no stable user-level desktop API; use Windows settings",
            SettingsPage::EnergySaver,
        ),
        hotspot: Capability::unsupported_with_fallback(
            "mobile hotspot has no stable user-level desktop API; use Windows settings",
            SettingsPage::MobileHotspot,
        ),
        cast: Capability::unsupported_with_fallback(
            "projecting/casting uses the Windows device picker; use Windows settings",
            SettingsPage::Project,
        ),
        night_light: Capability::unsupported_with_fallback(
            "night light is an OS display setting; use Windows settings",
            SettingsPage::NightLight,
        ),
    }
}