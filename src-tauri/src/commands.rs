use std::{path::PathBuf, process::Command};

use tauri::{AppHandle, Manager, State};

use crate::{
    config,
    errors::{AppError, AppResult},
    logging,
    models::{
        preset_apps, AppConfig, LogEntry, LogSeverity, MonitorInfo, RestoreResult,
        WindowAutoLayoutConfig, WindowInfo,
    },
    monitors, profiles, startup,
    state::AppState,
    window_actions, windows_enum,
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
pub fn save_config(
    state: State<'_, AppState>,
    mut next_config: WindowAutoLayoutConfig,
) -> AppResult<WindowAutoLayoutConfig> {
    next_config.schema_version = crate::models::CONFIG_SCHEMA_VERSION;
    next_config.app_version = crate::models::APP_VERSION.to_string();
    next_config.startup.launch_missing_apps = true;
    for profile in &mut next_config.profiles {
        for app in &mut profile.apps {
            app.launch_if_missing = true;
        }
    }
    startup::set_startup_enabled(next_config.startup.enabled)?;
    config::save(&state.config_dir, &next_config)?;
    *state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))? =
        next_config.clone();
    logging::append(
        &state.config_dir,
        LogSeverity::Info,
        None,
        None,
        "Settings saved",
    )?;
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
pub async fn lock_layout_temporarily(
    state: State<'_, AppState>,
    profile_id: Option<String>,
    duration_seconds: u64,
) -> AppResult<RestoreResult> {
    let config_dir = state.config_dir.clone();
    let config = state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .clone();
    let interval_ms = config.enforcement.interval_ms;

    tauri::async_runtime::spawn_blocking(move || {
        profiles::enforce_profile_for(
            &config_dir,
            &config,
            profile_id,
            duration_seconds,
            interval_ms,
        )
    })
    .await
    .map_err(|error| AppError::Config(format!("Lock task failed: {error}")))?
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
    let monitor_id = window
        .monitor_id
        .clone()
        .ok_or_else(|| AppError::MonitorNotFound)?;
    let monitor = monitors::list_monitors()?
        .into_iter()
        .find(|monitor| monitor.id == monitor_id)
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
    logging::append(
        &state.config_dir,
        LogSeverity::Info,
        Some(&profile_id),
        Some(&app_id),
        "Captured window layout",
    )?;
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
        let Some(monitor) = monitors.iter().find(|monitor| &monitor.id == monitor_id) else {
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
    logging::append(
        &state.config_dir,
        LogSeverity::Info,
        Some(&profile_id),
        None,
        "Captured layouts for matching configured apps",
    )?;
    Ok(next_config)
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
    if let Some(window) = app.get_webview_window("main") {
        window
            .show()
            .map_err(|error| AppError::Config(error.to_string()))?;
        window
            .unminimize()
            .map_err(|error| AppError::Config(error.to_string()))?;
        window
            .set_focus()
            .map_err(|error| AppError::Config(error.to_string()))?;
    }
    Ok(())
}

fn open_path(path: PathBuf) -> AppResult<()> {
    Command::new("explorer")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(AppError::Io)
}
