use anyhow::Result;
use log::{debug, info, warn};
use skim::prelude::*;
use std::fs;
use std::io::Cursor;

use crate::app::App;
use crate::utils::{expand_tilde, format_project_for_selection, parse_project_selection};

pub fn cmd_select(app: &App) -> Result<()> {
    info!("Starting interactive project selection");

    let directories = app.config.expanded_directories()?;
    let projects = get_projects_from_directories(directories)?;

    if projects.is_empty() {
        println!("No projects found");
        return Ok(());
    }

    // Prepare input for skim - format as "project_name (path)"
    let input_data = projects
        .iter()
        .map(|(name, path)| format_project_for_selection(name, path))
        .collect::<Vec<_>>()
        .join("\n");

    // Configure skim options
    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .prompt(Some("Select project> "))
        .build()
        .unwrap();

    // Create item reader
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(input_data));

    // Run skim
    let output = match Skim::run_with(&options, Some(items)) {
        Some(out) => out,
        None => {
            info!("Skim failed to start");
            println!("Selection failed");
            return Ok(());
        }
    };

    // Check if user aborted (ESC, Ctrl-C, etc.)
    if output.is_abort {
        info!("User aborted selection");
        return Ok(());
    }

    // Check if user selected anything
    if output.selected_items.is_empty() {
        info!("No project selected by user");
        return Ok(());
    }

    let selected_items = output.selected_items;

    if let Some(selected_item) = selected_items.first() {
        let selected_text = selected_item.output().to_string();

        match parse_project_selection(&selected_text) {
            Ok((project_name, project_path)) => {
                info!(
                    "Selected project: '{}' at path: {}",
                    project_name, project_path
                );

                // Check if session exists
                match app.kitty.match_session_tab(&project_name) {
                    Ok(Some(existing_tab)) => {
                        info!("Session already exists, focusing existing tab");
                        app.kitty.focus_tab(existing_tab.id)?;
                        println!("Switched to existing session: {}", project_name);
                    }
                    _ => {
                        info!("No existing session found, creating new one");
                        app.kitty
                            .create_session_tab_by_path(&project_path, &project_name)?;
                        println!(
                            "Created and switched to new session: {} ({})",
                            project_name, project_path
                        );
                    }
                }

                return Ok(());
            }
            Err(e) => {
                warn!("Failed to parse selected project: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

fn get_projects_from_directories(directories: Vec<String>) -> Result<Vec<(String, String)>> {
    let mut all_projects = Vec::new();

    for dir in directories {
        let expanded_dir = expand_tilde(&dir);
        debug!("Using directory as project: {}", expanded_dir);

        if let Ok(path) = fs::canonicalize(&expanded_dir) {
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    all_projects.push((name.to_string(), path.to_string_lossy().to_string()));
                }
            } else {
                warn!("Directory does not exist: {}", expanded_dir);
            }
        } else {
            warn!("Cannot resolve directory path: {}", expanded_dir);
        }
    }

    all_projects.sort_by(|a, b| a.0.cmp(&b.0));
    info!("Found {} projects from directories", all_projects.len());
    Ok(all_projects)
}
