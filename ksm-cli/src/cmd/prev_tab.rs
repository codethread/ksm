use anyhow::Result;
use kitty_lib::CommandExecutor;

use crate::app::App;
use crate::utils::{NavigationDirection, navigate_tab};

/// Navigate to the previous tab within the current session
pub fn cmd_prev_tab<E: CommandExecutor>(app: &App<E>, no_wrap: Option<bool>) -> Result<()> {
    navigate_tab(app, NavigationDirection::Previous, no_wrap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::Config;
    use crate::kitty::Kitty;
    use crate::utils::test_utils::*;
    use anyhow::Result;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use kitty_lib::MockExecutor;

    #[test]
    fn test_cmd_prev_tab() -> Result<()> {
        let env_guard = EnvGuard::new("KITTY_SESSION_PROJECT");
        let session_name = "test-prev-tab-session";
        env_guard.set(session_name);

        let mock_executor = MockExecutor::new();
        setup_session_tabs(&mock_executor, session_name);
        let kitty = Kitty::with_executor(&mock_executor);
        let temp_dir = TempDir::new().unwrap();

        let config_content = create_test_config_content();
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

        Ok(())
    }

    #[test]
    fn test_cmd_prev_tab_no_session() -> Result<()> {
        let env_guard = EnvGuard::new("KITTY_SESSION_PROJECT");
        env_guard.remove();

        let mock_executor = MockExecutor::new();
        // Don't add any session tabs - this should result in no navigation occurring
        let kitty = Kitty::with_executor(&mock_executor);
        let temp_dir = TempDir::new().unwrap();

        let config_content = create_test_config_content();
        let config_file = temp_dir.child("test_config.toml");
        config_file.write_str(config_content).unwrap();

        let config = Config::load_from_path(Some(config_file.path().to_path_buf()), None).unwrap();
        let app = App::with_kitty(config, kitty);

        // Should succeed but be a no-op when no session context and no session tabs
        cmd_prev_tab(&app, None)?;

        // Should not make navigation calls since there's no session context
        assert_eq!(mock_executor.get_navigate_tab_calls().len(), 0);

        Ok(())
    }
}
