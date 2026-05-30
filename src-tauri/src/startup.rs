use winreg::{enums::HKEY_CURRENT_USER, RegKey};

use crate::errors::AppResult;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "WindowAutoLayout";

pub fn set_startup_enabled(enabled: bool) -> AppResult<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = hkcu.create_subkey(RUN_KEY)?;

    if enabled {
        let exe = std::env::current_exe()?;
        let command = format!("\"{}\" --startup-restore", exe.display());
        run.set_value(VALUE_NAME, &command)?;
    } else {
        let _ = run.delete_value(VALUE_NAME);
    }

    Ok(())
}

pub fn startup_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(RUN_KEY)
        .and_then(|run: RegKey| run.get_value::<String, _>(VALUE_NAME))
        .map(|value| value.to_ascii_lowercase().contains("windowautolayout"))
        .unwrap_or(false)
}
