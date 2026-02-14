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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(AppError::NotFound.to_string(), "Resource Not Found");
        assert_eq!(AppError::ApiError("test".to_string()).to_string(), "API Error: test");
        assert_eq!(AppError::ConfigError("test".to_string()).to_string(), "Configuration Error: test");
        assert_eq!(AppError::Internal("test".to_string()).to_string(), "Internal Error: test");
    }

    #[test]
    fn test_error_conversions() {
        let e: AppError = "test".into();
        assert!(matches!(e, AppError::Internal(_)));

        let e: AppError = "test".to_string().into();
        assert!(matches!(e, AppError::Internal(_)));

        let io_err = io::Error::new(io::ErrorKind::Other, "test");
        let e: AppError = io_err.into();
        assert!(matches!(e, AppError::Io(_)));

        // Test other conversions using results to avoid complex manual creation
        let res: Result<(), serde_json::Error> = serde_json::from_str("{");
        let e: AppError = res.unwrap_err().into();
        assert!(matches!(e, AppError::Serialization(_)));

        let res: Result<(), toml::de::Error> = toml::from_str("a = ");
        let e: AppError = res.unwrap_err().into();
        assert!(matches!(e, AppError::Toml(_)));

        let res: Result<(), serde_yaml::Error> = serde_yaml::from_str(":");
        let e: AppError = res.unwrap_err().into();
        assert!(matches!(e, AppError::Yaml(_)));
    }

    #[test]
    fn test_to_json_rpc_error() {
        let e = AppError::NotFound;
        let json = e.to_json_rpc_error();
        assert_eq!(json["code"], -32004);

        let e = AppError::ApiError("test".to_string());
        let json = e.to_json_rpc_error();
        assert_eq!(json["code"], -32005);

        let e = AppError::ConfigError("test".to_string());
        let json = e.to_json_rpc_error();
        assert_eq!(json["code"], -32603);

        let e = AppError::Internal("test".to_string());
        let json = e.to_json_rpc_error();
        assert_eq!(json["code"], -32603);
    }
}
