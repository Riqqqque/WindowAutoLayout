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
    models::{
        AppConfig, CapturedDisplay, MonitorInfo, MonitorMissingBehavior, Profile,
        WindowAutoLayoutConfig, APP_VERSION, CONFIG_SCHEMA_VERSION,
    },
    monitors::monitor_id_matches,
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

            let invalid_captured_display = app.captured_display.as_ref().is_some_and(|display| {
                !(display.width > 0
                    && display.height > 0
                    && display.work_width > 0
                    && display.work_height > 0)
            });
            if invalid_captured_display {
                app.captured_display = None;
                changed = true;
            } else if let Some(display) = &mut app.captured_display {
                let width = display.width.clamp(1, MAX_LAYOUT_DIMENSION);
                let height = display.height.clamp(1, MAX_LAYOUT_DIMENSION);
                let work_x = display.work_x.clamp(0, width.saturating_sub(1));
                let work_y = display.work_y.clamp(0, height.saturating_sub(1));
                let work_width = display.work_width.clamp(1, width - work_x);
                let work_height = display.work_height.clamp(1, height - work_y);
                let scale_percent = display.scale_percent.clamp(50, 500);
                if (
                    display.width,
                    display.height,
                    display.work_x,
                    display.work_y,
                    display.work_width,
                    display.work_height,
                    display.scale_percent,
                ) != (
                    width,
                    height,
                    work_x,
                    work_y,
                    work_width,
                    work_height,
                    scale_percent,
                ) {
                    display.width = width;
                    display.height = height;
                    display.work_x = work_x;
                    display.work_y = work_y;
                    display.work_width = work_width;
                    display.work_height = work_height;
                    display.scale_percent = scale_percent;
                    changed = true;
                }
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

pub fn hydrate_captured_displays(
    config: &mut WindowAutoLayoutConfig,
    monitors: &[MonitorInfo],
) -> bool {
    let default_monitor_id = config.global.default_monitor_id.clone();
    let mut changed = false;
    for profile in &mut config.profiles {
        let profile_monitor_id = profile
            .target_monitor_id
            .as_ref()
            .or(default_monitor_id.as_ref())
            .cloned();
        for app in &mut profile.apps {
            if app.captured_display.is_some() {
                continue;
            }
            let monitor_id = app
                .target_monitor_id
                .as_ref()
                .or(profile_monitor_id.as_ref());
            let Some(monitor) = monitor_id.and_then(|id| {
                monitors
                    .iter()
                    .find(|monitor| monitor_id_matches(monitor, id))
            }) else {
                continue;
            };
            app.captured_display = Some(CapturedDisplay::from_monitor(monitor));
            changed = true;
        }
    }
    changed
}

pub fn reconcile_monitor_targets(
    config: &mut WindowAutoLayoutConfig,
    monitors: &[MonitorInfo],
) -> bool {
    if monitors.is_empty() {
        return false;
    }

    let default_profile_size = config
        .startup
        .default_profile_id
        .as_ref()
        .and_then(|id| config.profiles.iter().find(|profile| &profile.id == id))
        .or_else(|| config.profiles.first())
        .and_then(profile_target_size);
    let default_resolution = config
        .global
        .default_monitor_id
        .as_deref()
        .and_then(|id| resolve_saved_monitor(id, default_profile_size, monitors));
    let default_retargeted = default_resolution
        .as_ref()
        .is_some_and(|resolution| resolution.legacy_retargeted);
    let mut changed = false;
    if let (Some(saved), Some(resolution)) = (
        config.global.default_monitor_id.as_ref(),
        default_resolution.as_ref(),
    ) {
        if !saved.eq_ignore_ascii_case(&resolution.monitor.id) {
            config.global.default_monitor_id = Some(resolution.monitor.id.clone());
            changed = true;
        }
    }

    let default_monitor_id = config.global.default_monitor_id.clone();
    for profile in &mut config.profiles {
        let explicit_profile_target = profile.target_monitor_id.clone();
        let requested_profile_target = explicit_profile_target
            .as_deref()
            .or(default_monitor_id.as_deref());
        let profile_resolution = requested_profile_target
            .and_then(|id| resolve_saved_monitor(id, profile_target_size(profile), monitors));
        let inherited_default_retarget = explicit_profile_target.is_none() && default_retargeted;
        let profile_retargeted = profile_resolution
            .as_ref()
            .is_some_and(|resolution| resolution.legacy_retargeted)
            || inherited_default_retarget;

        if explicit_profile_target.is_some() {
            if let Some(resolution) = &profile_resolution {
                if explicit_profile_target
                    .as_ref()
                    .is_some_and(|id| !id.eq_ignore_ascii_case(&resolution.monitor.id))
                {
                    profile.target_monitor_id = Some(resolution.monitor.id.clone());
                    changed = true;
                }
            }
        }

        let inherited_monitor = profile_resolution
            .as_ref()
            .map(|resolution| &resolution.monitor);
        for app in &mut profile.apps {
            let explicit_app_target = app.target_monitor_id.clone();
            let app_resolution = explicit_app_target
                .as_deref()
                .and_then(|id| resolve_saved_monitor(id, app_target_size(app), monitors));
            let app_monitor = app_resolution
                .as_ref()
                .map(|resolution| &resolution.monitor)
                .or(inherited_monitor);
            let app_retargeted = app_resolution
                .as_ref()
                .is_some_and(|resolution| resolution.legacy_retargeted)
                || (explicit_app_target.is_none() && profile_retargeted);

            if explicit_app_target.is_some() {
                if let Some(resolution) = &app_resolution {
                    if explicit_app_target
                        .as_ref()
                        .is_some_and(|id| !id.eq_ignore_ascii_case(&resolution.monitor.id))
                    {
                        app.target_monitor_id = Some(resolution.monitor.id.clone());
                        changed = true;
                    }
                }
            }

            if app_retargeted {
                if let Some(monitor) = app_monitor {
                    let captured = CapturedDisplay::from_monitor(monitor);
                    if app.captured_display.as_ref() != Some(&captured) {
                        app.captured_display = Some(captured);
                        changed = true;
                    }
                }
            }
        }
    }

    changed
}

struct SavedMonitorResolution<'a> {
    monitor: &'a MonitorInfo,
    legacy_retargeted: bool,
}

fn resolve_saved_monitor<'a>(
    saved_id: &str,
    target_size: Option<(i32, i32)>,
    monitors: &'a [MonitorInfo],
) -> Option<SavedMonitorResolution<'a>> {
    if let Some(monitor) = monitors.iter().find(|monitor| {
        monitor.id.eq_ignore_ascii_case(saved_id)
            && !monitor.device_name.eq_ignore_ascii_case(saved_id)
    }) {
        return Some(SavedMonitorResolution {
            monitor,
            legacy_retargeted: false,
        });
    }

    let legacy_match = monitors
        .iter()
        .find(|monitor| monitor.device_name.eq_ignore_ascii_case(saved_id))
        .or_else(|| {
            monitors
                .iter()
                .find(|monitor| monitor.id.eq_ignore_ascii_case(saved_id))
        })?;
    let Some((target_width, target_height)) = target_size else {
        return Some(SavedMonitorResolution {
            monitor: legacy_match,
            legacy_retargeted: false,
        });
    };

    let overflows_legacy = target_width > legacy_match.width.saturating_add(16)
        || target_height > legacy_match.height.saturating_add(16);
    if overflows_legacy {
        if let Some(best) = monitors
            .iter()
            .filter(|monitor| {
                target_width <= monitor.width.saturating_add(16)
                    && target_height <= monitor.height.saturating_add(16)
            })
            .min_by_key(|monitor| monitor_size_score(monitor, target_width, target_height))
        {
            if best.id != legacy_match.id {
                return Some(SavedMonitorResolution {
                    monitor: best,
                    legacy_retargeted: true,
                });
            }
        }
    }

    Some(SavedMonitorResolution {
        monitor: legacy_match,
        legacy_retargeted: false,
    })
}

fn profile_target_size(profile: &Profile) -> Option<(i32, i32)> {
    let left = profile.apps.iter().map(|app| app.layout.x).min()?.min(0);
    let top = profile.apps.iter().map(|app| app.layout.y).min()?.min(0);
    let right = profile
        .apps
        .iter()
        .map(|app| app.layout.x.saturating_add(app.layout.width))
        .max()?;
    let bottom = profile
        .apps
        .iter()
        .map(|app| app.layout.y.saturating_add(app.layout.height))
        .max()?;
    Some((
        right.saturating_sub(left).max(1),
        bottom.saturating_sub(top).max(1),
    ))
}

fn app_target_size(app: &AppConfig) -> Option<(i32, i32)> {
    let left = app.layout.x.min(0);
    let top = app.layout.y.min(0);
    Some((
        app.layout
            .x
            .saturating_add(app.layout.width)
            .saturating_sub(left)
            .max(1),
        app.layout
            .y
            .saturating_add(app.layout.height)
            .saturating_sub(top)
            .max(1),
    ))
}

fn monitor_size_score(monitor: &MonitorInfo, width: i32, height: i32) -> u64 {
    i64::from(monitor.width).abs_diff(i64::from(width))
        + i64::from(monitor.height).abs_diff(i64::from(height))
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

    fn test_monitor(
        id: &str,
        device_name: &str,
        width: i32,
        height: i32,
        work_height: i32,
        is_primary: bool,
    ) -> MonitorInfo {
        MonitorInfo {
            id: id.into(),
            name: device_name.into(),
            device_name: device_name.into(),
            x: 0,
            y: 0,
            width,
            height,
            work_x: 0,
            work_y: 0,
            work_width: width,
            work_height,
            scale_factor: if width >= 3840 { 1.5 } else { 1.25 },
            is_primary,
        }
    }

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
    fn preserves_launch_missing_apps_preference() {
        let mut config = WindowAutoLayoutConfig::default();
        config.startup.launch_missing_apps = false;

        normalize_config(&mut config);

        assert!(!config.startup.launch_missing_apps);
    }

    #[test]
    fn normalizes_captured_work_area_inside_display_bounds() {
        let mut config = WindowAutoLayoutConfig::default();
        config.profiles[0].apps[0].captured_display = Some(CapturedDisplay {
            width: 1920,
            height: 1080,
            work_x: -50,
            work_y: 2000,
            work_width: 4000,
            work_height: 4000,
            scale_percent: 900,
        });

        assert!(normalize_config(&mut config));

        let display = config.profiles[0].apps[0]
            .captured_display
            .as_ref()
            .expect("captured display");
        assert_eq!(display.work_x, 0);
        assert_eq!(display.work_y, 1079);
        assert_eq!(display.work_width, 1920);
        assert_eq!(display.work_height, 1);
        assert_eq!(display.scale_percent, 500);
    }

    #[test]
    fn save_replaces_an_existing_config() {
        let config_dir =
            std::env::temp_dir().join(format!("windowautolayout-config-test-{}", Uuid::new_v4()));
        let first = WindowAutoLayoutConfig::default();
        save(&config_dir, &first).expect("save initial config");

        let mut second = first;
        second.startup.launch_missing_apps = false;
        save(&config_dir, &second).expect("replace config");

        let raw = fs::read_to_string(config_file_path(&config_dir)).expect("read config");
        let loaded: WindowAutoLayoutConfig = serde_json::from_str(&raw).expect("parse config");
        assert!(!loaded.startup.launch_missing_apps);
        let _ = fs::remove_dir_all(config_dir);
    }

    #[test]
    fn hydrates_legacy_app_display_geometry_from_its_saved_target() {
        let mut config = WindowAutoLayoutConfig::default();
        config.profiles[0].target_monitor_id = Some("display-2".into());
        let monitor = MonitorInfo {
            id: "display-2".into(),
            name: "Display 2".into(),
            device_name: "DISPLAY2".into(),
            x: -3840,
            y: 0,
            width: 3840,
            height: 2160,
            work_x: -3840,
            work_y: 0,
            work_width: 3840,
            work_height: 2080,
            scale_factor: 1.5,
            is_primary: false,
        };

        assert!(hydrate_captured_displays(&mut config, &[monitor]));
        let captured = config.profiles[0].apps[0]
            .captured_display
            .as_ref()
            .expect("captured display");
        assert_eq!(captured.width, 3840);
        assert_eq!(captured.work_height, 2080);
        assert_eq!(captured.scale_percent, 150);
    }

    #[test]
    fn repairs_legacy_display_number_after_windows_swaps_monitor_names() {
        let mut config = WindowAutoLayoutConfig::default();
        config.global.default_monitor_id = Some(r"\\.\DISPLAY2".into());
        let profile = &mut config.profiles[0];
        profile.target_monitor_id = Some(r"\\.\DISPLAY2".into());
        profile.apps[0].target_monitor_id = Some(r"\\.\DISPLAY2".into());
        profile.apps[0].layout = crate::models::LayoutRect {
            x: 1920,
            y: 0,
            width: 1920,
            height: 1080,
        };
        profile.apps[1].target_monitor_id = Some(r"\\.\DISPLAY2".into());
        profile.apps[1].layout = crate::models::LayoutRect {
            x: -6,
            y: 1078,
            width: 1914,
            height: 1007,
        };
        let wrong_capture = CapturedDisplay::from_monitor(&test_monitor(
            "hardware-1440",
            r"\\.\DISPLAY2",
            2560,
            1440,
            1380,
            true,
        ));
        for app in &mut profile.apps {
            app.captured_display = Some(wrong_capture.clone());
        }

        let monitors = vec![
            test_monitor("hardware-1440", r"\\.\DISPLAY2", 2560, 1440, 1380, true),
            test_monitor("hardware-4k", r"\\.\DISPLAY1", 3840, 2160, 2088, false),
        ];

        assert!(reconcile_monitor_targets(&mut config, &monitors));
        assert_eq!(
            config.global.default_monitor_id.as_deref(),
            Some("hardware-4k")
        );
        assert_eq!(
            config.profiles[0].target_monitor_id.as_deref(),
            Some("hardware-4k")
        );
        for app in &config.profiles[0].apps {
            assert_eq!(app.target_monitor_id.as_deref(), Some("hardware-4k"));
            assert_eq!(
                app.captured_display.as_ref().map(|display| display.width),
                Some(3840)
            );
        }
    }

    #[test]
    fn trusts_an_explicit_stable_monitor_identity() {
        let mut config = WindowAutoLayoutConfig::default();
        config.global.default_monitor_id = Some("hardware-1440".into());
        config.profiles[0].target_monitor_id = Some("hardware-1440".into());
        config.profiles[0].apps[0].layout = crate::models::LayoutRect {
            x: 1920,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let monitors = vec![
            test_monitor("hardware-1440", r"\\.\DISPLAY2", 2560, 1440, 1380, true),
            test_monitor("hardware-4k", r"\\.\DISPLAY1", 3840, 2160, 2088, false),
        ];

        reconcile_monitor_targets(&mut config, &monitors);

        assert_eq!(
            config.global.default_monitor_id.as_deref(),
            Some("hardware-1440")
        );
        assert_eq!(
            config.profiles[0].target_monitor_id.as_deref(),
            Some("hardware-1440")
        );
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
