mod commands;
mod config;
mod desktop_guard;
mod errors;
mod launcher;
mod layout_lock;
mod logging;
mod models;
mod monitors;
mod performance;
mod processes;
mod profiles;
mod single_instance;
mod startup;
mod state;
mod tray_ui;
mod window_actions;
mod windows_enum;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use crate::{
    logging::append,
    models::{LogSeverity, RestoreStatus, TrayClickAction, WindowAutoLayoutConfig},
    state::AppState,
};

static MAIN_WINDOW_OPENING: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let Some(single_instance) = single_instance::acquire() else {
        return;
    };
    let activation_event_address = single_instance.activation_event_address();

    let app = tauri::Builder::default()
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .setup(move |app| {
            performance::lower_process_priority();

            let config_dir = app.path().app_config_dir()?;
            let mut config = match config::load_or_create(&config_dir) {
                Ok(config) => config,
                Err(error) => {
                    let fallback = WindowAutoLayoutConfig::default();
                    let _ = append(
                        &config_dir,
                        LogSeverity::Warn,
                        None,
                        None,
                        format!("Started with a fresh config after load error: {error}"),
                    );
                    fallback
                }
            };
            if let Ok(connected_monitors) = monitors::list_monitors() {
                let targets_changed =
                    config::reconcile_monitor_targets(&mut config, &connected_monitors);
                let display_metadata_changed =
                    config::hydrate_captured_displays(&mut config, &connected_monitors);
                if targets_changed || display_metadata_changed {
                    if let Err(error) = config::save(&config_dir, &config) {
                        let _ = append(
                            &config_dir,
                            LogSeverity::Warn,
                            None,
                            None,
                            format!("Could not save reconciled monitor settings: {error}"),
                        );
                    }
                }
            }

            app.manage(AppState::new(config_dir.clone(), config));
            let _ = append(
                &config_dir,
                LogSeverity::Info,
                None,
                None,
                "WindowAutoLayout started",
            );
            let (severity, input_status) = match performance::registered_raw_input_device_count() {
                Some(0) => (LogSeverity::Info, "Raw input capture disabled".to_string()),
                Some(count) => (
                    LogSeverity::Warn,
                    format!("Unexpected raw input registration count: {count}"),
                ),
                None => (
                    LogSeverity::Warn,
                    "Could not verify raw input registration state".to_string(),
                ),
            };
            let _ = append(&config_dir, severity, None, None, input_status);
            if let Some(state) = app.try_state::<AppState>() {
                if let Ok(config) = state.config.lock() {
                    if let Err(error) = startup::set_startup_enabled(config.startup.enabled) {
                        let _ = append(
                            &config_dir,
                            LogSeverity::Warn,
                            None,
                            None,
                            format!("Startup registration sync failed: {error}"),
                        );
                    }
                }
            }

            build_tray(app)?;
            if let Some(event_address) = activation_event_address {
                if let Err(error) =
                    single_instance::start_activation_listener(event_address, app.handle().clone())
                {
                    let _ = append(
                        &config_dir,
                        LogSeverity::Warn,
                        None,
                        None,
                        format!("Could not start second-launch listener: {error}"),
                    );
                }
            }
            desktop_guard::start(app.handle().clone());
            maybe_startup_restore(app.handle().clone());
            if !is_startup_restore() {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::clear_logs,
            commands::config_path,
            commands::capture_current_layout,
            commands::get_app_presets,
            commands::get_config,
            commands::list_monitors,
            commands::list_windows,
            commands::layout_lock_enabled,
            commands::log_path,
            commands::open_log_file,
            commands::parse_config_json,
            commands::read_logs,
            commands::restore_profile,
            commands::runtime_status,
            commands::save_all_current_layouts,
            commands::save_config,
            commands::save_window_layout,
            commands::set_startup_enabled,
            commands::set_layout_lock,
            commands::show_main_window,
            commands::startup_enabled,
            commands::validate_current_config
        ])
        .build(tauri::generate_context!())
        .expect("error while building WindowAutoLayout");

    app.run(|_app, event| {
        if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}

fn wire_close_to_tray(app_handle: tauri::AppHandle, window: &WebviewWindow) {
    let window_to_hide = window.clone();

    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            let should_hide = app_handle
                .try_state::<AppState>()
                .and_then(|state| {
                    state
                        .config
                        .lock()
                        .ok()
                        .map(|config| config.tray.minimize_to_tray_on_close)
                })
                .unwrap_or(true);

            if should_hide {
                api.prevent_close();
                let _ = window_to_hide.hide();
                let window_to_destroy = window_to_hide.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = window_to_destroy.destroy();
                });
            } else {
                api.prevent_close();
                app_handle.exit(0);
            }
        }
    });
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let automatic_restore_enabled = layout_lock::enabled(app.handle()).unwrap_or(false);
    let restore = MenuItem::with_id(
        app,
        "restore_default",
        "Restore windows now",
        true,
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, "open", "Open WindowAutoLayout", true, None::<&str>)?;
    let logs = MenuItem::with_id(app, "logs", "Open activity log", true, None::<&str>)?;
    let automatic = CheckMenuItem::with_id(
        app,
        "layout_lock",
        if automatic_restore_enabled {
            "Automatic restore: On"
        } else {
            "Automatic restore: Off"
        },
        true,
        automatic_restore_enabled,
        None::<&str>,
    )?;
    let separator_one = PredefinedMenuItem::separator(app)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &restore,
            &automatic,
            &separator_one,
            &open,
            &logs,
            &separator_two,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id(tray_ui::TRAY_ID)
        .tooltip(if automatic_restore_enabled {
            "WindowAutoLayout - Automatic restore on"
        } else {
            "WindowAutoLayout - Automatic restore off"
        })
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "restore_default" => restore_default_in_background(app.clone(), None, None),
            "layout_lock" => toggle_layout_lock(app),
            "open" => show_main_window(app),
            "logs" => open_logs(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if tray_left_click_restores(tray.app_handle()) {
                    restore_default_in_background(tray.app_handle().clone(), None, None);
                } else {
                    show_main_window(tray.app_handle());
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    if let Err(error) = tray_ui::register(app.handle().clone(), restore, automatic) {
        let state = app.state::<AppState>();
        let _ = append(
            &state.config_dir,
            LogSeverity::Warn,
            None,
            None,
            format!("Could not initialize tray status controls: {error}"),
        );
    }
    Ok(())
}

fn tray_left_click_restores(app: &tauri::AppHandle) -> bool {
    app.try_state::<AppState>()
        .and_then(|state| {
            state
                .config
                .lock()
                .ok()
                .map(|config| config.tray.left_click_action.clone())
        })
        .is_some_and(|action| matches!(action, TrayClickAction::RestoreLayout))
}

fn maybe_startup_restore(app: tauri::AppHandle) {
    let is_startup = is_startup_restore();
    if !is_startup {
        return;
    }

    let state = app.state::<AppState>();
    let config = match state.config.lock() {
        Ok(config) => config.clone(),
        Err(_) => return,
    };
    if config.startup.start_minimized_to_tray {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    }
    if !config.startup.restore_on_launch && !config.enforcement.enabled {
        return;
    }

    let delay = config.startup.delay_seconds;
    tauri::async_runtime::spawn(async move {
        if delay > 0 {
            tokio_sleep(delay).await;
        }
        let state = app.state::<AppState>();
        let config_dir = state.config_dir.clone();
        let config = match state.config.lock() {
            Ok(config) => config.clone(),
            Err(_) => return,
        };
        let _ = tauri::async_runtime::spawn_blocking(move || {
            if let Err(error) =
                profiles::restore_profile_background(&config_dir, &config, None, None)
            {
                let _ = append(
                    &config_dir,
                    LogSeverity::Warn,
                    None,
                    None,
                    format!("Startup restore did not run: {error}"),
                );
            }
        })
        .await;
    });
}

fn is_startup_restore() -> bool {
    std::env::args().any(|arg| arg == "--startup-restore")
}

async fn tokio_sleep(seconds: u64) {
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_secs(seconds))
    })
    .await
    .ok();
}

fn restore_default_in_background(
    app: tauri::AppHandle,
    profile_id: Option<String>,
    launch_missing: Option<bool>,
) {
    if tray_ui::restoring() {
        return;
    }
    let state = app.state::<AppState>();
    let config_dir = state.config_dir.clone();
    let config = match state.config.lock() {
        Ok(config) => config.clone(),
        Err(_) => return,
    };

    tauri::async_runtime::spawn(async move {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            if let Err(error) =
                profiles::restore_profile(&config_dir, &config, profile_id, launch_missing)
            {
                let _ = append(
                    &config_dir,
                    LogSeverity::Warn,
                    None,
                    None,
                    format!("Tray restore did not run: {error}"),
                );
            }
        })
        .await;
    });
}

fn toggle_layout_lock(app: &tauri::AppHandle) {
    if tray_ui::restoring() {
        return;
    }
    match layout_lock::enabled(app) {
        Ok(true) => {
            if let Err(error) = layout_lock::set(app, false, None) {
                log_layout_lock_error(app, error);
            }
        }
        Ok(false) => enable_layout_lock_in_background(app.clone()),
        Err(error) => log_layout_lock_error(app, error),
    }
}

fn enable_layout_lock_in_background(app: tauri::AppHandle) {
    let state = app.state::<AppState>();
    let config_dir = state.config_dir.clone();
    let config = match state.config.lock() {
        Ok(config) => config.clone(),
        Err(_) => return,
    };
    let profile_id = config
        .enforcement
        .profile_id
        .clone()
        .or_else(|| config.startup.default_profile_id.clone());

    tauri::async_runtime::spawn(async move {
        let restore_profile_id = profile_id.clone();
        let restore = tauri::async_runtime::spawn_blocking(move || {
            profiles::restore_profile(&config_dir, &config, restore_profile_id, None)
        })
        .await;
        match restore {
            Ok(Ok(result))
                if !matches!(
                    result.status,
                    RestoreStatus::Paused | RestoreStatus::Failed | RestoreStatus::MonitorMissing
                ) =>
            {
                if let Err(error) = layout_lock::set(&app, true, Some(result.profile_id)) {
                    log_layout_lock_error(&app, error);
                }
            }
            Ok(Err(error)) => log_layout_lock_error(&app, error),
            Err(error) => {
                let state = app.state::<AppState>();
                let _ = append(
                    &state.config_dir,
                    LogSeverity::Error,
                    None,
                    None,
                    format!("Could not enable automatic restore: {error}"),
                );
            }
            _ => {
                let _ = tray_ui::sync(&app);
            }
        }
    });
}

fn log_layout_lock_error(app: &tauri::AppHandle, error: impl std::fmt::Display) {
    let state = app.state::<AppState>();
    let _ = append(
        &state.config_dir,
        LogSeverity::Error,
        None,
        None,
        format!("Could not change automatic restore: {error}"),
    );
}

pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        present_main_window(&window);
        return;
    }
    if MAIN_WINDOW_OPENING.swap(true, Ordering::AcqRel) {
        return;
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = create_main_window(&app);
        MAIN_WINDOW_OPENING.store(false, Ordering::Release);
        if let Err(error) = result {
            let state = app.state::<AppState>();
            let _ = append(
                &state.config_dir,
                LogSeverity::Error,
                None,
                None,
                format!("Could not open WindowAutoLayout: {error}"),
            );
        }
    });
}

fn create_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        present_main_window(&window);
        return Ok(());
    }
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == "main")
        .ok_or(tauri::Error::WindowNotFound)?;
    let window = WebviewWindowBuilder::from_config(app, config)?.build()?;
    wire_close_to_tray(app.clone(), &window);
    present_main_window(&window);
    Ok(())
}

fn present_main_window(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    if let Ok(hwnd) = window.hwnd() {
        let hwnd = windows::Win32::Foundation::HWND(hwnd.0 as usize as *mut std::ffi::c_void);
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
        }
    }
}

fn open_logs(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let path = logging::log_file_path(&state.config_dir);
    if !path.exists() {
        let _ = append(
            &state.config_dir,
            LogSeverity::Info,
            None,
            None,
            "Created log file",
        );
    }
    let _ = std::process::Command::new("explorer").arg(path).spawn();
}
