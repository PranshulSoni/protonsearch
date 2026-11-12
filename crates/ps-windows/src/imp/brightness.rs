//! Monitor brightness via Dxva2 with a DDC/CI fallback for external monitors.
//! User-level, bounded, never blocks the UI; unsupported monitors get an honest
//! reason and the UI can fall back to Windows display settings.

use anyhow::anyhow;
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitors, GetMonitorBrightness, GetNumberOfPhysicalMonitorsFromHMONITOR,
    GetPhysicalMonitorsFromHMONITOR, GetVCPFeatureAndVCPFeatureReply, SetMonitorBrightness, SetVCPFeature,
    MC_VCP_CODE_TYPE, PHYSICAL_MONITOR,
};
use windows::Win32::Foundation::{BOOL, HANDLE, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};

use crate::{BrightnessMethod, BrightnessMonitorInfo};

const VCP_BRIGHTNESS: u8 = 0x10;

struct PhysicalMonitor {
    handle: HANDLE,
    description: String,
}

impl Drop for PhysicalMonitor {
    fn drop(&mut self) {
        let array = [PHYSICAL_MONITOR {
            hPhysicalMonitor: self.handle,
            szPhysicalMonitorDescription: [0; 128],
        }];
        unsafe {
            let _ = DestroyPhysicalMonitors(&array);
        }
    }
}

unsafe extern "system" fn enum_monitor(monitor: HMONITOR, _hdc: HDC, _rect: *mut RECT, data: LPARAM) -> BOOL {
    let monitors = &mut *(data.0 as *mut Vec<HMONITOR>);
    monitors.push(monitor);
    BOOL(1)
}

fn enumerate_monitors() -> Vec<HMONITOR> {
    let mut monitors: Vec<HMONITOR> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_monitor),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    monitors
}

fn physical_monitors(monitor: HMONITOR) -> Vec<PhysicalMonitor> {
    let mut count = 0u32;
    if unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut count) }.is_err() || count == 0 {
        return Vec::new();
    }
    let mut array: Vec<PHYSICAL_MONITOR> = (0..count)
        .map(|_| PHYSICAL_MONITOR {
            hPhysicalMonitor: HANDLE::default(),
            szPhysicalMonitorDescription: [0; 128],
        })
        .collect();
    if unsafe { GetPhysicalMonitorsFromHMONITOR(monitor, &mut array) }.is_err() {
        return Vec::new();
    }
    array
        .into_iter()
        .map(|monitor| {
            let description_units: [u16; 128] = monitor.szPhysicalMonitorDescription;
            let handle = monitor.hPhysicalMonitor;
            let end = description_units
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(description_units.len());
            PhysicalMonitor {
                handle,
                description: String::from_utf16_lossy(&description_units[..end]),
            }
        })
        .collect()
}

fn all_physical_monitors() -> Vec<PhysicalMonitor> {
    let mut monitors = Vec::new();
    for monitor in enumerate_monitors() {
        monitors.extend(physical_monitors(monitor));
    }
    monitors
}

struct MonitorReading {
    current: u8,
    minimum: u32,
    maximum: u32,
    method: BrightnessMethod,
}

fn normalize(current: u32, minimum: u32, maximum: u32) -> u8 {
    let span = maximum.saturating_sub(minimum).max(1);
    (((current.saturating_sub(minimum)) as f64 / span as f64) * 100.0)
        .round()
        .min(100.0) as u8
}

fn dxva2_reading(monitor: &PhysicalMonitor) -> Option<MonitorReading> {
    let mut minimum = 0u32;
    let mut current = 0u32;
    let mut maximum = 0u32;
    let ok = unsafe { GetMonitorBrightness(monitor.handle, &mut minimum, &mut current, &mut maximum) };
    if ok == 0 || maximum == 0 {
        return None;
    }
    Some(MonitorReading {
        current: normalize(current, minimum, maximum),
        minimum,
        maximum,
        method: BrightnessMethod::Dxva2,
    })
}

/// DDC/CI fallback via VCP code 0x10 (brightness) for external monitors.
fn ddc_reading(monitor: &PhysicalMonitor) -> Option<MonitorReading> {
    let mut code_type = MC_VCP_CODE_TYPE::default();
    let mut current = 0u32;
    let mut maximum = 0u32;
    let ok = unsafe {
        GetVCPFeatureAndVCPFeatureReply(
            monitor.handle,
            VCP_BRIGHTNESS,
            Some(&mut code_type),
            &mut current,
            Some(&mut maximum),
        )
    };
    if ok == 0 || maximum == 0 {
        return None;
    }
    Some(MonitorReading {
        current: normalize(current, 0, maximum),
        minimum: 0,
        maximum,
        method: BrightnessMethod::DdcCi,
    })
}

fn reading(monitor: &PhysicalMonitor) -> Option<MonitorReading> {
    dxva2_reading(monitor).or_else(|| ddc_reading(monitor))
}

/// True when at least one monitor exposes brightness control (Dxva2 or DDC/CI).
pub fn supported() -> bool {
    all_physical_monitors()
        .iter()
        .any(|monitor| reading(monitor).is_some())
}

/// Descriptions of monitors that expose brightness control.
pub fn monitors() -> Vec<String> {
    all_physical_monitors()
        .iter()
        .filter(|monitor| reading(monitor).is_some())
        .map(|monitor| monitor.description.clone())
        .collect()
}

/// Per-monitor brightness details (current value + control method).
pub fn list_monitors() -> Vec<BrightnessMonitorInfo> {
    all_physical_monitors()
        .iter()
        .enumerate()
        .filter_map(|(index, monitor)| {
            let reading = reading(monitor)?;
            Some(BrightnessMonitorInfo {
                index,
                name: monitor.description.clone(),
                current: reading.current,
                minimum: reading.minimum,
                maximum: reading.maximum,
                method: reading.method,
            })
        })
        .collect()
}

/// Current brightness (0..=100) of the first brightness-capable monitor.
pub fn get_brightness() -> anyhow::Result<u8> {
    for monitor in all_physical_monitors().iter() {
        if let Some(reading) = reading(monitor) {
            return Ok(reading.current);
        }
    }
    Err(anyhow!("no monitor exposes brightness control on this host"))
}

/// Set brightness (0..=100) on every brightness-capable monitor.
pub fn set_brightness(percent: u8) -> anyhow::Result<()> {
    let percent = percent.min(100);
    let monitors = all_physical_monitors();
    let mut changed = 0usize;
    let mut last_error = None;
    for monitor in &monitors {
        let Some(reading) = reading(monitor) else {
            continue;
        };
        let target = reading.minimum
            + ((reading.maximum.saturating_sub(reading.minimum)) as f64 * percent as f64 / 100.0).round()
                as u32;
        let ok = match reading.method {
            BrightnessMethod::Dxva2 => (unsafe { SetMonitorBrightness(monitor.handle, target) }) != 0,
            BrightnessMethod::DdcCi => {
                (unsafe { SetVCPFeature(monitor.handle, VCP_BRIGHTNESS, target) }) != 0
            }
        };
        if ok {
            changed += 1;
        } else {
            last_error = Some(anyhow!(
                "could not change brightness for {} ({:?})",
                monitor.description,
                reading.method
            ));
        }
    }
    if changed > 0 {
        Ok(())
    } else {
        Err(last_error.unwrap_or_else(|| anyhow!("no monitor exposes brightness control")))
    }
}
