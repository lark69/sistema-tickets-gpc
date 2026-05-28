use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum AppError {
    Database(String),
    Io(String),
    InvalidInput(String),
    Printer(String),
    System(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
    pub kind: String,
}

pub type AppResult<T> = Result<T, AppError>;
pub type CommandResult<T> = Result<T, CommandError>;

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Database(message)
            | AppError::Io(message)
            | AppError::InvalidInput(message)
            | AppError::Printer(message)
            | AppError::System(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        AppError::Database(value.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Io(value.to_string())
    }
}

impl From<AppError> for CommandError {
    fn from(value: AppError) -> Self {
        let kind = match &value {
            AppError::Database(_) => "database",
            AppError::Io(_) => "io",
            AppError::InvalidInput(_) => "validation",
            AppError::Printer(_) => "printer",
            AppError::System(_) => "system",
        };

        CommandError {
            message: value.to_string(),
            kind: kind.to_string(),
        }
    }
}
