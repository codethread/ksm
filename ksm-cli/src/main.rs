use anyhow::Result;
use clap::Parser;
use log::{debug, info};
use std::path::PathBuf;

use ksm::app::App;
use ksm::cli::{Cli, Commands};
use ksm::cmd::{cmd_key, cmd_keys, cmd_list, cmd_select};
use ksm::config::Config;

#[cfg(not(tarpaulin_include))]
fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    info!("Starting ksm session manager");

    let cli = Cli::parse();
    debug!("Parsed CLI arguments: {:?}", cli);

    // Load configuration
    let config_path = cli.config.map(PathBuf::from);
    let config = Config::load_from_path(config_path, cli.profile)?;

    // Create App instance with config and Kitty
    let app = App::new(config);

    match cli.command {
        Some(Commands::List) => {
            info!("Listing sessions");
            cmd_list(&app)
        }
        Some(Commands::Key { key, path }) => {
            info!("Switching to project by key: {}", key);
            cmd_key(&app, &key, path)?;
            if !path {
                println!("Switched to session by key: {}", key);
            }
            Ok(())
        }
        Some(Commands::Keys) => {
            info!("Listing all Keys");
            cmd_keys(&app)
        }
        Some(Commands::Select) => {
            info!("Interactive project selection");
            cmd_select(&app)
        }
        None => {
            info!("No command specified, listing sessions");
            cmd_list(&app)
        }
    }
}
