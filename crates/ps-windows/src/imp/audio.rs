//! Core Audio adapters: render endpoint enumeration, default-device switching,
//! master volume and mute. All calls are user-level (no admin) and bounded; every
//! failure returns a structured error so the UI can show an honest unsupported
//! state instead of pretending success.

use anyhow::{bail, Context};
use windows::core::{GUID, Interface, PROPVARIANT, PWSTR};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eConsole, eMultimedia, eRender, IMMDevice, IMMDeviceCollection, IMMDeviceEnumerator,
    DEVICE_STATE_ACTIVE, ERole,
};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL, STGM_READ};
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

use super::util::ComGuard;
use crate::{AudioOutputDevice, AudioState};

/// CLSID_MMDeviceEnumerator (not exported by the windows crate).
const CLSID_MM_DEVICE_ENUMERATOR: GUID = GUID::from_u128(0xBCDE0395_E52F_467C_8E3D_C4579291692E);
/// CLSID_PolicyConfigClient + IPolicyConfig (undocumented but stable Win7+ API
/// used by the Windows sound UI for default-device switching).
const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870AF99C_171D_4F9E_AF0D_E63DF40C2BC9);
const IID_IPOLICY_CONFIG: GUID = GUID::from_u128(0xF8679F50_850A_41CF_9C72_430F290290C8);

/// PKEY_Device_FriendlyName {a45c254e-df1c-4efd-8020-67d146a850e0}, pid 14.
const PKEY_DEVICE_FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};

fn enumerator() -> anyhow::Result<IMMDeviceEnumerator> {
    unsafe { CoCreateInstance(&CLSID_MM_DEVICE_ENUMERATOR, None, CLSCTX_ALL) }
        .context("CoCreateInstance(MMDeviceEnumerator)")
}

fn default_render_device() -> anyhow::Result<IMMDevice> {
    let enumerator = enumerator()?;
    unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) }
        .context("GetDefaultAudioEndpoint(render)")
}

fn device_id(device: &IMMDevice) -> anyhow::Result<String> {
    unsafe {
        let id: PWSTR = device.GetId().context("IMMDevice::GetId")?;
        let text = id.to_string().unwrap_or_default();
        CoTaskMemFree(Some(id.0 as *const _));
        if text.is_empty() {
            bail!("audio endpoint has an empty id");
        }
        Ok(text)
    }
}

fn friendly_name(device: &IMMDevice) -> anyhow::Result<String> {
    unsafe {
        let store = device
            .OpenPropertyStore(STGM_READ)
            .context("IMMDevice::OpenPropertyStore")?;
        let mut value: PROPVARIANT = store
            .GetValue(&PKEY_DEVICE_FRIENDLY_NAME)
            .context("IPropertyStore::GetValue(friendly name)")?;
        let wide = PropVariantToStringAlloc(&value);
        let _ = PropVariantClear(&mut value);
        let wide = wide.context("PropVariantToStringAlloc")?;
        let name = wide.to_string().unwrap_or_default();
        CoTaskMemFree(Some(wide.0 as *const _));
        Ok(name)
    }
}

fn collection() -> anyhow::Result<IMMDeviceCollection> {
    unsafe {
        enumerator()?
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .context("EnumAudioEndpoints(render)")
    }
}

fn endpoint_volume() -> anyhow::Result<IAudioEndpointVolume> {
    let device = default_render_device()?;
    unsafe { device.Activate(CLSCTX_ALL, None) }.context("IMMDevice::Activate(IAudioEndpointVolume)")
}

/// Enumerate active output endpoints with friendly names and default marking.
pub fn list_output_devices() -> anyhow::Result<Vec<AudioOutputDevice>> {
    let _com = ComGuard::new();
    let default_id = default_render_device()
        .ok()
        .and_then(|device| device_id(&device).ok());
    let collection = collection()?;
    let count = unsafe { collection.GetCount() }.context("IMMDeviceCollection::GetCount")?;
    let mut devices = Vec::with_capacity(count as usize);
    for index in 0..count {
        let device = match unsafe { collection.Item(index) } {
            Ok(device) => device,
            Err(_) => continue,
        };
        let Ok(id) = device_id(&device) else {
            continue;
        };
        let name = friendly_name(&device).unwrap_or_else(|_| "Unknown output device".to_string());
        devices.push(AudioOutputDevice {
            is_default: default_id.as_deref() == Some(id.as_str()),
            id,
            name,
        });
    }
    Ok(devices)
}

/// Current default output device (id + name).
pub fn default_output_device() -> anyhow::Result<AudioOutputDevice> {
    let _com = ComGuard::new();
    let device = default_render_device()?;
    let id = device_id(&device)?;
    let name = friendly_name(&device).unwrap_or_else(|_| "Unknown output device".to_string());
    Ok(AudioOutputDevice {
        id,
        name,
        is_default: true,
    })
}

/// Set master volume (0.0..=1.0) of the default output endpoint.
pub fn set_volume(volume: f32) -> anyhow::Result<()> {
    let _com = ComGuard::new();
    let endpoint = endpoint_volume()?;
    let level = volume.clamp(0.0, 1.0);
    unsafe { endpoint.SetMasterVolumeLevelScalar(level, std::ptr::null()) }
        .context("SetMasterVolumeLevelScalar")
}

pub fn get_volume() -> anyhow::Result<f32> {
    let _com = ComGuard::new();
    let endpoint = endpoint_volume()?;
    unsafe { endpoint.GetMasterVolumeLevelScalar() }.context("GetMasterVolumeLevelScalar")
}

pub fn set_mute(muted: bool) -> anyhow::Result<()> {
    let _com = ComGuard::new();
    let endpoint = endpoint_volume()?;
    let value = windows::Win32::Foundation::BOOL(if muted { 1 } else { 0 });
    unsafe { endpoint.SetMute(value, std::ptr::null()) }.context("IAudioEndpointVolume::SetMute")
}

pub fn get_mute() -> anyhow::Result<bool> {
    let _com = ComGuard::new();
    let endpoint = endpoint_volume()?;
    unsafe { endpoint.GetMute() }
        .map(|muted| muted.as_bool())
        .context("IAudioEndpointVolume::GetMute")
}

/// Current volume + mute of the default output endpoint.
pub fn state() -> anyhow::Result<AudioState> {
    Ok(AudioState {
        volume: get_volume()?,
        muted: get_mute()?,
    })
}

/// Switch the default output endpoint. Uses the PolicyConfig COM object that the
/// Windows sound UI uses; it is not a public SDK interface, so failures are
/// surfaced as a structured error (UI should then open Windows sound settings).
pub fn set_default_output(device_id: &str) -> anyhow::Result<()> {
    let _com = ComGuard::new();
    unsafe {
        let unknown: windows::core::IUnknown =
            CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)
                .context("CoCreateInstance(PolicyConfigClient)")?;
        let raw = windows::core::Interface::as_raw(&unknown);
        let mut policy: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = (Interface::vtable(&unknown).QueryInterface)(raw, &IID_IPOLICY_CONFIG, &mut policy);
        if hr.is_err() || policy.is_null() {
            bail!("IPolicyConfig is not available on this build (HRESULT {hr:?})");
        }
        // IPolicyConfig vtable after IUnknown:
        // 3 GetMixFormat, 4 GetDeviceFormat, 5 ResetDeviceFormat, 6 SetDeviceFormat,
        // 7 GetProcessingPeriod, 8 SetProcessingPeriod, 9 GetShareMode,
        // 10 SetShareMode, 11 GetPropertyValue, 12 SetPropertyValue,
        // 13 SetDefaultEndpoint(deviceId, role), 14 SetEndpointVisibility.
        type SetDefaultEndpointFn =
            unsafe extern "system" fn(*mut std::ffi::c_void, *const u16, ERole) -> windows::core::HRESULT;
        let vtable = *(policy as *mut *const usize);
        let set_default: SetDefaultEndpointFn = std::mem::transmute(*vtable.add(13));
        let wide: Vec<u16> = device_id.encode_utf16().chain(std::iter::once(0)).collect();
        for role in [eConsole, eMultimedia] {
            let hr = set_default(policy, wide.as_ptr(), role);
            if hr.is_err() {
                let _ = (Interface::vtable(&unknown).Release)(raw);
                bail!("IPolicyConfig::SetDefaultEndpoint failed: {hr:?}");
            }
        }
        let _ = (Interface::vtable(&unknown).Release)(raw);
    }
    Ok(())
}

/// True when the default output device supports master volume control.
pub fn volume_supported() -> bool {
    let _com = ComGuard::new();
    endpoint_volume().is_ok()
}

/// True when at least one active render endpoint exists.
pub fn output_supported() -> bool {
    let _com = ComGuard::new();
    collection()
        .map(|collection| unsafe { collection.GetCount() }.map(|count| count > 0).unwrap_or(false))
        .unwrap_or(false)
}

/// True when the PolicyConfig default-switch interface can be created.
pub fn output_switch_supported() -> bool {
    let _com = ComGuard::new();
    unsafe {
        let unknown: windows::core::IUnknown =
            match CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL) {
                Ok(unknown) => unknown,
                Err(_) => return false,
            };
        let raw = windows::core::Interface::as_raw(&unknown);
        let mut policy: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = (Interface::vtable(&unknown).QueryInterface)(raw, &IID_IPOLICY_CONFIG, &mut policy);
        let _ = (Interface::vtable(&unknown).Release)(raw);
        hr.is_ok() && !policy.is_null()
    }
}
