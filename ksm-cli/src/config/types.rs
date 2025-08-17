use serde::Deserialize;
use std::collections::HashMap;

pub type KeyedProject = (String, String);

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ProjectDefinition {
    Simple(String),
    Detailed {
        path: String,
        description: Option<String>,
    },
}

impl ProjectDefinition {
    pub fn path(&self) -> &str {
        match self {
            ProjectDefinition::Simple(path) => path,
            ProjectDefinition::Detailed { path, .. } => path,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            ProjectDefinition::Simple(_) => None,
            ProjectDefinition::Detailed { description, .. } => description.as_deref(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SessionConfigData {
    pub global: Option<GlobalConfig>,
    pub search: Option<SearchConfig>,
    pub projects: Option<HashMap<String, ProjectDefinition>>,
    pub keys: Option<HashMap<String, String>>,
    pub profiles: Option<HashMap<String, ProfileConfig>>,
    pub auto_profile: Option<AutoProfileConfig>,
}

#[derive(Debug, Deserialize)]
pub struct GlobalConfig {
    pub version: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProfileConfig {
    pub extends: Option<ProfileExtends>,
    pub search: Option<SearchConfig>,
    pub projects: Option<HashMap<String, ProjectDefinition>>,
    pub keys: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ProfileExtends {
    Single(String),
    Disabled(bool), // for extends = false
}

#[derive(Debug, Deserialize)]
pub struct AutoProfileConfig {
    pub rules: Vec<AutoProfileRule>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AutoProfileRule {
    pub hostname_regex: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub ssh_session: Option<bool>,
    pub default: Option<bool>,
    pub profile: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SearchConfig {
    pub dirs: Option<Vec<String>>,
    pub vsc: Option<Vec<String>>,
    pub max_depth: Option<u32>,
    pub exclude: Option<Vec<String>>,
}
