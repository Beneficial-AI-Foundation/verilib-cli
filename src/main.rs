use anyhow::Result;
use clap::Parser;

mod auth;
mod cli;
mod keyring_utils;
mod status;

use auth::handle_auth;
use cli::{Cli, Commands};
use status::handle_status;

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
