use anyhow::{Result, anyhow};
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::env;
use std::process::Command;

const SESSION_ENV_VAR: &str = "KITTY_SESSION_PROJECT";

#[derive(Debug, Deserialize)]
pub struct KittyTab {
    pub id: u32,
}

#[derive(Debug, Deserialize)]
pub struct KittyWindow {
    pub tabs: Vec<KittyTab>,
}

pub fn get_kitty_socket() -> String {
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

pub fn match_session_tab(project_name: &str) -> Result<Option<KittyTab>> {
    debug!("Matching session tab for project: {}", project_name);

    let socket = get_kitty_socket();
    let match_arg = format!("env:{}={}", SESSION_ENV_VAR, project_name);

    debug!("Running kitten @ --to={} ls --match={}", socket, match_arg);

    let output = Command::new("kitten")
        .args(&[
            "@",
            &format!("--to={}", socket),
            "ls",
            &format!("--match={}", match_arg),
        ])
        .output()?;

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

pub fn focus_tab(tab_id: u32) -> Result<()> {
    info!("Focusing tab with id: {}", tab_id);

    let socket = get_kitty_socket();
    debug!(
        "Running kitten @ --to={} focus-tab --match=id:{}",
        socket, tab_id
    );

    let status = Command::new("kitten")
        .args(&[
            "@",
            &format!("--to={}", socket),
            "focus-tab",
            &format!("--match=id:{}", tab_id),
        ])
        .status()?;

    if !status.success() {
        error!("Failed to focus tab {}", tab_id);
        return Err(anyhow!("Failed to focus tab {}", tab_id));
    }

    info!("Successfully focused tab: {}", tab_id);
    Ok(())
}

pub fn create_session_tab_by_path(project_path: &str, project_name: &str) -> Result<()> {
    info!(
        "Creating new session tab for project '{}' at path: {}",
        project_name, project_path
    );

    let socket = get_kitty_socket();
    let session_name = format!("📁 {}", project_name);
    let env_arg = format!("{}={}", SESSION_ENV_VAR, project_name);

    debug!(
        "Running kitten @ --to={} launch --type=tab --cwd={} --env={} --tab-title={}",
        socket, project_path, env_arg, session_name
    );

    let status = Command::new("kitten")
        .args(&[
            "@",
            &format!("--to={}", socket),
            "launch",
            "--type=tab",
            &format!("--cwd={}", project_path),
            &format!("--env={}", env_arg),
            "--tab-title",
            &session_name,
        ])
        .status()?;

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
