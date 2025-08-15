use anyhow::Result;
use glob::glob;
use log::{debug, error, info};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

pub type KeyedProject = (String, String);

#[derive(Debug, Deserialize)]
struct SessionConfigData {
    search: ConfigSearchData,
    projects: ConfigProjectsData,
}

#[derive(Debug, Deserialize)]
struct ConfigSearchData {
    dirs: Vec<String>,
    #[allow(dead_code)]
    vsc: Vec<String>,
    #[allow(dead_code)]
    cmd: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ConfigProjectsData {
    #[serde(rename = "*")]
    base: Option<HashMap<String, String>>,
    #[serde(flatten)]
    profiles: HashMap<String, HashMap<String, String>>,
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

        Self::from_config_data(data)
    }

    fn from_config_data(data: SessionConfigData) -> Result<Self> {
        // Extract base projects from the "*" key if it exists
        let base = if let Some(base_map) = data.projects.base {
            base_map.into_iter().collect()
        } else {
            Vec::new()
        };

        // Extract personal and work projects from profiles
        let personal: Vec<KeyedProject> = data
            .projects
            .profiles
            .get("personal")
            .map(|p| p.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let work: Vec<KeyedProject> = data
            .projects
            .profiles
            .get("work")
            .map(|p| p.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        info!(
            "Successfully loaded config with {} base, {} personal, {} work projects",
            base.len(),
            personal.len(),
            work.len()
        );

        Ok(Config {
            dirs: data.search.dirs,
            base,
            personal,
            work,
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

fn get_config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".local/data/sessions.json")
}

#[cfg(test)]
fn get_all_directories_from_path(config_path: Option<PathBuf>) -> Result<Vec<String>> {
    let config = Config::load_from_path(config_path)?;
    config.expanded_directories()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use std::path::PathBuf;

    #[test]
    fn test_config_keyed_projects_personal() {
        let temp = TempDir::new().unwrap();

        temp.child("test_config.json")
            .write_str(
                r#"{
            "search": {
                "dirs": [],
                "vsc": [],
                "cmd": []
            },
            "projects": {
                "*": {
                    "P0": "~/base"
                },
                "personal": {
                    "P1": "~/personal"
                },
                "work": {
                    "P2": "~/work"
                }
            }
        }"#,
            )
            .unwrap();

        let config = Config::load_from_path(Some(temp.path().join("test_config.json"))).unwrap();
        let projects = config.keyed_projects(false);

        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&("P0".to_string(), "~/base".to_string())));
        assert!(projects.contains(&("P1".to_string(), "~/personal".to_string())));
        assert!(!projects.contains(&("P2".to_string(), "~/work".to_string())));
    }

    #[test]
    fn test_config_keyed_projects_work() {
        let temp = TempDir::new().unwrap();

        temp.child("test_config.json")
            .write_str(
                r#"{
            "search": {
                "dirs": [],
                "vsc": [],
                "cmd": []
            },
            "projects": {
                "*": {
                    "P0": "~/base"
                },
                "personal": {
                    "P1": "~/personal"
                },
                "work": {
                    "P2": "~/work"
                }
            }
        }"#,
            )
            .unwrap();

        let config = Config::load_from_path(Some(temp.path().join("test_config.json"))).unwrap();
        let projects = config.keyed_projects(true);

        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&("P0".to_string(), "~/base".to_string())));
        assert!(projects.contains(&("P2".to_string(), "~/work".to_string())));
        assert!(!projects.contains(&("P1".to_string(), "~/personal".to_string())));
    }

    #[test]
    fn test_config_expanded_directories() {
        let temp = TempDir::new().unwrap();

        // Create test directories
        temp.child("project1").create_dir_all().unwrap();
        temp.child("project2/subdir").create_dir_all().unwrap();
        temp.child("non-project").create_dir_all().unwrap();

        temp.child("test_config.json")
            .write_str(&format!(
                r#"{{
                "search": {{
                    "dirs": ["{}/project*"],
                    "vsc": [],
                    "cmd": []
                }},
                "projects": {{
                    "*": {{}},
                    "personal": {{}},
                    "work": {{}}
                }}
            }}"#,
                temp.path().display()
            ))
            .unwrap();

        let config = Config::load_from_path(Some(temp.path().join("test_config.json"))).unwrap();
        let directories = config.expanded_directories().unwrap();

        assert_eq!(directories.len(), 2);
        let dir_names: Vec<String> = directories
            .iter()
            .map(|p| {
                PathBuf::from(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert!(dir_names.contains(&"project1".to_string()));
        assert!(dir_names.contains(&"project2".to_string()));
    }

    #[test]
    fn test_dirs_mixed_glob_and_regular_patterns() {
        let temp = TempDir::new().unwrap();

        // Create directory structure more readably
        for dir in &[
            "glob_project1",
            "glob_project2",
            "regular_project1",
            "regular_project2",
        ] {
            temp.child(dir).create_dir_all().unwrap();
        }

        temp.child("subdir/nested1").create_dir_all().unwrap();
        temp.child("subdir/nested2").create_dir_all().unwrap();

        let config_content = format!(
            r#"{{
            "search": {{
                "dirs": [
                    "{}/glob_*",
                    "{}/regular_project1",
                    "{}/regular_project2",
                    "{}/subdir/*"
                ],
                "vsc": [],
                "cmd": []
            }},
            "projects": {{
                "*": {{}},
                "personal": {{}},
                "work": {{}}
            }}
        }}"#,
            temp.path().display(),
            temp.path().display(),
            temp.path().display(),
            temp.path().display()
        );

        temp.child("test_config.json")
            .write_str(&config_content)
            .unwrap();

        let result =
            get_all_directories_from_path(Some(temp.path().join("test_config.json"))).unwrap();

        // Should find: glob_project1, glob_project2, regular_project1 (literal), regular_project2 (literal), nested1, nested2
        assert_eq!(result.len(), 6);

        // Check that regular paths are returned as literal paths (full paths)
        let regular_project1_path = format!("{}/regular_project1", temp.path().display());
        let regular_project2_path = format!("{}/regular_project2", temp.path().display());
        assert!(result.contains(&regular_project1_path));
        assert!(result.contains(&regular_project2_path));

        // Check that glob patterns still expand to actual directories
        let glob_matches: Vec<&String> = result
            .iter()
            .filter(|p| p.contains("glob_project"))
            .collect();
        assert_eq!(glob_matches.len(), 2);

        let nested_matches: Vec<&String> = result.iter().filter(|p| p.contains("nested")).collect();
        assert_eq!(nested_matches.len(), 2);
    }

    #[test]
    fn test_literal_paths_behavior() {
        let temp = TempDir::new().unwrap();

        // Create one existing directory
        temp.child("existing_dir").create_dir_all().unwrap();

        let config_content = format!(
            r#"{{
            "search": {{
                "dirs": [
                    "{}/existing_dir",
                    "{}/nonexistent_dir",
                    "~/dev"
                ],
                "vsc": [],
                "cmd": []
            }},
            "projects": {{
                "*": {{}},
                "personal": {{}},
                "work": {{}}
            }}
        }}"#,
            temp.path().display(),
            temp.path().display()
        );

        temp.child("test_config.json")
            .write_str(&config_content)
            .unwrap();

        let result =
            get_all_directories_from_path(Some(temp.path().join("test_config.json"))).unwrap();

        // Should contain all 3 paths as literals, regardless of existence
        assert_eq!(result.len(), 3);

        let existing_path = format!("{}/existing_dir", temp.path().display());
        let nonexistent_path = format!("{}/nonexistent_dir", temp.path().display());

        assert!(result.contains(&existing_path));
        assert!(result.contains(&nonexistent_path));

        // Should expand tilde
        let home_expanded = result.iter().find(|p| p.contains("/dev")).unwrap();
        assert!(
            home_expanded.starts_with('/')
                || home_expanded.starts_with(std::env::var("HOME").unwrap_or_default().as_str())
        );
    }
}
