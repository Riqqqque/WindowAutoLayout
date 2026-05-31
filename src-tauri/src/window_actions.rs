use std::{thread, time::Duration};

use windows::Win32::{
    Foundation::{GetLastError, HWND, RECT},
    Graphics::Gdi::{
        RedrawWindow, UpdateWindow, RDW_ALLCHILDREN, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW,
    },
    UI::WindowsAndMessaging::{
        BringWindowToTop, IsIconic, SetForegroundWindow, SetWindowPos, ShowWindow, HWND_TOP,
        SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE,
        SW_SHOW,
    },
};

use crate::{
    errors::{AppError, AppResult},
    models::{LayoutRect, MonitorInfo, WindowStatePreference},
};

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
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        thread::sleep(Duration::from_millis(120));
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
