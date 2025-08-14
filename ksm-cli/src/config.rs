use anyhow::Result;
use glob::glob;
use log::{debug, error, info};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

pub type KeyedProject = (String, String);

#[derive(Debug, Deserialize)]
struct SessionConfigData {
    dirs: Vec<String>,
    base: Vec<KeyedProject>,
    personal: Vec<KeyedProject>,
    work: Vec<KeyedProject>,
}

#[derive(Debug, Clone)]
pub struct Config {
    dirs: Vec<String>,
    base: Vec<KeyedProject>,
    personal: Vec<KeyedProject>,
    work: Vec<KeyedProject>,
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_from_path(None)
    }

    pub fn load_from_path(config_path: Option<PathBuf>) -> Result<Self> {
        let config_path = config_path.unwrap_or_else(get_config_path);
        debug!("Loading config from: {:?}", config_path);

        let content = fs::read_to_string(&config_path).map_err(|e| {
            error!("Failed to read config file {:?}: {}", config_path, e);
            e
        })?;

        let data: SessionConfigData = serde_json::from_str(&content).map_err(|e| {
            error!("Failed to parse config JSON: {}", e);
            e
        })?;

        info!(
            "Successfully loaded config with {} base, {} personal, {} work projects",
            data.base.len(),
            data.personal.len(),
            data.work.len()
        );

        Ok(Config {
            dirs: data.dirs,
            base: data.base,
            personal: data.personal,
            work: data.work,
        })
    }

    pub fn keyed_projects(&self, is_work: bool) -> Vec<KeyedProject> {
        let mut result = self.base.clone();

        if is_work {
            result.extend(self.work.clone());
        } else {
            result.extend(self.personal.clone());
        }

        result
    }

    pub fn expanded_directories(&self) -> Result<Vec<String>> {
        let mut expanded_dirs = Vec::new();

        for dir_pattern in &self.dirs {
            let expanded_path = shellexpand::tilde(dir_pattern);

            // Check if the pattern contains glob characters
            if dir_pattern.contains('*') || dir_pattern.contains('?') || dir_pattern.contains('[') {
                // Handle as glob pattern
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
                // Handle as literal path - add it regardless of whether it exists
                expanded_dirs.push(expanded_path.to_string());
            }
        }

        Ok(expanded_dirs)
    }
}

pub fn get_config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".local/data/sessions.json")
}

// Backward compatibility functions - delegate to new Config struct
pub fn get_keyed_projects(is_work: bool) -> Result<Vec<KeyedProject>> {
    get_keyed_projects_from_path(None, is_work)
}

pub fn get_keyed_projects_from_path(
    config_path: Option<PathBuf>,
    is_work: bool,
) -> Result<Vec<KeyedProject>> {
    let config = Config::load_from_path(config_path)?;
    Ok(config.keyed_projects(is_work))
}

pub fn get_all_directories(_is_work: bool) -> Result<Vec<String>> {
    get_all_directories_from_path(None)
}

pub fn get_all_directories_from_path(config_path: Option<PathBuf>) -> Result<Vec<String>> {
    let config = Config::load_from_path(config_path)?;
    config.expanded_directories()
}

// Deprecated - use Config::load() instead
#[deprecated(note = "Use Config::load() instead")]
pub fn load_config() -> Result<Config> {
    Config::load()
}

#[deprecated(note = "Use Config::load_from_path() instead")]
pub fn load_config_from_path(config_path: Option<PathBuf>) -> Result<Config> {
    Config::load_from_path(config_path)
}

#[deprecated(note = "Use config.keyed_projects() instead")]
pub fn get_keyed_projects_from_config(config: &Config, is_work: bool) -> Vec<KeyedProject> {
    config.keyed_projects(is_work)
}
