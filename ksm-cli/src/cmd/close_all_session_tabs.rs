use anyhow::Result;
use kitty_lib::{CommandExecutor, KittenLsCommand};
use log::{debug, info, warn};
use std::io::{self, Write};

use crate::app::App;
use crate::session::SessionContext;

/// Close all tabs in the current session (or specified session)
pub fn cmd_close_all_session_tabs<E: CommandExecutor>(
    app: &App<E>,
    session_name: Option<&str>,
    force: bool,
) -> Result<()> {
    cmd_close_all_session_tabs_with_context(app, session_name, force, SessionContext::detect)
}

/// Close all tabs in the current session (or specified session) with injectable session detection
pub fn cmd_close_all_session_tabs_with_context<E: CommandExecutor, F>(
    app: &App<E>,
    session_name: Option<&str>,
    force: bool,
    detect_session: F,
) -> Result<()>
where
    F: FnOnce() -> SessionContext,
{
    // Determine the target session
    let target_session = match session_name {
        Some(name) => name.to_string(),
        None => {
            let context = detect_session();
            if !context.is_explicit {
                warn!(
                    "No active session detected. Use --session <name> to specify a session, or run from within a session context."
                );
                return Ok(());
            }
            context.session_name
        }
    };

    info!("Querying tabs for session '{}'", target_session);

    // Query all tabs in the target session
    let ls_command = KittenLsCommand::new().match_tab_env("KITTY_SESSION_PROJECT", &target_session);
    let os_windows = app.kitty.ls(ls_command)?;

    // Collect all tabs from the session
    let mut session_tabs = Vec::new();
    for os_window in os_windows {
        for tab in os_window.tabs {
            // Verify this tab actually belongs to our target session
            let matches_session = tab.windows.iter().any(|w| {
                w.env
                    .get("KITTY_SESSION_PROJECT")
                    .is_some_and(|v| v == &target_session)
            });
            if matches_session {
                session_tabs.push(tab);
            }
        }
    }

    if session_tabs.is_empty() {
        info!("No tabs found in session '{}'", target_session);
        return Ok(());
    }

    // Sort tabs by ID for consistent ordering
    session_tabs.sort_by_key(|t| t.id);
    let tab_count = session_tabs.len();

    // Handle the edge case of closing the last tab
    if tab_count == 1 {
        warn!(
            "This is the only tab in session '{}'. Closing it will end the session.",
            target_session
        );
    }

    // Confirmation prompt (unless --force is specified)
    if !force {
        print!(
            "Close {} tab{} in session '{}'? (y/N): ",
            tab_count,
            if tab_count == 1 { "" } else { "s" },
            target_session
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            info!("Operation cancelled.");
            return Ok(());
        }
    }

    // Close all tabs in the session
    let mut successfully_closed = 0;
    let mut failed_closes = Vec::new();

    for tab in &session_tabs {
        debug!("Closing tab {} ('{}')", tab.id, tab.title);

        match app.kitty.close_tab(tab.id) {
            Ok(result) => {
                if result.is_success() {
                    successfully_closed += 1;
                    debug!("Successfully closed tab {} ('{}')", tab.id, tab.title);
                } else {
                    let error_msg = result.error_message.unwrap_or_default();
                    warn!(
                        "Failed to close tab {} ('{}'): {}",
                        tab.id, tab.title, error_msg
                    );
                    failed_closes.push((tab.id, error_msg));
                }
            }
            Err(e) => {
                failed_closes.push((tab.id, e.to_string()));
                warn!("Error closing tab {} ('{}'): {}", tab.id, tab.title, e);
            }
        }
    }

    // Report results
    if successfully_closed > 0 {
        info!(
            "Successfully closed {} tab{} in session '{}'",
            successfully_closed,
            if successfully_closed == 1 { "" } else { "s" },
            target_session
        );
    }

    if !failed_closes.is_empty() {
        warn!(
            "Failed to close {} tab{}: {:?}",
            failed_closes.len(),
            if failed_closes.len() == 1 { "" } else { "s" },
            failed_closes.iter().map(|(id, _)| id).collect::<Vec<_>>()
        );

        // If we failed to close any tabs, return an error
        return Err(anyhow::anyhow!(
            "Failed to close {} out of {} tabs in session '{}'",
            failed_closes.len(),
            tab_count,
            target_session
        ));
    }

    if successfully_closed == tab_count {
        info!(
            "All tabs in session '{}' have been closed. The session is now empty.",
            target_session
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use kitty_lib::MockExecutor;

    use crate::app::App;
    use crate::config::Config;
    use crate::kitty::Kitty;

    #[test]
    fn test_cmd_close_all_session_tabs_with_explicit_session() -> Result<()> {
        let mock_executor = MockExecutor::new();

        // Set up mock data: 3 tabs in "test-project" session
        let tab1 = mock_executor.add_session_tab("test-project", Some("Tab 1".to_string()));
        let tab2 = mock_executor.add_session_tab("test-project", Some("Tab 2".to_string()));
        let tab3 = mock_executor.add_session_tab("test-project", Some("Tab 3".to_string()));

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

        // Test closing all tabs with --force (no confirmation)
        cmd_close_all_session_tabs(&app, Some("test-project"), true)?;

        // Verify ls was called to query session tabs
        assert_eq!(mock_executor.ls_call_count(), 1);
        let ls_calls = mock_executor.get_ls_calls();
        assert!(
            ls_calls[0]
                .match_arg
                .as_ref()
                .unwrap()
                .contains("test-project")
        );

        // Verify all tabs were closed
        assert_eq!(mock_executor.close_tab_call_count(), 3);
        let close_calls = mock_executor.get_close_tab_calls();
        let closed_tab_ids: Vec<u32> = close_calls.iter().map(|c| c.tab_id).collect();
        assert!(closed_tab_ids.contains(&tab1));
        assert!(closed_tab_ids.contains(&tab2));
        assert!(closed_tab_ids.contains(&tab3));

        Ok(())
    }

    #[test]
    fn test_cmd_close_all_session_tabs_no_tabs() -> Result<()> {
        let mock_executor = MockExecutor::new();
        // Don't add any tabs

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

        // Test with non-existent session
        let result = cmd_close_all_session_tabs(&app, Some("non-existent-session"), true);
        assert!(result.is_ok()); // Should succeed gracefully

        // Verify ls was called but no close operations
        assert_eq!(mock_executor.ls_call_count(), 1);
        assert_eq!(mock_executor.close_tab_call_count(), 0);

        Ok(())
    }

    #[test]
    fn test_cmd_close_all_session_tabs_current_session() -> Result<()> {
        let mock_executor = MockExecutor::new();

        // Set up mock data: 2 tabs in current session
        let tab1 =
            mock_executor.add_session_tab("current-session", Some("Current Tab 1".to_string()));
        let tab2 =
            mock_executor.add_session_tab("current-session", Some("Current Tab 2".to_string()));

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

        // Mock session detection to simulate being in a session
        let mock_detect = || SessionContext::new("current-session");

        // Test closing all tabs in current session (no explicit session name)
        cmd_close_all_session_tabs_with_context(&app, None, true, mock_detect)?;

        // Verify correct session was queried
        assert_eq!(mock_executor.ls_call_count(), 1);
        let ls_calls = mock_executor.get_ls_calls();
        assert!(
            ls_calls[0]
                .match_arg
                .as_ref()
                .unwrap()
                .contains("current-session")
        );

        // Verify both tabs were closed
        assert_eq!(mock_executor.close_tab_call_count(), 2);
        let close_calls = mock_executor.get_close_tab_calls();
        let closed_tab_ids: Vec<u32> = close_calls.iter().map(|c| c.tab_id).collect();
        assert!(closed_tab_ids.contains(&tab1));
        assert!(closed_tab_ids.contains(&tab2));

        Ok(())
    }

    #[test]
    fn test_cmd_close_all_session_tabs_no_session_context() -> Result<()> {
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

        // Mock session detection to simulate no session context
        let mock_detect = || SessionContext::unnamed();

        // Test with no session context and no explicit session name
        let result = cmd_close_all_session_tabs_with_context(&app, None, true, mock_detect);
        assert!(result.is_ok()); // Should succeed but do nothing

        // Verify no operations were performed
        assert_eq!(mock_executor.ls_call_count(), 0);
        assert_eq!(mock_executor.close_tab_call_count(), 0);

        Ok(())
    }

    #[test]
    fn test_cmd_close_all_session_tabs_partial_failure() -> Result<()> {
        let mock_executor = MockExecutor::new();

        // Set up mock data: 3 tabs in session
        let tab1 = mock_executor.add_session_tab("test-session", Some("Tab 1".to_string()));
        let _tab2 = mock_executor.add_session_tab("test-session", Some("Tab 2".to_string()));
        let tab3 = mock_executor.add_session_tab("test-session", Some("Tab 3".to_string()));

        // Queue responses: first close succeeds, second fails, third succeeds
        use kitty_lib::KittyCommandResult;
        mock_executor.expect_close_tab_response(Ok(KittyCommandResult::success_empty()));
        mock_executor.expect_close_tab_response(Ok(KittyCommandResult::error("Permission denied")));
        mock_executor.expect_close_tab_response(Ok(KittyCommandResult::success_empty()));

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

        // Test should return an error due to partial failure
        let result = cmd_close_all_session_tabs(&app, Some("test-session"), true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to close 1 out of 3 tabs")
        );

        // Verify all tabs were attempted to be closed
        assert_eq!(mock_executor.close_tab_call_count(), 3);
        let close_calls = mock_executor.get_close_tab_calls();
        let attempted_tab_ids: Vec<u32> = close_calls.iter().map(|c| c.tab_id).collect();
        assert!(attempted_tab_ids.contains(&tab1));
        assert!(attempted_tab_ids.contains(&tab3));

        Ok(())
    }
}
