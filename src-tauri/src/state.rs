use std::{path::PathBuf, sync::Mutex};

use crate::models::WindowAutoLayoutConfig;

pub struct AppState {
    pub config_dir: PathBuf,
    pub config: Mutex<WindowAutoLayoutConfig>,
}

impl AppState {
    pub fn new(config_dir: PathBuf, config: WindowAutoLayoutConfig) -> Self {
        Self {
            config_dir,
            config: Mutex::new(config),
        }
    }
}
