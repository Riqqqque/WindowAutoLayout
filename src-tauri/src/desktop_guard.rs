use std::{
    sync::{Arc, Condvar, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager};
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Accessibility::{SetWinEventHook, HWINEVENTHOOK},
        WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_MINIMIZESTART, MSG, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
        },
    },
};

use crate::{
    layout_lock, logging,
    models::{LogSeverity, RestoreStatus, WindowInfo},
    performance, profiles,
    state::AppState,
    windows_enum,
};

const DESKTOP_RESTORE_DELAY: Duration = Duration::from_millis(500);
const GAME_EXIT_RESTORE_DELAY: Duration = Duration::from_millis(750);
const RESTORE_DEBOUNCE: Duration = Duration::from_secs(3);
const RECENT_MINIMIZE_WINDOW: Duration = Duration::from_secs(2);

static GUARD: OnceLock<Arc<DesktopGuard>> = OnceLock::new();

pub fn start(app: AppHandle) {
    let guard = Arc::new(DesktopGuard {
        app,
        timing: Mutex::new(GuardTiming {
            latency_sensitive_foreground: performance::foreground_is_latency_sensitive(),
            ..GuardTiming::default()
        }),
        schedule: Mutex::new(None),
        schedule_changed: Condvar::new(),
    });
    if GUARD.set(guard.clone()).is_err() {
        return;
    }

    let restore_guard = guard.clone();
    if let Err(error) = thread::Builder::new()
        .name("windowautolayout-restore-worker".into())
        .spawn(move || restore_guard.run_restore_worker())
    {
        let state = guard.app.state::<AppState>();
        let _ = logging::append(
            &state.config_dir,
            LogSeverity::Error,
            None,
            None,
            format!("Could not start restore worker: {error}"),
        );
    }
    let config_dir = guard.app.state::<AppState>().config_dir.clone();
    if let Err(error) = thread::Builder::new()
        .name("windowautolayout-desktop-events".into())
        .spawn(move || run_event_thread(config_dir))
    {
        let state = guard.app.state::<AppState>();
        let _ = logging::append(
            &state.config_dir,
            LogSeverity::Error,
            None,
            None,
            format!("Could not start desktop event worker: {error}"),
        );
    }
}

struct DesktopGuard {
    app: AppHandle,
    timing: Mutex<GuardTiming>,
    schedule: Mutex<Option<Instant>>,
    schedule_changed: Condvar,
}

#[derive(Default)]
struct GuardTiming {
    latency_sensitive_foreground: bool,
    last_minimize_start: Option<Instant>,
    last_restore: Option<Instant>,
}

impl DesktopGuard {
    fn handle_event(&self, event: u32, hwnd: HWND) {
        if hwnd.0.is_null() {
            return;
        }

        match event {
            EVENT_SYSTEM_FOREGROUND => self.handle_foreground(hwnd),
            EVENT_SYSTEM_MINIMIZESTART => self.handle_minimize_start(),
            _ => {}
        }
    }

    fn handle_foreground(&self, hwnd: HWND) {
        let Some(window) = windows_enum::window_info_from_handle_lightweight(hwnd) else {
            return;
        };
        let is_latency_sensitive = performance::is_latency_sensitive_window(&window);
        let (left_latency_sensitive, desktop_reveal) = {
            let Ok(mut timing) = self.timing.lock() else {
                return;
            };
            let was_latency_sensitive = timing.latency_sensitive_foreground;
            timing.latency_sensitive_foreground = is_latency_sensitive;
            let recent_minimize = timing
                .last_minimize_start
                .map(|at| at.elapsed() <= RECENT_MINIMIZE_WINDOW)
                .unwrap_or(false);

            (
                was_latency_sensitive && !is_latency_sensitive && !is_own_window(&window),
                is_desktop_shell_window(&window)
                    || (recent_minimize && is_taskbar_shell_window(&window)),
            )
        };

        if desktop_reveal {
            self.request_restore(DESKTOP_RESTORE_DELAY);
        } else if left_latency_sensitive {
            self.request_restore(GAME_EXIT_RESTORE_DELAY);
        }
    }

    fn handle_minimize_start(&self) {
        let Ok(mut timing) = self.timing.lock() else {
            return;
        };
        timing.last_minimize_start = Some(Instant::now());
    }

    fn request_restore(&self, delay: Duration) {
        if !layout_lock::enabled(&self.app).unwrap_or(false) {
            return;
        }

        {
            let Ok(timing) = self.timing.lock() else {
                return;
            };
            if timing
                .last_restore
                .map(|at| at.elapsed() < RESTORE_DEBOUNCE)
                .unwrap_or(false)
            {
                return;
            }
        }

        let Ok(mut schedule) = self.schedule.lock() else {
            return;
        };
        if schedule.is_none() {
            *schedule = Some(Instant::now() + delay);
            self.schedule_changed.notify_one();
        }
    }

    fn run_restore_worker(&self) {
        performance::lower_current_thread_priority();

        loop {
            {
                let Ok(mut schedule) = self.schedule.lock() else {
                    return;
                };
                loop {
                    let Some(deadline) = *schedule else {
                        let Ok(next) = self.schedule_changed.wait(schedule) else {
                            return;
                        };
                        schedule = next;
                        continue;
                    };
                    let now = Instant::now();
                    if deadline <= now {
                        *schedule = None;
                        break;
                    }

                    let Ok((next, wait)) = self
                        .schedule_changed
                        .wait_timeout(schedule, deadline.saturating_duration_since(now))
                    else {
                        return;
                    };
                    schedule = next;
                    if wait.timed_out() && *schedule == Some(deadline) {
                        *schedule = None;
                        break;
                    }
                }
            }

            if !layout_lock::enabled(&self.app).unwrap_or(false)
                || performance::foreground_is_latency_sensitive()
            {
                continue;
            }

            let state = self.app.state::<AppState>();
            let config_dir = state.config_dir.clone();
            let Ok(config) = state.config.lock().map(|config| config.clone()) else {
                continue;
            };
            let profile_id = config
                .enforcement
                .profile_id
                .clone()
                .or_else(|| config.startup.default_profile_id.clone());
            if profiles::restore_profile_silent(&config_dir, &config, profile_id, Some(false))
                .is_ok_and(|result| !matches!(result.status, RestoreStatus::Paused))
            {
                if let Ok(mut timing) = self.timing.lock() {
                    timing.last_restore = Some(Instant::now());
                }
            }
        }
    }
}

fn run_event_thread(config_dir: std::path::PathBuf) {
    performance::lower_current_thread_priority();
    let hook_foreground = install_hook(EVENT_SYSTEM_FOREGROUND);
    let hook_minimize = install_hook(EVENT_SYSTEM_MINIMIZESTART);
    if hook_foreground.0.is_null() && hook_minimize.0.is_null() {
        let _ = logging::append(
            &config_dir,
            LogSeverity::Error,
            None,
            None,
            "Could not register Windows desktop event hooks",
        );
        return;
    }
    if hook_foreground.0.is_null() || hook_minimize.0.is_null() {
        let _ = logging::append(
            &config_dir,
            LogSeverity::Warn,
            None,
            None,
            "One Windows desktop event hook could not be registered",
        );
    }

    let mut message = MSG::default();
    unsafe {
        loop {
            let result = GetMessageW(&mut message, None, 0, 0);
            if result.0 <= 0 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if !hook_foreground.0.is_null() {
            let _ = windows::Win32::UI::Accessibility::UnhookWinEvent(hook_foreground);
        }
        if !hook_minimize.0.is_null() {
            let _ = windows::Win32::UI::Accessibility::UnhookWinEvent(hook_minimize);
        }
    }
}

fn install_hook(event: u32) -> HWINEVENTHOOK {
    unsafe {
        SetWinEventHook(
            event,
            event,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    }
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _object_id: i32,
    _child_id: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if let Some(guard) = GUARD.get() {
        guard.handle_event(event, hwnd);
    }
}

fn is_own_window(window: &WindowInfo) -> bool {
    matches!(
        window.process_name.trim().to_ascii_lowercase().as_str(),
        "windowautolayout.exe" | "windowautolayout"
    )
}

fn is_desktop_shell_window(window: &WindowInfo) -> bool {
    let class = window.class_name.trim().to_ascii_lowercase();
    let title = window.title.trim().to_ascii_lowercase();
    let process = window.process_name.trim().to_ascii_lowercase();

    process == "explorer.exe"
        && (class == "progman" || class == "workerw" || title == "program manager")
}

fn is_taskbar_shell_window(window: &WindowInfo) -> bool {
    let class = window.class_name.trim().to_ascii_lowercase();
    let process = window.process_name.trim().to_ascii_lowercase();

    process == "explorer.exe" && class == "shell_traywnd"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_desktop_shell_windows() {
        let window = test_window("explorer.exe", "Program Manager", "Progman");

        assert!(is_desktop_shell_window(&window));
        assert!(!is_taskbar_shell_window(&window));
    }

    #[test]
    fn detects_taskbar_shell_window_separately() {
        let window = test_window("explorer.exe", "", "Shell_TrayWnd");

        assert!(!is_desktop_shell_window(&window));
        assert!(is_taskbar_shell_window(&window));
    }

    #[test]
    fn ignores_own_windows() {
        let window = test_window("WindowAutoLayout.exe", "WindowAutoLayout", "Tauri Window");

        assert!(is_own_window(&window));
    }

    fn test_window(process_name: &str, title: &str, class_name: &str) -> WindowInfo {
        WindowInfo {
            handle: "0x1".into(),
            title: title.into(),
            class_name: class_name.into(),
            process_id: 10,
            process_name: process_name.into(),
            executable_path: None,
            monitor_id: None,
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            is_visible: true,
            is_minimized: false,
            is_maximized: false,
        }
    }
}
