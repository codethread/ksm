use anyhow::Result;
use clap::Parser;
use log::{debug, info};
use std::env;

use ksm::app::App;
use ksm::cli::{Cli, Commands};
use ksm::cmd::{cmd_key, cmd_list, cmd_select};

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    info!("Starting ksm session manager");

    // Create App instance with Kitty
    let app = App::new();

    let cli = Cli::parse();
    debug!("Parsed CLI arguments: {:?}", cli);

    // Check if KSM_WORK environment variable is set (truthy)
    let env_work = env::var("KSM_WORK")
        .map(|val| !val.is_empty() && val != "0" && val.to_lowercase() != "false")
        .unwrap_or(false);

    match cli.command {
        Some(Commands::List) => {
            info!("Listing sessions");
            cmd_list(&app)
        }
        Some(Commands::Key { key, work, path }) => {
            let effective_work = work || env_work;
            info!(
                "Switching to project by key: {} (work: {})",
                key, effective_work
            );
            cmd_key(&app, &key, effective_work, path)?;
            if !path {
                println!("Switched to session by key: {}", key);
            }
            Ok(())
        }
        Some(Commands::Select { work }) => {
            let effective_work = work || env_work;
            info!("Interactive project selection (work: {})", effective_work);
            cmd_select(&app, effective_work)
        }
        None => {
            info!("No command specified, listing sessions");
            cmd_list(&app)
        }
    }
}
