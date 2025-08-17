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
    projects: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct ConfigSearchData {
    dirs: Vec<String>,
    vsc: Vec<String>,
    // #[allow(dead_code)]
    // cmd: Vec<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct Config {
    dirs: Vec<String>,
    vsc_dirs: Vec<String>,
    base: Vec<KeyedProject>,
    /// profiles chosen for use, either implicitly or explicitly via args
    profiles: Vec<String>,
    /// all available profiles from the config file
    available_profiles: HashMap<String, Vec<KeyedProject>>,
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_from_path(None, None)
    }

    pub fn load_with_profiles(profiles: Option<Vec<String>>) -> Result<Self> {
        Self::load_from_path(None, profiles)
    }

    pub fn load_from_path(
        config_path: Option<PathBuf>,
        profiles: Option<Vec<String>>,
    ) -> Result<Self> {
        let config_path = config_path.unwrap_or_else(get_config_path);
        debug!("Loading config from: {:?}", config_path);

        let content = fs::read_to_string(&config_path).map_err(|e| {
            error!("Failed to read config file {:?}: {}", config_path, e);
            e
        })?;

        debug!("Loaded config {:?}", content);

        let data: SessionConfigData = serde_json::from_str(&content).map_err(|e| {
            error!("Failed to parse config JSON: {}", e);
            e
        })?;

        Self::from_config_data(data, profiles)
    }

    fn from_config_data(data: SessionConfigData, profiles: Option<Vec<String>>) -> Result<Self> {
        // Convert projects HashMap<String, HashMap<String, String>> to HashMap<String, Vec<KeyedProject>>
        let available_profiles: HashMap<String, Vec<KeyedProject>> = data
            .projects
            .into_iter()
            .map(|(profile_name, profile_map)| {
                let keyed_projects: Vec<KeyedProject> = profile_map.into_iter().collect();
                (profile_name, keyed_projects)
            })
            .collect();

        let profile_count: usize = available_profiles.values().map(|v| v.len()).sum();

        info!(
            "Successfully loaded config with {} profile projects across {} profiles",
            profile_count,
            available_profiles.len()
        );

        // Use only the profiles provided by the user, or empty list if none provided
        let selected_profiles = profiles.unwrap_or_default();

        Ok(Config {
            profiles: selected_profiles,
            dirs: data.search.dirs,
            vsc_dirs: data.search.vsc,
            base: Vec::new(), // No more base projects - all are in profiles
            available_profiles,
        })
    }

    pub fn keyed_projects(&self) -> Vec<KeyedProject> {
        // Start with base projects as a HashMap to handle key overrides
        let mut project_map: HashMap<String, String> = self.base.iter().cloned().collect();

        // Merge selected profiles in alphabetical order, with later profiles overriding earlier ones
        let mut selected_profile_names = self.profiles.clone();
        selected_profile_names.sort();

        for profile_name in selected_profile_names {
            if let Some(profile_projects) = self.available_profiles.get(&profile_name) {
                for (key, value) in profile_projects {
                    project_map.insert(key.clone(), value.clone());
                }
            }
        }

        // Convert back to Vec<KeyedProject>
        project_map.into_iter().collect()
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

        // Add discovered git projects
        if let Ok(git_projects) = self.discover_git_projects() {
            expanded_dirs.extend(git_projects);
        }

        Ok(expanded_dirs)
    }

    fn discover_git_projects(&self) -> Result<Vec<String>> {
        let mut git_projects = Vec::new();

        for vsc_dir_pattern in &self.vsc_dirs {
            let expanded_path = shellexpand::tilde(vsc_dir_pattern);
            let vsc_path = PathBuf::from(expanded_path.as_ref());

            if vsc_path.exists() && vsc_path.is_dir() {
                self.find_git_projects_recursive(&vsc_path, &mut git_projects)?;
            }
        }

        Ok(git_projects)
    }

    fn find_git_projects_recursive(
        &self,
        dir: &PathBuf,
        git_projects: &mut Vec<String>,
    ) -> Result<()> {
        Self::find_git_projects_recursive_helper(dir, git_projects)
    }

    fn find_git_projects_recursive_helper(
        dir: &PathBuf,
        git_projects: &mut Vec<String>,
    ) -> Result<()> {
        let git_dir = dir.join(".git");

        if git_dir.exists() {
            if let Some(dir_str) = dir.to_str() {
                git_projects.push(dir_str.to_string());
            }
            return Ok(());
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name() {
                        if !dir_name.to_string_lossy().starts_with('.') {
                            Self::find_git_projects_recursive_helper(&path, git_projects)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn get_config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".local/data/sessions.json")
}

#[cfg(test)]
fn get_all_directories_from_path(config_path: Option<PathBuf>) -> Result<Vec<String>> {
    let config = Config::load_from_path(config_path, None)?;
    config.expanded_directories()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use std::path::PathBuf;

    #[test]
    fn test_config_keyed_projects_no_profiles_only_base() {
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
                "default": {
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

        let config =
            Config::load_from_path(Some(temp.path().join("test_config.json")), None).unwrap();
        let projects = config.keyed_projects();

        // When no profiles are specified, no projects should be used (since base is now empty)
        assert_eq!(projects.len(), 0);
    }

    #[test]
    fn test_config_keyed_projects_selected_profiles_only() {
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
                "default": {
                    "P0": "~/base"
                },
                "personal": {
                    "P1": "~/personal"
                },
                "work": {
                    "P2": "~/work",
                    "P3": "~/work_specific"
                },
                "dev": {
                    "P4": "~/dev"
                }
            }
        }"#,
            )
            .unwrap();

        // Test selecting only "work" profile
        let config = Config::load_from_path(
            Some(temp.path().join("test_config.json")),
            Some(vec!["work".to_string()]),
        )
        .unwrap();
        let projects = config.keyed_projects();

        // Should have 2 projects: P2 (work), P3 (work_specific)
        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&("P2".to_string(), "~/work".to_string())));
        assert!(projects.contains(&("P3".to_string(), "~/work_specific".to_string())));
        // Should NOT contain default, personal or dev profiles
        assert!(!projects.contains(&("P0".to_string(), "~/base".to_string())));
        assert!(!projects.contains(&("P1".to_string(), "~/personal".to_string())));
        assert!(!projects.contains(&("P4".to_string(), "~/dev".to_string())));
    }

    #[test]
    fn test_config_keyed_projects_all_profiles_explicit() {
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
                "default": {
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

        // Explicitly specify all profiles
        let config = Config::load_from_path(
            Some(temp.path().join("test_config.json")),
            Some(vec!["personal".to_string(), "work".to_string()]),
        )
        .unwrap();
        let projects = config.keyed_projects();

        // Only selected profiles should be merged (personal + work)
        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&("P1".to_string(), "~/personal".to_string())));
        assert!(projects.contains(&("P2".to_string(), "~/work".to_string())));
        // Should NOT contain default
        assert!(!projects.contains(&("P0".to_string(), "~/base".to_string())));
    }

    #[test]
    fn test_config_keyed_projects_profile_override() {
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
                "default": {
                    "P0": "~/base",
                    "P1": "~/base_default"
                },
                "personal": {
                    "P1": "~/personal_override"
                },
                "work": {
                    "P1": "~/work_override",
                    "P2": "~/work"
                }
            }
        }"#,
            )
            .unwrap();

        // Test with both personal and work profiles to see override behavior
        let config = Config::load_from_path(
            Some(temp.path().join("test_config.json")),
            Some(vec!["personal".to_string(), "work".to_string()]),
        )
        .unwrap();
        let projects = config.keyed_projects();

        // Should have personal and work projects, with work overriding personal for P1
        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&("P1".to_string(), "~/work_override".to_string())));
        assert!(projects.contains(&("P2".to_string(), "~/work".to_string())));
        // Should NOT contain default or personal version of P1
        assert!(!projects.contains(&("P0".to_string(), "~/base".to_string())));
        assert!(!projects.contains(&("P1".to_string(), "~/personal_override".to_string())));
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
                    "default": {{}},
                    "personal": {{}},
                    "work": {{}}
                }}
            }}"#,
                temp.path().display()
            ))
            .unwrap();

        let config =
            Config::load_from_path(Some(temp.path().join("test_config.json")), None).unwrap();
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
                "default": {{}},
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
                "default": {{}},
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

    #[test]
    fn test_discover_git_projects() {
        let temp = TempDir::new().unwrap();

        // Create directory structure with git repos
        temp.child("dev/project1/.git").create_dir_all().unwrap();
        temp.child("dev/project2/.git/exclude")
            .create_dir_all()
            .unwrap();
        temp.child("dev/project3/.git").create_dir_all().unwrap();
        temp.child("dev/project3/submodule/.git")
            .create_dir_all()
            .unwrap();
        temp.child("dev/non-git-project/src")
            .create_dir_all()
            .unwrap();
        temp.child("work/repo1/.git").create_dir_all().unwrap();

        let config_content = format!(
            r#"{{
            "search": {{
                "dirs": [],
                "vsc": ["{}/dev", "{}/work"]
            }},
            "projects": {{
                "default": {{}},
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

        let config =
            Config::load_from_path(Some(temp.path().join("test_config.json")), None).unwrap();
        let git_projects = config.discover_git_projects().unwrap();

        assert_eq!(git_projects.len(), 4);

        let project_names: Vec<String> = git_projects
            .iter()
            .map(|p| {
                PathBuf::from(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert!(project_names.contains(&"project1".to_string()));
        assert!(project_names.contains(&"project2".to_string()));
        assert!(project_names.contains(&"project3".to_string()));
        assert!(project_names.contains(&"repo1".to_string()));
        assert!(!project_names.contains(&"submodule".to_string()));
        assert!(!project_names.contains(&"non-git-project".to_string()));
    }

    #[test]
    fn test_discover_git_projects_stops_at_git_boundary() {
        let temp = TempDir::new().unwrap();

        // Create nested git repos to test boundary stopping
        temp.child("parent/.git").create_dir_all().unwrap();
        temp.child("parent/child/.git").create_dir_all().unwrap();
        temp.child("parent/regular-dir/nested/.git")
            .create_dir_all()
            .unwrap();

        let config_content = format!(
            r#"{{
            "search": {{
                "dirs": [],
                "vsc": ["{}"]
            }},
            "projects": {{
                "default": {{}},
                "personal": {{}},
                "work": {{}}
            }}
        }}"#,
            temp.path().display()
        );

        temp.child("test_config.json")
            .write_str(&config_content)
            .unwrap();

        let config =
            Config::load_from_path(Some(temp.path().join("test_config.json")), None).unwrap();
        let git_projects = config.discover_git_projects().unwrap();

        assert_eq!(git_projects.len(), 1);

        let project_path = &git_projects[0];
        assert!(project_path.ends_with("parent"));
        assert!(!git_projects.iter().any(|p| p.contains("child")));
        assert!(!git_projects.iter().any(|p| p.contains("nested")));
    }

    #[test]
    fn test_expanded_directories_includes_git_projects() {
        let temp = TempDir::new().unwrap();

        // Create both regular dirs and git projects
        temp.child("regular_dir1").create_dir_all().unwrap();
        temp.child("regular_dir2").create_dir_all().unwrap();
        temp.child("git_projects/project1/.git")
            .create_dir_all()
            .unwrap();
        temp.child("git_projects/project2/.git")
            .create_dir_all()
            .unwrap();

        let config_content = format!(
            r#"{{
            "search": {{
                "dirs": ["{}/regular_dir1", "{}/regular_dir2"],
                "vsc": ["{}/git_projects"]
            }},
            "projects": {{
                "default": {{}},
                "personal": {{}},
                "work": {{}}
            }}
        }}"#,
            temp.path().display(),
            temp.path().display(),
            temp.path().display()
        );

        temp.child("test_config.json")
            .write_str(&config_content)
            .unwrap();

        let config =
            Config::load_from_path(Some(temp.path().join("test_config.json")), None).unwrap();
        let all_dirs = config.expanded_directories().unwrap();

        // Should include both regular dirs and discovered git projects
        assert_eq!(all_dirs.len(), 4);

        // Check regular dirs are included
        let regular_dir1_path = format!("{}/regular_dir1", temp.path().display());
        let regular_dir2_path = format!("{}/regular_dir2", temp.path().display());
        assert!(all_dirs.contains(&regular_dir1_path));
        assert!(all_dirs.contains(&regular_dir2_path));

        // Check git projects are included
        let git_project_paths: Vec<&String> = all_dirs
            .iter()
            .filter(|p| p.contains("git_projects") && p.contains("project"))
            .collect();
        assert_eq!(git_project_paths.len(), 2);
    }
}
