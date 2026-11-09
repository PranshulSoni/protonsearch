//! Monitor brightness via Dxva2 (user-level, no admin). Works for internal
//! laptop panels that expose brightness controls; external monitors may not
//! expose it. Every call returns a structured error when unsupported.

use anyhow::anyhow;
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitors, GetMonitorBrightness, GetNumberOfPhysicalMonitorsFromHMONITOR,
    GetPhysicalMonitorsFromHMONITOR, SetMonitorBrightness, PHYSICAL_MONITOR,
};
use windows::Win32::Foundation::{BOOL, HANDLE, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};

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

unsafe extern "system" fn enum_monitor(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
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
            // PHYSICAL_MONITOR is a packed struct: copy fields by value instead
            // of taking references to packed fields.
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

fn brightness_of(monitor: &PhysicalMonitor) -> anyhow::Result<u8> {
    let mut minimum = 0u32;
    let mut current = 0u32;
    let mut maximum = 0u32;
    let ok = unsafe { GetMonitorBrightness(monitor.handle, &mut minimum, &mut current, &mut maximum) };
    if ok == 0 || maximum == 0 {
        return Err(anyhow!("monitor does not expose brightness control"));
    }
    let value = ((current.saturating_sub(minimum)) as f64
        / (maximum.saturating_sub(minimum)).max(1) as f64
        * 100.0)
        .round() as u8;
    Ok(value.min(100))
}

/// True when at least one monitor exposes brightness control.
pub fn supported() -> bool {
    all_physical_monitors()
        .iter()
        .any(|monitor| brightness_of(monitor).is_ok())
}

/// Monitor descriptions that expose brightness control.
pub fn monitors() -> Vec<String> {
    all_physical_monitors()
        .iter()
        .filter(|monitor| brightness_of(monitor).is_ok())
        .map(|monitor| monitor.description.clone())
        .collect()
}

/// Current brightness (0..=100) of the first brightness-capable monitor.
pub fn get_brightness() -> anyhow::Result<u8> {
    let monitors = all_physical_monitors();
    for monitor in &monitors {
        if let Ok(value) = brightness_of(monitor) {
            return Ok(value);
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
        let mut minimum = 0u32;
        let mut current = 0u32;
        let mut maximum = 0u32;
        let ok =
            unsafe { GetMonitorBrightness(monitor.handle, &mut minimum, &mut current, &mut maximum) };
        if ok == 0 || maximum == 0 {
            continue;
        }
        let target =
            minimum + ((maximum.saturating_sub(minimum)) as f64 * percent as f64 / 100.0).round() as u32;
        if unsafe { SetMonitorBrightness(monitor.handle, target) } == 0 {
            last_error = Some(anyhow!(
                "SetMonitorBrightness failed for {}",
                monitor.description
            ));
            continue;
        }
        changed += 1;
    }
    if changed > 0 {
        Ok(())
    } else {
        Err(last_error.unwrap_or_else(|| anyhow!("no monitor exposes brightness control")))
    }
}
