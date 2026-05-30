use std::{thread, time::Duration};

use tauri::{AppHandle, Manager};

use crate::{
    config,
    errors::{AppError, AppResult},
    logging,
    models::LogSeverity,
    profiles,
    state::AppState,
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

pub fn toggle(app: &AppHandle, profile_id: Option<String>) -> AppResult<bool> {
    let next = !enabled(app)?;
    set(app, next, profile_id)
}

fn spawn(app: AppHandle, generation: u64) {
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
            let interval_ms = config.enforcement.interval_ms.clamp(150, 250);
            (
                state.config_dir.clone(),
                config,
                lock.profile_id.clone(),
                interval_ms,
            )
        };

        let _ = profiles::restore_profile_silent(&config_dir, &config, profile_id, Some(true));
        thread::sleep(Duration::from_millis(interval_ms));
    });
}
