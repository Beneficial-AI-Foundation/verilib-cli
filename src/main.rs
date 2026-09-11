use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod cli;
mod commands;
mod config;
mod constants;
mod download;
mod executor;
mod storage;
mod structure;

use cli::{ApiCommands, Cli, Commands};
use commands::{
    handle_api, handle_atomize, handle_auth, handle_create, handle_deploy, handle_init,
    handle_pull, handle_reclone, handle_specify, handle_status, handle_verify, ApiSubcommand,
    StatusFilter,
};

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
        Commands::Pull { url } => {
            handle_pull(url, cli.debug).await?;
        }
        Commands::Api { command } => {
            let subcommand = map_api_command(command)?;
            handle_api(subcommand, cli.json, cli.dry_run).await?;
        }
        Commands::Create { project_root, root } => {
            handle_create(project_root, root).await?;
        }
        Commands::Atomize {
            project_root,
            update_stubs,
            no_probe,
            check_only,
            atoms_only,
            rust_analyzer,
        } => {
            handle_atomize(
                project_root,
                update_stubs,
                no_probe,
                check_only,
                atoms_only,
                rust_analyzer,
            )
            .await?;
        }
        Commands::Specify {
            project_root,
            no_probe,
            check_only,
        } => {
            handle_specify(project_root, no_probe, check_only).await?;
        }
        Commands::Verify {
            project_root,
            package,
            verify_only_module,
            no_probe,
            check_only,
        } => {
            handle_verify(
                project_root,
                package,
                verify_only_module,
                no_probe,
                check_only,
            )
            .await?;
        }
    }

    Ok(())
}

fn map_api_command(command: ApiCommands) -> Result<ApiSubcommand> {
    match command {
        ApiCommands::Get { file } => Ok(ApiSubcommand::Get {
            file: PathBuf::from(file),
        }),
        ApiCommands::List { filter } => {
            let parsed = filter
                .as_deref()
                .map(parse_status_filter)
                .transpose()?;
            Ok(ApiSubcommand::List { filter: parsed })
        }
        ApiCommands::Set {
            file,
            specified,
            ignored,
            verified,
        } => Ok(ApiSubcommand::Set {
            file: PathBuf::from(file),
            specified,
            ignored,
            verified,
        }),
        ApiCommands::Batch { input } => Ok(ApiSubcommand::Batch {
            input: PathBuf::from(input),
        }),
        ApiCommands::CreateFile {
            path,
            content,
            from_file,
            disabled,
            specified,
            status_id,
            statement_type,
            code_name,
        } => Ok(ApiSubcommand::CreateFile {
            path: PathBuf::from(path),
            content,
            from_file: from_file.map(PathBuf::from),
            disabled,
            specified,
            status_id,
            statement_type,
            code_name,
        }),
    }
}

fn parse_status_filter(value: &str) -> Result<StatusFilter> {
    match value.to_lowercase().as_str() {
        "specified" => Ok(StatusFilter::Specified),
        "ignored" => Ok(StatusFilter::Ignored),
        "verified" => Ok(StatusFilter::Verified),
        other => anyhow::bail!("Unknown filter '{}'. Use: specified, ignored, verified", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_filter() {
        assert!(matches!(
            parse_status_filter("specified").unwrap(),
            StatusFilter::Specified
        ));
        assert!(parse_status_filter("bad").is_err());
    }
}
