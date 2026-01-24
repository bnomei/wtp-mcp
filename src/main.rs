use clap::Parser;
use rmcp::ServiceExt;
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use wtp_mcp_rs::WtpServer;
use wtp_mcp_rs::config::{CliArgs, Config};

#[derive(Parser, Debug)]
#[command(name = "wtp-mcp", version, about = "WTP MCP Server")]
struct Cli {
    /// Repository root directory (default: current directory)
    #[arg(long)]
    repo_root: Option<PathBuf>,

    /// Path to wtp binary (overrides auto-detection)
    #[arg(long)]
    wtp_path: Option<PathBuf>,

    /// Path to TOML configuration file
    #[arg(long, short)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(std::io::stderr))
            .with(EnvFilter::from_default_env())
            .init();
    }

    let cli = Cli::parse();

    // Load config from file if specified
    let mut config = Config::load(cli.config.as_deref()).unwrap_or_default();

    // Merge CLI overrides
    config.merge_cli(&CliArgs {
        repo_root: cli.repo_root.clone(),
        wtp_path: cli.wtp_path.clone(),
    });

    // Create the server with resolved configuration
    let server = WtpServer::new(config);

    // Start the MCP server on stdio transport
    let service = server.serve(rmcp::transport::stdio()).await?;

    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::select! {
            _ = service.waiting() => {},
            _ = tokio::signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        tokio::select! {
            _ = service.waiting() => {},
            _ = tokio::signal::ctrl_c() => {},
        }
    }

    Ok(())
}
