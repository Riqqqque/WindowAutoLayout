use std::{collections::HashMap, ffi::c_void};

use windows::{
    core::BOOL,
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
        UI::WindowsAndMessaging::{
            EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextLengthW,
            GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsZoomed,
        },
    },
};

use crate::{
    errors::{AppError, AppResult},
    models::WindowInfo,
    monitors, processes,
};

pub fn list_windows_with_hidden(include_hidden: bool) -> AppResult<Vec<WindowInfo>> {
    list_windows_with_options(include_hidden, true)
}

fn list_windows_with_options(
    include_hidden: bool,
    include_executable_path: bool,
) -> AppResult<Vec<WindowInfo>> {
    let mut windows: Vec<WindowInfo> = Vec::new();
    let process_names = processes::list_processes()
        .unwrap_or_default()
        .into_iter()
        .map(|process| (process.pid, process.name))
        .collect::<HashMap<_, _>>();
    let mut context = EnumWindowsContext {
        windows: &mut windows,
        process_names: &process_names,
        include_hidden,
        include_executable_path,
    };
    let lparam = LPARAM(&mut context as *mut EnumWindowsContext as isize);
    let ok = unsafe { EnumWindows(Some(enum_window_proc), lparam) };
    if ok.is_ok() {
        windows.sort_by(|left, right| {
            left.process_name
                .cmp(&right.process_name)
                .then(left.title.cmp(&right.title))
        });
        Ok(windows)
    } else {
        Err(AppError::Windows("EnumWindows failed".to_string()))
    }
}

pub fn foreground_window() -> Option<WindowInfo> {
    let hwnd = unsafe { GetForegroundWindow() };
    window_info_from_handle_lightweight(hwnd)
}

struct EnumWindowsContext<'a> {
    windows: &'a mut Vec<WindowInfo>,
    process_names: &'a HashMap<u32, String>,
    include_hidden: bool,
    include_executable_path: bool,
}

pub fn window_info_from_handle(hwnd: HWND) -> Option<WindowInfo> {
    window_info_from_handle_with_cache(hwnd, None, true)
}

pub fn window_info_from_handle_lightweight(hwnd: HWND) -> Option<WindowInfo> {
    window_info_from_handle_with_cache(hwnd, None, false)
}

fn window_info_from_handle_with_cache(
    hwnd: HWND,
    process_names: Option<&HashMap<u32, String>>,
    include_executable_path: bool,
) -> Option<WindowInfo> {
    if hwnd.0.is_null() {
        return None;
    }

    let visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    let minimized = unsafe { IsIconic(hwnd).as_bool() };
    let maximized = unsafe { IsZoomed(hwnd).as_bool() };
    let rect = window_rect(hwnd)?;
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return None;
    }

    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }

    let title = window_text(hwnd);
    let class_name = class_name(hwnd);
    let executable_path = if include_executable_path {
        processes::query_process_path(pid)
    } else {
        None
    };
    let process_name = process_names
        .and_then(|names| names.get(&pid).cloned())
        .or_else(|| {
            executable_path
                .as_deref()
                .and_then(processes::file_name_from_path)
        })
        .unwrap_or_else(|| processes::query_process_name(pid));
    let monitor_id = monitors::monitor_for_rect(rect).map(|monitor| monitor.id);

    Some(WindowInfo {
        handle: handle_to_string(hwnd),
        title,
        class_name,
        process_id: pid,
        process_name,
        executable_path,
        monitor_id,
        x: rect.left,
        y: rect.top,
        width,
        height,
        is_visible: visible,
        is_minimized: minimized,
        is_maximized: maximized,
    })
}

pub fn parse_handle(handle: &str) -> AppResult<HWND> {
    let trimmed = handle.trim().trim_start_matches("0x");
    let value = usize::from_str_radix(trimmed, 16)
        .map_err(|_| AppError::InvalidWindowHandle(handle.to_string()))?;
    if value == 0 {
        return Err(AppError::InvalidWindowHandle(handle.to_string()));
    }
    Ok(HWND(value as *mut c_void))
}

pub fn handle_to_string(hwnd: HWND) -> String {
    format!("0x{:X}", hwnd.0 as usize)
}

unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let context = &mut *(lparam.0 as *mut EnumWindowsContext);

    if let Some(info) = window_info_from_handle_with_cache(
        hwnd,
        Some(context.process_names),
        context.include_executable_path,
    ) {
        if (info.is_visible || context.include_hidden) && !is_shell_or_helper_window(&info) {
            context.windows.push(info);
        }
    }

    BOOL(1)
}

fn is_shell_or_helper_window(window: &WindowInfo) -> bool {
    let class = window.class_name.to_ascii_lowercase();
    let title = window.title.to_ascii_lowercase();
    class == "tooltips_class32"
        || class == "progman"
        || class == "workerw"
        || class.contains("ime")
        || title == "program manager"
}

fn window_text(hwnd: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 {
        return String::new();
    }

    let mut buffer = vec![0u16; length as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if copied <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buffer[..copied as usize])
    }
}

fn class_name(hwnd: HWND) -> String {
    let mut buffer = vec![0u16; 512];
    let copied = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if copied <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buffer[..copied as usize])
    }
}

fn window_rect(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    let extended = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut RECT as *mut c_void,
            std::mem::size_of::<RECT>() as u32,
        )
    };

    if extended.is_ok() {
        return Some(rect);
    }

    let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
    if ok.is_ok() {
        Some(rect)
    } else {
        None
    }
}
