use anyhow::Result;
use clap::Parser;
use log::{debug, info};
use std::path::PathBuf;

use ksm::app::App;
use ksm::cli::{Cli, Commands};
use ksm::cmd::{
    cmd_close_all_session_tabs, cmd_key, cmd_keys, cmd_list, cmd_new_tab, cmd_next_tab,
    cmd_prev_tab, cmd_rename_tab, cmd_select,
};
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
        Some(Commands::NextTab { no_wrap }) => {
            info!("Navigating to next tab in session");
            let no_wrap_option = if no_wrap { Some(true) } else { None };
            cmd_next_tab(&app, no_wrap_option)?;
            Ok(())
        }
        Some(Commands::PrevTab { no_wrap }) => {
            info!("Navigating to previous tab in session");
            let no_wrap_option = if no_wrap { Some(true) } else { None };
            cmd_prev_tab(&app, no_wrap_option)?;
            Ok(())
        }
        Some(Commands::NewTab { cwd, title }) => {
            info!("Creating new tab with session inheritance");
            cmd_new_tab(&app, cwd.as_deref(), title.as_deref())?;
            Ok(())
        }
        Some(Commands::CloseAllSessionTabs { session, force }) => {
            info!(
                "Closing all tabs in session{}",
                if force { " (forced)" } else { "" }
            );
            cmd_close_all_session_tabs(&app, session.as_deref(), force)?;
            Ok(())
        }
        Some(Commands::RenameTab { description }) => {
            info!("Renaming current tab to: {}", description);
            cmd_rename_tab(&app, &description)?;
            Ok(())
        }
        None => {
            info!("No command specified, listing sessions");
            cmd_list(&app)
        }
    }
}
