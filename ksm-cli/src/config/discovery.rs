use anyhow::Result;
use glob::glob;
use log::error;
use std::fs;
use std::path::PathBuf;

use super::types::SearchConfig;
use crate::Config;

impl Config {
    pub fn expanded_directories(&self) -> Result<Vec<String>> {
        let mut expanded_dirs = Vec::new();
        let resolved_search = self.resolved_search();

        let dirs = resolved_search.dirs.unwrap_or_default();
        for dir_pattern in &dirs {
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

    pub(crate) fn discover_git_projects(&self) -> Result<Vec<String>> {
        let mut git_projects = Vec::new();
        let resolved_search = self.resolved_search();
        let vsc_dirs = resolved_search.vsc.clone().unwrap_or_default();

        for vsc_dir_pattern in &vsc_dirs {
            let expanded_path = shellexpand::tilde(vsc_dir_pattern);
            let vsc_path = PathBuf::from(expanded_path.as_ref());

            if vsc_path.exists() && vsc_path.is_dir() {
                self.find_git_projects_recursive(&vsc_path, &mut git_projects, &resolved_search)?;
            }
        }

        Ok(git_projects)
    }

    fn find_git_projects_recursive(
        &self,
        dir: &PathBuf,
        git_projects: &mut Vec<String>,
        search_config: &SearchConfig,
    ) -> Result<()> {
        Self::find_git_projects_recursive_helper(dir, git_projects, search_config, 0)
    }

    fn find_git_projects_recursive_helper(
        dir: &PathBuf,
        git_projects: &mut Vec<String>,
        search_config: &SearchConfig,
        current_depth: u32,
    ) -> Result<()> {
        // Check max_depth limit
        if let Some(max_depth) = search_config.max_depth {
            if current_depth >= max_depth {
                return Ok(());
            }
        }

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
                        let dir_name_str = dir_name.to_string_lossy();

                        // Skip hidden directories
                        if dir_name_str.starts_with('.') {
                            continue;
                        }

                        // Check exclude patterns
                        if let Some(ref exclude_patterns) = search_config.exclude {
                            if exclude_patterns
                                .iter()
                                .any(|pattern| dir_name_str.contains(pattern))
                            {
                                continue;
                            }
                        }

                        Self::find_git_projects_recursive_helper(
                            &path,
                            git_projects,
                            search_config,
                            current_depth + 1,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }
}
