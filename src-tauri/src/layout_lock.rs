use std::{thread, time::Duration};

use tauri::{AppHandle, Manager};

use crate::{
    config,
    errors::{AppError, AppResult},
    logging,
    models::{LogSeverity, MonitorInfo, WindowAutoLayoutConfig, WindowInfo},
    monitors, profiles,
    state::AppState,
    windows_enum,
};

pub fn enabled(app: &AppHandle) -> AppResult<bool> {
    let state = app.state::<AppState>();
    let enabled = state
        .layout_lock
        .lock()
        .map_err(|_| AppError::Config("Layout lock state was poisoned".to_string()))?
        .enabled;
    Ok(enabled)
}

pub fn set(app: &AppHandle, enabled: bool, profile_id: Option<String>) -> AppResult<bool> {
    let state = app.state::<AppState>();
    let (generation, next_config) = {
        let mut lock = state
            .layout_lock
            .lock()
            .map_err(|_| AppError::Config("Layout lock state was poisoned".to_string()))?;
        let mut config = state
            .config
            .lock()
            .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?;
        let next_profile_id = profile_id
            .clone()
            .or_else(|| lock.profile_id.clone())
            .or_else(|| config.enforcement.profile_id.clone())
            .or_else(|| config.startup.default_profile_id.clone());

        lock.generation = lock.generation.wrapping_add(1);
        lock.enabled = enabled;
        lock.profile_id = next_profile_id.clone();
        config.enforcement.enabled = enabled;
        config.enforcement.profile_id = next_profile_id;
        (lock.generation, config.clone())
    };
    config::save(&state.config_dir, &next_config)?;

    logging::append(
        &state.config_dir,
        LogSeverity::Info,
        None,
        None,
        if enabled {
            "Layout lock enabled"
        } else {
            "Layout lock disabled"
        },
    )?;

    if enabled {
        spawn(app.clone(), generation);
    }

    Ok(enabled)
}

pub fn sync_from_config(app: &AppHandle, config: &WindowAutoLayoutConfig) -> AppResult<()> {
    let state = app.state::<AppState>();
    let (enabled, generation) = {
        let mut lock = state
            .layout_lock
            .lock()
            .map_err(|_| AppError::Config("Layout lock state was poisoned".to_string()))?;
        let next_profile_id = config
            .enforcement
            .profile_id
            .clone()
            .or_else(|| config.startup.default_profile_id.clone());

        if lock.enabled == config.enforcement.enabled && lock.profile_id == next_profile_id {
            return Ok(());
        }

        lock.generation = lock.generation.wrapping_add(1);
        lock.enabled = config.enforcement.enabled;
        lock.profile_id = next_profile_id;
        (lock.enabled, lock.generation)
    };

    if enabled {
        spawn(app.clone(), generation);
    }

    Ok(())
}

pub fn toggle(app: &AppHandle, profile_id: Option<String>) -> AppResult<bool> {
    let next = !enabled(app)?;
    set(app, next, profile_id)
}

fn spawn(app: AppHandle, generation: u64) {
    let mut allow_launch_missing = true;
    thread::spawn(move || loop {
        let (config_dir, config, profile_id, interval_ms) = {
            let state = app.state::<AppState>();
            let lock = match state.layout_lock.lock() {
                Ok(lock) => lock.clone(),
                Err(_) => break,
            };
            if !lock.enabled || lock.generation != generation {
                break;
            }

            let config = match state.config.lock() {
                Ok(config) => config.clone(),
                Err(_) => break,
            };
            let interval_ms = config.enforcement.interval_ms.clamp(1000, 5000);
            (
                state.config_dir.clone(),
                config,
                lock.profile_id.clone(),
                interval_ms,
            )
        };

        if !should_pause_for_fullscreen(&config, profile_id.as_deref()) {
            let _ = profiles::restore_profile_silent(
                &config_dir,
                &config,
                profile_id.clone(),
                Some(allow_launch_missing),
            );
            allow_launch_missing = false;
        }
        thread::sleep(Duration::from_millis(interval_ms));
    });
}

fn should_pause_for_fullscreen(config: &WindowAutoLayoutConfig, profile_id: Option<&str>) -> bool {
    if !config.enforcement.pause_for_fullscreen_games {
        return false;
    }

    let Some(window) = windows_enum::foreground_window() else {
        return false;
    };
    if !is_fullscreen_window(&window) {
        return false;
    }

    profiles::resolve_profile(config, profile_id)
        .map(|profile| !profiles::window_matches_profile_app(profile, &window))
        .unwrap_or(true)
}

fn is_fullscreen_window(window: &WindowInfo) -> bool {
    if !window.is_visible || window.is_minimized {
        return false;
    }

    monitors::list_monitors()
        .unwrap_or_default()
        .iter()
        .any(|monitor| covers_monitor(window, monitor))
}

fn covers_monitor(window: &WindowInfo, monitor: &MonitorInfo) -> bool {
    let tolerance = 4;
    (window.x - monitor.x).abs() <= tolerance
        && (window.y - monitor.y).abs() <= tolerance
        && window.width >= monitor.width - tolerance
        && window.height >= monitor.height - tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_monitor_covering_windows() {
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
            process_name: "game.exe".into(),
            executable_path: None,
            monitor_id: Some("display".into()),
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
            is_visible: true,
            is_minimized: false,
        };
        let maximized = WindowInfo {
            height: 1392,
            ..fullscreen.clone()
        };

        assert!(covers_monitor(&fullscreen, &monitor));
        assert!(!covers_monitor(&maximized, &monitor));
    }
}
