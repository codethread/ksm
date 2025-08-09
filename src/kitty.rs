use crate::kitty_lib::{KittenFocusTabCommand, KittenLaunchCommand, KittenLsCommand};
use anyhow::Result;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::env;
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct KittyTab {
    pub id: u32,
}

#[derive(Debug, Deserialize)]
pub struct KittyWindow {
    pub tabs: Vec<KittyTab>,
}

fn get_kitty_socket() -> String {
    if let Ok(socket) = env::var("KITTY_LISTEN_ON") {
        debug!("Using KITTY_LISTEN_ON environment variable: {socket}");
        return socket;
    }

    debug!("KITTY_LISTEN_ON not set, searching for socket files");

    // Find socket file
    if let Ok(output) = Command::new("sh")
        .arg("-c")
        .arg("ls /tmp/mykitty* 2>/dev/null | head -1")
        .output()
    {
        if let Ok(socket_file) = String::from_utf8(output.stdout) {
            let socket_file = socket_file.trim();
            if !socket_file.is_empty() {
                let socket_path = format!("unix:{}", socket_file);
                debug!("Found socket file: {}", socket_path);
                return socket_path;
            }
        }
    }

    let default_socket = "unix:/tmp/mykitty".to_string();
    warn!("No socket file found, using default: {}", default_socket);
    default_socket
}

pub struct Kitty {
    socket: String,
}

impl Kitty {
    pub fn new() -> Self {
        let socket = get_kitty_socket();
        Self { socket }
    }

    pub fn socket(&self) -> &str {
        &self.socket
    }

    pub fn match_session_tab(&self, project_name: &str) -> Result<Option<KittyTab>> {
        debug!("Matching session tab for project: {}", project_name);

        let output = KittenLsCommand::new(self.socket.clone())
            .match_env("KITTY_SESSION_PROJECT", project_name)
            .execute()?;

        if !output.status.success() {
            debug!("No matching session found for project: {}", project_name);
            return Ok(None);
        }

        let windows: Vec<KittyWindow> = serde_json::from_slice(&output.stdout).map_err(|e| {
            error!("Failed to parse kitten ls output: {}", e);
            e
        })?;

        for window in windows {
            if let Some(tab) = window.tabs.into_iter().next() {
                info!(
                    "Found existing session tab for project '{}' with id: {}",
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
        use anyhow::anyhow;

        info!("Focusing tab with id: {}", tab_id);

        let status = KittenFocusTabCommand::new(self.socket.clone(), tab_id).execute()?;

        if !status.success() {
            error!("Failed to focus tab {}", tab_id);
            return Err(anyhow!("Failed to focus tab {}", tab_id));
        }

        info!("Successfully focused tab: {}", tab_id);
        Ok(())
    }

    pub fn create_session_tab_by_path(&self, project_path: &str, project_name: &str) -> Result<()> {
        use anyhow::anyhow;

        info!(
            "Creating new session tab for project '{}' at path: {}",
            project_name, project_path
        );

        let session_name = format!("📁 {}", project_name);

        let status = KittenLaunchCommand::new(self.socket.clone())
            .launch_type("tab")
            .cwd(project_path)
            .env("KITTY_SESSION_PROJECT", project_name)
            .tab_title(&session_name)
            .execute()?;

        if !status.success() {
            error!(
                "Failed to create session tab for project '{}'",
                project_name
            );
            return Err(anyhow!("Failed to create session tab"));
        }

        info!(
            "Successfully created session tab for project: {}",
            project_name
        );
        Ok(())
    }
}
