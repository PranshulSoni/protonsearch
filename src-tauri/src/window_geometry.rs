use serde::Deserialize;
use windows::Win32::{Foundation::HWND, Graphics::Gdi::*};

#[derive(Clone, Deserialize)]
pub struct Region {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default = "default_radius")]
    pub radius: f64,
}
fn default_radius() -> f64 {
    8.0
}
#[derive(Clone)]
pub struct DockGeometry {
    pub width: f64,
    pub height: f64,
    pub regions: Vec<Region>,
}

impl DockGeometry {
    pub fn validate(&self) -> Result<(), String> {
        if !self.width.is_finite()
            || !(64.0..=16384.0).contains(&self.width)
            || !self.height.is_finite()
            || !(64.0..=1000.0).contains(&self.height)
            || self.regions.is_empty()
            || self.regions.len() > 16
            || self.regions.iter().any(|r| {
                ![r.x, r.y, r.width, r.height, r.radius]
                    .iter()
                    .all(|v| v.is_finite())
                    || !(0.0..=16384.0).contains(&r.radius)
                    || r.width <= 0.0
                    || r.height <= 0.0
                    || r.x < -16384.0
                    || r.y < -1000.0
                    || r.x + r.width > 32768.0
                    || r.y + r.height > 2000.0
            })
        {
            return Err("Invalid dock geometry".into());
        }
        Ok(())
    }
}

// All placement uses physical pixels from rcWork; CSS measurements convert once.
pub fn bounds(
    area: [i32; 4],
    scale: f64,
    width: f64,
    height: f64,
    dock: bool,
) -> Result<[i32; 4], String> {
    let [left, top, right, bottom] = area;
    if !scale.is_finite() || !(0.5..=8.0).contains(&scale) || right - left < 64 || bottom - top < 64
    {
        return Err("Invalid monitor geometry".into());
    }
    let gap = (if dock { 10.0 } else { 16.0 } * scale).ceil() as i32;
    let w = (width * scale)
        .ceil()
        .min(f64::from(right - left - 2 * gap))
        .max(1.0) as i32;
    let h = (height * scale)
        .ceil()
        .min(f64::from(bottom - top - 2 * gap))
        .max(1.0) as i32;
    Ok([
        left + (right - left - w) / 2,
        if dock {
            bottom - gap - h
        } else {
            // Reserve the maximum results height so typing cannot move the input.
            let anchor_height = (500.0 * scale).ceil() as i32;
            top + gap + (bottom - top - anchor_height - 2 * gap).max(0) / 3
        },
        w,
        h,
    ])
}

pub fn work_area(window: &tauri::WebviewWindow) -> Result<([i32; 4], f64), String> {
    let scale = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("Monitor unavailable")?
        .scale_factor();
    // SAFETY: monitor info has the required size; HWND is obtained from this window.
    unsafe {
        let monitor = MonitorFromWindow(
            HWND(window.hwnd().map_err(|e| e.to_string())?.0),
            MONITOR_DEFAULTTONEAREST,
        );
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return Err("Monitor work area unavailable".into());
        }
        let r = info.rcWork;
        Ok(([r.left, r.top, r.right, r.bottom], scale))
    }
}

pub fn place(window: &tauri::WebviewWindow, b: [i32; 4]) -> Result<(), String> {
    let size = window.outer_size().map_err(|e| e.to_string())?;
    if size.width != b[2] as u32 || size.height != b[3] as u32 {
        window
            .set_size(tauri::PhysicalSize::new(b[2] as u32, b[3] as u32))
            .map_err(|e| e.to_string())?;
    }
    let position = window.outer_position().map_err(|e| e.to_string())?;
    if position.x != b[0] || position.y != b[1] {
        window
            .set_position(tauri::PhysicalPosition::new(b[0], b[1]))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn content_region(geometry: &DockGeometry, b: [i32; 4], scale: f64) -> Result<HRGN, String> {
    geometry.validate()?;
    // Build before mutating the host, and transfer GDI ownership only on success.
    // SAFETY: region coordinates are bounded, handles are released on every failure.
    unsafe {
        let union = CreateRectRgn(0, 0, 0, 0);
        if union.0.is_null() {
            return Err("Cannot create dock input region".into());
        }
        let mut visible = false;
        for r in &geometry.regions {
            let x = (r.x * scale).floor().max(0.0).min(f64::from(b[2])) as i32;
            let y = (r.y * scale).floor().max(0.0).min(f64::from(b[3])) as i32;
            let right = ((r.x + r.width) * scale)
                .ceil()
                .max(0.0)
                .min(f64::from(b[2])) as i32;
            let bottom = ((r.y + r.height) * scale)
                .ceil()
                .max(0.0)
                .min(f64::from(b[3])) as i32;
            if right <= x || bottom <= y {
                continue;
            }
            let region = CreateRoundRectRgn(
                x,
                y,
                right + 1,
                bottom + 1,
                (2.0 * r.radius.min(r.width.min(r.height) / 2.0) * scale).round() as i32,
                (2.0 * r.radius.min(r.width.min(r.height) / 2.0) * scale).round() as i32,
            );
            if region.0.is_null() {
                let _ = DeleteObject(union);
                return Err("Cannot create dock content region".into());
            }
            let combined = CombineRgn(union, union, region, RGN_OR);
            let _ = DeleteObject(region);
            if combined == RGN_ERROR {
                let _ = DeleteObject(union);
                return Err("Cannot combine dock content region".into());
            }
            visible = true;
        }
        if !visible {
            let _ = DeleteObject(union);
            return Err("Dock content is outside the window".into());
        }
        Ok(union)
    }
}

pub fn fit(
    window: &tauri::WebviewWindow,
    geometry: &DockGeometry,
    reveal: bool,
) -> Result<(), String> {
    geometry.validate()?;
    let (area, scale) = work_area(window)?;
    let b = bounds(area, scale, geometry.width, geometry.height, true)?;
    let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
    let region = content_region(geometry, b, scale)?;
    // SAFETY: on success Windows owns this region; otherwise we release it.
    unsafe {
        if let Err(error) = place(window, b) {
            let _ = DeleteObject(region);
            return Err(error);
        }
        if SetWindowRgn(hwnd, region, true) == 0 {
            let _ = DeleteObject(region);
            return Err("Cannot apply dock content region".into());
        }
    }
    if reveal {
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn show_fallback(window: &tauri::WebviewWindow, reveal: bool) -> Result<(), String> {
    let placement = work_area(window)
        .and_then(|(area, scale)| place(window, bounds(area, scale, 600.0, 132.0, true)?));
    // An initial region failure must leave a visible error surface and controls.
    // SAFETY: a null region removes clipping; Windows owns any previous region.
    let reset = window
        .hwnd()
        .map_err(|e| e.to_string())
        .and_then(|hwnd| unsafe {
            if SetWindowRgn(HWND(hwnd.0), HRGN::default(), true) == 0 {
                Err("Cannot remove dock fallback clipping".into())
            } else {
                Ok(())
            }
        });
    // Even failed monitor discovery must not suppress the error surface.
    if reveal {
        window.show().map_err(|e| e.to_string())?;
    }
    reset?;
    placement
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dock_is_content_sized_centered_above_existing_taskbar() {
        assert_eq!(
            bounds([0, 0, 3000, 1950], 1.0, 1200.0, 132.0, true).unwrap(),
            [900, 1808, 1200, 132]
        );
        assert_eq!(
            bounds([-2000, 0, 0, 1500], 1.5, 800.0, 132.0, true).unwrap(),
            [-1600, 1287, 1200, 198]
        );
    }
    #[test]
    fn search_and_oversized_dock_stay_in_work_area() {
        assert_eq!(
            bounds([0, 0, 640, 480], 1.0, 638.0, 500.0, false).unwrap(),
            [16, 16, 608, 448]
        );
        let b = bounds([0, 0, 3000, 1950], 1.0, 20000.0, 132.0, true).unwrap();
        assert_eq!(b, [10, 1808, 2980, 132]);
        assert!(bounds([0, 0, 3000, 1950], f64::NAN, 638.0, 500.0, false).is_err());
        for scale in [1.0, 1.5] {
            let small = bounds([0, 0, 3000, 1950], scale, 638.0, 174.0, false).unwrap();
            let large = bounds([0, 0, 3000, 1950], scale, 638.0, 500.0, false).unwrap();
            assert_eq!(
                small[1], large[1],
                "Result changes must not move the search input"
            );
        }
    }
    #[test]
    fn reject_unbounded_or_empty_frontend_geometry() {
        let mut g = DockGeometry {
            width: 1200.0,
            height: 132.0,
            regions: vec![Region {
                x: 8.0,
                y: 42.0,
                width: 1184.0,
                height: 82.0,
                radius: 20.0,
            }],
        };
        assert!(g.validate().is_ok());
        for radius in [f64::NAN, -1.0, 16385.0] {
            g.regions[0].radius = radius;
            assert!(g.validate().is_err());
        }
        g.regions[0].radius = 20.0;
        g.regions[0].width = f64::INFINITY;
        assert!(g.validate().is_err());
        g.regions.clear();
        assert!(g.validate().is_err());
    }

    #[test]
    fn older_region_payloads_keep_eight_pixel_radius() {
        let region: Region =
            serde_json::from_value(serde_json::json!({"x":8,"y":42,"width":1184,"height":82}))
                .unwrap();
        assert_eq!(region.radius, 8.0);
    }

    #[test]
    fn native_region_excludes_transparent_margin_and_keeps_controls() {
        let geometry = DockGeometry {
            width: 1200.0,
            height: 132.0,
            regions: vec![
                Region {
                    x: 8.0,
                    y: 42.0,
                    width: 1184.0,
                    height: 82.0,
                    radius: 20.0,
                },
                Region {
                    x: 900.0,
                    y: 3.0,
                    width: 280.0,
                    height: 31.0,
                    radius: 8.0,
                },
            ],
        };
        for scale in [1.0, 1.5] {
            let b = bounds([0, 0, 3000, 1950], scale, 1200.0, 132.0, true).unwrap();
            let region = content_region(&geometry, b, scale).unwrap();
            // SAFETY: the local GDI region has not been transferred to a window.
            unsafe {
                assert!(
                    !PtInRegion(region, (50.0 * scale) as i32, (15.0 * scale) as i32).as_bool()
                );
                assert!(
                    PtInRegion(region, (1000.0 * scale) as i32, (15.0 * scale) as i32).as_bool()
                );
                assert!(PtInRegion(region, (50.0 * scale) as i32, (70.0 * scale) as i32).as_bool());
                assert!(
                    !PtInRegion(region, (11.0 * scale) as i32, (45.0 * scale) as i32).as_bool()
                );
                assert!(PtInRegion(region, (23.0 * scale) as i32, (57.0 * scale) as i32).as_bool());
                assert!(DeleteObject(region).as_bool());
            }
        }
    }
}
