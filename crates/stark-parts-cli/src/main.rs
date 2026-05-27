use clap::{Parser, Subcommand};
use std::process::ExitCode;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Command line entrypoint for maintaining the committed Stark catalog.
///
/// This scaffold intentionally stops short of implementing catalog behavior.
/// The next plan steps define the schema and crawler layers; this binary exists
/// now so CI can prove the workspace, argument parser, and tracing setup compile.
#[derive(Debug, Parser)]
#[command(name = "stark-parts")]
#[command(about = "Maintain the Stark parts catalog")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Commands that read or write the committed catalog state.
    Catalog {
        #[command(subcommand)]
        command: CatalogCommand,
    },
}

#[derive(Debug, Subcommand)]
enum CatalogCommand {
    /// Create the initial committed catalog.
    Init,
    /// Refresh an existing committed catalog.
    Update,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match run(cli) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<&'static str, &'static str> {
    match cli.command {
        Command::Catalog {
            command: CatalogCommand::Init,
        } => {
            info!("catalog init requested");
            Err("catalog init is not implemented yet")
        }
        Command::Catalog {
            command: CatalogCommand::Update,
        } => {
            info!("catalog update requested");
            Err("catalog update is not implemented yet")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn catalog_subcommands_report_unimplemented_status() {
        let init = Cli::try_parse_from(["stark-parts", "catalog", "init"]).unwrap();
        assert_eq!(run(init), Err("catalog init is not implemented yet"));

        let update = Cli::try_parse_from(["stark-parts", "catalog", "update"]).unwrap();
        assert_eq!(run(update), Err("catalog update is not implemented yet"));
    }
}
