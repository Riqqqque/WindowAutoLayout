use tauri::{AppHandle, Manager};

use crate::{
    config,
    errors::{AppError, AppResult},
    logging,
    models::{LogSeverity, WindowAutoLayoutConfig},
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
    let next_config = {
        let mut lock = state
            .layout_lock
            .lock()
            .map_err(|_| AppError::Config("Layout lock state was poisoned".to_string()))?;
        let mut config = state
            .config
            .lock()
            .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?;
        let next_profile_id = profile_id
            .or_else(|| lock.profile_id.clone())
            .or_else(|| config.enforcement.profile_id.clone())
            .or_else(|| config.startup.default_profile_id.clone());

        lock.generation = lock.generation.wrapping_add(1);
        lock.enabled = enabled;
        lock.profile_id = next_profile_id.clone();
        config.enforcement.enabled = enabled;
        config.enforcement.profile_id = next_profile_id;
        config.clone()
    };
    config::save(&state.config_dir, &next_config)?;

    logging::append(
        &state.config_dir,
        LogSeverity::Info,
        None,
        None,
        if enabled {
            "Event-driven layout lock enabled"
        } else {
            "Layout lock disabled"
        },
    )?;

    Ok(enabled)
}

pub fn sync_from_config(app: &AppHandle, config: &WindowAutoLayoutConfig) -> AppResult<()> {
    let state = app.state::<AppState>();
    let mut lock = state
        .layout_lock
        .lock()
        .map_err(|_| AppError::Config("Layout lock state was poisoned".to_string()))?;
    let next_profile_id = config
        .enforcement
        .profile_id
        .clone()
        .or_else(|| config.startup.default_profile_id.clone());

    if lock.enabled != config.enforcement.enabled || lock.profile_id != next_profile_id {
        lock.generation = lock.generation.wrapping_add(1);
        lock.enabled = config.enforcement.enabled;
        lock.profile_id = next_profile_id;
    }

    Ok(())
}

pub fn toggle(app: &AppHandle, profile_id: Option<String>) -> AppResult<bool> {
    set(app, !enabled(app)?, profile_id)
}
