use serde::{Deserialize, Serialize};

pub const CONFIG_SCHEMA_VERSION: u32 = 2;
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MonitorMissingBehavior {
    DoNothing,
    UsePrimary,
    NearestMatch,
    AskNextOpen,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WindowStatePreference {
    Normal,
    Maximized,
    Minimized,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TitleMatchMode {
    Contains,
    Exact,
    StartsWith,
    EndsWith,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RestoreStatus {
    Success,
    PartialSuccess,
    Failed,
    MonitorMissing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRestoreStatus {
    Success,
    Skipped,
    Launched,
    LaunchedWindowNotFound,
    ProcessRunningWindowNotFound,
    MonitorMissing,
    InvalidExecutablePath,
    PermissionDenied,
    MoveFailed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LogSeverity {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Default for LayoutRect {
    fn default() -> Self {
        Self {
            x: 80,
            y: 80,
            width: 1280,
            height: 720,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MatchRule {
    pub mode: TitleMatchMode,
    pub value: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub id: String,
    pub display_name: String,
    pub executable_path: Option<String>,
    pub arguments: Vec<String>,
    pub working_directory: Option<String>,
    pub process_name: Option<String>,
    pub title_rule: Option<MatchRule>,
    pub class_name: Option<String>,
    pub target_monitor_id: Option<String>,
    pub layout: LayoutRect,
    pub window_state: WindowStatePreference,
    pub launch_delay_seconds: u64,
    pub detection_timeout_seconds: u64,
    pub retry_interval_ms: u64,
    pub launch_if_missing: bool,
    pub move_if_running: bool,
    pub force_resize: bool,
    pub apply_to_all_matching_windows: bool,
    pub restore_if_minimized: bool,
    #[serde(default = "default_true")]
    pub pull_hidden_windows: bool,
    #[serde(default = "default_true")]
    pub wake_running_process: bool,
    pub allow_empty_title: bool,
    pub notes: Option<String>,
}

impl AppConfig {
    pub fn new_preset(
        id: &str,
        display_name: &str,
        process_name: &str,
        layout: LayoutRect,
    ) -> Self {
        Self {
            id: id.to_string(),
            display_name: display_name.to_string(),
            executable_path: None,
            arguments: Vec::new(),
            working_directory: None,
            process_name: Some(process_name.to_string()),
            title_rule: None,
            class_name: None,
            target_monitor_id: None,
            layout,
            window_state: WindowStatePreference::Normal,
            launch_delay_seconds: 0,
            detection_timeout_seconds: 25,
            retry_interval_ms: 700,
            launch_if_missing: true,
            move_if_running: true,
            force_resize: true,
            apply_to_all_matching_windows: false,
            restore_if_minimized: true,
            pull_hidden_windows: true,
            wake_running_process: true,
            allow_empty_title: false,
            notes: None,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub target_monitor_id: Option<String>,
    pub apps: Vec<AppConfig>,
    pub startup_restore: bool,
    pub enforce_after_restore: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    pub default_monitor_id: Option<String>,
    pub monitor_missing_behavior: MonitorMissingBehavior,
    pub warn_when_monitor_missing: bool,
    pub advanced_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupSettings {
    pub enabled: bool,
    pub start_minimized_to_tray: bool,
    pub default_profile_id: Option<String>,
    pub delay_seconds: u64,
    pub restore_on_launch: bool,
    pub launch_missing_apps: bool,
    pub enforce_after_startup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraySettings {
    pub minimize_to_tray_on_close: bool,
    pub show_restore_status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    pub enabled: bool,
    pub accelerator: String,
    pub restore_without_opening: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnforcementSettings {
    pub enabled: bool,
    #[serde(default)]
    pub profile_id: Option<String>,
    pub duration_seconds: u64,
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowAutoLayoutConfig {
    pub schema_version: u32,
    pub app_version: String,
    pub global: GlobalSettings,
    pub startup: StartupSettings,
    pub tray: TraySettings,
    pub hotkey: HotkeySettings,
    pub enforcement: EnforcementSettings,
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub device_name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub work_x: i32,
    pub work_y: i32,
    pub work_width: i32,
    pub work_height: i32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub handle: String,
    pub title: String,
    pub class_name: String,
    pub process_id: u32,
    pub process_name: String,
    pub executable_path: Option<String>,
    pub monitor_id: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub is_visible: bool,
    pub is_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRestoreResult {
    pub app_id: String,
    pub display_name: String,
    pub status: AppRestoreStatus,
    pub message: String,
    pub matched_windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub profile_id: String,
    pub profile_name: String,
    pub status: RestoreStatus,
    pub started_at: String,
    pub finished_at: String,
    pub monitor: Option<MonitorInfo>,
    pub results: Vec<AppRestoreResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapturedWindowSummary {
    pub app_id: String,
    pub display_name: String,
    pub process_name: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureLayoutResult {
    pub config: WindowAutoLayoutConfig,
    pub profile_id: String,
    pub monitor: MonitorInfo,
    pub captured_count: usize,
    pub skipped_count: usize,
    pub captured_windows: Vec<CapturedWindowSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub severity: LogSeverity,
    pub profile: Option<String>,
    pub app: Option<String>,
    pub message: String,
}

pub fn preset_apps() -> Vec<AppConfig> {
    let mut obs = AppConfig::new_preset(
        "preset-obs",
        "OBS Studio",
        "obs64.exe",
        LayoutRect {
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
        },
    );
    obs.title_rule = Some(MatchRule {
        mode: TitleMatchMode::StartsWith,
        value: "OBS".to_string(),
        case_sensitive: false,
    });

    let mut github_desktop = AppConfig::new_preset(
        "preset-github-desktop",
        "GitHub Desktop",
        "GitHubDesktop.exe",
        LayoutRect::default(),
    );
    github_desktop.title_rule = Some(MatchRule {
        mode: TitleMatchMode::Contains,
        value: "GitHub Desktop".to_string(),
        case_sensitive: false,
    });
    github_desktop.launch_if_missing = true;
    github_desktop.detection_timeout_seconds = 45;

    vec![
        obs,
        AppConfig::new_preset(
            "preset-discord",
            "Discord",
            "Discord.exe",
            LayoutRect {
                x: 1280,
                y: 0,
                width: 640,
                height: 720,
            },
        ),
        AppConfig::new_preset("preset-steam", "Steam", "steam.exe", LayoutRect::default()),
        AppConfig::new_preset(
            "preset-spotify",
            "Spotify",
            "Spotify.exe",
            LayoutRect::default(),
        ),
        AppConfig::new_preset(
            "preset-chrome",
            "Chrome",
            "chrome.exe",
            LayoutRect::default(),
        ),
        AppConfig::new_preset(
            "preset-firefox",
            "Firefox",
            "firefox.exe",
            LayoutRect::default(),
        ),
        AppConfig::new_preset(
            "preset-edge",
            "Microsoft Edge",
            "msedge.exe",
            LayoutRect::default(),
        ),
        github_desktop,
    ]
}

impl Default for WindowAutoLayoutConfig {
    fn default() -> Self {
        let streaming_apps = vec![preset_apps()[0].clone(), preset_apps()[1].clone()];
        let default_profile_id = "profile-streaming".to_string();

        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            app_version: APP_VERSION.to_string(),
            global: GlobalSettings {
                default_monitor_id: None,
                monitor_missing_behavior: MonitorMissingBehavior::DoNothing,
                warn_when_monitor_missing: true,
                advanced_mode: false,
            },
            startup: StartupSettings {
                enabled: false,
                start_minimized_to_tray: true,
                default_profile_id: Some(default_profile_id.clone()),
                delay_seconds: 8,
                restore_on_launch: true,
                launch_missing_apps: true,
                enforce_after_startup: true,
            },
            tray: TraySettings {
                minimize_to_tray_on_close: true,
                show_restore_status: true,
            },
            hotkey: HotkeySettings {
                enabled: true,
                accelerator: "Ctrl+Alt+L".to_string(),
                restore_without_opening: true,
            },
            enforcement: EnforcementSettings {
                enabled: false,
                profile_id: Some(default_profile_id.clone()),
                duration_seconds: 30,
                interval_ms: 250,
            },
            profiles: vec![Profile {
                id: default_profile_id,
                name: "Streaming".to_string(),
                description: Some("OBS and Discord arranged on a selected monitor.".to_string()),
                target_monitor_id: None,
                apps: streaming_apps,
                startup_restore: true,
                enforce_after_restore: true,
            }],
        }
    }
}
