use windows::Win32::Foundation::RECT;
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, SetPriorityClass, SetThreadPriority,
    BELOW_NORMAL_PRIORITY_CLASS, THREAD_PRIORITY_BELOW_NORMAL,
};
use windows::Win32::UI::Input::{GetRegisteredRawInputDevices, RAWINPUTDEVICE};

use crate::{models::WindowInfo, monitors, windows_enum};

pub fn lower_process_priority() {
    unsafe {
        let _ = SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }
}

pub fn lower_current_thread_priority() {
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }
}

pub fn registered_raw_input_device_count() -> Option<u32> {
    let mut count = 0u32;
    let result = unsafe {
        GetRegisteredRawInputDevices(
            None,
            &mut count,
            std::mem::size_of::<RAWINPUTDEVICE>() as u32,
        )
    };
    if result == u32::MAX {
        None
    } else {
        Some(count)
    }
}

pub fn foreground_is_latency_sensitive() -> bool {
    windows_enum::foreground_window()
        .map(|window| is_latency_sensitive_window(&window))
        .unwrap_or(false)
}

pub fn is_latency_sensitive_window(window: &WindowInfo) -> bool {
    if is_latency_sensitive_game_process(&window.process_name) {
        return true;
    }

    let rect = RECT {
        left: window.x,
        top: window.y,
        right: window.x + window.width,
        bottom: window.y + window.height,
    };
    monitors::monitor_for_rect(rect)
        .map(|monitor| covers_monitor(window, &monitor))
        .unwrap_or(false)
}

pub fn is_latency_sensitive_game_process(process_name: &str) -> bool {
    matches!(
        process_name.trim().to_ascii_lowercase().as_str(),
        "valorant-win64-shipping.exe"
            | "valorant.exe"
            | "cs2.exe"
            | "csgo.exe"
            | "fortniteclient-win64-shipping.exe"
            | "r5apex.exe"
            | "overwatch.exe"
            | "league of legends.exe"
            | "rocketleague.exe"
            | "tslgame.exe"
            | "rainbowsix.exe"
            | "rainbowsix_vulkan.exe"
            | "escapefromtarkov.exe"
            | "minecraft.windows.exe"
            | "javaw.exe"
            | "osu!.exe"
            | "eldenring.exe"
            | "gta5.exe"
            | "fivem.exe"
            | "helldivers2.exe"
    )
}

fn covers_monitor(window: &WindowInfo, monitor: &crate::models::MonitorInfo) -> bool {
    if !window.is_visible || window.is_minimized {
        return false;
    }

    let tolerance = 4;
    (window.x - monitor.x).abs() <= tolerance
        && (window.y - monitor.y).abs() <= tolerance
        && window.width >= monitor.width - tolerance
        && window.height >= monitor.height - tolerance
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MonitorInfo;

    #[test]
    fn recognizes_named_games_without_requiring_fullscreen() {
        assert!(is_latency_sensitive_game_process(
            "VALORANT-Win64-Shipping.exe"
        ));
        assert!(is_latency_sensitive_game_process("javaw.exe"));
        assert!(!is_latency_sensitive_game_process("Discord.exe"));
    }

    #[test]
    fn distinguishes_fullscreen_from_work_area_windows() {
        let monitor = MonitorInfo {
            id: "display".into(),
            name: "Display".into(),
            device_name: "DISPLAY1".into(),
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
            work_x: 0,
            work_y: 0,
            work_width: 2560,
            work_height: 1392,
            scale_factor: 1.25,
            is_primary: true,
        };
        let fullscreen = WindowInfo {
            handle: "0x1".into(),
            title: "Game".into(),
            class_name: "GameWindow".into(),
            process_id: 10,
            process_name: "unknown-game.exe".into(),
            executable_path: None,
            monitor_id: Some("display".into()),
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
            is_visible: true,
            is_minimized: false,
            is_maximized: false,
        };
        let maximized = WindowInfo {
            height: 1392,
            ..fullscreen.clone()
        };

        assert!(covers_monitor(&fullscreen, &monitor));
        assert!(!covers_monitor(&maximized, &monitor));
    }
}
