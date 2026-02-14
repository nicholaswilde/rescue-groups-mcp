use crate::cli::Cli;
use crate::error::AppError;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use moka::future::Cache;
use nonzero_ext::nonzero;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Deserialize, Debug, Clone)]
struct ConfigFile {
    api_key: Option<String>,
    postal_code: Option<String>,
    species: Option<String>,
    miles: Option<u32>,
    timeout_seconds: Option<u64>,
    lazy: Option<bool>,
    rate_limit_requests: Option<u32>,
    rate_limit_window: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub api_key: String,
    pub base_url: String,
    pub default_postal_code: String,
    pub default_miles: u32,
    pub default_species: String,
    pub timeout: std::time::Duration,
    pub lazy: bool,
    pub cache: Arc<Cache<String, Value>>,
    pub limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

pub fn merge_configuration(cli: &Cli) -> Result<Settings, AppError> {
    let config_path = Path::new(&cli.config);

    let file_config: Option<ConfigFile> = if config_path.exists() {
        let content = fs::read_to_string(config_path).map_err(AppError::Io)?;
        let ext = config_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        match ext {
            "toml" => Some(toml::from_str(&content).map_err(AppError::Toml)?),
            "json" => Some(serde_json::from_str(&content).map_err(AppError::Serialization)?),
            "yaml" | "yml" => Some(serde_yaml::from_str(&content).map_err(AppError::Yaml)?),
            _ => None,
        }
    } else {
        None
    };

    let api_key = cli
        .api_key
        .clone()
        .or(file_config.as_ref().and_then(|c| c.api_key.clone()))
        .ok_or_else(|| {
            AppError::ConfigError(
                "API Key is missing! Set RESCUE_GROUPS_API_KEY or use config.toml".to_string(),
            )
        })?;

    let cache = Cache::builder()
        .max_capacity(100)
        .time_to_live(std::time::Duration::from_secs(15 * 60)) // 15 minutes
        .build();

    // Default: 60 requests per 60 seconds (1 req/sec)
    let max_requests = std::num::NonZeroU32::new(
        file_config
            .as_ref()
            .and_then(|c| c.rate_limit_requests)
            .unwrap_or(60),
    )
    .unwrap_or(nonzero!(60u32));

    let window = std::time::Duration::from_secs(
        file_config
            .as_ref()
            .and_then(|c| c.rate_limit_window)
            .unwrap_or(60),
    );

    let quota = Quota::with_period(window)
        .unwrap()
        .allow_burst(max_requests);
    let limiter = Arc::new(RateLimiter::direct(quota));

    let base_url = std::env::var("RESCUE_GROUPS_BASE_URL")
        .unwrap_or_else(|_| "https://api.rescuegroups.org/v5".to_string());

    Ok(Settings {
        api_key,
        base_url,
        default_postal_code: file_config
            .as_ref()
            .and_then(|c| c.postal_code.clone())
            .unwrap_or_else(|| "90210".to_string()),
        default_miles: file_config.as_ref().and_then(|c| c.miles).unwrap_or(50),
        default_species: file_config
            .as_ref()
            .and_then(|c| c.species.clone())
            .unwrap_or_else(|| "dogs".to_string()),
        timeout: std::time::Duration::from_secs(
            file_config
                .as_ref()
                .and_then(|c| c.timeout_seconds)
                .unwrap_or(30),
        ),
        lazy: file_config.as_ref().and_then(|c| c.lazy).unwrap_or(true),
        cache: Arc::new(cache),
        limiter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;

    #[test]
    fn test_merge_configuration_cli_key() {
        let cli = Cli {
            api_key: Some("cli_key".to_string()),
            config: "non_existent.toml".to_string(),
            json: false,
            command: None,
        };

        let settings = merge_configuration(&cli).unwrap();
        assert_eq!(settings.api_key, "cli_key");
        assert_eq!(settings.default_postal_code, "90210"); // Default
    }

    #[test]
    fn test_merge_configuration_missing_key() {
        let cli = Cli {
            api_key: None,
            config: "non_existent.toml".to_string(),
            json: false,
            command: None,
        };

        let result = merge_configuration(&cli);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ConfigError(msg) => assert!(msg.contains("API Key is missing")),
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_merge_configuration_toml() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("config.toml");
        fs::write(
            &config_path,
            "api_key = \"toml_key\"\npostal_code = \"12345\"",
        )
        .unwrap();

        let cli = Cli {
            api_key: None,
            config: config_path.to_str().unwrap().to_string(),
            json: false,
            command: None,
        };

        let settings = merge_configuration(&cli).unwrap();
        assert_eq!(settings.api_key, "toml_key");
        assert_eq!(settings.default_postal_code, "12345");
        fs::remove_file(config_path).unwrap();
    }

    #[test]
    fn test_merge_configuration_json() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("config.json");
        fs::write(&config_path, "{\"api_key\": \"json_key\", \"miles\": 10}").unwrap();

        let cli = Cli {
            api_key: None,
            config: config_path.to_str().unwrap().to_string(),
            json: false,
            command: None,
        };

        let settings = merge_configuration(&cli).unwrap();
        assert_eq!(settings.api_key, "json_key");
        assert_eq!(settings.default_miles, 10);
        fs::remove_file(config_path).unwrap();
    }

    #[test]
    fn test_merge_configuration_yaml() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("config.yaml");
        fs::write(&config_path, "api_key: yaml_key\nspecies: cats").unwrap();

        let cli = Cli {
            api_key: None,
            config: config_path.to_str().unwrap().to_string(),
            json: false,
            command: None,
        };

        let settings = merge_configuration(&cli).unwrap();
        assert_eq!(settings.api_key, "yaml_key");
        assert_eq!(settings.default_species, "cats");
        fs::remove_file(config_path).unwrap();
    }

    #[test]
    fn test_merge_configuration_unsupported_ext() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("config.txt");
        fs::write(&config_path, "api_key = \"txt_key\"").unwrap();

        let cli = Cli {
            api_key: Some("fallback".to_string()),
            config: config_path.to_str().unwrap().to_string(),
            json: false,
            command: None,
        };

        let settings = merge_configuration(&cli).unwrap();
        assert_eq!(settings.api_key, "fallback");
        fs::remove_file(config_path).unwrap();
    }

    #[test]
    fn test_merge_configuration_invalid_toml() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("invalid.toml");
        fs::write(&config_path, "api_key = ").unwrap();

        let cli = Cli {
            api_key: None,
            config: config_path.to_str().unwrap().to_string(),
            json: false,
            command: None,
        };

        let result = merge_configuration(&cli);
        assert!(result.is_err());
        fs::remove_file(config_path).unwrap();
    }
}
