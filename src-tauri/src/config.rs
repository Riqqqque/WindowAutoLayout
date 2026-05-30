use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;

use crate::{
    errors::{AppError, AppResult},
    models::{WindowAutoLayoutConfig, APP_VERSION, CONFIG_SCHEMA_VERSION},
};

pub const CONFIG_FILE_NAME: &str = "config.json";

pub fn config_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CONFIG_FILE_NAME)
}

pub fn load_or_create(config_dir: &Path) -> AppResult<WindowAutoLayoutConfig> {
    fs::create_dir_all(config_dir)?;
    let path = config_file_path(config_dir);

    if !path.exists() {
        let mut config = WindowAutoLayoutConfig::default();
        normalize_config(&mut config);
        save(config_dir, &config)?;
        return Ok(config);
    }

    let raw = fs::read_to_string(&path)?;
    match serde_json::from_str::<WindowAutoLayoutConfig>(&raw) {
        Ok(mut config) => {
            let mut should_save = false;
            if config.schema_version != CONFIG_SCHEMA_VERSION {
                backup_config(&path, "pre-migration")?;
                config.schema_version = CONFIG_SCHEMA_VERSION;
                should_save = true;
            }
            if config.app_version != APP_VERSION {
                config.app_version = APP_VERSION.to_string();
                should_save = true;
            }
            if normalize_config(&mut config) {
                should_save = true;
            }
            if should_save {
                save(config_dir, &config)?;
            }
            Ok(config)
        }
        Err(error) => {
            backup_config(&path, "corrupt")?;
            let mut config = WindowAutoLayoutConfig::default();
            normalize_config(&mut config);
            save(config_dir, &config)?;
            Err(AppError::Config(format!(
                "Config was unreadable and was backed up before creating a fresh file: {error}"
            )))
        }
    }
}

pub fn save(config_dir: &Path, config: &WindowAutoLayoutConfig) -> AppResult<()> {
    fs::create_dir_all(config_dir)?;
    let path = config_file_path(config_dir);
    let temp_path = path.with_extension("json.tmp");
    let mut config = config.clone();
    normalize_config(&mut config);
    let payload = serde_json::to_string_pretty(&config)?;
    fs::write(&temp_path, payload)?;
    fs::rename(temp_path, path)?;
    Ok(())
}

fn normalize_config(config: &mut WindowAutoLayoutConfig) -> bool {
    let mut changed = false;

    if !config.startup.launch_missing_apps {
        config.startup.launch_missing_apps = true;
        changed = true;
    }

    for profile in &mut config.profiles {
        for app in &mut profile.apps {
            if !app.launch_if_missing {
                app.launch_if_missing = true;
                changed = true;
            }
        }
    }

    if let Some(profile_id) = &config.enforcement.profile_id {
        if !config
            .profiles
            .iter()
            .any(|profile| &profile.id == profile_id)
        {
            config.enforcement.profile_id = None;
            changed = true;
        }
    }

    let interval_ms = config.enforcement.interval_ms.clamp(150, 250);
    if config.enforcement.interval_ms != interval_ms {
        config.enforcement.interval_ms = interval_ms;
        changed = true;
    }

    changed
}

pub fn backup_config(path: &Path, reason: &str) -> AppResult<PathBuf> {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let backup = path.with_file_name(format!("config.{reason}.{timestamp}.bak.json"));
    fs::copy(path, &backup)?;
    Ok(backup)
}

pub fn validate_config(config: &WindowAutoLayoutConfig) -> Vec<String> {
    let mut issues = Vec::new();
    for profile in &config.profiles {
        if profile.name.trim().is_empty() {
            issues.push(format!("Profile {} has an empty name", profile.id));
        }

        for app in &profile.apps {
            if app.display_name.trim().is_empty() {
                issues.push(format!(
                    "Profile {} contains an app with an empty display name",
                    profile.name
                ));
            }
            if app.layout.width < 80 || app.layout.height < 80 {
                issues.push(format!(
                    "{} has a very small saved size ({}x{})",
                    app.display_name, app.layout.width, app.layout.height
                ));
            }
            if app.retry_interval_ms < 100 {
                issues.push(format!(
                    "{} has a retry interval below 100 ms",
                    app.display_name
                ));
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trips() {
        let config = WindowAutoLayoutConfig::default();
        let json = serde_json::to_string_pretty(&config).expect("serialize");
        let parsed: WindowAutoLayoutConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.schema_version, CONFIG_SCHEMA_VERSION);
        assert_eq!(parsed.profiles[0].name, "Streaming");
    }

    #[test]
    fn validates_tiny_layouts() {
        let mut config = WindowAutoLayoutConfig::default();
        config.profiles[0].apps[0].layout.width = 20;
        let issues = validate_config(&config);
        assert!(issues.iter().any(|issue| issue.contains("very small")));
    }
}
