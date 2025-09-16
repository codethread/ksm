use anyhow::Result;
use log::{debug, error};
use std::env;
use std::process::Command;

use crate::commands::close_tab::KittenCloseTabCommand;
use crate::commands::focus_tab::KittenFocusTabCommand;
use crate::commands::launch::KittenLaunchCommand;
use crate::commands::ls::KittenLsCommand;
use crate::commands::navigate_tab::{KittenNavigateTabCommand, TabNavigationDirection};
use crate::commands::set_tab_title::KittenSetTabTitleCommand;
use crate::executor::CommandExecutor;
use crate::types::{KittyCommandResult, KittyLaunchResponse, KittyLsResponse};
use crate::utils::get_kitty_socket;

pub struct KittyExecutor {
    socket: String,
}

impl KittyExecutor {
    pub fn new() -> Self {
        let socket = get_kitty_socket();
        Self { socket }
    }
}

impl Default for KittyExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor for KittyExecutor {
    fn ls(&self, command: KittenLsCommand) -> Result<KittyLsResponse> {
        let socket_arg = format!("--to={}", self.socket);
        let mut args = vec!["@", &socket_arg, "ls"];

        let match_formatted;
        if let Some(match_arg) = &command.match_arg {
            let match_flag = if command.use_tab_match {
                "--match-tab"
            } else {
                "--match"
            };
            match_formatted = format!("{}={}", match_flag, match_arg);
            args.push(&match_formatted);
            debug!(
                "Running kitten @ --to={} ls {}={}",
                self.socket, match_flag, match_arg
            );
        } else {
            debug!("Running kitten @ --to={} ls", self.socket);
        }

        let output = Command::new("kitten").args(&args).output()?;

        if !output.status.success() {
            debug!("kitten ls command failed with non-zero exit status");
            return Ok(Vec::new());
        }

        let response: KittyLsResponse = serde_json::from_slice(&output.stdout).map_err(|e| {
            error!("Failed to parse kitten ls output: {}", e);
            e
        })?;

        Ok(response)
    }

    fn focus_tab(&self, command: KittenFocusTabCommand) -> Result<KittyCommandResult<()>> {
        let socket_arg = format!("--to={}", self.socket);
        let match_arg = format!("--match=id:{}", command.tab_id);
        let args = ["@", &socket_arg, "focus-tab", &match_arg];

        debug!(
            "Running kitten @ --to={} focus-tab --match=id:{}",
            self.socket, command.tab_id
        );

        let status = Command::new("kitten").args(args).status()?;

        if status.success() {
            Ok(KittyCommandResult::success_empty())
        } else {
            Ok(KittyCommandResult::error(format!(
                "Failed to focus tab {}",
                command.tab_id
            )))
        }
    }

    fn close_tab(&self, command: KittenCloseTabCommand) -> Result<KittyCommandResult<()>> {
        let socket_arg = format!("--to={}", self.socket);
        let match_arg = format!("--match=id:{}", command.tab_id);
        let args = ["@", &socket_arg, "close-tab", &match_arg];

        debug!(
            "Running kitten @ --to={} close-tab --match=id:{}",
            self.socket, command.tab_id
        );

        let status = Command::new("kitten").args(args).status()?;

        if status.success() {
            Ok(KittyCommandResult::success_empty())
        } else {
            Ok(KittyCommandResult::error(format!(
                "Failed to close tab {}",
                command.tab_id
            )))
        }
    }

    fn launch(
        &self,
        command: KittenLaunchCommand,
    ) -> Result<KittyCommandResult<KittyLaunchResponse>> {
        let socket_arg = format!("--to={}", self.socket);
        let type_arg = format!("--type={}", command.launch_type);
        let mut args = vec!["@", &socket_arg, "launch", &type_arg];

        let cwd_formatted;
        if let Some(cwd) = &command.cwd {
            cwd_formatted = format!("--cwd={}", cwd);
            args.push(&cwd_formatted);
        }

        let env_formatted;
        let mut effective_env = command.env.clone();

        // Handle session inheritance
        if command.inherit_session {
            if let Ok(session_project) = env::var("KITTY_SESSION_PROJECT") {
                if !session_project.is_empty() {
                    // Check if env is already set, if not add the session
                    if effective_env.is_none() {
                        effective_env = Some(format!("KITTY_SESSION_PROJECT={}", session_project));
                    } else if let Some(ref current_env) = effective_env {
                        // If env is set but doesn't contain session, append it
                        if !current_env.contains("KITTY_SESSION_PROJECT=") {
                            effective_env = Some(format!(
                                "{},KITTY_SESSION_PROJECT={}",
                                current_env, session_project
                            ));
                        }
                    }
                }
            }
        }

        if let Some(env) = &effective_env {
            env_formatted = format!("--env={}", env);
            args.push(&env_formatted);
        }

        if let Some(tab_title) = &command.tab_title {
            args.push("--tab-title");
            args.push(tab_title);
        }

        debug!(
            "Running kitten @ --to={} launch --type={} {}{}{}{}",
            self.socket,
            command.launch_type,
            command
                .cwd
                .as_ref()
                .map(|c| format!("--cwd={} ", c))
                .unwrap_or_default(),
            effective_env
                .as_ref()
                .map(|e| format!("--env={} ", e))
                .unwrap_or_default(),
            command
                .tab_title
                .as_ref()
                .map(|t| format!("--tab-title={} ", t))
                .unwrap_or_default(),
            if command.inherit_session {
                "(inherit_session)"
            } else {
                ""
            }
        );

        let status = Command::new("kitten").args(&args).status()?;

        if status.success() {
            // Note: kitten launch doesn't typically return the ID of the created tab/window
            // This would need to be enhanced if we need the actual IDs
            Ok(KittyCommandResult::success(KittyLaunchResponse {
                tab_id: None,
                window_id: None,
            }))
        } else {
            Ok(KittyCommandResult::error("Failed to launch tab"))
        }
    }

    fn navigate_tab(&self, command: KittenNavigateTabCommand) -> Result<KittyCommandResult<()>> {
        let session_name = command.session_name.as_deref().unwrap_or("unnamed");

        // Get all tabs for the session
        let mut session_tabs = Vec::new();

        if session_name != "unnamed" {
            // First try tab title matching for named sessions
            let session_title_pattern = format!("session:{}", session_name);
            let ls_command_title = KittenLsCommand::new().match_tab_title(&session_title_pattern);

            if let Ok(os_windows) = self.ls(ls_command_title) {
                for os_window in os_windows {
                    session_tabs.extend(os_window.tabs);
                }
            }

            // Also include tabs matched by environment variable for backward compatibility
            let ls_command_env =
                KittenLsCommand::new().match_tab_env("KITTY_SESSION_PROJECT", session_name);

            if let Ok(os_windows) = self.ls(ls_command_env) {
                for os_window in os_windows {
                    for tab in os_window.tabs {
                        // Only add if not already included (check by ID)
                        if !session_tabs.iter().any(|existing| existing.id == tab.id) {
                            session_tabs.push(tab);
                        }
                    }
                }
            }
        } else {
            // For unnamed session, get all tabs and filter out those with session env var or session title
            let ls_command = KittenLsCommand::new();
            let os_windows = self.ls(ls_command)?;

            for os_window in os_windows {
                for tab in os_window.tabs {
                    let has_session_env = tab
                        .windows
                        .iter()
                        .any(|w| w.env.contains_key("KITTY_SESSION_PROJECT"));
                    let has_session_title = tab.title.starts_with("session:");
                    if !has_session_env && !has_session_title {
                        session_tabs.push(tab);
                    }
                }
            }
        }

        // Sort tabs by ID to maintain consistent order
        session_tabs.sort_by_key(|t| t.id);

        if session_tabs.is_empty() {
            return Ok(KittyCommandResult::error(format!(
                "No tabs found in session '{}'",
                session_name
            )));
        }

        if session_tabs.len() == 1 {
            // Only one tab, nothing to navigate to
            return Ok(KittyCommandResult::success_empty());
        }

        // Sort tabs by ID to maintain consistent order
        session_tabs.sort_by_key(|t| t.id);

        // Find the currently active tab
        let current_active = session_tabs.iter().position(|t| t.is_active);

        let current_index = current_active.unwrap_or(0);

        // Calculate next index based on direction
        let next_index = match command.direction {
            TabNavigationDirection::Next => {
                if current_index + 1 >= session_tabs.len() {
                    if command.allow_wrap { 0 } else { current_index }
                } else {
                    current_index + 1
                }
            }
            TabNavigationDirection::Previous => {
                if current_index == 0 {
                    if command.allow_wrap {
                        session_tabs.len() - 1
                    } else {
                        0
                    }
                } else {
                    current_index - 1
                }
            }
        };

        // If no change needed due to no-wrap
        if next_index == current_index {
            return Ok(KittyCommandResult::success_empty());
        }

        let target_tab_id = session_tabs[next_index].id;

        debug!(
            "Navigating {:?} in session '{}' from tab {} to tab {}",
            command.direction, session_name, session_tabs[current_index].id, target_tab_id
        );

        // Focus the target tab
        let focus_command = KittenFocusTabCommand::new(target_tab_id);
        self.focus_tab(focus_command)
    }

    fn set_tab_title(&self, command: KittenSetTabTitleCommand) -> Result<KittyCommandResult<()>> {
        let socket_arg = format!("--to={}", self.socket);
        let mut args = vec!["@", &socket_arg, "set-tab-title"];

        let match_formatted;
        if let Some(match_pattern) = &command.match_pattern {
            match_formatted = format!("--match={}", match_pattern);
            args.push(&match_formatted);
            debug!(
                "Running kitten @ --to={} set-tab-title --match={} '{}'",
                self.socket, match_pattern, command.title
            );
        } else {
            debug!(
                "Running kitten @ --to={} set-tab-title '{}'",
                self.socket, command.title
            );
        }

        args.push(&command.title);

        let status = Command::new("kitten").args(args).status()?;

        if status.success() {
            Ok(KittyCommandResult::success_empty())
        } else {
            Ok(KittyCommandResult::error("Failed to set tab title"))
        }
    }
}
