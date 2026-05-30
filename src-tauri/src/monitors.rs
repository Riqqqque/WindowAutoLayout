use windows::{
    core::BOOL,
    Win32::{
        Foundation::{LPARAM, RECT},
        Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, MonitorFromRect, HDC, HMONITOR, MONITORINFOEXW,
            MONITOR_DEFAULTTONEAREST,
        },
        UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
    },
};

use crate::{
    errors::{AppError, AppResult},
    models::MonitorInfo,
};

pub fn list_monitors() -> AppResult<Vec<MonitorInfo>> {
    let mut monitors: Vec<MonitorInfo> = Vec::new();
    let lparam = LPARAM(&mut monitors as *mut Vec<MonitorInfo> as isize);

    let ok = unsafe { EnumDisplayMonitors(None, None, Some(enum_monitor_proc), lparam) };
    if ok.as_bool() {
        Ok(monitors)
    } else {
        Err(AppError::Windows("EnumDisplayMonitors failed".to_string()))
    }
}

pub fn monitor_for_rect(rect: RECT) -> Option<MonitorInfo> {
    let handle = unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST) };
    if handle.is_invalid() {
        return None;
    }

    list_monitors().ok().and_then(|monitors| {
        monitors
            .into_iter()
            .find(|monitor| monitor_matches_handle(monitor, handle))
    })
}

fn monitor_matches_handle(monitor: &MonitorInfo, handle: HMONITOR) -> bool {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    let ok = unsafe { GetMonitorInfoW(handle, &mut info as *mut MONITORINFOEXW as *mut _) };
    if !ok.as_bool() {
        return false;
    }
    monitor.device_name == wide_to_string(&info.szDevice)
}

unsafe extern "system" fn enum_monitor_proc(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
    if let Some(info) = monitor_info(monitor) {
        monitors.push(info);
    }
    BOOL(1)
}

fn monitor_info(monitor: HMONITOR) -> Option<MonitorInfo> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    let ok = unsafe { GetMonitorInfoW(monitor, &mut info as *mut MONITORINFOEXW as *mut _) };
    if !ok.as_bool() {
        return None;
    }

    let bounds = info.monitorInfo.rcMonitor;
    let work = info.monitorInfo.rcWork;
    let device_name = wide_to_string(&info.szDevice);
    let is_primary = (info.monitorInfo.dwFlags & 1) == 1;
    let scale_factor = scale_factor_for_monitor(monitor);

    Some(MonitorInfo {
        id: stable_monitor_id(&device_name, &bounds),
        name: if device_name.is_empty() {
            "Display".to_string()
        } else {
            device_name.clone()
        },
        device_name,
        x: bounds.left,
        y: bounds.top,
        width: bounds.right - bounds.left,
        height: bounds.bottom - bounds.top,
        work_x: work.left,
        work_y: work.top,
        work_width: work.right - work.left,
        work_height: work.bottom - work.top,
        scale_factor,
        is_primary,
    })
}

fn stable_monitor_id(device_name: &str, bounds: &RECT) -> String {
    if device_name.trim().is_empty() {
        format!(
            "bounds:{}:{}:{}:{}",
            bounds.left,
            bounds.top,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top
        )
    } else {
        device_name.to_ascii_lowercase()
    }
}

fn scale_factor_for_monitor(monitor: HMONITOR) -> f64 {
    let mut dpi_x = 96u32;
    let mut dpi_y = 96u32;
    let result = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    if result.is_ok() && dpi_y > 0 {
        dpi_x as f64 / 96.0
    } else {
        1.0
    }
}

fn wide_to_string(buffer: &[u16]) -> String {
    let len = buffer
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}
