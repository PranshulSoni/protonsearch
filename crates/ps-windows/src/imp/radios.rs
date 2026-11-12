//! WinRT radio + Bluetooth enumeration shared by Wi-Fi and Bluetooth adapters.
//!
//! Uses Windows.Devices.Radios / Windows.Devices.Bluetooth (user-level; no admin).
//! SetStateAsync is gated by RadioAccessStatus: DeniedByUser/DeniedBySystem become
//! structured errors with the matching Windows settings fallback.
//!
//! WinRT cannot be safely re-initialised/ torn down on a new short-lived thread for
//! every call (repeated Radio.GetRadiosAsync() calls from fresh MTA threads crash
//! natively in Windows). All WinRT radio/Bluetooth work therefore runs on ONE
//! persistent MTA worker thread; results are plain copyable data, requests are
//! bounded with a timeout.

use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use windows::core::HSTRING;
use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice};
use windows::Devices::Enumeration::DeviceInformation;
use windows::Devices::Radios::{Radio, RadioAccessStatus, RadioKind, RadioState};

use super::winrt::ensure_apartment;
use crate::BluetoothDeviceInfo;

const WORKER_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_DEVICES: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioControlState {
    pub available: bool,
    pub enabled: bool,
}

enum Request {
    State(RadioKind, mpsc::Sender<Result<RadioControlState>>),
    Set(RadioKind, bool, mpsc::Sender<Result<()>>),
    Has(RadioKind, mpsc::Sender<bool>),
    PairedBluetoothDevices(mpsc::Sender<Result<Vec<BluetoothDeviceInfo>>>),
}

fn worker() -> Result<&'static mpsc::Sender<Request>> {
    static WORKER: OnceLock<Option<mpsc::Sender<Request>>> = OnceLock::new();
    let slot = WORKER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<Request>();
        let spawned = std::thread::Builder::new()
            .name("ps-winrt-radio".into())
            .spawn(move || {
                // Persistent MTA apartment for the whole process lifetime.
                let _owned = ensure_apartment();
                while let Ok(request) = receiver.recv() {
                    handle_request(request);
                }
            });
        if spawned.is_err() {
            None
        } else {
            Some(sender)
        }
    });
    slot.as_ref()
        .ok_or_else(|| anyhow::anyhow!("could not start the WinRT radio worker"))
}

fn handle_request(request: Request) {
    match request {
        Request::State(kind, reply) => {
            let result = catch(|| radio_state_inner(kind));
            let _ = reply.send(result);
        }
        Request::Set(kind, enabled, reply) => {
            let result = catch(|| set_radio_inner(kind, enabled));
            let _ = reply.send(result);
        }
        Request::Has(kind, reply) => {
            let found = catch(|| Ok::<bool, anyhow::Error>(has_radio_inner(kind))).unwrap_or(false);
            let _ = reply.send(found);
        }
        Request::PairedBluetoothDevices(reply) => {
            let result = catch(paired_devices_inner);
            let _ = reply.send(result);
        }
    }
}

fn catch<T>(operation: impl FnOnce() -> Result<T>) -> Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(_) => bail!("WinRT radio operation panicked"),
    }
}

fn collect_radios() -> Result<Vec<Radio>> {
    let operation = Radio::GetRadiosAsync().context("Radio::GetRadiosAsync")?;
    let collection = operation.get().context("enumerating radios")?;
    let mut radios = Vec::new();
    for index in 0..collection.Size().unwrap_or(0) {
        if let Ok(radio) = collection.GetAt(index) {
            radios.push(radio);
        }
    }
    Ok(radios)
}

fn find_radio(kind: RadioKind) -> Result<Option<Radio>> {
    Ok(collect_radios()?
        .into_iter()
        .find(|radio| radio.Kind().map(|value| value == kind).unwrap_or(false)))
}

fn radio_state_inner(kind: RadioKind) -> Result<RadioControlState> {
    let Some(radio) = find_radio(kind)? else {
        return Ok(RadioControlState {
            available: false,
            enabled: false,
        });
    };
    let state = radio.State().context("Radio::State")?;
    Ok(RadioControlState {
        available: true,
        enabled: state == RadioState::On,
    })
}

fn set_radio_inner(kind: RadioKind, enabled: bool) -> Result<()> {
    let Some(radio) = find_radio(kind)? else {
        bail!("no {:?} radio is available on this device", kind);
    };
    let target = if enabled { RadioState::On } else { RadioState::Off };
    let operation = radio.SetStateAsync(target).context("Radio::SetStateAsync")?;
    let status = operation.get().context("changing radio state")?;
    match status {
        RadioAccessStatus::Allowed => Ok(()),
        RadioAccessStatus::DeniedByUser => {
            bail!("Windows blocked radio control (denied by user); open Windows settings to change it")
        }
        RadioAccessStatus::DeniedBySystem => {
            bail!("Windows blocked radio control (denied by system policy); open Windows settings")
        }
        other => bail!("Windows refused the radio change (RadioAccessStatus {:?})", other),
    }
}

fn has_radio_inner(kind: RadioKind) -> bool {
    find_radio(kind).map(|found| found.is_some()).unwrap_or(false)
}

fn paired_devices_inner() -> Result<Vec<BluetoothDeviceInfo>> {
    if !has_radio_inner(RadioKind::Bluetooth) {
        bail!("no Bluetooth radio is available on this device");
    }
    let selector = BluetoothDevice::GetDeviceSelectorFromPairingState(true)
        .context("BluetoothDevice::GetDeviceSelectorFromPairingState(true)")?;
    let operation = DeviceInformation::FindAllAsyncAqsFilter(&selector)
        .context("DeviceInformation::FindAllAsyncAqsFilter(paired Bluetooth)")?;
    let collection = operation.get().context("enumerating paired Bluetooth devices")?;
    let mut devices = Vec::new();
    let count = collection.Size().unwrap_or(0).min(MAX_DEVICES as u32);
    for index in 0..count {
        let Ok(information) = collection.GetAt(index) else {
            continue;
        };
        let id = information.Id().map(|id| id.to_string()).unwrap_or_default();
        let name = information
            .Name()
            .map(|name| name.to_string())
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let connected = BluetoothDevice::FromIdAsync(&HSTRING::from(&id))
            .and_then(|operation| operation.get())
            .and_then(|device| device.ConnectionStatus())
            .map(|status| status == BluetoothConnectionStatus::Connected)
            .unwrap_or(false);
        devices.push(BluetoothDeviceInfo {
            id,
            name: if name.is_empty() {
                "Unknown device".to_string()
            } else {
                name
            },
            connected,
            paired: true,
        });
    }
    devices.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| a.name.cmp(&b.name)));
    Ok(devices)
}

pub(crate) fn radio_state(kind: RadioKind) -> Result<RadioControlState> {
    let (sender, receiver) = mpsc::channel();
    worker()?
        .send(Request::State(kind, sender))
        .map_err(|_| anyhow::anyhow!("WinRT radio worker stopped"))?;
    receiver
        .recv_timeout(WORKER_TIMEOUT)
        .map_err(|_| anyhow::anyhow!("WinRT radio state timed out"))?
}

pub(crate) fn set_radio(kind: RadioKind, enabled: bool) -> Result<()> {
    let (sender, receiver) = mpsc::channel();
    worker()?
        .send(Request::Set(kind, enabled, sender))
        .map_err(|_| anyhow::anyhow!("WinRT radio worker stopped"))?;
    receiver
        .recv_timeout(WORKER_TIMEOUT)
        .map_err(|_| anyhow::anyhow!("WinRT radio change timed out"))?
}

pub(crate) fn has_radio(kind: RadioKind) -> bool {
    let (sender, receiver) = mpsc::channel();
    let Ok(sender_ref) = worker() else {
        return false;
    };
    if sender_ref.send(Request::Has(kind, sender)).is_err() {
        return false;
    }
    receiver.recv_timeout(WORKER_TIMEOUT).unwrap_or(false)
}

pub(crate) fn bluetooth_paired_devices() -> Result<Vec<BluetoothDeviceInfo>> {
    let (sender, receiver) = mpsc::channel();
    worker()?
        .send(Request::PairedBluetoothDevices(sender))
        .map_err(|_| anyhow::anyhow!("WinRT radio worker stopped"))?;
    receiver
        .recv_timeout(WORKER_TIMEOUT)
        .map_err(|_| anyhow::anyhow!("Bluetooth device enumeration timed out"))?
}
