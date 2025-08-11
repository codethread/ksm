use anyhow::{Result, anyhow};
use log::{debug, error, info};
use std::path::Path;

use crate::config::{KeyedProject, get_keyed_projects};
use crate::app::App;
use crate::utils::expand_tilde;

pub fn cmd_key(app: &App, key: &str, is_work: bool, print_path: bool) -> Result<()> {
    let keyed_projects = get_keyed_projects(is_work)?;
    cmd_key_with_projects(app, key, print_path, &keyed_projects)
}

pub fn cmd_key_with_projects(
    app: &App,
    key: &str,
    print_path: bool,
    keyed_projects: &[KeyedProject],
) -> Result<()> {
    info!(
        "Switching to project by key '{}' (print_path: {})",
        key, print_path
    );

    debug!("Loaded {} keyed projects", keyed_projects.len());

    let expanded_path = resolve_project_path(key, keyed_projects)?;

    if print_path {
        println!("{}", expanded_path);
        return Ok(());
    }

    let project_name = Path::new(&expanded_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    info!(
        "Found project '{}' at path: {}",
        project_name, expanded_path
    );

    // Check if session exists
    if let Ok(Some(existing_tab)) = app.kitty.match_session_tab(project_name) {
        info!("Session already exists, focusing existing tab");
        return app.kitty.focus_tab(existing_tab.id);
    }

    info!("No existing session found, creating new one");
    app.kitty.create_session_tab_by_path(&expanded_path, project_name)
}

pub fn resolve_project_path(key: &str, keyed_projects: &[KeyedProject]) -> Result<String> {
    let project_path = keyed_projects
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, path)| path)
        .ok_or_else(|| {
            error!("No project found for key: {}", key);
            anyhow!("No project found for key: {}", key)
        })?;

    Ok(expand_tilde(project_path))
}
