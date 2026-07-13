use std::{ffi::c_void, thread, time::Duration};

use windows::{
    core::BOOL,
    Win32::{
        Foundation::{GetLastError, HWND, LPARAM, RECT, WPARAM},
        Graphics::{
            Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
            Gdi::{
                RedrawWindow, UpdateWindow, RDW_ALLCHILDREN, RDW_FRAME, RDW_INVALIDATE,
                RDW_UPDATENOW,
            },
        },
        UI::WindowsAndMessaging::{
            BringWindowToTop, EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextW,
            GetWindowThreadProcessId, IsIconic, IsZoomed, PostMessageW, SetForegroundWindow,
            SetWindowPos, ShowWindow, HWND_TOP, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW,
            SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, WM_APP,
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
    let should_unmaximize =
        !matches!(state, WindowStatePreference::Minimized) && unsafe { IsZoomed(hwnd).as_bool() };
    if should_show {
        show_window_for_restore(hwnd, activate_after_show);
    } else if should_unmaximize {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        thread::sleep(Duration::from_millis(120));
    }

    let frame_rect = absolute_rect(monitor, layout);
    let window_rect = rect_for_set_window_pos(hwnd, frame_rect);
    let width = if force_resize {
        window_rect.right - window_rect.left
    } else {
        0
    };
    let height = if force_resize {
        window_rect.bottom - window_rect.top
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
            window_rect.left,
            window_rect.top,
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
            WindowStatePreference::Normal => {}
            WindowStatePreference::Maximized => {
                let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            }
            WindowStatePreference::Minimized => {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
        };
    }

    if activate_after_show {
        wake_painted_window(hwnd, true);
    }

    Ok(())
}

pub fn show_window_for_restore(hwnd: HWND, activate: bool) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = ShowWindow(hwnd, SW_SHOW);
    }
    thread::sleep(Duration::from_millis(120));
    wake_painted_window(hwnd, activate);
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

fn wake_painted_window(hwnd: HWND, activate: bool) {
    unsafe {
        if activate {
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
        }
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

fn rect_for_set_window_pos(hwnd: HWND, target_frame_rect: RECT) -> RECT {
    window_frame_margins(hwnd)
        .map(|margins| adjust_rect_for_frame_margins(target_frame_rect, margins))
        .unwrap_or(target_frame_rect)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameMargins {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

fn window_frame_margins(hwnd: HWND) -> Option<FrameMargins> {
    let mut outer = RECT::default();
    let outer_ok = unsafe { GetWindowRect(hwnd, &mut outer) };
    if outer_ok.is_err() {
        return None;
    }

    let mut frame = RECT::default();
    let frame_ok = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame as *mut RECT as *mut c_void,
            std::mem::size_of::<RECT>() as u32,
        )
    };
    if frame_ok.is_err() {
        return None;
    }

    if outer.right <= outer.left
        || outer.bottom <= outer.top
        || frame.right <= frame.left
        || frame.bottom <= frame.top
    {
        return None;
    }

    let margins = FrameMargins {
        left: frame.left - outer.left,
        top: frame.top - outer.top,
        right: outer.right - frame.right,
        bottom: outer.bottom - frame.bottom,
    };
    let values = [margins.left, margins.top, margins.right, margins.bottom];
    if values
        .iter()
        .any(|value| *value < 0 || *value > MAX_FRAME_MARGIN_PX)
    {
        return None;
    }

    Some(margins)
}

const MAX_FRAME_MARGIN_PX: i32 = 128;

fn adjust_rect_for_frame_margins(target_frame_rect: RECT, margins: FrameMargins) -> RECT {
    RECT {
        left: target_frame_rect.left - margins.left,
        top: target_frame_rect.top - margins.top,
        right: target_frame_rect.right + margins.right,
        bottom: target_frame_rect.bottom + margins.bottom,
    }
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

    #[test]
    fn adjusts_saved_frame_rect_to_outer_window_rect() {
        let target = RECT {
            left: -3061,
            top: -244,
            right: -1167,
            bottom: 815,
        };
        let adjusted = adjust_rect_for_frame_margins(
            target,
            FrameMargins {
                left: 9,
                top: 0,
                right: 9,
                bottom: 9,
            },
        );

        assert_eq!(adjusted.left, -3070);
        assert_eq!(adjusted.top, -244);
        assert_eq!(adjusted.right, -1158);
        assert_eq!(adjusted.bottom, 824);
    }
}
