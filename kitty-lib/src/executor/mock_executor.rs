use anyhow::Result;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::commands::close_tab::KittenCloseTabCommand;
use crate::commands::focus_tab::KittenFocusTabCommand;
use crate::commands::launch::KittenLaunchCommand;
use crate::commands::ls::KittenLsCommand;
use crate::commands::navigate_tab::{KittenNavigateTabCommand, TabNavigationDirection};
use crate::executor::CommandExecutor;
use crate::types::{
    KittyCommandResult, KittyLaunchResponse, KittyLsResponse, KittyOsWindow, KittyTab, KittyWindow,
};

/// In-memory state for simulating Kitty's tab/window layout
#[derive(Debug, Clone)]
pub struct MockLayout {
    pub os_windows: Vec<KittyOsWindow>,
    pub active_tab_id: Option<u32>,
    pub next_tab_id: u32,
    pub next_window_id: u32,
    pub next_os_window_id: u32,
}

impl MockLayout {
    pub fn new() -> Self {
        Self {
            os_windows: Vec::new(),
            active_tab_id: None,
            next_tab_id: 1,
            next_window_id: 1,
            next_os_window_id: 1,
        }
    }

    /// Add a tab with the given session context
    pub fn add_tab_with_session(&mut self, session_name: &str, tab_title: Option<String>) -> u32 {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let window_id = self.next_window_id;
        self.next_window_id += 1;

        let mut env = HashMap::new();
        env.insert(
            "KITTY_SESSION_PROJECT".to_string(),
            session_name.to_string(),
        );

        let window = KittyWindow {
            id: window_id,
            title: "shell".to_string(),
            pid: 12345 + window_id,
            cwd: format!("/tmp/{}", session_name),
            cmdline: vec!["zsh".to_string()],
            env,
            is_self: true,
            state: Some("active".to_string()),
            num: Some(0),
            recent: Some(0),
        };

        let tab = KittyTab {
            id: tab_id,
            index: Some(0),
            title: tab_title.unwrap_or_else(|| format!("Tab {}", tab_id)),
            windows: vec![window],
            state: Some("active".to_string()),
            recent: Some(0),
        };

        // Find or create an OS window
        if self.os_windows.is_empty() {
            let os_window_id = self.next_os_window_id;
            self.next_os_window_id += 1;

            let os_window = KittyOsWindow {
                id: os_window_id,
                tabs: vec![tab],
                title: Some("Kitty".to_string()),
                state: Some("active".to_string()),
            };
            self.os_windows.push(os_window);
        } else {
            self.os_windows[0].tabs.push(tab);
        }

        // Set as active if it's the first tab
        if self.active_tab_id.is_none() {
            self.active_tab_id = Some(tab_id);
        }

        tab_id
    }

    /// Add a tab without any session context (no KITTY_SESSION_PROJECT environment variable)
    pub fn add_unnamed_tab(&mut self, tab_title: Option<String>) -> u32 {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let window_id = self.next_window_id;
        self.next_window_id += 1;

        // No session environment variable for unnamed tabs
        let env = HashMap::new();

        let window = KittyWindow {
            id: window_id,
            title: "shell".to_string(),
            pid: 12345 + window_id,
            cwd: "/tmp/default".to_string(),
            cmdline: vec!["zsh".to_string()],
            env,
            is_self: true,
            state: Some("active".to_string()),
            num: Some(0),
            recent: Some(0),
        };

        let tab = KittyTab {
            id: tab_id,
            index: Some(0),
            title: tab_title.unwrap_or_else(|| format!("Unnamed Tab {}", tab_id)),
            windows: vec![window],
            state: Some("active".to_string()),
            recent: Some(0),
        };

        // Find or create the first OS window
        if self.os_windows.is_empty() {
            let os_window_id = self.next_os_window_id;
            self.next_os_window_id += 1;

            let os_window = KittyOsWindow {
                id: os_window_id,
                tabs: vec![tab],
                title: Some("Kitty".to_string()),
                state: Some("active".to_string()),
            };
            self.os_windows.push(os_window);
        } else {
            self.os_windows[0].tabs.push(tab);
        }

        // Set as active if it's the first tab
        if self.active_tab_id.is_none() {
            self.active_tab_id = Some(tab_id);
        }

        tab_id
    }

    /// Set the active tab
    pub fn set_active_tab(&mut self, tab_id: u32) -> bool {
        // Check if the tab exists
        for os_window in &self.os_windows {
            for tab in &os_window.tabs {
                if tab.id == tab_id {
                    self.active_tab_id = Some(tab_id);
                    return true;
                }
            }
        }
        false
    }

    /// Get tabs filtered by session (environment variable)
    pub fn get_tabs_for_session(&self, session_name: &str) -> Vec<KittyTab> {
        let mut matching_tabs = Vec::new();

        for os_window in &self.os_windows {
            for tab in &os_window.tabs {
                // Check if any window in the tab has the matching session environment variable
                for window in &tab.windows {
                    if let Some(env_session) = window.env.get("KITTY_SESSION_PROJECT") {
                        if env_session == session_name {
                            matching_tabs.push(tab.clone());
                            break; // Found matching window in this tab, move to next tab
                        }
                    }
                }
            }
        }

        // Sort by tab ID to maintain consistent ordering
        matching_tabs.sort_by_key(|tab| tab.id);
        matching_tabs
    }

    /// Get all tabs in the current layout
    pub fn get_all_tabs(&self) -> Vec<KittyTab> {
        let mut all_tabs = Vec::new();
        for os_window in &self.os_windows {
            all_tabs.extend(os_window.tabs.clone());
        }
        all_tabs.sort_by_key(|tab| tab.id);
        all_tabs
    }

    /// Navigate to next/previous tab within a session
    pub fn navigate_tab(
        &mut self,
        session_name: &str,
        direction: TabNavigationDirection,
        allow_wrap: bool,
    ) -> Option<u32> {
        let session_tabs = self.get_tabs_for_session(session_name);
        if session_tabs.is_empty() {
            return None;
        }

        // If only one tab, navigation doesn't change anything
        if session_tabs.len() == 1 {
            let tab_id = session_tabs[0].id;
            self.set_active_tab(tab_id);
            return Some(tab_id);
        }

        // Find current active tab in this session
        let current_active = self.active_tab_id?;
        let current_index = session_tabs.iter().position(|t| t.id == current_active);

        // If current tab is not in this session, navigate to first tab
        let current_index = match current_index {
            Some(idx) => idx,
            None => {
                let first_tab_id = session_tabs[0].id;
                self.set_active_tab(first_tab_id);
                return Some(first_tab_id);
            }
        };

        // Calculate next index based on direction
        let next_index = match direction {
            TabNavigationDirection::Next => {
                if current_index + 1 >= session_tabs.len() {
                    if allow_wrap { 0 } else { current_index }
                } else {
                    current_index + 1
                }
            }
            TabNavigationDirection::Previous => {
                if current_index == 0 {
                    if allow_wrap {
                        session_tabs.len() - 1
                    } else {
                        0
                    }
                } else {
                    current_index - 1
                }
            }
        };

        let target_tab_id = session_tabs[next_index].id;
        self.set_active_tab(target_tab_id);
        Some(target_tab_id)
    }

    /// Remove a specific tab by ID
    pub fn remove_tab(&mut self, tab_id: u32) -> bool {
        for os_window in &mut self.os_windows {
            if let Some(pos) = os_window.tabs.iter().position(|t| t.id == tab_id) {
                os_window.tabs.remove(pos);

                // If this was the active tab, clear active tab ID
                if self.active_tab_id == Some(tab_id) {
                    self.active_tab_id = None;
                }

                return true;
            }
        }
        false
    }

    /// Clear all tabs and reset state
    pub fn clear(&mut self) {
        self.os_windows.clear();
        self.active_tab_id = None;
        self.next_tab_id = 1;
        self.next_window_id = 1;
        self.next_os_window_id = 1;
    }
}

impl Default for MockLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct MockExecutor {
    pub ls_calls: RefCell<Vec<KittenLsCommand>>,
    pub focus_tab_calls: RefCell<Vec<KittenFocusTabCommand>>,
    pub close_tab_calls: RefCell<Vec<KittenCloseTabCommand>>,
    pub launch_calls: RefCell<Vec<KittenLaunchCommand>>,
    pub navigate_tab_calls: RefCell<Vec<KittenNavigateTabCommand>>,
    pub ls_responses: RefCell<Vec<Result<KittyLsResponse>>>,
    pub focus_tab_responses: RefCell<Vec<Result<KittyCommandResult<()>>>>,
    pub close_tab_responses: RefCell<Vec<Result<KittyCommandResult<()>>>>,
    pub launch_responses: RefCell<Vec<Result<KittyCommandResult<KittyLaunchResponse>>>>,
    pub navigate_tab_responses: RefCell<Vec<Result<KittyCommandResult<()>>>>,
    pub layout: RefCell<MockLayout>,
}

impl MockExecutor {
    pub fn new() -> Self {
        Self {
            ls_calls: RefCell::new(Vec::new()),
            focus_tab_calls: RefCell::new(Vec::new()),
            close_tab_calls: RefCell::new(Vec::new()),
            launch_calls: RefCell::new(Vec::new()),
            navigate_tab_calls: RefCell::new(Vec::new()),
            ls_responses: RefCell::new(Vec::new()),
            focus_tab_responses: RefCell::new(Vec::new()),
            close_tab_responses: RefCell::new(Vec::new()),
            launch_responses: RefCell::new(Vec::new()),
            navigate_tab_responses: RefCell::new(Vec::new()),
            layout: RefCell::new(MockLayout::new()),
        }
    }

    pub fn with_default_socket() -> Self {
        Self::new()
    }

    pub fn expect_ls_response(&self, response: Result<KittyLsResponse>) {
        self.ls_responses.borrow_mut().push(response);
    }

    pub fn expect_focus_tab_response(&self, response: Result<KittyCommandResult<()>>) {
        self.focus_tab_responses.borrow_mut().push(response);
    }

    pub fn expect_close_tab_response(&self, response: Result<KittyCommandResult<()>>) {
        self.close_tab_responses.borrow_mut().push(response);
    }

    pub fn expect_launch_response(
        &self,
        response: Result<KittyCommandResult<KittyLaunchResponse>>,
    ) {
        self.launch_responses.borrow_mut().push(response);
    }

    pub fn expect_navigate_tab_response(&self, response: Result<KittyCommandResult<()>>) {
        self.navigate_tab_responses.borrow_mut().push(response);
    }

    pub fn ls_call_count(&self) -> usize {
        self.ls_calls.borrow().len()
    }

    pub fn focus_tab_call_count(&self) -> usize {
        self.focus_tab_calls.borrow().len()
    }

    pub fn close_tab_call_count(&self) -> usize {
        self.close_tab_calls.borrow().len()
    }

    pub fn launch_call_count(&self) -> usize {
        self.launch_calls.borrow().len()
    }

    pub fn navigate_tab_call_count(&self) -> usize {
        self.navigate_tab_calls.borrow().len()
    }

    pub fn get_ls_calls(&self) -> Vec<KittenLsCommand> {
        self.ls_calls.borrow().clone()
    }

    pub fn get_focus_tab_calls(&self) -> Vec<KittenFocusTabCommand> {
        self.focus_tab_calls.borrow().clone()
    }

    pub fn get_close_tab_calls(&self) -> Vec<KittenCloseTabCommand> {
        self.close_tab_calls.borrow().clone()
    }

    pub fn get_launch_calls(&self) -> Vec<KittenLaunchCommand> {
        self.launch_calls.borrow().clone()
    }

    pub fn get_navigate_tab_calls(&self) -> Vec<KittenNavigateTabCommand> {
        self.navigate_tab_calls.borrow().clone()
    }

    /// Layout management methods
    pub fn add_session_tab(&self, session_name: &str, tab_title: Option<String>) -> u32 {
        self.layout
            .borrow_mut()
            .add_tab_with_session(session_name, tab_title)
    }

    pub fn add_unnamed_tab(&self, tab_title: Option<String>) -> u32 {
        self.layout.borrow_mut().add_unnamed_tab(tab_title)
    }

    pub fn set_active_tab(&self, tab_id: u32) -> bool {
        self.layout.borrow_mut().set_active_tab(tab_id)
    }

    pub fn get_tabs_for_session(&self, session_name: &str) -> Vec<KittyTab> {
        self.layout.borrow().get_tabs_for_session(session_name)
    }

    pub fn get_all_tabs(&self) -> Vec<KittyTab> {
        self.layout.borrow().get_all_tabs()
    }

    pub fn clear_layout(&self) {
        self.layout.borrow_mut().clear()
    }

    pub fn get_active_tab_id(&self) -> Option<u32> {
        self.layout.borrow().active_tab_id
    }

    pub fn remove_tab(&self, tab_id: u32) -> bool {
        self.layout.borrow_mut().remove_tab(tab_id)
    }

    /// Navigate tabs within a session using the internal layout
    pub fn navigate_session_tab(
        &self,
        session_name: &str,
        direction: TabNavigationDirection,
        allow_wrap: bool,
    ) -> Option<u32> {
        self.layout
            .borrow_mut()
            .navigate_tab(session_name, direction, allow_wrap)
    }

    /// Enable smart behavior where the MockExecutor uses its internal layout
    /// to generate responses automatically when no explicit responses are queued
    pub fn enable_smart_responses(&self) {
        // This is a flag we can use to modify behavior in the CommandExecutor implementation
        // For now, the smart behavior is always enabled
    }
}

impl CommandExecutor for &MockExecutor {
    fn ls(&self, command: KittenLsCommand) -> Result<KittyLsResponse> {
        self.ls_calls.borrow_mut().push(command.clone());

        // If there's a queued response, use it
        if let Some(response) = self.ls_responses.borrow_mut().pop() {
            return response;
        }

        // Smart response using internal layout
        let layout = self.layout.borrow();
        let mut result_os_windows = Vec::new();

        // Check if we need to filter by environment variable
        if let Some(match_arg) = &command.match_arg {
            if match_arg.starts_with("env:KITTY_SESSION_PROJECT=") {
                let session_name = match_arg.trim_start_matches("env:KITTY_SESSION_PROJECT=");
                let matching_tabs = layout.get_tabs_for_session(session_name);

                if !matching_tabs.is_empty() {
                    // Create OS window containing the matching tabs
                    let os_window = KittyOsWindow {
                        id: 1,
                        tabs: matching_tabs,
                        title: Some("Kitty".to_string()),
                        state: Some("active".to_string()),
                    };
                    result_os_windows.push(os_window);
                }
            }
        } else {
            // Return all OS windows if no filter
            result_os_windows = layout.os_windows.clone();
        }

        Ok(result_os_windows)
    }

    fn focus_tab(&self, command: KittenFocusTabCommand) -> Result<KittyCommandResult<()>> {
        self.focus_tab_calls.borrow_mut().push(command.clone());

        // If there's a queued response, use it
        if let Some(response) = self.focus_tab_responses.borrow_mut().pop() {
            return response;
        }

        // Smart response: check if the tab exists in our layout
        let success = self.set_active_tab(command.tab_id);

        if success {
            Ok(KittyCommandResult::success_empty())
        } else {
            Ok(KittyCommandResult::error(format!(
                "Tab {} not found",
                command.tab_id
            )))
        }
    }

    fn close_tab(&self, command: KittenCloseTabCommand) -> Result<KittyCommandResult<()>> {
        self.close_tab_calls.borrow_mut().push(command.clone());

        // If there's a queued response, use it
        if let Some(response) = self.close_tab_responses.borrow_mut().pop() {
            return response;
        }

        // Smart response: check if the tab exists and remove it from our layout
        let success = self.remove_tab(command.tab_id);

        if success {
            Ok(KittyCommandResult::success_empty())
        } else {
            Ok(KittyCommandResult::error(format!(
                "Tab {} not found",
                command.tab_id
            )))
        }
    }

    fn launch(
        &self,
        command: KittenLaunchCommand,
    ) -> Result<KittyCommandResult<KittyLaunchResponse>> {
        self.launch_calls.borrow_mut().push(command.clone());

        // If there's a queued response, use it
        if let Some(response) = self.launch_responses.borrow_mut().pop() {
            return response;
        }

        // Smart response: actually create the tab in our layout
        if command.launch_type == "tab" {
            // Extract session name from environment variable if present
            let session_name = if let Some(env) = &command.env {
                if env.starts_with("KITTY_SESSION_PROJECT=") {
                    env.trim_start_matches("KITTY_SESSION_PROJECT=")
                } else {
                    "unnamed" // Default session
                }
            } else {
                "unnamed" // Default session
            };

            let tab_id = self.add_session_tab(session_name, command.tab_title);

            Ok(KittyCommandResult::success(KittyLaunchResponse {
                tab_id: Some(tab_id),
                window_id: None, // We don't track individual window IDs in launches for simplicity
            }))
        } else {
            // For non-tab launches, return basic success
            Ok(KittyCommandResult::success(KittyLaunchResponse {
                tab_id: None,
                window_id: None,
            }))
        }
    }

    fn navigate_tab(&self, command: KittenNavigateTabCommand) -> Result<KittyCommandResult<()>> {
        self.navigate_tab_calls.borrow_mut().push(command.clone());

        // If there's a queued response, use it
        if let Some(response) = self.navigate_tab_responses.borrow_mut().pop() {
            return response;
        }

        // Smart response: use the internal layout to navigate
        let session_name = command.session_name.as_deref().unwrap_or("unnamed");
        let result = self.navigate_session_tab(session_name, command.direction, command.allow_wrap);

        match result {
            Some(_target_tab_id) => Ok(KittyCommandResult::success_empty()),
            None => Ok(KittyCommandResult::error(format!(
                "No tabs found in session '{}' for navigation",
                session_name
            ))),
        }
    }
}

impl Default for MockExecutor {
    fn default() -> Self {
        Self::with_default_socket()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::focus_tab::KittenFocusTabCommand;
    use crate::commands::launch::KittenLaunchCommand;
    use crate::commands::ls::KittenLsCommand;
    use crate::executor::CommandExecutor;

    #[test]
    fn test_mock_layout_add_tab_with_session() {
        let mut layout = MockLayout::new();

        let tab_id = layout.add_tab_with_session("test-project", Some("Test Tab".to_string()));
        assert_eq!(tab_id, 1);
        assert_eq!(layout.active_tab_id, Some(1));

        let tabs = layout.get_tabs_for_session("test-project");
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].id, 1);
        assert_eq!(tabs[0].title, "Test Tab");

        // Check environment variable
        assert_eq!(
            tabs[0].windows[0].env.get("KITTY_SESSION_PROJECT"),
            Some(&"test-project".to_string())
        );
    }

    #[test]
    fn test_mock_layout_multiple_sessions() {
        let mut layout = MockLayout::new();

        let tab1 = layout.add_tab_with_session("project1", None);
        let tab2 = layout.add_tab_with_session("project2", None);
        let tab3 = layout.add_tab_with_session("project1", None);

        assert_eq!(tab1, 1);
        assert_eq!(tab2, 2);
        assert_eq!(tab3, 3);

        let project1_tabs = layout.get_tabs_for_session("project1");
        assert_eq!(project1_tabs.len(), 2);
        assert_eq!(project1_tabs[0].id, 1);
        assert_eq!(project1_tabs[1].id, 3);

        let project2_tabs = layout.get_tabs_for_session("project2");
        assert_eq!(project2_tabs.len(), 1);
        assert_eq!(project2_tabs[0].id, 2);
    }

    #[test]
    fn test_mock_layout_set_active_tab() {
        let mut layout = MockLayout::new();

        let _tab1 = layout.add_tab_with_session("project1", None);
        let tab2 = layout.add_tab_with_session("project2", None);

        assert_eq!(layout.active_tab_id, Some(1)); // First tab is active

        assert!(layout.set_active_tab(tab2));
        assert_eq!(layout.active_tab_id, Some(2));

        assert!(!layout.set_active_tab(999)); // Non-existent tab
        assert_eq!(layout.active_tab_id, Some(2)); // Should remain unchanged
    }

    #[test]
    fn test_mock_executor_smart_ls_responses() {
        let executor = MockExecutor::new();

        // Add some tabs to different sessions
        executor.add_session_tab("project1", Some("Project 1 Tab".to_string()));
        executor.add_session_tab("project2", Some("Project 2 Tab".to_string()));
        executor.add_session_tab("project1", Some("Project 1 Tab 2".to_string()));

        // Test filtering by session
        let ls_command = KittenLsCommand::new().match_env("KITTY_SESSION_PROJECT", "project1");
        let response = (&executor).ls(ls_command).unwrap();

        assert_eq!(response.len(), 1); // One OS window
        assert_eq!(response[0].tabs.len(), 2); // Two tabs for project1
        assert_eq!(response[0].tabs[0].title, "Project 1 Tab");
        assert_eq!(response[0].tabs[1].title, "Project 1 Tab 2");

        // Test filtering by different session
        let ls_command = KittenLsCommand::new().match_env("KITTY_SESSION_PROJECT", "project2");
        let response = (&executor).ls(ls_command).unwrap();

        assert_eq!(response.len(), 1); // One OS window
        assert_eq!(response[0].tabs.len(), 1); // One tab for project2
        assert_eq!(response[0].tabs[0].title, "Project 2 Tab");

        // Test filtering by non-existent session
        let ls_command = KittenLsCommand::new().match_env("KITTY_SESSION_PROJECT", "nonexistent");
        let response = (&executor).ls(ls_command).unwrap();

        assert_eq!(response.len(), 0); // No matching tabs
    }

    #[test]
    fn test_mock_executor_smart_focus_tab_responses() {
        let executor = MockExecutor::new();

        let tab_id = executor.add_session_tab("project1", None);

        // Test focusing existing tab
        let focus_command = KittenFocusTabCommand::new(tab_id);
        let response = (&executor).focus_tab(focus_command).unwrap();

        assert!(response.is_success());
        assert_eq!(executor.get_active_tab_id(), Some(tab_id));

        // Test focusing non-existent tab
        let focus_command = KittenFocusTabCommand::new(999);
        let response = (&executor).focus_tab(focus_command).unwrap();

        assert!(!response.is_success());
        assert!(response.error_message.is_some());
        assert!(
            response
                .error_message
                .unwrap()
                .contains("Tab 999 not found")
        );
    }

    #[test]
    fn test_mock_executor_smart_launch_responses() {
        let executor = MockExecutor::new();

        // Test launching a tab with session
        let launch_command = KittenLaunchCommand::new()
            .launch_type("tab")
            .env("KITTY_SESSION_PROJECT", "test-project")
            .tab_title("Test Tab");

        let response = (&executor).launch(launch_command).unwrap();

        assert!(response.is_success());
        assert!(response.data.is_some());
        let launch_response = response.data.unwrap();
        assert!(launch_response.tab_id.is_some());

        // Verify the tab was actually added to the layout
        let tabs = executor.get_tabs_for_session("test-project");
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].title, "Test Tab");
    }

    #[test]
    fn test_mock_executor_queued_responses_override_smart_responses() {
        let executor = MockExecutor::new();

        // Add a tab to the layout
        executor.add_session_tab("project1", None);

        // Queue a custom response
        let custom_response = Ok(vec![KittyOsWindow {
            id: 999,
            tabs: vec![KittyTab {
                id: 999,
                index: Some(0),
                title: "Custom Tab".to_string(),
                windows: vec![],
                state: Some("active".to_string()),
                recent: Some(0),
            }],
            title: Some("Custom Window".to_string()),
            state: Some("active".to_string()),
        }]);
        executor.expect_ls_response(custom_response);

        // The queued response should be used instead of smart response
        let ls_command = KittenLsCommand::new();
        let response = (&executor).ls(ls_command).unwrap();

        assert_eq!(response.len(), 1);
        assert_eq!(response[0].id, 999);
        assert_eq!(response[0].tabs[0].title, "Custom Tab");
    }

    #[test]
    fn test_mock_executor_layout_utilities() {
        let executor = MockExecutor::new();

        // Test empty layout
        assert_eq!(executor.get_all_tabs().len(), 0);
        assert_eq!(executor.get_active_tab_id(), None);

        // Add some tabs
        let tab1 = executor.add_session_tab("project1", None);
        let tab2 = executor.add_session_tab("project2", None);

        assert_eq!(executor.get_all_tabs().len(), 2);
        assert_eq!(executor.get_active_tab_id(), Some(tab1)); // First tab is active

        // Change active tab
        assert!(executor.set_active_tab(tab2));
        assert_eq!(executor.get_active_tab_id(), Some(tab2));

        // Clear layout
        executor.clear_layout();
        assert_eq!(executor.get_all_tabs().len(), 0);
        assert_eq!(executor.get_active_tab_id(), None);
    }

    #[test]
    fn test_mock_layout_navigation() {
        let mut layout = MockLayout::new();

        // Add multiple tabs to the same session
        let tab1 = layout.add_tab_with_session("project1", Some("Tab 1".to_string()));
        let tab2 = layout.add_tab_with_session("project1", Some("Tab 2".to_string()));
        let tab3 = layout.add_tab_with_session("project1", Some("Tab 3".to_string()));

        // Tab 1 should be active initially
        assert_eq!(layout.active_tab_id, Some(tab1));

        // Navigate next with wrap
        let result = layout.navigate_tab("project1", TabNavigationDirection::Next, true);
        assert_eq!(result, Some(tab2));
        assert_eq!(layout.active_tab_id, Some(tab2));

        // Navigate next again
        let result = layout.navigate_tab("project1", TabNavigationDirection::Next, true);
        assert_eq!(result, Some(tab3));
        assert_eq!(layout.active_tab_id, Some(tab3));

        // Navigate next from last tab (should wrap to first)
        let result = layout.navigate_tab("project1", TabNavigationDirection::Next, true);
        assert_eq!(result, Some(tab1));
        assert_eq!(layout.active_tab_id, Some(tab1));

        // Navigate previous with wrap (should go to last)
        let result = layout.navigate_tab("project1", TabNavigationDirection::Previous, true);
        assert_eq!(result, Some(tab3));
        assert_eq!(layout.active_tab_id, Some(tab3));

        // Test no-wrap behavior
        let result = layout.navigate_tab("project1", TabNavigationDirection::Next, false);
        assert_eq!(result, Some(tab3)); // Should stay on last tab
        assert_eq!(layout.active_tab_id, Some(tab3));

        // Go to first tab and test no-wrap previous
        layout.set_active_tab(tab1);
        let result = layout.navigate_tab("project1", TabNavigationDirection::Previous, false);
        assert_eq!(result, Some(tab1)); // Should stay on first tab
        assert_eq!(layout.active_tab_id, Some(tab1));
    }

    #[test]
    fn test_mock_layout_navigation_single_tab() {
        let mut layout = MockLayout::new();

        let tab1 = layout.add_tab_with_session("project1", None);

        // Navigation with single tab should be no-op
        let result = layout.navigate_tab("project1", TabNavigationDirection::Next, true);
        assert_eq!(result, Some(tab1));
        assert_eq!(layout.active_tab_id, Some(tab1));

        let result = layout.navigate_tab("project1", TabNavigationDirection::Previous, true);
        assert_eq!(result, Some(tab1));
        assert_eq!(layout.active_tab_id, Some(tab1));
    }

    #[test]
    fn test_mock_layout_navigation_empty_session() {
        let mut layout = MockLayout::new();

        // Navigation in empty session should return None
        let result = layout.navigate_tab("nonexistent", TabNavigationDirection::Next, true);
        assert_eq!(result, None);
    }

    #[test]
    fn test_mock_executor_navigate_tab_command() {
        let executor = MockExecutor::new();

        // Add tabs to a session
        executor.add_session_tab("project1", Some("Tab 1".to_string()));
        executor.add_session_tab("project1", Some("Tab 2".to_string()));
        executor.add_session_tab("project1", Some("Tab 3".to_string()));

        // Test next navigation
        let nav_command = KittenNavigateTabCommand::next().with_session("project1");
        let response = (&executor).navigate_tab(nav_command).unwrap();
        assert!(response.is_success());
        assert_eq!(executor.get_active_tab_id(), Some(2)); // Should move to second tab

        // Test previous navigation
        let nav_command = KittenNavigateTabCommand::previous().with_session("project1");
        let response = (&executor).navigate_tab(nav_command).unwrap();
        assert!(response.is_success());
        assert_eq!(executor.get_active_tab_id(), Some(1)); // Should move back to first tab

        // Test no-wrap behavior at boundary
        let nav_command = KittenNavigateTabCommand::previous()
            .with_session("project1")
            .no_wrap();
        let response = (&executor).navigate_tab(nav_command).unwrap();
        assert!(response.is_success());
        assert_eq!(executor.get_active_tab_id(), Some(1)); // Should stay on first tab

        // Verify call tracking
        assert_eq!(executor.navigate_tab_call_count(), 3);
        let calls = executor.get_navigate_tab_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].direction, TabNavigationDirection::Next);
        assert_eq!(calls[1].direction, TabNavigationDirection::Previous);
        assert_eq!(calls[2].direction, TabNavigationDirection::Previous);
        assert!(!calls[2].allow_wrap);
    }

    #[test]
    fn test_mock_executor_navigate_tab_empty_session() {
        let executor = MockExecutor::new();

        let nav_command = KittenNavigateTabCommand::next().with_session("nonexistent");
        let response = (&executor).navigate_tab(nav_command).unwrap();

        assert!(!response.is_success());
        assert!(response.error_message.is_some());
        assert!(response.error_message.unwrap().contains("No tabs found"));
    }

    #[test]
    fn test_mock_executor_navigate_tab_queued_response() {
        let executor = MockExecutor::new();

        executor.add_session_tab("project1", None);

        // Queue a custom error response
        executor.expect_navigate_tab_response(Ok(KittyCommandResult::error("Custom error")));

        let nav_command = KittenNavigateTabCommand::next().with_session("project1");
        let response = (&executor).navigate_tab(nav_command).unwrap();

        assert!(!response.is_success());
        assert_eq!(response.error_message, Some("Custom error".to_string()));
    }
}
