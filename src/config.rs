use anyhow::Result;
use glob::glob;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;


pub type KeyedProject = (String, String);

#[derive(Debug, Deserialize)]
pub struct SessionConfig {
    pub dirs: Vec<String>,
    pub dirs_special: Option<Vec<serde_json::Value>>,
    pub base: Vec<KeyedProject>,
    pub personal: Vec<KeyedProject>,
    pub work: Vec<KeyedProject>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpandedConfig {
    pub dirs: Vec<String>,
    pub base: Vec<KeyedProject>,
    pub personal: Vec<KeyedProject>,
    pub work: Vec<KeyedProject>,
}

pub fn get_config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".local/data/sessions.json")
}

pub fn load_config() -> Result<SessionConfig> {
    load_config_from_path(None)
}

pub fn load_config_from_path(config_path: Option<PathBuf>) -> Result<SessionConfig> {
    let config_path = config_path.unwrap_or_else(get_config_path);
    debug!("Loading config from: {:?}", config_path);
    
    let content = fs::read_to_string(&config_path)
        .map_err(|e| {
            error!("Failed to read config file {:?}: {}", config_path, e);
            e
        })?;
    
    let config: SessionConfig = serde_json::from_str(&content)
        .map_err(|e| {
            error!("Failed to parse config JSON: {}", e);
            e
        })?;
    
    info!("Successfully loaded config with {} base, {} personal, {} work projects", 
          config.base.len(), config.personal.len(), config.work.len());
    Ok(config)
}

pub fn get_keyed_projects_from_config(config: &SessionConfig, is_work: bool) -> Vec<KeyedProject> {
    let mut result = config.base.clone();
    
    if is_work {
        result.extend(config.work.clone());
    } else {
        result.extend(config.personal.clone());
    }

    result
}

pub fn get_keyed_projects(is_work: bool) -> Result<Vec<KeyedProject>> {
    get_keyed_projects_from_path(None, is_work)
}

pub fn get_keyed_projects_from_path(config_path: Option<PathBuf>, is_work: bool) -> Result<Vec<KeyedProject>> {
    let config = load_config_from_path(config_path)?;
    Ok(get_keyed_projects_from_config(&config, is_work))
}

pub fn get_all_directories(_is_work: bool) -> Result<Vec<String>> {
    get_all_directories_from_path(None, _is_work)
}

pub fn get_all_directories_from_path(config_path: Option<PathBuf>, _is_work: bool) -> Result<Vec<String>> {
    let config = load_config_from_path(config_path)?;
    let mut expanded_dirs = Vec::new();
    
    for dir_pattern in &config.dirs {
        let expanded_path = shellexpand::tilde(dir_pattern);
        
        if dir_pattern.contains('*') || dir_pattern.contains('?') || dir_pattern.contains('[') {
            // This is a glob pattern
            match glob(&expanded_path) {
                Ok(paths) => {
                    for entry in paths {
                        match entry {
                            Ok(path) => {
                                if path.is_dir() {
                                    if let Some(path_str) = path.to_str() {
                                        expanded_dirs.push(path_str.to_string());
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error reading glob path: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Invalid glob pattern '{}': {}", dir_pattern, e);
                }
            }
        } else {
            // Regular directory path
            expanded_dirs.push(expanded_path.to_string());
        }
    }
    
    Ok(expanded_dirs)
}