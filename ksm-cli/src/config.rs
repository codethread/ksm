mod auto_profile;
mod discovery;
mod types;

use types::*;
pub use types::{KeyedProject, ProjectDefinition};

use anyhow::Result;
use log::{debug, error, info};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    // Raw config data
    #[allow(dead_code)]
    global_version: Option<String>,
    base_search: SearchConfig,
    base_projects: HashMap<String, ProjectDefinition>,
    base_keys: HashMap<String, String>,
    profiles: HashMap<String, ProfileConfig>,
    #[allow(dead_code)]
    auto_profile_rules: Vec<AutoProfileRule>,

    // Runtime state
    selected_profiles: Vec<String>,
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

        let data: SessionConfigData = toml::from_str(&content).map_err(|e| {
            error!("Failed to parse config TOML: {}", e);
            e
        })?;

        Self::from_config_data(data, profiles)
    }

    fn from_config_data(
        data: SessionConfigData,
        manual_profile: Option<Vec<String>>,
    ) -> Result<Self> {
        let global_version = data.global.map(|g| g.version);
        let base_search = data.search.unwrap_or_default();
        let base_projects = data.projects.unwrap_or_default();
        let base_keys = data.keys.unwrap_or_default();
        let profiles = data.profiles.unwrap_or_default();
        let auto_profile_rules = data.auto_profile.map(|ap| ap.rules).unwrap_or_default();

        // Determine which profiles to use
        let selected_profiles = if let Some(manual_profiles) = manual_profile {
            // Use all manual profiles if provided
            manual_profiles
        } else {
            // Use auto-selection rules
            if let Some(auto_profile) = auto_profile::select_auto_profile(&auto_profile_rules)? {
                vec![auto_profile]
            } else {
                vec![]
            }
        };

        let profile_count: usize = profiles
            .values()
            .map(|p| p.projects.as_ref().map(|proj| proj.len()).unwrap_or(0))
            .sum();
        let base_project_count = base_projects.len();

        info!(
            "Successfully loaded config with {} base projects and {} profile projects across {} profiles",
            base_project_count,
            profile_count,
            profiles.len()
        );

        if !selected_profiles.is_empty() {
            info!("Selected profiles: {:?}", selected_profiles);
        }

        Ok(Config {
            global_version,
            base_search,
            base_projects,
            base_keys,
            profiles,
            auto_profile_rules,
            selected_profiles,
        })
    }

    pub fn keyed_projects(&self) -> Vec<KeyedProject> {
        let resolved_keys = self.resolved_keys();
        let resolved_projects = self.resolved_projects();

        resolved_keys
            .into_iter()
            .filter_map(|(key, project_name)| {
                resolved_projects
                    .get(&project_name)
                    .map(|project_def| (key, project_def.path().to_string()))
            })
            .collect()
    }

    fn resolved_search(&self) -> SearchConfig {
        let mut result = self.base_search.clone();

        for profile_name in &self.selected_profiles {
            if self.profiles.contains_key(profile_name) {
                let profile_chain = self.build_profile_chain(profile_name);

                for chain_profile_name in profile_chain {
                    if let Some(chain_profile) = self.profiles.get(&chain_profile_name) {
                        if let Some(ref search) = chain_profile.search {
                            // Merge arrays by concatenation
                            if let Some(ref profile_dirs) = search.dirs {
                                result
                                    .dirs
                                    .get_or_insert_with(Vec::new)
                                    .extend(profile_dirs.clone());
                            }
                            if let Some(ref profile_vsc) = search.vsc {
                                result
                                    .vsc
                                    .get_or_insert_with(Vec::new)
                                    .extend(profile_vsc.clone());
                            }
                            // Override scalar values
                            if search.max_depth.is_some() {
                                result.max_depth = search.max_depth;
                            }
                            if let Some(ref profile_exclude) = search.exclude {
                                result
                                    .exclude
                                    .get_or_insert_with(Vec::new)
                                    .extend(profile_exclude.clone());
                            }
                        }
                    }
                }
            }
        }

        result
    }

    fn resolved_projects(&self) -> HashMap<String, ProjectDefinition> {
        let mut result = self.base_projects.clone();

        for profile_name in &self.selected_profiles {
            if self.profiles.contains_key(profile_name) {
                let profile_chain = self.build_profile_chain(profile_name);

                for chain_profile_name in profile_chain {
                    if let Some(chain_profile) = self.profiles.get(&chain_profile_name) {
                        if let Some(ref projects) = chain_profile.projects {
                            // Later profiles override earlier ones
                            result.extend(projects.clone());
                        }
                    }
                }
            }
        }

        result
    }

    fn resolved_keys(&self) -> HashMap<String, String> {
        let mut result = self.base_keys.clone();

        for profile_name in &self.selected_profiles {
            if self.profiles.contains_key(profile_name) {
                let profile_chain = self.build_profile_chain(profile_name);

                for chain_profile_name in profile_chain {
                    if let Some(chain_profile) = self.profiles.get(&chain_profile_name) {
                        if let Some(ref keys) = chain_profile.keys {
                            // Later profiles override earlier ones
                            result.extend(keys.clone());
                        }
                    }
                }
            }
        }

        result
    }

    fn build_profile_chain(&self, profile_name: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current_profile_name = profile_name.to_string();
        let mut visited = std::collections::HashSet::new();

        loop {
            if visited.contains(&current_profile_name) {
                // Prevent infinite loops
                break;
            }
            visited.insert(current_profile_name.clone());

            if let Some(profile) = self.profiles.get(&current_profile_name) {
                match &profile.extends {
                    Some(ProfileExtends::Single(extends_name)) => {
                        chain.push(current_profile_name.clone());
                        current_profile_name = extends_name.clone();
                    }
                    Some(ProfileExtends::Disabled(false)) => {
                        // extends = false, don't extend base
                        chain.push(current_profile_name);
                        break;
                    }
                    None => {
                        // No extends, use base + this profile
                        chain.push(current_profile_name);
                        break;
                    }
                    Some(ProfileExtends::Disabled(true)) => {
                        // This shouldn't happen but treat as no extends
                        chain.push(current_profile_name);
                        break;
                    }
                }
            } else {
                // Profile not found
                break;
            }
        }

        // Reverse to get correct order (base -> ... -> target)
        chain.reverse();
        chain
    }
}

fn get_config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".local/data/sessions.toml")
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

    mod test_helpers {
        use super::*;

        pub struct ConfigBuilder {
            global_version: Option<String>,
            search_dirs: Vec<String>,
            search_vsc: Vec<String>,
            search_max_depth: Option<u32>,
            search_exclude: Vec<String>,
            base_projects: Vec<(String, String)>,
            base_keys: Vec<(String, String)>,
            profiles: Vec<ProfileBuilder>,
            auto_profile_rules: Vec<AutoProfileRuleBuilder>,
        }

        pub struct ProfileBuilder {
            name: String,
            extends: Option<String>,
            extends_disabled: bool,
            search_dirs: Vec<String>,
            search_vsc: Vec<String>,
            search_max_depth: Option<u32>,
            search_exclude: Vec<String>,
            projects: Vec<(String, String)>,
            detailed_projects: Vec<(String, String, String)>, // (name, path, description)
            keys: Vec<(String, String)>,
        }

        pub struct AutoProfileRuleBuilder {
            hostname_regex: Option<String>,
            env: Vec<(String, String)>,
            ssh_session: Option<bool>,
            default: Option<bool>,
            profile: String,
        }

        #[allow(dead_code)]
        impl ConfigBuilder {
            pub fn new() -> Self {
                Self {
                    global_version: Some("1.0".to_string()),
                    search_dirs: Vec::new(),
                    search_vsc: Vec::new(),
                    search_max_depth: None,
                    search_exclude: Vec::new(),
                    base_projects: Vec::new(),
                    base_keys: Vec::new(),
                    profiles: Vec::new(),
                    auto_profile_rules: Vec::new(),
                }
            }

            pub fn version(mut self, version: &str) -> Self {
                self.global_version = Some(version.to_string());
                self
            }

            pub fn search_dirs(mut self, dirs: Vec<&str>) -> Self {
                self.search_dirs = dirs.into_iter().map(|s| s.to_string()).collect();
                self
            }

            pub fn search_vsc(mut self, vsc: Vec<&str>) -> Self {
                self.search_vsc = vsc.into_iter().map(|s| s.to_string()).collect();
                self
            }

            pub fn search_max_depth(mut self, depth: u32) -> Self {
                self.search_max_depth = Some(depth);
                self
            }

            pub fn search_exclude(mut self, exclude: Vec<&str>) -> Self {
                self.search_exclude = exclude.into_iter().map(|s| s.to_string()).collect();
                self
            }

            pub fn base_project(mut self, name: &str, path: &str) -> Self {
                self.base_projects
                    .push((name.to_string(), path.to_string()));
                self
            }

            pub fn base_key(mut self, key: &str, project: &str) -> Self {
                self.base_keys.push((key.to_string(), project.to_string()));
                self
            }

            pub fn profile(mut self, profile: ProfileBuilder) -> Self {
                self.profiles.push(profile);
                self
            }

            pub fn auto_profile_rule(mut self, rule: AutoProfileRuleBuilder) -> Self {
                self.auto_profile_rules.push(rule);
                self
            }

            pub fn build_toml(&self) -> String {
                let mut toml = String::new();

                // Global section
                if let Some(ref version) = self.global_version {
                    toml.push_str(&format!("[global]\nversion = \"{}\"\n\n", version));
                }

                // Search section
                if !self.search_dirs.is_empty()
                    || !self.search_vsc.is_empty()
                    || self.search_max_depth.is_some()
                    || !self.search_exclude.is_empty()
                {
                    toml.push_str("[search]\n");
                    if !self.search_dirs.is_empty() {
                        let dirs_str = self
                            .search_dirs
                            .iter()
                            .map(|d| format!("\"{}\"", d))
                            .collect::<Vec<_>>()
                            .join(", ");
                        toml.push_str(&format!("dirs = [{}]\n", dirs_str));
                    }
                    if !self.search_vsc.is_empty() {
                        let vsc_str = self
                            .search_vsc
                            .iter()
                            .map(|v| format!("\"{}\"", v))
                            .collect::<Vec<_>>()
                            .join(", ");
                        toml.push_str(&format!("vsc = [{}]\n", vsc_str));
                    }
                    if let Some(depth) = self.search_max_depth {
                        toml.push_str(&format!("max_depth = {}\n", depth));
                    }
                    if !self.search_exclude.is_empty() {
                        let exclude_str = self
                            .search_exclude
                            .iter()
                            .map(|e| format!("\"{}\"", e))
                            .collect::<Vec<_>>()
                            .join(", ");
                        toml.push_str(&format!("exclude = [{}]\n", exclude_str));
                    }
                    toml.push('\n');
                }

                // Base projects section
                if !self.base_projects.is_empty() {
                    toml.push_str("[projects]\n");
                    for (name, path) in &self.base_projects {
                        toml.push_str(&format!("{} = \"{}\"\n", name, path));
                    }
                    toml.push('\n');
                }

                // Base keys section
                if !self.base_keys.is_empty() {
                    toml.push_str("[keys]\n");
                    for (key, project) in &self.base_keys {
                        toml.push_str(&format!("{} = \"{}\"\n", key, project));
                    }
                    toml.push('\n');
                }

                // Profiles
                for profile in &self.profiles {
                    toml.push_str(&profile.build_toml());
                }

                // Auto profile rules
                if !self.auto_profile_rules.is_empty() {
                    toml.push_str("[auto_profile]\n\n");
                    for rule in &self.auto_profile_rules {
                        toml.push_str(&rule.build_toml());
                    }
                }

                toml
            }

            pub fn write_to_temp_file(&self, temp: &TempDir, filename: &str) -> std::path::PathBuf {
                let file_path = temp.path().join(filename);
                temp.child(filename).write_str(&self.build_toml()).unwrap();
                file_path
            }
        }

        #[allow(dead_code)]
        impl ProfileBuilder {
            pub fn new(name: &str) -> Self {
                Self {
                    name: name.to_string(),
                    extends: None,
                    extends_disabled: false,
                    search_dirs: Vec::new(),
                    search_vsc: Vec::new(),
                    search_max_depth: None,
                    search_exclude: Vec::new(),
                    projects: Vec::new(),
                    detailed_projects: Vec::new(),
                    keys: Vec::new(),
                }
            }

            pub fn extends(mut self, profile: &str) -> Self {
                self.extends = Some(profile.to_string());
                self
            }

            pub fn extends_disabled(mut self) -> Self {
                self.extends_disabled = true;
                self
            }

            pub fn search_dirs(mut self, dirs: Vec<&str>) -> Self {
                self.search_dirs = dirs.into_iter().map(|s| s.to_string()).collect();
                self
            }

            pub fn search_vsc(mut self, vsc: Vec<&str>) -> Self {
                self.search_vsc = vsc.into_iter().map(|s| s.to_string()).collect();
                self
            }

            pub fn search_max_depth(mut self, depth: u32) -> Self {
                self.search_max_depth = Some(depth);
                self
            }

            pub fn search_exclude(mut self, exclude: Vec<&str>) -> Self {
                self.search_exclude = exclude.into_iter().map(|s| s.to_string()).collect();
                self
            }

            pub fn project(mut self, name: &str, path: &str) -> Self {
                self.projects.push((name.to_string(), path.to_string()));
                self
            }

            pub fn detailed_project(mut self, name: &str, path: &str, description: &str) -> Self {
                self.detailed_projects.push((
                    name.to_string(),
                    path.to_string(),
                    description.to_string(),
                ));
                self
            }

            pub fn key(mut self, key: &str, project: &str) -> Self {
                self.keys.push((key.to_string(), project.to_string()));
                self
            }

            fn build_toml(&self) -> String {
                let mut toml = String::new();

                // Profile header with extends
                if self.extends_disabled {
                    toml.push_str(&format!("[profiles.{}]\nextends = false\n\n", self.name));
                } else if let Some(ref extends) = self.extends {
                    toml.push_str(&format!(
                        "[profiles.{}]\nextends = '{}'\n\n",
                        self.name, extends
                    ));
                }

                // Profile search section
                if !self.search_dirs.is_empty()
                    || !self.search_vsc.is_empty()
                    || self.search_max_depth.is_some()
                    || !self.search_exclude.is_empty()
                {
                    toml.push_str(&format!("[profiles.{}.search]\n", self.name));
                    if !self.search_dirs.is_empty() {
                        let dirs_str = self
                            .search_dirs
                            .iter()
                            .map(|d| format!("\"{}\"", d))
                            .collect::<Vec<_>>()
                            .join(", ");
                        toml.push_str(&format!("dirs = [{}]\n", dirs_str));
                    }
                    if !self.search_vsc.is_empty() {
                        let vsc_str = self
                            .search_vsc
                            .iter()
                            .map(|v| format!("\"{}\"", v))
                            .collect::<Vec<_>>()
                            .join(", ");
                        toml.push_str(&format!("vsc = [{}]\n", vsc_str));
                    }
                    if let Some(depth) = self.search_max_depth {
                        toml.push_str(&format!("max_depth = {}\n", depth));
                    }
                    if !self.search_exclude.is_empty() {
                        let exclude_str = self
                            .search_exclude
                            .iter()
                            .map(|e| format!("\"{}\"", e))
                            .collect::<Vec<_>>()
                            .join(", ");
                        toml.push_str(&format!("exclude = [{}]\n", exclude_str));
                    }
                    toml.push('\n');
                }

                // Profile projects section
                if !self.projects.is_empty() || !self.detailed_projects.is_empty() {
                    toml.push_str(&format!("[profiles.{}.projects]\n", self.name));
                    for (name, path) in &self.projects {
                        toml.push_str(&format!("{} = \"{}\"\n", name, path));
                    }
                    toml.push('\n');

                    // Detailed projects as tables
                    for (name, path, description) in &self.detailed_projects {
                        toml.push_str(&format!("[profiles.{}.projects.{}]\n", self.name, name));
                        toml.push_str(&format!("path = \"{}\"\n", path));
                        toml.push_str(&format!("description = \"{}\"\n\n", description));
                    }
                }

                // Profile keys section
                if !self.keys.is_empty() {
                    toml.push_str(&format!("[profiles.{}.keys]\n", self.name));
                    for (key, project) in &self.keys {
                        toml.push_str(&format!("{} = \"{}\"\n", key, project));
                    }
                    toml.push('\n');
                }

                toml
            }
        }

        #[allow(dead_code)]
        impl AutoProfileRuleBuilder {
            pub fn new(profile: &str) -> Self {
                Self {
                    hostname_regex: None,
                    env: Vec::new(),
                    ssh_session: None,
                    default: None,
                    profile: profile.to_string(),
                }
            }

            pub fn hostname_regex(mut self, regex: &str) -> Self {
                self.hostname_regex = Some(regex.to_string());
                self
            }

            pub fn env_var(mut self, key: &str, value: &str) -> Self {
                self.env.push((key.to_string(), value.to_string()));
                self
            }

            pub fn ssh_session(mut self, is_ssh: bool) -> Self {
                self.ssh_session = Some(is_ssh);
                self
            }

            pub fn default_rule(mut self) -> Self {
                self.default = Some(true);
                self
            }

            fn build_toml(&self) -> String {
                let mut toml = String::new();
                toml.push_str("[[auto_profile.rules]]\n");

                if let Some(ref regex) = self.hostname_regex {
                    toml.push_str(&format!("hostname_regex = \"{}\"\n", regex));
                }

                if !self.env.is_empty() {
                    toml.push_str("env = { ");
                    let env_pairs: Vec<String> = self
                        .env
                        .iter()
                        .map(|(k, v)| format!("{} = \"{}\"", k, v))
                        .collect();
                    toml.push_str(&env_pairs.join(", "));
                    toml.push_str(" }\n");
                }

                if let Some(ssh) = self.ssh_session {
                    toml.push_str(&format!("ssh_session = {}\n", ssh));
                }

                if let Some(default) = self.default {
                    toml.push_str(&format!("default = {}\n", default));
                }

                toml.push_str(&format!("profile = \"{}\"\n\n", self.profile));
                toml
            }
        }

        // Convenience functions for common test patterns
        pub fn simple_config() -> ConfigBuilder {
            ConfigBuilder::new()
                .base_project("dots", "~/dotfiles")
                .base_key("P1", "dots")
        }

        pub fn config_with_profiles() -> ConfigBuilder {
            ConfigBuilder::new()
                .base_project("dots", "~/dotfiles")
                .base_key("P1", "dots")
                .profile(
                    ProfileBuilder::new("work")
                        .project("frontend", "~/work/frontend")
                        .project("backend", "~/work/backend")
                        .key("P2", "frontend")
                        .key("P3", "backend"),
                )
        }

        #[allow(dead_code)]
        pub fn config_with_search() -> ConfigBuilder {
            ConfigBuilder::new()
                .search_dirs(vec!["~/project1", "~/project2"])
                .search_vsc(vec!["~/dev", "~/work"])
        }
    }

    use test_helpers::*;

    #[test]
    fn test_config_keyed_projects_no_profiles_only_base() {
        let temp = TempDir::new().unwrap();
        let config_path = simple_config().write_to_temp_file(&temp, "test_config.toml");

        let config = Config::load_from_path(Some(config_path), None).unwrap();
        let projects = config.keyed_projects();

        // Should use base projects and keys
        assert_eq!(projects.len(), 1);
        assert!(projects.contains(&("P1".to_string(), "~/dotfiles".to_string())));
    }

    #[test]
    fn test_config_keyed_projects_selected_profiles_only() {
        let temp = TempDir::new().unwrap();
        let config_path = config_with_profiles().write_to_temp_file(&temp, "test_config.toml");

        // Test selecting only "work" profile
        let config =
            Config::load_from_path(Some(config_path), Some(vec!["work".to_string()])).unwrap();
        let projects = config.keyed_projects();

        // Should have 3 projects: P1 (base), P2 (frontend), P3 (backend)
        assert_eq!(projects.len(), 3);
        assert!(projects.contains(&("P1".to_string(), "~/dotfiles".to_string())));
        assert!(projects.contains(&("P2".to_string(), "~/work/frontend".to_string())));
        assert!(projects.contains(&("P3".to_string(), "~/work/backend".to_string())));
    }

    #[test]
    fn test_config_keyed_projects_all_profiles_explicit() {
        let temp = TempDir::new().unwrap();
        let config_path = ConfigBuilder::new()
            .profile(
                ProfileBuilder::new("personal")
                    .project("personal_proj", "~/personal")
                    .key("P1", "personal_proj"),
            )
            .profile(
                ProfileBuilder::new("work")
                    .project("work_proj", "~/work")
                    .key("P2", "work_proj"),
            )
            .write_to_temp_file(&temp, "test_config.toml");

        // Explicitly specify all profiles
        let config = Config::load_from_path(
            Some(config_path),
            Some(vec!["personal".to_string(), "work".to_string()]),
        )
        .unwrap();
        let projects = config.keyed_projects();

        // Only selected profiles should be merged (personal + work)
        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&("P1".to_string(), "~/personal".to_string())));
        assert!(projects.contains(&("P2".to_string(), "~/work".to_string())));
    }

    #[test]
    fn test_config_keyed_projects_profile_override() {
        let temp = TempDir::new().unwrap();
        let config_path = ConfigBuilder::new()
            .profile(
                ProfileBuilder::new("personal")
                    .project("common_proj", "~/personal_override")
                    .key("P1", "common_proj"),
            )
            .profile(
                ProfileBuilder::new("work")
                    .project("common_proj", "~/work_override") // Override the same project
                    .project("work_only", "~/work")
                    .key("P1", "common_proj") // Override the same key
                    .key("P2", "work_only"),
            )
            .write_to_temp_file(&temp, "test_config.toml");

        // Test with both personal and work profiles to see override behavior
        let config = Config::load_from_path(
            Some(config_path),
            Some(vec!["personal".to_string(), "work".to_string()]),
        )
        .unwrap();
        let projects = config.keyed_projects();

        // Should have work projects overriding personal ones
        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&("P1".to_string(), "~/work_override".to_string())));
        assert!(projects.contains(&("P2".to_string(), "~/work".to_string())));
        // Should NOT contain personal version of P1
        assert!(!projects.contains(&("P1".to_string(), "~/personal_override".to_string())));
    }

    #[test]
    fn test_config_expanded_directories() {
        let temp = TempDir::new().unwrap();

        // Create test directories
        temp.child("project1").create_dir_all().unwrap();
        temp.child("project2/subdir").create_dir_all().unwrap();
        temp.child("non-project").create_dir_all().unwrap();

        let config_path = ConfigBuilder::new()
            .search_dirs(vec![&format!("{}/project*", temp.path().display())])
            .write_to_temp_file(&temp, "test_config.toml");

        let config = Config::load_from_path(Some(config_path), None).unwrap();
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

        let config_path = ConfigBuilder::new()
            .search_dirs(vec![
                &format!("{}/glob_*", temp.path().display()),
                &format!("{}/regular_project1", temp.path().display()),
                &format!("{}/regular_project2", temp.path().display()),
                &format!("{}/subdir/*", temp.path().display()),
            ])
            .write_to_temp_file(&temp, "test_config.toml");

        let result = get_all_directories_from_path(Some(config_path)).unwrap();

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

        let config_path = ConfigBuilder::new()
            .search_dirs(vec![
                &format!("{}/existing_dir", temp.path().display()),
                &format!("{}/nonexistent_dir", temp.path().display()),
                "~/dev",
            ])
            .write_to_temp_file(&temp, "test_config.toml");

        let result = get_all_directories_from_path(Some(config_path)).unwrap();

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

        let config_path = ConfigBuilder::new()
            .search_vsc(vec![
                &format!("{}/dev", temp.path().display()),
                &format!("{}/work", temp.path().display()),
            ])
            .write_to_temp_file(&temp, "test_config.toml");

        let config = Config::load_from_path(Some(config_path), None).unwrap();
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

        let config_path = ConfigBuilder::new()
            .search_vsc(vec![&temp.path().display().to_string()])
            .write_to_temp_file(&temp, "test_config.toml");

        let config = Config::load_from_path(Some(config_path), None).unwrap();
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

        let config_path = ConfigBuilder::new()
            .search_dirs(vec![
                &format!("{}/regular_dir1", temp.path().display()),
                &format!("{}/regular_dir2", temp.path().display()),
            ])
            .search_vsc(vec![&format!("{}/git_projects", temp.path().display())])
            .write_to_temp_file(&temp, "test_config.toml");

        let config = Config::load_from_path(Some(config_path), None).unwrap();
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
