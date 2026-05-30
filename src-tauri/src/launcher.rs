use std::{path::Path, process::Command, thread, time::Duration};

use crate::{
    errors::{AppError, AppResult},
    models::AppConfig,
};

pub fn launch_app_with_path(
    app: &AppConfig,
    executable_path: Option<&str>,
) -> AppResult<Option<u32>> {
    if app.launch_delay_seconds > 0 {
        thread::sleep(Duration::from_secs(app.launch_delay_seconds));
    }

    let executable = executable_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| AppError::InvalidExecutablePath(app.display_name.clone()))?;

    if !Path::new(executable).exists() {
        return Err(AppError::InvalidExecutablePath(executable.to_string()));
    }

    let mut command = Command::new(executable);
    command.args(&app.arguments);

    if let Some(working_directory) = &app.working_directory {
        let working_directory = working_directory.trim();
        if !working_directory.is_empty() && Path::new(working_directory).exists() {
            command.current_dir(working_directory);
        }
    } else if let Some(parent) = Path::new(executable).parent() {
        command.current_dir(parent);
    }

    let child = command.spawn()?;
    Ok(Some(child.id()))
}
