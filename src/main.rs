use anyhow::Result;
use std::{
    os::unix::fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use epithet::epithet_config::{get_config_path, EpithetConfig};

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
            install_aliases(*force, config)?;
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

fn install_aliases(force: bool, config: &EpithetConfig) -> Result<()> {
    let executable = std::env::current_exe()?;
    let bin_path = PathBuf::from(shellexpand::tilde("~/.local/epithet/bin").to_string());

    if !bin_path.exists() {
        eprintln!("Creating directory: {}", bin_path.display());
        std::fs::create_dir_all(&bin_path)?;
    }

    let binary_path = executable.canonicalize()?;

    eprintln!("Installing aliases to: {}", bin_path.display());
    eprintln!("export PATH=$PATH:{}", bin_path.display());

    if let Some(aliases) = &config.aliases {
        for alias in aliases.keys() {
            let alias_path = bin_path.join(alias);
            if alias_path.exists() {
                if force {
                    eprintln!("Removing existing alias: {}", alias_path.display());
                    std::fs::remove_file(&alias_path)?;
                } else {
                    eprintln!(
                        "Alias already exists (run with --force to overwrite): {}",
                        alias_path.display()
                    );
                    continue;
                }
            }

            eprintln!(
                "Creating symlink: {} -> {}",
                binary_path.display(),
                alias_path.display()
            );
            fs::symlink(&binary_path, &alias_path)?;
        }
    }

    Ok(())
}
