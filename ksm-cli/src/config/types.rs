use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type KeyedProject = (String, String);

/// Project definition - can be a simple path string or detailed configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum ProjectDefinition {
    /// Simple project definition with just a path
    Simple(String),
    /// Detailed project definition with path and optional description
    Detailed {
        /// Path to the project directory (required)
        path: String,
        /// Optional description of the project
        description: Option<String>,
    },
}

impl ProjectDefinition {
    /// Get the path from either Simple or Detailed project definition
    pub fn path(&self) -> &str {
        match self {
            ProjectDefinition::Simple(path) => path,
            ProjectDefinition::Detailed { path, .. } => path,
        }
    }

    /// Get the optional description from Detailed project definition
    pub fn description(&self) -> Option<&str> {
        match self {
            ProjectDefinition::Simple(_) => None,
            ProjectDefinition::Detailed { description, .. } => description.as_deref(),
        }
    }
}

/// KSM configuration data structure
/// Supports profile inheritance for shared settings with machine-specific overrides
#[derive(Debug, Deserialize, Serialize)]
pub struct SessionConfigData {
    /// Global configuration settings including version
    pub global: Option<GlobalConfig>,
    /// Default search configuration (inherited by profiles)
    pub search: Option<SearchConfig>,
    /// Default project definitions (inherited by profiles)
    pub projects: Option<HashMap<String, ProjectDefinition>>,
    /// Default key bindings mapping keys to project names (inherited by profiles)
    pub keys: Option<HashMap<String, String>>,
    /// Session behavior configuration (inherited by profiles)
    pub session: Option<SessionBehaviorConfig>,
    /// Named profiles that can extend default or other profiles
    pub profiles: Option<HashMap<String, ProfileConfig>>,
    /// Rules for automatic profile selection based on environment
    pub auto_profile: Option<AutoProfileConfig>,
}

/// Global configuration settings
#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// Config version for migrations
    pub version: String,
}

/// Profile configuration that can extend default or other profiles
/// All arrays are merged/extended, objects are merged with profile taking precedence
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProfileConfig {
    /// Which profile to extend (or false to disable default extension)
    /// If default profile exists, it's always applied first: default -> extends -> profile
    pub extends: Option<ProfileExtends>,
    /// Search configuration for this profile
    pub search: Option<SearchConfig>,
    /// Project definitions for this profile
    pub projects: Option<HashMap<String, ProjectDefinition>>,
    /// Key bindings for this profile (keys reference project names)
    pub keys: Option<HashMap<String, String>>,
    /// Session behavior configuration for this profile
    pub session: Option<SessionBehaviorConfig>,
}

/// Profile extension configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum ProfileExtends {
    /// Name of profile to extend
    Single(String),
    /// Set to false to disable extending the default profile
    Disabled(bool),
}

/// Configuration for automatic profile selection
#[derive(Debug, Deserialize, Serialize)]
pub struct AutoProfileConfig {
    /// Rules evaluated in order, first match wins
    pub rules: Vec<AutoProfileRule>,
}

/// Rule for automatic profile selection based on environment conditions
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AutoProfileRule {
    /// Regex pattern to match against hostname
    pub hostname_regex: Option<String>,
    /// Environment variables that must match (key=value pairs)
    pub env: Option<HashMap<String, String>>,
    /// Whether this rule applies to SSH sessions
    pub ssh_session: Option<bool>,
    /// Whether this is the default fallback rule
    pub default: Option<bool>,
    /// Name of profile to use when this rule matches
    pub profile: String,
}

/// Search configuration for finding projects
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct SearchConfig {
    /// Direct directories to include in search (globs will be expanded)
    pub dirs: Option<Vec<String>>,
    /// Directories to recursively search for .git-based projects
    pub vsc: Option<Vec<String>>,
    /// Maximum depth for globbing (useful for performance in large directories)
    pub max_depth: Option<u32>,
    /// Patterns to exclude from search (e.g., "node_modules", "target", ".git")
    pub exclude: Option<Vec<String>>,
}

/// Session behavior configuration
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct SessionBehaviorConfig {
    /// Default navigation behavior for session tabs
    pub navigation: Option<NavigationConfig>,
    /// Keybinding configuration for session commands
    pub keybindings: Option<KeybindingConfig>,
    /// Default behavior for unnamed sessions
    pub unnamed_session: Option<UnnamedSessionConfig>,
}

/// Navigation configuration for session tabs
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct NavigationConfig {
    /// Whether to wrap around when navigating tabs (default: true)
    pub wrap_tabs: Option<bool>,
}

/// Keybinding configuration for session commands
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct KeybindingConfig {
    /// Keybinding for next-tab command
    pub next_tab: Option<String>,
    /// Keybinding for prev-tab command  
    pub prev_tab: Option<String>,
    /// Keybinding for new-tab command
    pub new_tab: Option<String>,
    /// Keybinding for close-all-session-tabs command
    pub close_all_session_tabs: Option<String>,
}

/// Configuration for default unnamed session behavior
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct UnnamedSessionConfig {
    /// Whether to treat regular tabs as session tabs when no explicit session is active
    pub treat_as_session: Option<bool>,
    /// Whether to enable session navigation commands when no explicit session is active
    pub enable_navigation: Option<bool>,
}
