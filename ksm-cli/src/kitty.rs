use anyhow::Result;
use kitty_lib::commands::set_tab_title::KittenSetTabTitleCommand;
use kitty_lib::{
    CommandExecutor, KittenCloseTabCommand, KittenFocusTabCommand, KittenLaunchCommand,
    KittenLsCommand, KittenNavigateTabCommand, KittyExecutor, KittyTab, TabNavigationDirection,
};
use log::{debug, error, info};

use crate::session::{SessionContext, SessionUtils};
use crate::utils::format_session_tab_title;

pub struct Kitty<E: CommandExecutor> {
    kitty: E,
}

impl Default for Kitty<KittyExecutor> {
    fn default() -> Self {
        Self::new()
    }
}

impl Kitty<KittyExecutor> {
    pub fn new() -> Self {
        Self {
            kitty: KittyExecutor::new(),
        }
    }
}

impl<E: CommandExecutor> Kitty<E> {
    pub fn with_executor(executor: E) -> Self {
        Self { kitty: executor }
    }

    pub fn match_session_tab(&self, project_name: &str) -> Result<Option<KittyTab>> {
        debug!("Matching session tab for project: {}", project_name);

        // First try matching by tab title with session: prefix
        let session_title_pattern = format!("session:{}", project_name);
        let ls_command_title = KittenLsCommand::new().match_tab_title(&session_title_pattern);

        if let Ok(os_windows) = self.kitty.ls(ls_command_title) {
            for os_window in os_windows {
                if let Some(tab) = os_window.tabs.into_iter().next() {
                    info!(
                        "Found existing session tab for project '{}' with id: {} using tab title matching",
                        project_name, tab.id
                    );
                    return Ok(Some(tab));
                }
            }
        }

        // Fall back to environment variable matching for backward compatibility
        let ls_command_env =
            KittenLsCommand::new().match_tab_env("KITTY_SESSION_PROJECT", project_name);
        let os_windows = self.kitty.ls(ls_command_env)?;

        if os_windows.is_empty() {
            debug!("No matching session found for project: {}", project_name);
            return Ok(None);
        }

        for os_window in os_windows {
            if let Some(tab) = os_window.tabs.into_iter().next() {
                info!(
                    "Found existing session tab for project '{}' with id: {} using environment variable matching",
                    project_name, tab.id
                );
                return Ok(Some(tab));
            }
        }

        debug!(
            "No tabs found in matching windows for project: {}",
            project_name
        );
        Ok(None)
    }

    pub fn focus_tab(&self, tab_id: u32) -> Result<()> {
        info!("Focusing tab with id: {}", tab_id);

        let focus_command = KittenFocusTabCommand::new(tab_id);
        let result = self.kitty.focus_tab(focus_command)?;

        if !result.is_success() {
            let error_msg = result
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            error!("Failed to focus tab {}: {}", tab_id, error_msg);
            return Err(anyhow::anyhow!(
                "Failed to focus tab {}: {}",
                tab_id,
                error_msg
            ));
        }

        info!("Successfully focused tab: {}", tab_id);
        Ok(())
    }

    pub fn create_session_tab_by_path(&self, project_path: &str, project_name: &str) -> Result<()> {
        info!(
            "Creating new session tab for project '{}' at path: {}",
            project_name, project_path
        );

        let session_name = format_session_tab_title(project_name);

        let launch_command = KittenLaunchCommand::new()
            .launch_type("tab")
            .cwd(project_path)
            .env("KITTY_SESSION_PROJECT", project_name)
            .tab_title(&session_name);
        let result = self.kitty.launch(launch_command)?;

        if !result.is_success() {
            let error_msg = result
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            error!(
                "Failed to create session tab for project '{}': {}",
                project_name, error_msg
            );
            return Err(anyhow::anyhow!(
                "Failed to create session tab: {}",
                error_msg
            ));
        }

        info!(
            "Successfully created session tab for project: {}",
            project_name
        );
        Ok(())
    }

    /// Create a new tab that automatically inherits the current session context
    pub fn create_tab_with_session_inheritance(
        &self,
        cwd: Option<&str>,
        tab_title: Option<&str>,
    ) -> Result<()> {
        let session_context = SessionContext::detect();

        if session_context.is_explicit {
            info!(
                "Creating tab with automatic session inheritance for session: {}",
                session_context.name()
            );
        } else {
            info!("Creating tab in unnamed session context");
        }

        let mut launch_command = KittenLaunchCommand::new()
            .launch_type("tab")
            .inherit_current_session();

        if let Some(cwd) = cwd {
            launch_command = launch_command.cwd(cwd);
        }

        if let Some(title) = tab_title {
            launch_command = launch_command.tab_title(title);
        } else if session_context.is_explicit {
            // Auto-generate tab title with session indicator
            let session_title = format_session_tab_title(session_context.name());
            launch_command = launch_command.tab_title(&session_title);
        }

        let result = self.kitty.launch(launch_command)?;

        if !result.is_success() {
            let error_msg = result
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            error!(
                "Failed to create tab with session inheritance: {}",
                error_msg
            );
            return Err(anyhow::anyhow!(
                "Failed to create tab with session inheritance: {}",
                error_msg
            ));
        }

        info!("Successfully created tab with session inheritance");
        Ok(())
    }

    /// Create an unnamed session tab (explicitly without session context)
    pub fn create_unnamed_tab(&self, cwd: Option<&str>, tab_title: Option<&str>) -> Result<()> {
        info!("Creating unnamed tab (no session context)");

        let mut launch_command = KittenLaunchCommand::new().launch_type("tab");

        if let Some(cwd) = cwd {
            launch_command = launch_command.cwd(cwd);
        }

        if let Some(title) = tab_title {
            launch_command = launch_command.tab_title(title);
        }

        let result = self.kitty.launch(launch_command)?;

        if !result.is_success() {
            let error_msg = result
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            error!("Failed to create unnamed tab: {}", error_msg);
            return Err(anyhow::anyhow!(
                "Failed to create unnamed tab: {}",
                error_msg
            ));
        }

        info!("Successfully created unnamed tab");
        Ok(())
    }

    /// Navigate to the next tab within the current session context
    pub fn next_session_tab(&self, allow_wrap: bool) -> Result<()> {
        let session_context = SessionContext::detect();
        self.navigate_session_tab(session_context, TabNavigationDirection::Next, allow_wrap)
    }

    /// Navigate to the previous tab within the current session context
    pub fn prev_session_tab(&self, allow_wrap: bool) -> Result<()> {
        let session_context = SessionContext::detect();
        self.navigate_session_tab(
            session_context,
            TabNavigationDirection::Previous,
            allow_wrap,
        )
    }

    /// Navigate tabs within a specific session
    pub fn navigate_session_tab(
        &self,
        session_context: SessionContext,
        direction: TabNavigationDirection,
        allow_wrap: bool,
    ) -> Result<()> {
        if !session_context.is_explicit && !self.has_session_tabs()? {
            info!("No session context and no session tabs found - navigation skipped");
            return Ok(());
        }

        let session_name = if session_context.is_explicit {
            Some(session_context.name().to_string())
        } else {
            None // Use unnamed session context for navigation
        };

        info!(
            "Navigating {:?} in session '{}'{}",
            direction,
            session_context.name(),
            if allow_wrap { "" } else { " (no-wrap)" }
        );

        let session_name_str = session_name
            .clone()
            .unwrap_or_else(|| session_context.name().to_string());

        let navigate_command = match direction {
            TabNavigationDirection::Next => KittenNavigateTabCommand::next()
                .with_session(&session_name_str)
                .with_wrap(allow_wrap),
            TabNavigationDirection::Previous => KittenNavigateTabCommand::previous()
                .with_session(&session_name_str)
                .with_wrap(allow_wrap),
        };

        let result = self.kitty.navigate_tab(navigate_command)?;

        if !result.is_success() {
            let error_msg = result
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            error!("Failed to navigate tabs: {}", error_msg);
            return Err(anyhow::anyhow!("Failed to navigate tabs: {}", error_msg));
        }

        info!("Successfully navigated to {:?} tab", direction);
        Ok(())
    }

    /// Get all tabs for the current session context
    pub fn get_current_session_tabs(&self) -> Result<Vec<KittyTab>> {
        let session_context = SessionContext::detect();
        self.get_session_tabs(&session_context)
    }

    /// Get all tabs for a specific session
    pub fn get_session_tabs(&self, session_context: &SessionContext) -> Result<Vec<KittyTab>> {
        if session_context.is_explicit {
            let mut session_tabs = Vec::new();

            // First try tab title matching
            let session_title_pattern = format!("session:{}", session_context.name());
            let ls_command_title = KittenLsCommand::new().match_tab_title(&session_title_pattern);

            if let Ok(os_windows) = self.kitty.ls(ls_command_title) {
                for os_window in os_windows {
                    session_tabs.extend(os_window.tabs);
                }
            }

            // Also include tabs matched by environment variable for backward compatibility
            let ls_command_env = KittenLsCommand::new()
                .match_tab_env("KITTY_SESSION_PROJECT", session_context.name());

            if let Ok(os_windows) = self.kitty.ls(ls_command_env) {
                for os_window in os_windows {
                    for tab in os_window.tabs {
                        // Only add if not already included (check by ID)
                        if !session_tabs.iter().any(|existing| existing.id == tab.id) {
                            session_tabs.push(tab);
                        }
                    }
                }
            }

            // Sort by ID to maintain consistent ordering
            session_tabs.sort_by_key(|t| t.id);
            Ok(session_tabs)
        } else {
            // For unnamed session, get all tabs and filter out those with session env vars or tab titles
            let ls_command = KittenLsCommand::new();
            let os_windows = self.kitty.ls(ls_command)?;

            let mut unnamed_tabs = Vec::new();
            for os_window in os_windows {
                for tab in os_window.tabs {
                    let has_session_env = tab
                        .windows
                        .iter()
                        .any(|w| w.env.contains_key("KITTY_SESSION_PROJECT"));
                    let has_session_title = tab.title.starts_with("session:");

                    if !has_session_env && !has_session_title {
                        unnamed_tabs.push(tab);
                    }
                }
            }

            Ok(unnamed_tabs)
        }
    }

    /// Check if there are any session tabs (tabs with KITTY_SESSION_PROJECT set or session: title)
    pub fn has_session_tabs(&self) -> Result<bool> {
        let ls_command = KittenLsCommand::new();
        let os_windows = self.kitty.ls(ls_command)?;

        for os_window in os_windows {
            for tab in os_window.tabs {
                // Check tab title first
                if tab.title.starts_with("session:") {
                    return Ok(true);
                }

                // Check environment variables for backward compatibility
                for window in tab.windows {
                    if window.env.contains_key("KITTY_SESSION_PROJECT") {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Switch to a specific session, focusing the last active tab if available
    pub fn switch_to_session(&self, session_name: &str) -> Result<()> {
        info!("Switching to session: {}", session_name);

        // First check if the session exists
        let session_tabs = self.get_session_tabs(&SessionContext::new(session_name))?;
        if session_tabs.is_empty() {
            return Err(anyhow::anyhow!(
                "Session '{}' not found or has no tabs",
                session_name
            ));
        }

        // Try to get the last active tab for this session
        if let Some(last_active_tab_id) = SessionUtils::get_last_active_tab(session_name) {
            // Verify the tab still exists in the session
            if session_tabs.iter().any(|tab| tab.id == last_active_tab_id) {
                info!(
                    "Focusing last active tab {} in session '{}'",
                    last_active_tab_id, session_name
                );
                return self.focus_tab(last_active_tab_id);
            } else {
                debug!(
                    "Last active tab {} no longer exists in session '{}', focusing first available",
                    last_active_tab_id, session_name
                );
            }
        }

        // Fallback to first tab in the session
        let first_tab = &session_tabs[0];
        info!(
            "Focusing first tab {} in session '{}'",
            first_tab.id, session_name
        );
        self.focus_tab(first_tab.id)?;

        // Update the last active tab tracking
        SessionUtils::set_last_active_tab(session_name, first_tab.id);

        Ok(())
    }

    /// Focus a tab and update last active tracking for its session
    pub fn focus_tab_with_tracking(&self, tab_id: u32) -> Result<()> {
        // Focus the tab first
        self.focus_tab(tab_id)?;

        // Find which session this tab belongs to and update tracking
        let ls_command = KittenLsCommand::new();
        let os_windows = self.kitty.ls(ls_command)?;

        for os_window in os_windows {
            for tab in os_window.tabs {
                if tab.id == tab_id {
                    // Check tab title first for session: prefix
                    if tab.title.starts_with("session:") {
                        if let Some(session_name) =
                            crate::session::SessionContext::parse_session_from_title(&tab.title)
                        {
                            SessionUtils::set_last_active_tab(&session_name, tab_id);
                            debug!(
                                "Updated last active tab tracking: session '{}' -> tab {} (from tab title)",
                                session_name, tab_id
                            );
                            return Ok(());
                        }
                    }

                    // Fall back to checking environment variables
                    for window in &tab.windows {
                        if let Some(session_name) = window.env.get("KITTY_SESSION_PROJECT") {
                            SessionUtils::set_last_active_tab(session_name, tab_id);
                            debug!(
                                "Updated last active tab tracking: session '{}' -> tab {} (from environment)",
                                session_name, tab_id
                            );
                            return Ok(());
                        }
                    }
                    // If no session context found, this is an unnamed session tab
                    debug!("Focused tab {} in unnamed session context", tab_id);
                    return Ok(());
                }
            }
        }

        debug!("Tab {} not found for session tracking update", tab_id);
        Ok(())
    }

    /// Get a list of all available sessions with their tab counts
    pub fn list_sessions(&self) -> Result<Vec<(String, usize)>> {
        let ls_command = KittenLsCommand::new();
        let os_windows = self.kitty.ls(ls_command)?;

        let mut session_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut unnamed_count = 0;

        for os_window in os_windows {
            for tab in os_window.tabs {
                let mut has_session = false;

                // Check tab title first for session: prefix
                if tab.title.starts_with("session:") {
                    if let Some(session_name) =
                        crate::session::SessionContext::parse_session_from_title(&tab.title)
                    {
                        *session_counts.entry(session_name).or_insert(0) += 1;
                        has_session = true;
                    }
                }

                // Fall back to environment variable for backward compatibility
                if !has_session {
                    for window in &tab.windows {
                        if let Some(session_name) = window.env.get("KITTY_SESSION_PROJECT") {
                            *session_counts.entry(session_name.clone()).or_insert(0) += 1;
                            has_session = true;
                            break;
                        }
                    }
                }

                if !has_session {
                    unnamed_count += 1;
                }
            }
        }

        let mut sessions: Vec<(String, usize)> = session_counts.into_iter().collect();
        sessions.sort_by(|a, b| a.0.cmp(&b.0));

        // Add unnamed session if it has tabs
        if unnamed_count > 0 {
            sessions.push(("unnamed".to_string(), unnamed_count));
        }

        Ok(sessions)
    }

    /// Switch to the next available session (cycling through sessions)
    pub fn next_session(&self) -> Result<()> {
        let sessions = self.list_sessions()?;
        if sessions.len() <= 1 {
            debug!("Only one or no sessions available, no switching needed");
            return Ok(());
        }

        let current_session = SessionContext::detect();
        let current_session_name = current_session.name();

        // Find the current session in the list
        let current_index = sessions
            .iter()
            .position(|(name, _)| name == current_session_name);

        let next_index = match current_index {
            Some(idx) => (idx + 1) % sessions.len(),
            None => 0, // If current session not found, go to first
        };

        let (next_session_name, _) = &sessions[next_index];
        info!(
            "Switching from session '{}' to '{}'",
            current_session_name, next_session_name
        );

        self.switch_to_session(next_session_name)
    }

    /// Switch to the previous available session (cycling through sessions)
    pub fn prev_session(&self) -> Result<()> {
        let sessions = self.list_sessions()?;
        if sessions.len() <= 1 {
            debug!("Only one or no sessions available, no switching needed");
            return Ok(());
        }

        let current_session = SessionContext::detect();
        let current_session_name = current_session.name();

        // Find the current session in the list
        let current_index = sessions
            .iter()
            .position(|(name, _)| name == current_session_name);

        let prev_index = match current_index {
            Some(0) => sessions.len() - 1, // Wrap around to last
            Some(idx) => idx - 1,
            None => sessions.len() - 1, // If current session not found, go to last
        };

        let (prev_session_name, _) = &sessions[prev_index];
        info!(
            "Switching from session '{}' to '{}'",
            current_session_name, prev_session_name
        );

        self.switch_to_session(prev_session_name)
    }

    /// Execute a kitty ls command
    pub fn ls(&self, command: KittenLsCommand) -> Result<Vec<kitty_lib::KittyOsWindow>> {
        self.kitty.ls(command)
    }

    /// Close a specific tab by ID
    pub fn close_tab(&self, tab_id: u32) -> Result<kitty_lib::KittyCommandResult<()>> {
        info!("Closing tab with id: {}", tab_id);
        let close_command = KittenCloseTabCommand::new(tab_id);
        let result = self.kitty.close_tab(close_command)?;

        if result.is_success() {
            info!("Successfully closed tab: {}", tab_id);
        } else {
            let default_error = "Unknown error".to_string();
            let error_msg = result.error_message.as_ref().unwrap_or(&default_error);
            error!("Failed to close tab {}: {}", tab_id, error_msg);
        }

        Ok(result)
    }

    /// Set the title of the current tab
    pub fn set_tab_title(&self, title: &str) -> Result<()> {
        info!("Setting tab title to: '{}'", title);
        let command = KittenSetTabTitleCommand::new(title);
        let result = self.kitty.set_tab_title(command)?;

        if !result.is_success() {
            let error_msg = result
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            error!("Failed to set tab title: {}", error_msg);
            return Err(anyhow::anyhow!("Failed to set tab title: {}", error_msg));
        }

        info!("Successfully set tab title");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use kitty_lib::{
        KittyCommandResult, KittyLaunchResponse, KittyOsWindow, KittyTab, KittyWindow, MockExecutor,
    };
    use std::collections::HashMap;

    #[test]
    fn test_kitty_with_mock_executor() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Setup mock response for ls command
        let mock_tab = KittyTab {
            id: 42,
            index: Some(0),
            title: "Test Tab".to_string(),
            windows: vec![KittyWindow {
                id: 1,
                title: "Test Window".to_string(),
                pid: 12345,
                cwd: "/tmp/test".to_string(),
                cmdline: vec!["zsh".to_string()],
                env: HashMap::new(),
                is_self: true,
                state: Some("active".to_string()),
                num: Some(0),
                recent: Some(0),
            }],
            is_active: false,
            is_focused: false,
            state: Some("active".to_string()),
            recent: Some(0),
        };
        let mock_os_window = KittyOsWindow {
            id: 1,
            tabs: vec![mock_tab],
            title: Some("Test OS Window".to_string()),
            state: Some("active".to_string()),
        };
        mock_executor.expect_ls_response(Ok(vec![mock_os_window]));

        // Setup mock response for focus command
        mock_executor.expect_focus_tab_response(Ok(KittyCommandResult::success_empty()));

        let kitty = Kitty::with_executor(&mock_executor);

        // Test match_session_tab
        let result = kitty.match_session_tab("test-project")?;
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 42);

        // Test focus_tab
        kitty.focus_tab(42)?;

        // Verify calls were made
        assert_eq!(mock_executor.ls_call_count(), 1);
        assert_eq!(mock_executor.focus_tab_call_count(), 1);

        // Verify call details
        let ls_calls = mock_executor.get_ls_calls();
        assert_eq!(ls_calls.len(), 1);

        let focus_calls = mock_executor.get_focus_tab_calls();
        assert_eq!(focus_calls.len(), 1);
        assert_eq!(focus_calls[0].tab_id, 42);

        Ok(())
    }

    #[test]
    fn test_kitty_mock_no_matching_tabs() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Setup mock response for ls command with no matching tabs
        mock_executor.expect_ls_response(Ok(Vec::new()));

        let kitty = Kitty::with_executor(&mock_executor);

        // Test match_session_tab with no matches
        let result = kitty.match_session_tab("nonexistent-project")?;
        assert!(result.is_none());

        // Verify calls were made (now makes 2 calls: tab title match + env var match)
        assert_eq!(mock_executor.ls_call_count(), 2);

        Ok(())
    }

    #[test]
    fn test_kitty_mock_create_session() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Setup mock response for launch command
        mock_executor.expect_launch_response(Ok(KittyCommandResult::success(
            KittyLaunchResponse {
                tab_id: None,
                window_id: None,
            },
        )));

        let kitty = Kitty::with_executor(&mock_executor);

        // Test create_session_tab_by_path
        kitty.create_session_tab_by_path("/tmp/test-project", "test-project")?;

        // Verify call was made
        assert_eq!(mock_executor.launch_call_count(), 1);

        // Verify call details
        let launch_calls = mock_executor.get_launch_calls();
        assert_eq!(launch_calls.len(), 1);
        assert_eq!(launch_calls[0].cwd, Some("/tmp/test-project".to_string()));
        assert_eq!(
            launch_calls[0].env,
            Some("KITTY_SESSION_PROJECT=test-project".to_string())
        );
        assert_eq!(
            launch_calls[0].tab_title,
            Some("session:test-project".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_session_aware_navigation() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Set up a session with multiple tabs
        mock_executor.add_session_tab("test-project", Some("Tab 1".to_string()));
        mock_executor.add_session_tab("test-project", Some("Tab 2".to_string()));
        mock_executor.add_session_tab("test-project", Some("Tab 3".to_string()));

        let kitty = Kitty::with_executor(&mock_executor);
        let session_context = SessionContext::new("test-project");

        // Test navigation
        kitty.navigate_session_tab(session_context.clone(), TabNavigationDirection::Next, true)?;
        assert_eq!(mock_executor.get_active_tab_id(), Some(2));

        kitty.navigate_session_tab(
            session_context.clone(),
            TabNavigationDirection::Previous,
            true,
        )?;
        assert_eq!(mock_executor.get_active_tab_id(), Some(1));

        // Test no-wrap behavior
        kitty.navigate_session_tab(session_context, TabNavigationDirection::Previous, false)?;
        assert_eq!(mock_executor.get_active_tab_id(), Some(1)); // Should stay on first tab

        // Verify calls were made
        assert_eq!(mock_executor.navigate_tab_call_count(), 3);

        Ok(())
    }

    #[test]
    fn test_get_session_tabs() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Add tabs to different sessions
        mock_executor.add_session_tab("project1", Some("Project 1 Tab 1".to_string()));
        mock_executor.add_session_tab("project1", Some("Project 1 Tab 2".to_string()));
        mock_executor.add_session_tab("project2", Some("Project 2 Tab".to_string()));

        let kitty = Kitty::with_executor(&mock_executor);

        // Get tabs for project1
        let project1_context = SessionContext::new("project1");
        let project1_tabs = kitty.get_session_tabs(&project1_context)?;
        assert_eq!(project1_tabs.len(), 2);
        assert_eq!(project1_tabs[0].title, "Project 1 Tab 1");
        assert_eq!(project1_tabs[1].title, "Project 1 Tab 2");

        // Get tabs for project2
        let project2_context = SessionContext::new("project2");
        let project2_tabs = kitty.get_session_tabs(&project2_context)?;
        assert_eq!(project2_tabs.len(), 1);
        assert_eq!(project2_tabs[0].title, "Project 2 Tab");

        Ok(())
    }

    #[test]
    fn test_has_session_tabs() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();
        let kitty = Kitty::with_executor(&mock_executor);

        // Initially no session tabs
        assert!(!kitty.has_session_tabs()?);

        // Add a session tab
        mock_executor.add_session_tab("test-project", None);

        // Now should have session tabs
        assert!(kitty.has_session_tabs()?);

        Ok(())
    }

    #[test]
    fn test_get_unnamed_session_tabs() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Add some session tabs and some unnamed tabs
        mock_executor.add_session_tab("project1", Some("Project Tab".to_string()));
        mock_executor.add_unnamed_tab(Some("Unnamed Tab 1".to_string()));
        mock_executor.add_unnamed_tab(Some("Unnamed Tab 2".to_string()));

        let kitty = Kitty::with_executor(&mock_executor);

        // Get unnamed session tabs
        let unnamed_context = SessionContext::unnamed();
        let unnamed_tabs = kitty.get_session_tabs(&unnamed_context)?;

        // Should return only the tabs without KITTY_SESSION_PROJECT environment variable
        assert_eq!(unnamed_tabs.len(), 2);
        assert_eq!(unnamed_tabs[0].title, "Unnamed Tab 1");
        assert_eq!(unnamed_tabs[1].title, "Unnamed Tab 2");

        Ok(())
    }

    #[test]
    fn test_create_tab_with_session_inheritance() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Setup mock response for launch command
        mock_executor.expect_launch_response(Ok(KittyCommandResult::success(
            KittyLaunchResponse {
                tab_id: None,
                window_id: None,
            },
        )));

        let kitty = Kitty::with_executor(&mock_executor);

        // Test creating tab with session inheritance
        kitty.create_tab_with_session_inheritance(Some("/tmp/test"), Some("Test Tab"))?;

        // Verify call was made
        assert_eq!(mock_executor.launch_call_count(), 1);

        // Verify call details
        let launch_calls = mock_executor.get_launch_calls();
        assert_eq!(launch_calls.len(), 1);
        assert_eq!(launch_calls[0].cwd, Some("/tmp/test".to_string()));
        assert_eq!(launch_calls[0].tab_title, Some("Test Tab".to_string()));
        assert!(launch_calls[0].inherit_session); // Should be true

        Ok(())
    }

    #[test]
    fn test_create_unnamed_tab() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Setup mock response for launch command
        mock_executor.expect_launch_response(Ok(KittyCommandResult::success(
            KittyLaunchResponse {
                tab_id: None,
                window_id: None,
            },
        )));

        let kitty = Kitty::with_executor(&mock_executor);

        // Test creating unnamed tab
        kitty.create_unnamed_tab(Some("/tmp/test"), Some("Unnamed Test Tab"))?;

        // Verify call was made
        assert_eq!(mock_executor.launch_call_count(), 1);

        // Verify call details
        let launch_calls = mock_executor.get_launch_calls();
        assert_eq!(launch_calls.len(), 1);
        assert_eq!(launch_calls[0].cwd, Some("/tmp/test".to_string()));
        assert_eq!(
            launch_calls[0].tab_title,
            Some("Unnamed Test Tab".to_string())
        );
        assert!(!launch_calls[0].inherit_session); // Should be false

        Ok(())
    }

    #[test]
    fn test_switch_to_session() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Clear any existing last active tab tracking from other tests
        SessionUtils::clear_last_active_tab("project1");
        SessionUtils::clear_last_active_tab("project2");

        // Add tabs to different sessions
        let _tab1_id =
            mock_executor.add_session_tab("project1", Some("Project 1 Tab 1".to_string()));
        let tab2_id =
            mock_executor.add_session_tab("project1", Some("Project 1 Tab 2".to_string()));
        mock_executor.add_session_tab("project2", Some("Project 2 Tab".to_string()));

        // Set last active tab for project1
        SessionUtils::set_last_active_tab("project1", tab2_id);

        let kitty = Kitty::with_executor(&mock_executor);

        // Test switching to session should focus last active tab
        kitty.switch_to_session("project1")?;

        // Should focus the last active tab (tab2_id)
        assert_eq!(mock_executor.focus_tab_call_count(), 1);
        let focus_calls = mock_executor.get_focus_tab_calls();
        assert_eq!(focus_calls[0].tab_id, tab2_id);

        Ok(())
    }

    #[test]
    fn test_switch_to_session_fallback_to_first() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Clear any existing last active tab tracking for this session from other tests
        SessionUtils::clear_last_active_tab("project1");

        // Add tabs to a session
        let tab1_id =
            mock_executor.add_session_tab("project1", Some("Project 1 Tab 1".to_string()));
        mock_executor.add_session_tab("project1", Some("Project 1 Tab 2".to_string()));

        let kitty = Kitty::with_executor(&mock_executor);

        // Get the count before the operation
        let focus_count_before = mock_executor.focus_tab_call_count();

        // Test switching to session without last active should focus first tab
        kitty.switch_to_session("project1")?;

        // Should have made exactly one additional focus call
        let focus_count_after = mock_executor.focus_tab_call_count();
        assert_eq!(focus_count_after - focus_count_before, 1);

        let focus_calls = mock_executor.get_focus_tab_calls();
        // Check the last focus call (most recent)
        assert_eq!(focus_calls.last().unwrap().tab_id, tab1_id);

        Ok(())
    }

    #[test]
    fn test_focus_tab_with_tracking() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Clear any existing last active tab tracking from other tests
        SessionUtils::clear_last_active_tab("project1");

        // Add a session tab
        let tab_id = mock_executor.add_session_tab("project1", Some("Project 1 Tab".to_string()));

        // Setup mock response for focus command
        mock_executor.expect_focus_tab_response(Ok(KittyCommandResult::success_empty()));

        let kitty = Kitty::with_executor(&mock_executor);

        // Test focusing tab with tracking
        kitty.focus_tab_with_tracking(tab_id)?;

        // Verify focus was called
        assert_eq!(mock_executor.focus_tab_call_count(), 1);

        // Verify last active tab was set
        assert_eq!(SessionUtils::get_last_active_tab("project1"), Some(tab_id));

        Ok(())
    }

    #[test]
    fn test_list_sessions() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Add tabs to different sessions
        mock_executor.add_session_tab("project1", Some("Project 1 Tab 1".to_string()));
        mock_executor.add_session_tab("project1", Some("Project 1 Tab 2".to_string()));
        mock_executor.add_session_tab("project2", Some("Project 2 Tab".to_string()));
        mock_executor.add_unnamed_tab(Some("Unnamed Tab".to_string()));

        let kitty = Kitty::with_executor(&mock_executor);

        // Test listing sessions
        let sessions = kitty.list_sessions()?;

        // Should have 3 sessions: project1 (2 tabs), project2 (1 tab), unnamed (1 tab)
        assert_eq!(sessions.len(), 3);

        // Sessions should be sorted alphabetically
        assert_eq!(sessions[0], ("project1".to_string(), 2));
        assert_eq!(sessions[1], ("project2".to_string(), 1));
        assert_eq!(sessions[2], ("unnamed".to_string(), 1));

        Ok(())
    }

    #[test]
    fn test_next_session() -> Result<()> {
        let mock_executor = MockExecutor::with_default_socket();

        // Add tabs to different sessions
        mock_executor.add_session_tab("alpha", Some("Alpha Tab".to_string()));
        let _beta_tab_id = mock_executor.add_session_tab("beta", Some("Beta Tab".to_string()));
        mock_executor.add_session_tab("gamma", Some("Gamma Tab".to_string()));

        let kitty = Kitty::with_executor(&mock_executor);

        // Note: Since SessionContext::detect() uses real env vars, we can't easily test this
        // without modifying the detection logic. This test would need dependency injection
        // to be fully testable. For now, we'll test the list_sessions functionality.

        let sessions = kitty.list_sessions()?;
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].0, "alpha");
        assert_eq!(sessions[1].0, "beta");
        assert_eq!(sessions[2].0, "gamma");

        Ok(())
    }

    #[test]
    fn test_session_utils_last_active_tracking() {
        // Test setting and getting last active tabs
        SessionUtils::set_last_active_tab("test-session", 42);
        assert_eq!(SessionUtils::get_last_active_tab("test-session"), Some(42));

        // Test updating existing session
        SessionUtils::set_last_active_tab("test-session", 84);
        assert_eq!(SessionUtils::get_last_active_tab("test-session"), Some(84));

        // Test different session
        SessionUtils::set_last_active_tab("other-session", 21);
        assert_eq!(SessionUtils::get_last_active_tab("other-session"), Some(21));
        assert_eq!(SessionUtils::get_last_active_tab("test-session"), Some(84)); // Should not affect

        // Test non-existent session
        assert_eq!(SessionUtils::get_last_active_tab("non-existent"), None);

        // Test clearing
        SessionUtils::clear_last_active_tab("test-session");
        assert_eq!(SessionUtils::get_last_active_tab("test-session"), None);
        assert_eq!(SessionUtils::get_last_active_tab("other-session"), Some(21)); // Should not affect

        // Test tracked sessions
        SessionUtils::set_last_active_tab("session1", 1);
        SessionUtils::set_last_active_tab("session2", 2);
        let tracked = SessionUtils::get_tracked_sessions();
        assert!(tracked.contains(&"session1".to_string()));
        assert!(tracked.contains(&"session2".to_string()));
        assert!(tracked.contains(&"other-session".to_string())); // From previous test
    }
}
