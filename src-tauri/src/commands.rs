use std::{collections::HashMap, path::PathBuf, process::Command};

use tauri::{AppHandle, State};

use crate::{
    config,
    errors::{AppError, AppResult},
    layout_lock, logging,
    models::{
        preset_apps, AppConfig, CaptureLayoutResult, CapturedDisplay, CapturedWindowSummary,
        LogEntry, LogSeverity, MatchRule, MonitorInfo, RestoreResult, RuntimeStatus,
        TitleMatchMode, WindowAutoLayoutConfig, WindowInfo, WindowStatePreference,
    },
    monitors, profiles, startup,
    state::AppState,
    tray_ui, window_actions, windows_enum,
};

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppResult<WindowAutoLayoutConfig> {
    Ok(state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .clone())
}

#[tauri::command]
pub fn parse_config_json(raw: String) -> AppResult<WindowAutoLayoutConfig> {
    config::parse_json(&raw)
}

#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<'_, AppState>,
    mut next_config: WindowAutoLayoutConfig,
) -> AppResult<WindowAutoLayoutConfig> {
    config::normalize_config(&mut next_config);
    let previous_config = state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .clone();
    config::save(&state.config_dir, &next_config)?;
    if let Err(error) = startup::set_startup_enabled(next_config.startup.enabled) {
        let _ = config::save(&state.config_dir, &previous_config);
        return Err(error);
    }
    *state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))? =
        next_config.clone();
    layout_lock::sync_from_config(&app, &next_config)?;
    let _ = logging::append(
        &state.config_dir,
        LogSeverity::Info,
        None,
        None,
        "Settings saved",
    );
    Ok(next_config)
}

#[tauri::command]
pub fn validate_current_config(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    let config = state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .clone();
    Ok(config::validate_config(&config))
}

#[tauri::command]
pub fn get_app_presets() -> Vec<AppConfig> {
    preset_apps()
}

#[tauri::command]
pub fn list_monitors() -> AppResult<Vec<MonitorInfo>> {
    monitors::list_monitors()
}

#[tauri::command]
pub fn list_windows() -> AppResult<Vec<WindowInfo>> {
    windows_enum::list_windows_with_hidden(true)
}

#[tauri::command]
pub async fn restore_profile(
    state: State<'_, AppState>,
    profile_id: Option<String>,
    launch_missing: Option<bool>,
) -> AppResult<RestoreResult> {
    let config_dir = state.config_dir.clone();
    let config = state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .clone();

    tauri::async_runtime::spawn_blocking(move || {
        profiles::restore_profile(&config_dir, &config, profile_id, launch_missing)
    })
    .await
    .map_err(|error| AppError::Config(format!("Restore task failed: {error}")))?
}

#[tauri::command]
pub fn set_layout_lock(
    app: AppHandle,
    enabled: bool,
    profile_id: Option<String>,
) -> AppResult<bool> {
    layout_lock::set(&app, enabled, profile_id)
}

#[tauri::command]
pub fn layout_lock_enabled(app: AppHandle) -> AppResult<bool> {
    layout_lock::enabled(&app)
}

#[tauri::command]
pub fn runtime_status(app: AppHandle) -> AppResult<RuntimeStatus> {
    tray_ui::runtime_status(&app)
}

#[tauri::command]
pub fn save_window_layout(
    state: State<'_, AppState>,
    profile_id: String,
    app_id: String,
    window_handle: String,
) -> AppResult<WindowAutoLayoutConfig> {
    let hwnd = windows_enum::parse_handle(&window_handle)?;
    let window = windows_enum::window_info_from_handle(hwnd)
        .ok_or_else(|| AppError::InvalidWindowHandle(window_handle.clone()))?;
    let monitor_id = window.monitor_id.clone().ok_or(AppError::MonitorNotFound)?;
    let monitor = monitors::list_monitors()?
        .into_iter()
        .find(|monitor| monitors::monitor_id_matches(monitor, &monitor_id))
        .ok_or(AppError::MonitorNotFound)?;

    let mut next_config = state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .clone();

    let profile = next_config
        .profiles
        .iter_mut()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| AppError::ProfileNotFound(profile_id.clone()))?;
    if profile.target_monitor_id.is_none() {
        profile.target_monitor_id = Some(monitor.id.clone());
    }

    let app = profile
        .apps
        .iter_mut()
        .find(|app| app.id == app_id)
        .ok_or_else(|| AppError::AppNotFound(app_id.clone()))?;

    app.layout = window_actions::relative_rect(
        &monitor,
        windows::Win32::Foundation::RECT {
            left: window.x,
            top: window.y,
            right: window.x + window.width,
            bottom: window.y + window.height,
        },
    );
    app.captured_display = Some(CapturedDisplay::from_monitor(&monitor));
    app.target_monitor_id = Some(monitor.id);
    if app
        .process_name
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        app.process_name = Some(window.process_name.clone());
    }
    if app.executable_path.is_none() {
        app.executable_path = window.executable_path.clone();
    }

    config::save(&state.config_dir, &next_config)?;
    *state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))? =
        next_config.clone();
    let _ = logging::append(
        &state.config_dir,
        LogSeverity::Info,
        Some(&profile_id),
        Some(&app_id),
        "Captured window layout",
    );
    Ok(next_config)
}

#[tauri::command]
pub fn save_all_current_layouts(
    state: State<'_, AppState>,
    profile_id: String,
) -> AppResult<WindowAutoLayoutConfig> {
    let mut next_config = state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .clone();
    let monitors = monitors::list_monitors()?;
    let profile = next_config
        .profiles
        .iter_mut()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| AppError::ProfileNotFound(profile_id.clone()))?;

    for app in &mut profile.apps {
        let Some(window) = profiles::find_matching_windows(app, None)
            .into_iter()
            .next()
        else {
            continue;
        };
        let Some(monitor_id) = &window.monitor_id else {
            continue;
        };
        let Some(monitor) = monitors
            .iter()
            .find(|monitor| monitors::monitor_id_matches(monitor, monitor_id))
        else {
            continue;
        };
        app.layout = window_actions::relative_rect(
            monitor,
            windows::Win32::Foundation::RECT {
                left: window.x,
                top: window.y,
                right: window.x + window.width,
                bottom: window.y + window.height,
            },
        );
        app.captured_display = Some(CapturedDisplay::from_monitor(monitor));
        app.target_monitor_id = Some(monitor.id.clone());
        if profile.target_monitor_id.is_none() {
            profile.target_monitor_id = Some(monitor.id.clone());
        }
    }

    config::save(&state.config_dir, &next_config)?;
    *state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))? =
        next_config.clone();
    let _ = logging::append(
        &state.config_dir,
        LogSeverity::Info,
        Some(&profile_id),
        None,
        "Captured layouts for matching configured apps",
    );
    Ok(next_config)
}

#[tauri::command]
pub fn capture_current_layout(
    state: State<'_, AppState>,
    profile_id: String,
    monitor_id: String,
) -> AppResult<CaptureLayoutResult> {
    let monitors = monitors::list_monitors()?;
    let monitor = monitors
        .iter()
        .find(|monitor| monitors::monitor_id_matches(monitor, &monitor_id))
        .cloned()
        .ok_or(AppError::MonitorNotFound)?;

    let all_windows = windows_enum::list_windows_with_hidden(false)?;
    let skipped_count = all_windows
        .iter()
        .filter(|window| !capture_window_candidate(window, &monitor))
        .count();
    let mut captured_windows: Vec<WindowInfo> = all_windows
        .into_iter()
        .filter(|window| capture_window_candidate(window, &monitor))
        .collect();

    captured_windows.sort_by(|left, right| {
        left.y
            .cmp(&right.y)
            .then(left.x.cmp(&right.x))
            .then(left.process_name.cmp(&right.process_name))
            .then(left.title.cmp(&right.title))
    });

    if captured_windows.is_empty() {
        return Err(AppError::Config(format!(
            "No visible windows were found on {}",
            monitor.name
        )));
    }

    let captured_apps = captured_apps_for_windows(&monitor, &captured_windows);
    let summaries = captured_apps
        .iter()
        .zip(captured_windows.iter())
        .map(|(app, window)| CapturedWindowSummary {
            app_id: app.id.clone(),
            display_name: app.display_name.clone(),
            process_name: window.process_name.clone(),
            title: window.title.clone(),
        })
        .collect::<Vec<_>>();

    let mut next_config = state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .clone();

    let profile = next_config
        .profiles
        .iter_mut()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| AppError::ProfileNotFound(profile_id.clone()))?;

    profile.target_monitor_id = Some(monitor.id.clone());
    profile.apps = captured_apps;

    config::save(&state.config_dir, &next_config)?;
    *state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))? =
        next_config.clone();
    let _ = logging::append(
        &state.config_dir,
        LogSeverity::Info,
        Some(&profile_id),
        None,
        format!(
            "Captured {} visible window(s) on {}",
            summaries.len(),
            monitor.name
        ),
    );

    Ok(CaptureLayoutResult {
        config: next_config,
        profile_id,
        monitor,
        captured_count: summaries.len(),
        skipped_count,
        captured_windows: summaries,
    })
}

fn captured_apps_for_windows(monitor: &MonitorInfo, windows: &[WindowInfo]) -> Vec<AppConfig> {
    let process_counts = process_counts(windows);

    windows
        .iter()
        .enumerate()
        .map(|(index, window)| {
            let process_name = non_empty(window.process_name.as_str());
            let title = non_empty(window.title.as_str());
            let class_name = non_empty(window.class_name.as_str());
            let duplicate_process = process_name
                .as_ref()
                .and_then(|name| process_counts.get(&name.to_ascii_lowercase()))
                .copied()
                .unwrap_or(0)
                > 1;

            AppConfig {
                id: captured_app_id(window, index),
                display_name: display_name_for_window(window),
                executable_path: window.executable_path.clone(),
                arguments: Vec::new(),
                working_directory: None,
                process_name,
                title_rule: title_rule_for_window(window, duplicate_process),
                class_name,
                target_monitor_id: Some(monitor.id.clone()),
                captured_display: Some(CapturedDisplay::from_monitor(monitor)),
                layout: window_actions::relative_rect(
                    monitor,
                    windows::Win32::Foundation::RECT {
                        left: window.x,
                        top: window.y,
                        right: window.x + window.width,
                        bottom: window.y + window.height,
                    },
                ),
                window_state: if window.is_maximized {
                    WindowStatePreference::Maximized
                } else {
                    WindowStatePreference::Normal
                },
                launch_delay_seconds: 0,
                detection_timeout_seconds: 25,
                retry_interval_ms: 700,
                move_if_running: true,
                force_resize: true,
                apply_to_all_matching_windows: false,
                restore_if_minimized: true,
                pull_hidden_windows: true,
                wake_running_process: true,
                allow_empty_title: title.is_none(),
                notes: None,
            }
        })
        .collect()
}

fn capture_window_candidate(window: &WindowInfo, monitor: &MonitorInfo) -> bool {
    window.is_visible
        && !window.is_minimized
        && !is_self_window(window)
        && !window.process_name.trim().is_empty()
        && window_matches_capture_monitor(window, monitor)
}

fn window_matches_capture_monitor(window: &WindowInfo, monitor: &MonitorInfo) -> bool {
    window
        .monitor_id
        .as_ref()
        .map(|id| id == &monitor.id)
        .unwrap_or_else(|| window_center_is_on_monitor(window, monitor))
}

fn window_center_is_on_monitor(window: &WindowInfo, monitor: &MonitorInfo) -> bool {
    let center_x = window.x + window.width / 2;
    let center_y = window.y + window.height / 2;
    let monitor_right = monitor.x + monitor.width;
    let monitor_bottom = monitor.y + monitor.height;

    center_x >= monitor.x
        && center_x < monitor_right
        && center_y >= monitor.y
        && center_y < monitor_bottom
}

fn is_self_window(window: &WindowInfo) -> bool {
    let process = window.process_name.trim().to_ascii_lowercase();
    process == "windowautolayout.exe"
        || process == "windowautolayout"
        || window.title.trim().eq_ignore_ascii_case("WindowAutoLayout")
}

fn process_counts(windows: &[WindowInfo]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for window in windows {
        if let Some(process_name) = non_empty(window.process_name.as_str()) {
            *counts.entry(process_name.to_ascii_lowercase()).or_insert(0) += 1;
        }
    }
    counts
}

fn title_rule_for_window(window: &WindowInfo, duplicate_process: bool) -> Option<MatchRule> {
    let title = non_empty(window.title.as_str())?;
    if process_is(window, "obs64.exe") && title.to_ascii_lowercase().starts_with("obs") {
        return Some(MatchRule {
            mode: TitleMatchMode::StartsWith,
            value: "OBS".to_string(),
            case_sensitive: false,
        });
    }
    if duplicate_process {
        return Some(MatchRule {
            mode: TitleMatchMode::Exact,
            value: title,
            case_sensitive: false,
        });
    }
    None
}

fn display_name_for_window(window: &WindowInfo) -> String {
    if process_is(window, "obs64.exe") {
        return "OBS Studio".to_string();
    }
    non_empty(window.title.as_str())
        .or_else(|| non_empty(window.process_name.trim_end_matches(".exe")))
        .unwrap_or_else(|| "Captured window".to_string())
}

fn captured_app_id(window: &WindowInfo, index: usize) -> String {
    let base = non_empty(window.process_name.trim_end_matches(".exe"))
        .or_else(|| non_empty(window.title.as_str()))
        .unwrap_or_else(|| "window".to_string());
    format!("captured-{}-{}", slug(&base), index + 1)
}

fn process_is(window: &WindowInfo, expected: &str) -> bool {
    window.process_name.eq_ignore_ascii_case(expected)
        || window
            .process_name
            .trim_end_matches(".exe")
            .eq_ignore_ascii_case(expected.trim_end_matches(".exe"))
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "window".to_string()
    } else {
        slug
    }
}

#[tauri::command]
pub fn read_logs(state: State<'_, AppState>, max_lines: Option<usize>) -> AppResult<Vec<LogEntry>> {
    logging::read(&state.config_dir, max_lines.unwrap_or(500))
}

#[tauri::command]
pub fn clear_logs(state: State<'_, AppState>) -> AppResult<()> {
    logging::clear(&state.config_dir)
}

#[tauri::command]
pub fn startup_enabled() -> bool {
    startup::startup_enabled()
}

#[tauri::command]
pub fn set_startup_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> AppResult<WindowAutoLayoutConfig> {
    startup::set_startup_enabled(enabled)?;
    let mut next_config = state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .clone();
    next_config.startup.enabled = enabled;
    config::save(&state.config_dir, &next_config)?;
    *state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))? =
        next_config.clone();
    let _ = tray_ui::sync(&app);
    Ok(next_config)
}

#[tauri::command]
pub fn config_path(state: State<'_, AppState>) -> String {
    config::config_file_path(&state.config_dir)
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
pub fn log_path(state: State<'_, AppState>) -> String {
    logging::log_file_path(&state.config_dir)
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
pub fn open_log_file(state: State<'_, AppState>) -> AppResult<()> {
    let path = logging::log_file_path(&state.config_dir);
    if !path.exists() {
        logging::append(
            &state.config_dir,
            LogSeverity::Info,
            None,
            None,
            "Created log file",
        )?;
    }
    open_path(path)
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> AppResult<()> {
    crate::show_main_window(&app);
    Ok(())
}

fn open_path(path: PathBuf) -> AppResult<()> {
    Command::new("explorer")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(AppError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(id: &str, x: i32, y: i32, width: i32, height: i32) -> MonitorInfo {
        MonitorInfo {
            id: id.to_string(),
            name: id.to_string(),
            device_name: id.to_string(),
            x,
            y,
            width,
            height,
            work_x: x,
            work_y: y,
            work_width: width,
            work_height: height,
            scale_factor: 1.0,
            is_primary: false,
        }
    }

    fn window(
        process_name: &str,
        title: &str,
        monitor_id: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> WindowInfo {
        WindowInfo {
            handle: format!("0x{:X}", x.abs() + y.abs() + width + height + 1),
            title: title.to_string(),
            class_name: "AppWindow".to_string(),
            process_id: 42,
            process_name: process_name.to_string(),
            executable_path: Some(format!("C:\\Apps\\{process_name}")),
            monitor_id: Some(monitor_id.to_string()),
            x,
            y,
            width,
            height,
            is_visible: true,
            is_minimized: false,
            is_maximized: false,
        }
    }

    #[test]
    fn capture_candidates_skip_self_and_wrong_monitor() {
        let target = monitor("display-2", -1920, 0, 1920, 1080);
        let mut candidate = window("Discord.exe", "Discord", "display-2", -800, 100, 700, 600);
        assert!(capture_window_candidate(&candidate, &target));

        candidate.is_minimized = true;
        assert!(!capture_window_candidate(&candidate, &target));

        let self_window = window(
            "WindowAutoLayout.exe",
            "WindowAutoLayout",
            "display-2",
            -800,
            100,
            700,
            600,
        );
        assert!(!capture_window_candidate(&self_window, &target));

        let other_monitor = window("Discord.exe", "Discord", "display-1", 100, 100, 700, 600);
        assert!(!capture_window_candidate(&other_monitor, &target));
    }

    #[test]
    fn capture_uses_monitor_id_before_overlap() {
        let target = monitor("display-2", -1920, 0, 1920, 1080);
        let barely_overlapping = window("Chat.exe", "Chat", "display-1", -10, 100, 600, 500);

        assert!(!capture_window_candidate(&barely_overlapping, &target));
    }

    #[test]
    fn capture_uses_window_center_when_monitor_id_is_missing() {
        let target = monitor("display-2", -1920, 0, 1920, 1080);
        let mut centered = window("Chat.exe", "Chat", "display-unknown", -1000, 100, 600, 500);
        centered.monitor_id = None;
        assert!(capture_window_candidate(&centered, &target));

        let mut off_target = window("Chat.exe", "Chat", "display-unknown", 100, 100, 600, 500);
        off_target.monitor_id = None;
        assert!(!capture_window_candidate(&off_target, &target));
    }

    #[test]
    fn captured_apps_use_target_monitor_layout_and_duplicate_title_rules() {
        let target = monitor("display-2", -1920, 160, 1920, 1080);
        let windows = vec![
            window("chrome.exe", "Docs", "display-2", -1900, 200, 900, 700),
            window("chrome.exe", "Calendar", "display-2", -980, 200, 900, 700),
            window(
                "obs64.exe",
                "OBS 32.1.2 - Profile",
                "display-2",
                -1900,
                920,
                1200,
                300,
            ),
        ];

        let apps = captured_apps_for_windows(&target, &windows);
        assert_eq!(apps.len(), 3);
        assert_eq!(apps[0].layout.x, 20);
        assert_eq!(apps[0].layout.y, 40);
        assert_eq!(apps[0].target_monitor_id.as_deref(), Some("display-2"));

        let chrome_rule = apps[0]
            .title_rule
            .as_ref()
            .expect("duplicate chrome title rule");
        assert!(matches!(&chrome_rule.mode, TitleMatchMode::Exact));
        assert_eq!(chrome_rule.value, "Docs");

        let obs_rule = apps[2].title_rule.as_ref().expect("obs title rule");
        assert!(matches!(&obs_rule.mode, TitleMatchMode::StartsWith));
        assert_eq!(obs_rule.value, "OBS");
        assert_eq!(apps[2].display_name, "OBS Studio");
    }

    #[test]
    fn capture_preserves_maximized_window_state() {
        let target = monitor("display-1", 0, 0, 1920, 1080);
        let mut maximized = window("Editor.exe", "Editor", "display-1", 0, 0, 1920, 1080);
        maximized.is_maximized = true;

        let apps = captured_apps_for_windows(&target, &[maximized]);

        assert_eq!(apps[0].window_state, WindowStatePreference::Maximized);
    }
}
