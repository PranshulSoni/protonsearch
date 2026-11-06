use serde_json::{json, Value};
use std::ffi::c_void;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use windows::{
    Devices::Radios::{Radio, RadioKind, RadioState},
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager as MediaManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
    },
    Win32::{
        Foundation::HANDLE,
        NetworkManagement::{
            IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2},
            Ndis::IfOperStatusUp,
            WiFi::{
                wlan_interface_state_connected, wlan_intf_opcode_current_connection,
                WlanCloseHandle, WlanEnumInterfaces, WlanFreeMemory, WlanOpenHandle,
                WlanQueryInterface, WLAN_CONNECTION_ATTRIBUTES, WLAN_INTERFACE_INFO,
                WLAN_INTERFACE_INFO_LIST,
            },
        },
        System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
    },
};

pub(crate) fn envelope(status: &str, data: Value, message: &str) -> Value {
    json!({"status":status,"data":data,"message":message,
        "updatedAt":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64})
}

struct Apartment;
impl Apartment {
    fn initialize() -> windows::core::Result<Self> {
        // Commands run on blocking workers; balance each successful WinRT initialization.
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}

pub fn snapshot(provider: &str) -> Result<Value, String> {
    match provider {
        "network" => network(),
        "media" => match media() {
            Ok(value) => Ok(value),
            Err(error) if error.code().0 as u32 == 0x80070005 => Ok(envelope(
                "permission-denied",
                Value::Null,
                "Windows denied media session access",
            )),
            Err(error) => Err(error.to_string()),
        },
        _ => Err("Unknown widget provider".into()),
    }
}

fn media() -> windows::core::Result<Value> {
    let _apartment = Apartment::initialize()?;
    let manager = MediaManager::RequestAsync()?.get()?;
    let sessions = manager.GetSessions()?;
    if sessions.Size()? == 0 {
        return Ok(envelope("empty", Value::Null, "No active media session"));
    }
    let session = manager.GetCurrentSession()?;
    let properties = session.TryGetMediaPropertiesAsync()?.get()?;
    let playback = session.GetPlaybackInfo()?;
    let controls = playback.Controls()?;
    Ok(envelope(
        "live",
        json!({
            "title":properties.Title()?.to_string(), "artist":properties.Artist()?.to_string(),
            "source":session.SourceAppUserModelId()?.to_string(),
            "playing":playback.PlaybackStatus()? == PlaybackStatus::Playing,
            "canToggle":controls.IsPlayPauseToggleEnabled()?,
            "canNext":controls.IsNextEnabled()?, "canPrevious":controls.IsPreviousEnabled()?
        }),
        "",
    ))
}

pub fn action(action: &str) -> Result<(), String> {
    if let Some(uri) = settings_uri(action) {
        return open_uri(uri).map_err(|error| error.to_string());
    }
    if !["media:toggle", "media:next", "media:previous"].contains(&action) {
        return Err("Unsupported widget action".into());
    }
    media_action(action).map_err(|error| error.to_string())
}
fn settings_uri(action: &str) -> Option<&'static str> {
    match action {
        "settings:sound" => Some("ms-settings:sound"),
        "settings:network" => Some("ms-settings:network-status"),
        "settings:bluetooth" => Some("ms-settings:bluetooth"),
        "settings:notifications" => Some("ms-settings:notifications"),
        _ => None,
    }
}
pub(crate) fn open_uri(uri: &str) -> windows::core::Result<()> {
    let _apartment = Apartment::initialize()?;
    use windows::{
        core::{w, PCWSTR},
        Win32::UI::{
            Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };
    let wide: Vec<u16> = uri.encode_utf16().chain(Some(0)).collect();
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        lpVerb: w!("open"),
        lpFile: PCWSTR(wide.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // Callers supply fixed settings routes or parsed HTTP(S) URLs from cached search results.
    unsafe { ShellExecuteExW(&mut info) }
}
fn media_action(action: &str) -> windows::core::Result<()> {
    let _apartment = Apartment::initialize()?;
    let manager = MediaManager::RequestAsync()?.get()?;
    let session = manager.GetCurrentSession()?;
    let controls = session.GetPlaybackInfo()?.Controls()?;
    let result = match action {
        "media:toggle" if controls.IsPlayPauseToggleEnabled()? => {
            session.TryTogglePlayPauseAsync()?.get()?
        }
        "media:next" if controls.IsNextEnabled()? => session.TrySkipNextAsync()?.get()?,
        "media:previous" if controls.IsPreviousEnabled()? => {
            session.TrySkipPreviousAsync()?.get()?
        }
        _ => false,
    };
    if result {
        Ok(())
    } else {
        Err(windows::core::Error::new(
            windows::core::HRESULT(0x80004005u32 as i32),
            "Media session rejected this action",
        ))
    }
}

type Counters = HashMap<u128, (u64, u64)>;
struct NetworkSample {
    at: Instant,
    counters: Counters,
}
fn rates(previous: &Counters, current: &Counters, seconds: f64) -> Option<(f64, f64)> {
    if seconds <= 0.0 {
        return None;
    }
    let mut bytes = (0u64, 0u64);
    let mut comparable = false;
    for (id, (received, sent)) in current {
        if let Some((old_received, old_sent)) = previous.get(id) {
            if let (Some(download), Some(upload)) = (
                received.checked_sub(*old_received),
                sent.checked_sub(*old_sent),
            ) {
                bytes.0 = bytes.0.saturating_add(download);
                bytes.1 = bytes.1.saturating_add(upload);
                comparable = true;
            }
        }
    }
    comparable.then_some((bytes.0 as f64 / seconds, bytes.1 as f64 / seconds))
}

fn wide_string(value: &[u16]) -> String {
    let length = value.iter().position(|c| *c == 0).unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length])
}
fn ssid_text(length: u32, bytes: &[u8; 32]) -> Option<String> {
    let length = (length as usize).min(bytes.len());
    (length > 0).then(|| String::from_utf8_lossy(&bytes[..length]).into_owned())
}
fn signal_percent(quality: u32) -> u8 {
    quality.min(100) as u8
}
fn wifi_state_label(state: i32) -> &'static str {
    match state {
        0 => "not_ready",
        1 => "connected",
        2 => "ad_hoc",
        3 => "disconnecting",
        4 => "disconnected",
        5 => "associating",
        6 => "discovering",
        7 => "authenticating",
        _ => "unknown",
    }
}
fn connection_type(wifi: bool, ethernet: bool, vpn: bool) -> &'static str {
    if vpn {
        "vpn"
    } else if wifi {
        "wifi"
    } else if ethernet {
        "ethernet"
    } else {
        "none"
    }
}
fn connection_present(
    wifi_connected: bool,
    wifi_api_available: bool,
    wifi_up: bool,
    ethernet: bool,
    vpn: bool,
    other_up: bool,
) -> bool {
    // A Wi-Fi adapter can be "up" in NDIS while WLAN reports it as disconnected.
    // Only trust NDIS when the WLAN API itself is unavailable.
    wifi_connected || (!wifi_api_available && wifi_up) || ethernet || vpn || (!wifi_up && other_up)
}
fn adapter_name(alias: &str, description: &str, prefer_description: bool) -> Option<String> {
    let (first, second) = if prefer_description {
        (description, alias)
    } else {
        (alias, description)
    };
    [first, second]
        .into_iter()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_owned)
}
fn is_vpn_adapter(if_type: u32, alias: &str, description: &str) -> bool {
    let text = format!("{alias} {description}").to_ascii_lowercase();
    if ["teredo", "isatap", "6to4"]
        .iter()
        .any(|word| text.contains(word))
    {
        return false;
    }
    if_type == 23 // IF_TYPE_PPP; tunnel adapters are too broad to count without a name match.
        || [
            "vpn",
            "wireguard",
            "openvpn",
            "nordlynx",
            "tap-windows",
            "wintun",
            "l2tp",
            "ikev2",
            "sstp",
            "pptp",
            "tailscale",
            "zerotier",
            "tunnel",
            "pangp",
            "globalprotect",
        ]
        .iter()
        .any(|word| text.contains(word))
}

struct WifiStatus {
    connected: bool,
    ssid: Option<String>,
    signal_percent: Option<u8>,
    state: &'static str,
    api_available: bool,
}
impl WifiStatus {
    fn unsupported() -> Self {
        Self {
            connected: false,
            ssid: None,
            signal_percent: None,
            state: "unsupported",
            api_available: false,
        }
    }
}
struct WlanHandle(HANDLE);
impl Drop for WlanHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = WlanCloseHandle(self.0, None);
        }
    }
}
fn open_wlan() -> Option<WlanHandle> {
    // Client version 2 targets Windows 10 and later.
    let mut handle = HANDLE::default();
    let mut negotiated = 0u32;
    let code = unsafe { WlanOpenHandle(2, None, &mut negotiated, &mut handle) };
    (code == 0).then_some(WlanHandle(handle))
}
fn wlan_interfaces(handle: &WlanHandle) -> Option<Vec<WLAN_INTERFACE_INFO>> {
    let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
    if unsafe { WlanEnumInterfaces(handle.0, None, &mut list) } != 0 || list.is_null() {
        return None;
    }
    let interfaces = unsafe {
        let list = &*list;
        let interfaces =
            std::slice::from_raw_parts(list.InterfaceInfo.as_ptr(), list.dwNumberOfItems as usize)
                .to_vec();
        WlanFreeMemory(list as *const WLAN_INTERFACE_INFO_LIST as *const c_void);
        interfaces
    };
    Some(interfaces)
}
fn current_wifi_connection(
    handle: &WlanHandle,
    guid: &windows::core::GUID,
) -> Option<WLAN_CONNECTION_ATTRIBUTES> {
    let mut size = 0u32;
    let mut data: *mut c_void = std::ptr::null_mut();
    let code = unsafe {
        WlanQueryInterface(
            handle.0,
            guid,
            wlan_intf_opcode_current_connection,
            None,
            &mut size,
            &mut data,
            None,
        )
    };
    if code != 0
        || data.is_null()
        || size < std::mem::size_of::<WLAN_CONNECTION_ATTRIBUTES>() as u32
    {
        if !data.is_null() {
            unsafe { WlanFreeMemory(data) };
        }
        return None;
    }
    let attributes = unsafe { std::ptr::read_unaligned(data.cast::<WLAN_CONNECTION_ATTRIBUTES>()) };
    unsafe { WlanFreeMemory(data) };
    Some(attributes)
}
fn read_wifi() -> WifiStatus {
    let Some(handle) = open_wlan() else {
        return WifiStatus::unsupported();
    };
    let Some(interfaces) = wlan_interfaces(&handle) else {
        return WifiStatus::unsupported();
    };
    if interfaces.is_empty() {
        return WifiStatus::unsupported();
    }
    let interface = interfaces
        .iter()
        .find(|interface| interface.isState == wlan_interface_state_connected)
        .unwrap_or(&interfaces[0]);
    let state = wifi_state_label(interface.isState.0);
    if interface.isState != wlan_interface_state_connected {
        return WifiStatus {
            connected: false,
            ssid: None,
            signal_percent: None,
            state,
            api_available: true,
        };
    }
    let Some(connection) = current_wifi_connection(&handle, &interface.InterfaceGuid)
        .filter(|connection| connection.isState == wlan_interface_state_connected)
    else {
        return WifiStatus {
            connected: false,
            ssid: None,
            signal_percent: None,
            state: "disconnected",
            api_available: true,
        };
    };
    let association = connection.wlanAssociationAttributes;
    WifiStatus {
        connected: true,
        ssid: ssid_text(
            association.dot11Ssid.uSSIDLength,
            &association.dot11Ssid.ucSSID,
        ),
        signal_percent: Some(signal_percent(association.wlanSignalQuality)),
        state: "connected",
        api_available: true,
    }
}
fn bluetooth_status() -> Value {
    // The radio manager keeps process-wide state; tearing down the apartment between
    // reads makes the second enumeration crash. Keep one initialization per thread.
    thread_local! {
        static APARTMENT_READY: bool = unsafe { RoInitialize(RO_INIT_MULTITHREADED).is_ok() };
    }
    let unavailable = json!({"available":false,"enabled":false,"name":null});
    if !APARTMENT_READY.with(|ready| *ready) {
        return unavailable;
    }
    let Ok(operation) = Radio::GetRadiosAsync() else {
        return unavailable;
    };
    let Ok(radios) = operation.get() else {
        return unavailable;
    };
    let Ok(size) = radios.Size() else {
        return unavailable;
    };
    for index in 0..size {
        let Ok(radio) = radios.GetAt(index) else {
            continue;
        };
        if !matches!(radio.Kind(), Ok(RadioKind::Bluetooth)) {
            continue;
        }
        let enabled = matches!(radio.State(), Ok(RadioState::On));
        let name = radio.Name().ok().and_then(|name| {
            let name = name.to_string();
            (!name.trim().is_empty()).then_some(name)
        });
        return json!({"available":true,"enabled":enabled,"name":name});
    }
    unavailable
}
fn network() -> Result<Value, String> {
    static PREVIOUS: OnceLock<Mutex<Option<NetworkSample>>> = OnceLock::new();
    let mut previous = PREVIOUS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|error| error.to_string())?;
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    let mut counters = Counters::new();
    let mut names = Vec::new();
    let mut wifi_up = false;
    let mut ethernet_name = None;
    let mut vpn_name = None;
    // GetIfTable2 allocates an aligned counted array; copy only needed data before freeing it.
    unsafe {
        GetIfTable2(&mut table)
            .ok()
            .map_err(|error| error.to_string())?;
        if table.is_null() {
            return Err("Windows returned no interface table".into());
        }
        for row in std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize)
        {
            let alias = wide_string(&row.Alias);
            let description = wide_string(&row.Description);
            let is_up = row.OperStatus == IfOperStatusUp;
            let is_hardware = row.InterfaceAndOperStatusFlags._bitfield & 1 != 0;
            if is_up {
                if row.Type == 71 {
                    wifi_up = true;
                }
                if is_vpn_adapter(row.Type, &alias, &description) {
                    if vpn_name.is_none() {
                        vpn_name = adapter_name(&alias, &description, true);
                    }
                } else if row.Type == 6 && is_hardware && ethernet_name.is_none() {
                    ethernet_name = adapter_name(&alias, &description, false);
                }
            }
            if is_up && [6, 71].contains(&row.Type) && is_hardware {
                counters.insert(row.InterfaceGuid.to_u128(), (row.InOctets, row.OutOctets));
                names.push(alias);
            }
        }
        FreeMibTable(table.cast());
    }
    let at = Instant::now();
    let speed = previous.as_ref().and_then(|old| {
        rates(
            &old.counters,
            &counters,
            at.duration_since(old.at).as_secs_f64(),
        )
    });
    let wifi = read_wifi();
    let wifi_active = wifi.connected || (!wifi.api_available && wifi_up);
    let bluetooth = bluetooth_status();
    let connected = connection_present(
        wifi.connected,
        wifi.api_available,
        wifi_up,
        ethernet_name.is_some(),
        vpn_name.is_some(),
        !wifi_up && !counters.is_empty(),
    );
    let connection_type = connection_type(wifi_active, ethernet_name.is_some(), vpn_name.is_some());
    *previous = Some(NetworkSample { at, counters });
    let (download, upload) = speed
        .map(|(down, up)| (Some(down), Some(up)))
        .unwrap_or((None, None));
    Ok(envelope(
        if !connected {
            "empty"
        } else if speed.is_none() {
            "loading"
        } else {
            "live"
        },
        json!({
            "downloadBytesPerSecond":download,"uploadBytesPerSecond":upload,"interfaces":names,
            "connectionType":connection_type,
            "wifi":{"connected":wifi.connected,"ssid":wifi.ssid,"signalPercent":wifi.signal_percent,"state":wifi.state},
            "ethernet":{"connected":ethernet_name.is_some(),"adapter":ethernet_name},
            "vpn":{"connected":vpn_name.is_some(),"adapter":vpn_name},
            "bluetooth":bluetooth
        }),
        if connected {
            ""
        } else {
            "No active network connection"
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_rates_ignore_new_and_reset_adapters() {
        let old = HashMap::from([(1, (100, 200)), (2, (500, 600))]);
        let new = HashMap::from([(1, (300, 300)), (2, (10, 20)), (3, (100000, 200000))]);
        assert_eq!(rates(&old, &new, 2.0), Some((100.0, 50.0)));
        assert_eq!(rates(&old, &new, 0.0), None);
        assert_eq!(rates(&HashMap::new(), &new, 1.0), None);
    }
    #[test]
    fn reused_interface_index_does_not_match_a_different_adapter_guid() {
        let old_guid = windows::core::GUID::from_u128(1);
        let replacement_guid = windows::core::GUID::from_u128(2);
        let old = HashMap::from([(old_guid.to_u128(), (100, 200))]);
        let replacement = HashMap::from([(replacement_guid.to_u128(), (100000, 200000))]);
        assert_eq!(rates(&old, &replacement, 1.0), None);
    }
    #[test]
    fn rejects_unlisted_actions_and_providers() {
        assert_eq!(
            action("shell:arbitrary").unwrap_err(),
            "Unsupported widget action"
        );
        assert_eq!(snapshot("files").unwrap_err(), "Unknown widget provider");
    }
    #[test]
    fn settings_actions_have_exact_native_routes_and_reject_injected_routes() {
        for (action_id, uri) in [
            ("settings:sound", "ms-settings:sound"),
            ("settings:network", "ms-settings:network-status"),
            ("settings:bluetooth", "ms-settings:bluetooth"),
            ("settings:notifications", "ms-settings:notifications"),
        ] {
            assert_eq!(settings_uri(action_id), Some(uri));
        }
        for action_id in [
            "ms-settings:sound",
            "settings:arbitrary",
            "settings:sound;calc.exe",
            "settings:network?url=https://example.com",
        ] {
            assert_eq!(settings_uri(action_id), None);
            assert_eq!(action(action_id).unwrap_err(), "Unsupported widget action");
        }
    }
    #[test]
    fn connection_type_prefers_vpn_then_wifi_then_ethernet() {
        assert_eq!(connection_type(true, true, true), "vpn");
        assert_eq!(connection_type(true, true, false), "wifi");
        assert_eq!(connection_type(false, true, false), "ethernet");
        assert_eq!(connection_type(false, false, false), "none");
    }
    #[test]
    fn disconnected_wifi_adapter_is_not_a_connection() {
        assert!(!connection_present(false, true, true, false, false, false));
        assert!(connection_present(false, false, true, false, false, false));
        assert!(connection_present(true, true, true, false, false, false));
        assert!(connection_present(false, true, false, true, false, false));
        assert!(connection_present(false, true, false, false, true, false));
        assert!(connection_present(false, true, false, false, false, true));
        assert!(!connection_present(false, true, false, false, false, false));
    }
    #[test]
    fn wifi_labels_and_signal_are_bounded() {
        assert_eq!(
            (0..=7).map(wifi_state_label).collect::<Vec<_>>(),
            [
                "not_ready",
                "connected",
                "ad_hoc",
                "disconnecting",
                "disconnected",
                "associating",
                "discovering",
                "authenticating"
            ]
        );
        assert_eq!(wifi_state_label(99), "unknown");
        assert_eq!(signal_percent(0), 0);
        assert_eq!(signal_percent(42), 42);
        assert_eq!(signal_percent(100), 100);
        assert_eq!(signal_percent(101), 100);
        assert_eq!(signal_percent(u32::MAX), 100);
    }
    #[test]
    fn ssid_decoding_uses_declared_length_and_ignores_padding() {
        let mut bytes = [0u8; 32];
        bytes[..4].copy_from_slice(b"Home");
        assert_eq!(ssid_text(4, &bytes).as_deref(), Some("Home"));
        assert_eq!(ssid_text(0, &bytes), None);
        assert_eq!(
            ssid_text(40, &[b'a'; 32]).as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }
    #[test]
    fn adapter_selection_and_vpn_heuristics_are_deliberate() {
        assert_eq!(
            adapter_name("Ethernet", "Intel I219", false).as_deref(),
            Some("Ethernet")
        );
        assert_eq!(
            adapter_name("  ", "Intel I219", false).as_deref(),
            Some("Intel I219")
        );
        assert_eq!(adapter_name("", "", false), None);
        assert_eq!(
            adapter_name("Ethernet 2", "TAP-Windows Adapter V9", true).as_deref(),
            Some("TAP-Windows Adapter V9")
        );
        assert!(is_vpn_adapter(6, "Ethernet 2", "TAP-Windows Adapter V9"));
        assert!(is_vpn_adapter(23, "PPP", "WAN Miniport"));
        assert!(is_vpn_adapter(6, "Tailscale", "Tailscale Tunnel"));
        assert!(is_vpn_adapter(
            6,
            "Ethernet 2",
            "PANGP Virtual Ethernet Adapter Secure"
        ));
        assert!(is_vpn_adapter(
            6,
            "Ethernet 3",
            "Palo Alto Networks GlobalProtect"
        ));
        assert!(!is_vpn_adapter(
            131,
            "Teredo",
            "Microsoft Teredo Tunneling Adapter"
        ));
        assert!(!is_vpn_adapter(
            6,
            "Ethernet",
            "Intel(R) Ethernet Connection I219-LM"
        ));
    }
    #[test]
    fn native_readonly_provider_smoke() {
        let first = snapshot("network").expect("Windows interface table");
        let second = snapshot("network").expect("Second Windows interface sample");
        assert!([
            "loading",
            "live",
            "empty",
            "unsupported",
            "permission-denied"
        ]
        .contains(&first["status"].as_str().unwrap()));
        assert!([
            "loading",
            "live",
            "empty",
            "unsupported",
            "permission-denied"
        ]
        .contains(&second["status"].as_str().unwrap()));
        if second["status"] == "live" {
            assert!(second["data"]["downloadBytesPerSecond"].as_f64().unwrap() >= 0.0);
            assert!(second["data"]["uploadBytesPerSecond"].as_f64().unwrap() >= 0.0);
        }
        assert!(["wifi", "ethernet", "vpn", "none"]
            .contains(&second["data"]["connectionType"].as_str().unwrap()));
        assert!(second["data"]["wifi"]["connected"].is_boolean());
        assert!(
            second["data"]["wifi"]["ssid"].is_string() || second["data"]["wifi"]["ssid"].is_null()
        );
        let signal = &second["data"]["wifi"]["signalPercent"];
        assert!(signal.is_null() || signal.as_u64().map(|value| value <= 100).unwrap_or(false));
        assert!(second["data"]["wifi"]["state"].is_string());
        assert!(second["data"]["ethernet"]["connected"].is_boolean());
        assert!(
            second["data"]["ethernet"]["adapter"].is_string()
                || second["data"]["ethernet"]["adapter"].is_null()
        );
        assert!(second["data"]["vpn"]["connected"].is_boolean());
        assert!(second["data"]["bluetooth"]["available"].is_boolean());
        assert!(second["data"]["bluetooth"]["enabled"].is_boolean());
        assert!(
            second["data"]["bluetooth"]["name"].is_string()
                || second["data"]["bluetooth"]["name"].is_null()
        );
        let media = snapshot("media").expect("Windows media session access");
        assert!(["live", "empty", "permission-denied"].contains(&media["status"].as_str().unwrap()));
        if media["status"] == "live" {
            assert!(media["data"]["title"].is_string());
            assert!(media["data"]["canToggle"].is_boolean());
        }
        println!(
            "Native readonly smoke: network first={}, second={}, adapterCount={}, type={}, wifi={}, ethernet={}, vpn={}, bluetooth={}; media={}",
            first["status"],
            second["status"],
            second["data"]["interfaces"].as_array().map_or(0, Vec::len),
            second["data"]["connectionType"],
            second["data"]["wifi"]["state"],
            second["data"]["ethernet"]["connected"],
            second["data"]["vpn"]["connected"],
            second["data"]["bluetooth"]["available"],
            media["status"]
        );
    }
}
