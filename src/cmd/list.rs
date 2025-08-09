use anyhow::Result;
use log::{debug, info};
use std::env;
use std::fs;

use crate::app::App;

pub fn cmd_list(app: &App) -> Result<()> {
    info!("Listing all available sessions");

    let projects = get_projects()?;

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

fn get_projects() -> Result<Vec<String>> {
    let home = env::var("HOME").unwrap_or_default();
    let projects_dir = format!("{}/dev/projects", home);

    debug!("Scanning for projects in directory: {}", projects_dir);

    let entries = fs::read_dir(&projects_dir)?;
    let mut projects = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Only include directories (equivalent to --type=d)
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                projects.push(name.to_string());
            }
        }
    }

    info!("Found {} projects in {}", projects.len(), projects_dir);
    debug!("Projects: {:?}", projects);
    Ok(projects)
}
