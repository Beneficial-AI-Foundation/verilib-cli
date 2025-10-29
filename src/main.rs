use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod constants;
mod download;
mod storage;

use cli::{Cli, Commands};
use commands::{handle_auth, handle_deploy, handle_init, handle_reclone, handle_status};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth => {
            handle_auth().await?;
        }
        Commands::Status => {
            handle_status().await?;
        }
        Commands::Init { id, url } => {
            handle_init(id, url, cli.debug).await?;
        }
        Commands::Reclone => {
            handle_reclone(cli.debug).await?;
        }
        Commands::Deploy { url } => {
            handle_deploy(url, cli.debug).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_api_key_validation() {
        // Add tests for API key validation logic
    }
}
