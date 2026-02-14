mod cli;
mod client;
mod commands;
mod config;
mod error;
mod fmt;
mod mcp;
mod server;

use clap::Parser;
use cli::{Cli, Commands};
use commands::handle_command;
use config::merge_configuration;
use server::{run_http_server, run_stdio_server};
use std::error::Error;
use std::io;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // 0. Initialize Logging
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "rescue_groups_mcp=info".into());

    if std::env::var("RUST_LOG_FORMAT").unwrap_or_default() == "json" {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(io::stderr),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().with_writer(io::stderr))
            .init();
    }

    // 1. Load Settings
    let cli = Cli::parse();
    // Clone command to use after merge_configuration (which consumes cli)
    let command = cli.command.clone();
    let settings = merge_configuration(&cli)?;

    match command {
        Some(Commands::Server) | None => {
            run_stdio_server(settings).await?;
        }
        Some(Commands::Http(args)) => {
            run_http_server(args, settings).await?;
        }
        Some(cmd) => {
            handle_command(cmd, &settings, cli.json).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;

    #[test]
    fn test_merge_configuration_integration() {
        let cli = Cli {
            api_key: Some("test".to_string()),
            config: "non_existent.toml".to_string(),
            json: false,
            command: None,
        };
        let res = merge_configuration(&cli);
        assert!(res.is_ok());
    }
}
