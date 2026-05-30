mod commands;
mod config;
mod errors;
mod launcher;
mod logging;
mod models;
mod monitors;
mod processes;
mod profiles;
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.handle()
                .plugin(tauri_plugin_global_shortcut::Builder::new().build())?;

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

            wire_close_to_tray(app);
            build_tray(app)?;
            maybe_startup_restore(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::clear_logs,
            commands::config_path,
            commands::get_app_presets,
            commands::get_config,
            commands::list_monitors,
            commands::list_windows,
            commands::lock_layout_temporarily,
            commands::log_path,
            commands::open_log_file,
            commands::read_logs,
            commands::restore_profile,
            commands::save_all_current_layouts,
            commands::save_config,
            commands::save_window_layout,
            commands::set_startup_enabled,
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
    let lock = MenuItem::with_id(
        app,
        "lock_30",
        "Lock layout for 30 seconds",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&restore, &lock, &open, &logs, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("WindowAutoLayout")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "restore_default" => restore_default_in_background(app.clone(), None, Some(true)),
            "lock_30" => lock_default_in_background(app.clone(), None, 30),
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
    let is_startup = std::env::args().any(|arg| arg == "--startup-restore");
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
    if !config.startup.restore_on_launch {
        return;
    }

    let delay = config.startup.delay_seconds;
    tauri::async_runtime::spawn(async move {
        if delay > 0 {
            tokio_sleep(delay).await;
        }
        restore_default_in_background(app, None, Some(true));
    });
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
            profiles::restore_profile(&config_dir, &config, profile_id, launch_missing)
        })
        .await;
    });
}

fn lock_default_in_background(app: tauri::AppHandle, profile_id: Option<String>, seconds: u64) {
    let state = app.state::<AppState>();
    let config_dir = state.config_dir.clone();
    let config = match state.config.lock() {
        Ok(config) => config.clone(),
        Err(_) => return,
    };

    tauri::async_runtime::spawn(async move {
        let interval_ms = config.enforcement.interval_ms;
        let _ = tauri::async_runtime::spawn_blocking(move || {
            profiles::enforce_profile_for(&config_dir, &config, profile_id, seconds, interval_ms)
        })
        .await;
    });
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn open_logs(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let path = logging::log_file_path(&state.config_dir);
    let _ = std::process::Command::new("explorer").arg(path).spawn();
}
