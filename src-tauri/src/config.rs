use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use regex::RegexBuilder;
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    models::{MonitorMissingBehavior, WindowAutoLayoutConfig, APP_VERSION, CONFIG_SCHEMA_VERSION},
};

pub const CONFIG_FILE_NAME: &str = "config.json";
const MAX_STARTUP_DELAY_SECONDS: u64 = 300;
const MAX_LAUNCH_DELAY_SECONDS: u64 = 120;
const MAX_LAYOUT_DIMENSION: i32 = 32_768;
const MAX_LAYOUT_OFFSET: i32 = 131_072;

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
    let raw = raw.trim_start_matches('\u{feff}');
    match serde_json::from_str::<WindowAutoLayoutConfig>(raw) {
        Ok(mut config) => {
            let mut should_save = false;
            if config.schema_version != CONFIG_SCHEMA_VERSION {
                backup_config(&path, "pre-migration")?;
                migrate_config(&mut config);
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

pub fn parse_json(raw: &str) -> AppResult<WindowAutoLayoutConfig> {
    let mut config =
        serde_json::from_str::<WindowAutoLayoutConfig>(raw.trim_start_matches('\u{feff}'))?;
    normalize_config(&mut config);
    Ok(config)
}

pub fn normalize_config(config: &mut WindowAutoLayoutConfig) -> bool {
    let mut changed = false;

    if config.schema_version != CONFIG_SCHEMA_VERSION {
        config.schema_version = CONFIG_SCHEMA_VERSION;
        changed = true;
    }
    if config.app_version != APP_VERSION {
        config.app_version = APP_VERSION.to_string();
        changed = true;
    }

    if !config.startup.launch_missing_apps {
        config.startup.launch_missing_apps = true;
        changed = true;
    }

    let startup_delay = config.startup.delay_seconds.min(MAX_STARTUP_DELAY_SECONDS);
    if config.startup.delay_seconds != startup_delay {
        config.startup.delay_seconds = startup_delay;
        changed = true;
    }

    if config.profiles.is_empty() {
        config.profiles = WindowAutoLayoutConfig::default().profiles;
        changed = true;
    }

    let mut profile_ids = HashSet::new();
    for (profile_index, profile) in config.profiles.iter_mut().enumerate() {
        if profile.id.trim().is_empty() || !profile_ids.insert(profile.id.clone()) {
            profile.id = unique_id("profile", &mut profile_ids);
            changed = true;
        }
        if profile.name.trim().is_empty() {
            profile.name = format!("Profile {}", profile_index + 1);
            changed = true;
        }

        let mut app_ids = HashSet::new();
        for (app_index, app) in profile.apps.iter_mut().enumerate() {
            if app.id.trim().is_empty() || !app_ids.insert(app.id.clone()) {
                app.id = unique_id("app", &mut app_ids);
                changed = true;
            }
            if app.display_name.trim().is_empty() {
                app.display_name = app
                    .process_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(|name| name.trim_end_matches(".exe").to_string())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| format!("App {}", app_index + 1));
                changed = true;
            }

            let launch_delay_seconds = app.launch_delay_seconds.min(MAX_LAUNCH_DELAY_SECONDS);
            if app.launch_delay_seconds != launch_delay_seconds {
                app.launch_delay_seconds = launch_delay_seconds;
                changed = true;
            }

            let retry_interval_ms = app.retry_interval_ms.clamp(250, 5000);
            if app.retry_interval_ms != retry_interval_ms {
                app.retry_interval_ms = retry_interval_ms;
                changed = true;
            }

            let detection_timeout_seconds = app.detection_timeout_seconds.clamp(1, 120);
            if app.detection_timeout_seconds != detection_timeout_seconds {
                app.detection_timeout_seconds = detection_timeout_seconds;
                changed = true;
            }

            let x = app.layout.x.clamp(-MAX_LAYOUT_OFFSET, MAX_LAYOUT_OFFSET);
            let y = app.layout.y.clamp(-MAX_LAYOUT_OFFSET, MAX_LAYOUT_OFFSET);
            let width = app.layout.width.clamp(80, MAX_LAYOUT_DIMENSION);
            let height = app.layout.height.clamp(80, MAX_LAYOUT_DIMENSION);
            if (
                app.layout.x,
                app.layout.y,
                app.layout.width,
                app.layout.height,
            ) != (x, y, width, height)
            {
                app.layout.x = x;
                app.layout.y = y;
                app.layout.width = width;
                app.layout.height = height;
                changed = true;
            }
        }
    }

    let first_profile_id = config.profiles[0].id.clone();
    if !config
        .startup
        .default_profile_id
        .as_ref()
        .is_some_and(|id| config.profiles.iter().any(|profile| &profile.id == id))
    {
        config.startup.default_profile_id = Some(first_profile_id.clone());
        changed = true;
    }

    if !config
        .enforcement
        .profile_id
        .as_ref()
        .is_some_and(|id| config.profiles.iter().any(|profile| &profile.id == id))
    {
        config.enforcement.profile_id = config
            .startup
            .default_profile_id
            .clone()
            .or(Some(first_profile_id));
        changed = true;
    }

    changed
}

fn unique_id(prefix: &str, seen: &mut HashSet<String>) -> String {
    loop {
        let candidate = format!("{prefix}-{}", Uuid::new_v4());
        if seen.insert(candidate.clone()) {
            return candidate;
        }
    }
}

fn migrate_config(config: &mut WindowAutoLayoutConfig) {
    if config.schema_version < 3
        && matches!(
            config.global.monitor_missing_behavior,
            MonitorMissingBehavior::DoNothing
        )
    {
        config.global.monitor_missing_behavior = MonitorMissingBehavior::NearestMatch;
    }
}

pub fn backup_config(path: &Path, reason: &str) -> AppResult<PathBuf> {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let backup = path.with_file_name(format!("config.{reason}.{timestamp}.bak.json"));
    fs::copy(path, &backup)?;
    Ok(backup)
}

pub fn validate_config(config: &WindowAutoLayoutConfig) -> Vec<String> {
    let mut issues = Vec::new();
    let mut profile_ids = HashSet::new();
    if config.profiles.is_empty() {
        issues.push("At least one profile is required".to_string());
    }
    for profile in &config.profiles {
        if profile.id.trim().is_empty() || !profile_ids.insert(profile.id.as_str()) {
            issues.push("Profile IDs must be present and unique".to_string());
        }
        if profile.name.trim().is_empty() {
            issues.push(format!("Profile {} has an empty name", profile.id));
        }

        let mut app_ids = HashSet::new();
        for app in &profile.apps {
            if app.id.trim().is_empty() || !app_ids.insert(app.id.as_str()) {
                issues.push(format!(
                    "{} contains duplicate or empty app IDs",
                    profile.name
                ));
            }
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
            if app.retry_interval_ms < 250 {
                issues.push(format!(
                    "{} has a retry interval below 250 ms",
                    app.display_name
                ));
            }
            if app
                .process_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
                && app
                    .executable_path
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_none()
            {
                issues.push(format!(
                    "{} needs a process name or executable path",
                    app.display_name
                ));
            }
            if let Some(rule) = &app.title_rule {
                if matches!(rule.mode, crate::models::TitleMatchMode::Regex)
                    && RegexBuilder::new(&rule.value)
                        .case_insensitive(!rule.case_sensitive)
                        .build()
                        .is_err()
                {
                    issues.push(format!("{} has an invalid title regex", app.display_name));
                }
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
        assert_eq!(
            parsed.global.monitor_missing_behavior,
            MonitorMissingBehavior::NearestMatch
        );
    }

    #[test]
    fn migration_moves_old_default_monitor_behavior_to_nearest_match() {
        let mut config = WindowAutoLayoutConfig {
            schema_version: 2,
            ..WindowAutoLayoutConfig::default()
        };
        config.global.monitor_missing_behavior = MonitorMissingBehavior::DoNothing;

        migrate_config(&mut config);

        assert_eq!(
            config.global.monitor_missing_behavior,
            MonitorMissingBehavior::NearestMatch
        );
    }

    #[test]
    fn validates_tiny_layouts() {
        let mut config = WindowAutoLayoutConfig::default();
        config.profiles[0].apps[0].layout.width = 20;
        let issues = validate_config(&config);
        assert!(issues.iter().any(|issue| issue.contains("very small")));
    }

    #[test]
    fn normalizes_active_restore_retry_limits() {
        let mut config = WindowAutoLayoutConfig::default();
        config.profiles[0].apps[0].retry_interval_ms = 10;
        config.profiles[0].apps[0].detection_timeout_seconds = 900;

        assert!(normalize_config(&mut config));
        assert_eq!(config.profiles[0].apps[0].retry_interval_ms, 250);
        assert_eq!(config.profiles[0].apps[0].detection_timeout_seconds, 120);
    }

    #[test]
    fn legacy_ask_next_open_value_migrates_to_do_nothing() {
        let json = serde_json::to_string(&WindowAutoLayoutConfig::default())
            .expect("serialize")
            .replace("nearestMatch", "askNextOpen");

        let parsed = parse_json(&json).expect("parse legacy config");

        assert_eq!(
            parsed.global.monitor_missing_behavior,
            MonitorMissingBehavior::DoNothing
        );
    }

    #[test]
    fn normalization_repairs_ids_delays_and_unsafe_layout_bounds() {
        let mut config = WindowAutoLayoutConfig::default();
        let duplicate = config.profiles[0].apps[0].clone();
        config.profiles[0].apps.push(duplicate);
        config.profiles[0].id.clear();
        config.profiles[0].name.clear();
        config.profiles[0].apps[0].id.clear();
        config.profiles[0].apps[0].launch_delay_seconds = u64::MAX;
        config.profiles[0].apps[0].layout.width = i32::MAX;
        config.startup.default_profile_id = Some("missing".into());
        config.enforcement.profile_id = Some("missing".into());

        assert!(normalize_config(&mut config));

        let profile = &config.profiles[0];
        assert!(!profile.id.is_empty());
        assert_eq!(profile.name, "Profile 1");
        assert_ne!(profile.apps[0].id, profile.apps[1].id);
        assert_eq!(
            profile.apps[0].launch_delay_seconds,
            MAX_LAUNCH_DELAY_SECONDS
        );
        assert_eq!(profile.apps[0].layout.width, MAX_LAYOUT_DIMENSION);
        assert_eq!(
            config.startup.default_profile_id.as_deref(),
            Some(profile.id.as_str())
        );
        assert_eq!(
            config.enforcement.profile_id.as_deref(),
            Some(profile.id.as_str())
        );
    }

    #[test]
    fn validation_reports_invalid_regex_and_missing_process_identity() {
        let mut config = WindowAutoLayoutConfig::default();
        let app = &mut config.profiles[0].apps[0];
        app.process_name = None;
        app.executable_path = None;
        app.title_rule = Some(crate::models::MatchRule {
            mode: crate::models::TitleMatchMode::Regex,
            value: "[".into(),
            case_sensitive: false,
        });

        let issues = validate_config(&config);

        assert!(issues.iter().any(|issue| issue.contains("process name")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("invalid title regex")));
    }
}
