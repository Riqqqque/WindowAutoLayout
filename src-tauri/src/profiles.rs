use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;
use regex::RegexBuilder;

use crate::{
    errors::{AppError, AppResult},
    launcher, logging,
    models::{
        AppConfig, AppRestoreResult, AppRestoreStatus, LayoutRect, LogSeverity, MatchRule,
        MonitorInfo, MonitorMissingBehavior, Profile, RestoreResult, RestoreStatus, TitleMatchMode,
        WindowAutoLayoutConfig, WindowInfo,
    },
    monitors, processes, window_actions, windows_enum,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppPresence {
    NotRunning,
    RunningWithWindow,
    RunningWithoutWindow,
}

#[derive(Debug, Clone)]
struct MatchingProcess {
    pid: u32,
    executable_path: Option<String>,
}

#[derive(Debug, Clone)]
struct MonitorResolution {
    monitor: MonitorInfo,
    is_fallback: bool,
}

struct RestoreTarget<'a> {
    monitor: &'a MonitorInfo,
    layout: &'a LayoutRect,
}

pub fn restore_profile(
    config_dir: &Path,
    config: &WindowAutoLayoutConfig,
    profile_id: Option<String>,
    launch_missing_override: Option<bool>,
) -> AppResult<RestoreResult> {
    restore_profile_inner(
        config_dir,
        config,
        profile_id,
        launch_missing_override,
        true,
        true,
    )
}

pub fn restore_profile_silent(
    config_dir: &Path,
    config: &WindowAutoLayoutConfig,
    profile_id: Option<String>,
    launch_missing_override: Option<bool>,
) -> AppResult<RestoreResult> {
    restore_profile_inner(
        config_dir,
        config,
        profile_id,
        launch_missing_override,
        false,
        false,
    )
}

fn restore_profile_inner(
    config_dir: &Path,
    config: &WindowAutoLayoutConfig,
    profile_id: Option<String>,
    launch_missing_override: Option<bool>,
    log_events: bool,
    activate_visible_windows: bool,
) -> AppResult<RestoreResult> {
    let started_at = Utc::now();
    let profile = resolve_profile(config, profile_id.as_deref())?.clone();
    if log_events {
        logging::append(
            config_dir,
            LogSeverity::Info,
            Some(&profile.name),
            None,
            "Restore started",
        )?;
    }

    let monitors = monitors::list_monitors()?;
    let monitor = resolve_profile_monitor(config, &profile, &monitors);
    let mut results = Vec::new();

    if monitor.is_none() {
        let result = RestoreResult {
            profile_id: profile.id,
            profile_name: profile.name.clone(),
            status: RestoreStatus::MonitorMissing,
            started_at: started_at.to_rfc3339(),
            finished_at: Utc::now().to_rfc3339(),
            monitor: None,
            results,
        };
        if log_events {
            logging::append(
                config_dir,
                LogSeverity::Warn,
                Some(&profile.name),
                None,
                "Restore skipped because the saved monitor is missing",
            )?;
        }
        return Ok(result);
    }

    let monitor = monitor.expect("checked above");
    if monitor.is_fallback && log_events {
        logging::append(
            config_dir,
            LogSeverity::Warn,
            Some(&profile.name),
            None,
            format!("Saved monitor is missing; using {}", monitor.monitor.name),
        )?;
    }
    for app in &profile.apps {
        let app_monitor = resolve_app_monitor(config, &profile, app, &monitors)
            .unwrap_or_else(|| monitor.clone());
        let layout = if app_monitor.is_fallback {
            scaled_layout_for_monitor(&profile, app, &app_monitor.monitor)
        } else {
            app.layout.clone()
        };
        let launch_missing = launch_missing_override.unwrap_or(config.startup.launch_missing_apps);
        let target = RestoreTarget {
            monitor: &app_monitor.monitor,
            layout: &layout,
        };
        results.push(restore_app(
            config_dir,
            &profile,
            app,
            target,
            launch_missing,
            log_events,
            activate_visible_windows,
        ));
    }

    let success_count = results
        .iter()
        .filter(|result| {
            matches!(
                result.status,
                AppRestoreStatus::Success | AppRestoreStatus::Launched
            )
        })
        .count();
    let status = if success_count == results.len() {
        RestoreStatus::Success
    } else if success_count > 0 {
        RestoreStatus::PartialSuccess
    } else {
        RestoreStatus::Failed
    };

    if log_events {
        logging::append(
            config_dir,
            if matches!(status, RestoreStatus::Success) {
                LogSeverity::Info
            } else {
                LogSeverity::Warn
            },
            Some(&profile.name),
            None,
            format!("Restore finished with status {status:?}"),
        )?;
    }

    Ok(RestoreResult {
        profile_id: profile.id,
        profile_name: profile.name,
        status,
        started_at: started_at.to_rfc3339(),
        finished_at: Utc::now().to_rfc3339(),
        monitor: Some(monitor.monitor),
        results,
    })
}

pub fn enforce_profile_for(
    config_dir: &Path,
    config: &WindowAutoLayoutConfig,
    profile_id: Option<String>,
    duration_seconds: u64,
    interval_ms: u64,
) -> AppResult<RestoreResult> {
    let interval = interval_ms.clamp(1000, 5000);
    let until = Instant::now() + Duration::from_secs(duration_seconds.max(1));
    let mut last = restore_profile(config_dir, config, profile_id.clone(), Some(false))?;

    while Instant::now() < until {
        thread::sleep(Duration::from_millis(interval));
        last = restore_profile(config_dir, config, profile_id.clone(), Some(false))?;
    }

    Ok(last)
}

pub fn resolve_profile<'a>(
    config: &'a WindowAutoLayoutConfig,
    profile_id: Option<&str>,
) -> AppResult<&'a Profile> {
    if let Some(id) = profile_id {
        return config
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| AppError::ProfileNotFound(id.to_string()));
    }

    if let Some(id) = &config.startup.default_profile_id {
        if let Some(profile) = config.profiles.iter().find(|profile| &profile.id == id) {
            return Ok(profile);
        }
    }

    config
        .profiles
        .first()
        .ok_or_else(|| AppError::ProfileNotFound("No profiles configured".to_string()))
}

fn restore_app(
    config_dir: &Path,
    profile: &Profile,
    app: &AppConfig,
    target: RestoreTarget<'_>,
    launch_missing: bool,
    log_events: bool,
    activate_visible_windows: bool,
) -> AppRestoreResult {
    let mut matched = find_matching_windows(app, None);
    let mut running_processes = matching_processes(app);
    let mut presence = if matched.is_empty() {
        if !running_processes.is_empty() {
            AppPresence::RunningWithoutWindow
        } else {
            AppPresence::NotRunning
        }
    } else {
        AppPresence::RunningWithWindow
    };
    let mut launched = false;
    let mut activate_after_hidden_restore = false;

    if launch_missing && should_show_matched_windows(app, &matched) {
        let selected = selected_matching_windows(app, &matched);
        if should_restore_through_qt_tray(app, &selected) {
            let mut activated_processes = Vec::new();
            for window in &selected {
                if !activated_processes.contains(&window.process_id)
                    && window_actions::activate_qt_tray_icon_for_process(window.process_id)
                {
                    activated_processes.push(window.process_id);
                }
            }

            if !activated_processes.is_empty() {
                if log_events {
                    let _ = logging::append(
                        config_dir,
                        LogSeverity::Info,
                        Some(&profile.name),
                        Some(&app.display_name),
                        "Asked tray icon to restore hidden running window",
                    );
                }
                let restored = wait_for_visible_windows(
                    app,
                    None,
                    app.detection_timeout_seconds,
                    app.retry_interval_ms,
                );
                if restored
                    .iter()
                    .any(|window| window.is_visible && !window.is_minimized)
                {
                    settle_visible_window(app);
                    matched = restored;
                    running_processes = matching_processes(app);
                    presence = if matched.is_empty() {
                        if running_processes.is_empty() {
                            AppPresence::NotRunning
                        } else {
                            AppPresence::RunningWithoutWindow
                        }
                    } else {
                        AppPresence::RunningWithWindow
                    };
                    activate_after_hidden_restore = true;
                }
            }
        }

        let selected = selected_matching_windows(app, &matched);
        let mut show_error = None;
        if should_show_matched_windows(app, &selected) {
            for window in &selected {
                match windows_enum::parse_handle(&window.handle) {
                    Ok(hwnd) => window_actions::show_window_for_restore(hwnd),
                    Err(error) => {
                        show_error = Some(error.to_string());
                        break;
                    }
                }
            }
        }

        if let Some(error) = show_error {
            if log_events {
                let _ = logging::append(
                    config_dir,
                    LogSeverity::Warn,
                    Some(&profile.name),
                    Some(&app.display_name),
                    format!("Could not restore hidden running window: {error}"),
                );
            }
        } else {
            if !selected.is_empty() && should_show_matched_windows(app, &selected) && log_events {
                let _ = logging::append(
                    config_dir,
                    LogSeverity::Info,
                    Some(&profile.name),
                    Some(&app.display_name),
                    "Restored hidden running window without launching another process",
                );
            }
            let _ = wait_for_visible_windows(
                app,
                None,
                app.detection_timeout_seconds,
                app.retry_interval_ms,
            );
            settle_visible_window(app);
            matched = find_matching_windows(app, None);
            running_processes = matching_processes(app);
            presence = if matched.is_empty() {
                if running_processes.is_empty() {
                    AppPresence::NotRunning
                } else {
                    AppPresence::RunningWithoutWindow
                }
            } else {
                AppPresence::RunningWithWindow
            };
            activate_after_hidden_restore = true;
        }
    }

    let should_wake_running =
        matches!(presence, AppPresence::RunningWithoutWindow) && app.wake_running_process;

    if matched.is_empty() && launch_missing && should_wake_running && is_obs_app(app) {
        let mut activated_processes = Vec::new();
        for process in &running_processes {
            if !activated_processes.contains(&process.pid)
                && window_actions::activate_qt_tray_icon_for_process(process.pid)
            {
                activated_processes.push(process.pid);
            }
        }

        if !activated_processes.is_empty() {
            if log_events {
                let _ = logging::append(
                    config_dir,
                    LogSeverity::Info,
                    Some(&profile.name),
                    Some(&app.display_name),
                    "Asked tray icon to restore running app",
                );
            }
            let _ = wait_for_visible_windows(
                app,
                None,
                app.detection_timeout_seconds,
                app.retry_interval_ms,
            );
            settle_visible_window(app);
            matched = find_matching_windows(app, None);
            running_processes = matching_processes(app);
            presence = if matched.is_empty() {
                if running_processes.is_empty() {
                    AppPresence::NotRunning
                } else {
                    AppPresence::RunningWithoutWindow
                }
            } else {
                AppPresence::RunningWithWindow
            };
            activate_after_hidden_restore = true;
        }
    }

    let should_launch_missing = matches!(presence, AppPresence::NotRunning);
    let should_wake_running =
        matches!(presence, AppPresence::RunningWithoutWindow) && app.wake_running_process;
    let should_launch_running_process = should_wake_running && !is_obs_app(app);

    if matched.is_empty()
        && launch_missing
        && (should_launch_missing || should_launch_running_process)
    {
        let launch_path = if should_launch_running_process {
            running_process_launch_path(app, &running_processes)
        } else {
            app.executable_path.clone()
        };
        let launched_pid = match launcher::launch_app_with_path(app, launch_path.as_deref()) {
            Ok(pid) => {
                launched = true;
                if log_events {
                    let _ = logging::append(
                        config_dir,
                        LogSeverity::Info,
                        Some(&profile.name),
                        Some(&app.display_name),
                        format!(
                            "{}{}",
                            if matches!(presence, AppPresence::RunningWithoutWindow) {
                                "Asked running app to show a window"
                            } else {
                                "Launched app"
                            },
                            pid.map(|pid| format!(" with PID {pid}"))
                                .unwrap_or_default()
                        ),
                    );
                }
                pid
            }
            Err(AppError::InvalidExecutablePath(path)) => {
                let message = format!("Invalid executable path: {path}");
                if log_events {
                    let _ = logging::append(
                        config_dir,
                        LogSeverity::Error,
                        Some(&profile.name),
                        Some(&app.display_name),
                        &message,
                    );
                }
                return app_result(
                    app,
                    AppRestoreStatus::InvalidExecutablePath,
                    message,
                    matched,
                );
            }
            Err(error) => {
                let message = error.to_string();
                if log_events {
                    let _ = logging::append(
                        config_dir,
                        LogSeverity::Error,
                        Some(&profile.name),
                        Some(&app.display_name),
                        &message,
                    );
                }
                return app_result(app, AppRestoreStatus::Failed, message, matched);
            }
        };

        let _ = wait_for_visible_windows(
            app,
            launched_pid,
            app.detection_timeout_seconds,
            app.retry_interval_ms,
        );
        settle_visible_window(app);
        matched = find_matching_windows(app, launched_pid);
        running_processes = matching_processes(app);
        presence = if matched.is_empty() {
            if running_processes.is_empty() {
                AppPresence::NotRunning
            } else {
                AppPresence::RunningWithoutWindow
            }
        } else {
            AppPresence::RunningWithWindow
        };
    }

    if matched.is_empty() {
        let status = if launched {
            AppRestoreStatus::LaunchedWindowNotFound
        } else {
            AppRestoreStatus::ProcessRunningWindowNotFound
        };
        let message = if launched {
            "App launched, but no valid matching window appeared before the timeout".to_string()
        } else if matches!(presence, AppPresence::RunningWithoutWindow) {
            "The process is running, but no pullable top-level window was found. Enable hidden-window pulling or tune title/class matching if this app lives in the tray."
                .to_string()
        } else {
            "No valid matching window was found".to_string()
        };
        if log_events {
            let _ = logging::append(
                config_dir,
                LogSeverity::Warn,
                Some(&profile.name),
                Some(&app.display_name),
                &message,
            );
        }
        return app_result(app, status, message, matched);
    }

    if !app.move_if_running && !launched {
        return app_result(
            app,
            AppRestoreStatus::Skipped,
            "Window is already running and move_if_running is disabled".to_string(),
            matched,
        );
    }

    let selected = selected_matching_windows(app, &matched);
    let mut moved_count = 0usize;
    let mut already_current_count = 0usize;

    for window in &selected {
        let hwnd = match windows_enum::parse_handle(&window.handle) {
            Ok(hwnd) => hwnd,
            Err(error) => {
                return app_result(app, AppRestoreStatus::Failed, error.to_string(), matched);
            }
        };
        let pull_hidden_window = app.pull_hidden_windows && !window.is_visible;
        let restore_minimized_window = app.restore_if_minimized && window.is_minimized;
        let activate_window = pull_hidden_window
            || restore_minimized_window
            || activate_after_hidden_restore
            || activate_visible_windows;
        if window_layout_is_current(
            window,
            target.monitor,
            app,
            target.layout,
            pull_hidden_window,
            restore_minimized_window,
            activate_window,
        ) {
            already_current_count += 1;
            continue;
        }
        if let Err(error) = window_actions::apply_layout(
            hwnd,
            target.monitor,
            target.layout,
            &app.window_state,
            app.force_resize,
            restore_minimized_window,
            pull_hidden_window,
            activate_window,
        ) {
            let message = error.to_string();
            let status = if message.to_ascii_lowercase().contains("access") {
                AppRestoreStatus::PermissionDenied
            } else {
                AppRestoreStatus::MoveFailed
            };
            if log_events {
                let _ = logging::append(
                    config_dir,
                    LogSeverity::Error,
                    Some(&profile.name),
                    Some(&app.display_name),
                    &message,
                );
            }
            return app_result(app, status, message, matched);
        }
        moved_count += 1;
    }

    let status = if launched {
        AppRestoreStatus::Launched
    } else {
        AppRestoreStatus::Success
    };
    let hidden_count = selected.iter().filter(|window| !window.is_visible).count();
    let message = if hidden_count > 0 {
        format!(
            "Pulled {} hidden/tray window(s) forward and applied layout to {} window(s)",
            hidden_count,
            selected.len()
        )
    } else if moved_count == 0 && already_current_count > 0 {
        format!(
            "{} window(s) already matched the saved layout",
            already_current_count
        )
    } else if already_current_count > 0 {
        format!(
            "Applied layout to {} window(s); {} already matched",
            moved_count, already_current_count
        )
    } else {
        format!("Applied layout to {} window(s)", moved_count)
    };
    if log_events {
        let _ = logging::append(
            config_dir,
            LogSeverity::Info,
            Some(&profile.name),
            Some(&app.display_name),
            &message,
        );
    }
    app_result(app, status, message, selected)
}

pub fn window_matches_profile_app(profile: &Profile, window: &WindowInfo) -> bool {
    profile
        .apps
        .iter()
        .any(|app| window_matches_app(window, app, None))
}

pub fn find_matching_windows(app: &AppConfig, launched_pid: Option<u32>) -> Vec<WindowInfo> {
    let mut matched =
        windows_enum::list_windows_with_hidden(app.pull_hidden_windows || launched_pid.is_some())
            .unwrap_or_default()
            .into_iter()
            .filter(|window| window_matches_app(window, app, launched_pid))
            .collect::<Vec<_>>();
    matched.sort_by(|left, right| {
        right
            .is_visible
            .cmp(&left.is_visible)
            .then(left.is_minimized.cmp(&right.is_minimized))
            .then((right.width * right.height).cmp(&(left.width * left.height)))
            .then(left.title.cmp(&right.title))
    });
    matched
}

fn window_layout_is_current(
    window: &WindowInfo,
    monitor: &MonitorInfo,
    app: &AppConfig,
    layout: &LayoutRect,
    pull_hidden_window: bool,
    restore_minimized_window: bool,
    activate_window: bool,
) -> bool {
    if pull_hidden_window
        || restore_minimized_window
        || activate_window
        || app.window_state != crate::models::WindowStatePreference::Normal
    {
        return false;
    }

    let rect = window_actions::absolute_rect(monitor, layout);
    nearly_equal(window.x, rect.left)
        && nearly_equal(window.y, rect.top)
        && (!app.force_resize
            || (nearly_equal(window.width, rect.right - rect.left)
                && nearly_equal(window.height, rect.bottom - rect.top)))
}

fn nearly_equal(left: i32, right: i32) -> bool {
    (left - right).abs() <= 3
}

fn should_show_matched_windows(app: &AppConfig, matched: &[WindowInfo]) -> bool {
    app.wake_running_process
        && !matched.is_empty()
        && matched
            .iter()
            .all(|window| !window.is_visible || window.is_minimized)
}

fn should_restore_through_qt_tray(app: &AppConfig, matched: &[WindowInfo]) -> bool {
    is_obs_app(app) && !matched.is_empty() && matched.iter().all(|window| !window.is_visible)
}

fn selected_matching_windows(app: &AppConfig, matched: &[WindowInfo]) -> Vec<WindowInfo> {
    if app.apply_to_all_matching_windows {
        matched.to_vec()
    } else {
        matched.iter().take(1).cloned().collect()
    }
}

fn wait_for_visible_windows(
    app: &AppConfig,
    launched_pid: Option<u32>,
    timeout_seconds: u64,
    retry_interval_ms: u64,
) -> Vec<WindowInfo> {
    let timeout = Duration::from_secs(timeout_seconds.max(1));
    let interval = Duration::from_millis(retry_interval_ms.max(100));
    let deadline = Instant::now() + timeout;
    let mut last_match = Vec::new();

    loop {
        let matched = find_matching_windows(app, launched_pid);
        if matched
            .iter()
            .any(|window| window.is_visible && !window.is_minimized)
        {
            return matched;
        }
        if !matched.is_empty() {
            last_match = matched;
        }
        if Instant::now() >= deadline {
            return last_match;
        }
        thread::sleep(interval);
    }
}

fn settle_visible_window(app: &AppConfig) {
    let delay = if is_obs_app(app) {
        app.retry_interval_ms.clamp(2500, 3500)
    } else {
        app.retry_interval_ms.clamp(250, 1200)
    };
    thread::sleep(Duration::from_millis(delay));
}

fn is_obs_app(app: &AppConfig) -> bool {
    configured_process_name(app)
        .map(|process_name| names_match("obs64.exe", &process_name))
        .unwrap_or(false)
}

fn window_matches_app(window: &WindowInfo, app: &AppConfig, launched_pid: Option<u32>) -> bool {
    if !window.is_visible && !app.pull_hidden_windows && launched_pid != Some(window.process_id) {
        return false;
    }
    if !app.allow_empty_title && window.title.trim().is_empty() {
        return false;
    }
    if is_obvious_helper_window(window) {
        return false;
    }

    let configured_process_name = configured_process_name(app);
    let process_matches = launched_pid == Some(window.process_id)
        || configured_process_name
            .as_ref()
            .map(|expected| names_match(expected, &window.process_name))
            .unwrap_or(false)
        || app
            .executable_path
            .as_ref()
            .zip(window.executable_path.as_ref())
            .map(|(expected, actual)| expected.eq_ignore_ascii_case(actual))
            .unwrap_or(false);

    if !process_matches {
        return false;
    }

    if app.title_rule.is_none()
        && is_known_process_tool_window(window, configured_process_name.as_deref())
    {
        return false;
    }

    if let Some(class_name) = &app.class_name {
        if !class_name.trim().is_empty()
            && !window
                .class_name
                .to_ascii_lowercase()
                .contains(&class_name.to_ascii_lowercase())
        {
            return false;
        }
    }

    if let Some(rule) = &app.title_rule {
        if !title_matches(&window.title, rule) {
            return false;
        }
    }

    true
}

fn matching_processes(app: &AppConfig) -> Vec<MatchingProcess> {
    let Some(expected) = configured_process_name(app) else {
        return Vec::new();
    };

    processes::list_processes()
        .unwrap_or_default()
        .into_iter()
        .filter(|process| names_match(&expected, &process.name))
        .map(|process| MatchingProcess {
            pid: process.pid,
            executable_path: processes::query_process_path(process.pid),
        })
        .collect()
}

fn running_process_launch_path(app: &AppConfig, processes: &[MatchingProcess]) -> Option<String> {
    app.executable_path.clone().or_else(|| {
        processes
            .iter()
            .find_map(|process| process.executable_path.clone())
    })
}

fn configured_process_name(app: &AppConfig) -> Option<String> {
    app.process_name
        .as_ref()
        .filter(|name| !name.trim().is_empty())
        .cloned()
        .or_else(|| {
            app.executable_path
                .as_ref()
                .and_then(|path| processes::file_name_from_path(path))
        })
}

fn names_match(expected: &str, actual: &str) -> bool {
    let expected = expected.trim().to_ascii_lowercase();
    let actual = actual.trim().to_ascii_lowercase();
    expected == actual
        || expected.trim_end_matches(".exe") == actual
        || expected == actual.trim_end_matches(".exe")
        || is_github_desktop_name_pair(&expected, &actual)
}

fn is_github_desktop_name_pair(expected: &str, actual: &str) -> bool {
    let expected = normalize_process_name(expected);
    let actual = normalize_process_name(actual);
    matches!(expected.as_str(), "github" | "githubdesktop") && actual == "githubdesktop"
}

fn normalize_process_name(name: &str) -> String {
    name.trim()
        .trim_end_matches(".exe")
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn title_matches(title: &str, rule: &MatchRule) -> bool {
    let (title, value) = if rule.case_sensitive {
        (title.to_string(), rule.value.clone())
    } else {
        (title.to_ascii_lowercase(), rule.value.to_ascii_lowercase())
    };

    match rule.mode {
        TitleMatchMode::Contains => title.contains(&value),
        TitleMatchMode::Exact => title == value,
        TitleMatchMode::StartsWith => title.starts_with(&value),
        TitleMatchMode::EndsWith => title.ends_with(&value),
        TitleMatchMode::Regex => RegexBuilder::new(&rule.value)
            .case_insensitive(!rule.case_sensitive)
            .build()
            .map(|regex| regex.is_match(title.as_str()))
            .unwrap_or(false),
    }
}

fn is_obvious_helper_window(window: &WindowInfo) -> bool {
    let class = window.class_name.to_ascii_lowercase();
    let title = window.title.to_ascii_lowercase();
    class == "tooltips_class32"
        || class.contains("toast")
        || class.contains("shadow")
        || title.contains("splash")
}

fn is_known_process_tool_window(
    window: &WindowInfo,
    configured_process_name: Option<&str>,
) -> bool {
    let is_obs = names_match("obs64.exe", &window.process_name)
        || configured_process_name
            .map(|process_name| names_match("obs64.exe", process_name))
            .unwrap_or(false);

    is_obs && !is_obs_main_window(window)
}

fn is_obs_main_window(window: &WindowInfo) -> bool {
    let title = window.title.trim().to_ascii_lowercase();
    title == "obs"
        || title.starts_with("obs ")
        || title.starts_with("obs-")
        || title.contains(" - profile:")
}

fn resolve_app_monitor(
    config: &WindowAutoLayoutConfig,
    profile: &Profile,
    app: &AppConfig,
    monitors: &[MonitorInfo],
) -> Option<MonitorResolution> {
    if let Some(id) = &app.target_monitor_id {
        if let Some(monitor) = monitors.iter().find(|monitor| &monitor.id == id) {
            return Some(MonitorResolution {
                monitor: monitor.clone(),
                is_fallback: false,
            });
        }
        if matches!(
            config.global.monitor_missing_behavior,
            MonitorMissingBehavior::DoNothing | MonitorMissingBehavior::AskNextOpen
        ) {
            return None;
        }
    }

    resolve_profile_monitor(config, profile, monitors)
}

fn resolve_profile_monitor(
    config: &WindowAutoLayoutConfig,
    profile: &Profile,
    monitors: &[MonitorInfo],
) -> Option<MonitorResolution> {
    let requested = profile
        .target_monitor_id
        .as_ref()
        .or(config.global.default_monitor_id.as_ref());

    if let Some(id) = requested {
        if let Some(monitor) = monitors.iter().find(|monitor| &monitor.id == id) {
            return Some(MonitorResolution {
                monitor: monitor.clone(),
                is_fallback: false,
            });
        }

        return match config.global.monitor_missing_behavior {
            MonitorMissingBehavior::DoNothing | MonitorMissingBehavior::AskNextOpen => None,
            MonitorMissingBehavior::UsePrimary => monitors
                .iter()
                .find(|monitor| monitor.is_primary)
                .cloned()
                .map(fallback_monitor),
            MonitorMissingBehavior::NearestMatch => monitors
                .iter()
                .find(|monitor| !monitor.is_primary)
                .cloned()
                .or_else(|| monitors.iter().find(|monitor| monitor.is_primary).cloned())
                .map(fallback_monitor),
        };
    }

    monitors
        .iter()
        .find(|monitor| !monitor.is_primary)
        .cloned()
        .or_else(|| monitors.iter().find(|monitor| monitor.is_primary).cloned())
        .map(|monitor| MonitorResolution {
            monitor,
            is_fallback: false,
        })
}

fn fallback_monitor(monitor: MonitorInfo) -> MonitorResolution {
    MonitorResolution {
        monitor,
        is_fallback: true,
    }
}

fn scaled_layout_for_monitor(
    profile: &Profile,
    app: &AppConfig,
    monitor: &MonitorInfo,
) -> LayoutRect {
    let Some((canvas_width, canvas_height)) = profile_layout_canvas(profile) else {
        return app.layout.clone();
    };
    if canvas_width <= 0 || canvas_height <= 0 || monitor.width <= 0 || monitor.height <= 0 {
        return app.layout.clone();
    }

    let scale_x = monitor.width as f64 / canvas_width as f64;
    let scale_y = monitor.height as f64 / canvas_height as f64;
    let x = scale_i32(app.layout.x, scale_x).clamp(0, monitor.width.saturating_sub(1));
    let y = scale_i32(app.layout.y, scale_y).clamp(0, monitor.height.saturating_sub(1));
    let available_width = (monitor.width - x).max(1);
    let available_height = (monitor.height - y).max(1);
    let width =
        scale_i32(app.layout.width, scale_x).clamp(80.min(available_width), available_width);
    let height =
        scale_i32(app.layout.height, scale_y).clamp(80.min(available_height), available_height);

    LayoutRect {
        x,
        y,
        width,
        height,
    }
}

fn profile_layout_canvas(profile: &Profile) -> Option<(i32, i32)> {
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
    Some((right.max(1), bottom.max(1)))
}

fn scale_i32(value: i32, scale: f64) -> i32 {
    ((value as f64) * scale).round() as i32
}

fn app_result(
    app: &AppConfig,
    status: AppRestoreStatus,
    message: String,
    matched_windows: Vec<WindowInfo>,
) -> AppRestoreResult {
    AppRestoreResult {
        app_id: app.id.clone(),
        display_name: app.display_name.clone(),
        status,
        message,
        matched_windows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LayoutRect, WindowStatePreference};

    #[test]
    fn process_names_match_with_or_without_exe() {
        assert!(names_match("Discord.exe", "Discord.exe"));
        assert!(names_match("Discord", "Discord.exe"));
        assert!(names_match("Discord.exe", "Discord"));
        assert!(names_match("GitHub.exe", "GitHubDesktop.exe"));
    }

    #[test]
    fn title_rules_cover_common_modes() {
        let rule = MatchRule {
            mode: TitleMatchMode::Contains,
            value: "studio".into(),
            case_sensitive: false,
        };
        assert!(title_matches("OBS Studio", &rule));
    }

    #[test]
    fn rejects_empty_titles_by_default() {
        let app = AppConfig {
            id: "app".into(),
            display_name: "App".into(),
            executable_path: None,
            arguments: vec![],
            working_directory: None,
            process_name: Some("app.exe".into()),
            title_rule: None,
            class_name: None,
            target_monitor_id: None,
            layout: LayoutRect::default(),
            window_state: WindowStatePreference::Normal,
            launch_delay_seconds: 0,
            detection_timeout_seconds: 1,
            retry_interval_ms: 100,
            launch_if_missing: false,
            move_if_running: true,
            force_resize: true,
            apply_to_all_matching_windows: false,
            restore_if_minimized: true,
            pull_hidden_windows: true,
            wake_running_process: true,
            allow_empty_title: false,
            notes: None,
        };
        let window = WindowInfo {
            handle: "0x1".into(),
            title: "".into(),
            class_name: "AppWindow".into(),
            process_id: 1,
            process_name: "app.exe".into(),
            executable_path: None,
            monitor_id: None,
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            is_visible: true,
            is_minimized: false,
        };

        assert!(!window_matches_app(&window, &app, None));
    }

    #[test]
    fn accepts_hidden_window_when_pull_is_enabled() {
        let mut app =
            AppConfig::new_preset("obs", "OBS Studio", "obs64.exe", LayoutRect::default());
        app.pull_hidden_windows = true;
        let window = WindowInfo {
            handle: "0x1".into(),
            title: "OBS Studio".into(),
            class_name: "QtWindow".into(),
            process_id: 10,
            process_name: "obs64.exe".into(),
            executable_path: None,
            monitor_id: None,
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            is_visible: false,
            is_minimized: false,
        };

        assert!(window_matches_app(&window, &app, None));
        assert!(should_show_matched_windows(&app, &[window]));
    }

    #[test]
    fn does_not_show_when_a_matching_window_is_visible() {
        let app = AppConfig::new_preset("obs", "OBS Studio", "obs64.exe", LayoutRect::default());
        let window = WindowInfo {
            handle: "0x1".into(),
            title: "OBS Studio".into(),
            class_name: "QtWindow".into(),
            process_id: 10,
            process_name: "obs64.exe".into(),
            executable_path: None,
            monitor_id: None,
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            is_visible: true,
            is_minimized: false,
        };

        assert!(!should_show_matched_windows(&app, &[window]));
    }

    #[test]
    fn uses_obs_tray_restore_only_for_hidden_windows() {
        let app = AppConfig::new_preset("obs", "OBS Studio", "obs64.exe", LayoutRect::default());
        let mut window = WindowInfo {
            handle: "0x1".into(),
            title: "OBS Studio".into(),
            class_name: "QtWindow".into(),
            process_id: 10,
            process_name: "obs64.exe".into(),
            executable_path: None,
            monitor_id: None,
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            is_visible: false,
            is_minimized: false,
        };

        assert!(should_restore_through_qt_tray(&app, &[window.clone()]));

        window.is_visible = true;
        window.is_minimized = true;
        assert!(!should_restore_through_qt_tray(&app, &[window]));
    }

    #[test]
    fn rejects_hidden_window_when_pull_is_disabled() {
        let mut app =
            AppConfig::new_preset("obs", "OBS Studio", "obs64.exe", LayoutRect::default());
        app.pull_hidden_windows = false;
        let window = WindowInfo {
            handle: "0x1".into(),
            title: "OBS Studio".into(),
            class_name: "QtWindow".into(),
            process_id: 10,
            process_name: "obs64.exe".into(),
            executable_path: None,
            monitor_id: None,
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            is_visible: false,
            is_minimized: false,
        };

        assert!(!window_matches_app(&window, &app, None));
    }

    #[test]
    fn accepts_obs_main_window_without_explicit_title_rule() {
        let mut app =
            AppConfig::new_preset("obs", "OBS Studio", "obs64.exe", LayoutRect::default());
        app.title_rule = None;
        let window = WindowInfo {
            handle: "0x1".into(),
            title: "OBS 32.1.2 - Profile: Untitled - Scenes: Untitled".into(),
            class_name: "Qt672QWindowIcon".into(),
            process_id: 10,
            process_name: "obs64.exe".into(),
            executable_path: None,
            monitor_id: None,
            x: 0,
            y: 0,
            width: 1200,
            height: 800,
            is_visible: true,
            is_minimized: false,
        };

        assert!(window_matches_app(&window, &app, None));
    }

    #[test]
    fn rejects_obs_dock_window_without_explicit_title_rule() {
        let mut app =
            AppConfig::new_preset("obs", "OBS Studio", "obs64.exe", LayoutRect::default());
        app.title_rule = None;
        let window = WindowInfo {
            handle: "0x1".into(),
            title: "Stats".into(),
            class_name: "Qt672QWindowIcon".into(),
            process_id: 10,
            process_name: "obs64.exe".into(),
            executable_path: None,
            monitor_id: None,
            x: 0,
            y: 0,
            width: 420,
            height: 500,
            is_visible: true,
            is_minimized: false,
        };

        assert!(!window_matches_app(&window, &app, None));
    }

    #[test]
    fn allows_obs_dock_window_with_explicit_title_rule() {
        let mut app =
            AppConfig::new_preset("obs", "OBS Studio", "obs64.exe", LayoutRect::default());
        app.title_rule = Some(MatchRule {
            mode: TitleMatchMode::Exact,
            value: "Stats".into(),
            case_sensitive: false,
        });
        let window = WindowInfo {
            handle: "0x1".into(),
            title: "Stats".into(),
            class_name: "Qt672QWindowIcon".into(),
            process_id: 10,
            process_name: "obs64.exe".into(),
            executable_path: None,
            monitor_id: None,
            x: 0,
            y: 0,
            width: 420,
            height: 500,
            is_visible: true,
            is_minimized: false,
        };

        assert!(window_matches_app(&window, &app, None));
    }

    #[test]
    fn current_windows_skip_repeated_layout_work() {
        let monitor = MonitorInfo {
            id: "display".into(),
            name: "Display".into(),
            device_name: "DISPLAY1".into(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            work_x: 0,
            work_y: 0,
            work_width: 1920,
            work_height: 1040,
            scale_factor: 1.0,
            is_primary: true,
        };
        let app = AppConfig::new_preset(
            "discord",
            "Discord",
            "Discord.exe",
            LayoutRect {
                x: 1280,
                y: 0,
                width: 640,
                height: 720,
            },
        );
        let window = WindowInfo {
            handle: "0x1".into(),
            title: "Discord".into(),
            class_name: "Chrome_WidgetWin_1".into(),
            process_id: 10,
            process_name: "Discord.exe".into(),
            executable_path: None,
            monitor_id: Some("display".into()),
            x: 1281,
            y: 1,
            width: 640,
            height: 719,
            is_visible: true,
            is_minimized: false,
        };

        assert!(window_layout_is_current(
            &window,
            &monitor,
            &app,
            &app.layout,
            false,
            false,
            false
        ));
        assert!(!window_layout_is_current(
            &window,
            &monitor,
            &app,
            &app.layout,
            false,
            false,
            true
        ));
    }

    #[test]
    fn missing_profile_monitor_uses_secondary_nearest_match() {
        let mut config = WindowAutoLayoutConfig::default();
        config.global.monitor_missing_behavior = MonitorMissingBehavior::NearestMatch;
        config.profiles[0].target_monitor_id = Some("missing-display".into());
        let monitors = vec![
            test_monitor("primary", true, 0, 0, 2048, 1152),
            test_monitor("secondary", false, -3072, -261, 2560, 1440),
        ];

        let resolved = resolve_profile_monitor(&config, &config.profiles[0], &monitors)
            .expect("fallback monitor");

        assert_eq!(resolved.monitor.id, "secondary");
        assert!(resolved.is_fallback);
    }

    #[test]
    fn fallback_layout_scales_profile_canvas_to_target_monitor() {
        let monitor = test_monitor("secondary", false, -3072, -261, 2560, 1440);
        let mut profile = Profile {
            id: "profile".into(),
            name: "Streaming".into(),
            description: None,
            target_monitor_id: Some("missing".into()),
            apps: vec![
                AppConfig::new_preset(
                    "obs",
                    "OBS Studio",
                    "obs64.exe",
                    LayoutRect {
                        x: 0,
                        y: 0,
                        width: 1920,
                        height: 2160,
                    },
                ),
                AppConfig::new_preset(
                    "discord",
                    "Discord",
                    "Discord.exe",
                    LayoutRect {
                        x: 1920,
                        y: 0,
                        width: 1920,
                        height: 1080,
                    },
                ),
                AppConfig::new_preset(
                    "codex",
                    "Codex",
                    "Codex.exe",
                    LayoutRect {
                        x: 1920,
                        y: 1080,
                        width: 1920,
                        height: 1080,
                    },
                ),
            ],
            startup_restore: true,
            enforce_after_restore: true,
        };
        let codex = profile.apps.pop().expect("codex app");

        let layout = scaled_layout_for_monitor(&profile, &codex, &monitor);

        assert_eq!(
            layout,
            LayoutRect {
                x: 1280,
                y: 720,
                width: 1280,
                height: 720
            }
        );
    }

    fn test_monitor(
        id: &str,
        is_primary: bool,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> MonitorInfo {
        MonitorInfo {
            id: id.into(),
            name: id.into(),
            device_name: id.into(),
            x,
            y,
            width,
            height,
            work_x: x,
            work_y: y,
            work_width: width,
            work_height: height,
            scale_factor: 1.0,
            is_primary,
        }
    }
}
