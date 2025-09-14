use anyhow::Result;
use kitty_lib::CommandExecutor;

use crate::app::App;
use crate::utils::{NavigationDirection, navigate_tab};

/// Navigate to the next tab within the current session
pub fn cmd_next_tab<E: CommandExecutor>(app: &App<E>, no_wrap: Option<bool>) -> Result<()> {
    navigate_tab(app, NavigationDirection::Next, no_wrap)
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
    fn test_cmd_next_tab() -> Result<()> {
        let env_guard = EnvGuard::new("KITTY_SESSION_PROJECT");
        let session_name = "test-next-tab-session";
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

        // Test next tab navigation with default (config default is true)
        cmd_next_tab(&app, None)?; // use config default (wrap = true)
        assert_eq!(mock_executor.get_active_tab_id(), Some(2));

        // Test next tab with explicit no-wrap
        mock_executor.set_active_tab(3); // Go to last tab
        cmd_next_tab(&app, Some(true))?; // explicit no wrap
        assert_eq!(mock_executor.get_active_tab_id(), Some(3)); // Should stay on last tab

        Ok(())
    }

    #[test]
    fn test_cmd_next_tab_no_session() -> Result<()> {
        let env_guard = EnvGuard::new("KITTY_SESSION_PROJECT");
        env_guard.remove();

        let mock_executor = MockExecutor::new();
        let kitty = Kitty::with_executor(&mock_executor);
        let temp_dir = TempDir::new().unwrap();

        let config_content = create_test_config_content();
        let config_file = temp_dir.child("test_config.toml");
        config_file.write_str(config_content).unwrap();

        let config = Config::load_from_path(Some(config_file.path().to_path_buf()), None).unwrap();
        let app = App::with_kitty(config, kitty);

        // Don't add any session tabs - this will trigger the "no session context" path
        // But add some unnamed tabs to ensure has_session_tabs() returns false
        mock_executor.add_unnamed_tab(Some("Regular Tab 1".to_string()));
        mock_executor.add_unnamed_tab(Some("Regular Tab 2".to_string()));

        // Check if we actually have a session context from the global environment
        let has_env_session = std::env::var("KITTY_SESSION_PROJECT").is_ok();

        let result = cmd_next_tab(&app, None);

        if has_env_session {
            // If there's an environment session, navigation may try to execute and fail
            // This is expected when no tabs exist for the session
            if result.is_err() {
                assert!(result.unwrap_err().to_string().contains("No tabs found"));
            }
        } else {
            // Should succeed as a no-op when no session context and no session tabs
            assert!(
                result.is_ok(),
                "Expected success when no session context, but got: {:?}",
                result
            );
            // Verify no navigation calls were made since there are no session tabs
            assert_eq!(mock_executor.get_navigate_tab_calls().len(), 0);
        }

        Ok(())
    }
}
