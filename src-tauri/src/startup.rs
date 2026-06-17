use std::path::Path;

use winreg::{enums::HKEY_CURRENT_USER, RegKey};

use crate::errors::AppResult;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "WindowAutoLayout";
const STARTUP_ARG: &str = "--startup-restore";

pub fn set_startup_enabled(enabled: bool) -> AppResult<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = hkcu.create_subkey(RUN_KEY)?;

    if enabled {
        let exe = std::env::current_exe()?;
        run.set_value(VALUE_NAME, &startup_command(&exe))?;
    } else {
        let _ = run.delete_value(VALUE_NAME);
    }

    Ok(())
}

pub fn startup_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(expected) = std::env::current_exe().map(|exe| startup_command(&exe)) else {
        return false;
    };

    hkcu.open_subkey(RUN_KEY)
        .and_then(|run: RegKey| run.get_value::<String, _>(VALUE_NAME))
        .map(|value| startup_commands_match(&value, &expected))
        .unwrap_or(false)
}

fn startup_command(exe: &Path) -> String {
    format!("\"{}\" {STARTUP_ARG}", exe.display())
}

fn startup_commands_match(actual: &str, expected: &str) -> bool {
    actual.trim().eq_ignore_ascii_case(expected.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn startup_command_quotes_exe_and_adds_startup_arg() {
        let exe =
            PathBuf::from(r"C:\Users\Cristian\AppData\Local\WindowAutoLayout\WindowAutoLayout.exe");

        assert_eq!(
            startup_command(&exe),
            r#""C:\Users\Cristian\AppData\Local\WindowAutoLayout\WindowAutoLayout.exe" --startup-restore"#
        );
    }

    #[test]
    fn startup_command_match_is_case_insensitive_but_requires_argument() {
        let expected = r#""C:\App\WindowAutoLayout.exe" --startup-restore"#;

        assert!(startup_commands_match(
            r#""c:\app\windowautolayout.exe" --STARTUP-RESTORE"#,
            expected
        ));
        assert!(!startup_commands_match(
            r#""C:\App\WindowAutoLayout.exe""#,
            expected
        ));
    }
}
