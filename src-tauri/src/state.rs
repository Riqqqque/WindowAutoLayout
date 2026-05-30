use std::{path::PathBuf, sync::Mutex};

use crate::models::WindowAutoLayoutConfig;

#[derive(Clone, Debug, Default)]
pub struct LayoutLockState {
    pub enabled: bool,
    pub generation: u64,
    pub profile_id: Option<String>,
}

pub struct AppState {
    pub config_dir: PathBuf,
    pub config: Mutex<WindowAutoLayoutConfig>,
    pub layout_lock: Mutex<LayoutLockState>,
}

impl AppState {
    pub fn new(config_dir: PathBuf, config: WindowAutoLayoutConfig) -> Self {
        let layout_lock = LayoutLockState {
            enabled: config.enforcement.enabled,
            generation: 0,
            profile_id: config
                .enforcement
                .profile_id
                .clone()
                .or_else(|| config.startup.default_profile_id.clone()),
        };

        Self {
            config_dir,
            config: Mutex::new(config),
            layout_lock: Mutex::new(layout_lock),
        }
    }
}
