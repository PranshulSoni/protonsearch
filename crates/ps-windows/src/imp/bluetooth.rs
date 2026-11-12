//! Bluetooth adapter: radio state/toggle and paired-device enumeration. Uses the
//! shared persistent WinRT worker (see imp::radios) so repeated calls are stable.
//! Pairing stays in the OS UI; this module never pairs/connects/changes pairing.

use anyhow::Result;
use windows::Devices::Radios::RadioKind;

use super::radios;
use crate::{BluetoothDeviceInfo, RadioStateInfo};

pub(crate) fn radio_state() -> Result<RadioStateInfo> {
    let state = radios::radio_state(RadioKind::Bluetooth)?;
    Ok(RadioStateInfo {
        available: state.available,
        enabled: state.enabled,
    })
}

pub(crate) fn set_radio(enabled: bool) -> Result<()> {
    radios::set_radio(RadioKind::Bluetooth, enabled)
}

pub(crate) fn supported() -> bool {
    radios::has_radio(RadioKind::Bluetooth)
}

/// Paired Bluetooth devices (name/id/connected). Read-only: no pairing/connecting.
pub(crate) fn paired_devices() -> Result<Vec<BluetoothDeviceInfo>> {
    radios::bluetooth_paired_devices()
}
