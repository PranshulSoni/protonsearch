use crate::known_folders;
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
        Foundation::{BOOL, HANDLE},
        Media::Audio::Endpoints::IAudioEndpointVolume,
        Media::Audio::{eConsole, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator},
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
        System::{
            Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ},
            WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
        },
        UI::Shell::PropertiesSystem::PROPERTYKEY,
    },
};

const PKEY_DEVICE_FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: windows::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};

// WinRT radio/media factories and Core Audio share process-wide singletons. Blocking
// workers can request different providers at the same time, so keep native work
// serialized rather than racing those singletons.
static NATIVE_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn envelope(status: &str, data: Value, message: &str) -> Value {
    json!({"status":status,"data":data,"message":message,
        "updatedAt":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64})
}

struct Apartment;
impl Apartment {
    fn initialize() -> windows::core::Result<Self> {
        // WinRT keeps process-wide state behind the radio and media managers. Uninitializing
        // the apartment while another blocking worker still uses that state crashes the
        // process, so initialize once per thread and keep it alive for the thread's lifetime.
        thread_local! {
            static APARTMENT: Result<(), windows::core::HRESULT> =
                unsafe { RoInitialize(RO_INIT_MULTITHREADED).map_err(|error| error.code()) };
        }
        APARTMENT.with(|ready| match ready {
            Ok(()) => Ok(Self),
            Err(code) => Err(windows::core::Error::from(*code)),
        })
    }
}

pub fn snapshot(provider: &str) -> Result<Value, String> {
    let _guard = NATIVE_LOCK
        .lock()
        .map_err(|_| "Native provider lock is poisoned")?;
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
        "volume" => match volume() {
            Ok(value) => Ok(value),
            Err(error) if error.code().0 as u32 == 0x80070005 => Ok(envelope(
                "permission-denied",
                Value::Null,
                "Windows denied audio endpoint access",
            )),
            Err(error) => Ok(envelope(
                "unsupported",
                Value::Null,
                &format!("Audio endpoint unavailable: {error}"),
            )),
        },
        "trash" => match trash() {
            Ok(value) => Ok(value),
            Err(error) if error.code().0 as u32 == 0x80070005 => Ok(envelope(
                "permission-denied",
                Value::Null,
                "Windows denied access to the Recycle Bin",
            )),
            Err(error) => Ok(envelope(
                "unsupported",
                Value::Null,
                &format!("Recycle Bin unavailable: {error}"),
            )),
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
    let status = playback.PlaybackStatus()?;
    let album = properties
        .AlbumTitle()
        .ok()
        .and_then(|value| non_empty_text(&value.to_string()));
    let album_artist = properties
        .AlbumArtist()
        .ok()
        .and_then(|value| non_empty_text(&value.to_string()));
    let timeline = session.GetTimelineProperties().ok();
    let (position_ms, duration_ms) = timeline_millis(
        timeline
            .as_ref()
            .and_then(|value| value.Position().ok())
            .map(|value| value.Duration),
        timeline
            .as_ref()
            .and_then(|value| value.StartTime().ok())
            .map(|value| value.Duration),
        timeline
            .as_ref()
            .and_then(|value| value.EndTime().ok())
            .map(|value| value.Duration),
    );
    let playback_rate = playback
        .PlaybackRate()
        .ok()
        .and_then(|value| value.Value().ok());
    Ok(envelope(
        "live",
        json!({
            "title":properties.Title()?.to_string(), "artist":properties.Artist()?.to_string(),
            "album":album, "albumArtist":album_artist,
            "source":session.SourceAppUserModelId()?.to_string(),
            "playing":status == PlaybackStatus::Playing,
            "playbackStatus":playback_status_label(status),
            "playbackRate":playback_rate,
            "positionMs":position_ms, "durationMs":duration_ms,
            "canToggle":controls.IsPlayPauseToggleEnabled()?,
            "canNext":controls.IsNextEnabled()?, "canPrevious":controls.IsPreviousEnabled()?
        }),
        "",
    ))
}

fn non_empty_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
fn timeline_millis(
    position: Option<i64>,
    start: Option<i64>,
    end: Option<i64>,
) -> (Option<u64>, Option<u64>) {
    // GSMTC uses 100 ns ticks; expose milliseconds for the React timeline.
    let start = start.unwrap_or(0);
    let position = position.map(|value| value.saturating_sub(start).max(0) as u64 / 10_000);
    let duration = end
        .map(|value| value.saturating_sub(start))
        .filter(|value| *value > 0)
        .map(|value| value as u64 / 10_000);
    (position, duration)
}
fn playback_status_label(status: PlaybackStatus) -> &'static str {
    if status == PlaybackStatus::Closed {
        "closed"
    } else if status == PlaybackStatus::Opened {
        "opened"
    } else if status == PlaybackStatus::Changing {
        "changing"
    } else if status == PlaybackStatus::Stopped {
        "stopped"
    } else if status == PlaybackStatus::Playing {
        "playing"
    } else if status == PlaybackStatus::Paused {
        "paused"
    } else {
        "unknown"
    }
}

fn volume() -> windows::core::Result<Value> {
    let _apartment = Apartment::initialize()?;
    let Some((endpoint, device)) = default_output()? else {
        return Ok(envelope("empty", Value::Null, "No audio output device"));
    };
    let level = unsafe { endpoint.GetMasterVolumeLevelScalar()? };
    let muted = unsafe { endpoint.GetMute()? }.as_bool();
    Ok(envelope(
        "live",
        json!({"level":volume_percent(level),"muted":muted,"device":device}),
        "",
    ))
}
fn default_output() -> windows::core::Result<Option<(IAudioEndpointVolume, String)>> {
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
    let device: IMMDevice = match unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) } {
        Ok(device) => device,
        Err(error) if error.code().0 as u32 == 0x80070490 => return Ok(None),
        Err(error) => return Err(error),
    };
    let endpoint: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None)? };
    Ok(Some((endpoint, device_name(&device))))
}
fn device_name(device: &IMMDevice) -> String {
    let name = unsafe {
        device
            .OpenPropertyStore(STGM_READ)
            .ok()
            .and_then(|store| store.GetValue(&PKEY_DEVICE_FRIENDLY_NAME).ok())
            .map(|value| value.to_string())
    };
    name.and_then(|value| non_empty_text(&value))
        .unwrap_or_else(|| "Default output device".into())
}
fn volume_percent(scalar: f32) -> u8 {
    (scalar * 100.0).round().clamp(0.0, 100.0) as u8
}
fn volume_set_percent(action: &str) -> Option<u8> {
    action
        .strip_prefix("volume:set:")?
        .parse::<i64>()
        .ok()
        .map(|value| value.clamp(0, 100) as u8)
}
fn volume_action(action: &str) -> windows::core::Result<()> {
    let _apartment = Apartment::initialize()?;
    let Some((endpoint, _)) = default_output()? else {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(0x80070490u32 as i32),
            "No audio output device",
        ));
    };
    if action == "volume:up" || action == "volume:down" {
        let level = unsafe { endpoint.GetMasterVolumeLevelScalar()? };
        let delta = if action == "volume:up" { 0.05 } else { -0.05 };
        unsafe {
            endpoint
                .SetMasterVolumeLevelScalar((level + delta).clamp(0.0, 1.0), std::ptr::null())?
        };
    } else if action == "volume:mute" {
        let muted = unsafe { endpoint.GetMute()? }.as_bool();
        unsafe { endpoint.SetMute(BOOL::from(!muted), std::ptr::null())? };
    } else if let Some(percent) = volume_set_percent(action) {
        unsafe { endpoint.SetMasterVolumeLevelScalar(percent as f32 / 100.0, std::ptr::null())? };
    } else {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(0x80070057u32 as i32),
            "Unsupported widget action",
        ));
    }
    Ok(())
}
fn trash() -> windows::core::Result<Value> {
    use windows::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};
    let mut info = SHQUERYRBINFO {
        cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
        i64Size: 0,
        i64NumItems: 0,
    };
    unsafe { SHQueryRecycleBinW(windows::core::PCWSTR::null(), &mut info)? };
    Ok(envelope(
        "live",
        json!({
            "itemCount":info.i64NumItems.max(0) as u64,
            "sizeBytes":info.i64Size.max(0) as u64,
            "lastEmptiedAt":Value::Null
        }),
        "",
    ))
}
fn empty_trash() -> windows::core::Result<()> {
    use windows::Win32::{
        Foundation::HWND,
        UI::Shell::{SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND},
    };
    unsafe {
        SHEmptyRecycleBinW(
            HWND::default(),
            windows::core::PCWSTR::null(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )
    }
}

pub fn action(action: &str) -> Result<(), String> {
    let _guard = NATIVE_LOCK
        .lock()
        .map_err(|_| "Native provider lock is poisoned")?;
    if let Some(uri) = settings_uri(action) {
        return open_uri(uri).map_err(|error| error.to_string());
    }
    if action.starts_with("volume:") {
        if !matches!(action, "volume:up" | "volume:down" | "volume:mute")
            && volume_set_percent(action).is_none()
        {
            return Err("Unsupported widget action".into());
        }
        return volume_action(action).map_err(|error| error.to_string());
    }
    if action == "trash:empty" {
        return empty_trash().map_err(|error| error.to_string());
    }
    if let Some(folder_id) = action.strip_prefix("folder:open:") {
        let path = known_folders::folder_path(folder_id)
            .ok_or_else(|| "Unknown known folder".to_string())?;
        let path = path
            .to_str()
            .ok_or_else(|| "Known folder path is not valid Unicode".to_string())?;
        return open_uri(path).map_err(|error| error.to_string());
    }
    if !["media:toggle", "media:next", "media:previous"].contains(&action) {
        drop(_guard);
        return crate::providers::action(action);
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
    // Radio::GetRadiosAsync and repeated Radio::State reads crash the process under
    // repeated or concurrent use in windows 0.58. Enumerate and read once, then reuse
    // the observed values. The widget never sees a value we did not actually read.
    static STATUS: OnceLock<Value> = OnceLock::new();
    STATUS.get_or_init(read_bluetooth_status).clone()
}
fn read_bluetooth_status() -> Value {
    let unavailable = json!({"available":false,"enabled":false,"name":null});
    if Apartment::initialize().is_err() {
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
    fn volume_scalar_and_actions_are_bounded() {
        assert_eq!(volume_percent(0.0), 0);
        assert_eq!(volume_percent(0.5), 50);
        assert_eq!(volume_percent(0.555), 56);
        assert_eq!(volume_percent(1.0), 100);
        assert_eq!(volume_percent(-1.0), 0);
        assert_eq!(volume_percent(2.0), 100);
        assert_eq!(volume_set_percent("volume:set:0"), Some(0));
        assert_eq!(volume_set_percent("volume:set:55"), Some(55));
        assert_eq!(volume_set_percent("volume:set:100"), Some(100));
        assert_eq!(volume_set_percent("volume:set:999"), Some(100));
        assert_eq!(volume_set_percent("volume:set:-5"), Some(0));
        assert_eq!(volume_set_percent("volume:set:abc"), None);
        assert_eq!(volume_set_percent("volume:set:"), None);
        assert_eq!(volume_set_percent("volume:up"), None);
        assert_eq!(volume_set_percent(""), None);
    }
    #[test]
    fn media_timeline_ticks_convert_to_milliseconds() {
        assert_eq!(
            timeline_millis(Some(5_000_000), Some(0), Some(180_000_000)),
            (Some(500), Some(18_000))
        );
        assert_eq!(
            timeline_millis(Some(0), Some(10_000), Some(10_000)),
            (Some(0), None)
        );
        assert_eq!(timeline_millis(Some(10_000), None, None), (Some(1), None));
        assert_eq!(timeline_millis(None, Some(0), None), (None, None));
    }
    #[test]
    fn playback_status_labels_cover_gsmtc_states() {
        assert_eq!(playback_status_label(PlaybackStatus::Closed), "closed");
        assert_eq!(playback_status_label(PlaybackStatus::Opened), "opened");
        assert_eq!(playback_status_label(PlaybackStatus::Changing), "changing");
        assert_eq!(playback_status_label(PlaybackStatus::Stopped), "stopped");
        assert_eq!(playback_status_label(PlaybackStatus::Playing), "playing");
        assert_eq!(playback_status_label(PlaybackStatus::Paused), "paused");
    }
    #[test]
    fn optional_media_text_is_trimmed_and_empty_is_null() {
        assert_eq!(non_empty_text("  Album  ").as_deref(), Some("Album"));
        assert_eq!(non_empty_text("\t"), None);
        assert_eq!(non_empty_text(""), None);
    }
    #[test]
    fn folder_actions_are_allowlisted_and_malformed_volume_actions_are_rejected() {
        for action_id in [
            "folder:open:",
            "folder:open:..",
            "folder:open:../Windows",
            r"folder:open:C:\Windows",
            "folder:open:desktop/../..",
            "volume:set:abc",
            "volume:set:",
            "volume:bogus",
        ] {
            assert!(action(action_id).is_err(), "accepted {action_id}");
        }
        assert_eq!(
            action("volume:set:abc").unwrap_err(),
            "Unsupported widget action"
        );
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
            for key in [
                "album",
                "albumArtist",
                "positionMs",
                "durationMs",
                "playbackRate",
                "playbackStatus",
            ] {
                assert!(media["data"].get(key).is_some(), "missing {key}");
            }
            assert!(media["data"]["playbackStatus"].is_string());
        }
        let volume = snapshot("volume").expect("Windows audio endpoint access");
        assert!(["live", "empty", "unsupported", "permission-denied"]
            .contains(&volume["status"].as_str().unwrap()));
        if volume["status"] == "live" {
            assert!(volume["data"]["level"].as_u64().unwrap() <= 100);
            assert!(volume["data"]["muted"].is_boolean());
            assert!(!volume["data"]["device"].as_str().unwrap().is_empty());
        }
        let trash = snapshot("trash").expect("Windows Recycle Bin access");
        assert!(["live", "unsupported", "permission-denied"]
            .contains(&trash["status"].as_str().unwrap()));
        if trash["status"] == "live" {
            assert!(trash["data"]["itemCount"].is_u64());
            assert!(trash["data"]["sizeBytes"].is_u64());
            assert!(trash["data"]["lastEmptiedAt"].is_null());
        }
        println!(
            "Native readonly smoke: network first={}, second={}, adapterCount={}, type={}, wifi={}, ethernet={}, vpn={}, bluetooth={}; media={}; volume={} ({}%), trash={} ({} items, {} bytes)",
            first["status"],
            second["status"],
            second["data"]["interfaces"].as_array().map_or(0, Vec::len),
            second["data"]["connectionType"],
            second["data"]["wifi"]["state"],
            second["data"]["ethernet"]["connected"],
            second["data"]["vpn"]["connected"],
            second["data"]["bluetooth"]["available"],
            media["status"],
            volume["status"],
            volume["data"]["level"],
            trash["status"],
            trash["data"]["itemCount"],
            trash["data"]["sizeBytes"]
        );
    }
}
