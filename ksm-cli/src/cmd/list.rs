use anyhow::Result;
use log::{debug, info, warn};
use std::fs;

use crate::app::App;
use crate::utils::expand_tilde;

pub fn cmd_list(app: &App) -> Result<()> {
    info!("Listing all available sessions");

    let projects = get_projects(app)?;

    println!("Available sessions:");
    for project in projects {
        let (status, tab_info) = match app.kitty.match_session_tab(&project) {
            Ok(Some(tab)) => {
                debug!(
                    "Project '{}' has active session with tab id: {}",
                    project, tab.id
                );
                ("✓ (active)".to_string(), format!(" [tab:{}]", tab.id))
            }
            _ => {
                debug!("Project '{}' has no active session", project);
                ("○ (available)".to_string(), String::new())
            }
        };

        println!("  {} {}{}", status, project, tab_info);
    }

    info!("Finished listing sessions");
    Ok(())
}

fn get_projects(app: &App) -> Result<Vec<String>> {
    let directories = app.config.expanded_directories()?;
    let mut all_projects = Vec::new();

    for dir in directories {
        let expanded_dir = expand_tilde(&dir);
        debug!("Using directory as project: {}", expanded_dir);

        if let Ok(path) = fs::canonicalize(&expanded_dir) {
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    all_projects.push(name.to_string());
                }
            } else {
                warn!("Directory does not exist: {}", expanded_dir);
            }
        } else {
            warn!("Cannot resolve directory path: {}", expanded_dir);
        }
    }

    all_projects.sort();
    all_projects.dedup(); // Remove duplicates in case directories overlap
    info!("Found {} projects from directories", all_projects.len());
    debug!("Projects: {:?}", all_projects);
    Ok(all_projects)
}
