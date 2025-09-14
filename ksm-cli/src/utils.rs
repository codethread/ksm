use anyhow::Result;
use kitty_lib::CommandExecutor;
use log::info;
use std::env;

use crate::app::App;

pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        let home = env::var("HOME").unwrap_or_default();
        path.replacen("~", &home, 1)
    } else {
        path.to_string()
    }
}

pub fn parse_project_selection(selected_text: &str) -> Result<(String, String)> {
    // Parse the selected item to get the project path
    // Format is "project_name (path)"
    // We need to find the last " (" that is followed by a matching ")" at the end

    if !selected_text.ends_with(')') {
        return Err(anyhow::anyhow!(
            "Invalid project selection format: {}",
            selected_text
        ));
    }

    // Find the matching opening parenthesis for the closing one at the end
    let mut paren_count = 0;
    let mut start_paren_pos = None;

    for (i, char) in selected_text.char_indices().rev() {
        match char {
            ')' => paren_count += 1,
            '(' => {
                paren_count -= 1;
                if paren_count == 0 {
                    // Check if this opening paren is preceded by a space
                    if i > 0 && selected_text.chars().nth(i - 1) == Some(' ') {
                        start_paren_pos = Some(i - 1); // Include the space
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(start_pos) = start_paren_pos {
        let project_name = &selected_text[..start_pos];
        let project_path = &selected_text[start_pos + 2..selected_text.len() - 1]; // +2 to skip " (", -1 to skip ")"
        return Ok((project_name.to_string(), project_path.to_string()));
    }

    Err(anyhow::anyhow!(
        "Invalid project selection format: {}",
        selected_text
    ))
}

pub fn format_project_for_selection(name: &str, path: &str) -> String {
    format!("{} ({})", name, path)
}

pub fn format_session_tab_title(project_name: &str) -> String {
    format!("📁 {}", project_name)
}

/// Direction for tab navigation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavigationDirection {
    Next,
    Previous,
}

impl NavigationDirection {
    pub fn action_name(&self) -> &'static str {
        match self {
            NavigationDirection::Next => "next",
            NavigationDirection::Previous => "previous",
        }
    }
}

/// Navigate to the next or previous tab within the current session
///
/// This function handles the shared logic for both next and previous tab navigation:
/// - Determines wrap behavior from explicit override or config default
/// - Logs navigation actions with appropriate messages
/// - Delegates to the appropriate Kitty method based on direction
pub fn navigate_tab<E: CommandExecutor>(
    app: &App<E>,
    direction: NavigationDirection,
    no_wrap: Option<bool>,
) -> Result<()> {
    // Determine wrap behavior: explicit override > config default
    let use_wrap = match no_wrap {
        Some(explicit_no_wrap) => !explicit_no_wrap,
        None => app.config.default_wrap_tabs(),
    };

    info!(
        "Navigating to {} tab in current session{}",
        direction.action_name(),
        if !use_wrap { " (no-wrap)" } else { "" }
    );

    match direction {
        NavigationDirection::Next => app.kitty.next_session_tab(use_wrap)?,
        NavigationDirection::Previous => app.kitty.prev_session_tab(use_wrap)?,
    }

    info!("Successfully navigated to {} tab", direction.action_name());
    Ok(())
}

#[cfg(test)]
pub mod test_utils {
    use crate::app::App;
    use crate::config::Config;
    use crate::kitty::Kitty;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use kitty_lib::MockExecutor;
    use std::env;

    /// Manages environment variable restoration for tests
    pub struct EnvGuard {
        var_name: String,
        original_value: Option<String>,
    }

    impl EnvGuard {
        pub fn new(var_name: &str) -> Self {
            let original_value = env::var(var_name).ok();
            Self {
                var_name: var_name.to_string(),
                original_value,
            }
        }

        pub fn set(&self, value: &str) {
            unsafe {
                env::set_var(&self.var_name, value);
            }
            // Small delay to ensure environment change propagates
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        pub fn remove(&self) {
            unsafe {
                env::remove_var(&self.var_name);
            }
            // Small delay to ensure environment change propagates
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original_value {
                Some(value) => unsafe { env::set_var(&self.var_name, value) },
                None => {} // Keep it unset
            }
            // Add delay after restoration to prevent race conditions
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }

    /// Creates a test app with MockExecutor and basic config
    /// Returns the app and temp_dir. The executor is embedded in the app.
    pub fn create_test_app_with_executor(
        mock_executor: &MockExecutor,
    ) -> (App<&MockExecutor>, TempDir) {
        let kitty = Kitty::with_executor(mock_executor);
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

        (app, temp_dir)
    }

    /// Sets up a mock session with multiple tabs for testing navigation
    pub fn setup_session_tabs(mock_executor: &MockExecutor, session_name: &str) {
        mock_executor.add_session_tab(session_name, Some("Tab 1".to_string()));
        mock_executor.add_session_tab(session_name, Some("Tab 2".to_string()));
        mock_executor.add_session_tab(session_name, Some("Tab 3".to_string()));
    }

    /// Gets or creates a test session name from environment
    pub fn get_test_session_name() -> String {
        env::var("KITTY_SESSION_PROJECT").unwrap_or("test-project".to_string())
    }

    /// Helper to create basic config content for tests
    pub fn create_test_config_content() -> &'static str {
        r#"[global]
version = "1.0"

[search]
dirs = []
vsc = []
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kitty_lib::MockExecutor;
    use std::env;

    #[test]
    fn test_expand_tilde_with_home() {
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { env::set_var("HOME", "/home/testuser") };
        assert_eq!(expand_tilde("~/Documents"), "/home/testuser/Documents");
        assert_eq!(expand_tilde("~/dev/project"), "/home/testuser/dev/project");
    }

    #[test]
    fn test_expand_tilde_without_tilde() {
        assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
        assert_eq!(expand_tilde("relative/path"), "relative/path");
    }

    #[test]
    fn test_parse_project_selection_valid() {
        let input = "my-project (/home/user/dev/my-project)";
        let result = parse_project_selection(input).unwrap();
        assert_eq!(result.0, "my-project");
        assert_eq!(result.1, "/home/user/dev/my-project");
    }

    #[test]
    fn test_parse_project_selection_with_spaces() {
        let input = "my awesome project (/home/user/dev/my awesome project)";
        let result = parse_project_selection(input).unwrap();
        assert_eq!(result.0, "my awesome project");
        assert_eq!(result.1, "/home/user/dev/my awesome project");
    }

    #[test]
    fn test_parse_project_selection_invalid() {
        let invalid_inputs = vec![
            "project-without-path",
            "project (/incomplete/path",
            "project incomplete/path)",
            "",
        ];

        for input in invalid_inputs {
            assert!(parse_project_selection(input).is_err());
        }
    }

    #[test]
    fn test_format_project_for_selection() {
        assert_eq!(
            format_project_for_selection("my-project", "/home/user/dev/my-project"),
            "my-project (/home/user/dev/my-project)"
        );

        assert_eq!(
            format_project_for_selection("project with spaces", "/path/with spaces"),
            "project with spaces (/path/with spaces)"
        );
    }

    #[test]
    fn test_format_and_parse_roundtrip() {
        let name = "test-project";
        let path = "/home/user/dev/test-project";

        let formatted = format_project_for_selection(name, path);
        let (parsed_name, parsed_path) = parse_project_selection(&formatted).unwrap();

        assert_eq!(parsed_name, name);
        assert_eq!(parsed_path, path);
    }

    #[test]
    fn test_parse_project_selection_edge_cases() {
        // Test with parentheses in project name
        let input = "project (with parens) (/home/user/project (with parens))";
        let result = parse_project_selection(input).unwrap();
        assert_eq!(result.0, "project (with parens)");
        assert_eq!(result.1, "/home/user/project (with parens)");

        // Test with multiple spaces
        let input = "project   with   spaces (/path/with/spaces)";
        let result = parse_project_selection(input).unwrap();
        assert_eq!(result.0, "project   with   spaces");
        assert_eq!(result.1, "/path/with/spaces");
    }

    #[test]
    fn test_format_session_tab_title() {
        assert_eq!(format_session_tab_title("my-project"), "📁 my-project");
        assert_eq!(
            format_session_tab_title("project with spaces"),
            "📁 project with spaces"
        );
        assert_eq!(format_session_tab_title(""), "📁 ");
    }

    #[test]
    fn test_navigation_direction_action_name() {
        assert_eq!(NavigationDirection::Next.action_name(), "next");
        assert_eq!(NavigationDirection::Previous.action_name(), "previous");
    }

    #[test]
    fn test_navigate_tab_next() -> Result<()> {
        use test_utils::*;

        let env_guard = EnvGuard::new("KITTY_SESSION_PROJECT");
        let session_name = "test-navigate-next-session";
        env_guard.set(session_name);

        let mock_executor = MockExecutor::new();
        setup_session_tabs(&mock_executor, session_name);
        let (app, _temp_dir) = create_test_app_with_executor(&mock_executor);

        // Test next tab navigation with default (config default is true)
        navigate_tab(&app, NavigationDirection::Next, None)?;
        assert_eq!(mock_executor.get_active_tab_id(), Some(2));

        // Test next tab with explicit no-wrap from last tab
        mock_executor.set_active_tab(3);
        navigate_tab(&app, NavigationDirection::Next, Some(true))?;
        assert_eq!(mock_executor.get_active_tab_id(), Some(3)); // Should stay on last tab

        Ok(())
    }

    #[test]
    fn test_navigate_tab_previous() -> Result<()> {
        use test_utils::*;

        let env_guard = EnvGuard::new("KITTY_SESSION_PROJECT");
        let session_name = "test-navigate-prev-session";
        env_guard.set(session_name);

        let mock_executor = MockExecutor::new();
        setup_session_tabs(&mock_executor, session_name);
        let (app, _temp_dir) = create_test_app_with_executor(&mock_executor);

        // Start on second tab
        mock_executor.set_active_tab(2);

        // Test previous tab navigation with default (config default is true)
        navigate_tab(&app, NavigationDirection::Previous, None)?;
        assert_eq!(mock_executor.get_active_tab_id(), Some(1));

        // Test previous tab with explicit no-wrap from first tab
        navigate_tab(&app, NavigationDirection::Previous, Some(true))?;
        assert_eq!(mock_executor.get_active_tab_id(), Some(1)); // Should stay on first tab

        Ok(())
    }

    #[test]
    fn test_navigate_tab_no_session() -> Result<()> {
        use test_utils::*;

        let env_guard = EnvGuard::new("KITTY_SESSION_PROJECT");
        env_guard.remove();

        let mock_executor = MockExecutor::new();
        let (app, _temp_dir) = create_test_app_with_executor(&mock_executor);

        // Should succeed but be a no-op when no session context
        navigate_tab(&app, NavigationDirection::Next, None)?;
        navigate_tab(&app, NavigationDirection::Previous, None)?;

        // Should not make navigation calls since there's no session context
        assert_eq!(mock_executor.get_navigate_tab_calls().len(), 0);

        Ok(())
    }
}
