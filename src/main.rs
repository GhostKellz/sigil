use clap::Parser;
use anyhow::Result;
use tracing::info;

mod cli;
mod config;
mod runtime;
mod modules;
mod error;

use cli::{Cli, Commands};
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("🔮 Sigil starting up...");

    let cli = Cli::parse();
    let config = Config::load().await?;

    match &cli.command {
        Commands::System(args) => {
            modules::system::handle_command(args, &config).await?;
        }
        Commands::Task(args) => {
            runtime::task_runner::handle_command(args, &config).await?;
        }
        Commands::Config(args) => {
            config::handle_command(args).await?;
        }
        Commands::Version => {
            println!("Sigil v{}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
