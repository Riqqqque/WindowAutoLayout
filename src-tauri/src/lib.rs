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
mod window_actions;
mod windows_enum;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

use crate::{
    logging::append,
    models::{LogSeverity, WindowAutoLayoutConfig},
    state::AppState,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let Some(single_instance) = single_instance::acquire() else {
        return;
    };
    let activation_event_address = single_instance.activation_event_address();

    tauri::Builder::default()
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .setup(move |app| {
            performance::lower_process_priority();

            let config_dir = app.path().app_config_dir()?;
            let config = match config::load_or_create(&config_dir) {
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

            wire_close_to_tray(app);
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
            commands::save_all_current_layouts,
            commands::save_config,
            commands::save_window_layout,
            commands::set_startup_enabled,
            commands::set_layout_lock,
            commands::show_main_window,
            commands::startup_enabled,
            commands::validate_current_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running WindowAutoLayout");
}

fn wire_close_to_tray(app: &tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let app_handle = app.handle().clone();
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
            } else {
                api.prevent_close();
                app_handle.exit(0);
            }
        }
    });
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let restore = MenuItem::with_id(
        app,
        "restore_default",
        "Restore default profile",
        true,
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, "open", "Open WindowAutoLayout", true, None::<&str>)?;
    let logs = MenuItem::with_id(app, "logs", "Open logs", true, None::<&str>)?;
    let lock = MenuItem::with_id(app, "layout_lock", "Toggle layout lock", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&restore, &lock, &open, &logs, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("WindowAutoLayout")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "restore_default" => restore_default_in_background(app.clone(), None, Some(true)),
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
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
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
                profiles::restore_profile_background(&config_dir, &config, None, Some(true))
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
    if let Err(error) = layout_lock::toggle(app, None) {
        let state = app.state::<AppState>();
        let _ = append(
            &state.config_dir,
            LogSeverity::Error,
            None,
            None,
            format!("Could not toggle layout lock: {error}"),
        );
    }
}

pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        if let Ok(hwnd) = window.hwnd() {
            let hwnd = windows::Win32::Foundation::HWND(hwnd.0 as usize as *mut std::ffi::c_void);
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
            }
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
