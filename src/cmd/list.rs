use anyhow::{anyhow, Result};
use log::{debug, error, info};
use std::env;
use std::path::Path;
use std::process::Command;

use crate::kitty::match_session_tab;

pub fn cmd_list() -> Result<()> {
    info!("Listing all available sessions");

    let projects = get_projects()?;

    println!("Available sessions:");
    for project in projects {
        let (status, tab_info) = match match_session_tab(&project) {
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

    let output = Command::new("fd")
        .args(&["--type=d", "--max-depth=1", ".", &projects_dir])
        .output()?;

    if !output.status.success() {
        error!("Failed to list projects from directory: {}", projects_dir);
        return Err(anyhow!("Failed to list projects"));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut projects = Vec::new();

    for line in stdout.lines() {
        if !line.is_empty() {
            if let Some(name) = Path::new(line).file_name().and_then(|n| n.to_str()) {
                if name
                    != Path::new(&projects_dir)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                {
                    projects.push(name.to_string());
                }
            }
        }
    }

    info!("Found {} projects in {}", projects.len(), projects_dir);
    debug!("Projects: {:?}", projects);
    Ok(projects)
}
