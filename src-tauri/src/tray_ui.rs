use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

use tauri::{
    menu::{CheckMenuItem, MenuItem},
    AppHandle, Emitter, Manager, Wry,
};

use crate::{
    errors::{AppError, AppResult},
    models::RuntimeStatus,
    startup,
    state::AppState,
};

pub const TRAY_ID: &str = "windowautolayout-tray";
pub const STATUS_EVENT: &str = "runtime-status-changed";

static RESTORING: AtomicBool = AtomicBool::new(false);
static TRAY_UI: OnceLock<TrayUi> = OnceLock::new();

struct TrayUi {
    app: AppHandle,
    restore_item: MenuItem<Wry>,
    automatic_item: CheckMenuItem<Wry>,
}

pub fn register(
    app: AppHandle,
    restore_item: MenuItem<Wry>,
    automatic_item: CheckMenuItem<Wry>,
) -> AppResult<()> {
    TRAY_UI
        .set(TrayUi {
            app: app.clone(),
            restore_item,
            automatic_item,
        })
        .map_err(|_| AppError::Config("Tray controls were already initialized".to_string()))?;
    let _ = sync(&app);
    Ok(())
}

pub fn runtime_status(app: &AppHandle) -> AppResult<RuntimeStatus> {
    let state = app.state::<AppState>();
    let lock = state
        .layout_lock
        .lock()
        .map_err(|_| AppError::Config("Layout lock state was poisoned".to_string()))?
        .clone();
    let profile_name = state
        .config
        .lock()
        .map_err(|_| AppError::Config("Config lock was poisoned".to_string()))?
        .profiles
        .iter()
        .find(|profile| Some(&profile.id) == lock.profile_id.as_ref())
        .map(|profile| profile.name.clone());

    Ok(RuntimeStatus {
        automatic_restore_enabled: lock.enabled,
        automatic_restore_profile_id: lock.profile_id,
        automatic_restore_profile_name: profile_name,
        restoring: RESTORING.load(Ordering::Acquire),
        startup_registered: startup::startup_enabled(),
    })
}

pub fn sync(app: &AppHandle) -> AppResult<RuntimeStatus> {
    let status = runtime_status(app)?;
    if let Some(ui) = TRAY_UI.get() {
        let automatic_label = if status.automatic_restore_enabled {
            "Automatic restore: On"
        } else {
            "Automatic restore: Off"
        };
        let restore_label = if status.restoring {
            "Restoring windows..."
        } else {
            "Restore windows now"
        };
        let tooltip = if status.restoring {
            "WindowAutoLayout - Restoring windows"
        } else if status.automatic_restore_enabled {
            "WindowAutoLayout - Automatic restore on"
        } else {
            "WindowAutoLayout - Automatic restore off"
        };

        let _ = ui
            .automatic_item
            .set_checked(status.automatic_restore_enabled);
        let _ = ui.automatic_item.set_text(automatic_label);
        let _ = ui.automatic_item.set_enabled(!status.restoring);
        let _ = ui.restore_item.set_text(restore_label);
        let _ = ui.restore_item.set_enabled(!status.restoring);
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(tooltip));
        }
    }
    let _ = app.emit(STATUS_EVENT, &status);
    Ok(status)
}

pub fn set_restoring(restoring: bool) {
    let changed = RESTORING.swap(restoring, Ordering::AcqRel) != restoring;
    if changed {
        if let Some(ui) = TRAY_UI.get() {
            let _ = sync(&ui.app);
        }
    }
}

pub fn restoring() -> bool {
    RESTORING.load(Ordering::Acquire)
}
