use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Windows API error: {0}")]
    Windows(String),
    #[error("Config error: {0}")]
    Config(String),
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),
    #[error("App not found: {0}")]
    AppNotFound(String),
    #[error("Monitor not found")]
    MonitorNotFound,
    #[error("Invalid window handle: {0}")]
    InvalidWindowHandle(String),
    #[error("Invalid executable path: {0}")]
    InvalidExecutablePath(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
