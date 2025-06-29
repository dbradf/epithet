use anyhow::Result;
use std::{env::args, path::Path};

use clap::Parser;
use epithet_2::epithet_config::{get_config_path, EpithetConfig};

const BUILD_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, clap::Parser)]
#[command(name = BUILD_NAME, version = VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    Install {
        #[arg(short, long, default_value = "false")]
        force: bool,
    },

    Lookup {
        alias: String,
        args: Vec<String>,
    },
}

fn main() {
    let command = std::env::args().next().unwrap();
    let config = EpithetConfig::new()
        .unwrap_or_else(|_| panic!("Failed to read config at '{}'", get_config_path().display()));

    let result = if command.ends_with(BUILD_NAME) {
        let cli = Cli::parse();
        epithet_command(&cli, &config)
    } else {
        let command = Path::new(&command).file_name().unwrap().to_str().unwrap();
        let rest_args: Vec<String> = std::env::args().skip(1).collect();
        alias_execution(command, &rest_args, &config)
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn epithet_command(cli: &Cli, config: &EpithetConfig) -> Result<()> {
    match &cli.command {
        Commands::Install { force } => {
            println!("Installing...");
        }
        Commands::Lookup { alias, args } => {
            if let Some(alias) = config.lookup_alias(alias, args) {
                println!("{}", alias);
            } else {
                println!("Alias not found: {}", alias);
            }
        }
    }

    Ok(())
}

fn alias_execution(command: &str, args: &[String], config: &EpithetConfig) -> Result<()> {
    config.execute(command, args)?;
    Ok(())
}
