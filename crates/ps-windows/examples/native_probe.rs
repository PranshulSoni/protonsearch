//! Native adapter host probe (milestone 2): prints capability matrix, audio,
//! Wi-Fi, Bluetooth, brightness and settings fallbacks.
//!
//!   cargo run -p ps-windows --example native_probe

fn section(title: &str) {
    println!("=== {title}");
}

fn main() {
    let capabilities = ps_windows::native_capabilities();
    section("native capability matrix (milestone 2)");
    println!(
        "{}",
        serde_json::to_string_pretty(&capabilities).unwrap_or_else(|_| format!("{capabilities:#?}"))
    );

    section("settings fallbacks");
    for page in [
        ps_windows::SettingsPage::Wifi,
        ps_windows::SettingsPage::Bluetooth,
        ps_windows::SettingsPage::Display,
        ps_windows::SettingsPage::Sound,
        ps_windows::SettingsPage::AirplaneMode,
        ps_windows::SettingsPage::MobileHotspot,
        ps_windows::SettingsPage::NightLight,
    ] {
        println!("page {:?} -> {} ({})", page, page.uri(), page.label());
    }

    section("audio outputs");
    match ps_windows::list_audio_outputs() {
        Ok(devices) => {
            for device in &devices {
                println!(
                    "device default={} name={:?} id={:?}",
                    device.is_default, device.name, device.id
                );
            }
            match ps_windows::audio_state() {
                Ok(state) => println!("audio state volume={:.2} muted={}", state.volume, state.muted),
                Err(error) => println!("audio state unavailable: {error:#}"),
            }
        }
        Err(error) => println!("audio outputs unavailable: {error:#}"),
    }

    section("wifi");
    match ps_windows::wifi_radio_state() {
        Ok(state) => {
            println!(
                "wifi radio available={} enabled={} adapters={:?}",
                state.available,
                state.enabled,
                ps_windows::wlan_adapters()
            );
            match ps_windows::list_wifi_networks() {
                Ok(networks) => {
                    println!("wifi networks: {}", networks.len());
                    for network in networks.iter().take(12) {
                        println!(
                            "network ssid={:?} signal={} security={} connected={} has_profile={} connectable={}",
                            network.ssid,
                            network.signal,
                            network.security,
                            network.connected,
                            network.has_profile,
                            network.connectable
                        );
                    }
                }
                Err(error) => println!("wifi list unavailable: {error:#}"),
            }
        }
        Err(error) => println!("wifi radio unavailable: {error:#}"),
    }

    section("bluetooth");
    match ps_windows::bluetooth_radio_state() {
        Ok(state) => {
            println!(
                "bluetooth radio available={} enabled={}",
                state.available, state.enabled
            );
            match ps_windows::list_bluetooth_devices() {
                Ok(devices) => {
                    println!("paired bluetooth devices: {}", devices.len());
                    for device in devices.iter().take(12) {
                        println!(
                            "device name={:?} connected={} id={:?}",
                            device.name, device.connected, device.id
                        );
                    }
                }
                Err(error) => println!("bluetooth devices unavailable: {error:#}"),
            }
        }
        Err(error) => println!("bluetooth radio unavailable: {error:#}"),
    }

    section("brightness");
    let monitors = ps_windows::list_brightness_monitors();
    if monitors.is_empty() {
        println!("brightness unsupported on this host");
    } else {
        for monitor in &monitors {
            println!(
                "monitor name={:?} current={} {}/{} method={:?}",
                monitor.name, monitor.current, monitor.minimum, monitor.maximum, monitor.method
            );
        }
        println!("current brightness: {:?}", ps_windows::get_brightness());
    }

    section("notifications");
    println!(
        "local store: supported; toast supported={}",
        ps_windows::toast_supported()
    );

    section("controls status snapshot");
    let status = ps_windows::controls_status();
    println!(
        "{}",
        serde_json::to_string_pretty(&status).unwrap_or_else(|_| format!("{status:#?}"))
    );
}