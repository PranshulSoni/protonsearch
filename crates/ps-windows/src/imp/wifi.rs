//! Wi-Fi adapter: radio state/toggle (WinRT Radio), available network list and
//! connect-if-profile via WlanAPI. User-level only; never asks for or stores
//! Wi-Fi keys; missing profiles return an honest error plus the settings fallback.

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use windows::core::{GUID, PCWSTR};
use windows::Devices::Radios::RadioKind;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::NetworkManagement::WiFi::{
    dot11_BSS_type_any, wlan_connection_mode_profile, WlanCloseHandle, WlanConnect, WlanEnumInterfaces,
    WlanFreeMemory, WlanGetAvailableNetworkList, WlanGetProfileList, WlanOpenHandle, DOT11_SSID,
    WLAN_AVAILABLE_NETWORK, WLAN_AVAILABLE_NETWORK_CONNECTED, WLAN_AVAILABLE_NETWORK_HAS_PROFILE,
    WLAN_AVAILABLE_NETWORK_LIST, WLAN_CONNECTION_PARAMETERS, WLAN_INTERFACE_INFO_LIST,
    WLAN_PROFILE_INFO_LIST,
};

use super::radios;
use crate::{RadioStateInfo, WifiConnectOutcome, WifiNetworkInfo};

struct WlanSession {
    handle: HANDLE,
}

impl WlanSession {
    fn open() -> Result<Self> {
        let mut negotiated = 0u32;
        let mut handle = HANDLE::default();
        let code = unsafe { WlanOpenHandle(2, None, &mut negotiated, &mut handle) };
        if code != 0 {
            bail!("WlanOpenHandle failed with code {code} (WLAN service unavailable)");
        }
        Ok(Self { handle })
    }

    fn interfaces(&self) -> Result<Vec<(GUID, String)>> {
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        let code = unsafe { WlanEnumInterfaces(self.handle, None, &mut list) };
        if code != 0 || list.is_null() {
            bail!("WlanEnumInterfaces failed with code {code}");
        }
        let mut interfaces = Vec::new();
        unsafe {
            let count = (*list).dwNumberOfItems as usize;
            let base = (*list).InterfaceInfo.as_ptr();
            for index in 0..count {
                let info = &*base.add(index);
                let end = info
                    .strInterfaceDescription
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(info.strInterfaceDescription.len());
                interfaces.push((
                    info.InterfaceGuid,
                    String::from_utf16_lossy(&info.strInterfaceDescription[..end]),
                ));
            }
            WlanFreeMemory(list as *const _);
        }
        Ok(interfaces)
    }

    fn profile_names(&self, interface: &GUID) -> Result<Vec<String>> {
        let mut list: *mut WLAN_PROFILE_INFO_LIST = std::ptr::null_mut();
        let code = unsafe { WlanGetProfileList(self.handle, interface, None, &mut list) };
        if code != 0 || list.is_null() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        unsafe {
            let count = (*list).dwNumberOfItems as usize;
            let base = (*list).ProfileInfo.as_ptr();
            for index in 0..count {
                let info = &*base.add(index);
                let end = info
                    .strProfileName
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(info.strProfileName.len());
                names.push(String::from_utf16_lossy(&info.strProfileName[..end]));
            }
            WlanFreeMemory(list as *const _);
        }
        Ok(names)
    }

    fn available_networks(&self, interface: &GUID) -> Result<Vec<WLAN_AVAILABLE_NETWORK>> {
        let mut list: *mut WLAN_AVAILABLE_NETWORK_LIST = std::ptr::null_mut();
        let code = unsafe { WlanGetAvailableNetworkList(self.handle, interface, 0, None, &mut list) };
        if code != 0 || list.is_null() {
            return Err(anyhow::anyhow!(wlan_error_message(code)));
        }
        let mut networks = Vec::new();
        unsafe {
            let count = (*list).dwNumberOfItems as usize;
            let base = (*list).Network.as_ptr();
            for index in 0..count {
                networks.push(*base.add(index));
            }
            WlanFreeMemory(list as *const _);
        }
        Ok(networks)
    }

    fn connect(&self, interface: &GUID, profile: &str, ssid: &[u8]) -> Result<()> {
        let wide: Vec<u16> = profile.encode_utf16().chain(std::iter::once(0)).collect();
        let mut dot11 = DOT11_SSID::default();
        let length = ssid.len().min(dot11.ucSSID.len());
        dot11.uSSIDLength = length as u32;
        dot11.ucSSID[..length].copy_from_slice(&ssid[..length]);
        let parameters = WLAN_CONNECTION_PARAMETERS {
            wlanConnectionMode: wlan_connection_mode_profile,
            strProfile: PCWSTR(wide.as_ptr()),
            pDot11Ssid: &mut dot11,
            pDesiredBssidList: std::ptr::null_mut(),
            dot11BssType: dot11_BSS_type_any,
            dwFlags: 0,
        };
        let code = unsafe { WlanConnect(self.handle, interface, &parameters, None) };
        if code != 0 {
            bail!("WlanConnect failed with code {code} (the saved profile could not be used)");
        }
        Ok(())
    }
}

impl Drop for WlanSession {
    fn drop(&mut self) {
        unsafe {
            let _ = WlanCloseHandle(self.handle, None);
        }
    }
}

fn ssid_of(network: &WLAN_AVAILABLE_NETWORK) -> String {
    let length = (network.dot11Ssid.uSSIDLength as usize).min(network.dot11Ssid.ucSSID.len());
    String::from_utf8_lossy(&network.dot11Ssid.ucSSID[..length]).to_string()
}

fn security_of(network: &WLAN_AVAILABLE_NETWORK) -> String {
    if !network.bSecurityEnabled.as_bool() {
        return "Open".to_string();
    }
    let auth = network.dot11DefaultAuthAlgorithm.0;
    match auth {
        1 => "Open (802.11)".to_string(),
        2 => "Shared".to_string(),
        3 | 4 | 5 => "WPA".to_string(),
        6 | 7 => "WPA2".to_string(),
        8 | 9 | 10 | 11 => "WPA3".to_string(),
        other => format!("Secured ({other})"),
    }
}

/// Radio state for Wi-Fi (WinRT radio).
pub(crate) fn radio_state() -> Result<RadioStateInfo> {
    let state = radios::radio_state(RadioKind::WiFi)?;
    Ok(RadioStateInfo {
        available: state.available,
        enabled: state.enabled,
    })
}

pub(crate) fn set_radio(enabled: bool) -> Result<()> {
    radios::set_radio(RadioKind::WiFi, enabled)
}

/// True when this host has a Wi-Fi radio and/or a usable WLAN adapter.
pub(crate) fn supported() -> bool {
    if radios::has_radio(RadioKind::WiFi) {
        return true;
    }
    WlanSession::open()
        .and_then(|session| session.interfaces())
        .map(|interfaces| !interfaces.is_empty())
        .unwrap_or(false)
}

/// Available Wi-Fi networks across adapters (connected first, then saved
/// profiles, then strongest signal). Networks without a visible SSID are skipped.
pub(crate) fn list_networks() -> Result<Vec<WifiNetworkInfo>> {
    let session = WlanSession::open()?;
    let interfaces = session.interfaces()?;
    let mut merged: BTreeMap<String, WifiNetworkInfo> = BTreeMap::new();
    let mut first_error: Option<anyhow::Error> = None;
    let mut interface_errors = 0usize;
    for (interface, _description) in interfaces {
        let profiles = session.profile_names(&interface).unwrap_or_default();
        let networks = match session.available_networks(&interface) {
            Ok(networks) => networks,
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
                interface_errors += 1;
                continue;
            }
        };
        for network in networks {
            let ssid = ssid_of(&network);
            if ssid.is_empty() {
                continue;
            }
            let connected = network.dwFlags & WLAN_AVAILABLE_NETWORK_CONNECTED != 0;
            let has_profile = network.dwFlags & WLAN_AVAILABLE_NETWORK_HAS_PROFILE != 0
                || profiles.iter().any(|profile| profile.eq_ignore_ascii_case(&ssid));
            let signal = network.wlanSignalQuality.min(100);
            let security = security_of(&network);
            let entry = merged.entry(ssid.clone()).or_insert_with(|| WifiNetworkInfo {
                ssid: ssid.clone(),
                signal,
                security: security.clone(),
                connected,
                has_profile,
                connectable: network.bNetworkConnectable.as_bool(),
            });
            entry.signal = entry.signal.max(signal);
            entry.connected |= connected;
            entry.has_profile |= has_profile;
            entry.connectable |= network.bNetworkConnectable.as_bool();
            if entry.security.is_empty() {
                entry.security = security;
            }
        }
    }
    if merged.is_empty() && interface_errors > 0 {
        if let Some(error) = first_error {
            return Err(error);
        }
    }
    let mut networks: Vec<WifiNetworkInfo> = merged.into_values().collect();
    networks.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then_with(|| b.has_profile.cmp(&a.has_profile))
            .then_with(|| b.signal.cmp(&a.signal))
            .then_with(|| a.ssid.cmp(&b.ssid))
    });
    Ok(networks)
}

/// Connect to an existing saved Wi-Fi profile. Never creates a profile and never
/// asks for keys: if no profile exists the caller gets an honest error telling
/// the UI to open Windows Wi-Fi settings.
pub(crate) fn connect(ssid: &str) -> Result<WifiConnectOutcome> {
    let trimmed = ssid.trim();
    if trimmed.is_empty() {
        bail!("Wi-Fi connect requires a non-empty SSID");
    }
    let state = radio_state().unwrap_or(RadioStateInfo {
        available: false,
        enabled: false,
    });
    if state.available && !state.enabled {
        bail!("Wi-Fi radio is off; turn it on first (ms-settings:network-wifi)");
    }
    let session = WlanSession::open()?;
    let interfaces = session.interfaces()?;
    if interfaces.is_empty() {
        bail!("no WLAN adapter is available on this device");
    }
    let mut profile_for: Option<(GUID, String)> = None;
    for (interface, _description) in &interfaces {
        let profiles = session.profile_names(interface).unwrap_or_default();
        if let Some(profile) = profiles
            .iter()
            .find(|profile| profile.eq_ignore_ascii_case(trimmed))
            .cloned()
        {
            profile_for = Some((*interface, profile));
            break;
        }
    }
    let Some((interface, profile)) = profile_for else {
        bail!(
            "no saved Wi-Fi profile for '{trimmed}'; open Windows network settings to connect (ms-settings:network-wifi)"
        );
    };
    let ssid_bytes = trimmed.as_bytes();
    session.connect(&interface, &profile, ssid_bytes)?;
    // WlanConnect only queues the connection: poll the cached network flags for a
    // bounded time so the caller gets a truthful "connected" flag when possible.
    let mut connected = false;
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(300));
        if let Ok(networks) = session.available_networks(&interface) {
            if networks.iter().any(|network| {
                ssid_of(network).eq_ignore_ascii_case(trimmed)
                    && network.dwFlags & WLAN_AVAILABLE_NETWORK_CONNECTED != 0
            }) {
                connected = true;
                break;
            }
        }
    }
    Ok(WifiConnectOutcome {
        ssid: trimmed.to_string(),
        connected,
        message: if connected {
            format!("Connected to {trimmed} using the saved profile")
        } else {
            format!("Connection requested for {trimmed}; Windows will report the result")
        },
    })
}

/// Human-readable WLAN error with an honest fallback hint.
fn wlan_error_message(code: u32) -> String {
    match code {
        5 => "Windows denied Wi-Fi scan access (location permission). Enable Settings > Privacy & security > Location for desktop apps, then retry (ms-settings:network-wifi)".to_string(),
        1062 => "WLAN AutoConfig service is not running; start it from Windows services (ms-settings:network-wifi)".to_string(),
        1168 => "Wi-Fi radio is off or the adapter is disconnected (ms-settings:network-wifi)".to_string(),
        other => format!("Windows WLAN scan failed with code {other} (ms-settings:network-wifi)"),
    }
}

/// Descriptions of WLAN adapters (diagnostics).
pub(crate) fn interface_descriptions() -> Vec<String> {
    WlanSession::open()
        .and_then(|session| session.interfaces())
        .map(|interfaces| {
            interfaces
                .into_iter()
                .map(|(_, description)| description)
                .collect()
        })
        .unwrap_or_default()
}
