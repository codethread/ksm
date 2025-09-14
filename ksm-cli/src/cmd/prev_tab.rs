use anyhow::Result;
use kitty_lib::CommandExecutor;
use log::info;

use crate::app::App;

/// Navigate to the previous tab within the current session
pub fn cmd_prev_tab<E: CommandExecutor>(app: &App<E>, no_wrap: Option<bool>) -> Result<()> {
    // Determine wrap behavior: explicit override > config default
    let use_wrap = match no_wrap {
        Some(explicit_no_wrap) => !explicit_no_wrap,
        None => app.config.default_wrap_tabs(),
    };

    info!(
        "Navigating to previous tab in current session{}",
        if !use_wrap { " (no-wrap)" } else { "" }
    );

    app.kitty.prev_session_tab(use_wrap)?;

    info!("Successfully navigated to previous tab");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use kitty_lib::MockExecutor;
    use std::env;

    use crate::app::App;
    use crate::config::Config;
    use crate::kitty::Kitty;
    use assert_fs::prelude::*;
    use assert_fs::TempDir;

    #[test]
    fn test_cmd_prev_tab() -> Result<()> {
        // Store original value to restore later
        let original_value = env::var("KITTY_SESSION_PROJECT").ok();

        // Determine the actual session name to use (from environment if set, otherwise test value)
        let session_name = env::var("KITTY_SESSION_PROJECT").unwrap_or("test-project".to_string());

        // Set up environment to simulate being in a session
        unsafe { env::set_var("KITTY_SESSION_PROJECT", &session_name) };

        // Small delay to ensure environment change propagates
        std::thread::sleep(std::time::Duration::from_millis(200));

        let mock_executor = MockExecutor::new();

        // Set up a session with multiple tabs using the detected session name
        mock_executor.add_session_tab(&session_name, Some("Tab 1".to_string()));
        mock_executor.add_session_tab(&session_name, Some("Tab 2".to_string()));
        mock_executor.add_session_tab(&session_name, Some("Tab 3".to_string()));

        let kitty = Kitty::with_executor(&mock_executor);
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"[global]
version = "1.0"

[search]
dirs = []
vsc = []
"#;
        let config_file = temp_dir.child("test_config.toml");
        config_file.write_str(config_content).unwrap();
        let config = Config::load_from_path(Some(config_file.path().to_path_buf()), None).unwrap();
        let app = App::with_kitty(config, kitty);

        // Start on second tab
        mock_executor.set_active_tab(2);

        // Test previous tab navigation with default (config default is true)
        cmd_prev_tab(&app, None)?; // use config default (wrap = true)
        assert_eq!(mock_executor.get_active_tab_id(), Some(1));

        // Test previous tab with explicit no-wrap from first tab
        cmd_prev_tab(&app, Some(true))?; // explicit no wrap
        assert_eq!(mock_executor.get_active_tab_id(), Some(1)); // Should stay on first tab

        // Restore original environment variable if it existed
        match original_value {
            Some(value) => unsafe { env::set_var("KITTY_SESSION_PROJECT", value) },
            None => unsafe { env::remove_var("KITTY_SESSION_PROJECT") },
        }

        // Add delay after restoration to prevent race conditions
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(())
    }

    #[test]
    fn test_cmd_prev_tab_no_session() -> Result<()> {
        // Store original value to restore later
        let original_value = env::var("KITTY_SESSION_PROJECT").ok();

        // Ensure no session context
        unsafe { env::remove_var("KITTY_SESSION_PROJECT") };

        // Small delay to ensure environment change propagates
        std::thread::sleep(std::time::Duration::from_millis(200));

        let mock_executor = MockExecutor::new();
        let kitty = Kitty::with_executor(&mock_executor);
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"[global]
version = "1.0"

[search]
dirs = []
vsc = []
"#;
        let config_file = temp_dir.child("test_config.toml");
        config_file.write_str(config_content).unwrap();
        let config = Config::load_from_path(Some(config_file.path().to_path_buf()), None).unwrap();
        let app = App::with_kitty(config, kitty);

        // Should succeed but be a no-op when no session context
        cmd_prev_tab(&app, None)?;

        // Should not make navigation calls since there's no session context or session tabs
        // The actual navigation logic is handled in the Kitty struct

        // Restore original environment variable if it existed
        match original_value {
            Some(value) => unsafe { env::set_var("KITTY_SESSION_PROJECT", value) },
            None => {} // Keep it unset
        }

        // Add delay after restoration to prevent race conditions
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(())
    }
}
