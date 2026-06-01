use std::{thread, time::Duration};

use windows::{
    core::BOOL,
    Win32::{
        Foundation::{GetLastError, HWND, LPARAM, RECT, WPARAM},
        Graphics::Gdi::{
            RedrawWindow, UpdateWindow, RDW_ALLCHILDREN, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW,
        },
        UI::WindowsAndMessaging::{
            BringWindowToTop, EnumWindows, GetClassNameW, GetWindowTextW, GetWindowThreadProcessId,
            IsIconic, PostMessageW, SetForegroundWindow, SetWindowPos, ShowWindow, HWND_TOP,
            SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE,
            SW_SHOW, WM_APP,
        },
    },
};

use crate::{
    errors::{AppError, AppResult},
    models::{LayoutRect, MonitorInfo, WindowStatePreference},
};

#[allow(clippy::too_many_arguments)]
pub fn apply_layout(
    hwnd: HWND,
    monitor: &MonitorInfo,
    layout: &LayoutRect,
    state: &WindowStatePreference,
    force_resize: bool,
    restore_if_minimized: bool,
    pull_hidden_window: bool,
    activate_after_show: bool,
) -> AppResult<()> {
    let should_show =
        pull_hidden_window || (restore_if_minimized && unsafe { IsIconic(hwnd).as_bool() });
    if should_show {
        show_window_for_restore(hwnd);
    }

    let rect = absolute_rect(monitor, layout);
    let width = if force_resize {
        rect.right - rect.left
    } else {
        0
    };
    let height = if force_resize {
        rect.bottom - rect.top
    } else {
        0
    };
    let mut flags = SWP_SHOWWINDOW;
    if !activate_after_show {
        flags |= SWP_NOZORDER | SWP_NOACTIVATE;
    }
    if !force_resize {
        flags |= windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE;
    }

    let ok = unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            rect.left,
            rect.top,
            width,
            height,
            flags,
        )
    };
    if ok.is_err() {
        let error = unsafe { GetLastError() };
        return Err(AppError::Windows(format!(
            "SetWindowPos failed with Windows error {}",
            error.0
        )));
    }

    unsafe {
        match state {
            WindowStatePreference::Normal => {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            WindowStatePreference::Maximized => {
                let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            }
            WindowStatePreference::Minimized => {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
        };
    }

    if activate_after_show {
        wake_painted_window(hwnd);
    }

    Ok(())
}

pub fn show_window_for_restore(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = ShowWindow(hwnd, SW_SHOW);
    }
    thread::sleep(Duration::from_millis(120));
    wake_painted_window(hwnd);
}

pub fn activate_qt_tray_icon_for_process(process_id: u32) -> bool {
    let mut context = TrayWindowSearch {
        process_id,
        tray_windows: Vec::new(),
    };
    let lparam = LPARAM(&mut context as *mut TrayWindowSearch as isize);
    let _ = unsafe { EnumWindows(Some(enum_tray_window_proc), lparam) };

    let mut sent = false;
    for hwnd in context.tray_windows {
        sent |= unsafe {
            PostMessageW(
                Some(hwnd),
                WM_APP + 101,
                WPARAM(0),
                LPARAM(nin_select_message() as isize),
            )
            .is_ok()
        };
    }
    if sent {
        thread::sleep(Duration::from_millis(350));
    }
    sent
}

fn wake_painted_window(hwnd: HWND) {
    unsafe {
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        let _ = RedrawWindow(
            Some(hwnd),
            None,
            None,
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN | RDW_FRAME,
        );
        let _ = UpdateWindow(hwnd);
    }
    thread::sleep(Duration::from_millis(120));
    unsafe {
        let _ = RedrawWindow(
            Some(hwnd),
            None,
            None,
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN | RDW_FRAME,
        );
        let _ = UpdateWindow(hwnd);
    }
}

struct TrayWindowSearch {
    process_id: u32,
    tray_windows: Vec<HWND>,
}

unsafe extern "system" fn enum_tray_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let context = &mut *(lparam.0 as *mut TrayWindowSearch);
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if process_id == context.process_id {
        let class_name = window_class_name(hwnd).to_ascii_lowercase();
        let title = window_title(hwnd).to_ascii_lowercase();
        if class_name.contains("trayiconmessagewindowclass")
            || title.contains("qtrayiconmessagewindow")
        {
            context.tray_windows.push(hwnd);
        }
    }
    BOOL(1)
}

fn window_class_name(hwnd: HWND) -> String {
    let mut buffer = vec![0u16; 512];
    let copied = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if copied <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buffer[..copied as usize])
    }
}

fn window_title(hwnd: HWND) -> String {
    let mut buffer = vec![0u16; 512];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if copied <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buffer[..copied as usize])
    }
}

fn nin_select_message() -> u32 {
    0x0400
}

pub fn absolute_rect(monitor: &MonitorInfo, layout: &LayoutRect) -> RECT {
    RECT {
        left: monitor.x + layout.x,
        top: monitor.y + layout.y,
        right: monitor.x + layout.x + layout.width,
        bottom: monitor.y + layout.y + layout.height,
    }
}

pub fn relative_rect(monitor: &MonitorInfo, rect: RECT) -> LayoutRect {
    LayoutRect {
        x: rect.left - monitor.x,
        y: rect.top - monitor.y,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_negative_monitor_coordinates() {
        let monitor = MonitorInfo {
            id: "left".into(),
            name: "Left".into(),
            device_name: "DISPLAY2".into(),
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
            work_x: -1920,
            work_y: 0,
            work_width: 1920,
            work_height: 1040,
            scale_factor: 1.0,
            is_primary: false,
        };
        let layout = LayoutRect {
            x: 100,
            y: 50,
            width: 800,
            height: 600,
        };

        let rect = absolute_rect(&monitor, &layout);
        assert_eq!(rect.left, -1820);
        assert_eq!(relative_rect(&monitor, rect), layout);
    }
}
