use anyhow::Result;
use log::{debug, error};
use std::process::Command;

use crate::commands::focus_tab::KittenFocusTabCommand;
use crate::commands::launch::KittenLaunchCommand;
use crate::commands::ls::KittenLsCommand;
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
            match_formatted = format!("--match={}", match_arg);
            args.push(&match_formatted);
            debug!(
                "Running kitten @ --to={} ls --match={}",
                self.socket, match_arg
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
        if let Some(env) = &command.env {
            env_formatted = format!("--env={}", env);
            args.push(&env_formatted);
        }

        if let Some(tab_title) = &command.tab_title {
            args.push("--tab-title");
            args.push(tab_title);
        }

        debug!(
            "Running kitten @ --to={} launch --type={} {}{}{}",
            self.socket,
            command.launch_type,
            command
                .cwd
                .as_ref()
                .map(|c| format!("--cwd={} ", c))
                .unwrap_or_default(),
            command
                .env
                .as_ref()
                .map(|e| format!("--env={} ", e))
                .unwrap_or_default(),
            command
                .tab_title
                .as_ref()
                .map(|t| format!("--tab-title={}", t))
                .unwrap_or_default()
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
}
