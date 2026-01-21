use serde_json::{json, Value};
use std::io;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum AppError {
    #[error("API Error: {0}")]
    ApiError(String),

    #[error("Configuration Error: {0}")]
    ConfigError(String),

    #[error("Resource Not Found")]
    NotFound,

    #[error("Internal Error: {0}")]
    Internal(String),

    #[error("IO Error: {0}")]
    Io(#[from] io::Error),

    #[error("Network Error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization Error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML Error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("YAML Error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

// Implement conversion for Box<dyn Error + Send + Sync> to make refactoring easier
impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Internal(s.to_string())
    }
}

impl AppError {
    pub fn to_json_rpc_error(&self) -> Value {
        let (code, message) = match self {
            AppError::NotFound => (-32004, self.to_string()),
            AppError::ApiError(_) | AppError::Network(_) => (-32005, self.to_string()),
            AppError::ConfigError(_) => (-32603, self.to_string()),
            AppError::Internal(_)
            | AppError::Io(_)
            | AppError::Serialization(_)
            | AppError::Toml(_)
            | AppError::Yaml(_) => (-32603, self.to_string()),
        };

        json!({
            "code": code,
            "message": message
        })
    }
}
