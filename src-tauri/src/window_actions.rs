use std::{
    ffi::c_void,
    thread,
    time::{Duration, Instant},
};

use windows::{
    core::BOOL,
    Win32::{
        Foundation::{GetLastError, HWND, LPARAM, RECT, WPARAM},
        Graphics::{
            Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
            Gdi::{
                GetDC, GetPixel, InvalidateRect, RedrawWindow, ReleaseDC, CLR_INVALID,
                RDW_ALLCHILDREN, RDW_FRAME, RDW_INTERNALPAINT, RDW_INVALIDATE, RDW_UPDATENOW,
            },
        },
        UI::WindowsAndMessaging::{
            BringWindowToTop, EnumWindows, GetClassNameW, GetClientRect, GetForegroundWindow,
            GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow,
            IsWindowVisible, IsZoomed, PostMessageW, SetForegroundWindow, SetWindowPos,
            ShowWindowAsync, HWND_TOP, SC_MAXIMIZE, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW,
            SW_RESTORE, SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, WM_APP, WM_SYSCOMMAND,
        },
    },
};

use crate::{
    errors::{AppError, AppResult},
    models::{LayoutRect, MonitorInfo, WindowStatePreference},
    performance,
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
        show_window_for_restore(hwnd);
    } else if should_unmaximize {
        unsafe {
            let _ = ShowWindowAsync(hwnd, SW_SHOWNOACTIVATE);
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
    let mut flags = SWP_SHOWWINDOW | SWP_NOZORDER | SWP_NOACTIVATE;
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
                let _ = PostMessageW(
                    Some(hwnd),
                    WM_SYSCOMMAND,
                    WPARAM(SC_MAXIMIZE as usize),
                    LPARAM(0),
                );
            }
            WindowStatePreference::Minimized => {
                let _ = ShowWindowAsync(hwnd, SW_SHOWMINNOACTIVE);
            }
        };
    }

    if !matches!(state, WindowStatePreference::Minimized) {
        refresh_window_surface(hwnd);
    }
    if activate_after_show {
        activate_window(hwnd);
    }

    Ok(())
}

pub fn show_window_for_restore(hwnd: HWND) {
    unsafe {
        let command = if IsIconic(hwnd).as_bool() {
            SW_RESTORE
        } else {
            SW_SHOWNOACTIVATE
        };
        let _ = ShowWindowAsync(hwnd, command);
    }
    thread::sleep(Duration::from_millis(160));
    refresh_window_surface(hwnd);
}

pub fn refresh_window_surface(hwnd: HWND) {
    queue_full_repaint(hwnd, true);
}

pub fn foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn wake_renderer_after_restore(hwnd: HWND, keep_active: bool, previous: HWND) -> bool {
    if performance::foreground_is_latency_sensitive() {
        return false;
    }

    show_window_for_restore(hwnd);
    if performance::foreground_is_latency_sensitive() {
        return false;
    }
    activate_window(hwnd);
    let mut surface_ready = wait_for_client_surface(hwnd, Duration::from_secs(2));
    if !surface_ready && !performance::foreground_is_latency_sensitive() {
        refresh_window_surface(hwnd);
        activate_window(hwnd);
        surface_ready = wait_for_client_surface(hwnd, Duration::from_secs(2));
    }

    let current = unsafe { GetForegroundWindow() };
    if !keep_active
        && previous != hwnd
        && current == hwnd
        && !previous.0.is_null()
        && unsafe { IsWindow(Some(previous)).as_bool() }
    {
        activate_window(previous);
    }
    surface_ready
}

fn wait_for_client_surface(hwnd: HWND, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if window_is_visible_and_restored(hwnd) {
            match client_surface_is_blank(hwnd) {
                Some(false) => return true,
                None => {
                    thread::sleep(Duration::from_millis(250));
                    return true;
                }
                Some(true) => {}
            }
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn window_is_visible_and_restored(hwnd: HWND) -> bool {
    unsafe {
        IsWindow(Some(hwnd)).as_bool()
            && IsWindowVisible(hwnd).as_bool()
            && !IsIconic(hwnd).as_bool()
    }
}

fn client_surface_is_blank(hwnd: HWND) -> Option<bool> {
    let mut rect = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut rect) }.is_err() {
        return None;
    }
    let width = rect.right.saturating_sub(rect.left);
    let height = rect.bottom.saturating_sub(rect.top);
    if width < 80 || height < 80 {
        return None;
    }

    let dc = unsafe { GetDC(Some(hwnd)) };
    if dc.0.is_null() {
        return None;
    }
    let points = [
        (width / 4, 8),
        (width / 2, 8),
        (width * 3 / 4, 8),
        (width / 6, height - 12),
        (width / 2, height - 12),
        (width * 5 / 6, height - 12),
        (12, height / 2),
        (width - 12, height / 2),
    ];
    let samples = points
        .into_iter()
        .map(|(x, y)| unsafe { GetPixel(dc, x, y) })
        .collect::<Vec<_>>();
    unsafe {
        ReleaseDC(Some(hwnd), dc);
    }
    surface_samples_are_blank(&samples)
}

fn surface_samples_are_blank(samples: &[windows::Win32::Foundation::COLORREF]) -> Option<bool> {
    let valid = samples
        .iter()
        .filter(|color| color.0 != CLR_INVALID)
        .copied()
        .collect::<Vec<_>>();
    if valid.len() < 4 {
        return None;
    }
    let white = valid
        .iter()
        .filter(|color| {
            let value = color.0;
            value & 0xff >= 245 && (value >> 8) & 0xff >= 245 && (value >> 16) & 0xff >= 245
        })
        .count();
    Some(white * 4 >= valid.len() * 3)
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

fn activate_window(hwnd: HWND) {
    if performance::foreground_is_latency_sensitive() {
        return;
    }
    unsafe {
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
    queue_full_repaint(hwnd, false);
}

fn queue_full_repaint(hwnd: HWND, update_now: bool) {
    let mut flags = RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_FRAME | RDW_INTERNALPAINT;
    if update_now {
        flags |= RDW_UPDATENOW;
    }
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
        let _ = RedrawWindow(Some(hwnd), None, None, flags);
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

    #[test]
    fn recognizes_uniform_white_client_samples() {
        let samples = vec![windows::Win32::Foundation::COLORREF(0x00ff_ffff); 8];

        assert_eq!(surface_samples_are_blank(&samples), Some(true));
    }

    #[test]
    fn recognizes_a_painted_client_surface() {
        let samples = vec![
            windows::Win32::Foundation::COLORREF(0x00ff_ffff),
            windows::Win32::Foundation::COLORREF(0x0018_1818),
            windows::Win32::Foundation::COLORREF(0x0024_2424),
            windows::Win32::Foundation::COLORREF(0x0030_3030),
            windows::Win32::Foundation::COLORREF(0x00ff_ffff),
            windows::Win32::Foundation::COLORREF(0x0010_1010),
            windows::Win32::Foundation::COLORREF(0x0020_2020),
            windows::Win32::Foundation::COLORREF(0x0030_3030),
        ];

        assert_eq!(surface_samples_are_blank(&samples), Some(false));
    }
}
