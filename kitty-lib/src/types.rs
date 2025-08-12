use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KittyWindow {
    pub id: u32,
    pub title: String,
    pub pid: u32,
    pub cwd: String,
    pub cmdline: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub is_self: bool,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub num: Option<u32>,
    #[serde(default)]
    pub recent: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KittyTab {
    pub id: u32,
    #[serde(default)]
    pub index: Option<u32>,
    pub title: String,
    pub windows: Vec<KittyWindow>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub recent: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KittyOsWindow {
    pub id: u32,
    pub tabs: Vec<KittyTab>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

pub type KittyLsResponse = Vec<KittyOsWindow>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KittyLaunchResponse {
    pub tab_id: Option<u32>,
    pub window_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KittyCommandResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error_message: Option<String>,
}

impl<T> KittyCommandResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error_message: None,
        }
    }

    pub fn success_empty() -> Self {
        Self {
            success: true,
            data: None,
            error_message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error_message: Some(message.into()),
        }
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn into_result(self) -> Result<T> {
        if self.success {
            match self.data {
                Some(data) => Ok(data),
                None => Err(anyhow::anyhow!("Command succeeded but no data returned")),
            }
        } else {
            let error_msg = self
                .error_message
                .unwrap_or_else(|| "Command failed".to_string());
            Err(anyhow::anyhow!(error_msg))
        }
    }
}
