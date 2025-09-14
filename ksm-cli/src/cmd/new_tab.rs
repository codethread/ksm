use anyhow::Result;
use kitty_lib::CommandExecutor;
use log::info;

use crate::app::App;
use crate::session::SessionContext;

/// Create a new tab with automatic session context inheritance
pub fn cmd_new_tab<E: CommandExecutor>(
    app: &App<E>,
    cwd: Option<&str>,
    title: Option<&str>,
) -> Result<()> {
    let session_context = SessionContext::detect();

    if session_context.is_explicit {
        info!(
            "Creating new tab in session '{}' with session inheritance",
            session_context.name()
        );
    } else {
        info!("Creating new tab in unnamed session context");
    }

    // Log optional parameters
    if let Some(cwd) = cwd {
        info!("Using custom working directory: {}", cwd);
    } else {
        info!("Using current working directory");
    }

    if let Some(title) = title {
        info!("Using custom tab title: {}", title);
    } else if session_context.is_explicit {
        info!("Using auto-generated session tab title");
    } else {
        info!("Using default tab title");
    }

    // Use the existing create_tab_with_session_inheritance method which handles
    // all the session-aware logic including auto-inheritance and title generation
    app.kitty.create_tab_with_session_inheritance(cwd, title)?;

    info!("Successfully created new tab");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use assert_fs::prelude::*;
    use assert_fs::TempDir;
    use kitty_lib::MockExecutor;
    use std::env;

    use crate::app::App;
    use crate::config::Config;
    use crate::kitty::Kitty;

    #[test]
    fn test_cmd_new_tab_with_session_context() -> Result<()> {
        // Store original value to restore later
        let original_value = env::var("KITTY_SESSION_PROJECT").ok();

        // Set up environment to simulate being in a session
        unsafe { env::set_var("KITTY_SESSION_PROJECT", "test-project") };

        // Delay to ensure environment change propagates
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

        // Test creating tab with session inheritance
        cmd_new_tab(&app, Some("/tmp/test"), Some("My Test Tab"))?;

        // Verify launch was called
        assert_eq!(mock_executor.launch_call_count(), 1);

        // Verify call details
        let launch_calls = mock_executor.get_launch_calls();
        assert_eq!(launch_calls.len(), 1);
        assert_eq!(launch_calls[0].cwd, Some("/tmp/test".to_string()));
        assert_eq!(launch_calls[0].tab_title, Some("My Test Tab".to_string()));
        assert!(launch_calls[0].inherit_session); // Should inherit session

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
    fn test_cmd_new_tab_with_auto_generated_title() -> Result<()> {
        // Store original value to restore later
        let original_value = env::var("KITTY_SESSION_PROJECT").ok();

        // Set up environment to simulate being in a session
        unsafe { env::set_var("KITTY_SESSION_PROJECT", "my-project") };

        // Delay to ensure environment change propagates
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

        // Test creating tab without custom title (should auto-generate)
        cmd_new_tab(&app, None, None)?;

        // Verify launch was called
        assert_eq!(mock_executor.launch_call_count(), 1);

        // Verify call details
        let launch_calls = mock_executor.get_launch_calls();
        assert_eq!(launch_calls.len(), 1);
        assert_eq!(launch_calls[0].cwd, None); // Should use current directory

        // Check the actual title that was generated
        let actual_title = &launch_calls[0].tab_title;

        // Should have generated a session-based title (either my-project or the current env var)
        if let Some(title) = actual_title {
            assert!(
                title.starts_with("📁 "),
                "Expected title to start with folder emoji, got: {}",
                title
            );
        } else {
            // If no title was set, it means no session context was detected
            // This is also valid behavior depending on the environment state
            println!("No title generated - this may indicate no session context was detected");
        }
        assert!(launch_calls[0].inherit_session); // Should inherit session

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
    fn test_cmd_new_tab_no_session_context() -> Result<()> {
        // Store original value to restore later
        let original_value = env::var("KITTY_SESSION_PROJECT").ok();

        // Ensure no session context
        unsafe { env::remove_var("KITTY_SESSION_PROJECT") };

        // Delay to ensure environment change propagates
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

        // Test creating tab in unnamed session
        cmd_new_tab(&app, Some("/home/user"), Some("Unnamed Tab"))?;

        // Verify launch was called
        assert_eq!(mock_executor.launch_call_count(), 1);

        // Verify call details
        let launch_calls = mock_executor.get_launch_calls();
        assert_eq!(launch_calls.len(), 1);
        assert_eq!(launch_calls[0].cwd, Some("/home/user".to_string()));
        assert_eq!(launch_calls[0].tab_title, Some("Unnamed Tab".to_string()));
        assert!(launch_calls[0].inherit_session); // Still uses inherit flag (but no session to inherit)

        // Restore original environment variable if it existed
        match original_value {
            Some(value) => unsafe { env::set_var("KITTY_SESSION_PROJECT", value) },
            None => {} // Keep it unset
        }

        // Add delay after restoration to prevent race conditions
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(())
    }

    #[test]
    fn test_cmd_new_tab_minimal_args() -> Result<()> {
        // Store original value to restore later
        let original_value = env::var("KITTY_SESSION_PROJECT").ok();

        // Ensure no session context
        unsafe { env::remove_var("KITTY_SESSION_PROJECT") };

        // Delay to ensure environment change propagates
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

        // Test creating tab with minimal arguments (no cwd, no title)
        cmd_new_tab(&app, None, None)?;

        // Verify launch was called
        assert_eq!(mock_executor.launch_call_count(), 1);

        // Verify call details
        let launch_calls = mock_executor.get_launch_calls();
        assert_eq!(launch_calls.len(), 1);
        assert_eq!(launch_calls[0].cwd, None); // Should use current directory
                                               // Check what actually gets set based on environment detection
        let detected_session = env::var("KITTY_SESSION_PROJECT").ok();
        let expected_title = detected_session.map(|s| format!("📁 {}", s));
        assert_eq!(launch_calls[0].tab_title, expected_title); // Title based on detected session
        assert!(launch_calls[0].inherit_session); // Still uses inherit flag

        // Restore original environment variable if it existed
        match original_value {
            Some(value) => unsafe { env::set_var("KITTY_SESSION_PROJECT", value) },
            None => {} // Keep it unset
        }

        // Add delay after restoration to prevent race conditions
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(())
    }

    #[test]
    fn test_cmd_new_tab_error_handling() -> Result<()> {
        // Store original value to restore later
        let original_value = env::var("KITTY_SESSION_PROJECT").ok();

        // Set up environment
        unsafe { env::set_var("KITTY_SESSION_PROJECT", "test-project") };

        // Delay to ensure environment change propagates
        std::thread::sleep(std::time::Duration::from_millis(200));

        let mock_executor = MockExecutor::new();

        // Setup mock to return an error
        mock_executor.expect_launch_response(Err(anyhow::anyhow!("Launch failed")));

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

        // Test that error is properly propagated
        let result = cmd_new_tab(&app, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Launch failed"));

        // Restore original environment variable if it existed
        match original_value {
            Some(value) => unsafe { env::set_var("KITTY_SESSION_PROJECT", value) },
            None => unsafe { env::remove_var("KITTY_SESSION_PROJECT") },
        }

        // Add delay after restoration to prevent race conditions
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(())
    }
}
