use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::Utc;

use crate::{
    errors::AppResult,
    models::{LogEntry, LogSeverity},
};

const LOG_DIR_NAME: &str = "logs";
const LOG_FILE_NAME: &str = "windowautolayout.log";
const MAX_LOG_BYTES: u64 = 1_048_576;

pub fn log_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(LOG_DIR_NAME).join(LOG_FILE_NAME)
}

pub fn append(
    config_dir: &Path,
    severity: LogSeverity,
    profile: Option<&str>,
    app: Option<&str>,
    message: impl AsRef<str>,
) -> AppResult<()> {
    let path = log_file_path(config_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    rotate_if_needed(&path)?;

    let entry = LogEntry {
        timestamp: Utc::now().to_rfc3339(),
        severity,
        profile: profile.map(ToOwned::to_owned),
        app: app.map(ToOwned::to_owned),
        message: message.as_ref().to_string(),
    };
    let line = serde_json::to_string(&entry)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn read(config_dir: &Path, max_lines: usize) -> AppResult<Vec<LogEntry>> {
    let path = log_file_path(config_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(path)?;
    let mut entries = raw
        .lines()
        .rev()
        .take(max_lines)
        .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
        .collect::<Vec<_>>();
    entries.reverse();
    Ok(entries)
}

pub fn clear(config_dir: &Path) -> AppResult<()> {
    let path = log_file_path(config_dir);
    if path.exists() {
        fs::write(path, "")?;
    }
    Ok(())
}

fn rotate_if_needed(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Ok(());
    }

    let metadata = fs::metadata(path)?;
    if metadata.len() < MAX_LOG_BYTES {
        return Ok(());
    }

    let rotated = path.with_extension("log.1");
    if rotated.exists() {
        fs::remove_file(&rotated)?;
    }
    fs::rename(path, rotated)?;
    Ok(())
}
