use anyhow::Result;
use kitty_lib::{
    CommandExecutor, KittenFocusTabCommand, KittenLaunchCommand, KittenLsCommand, KittyExecutor,
};
use log::{debug, error, info};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KittyTab {
    pub id: u32,
}

#[derive(Debug, Deserialize)]
pub struct KittyWindow {
    pub tabs: Vec<KittyTab>,
}

pub struct Kitty<E: CommandExecutor> {
    executor: E,
}

impl Default for Kitty<KittyExecutor> {
    fn default() -> Self {
        Self::new()
    }
}

impl Kitty<KittyExecutor> {
    pub fn new() -> Self {
        Self {
            executor: KittyExecutor::new(),
        }
    }
}

impl<E: CommandExecutor> Kitty<E> {
    pub fn with_executor(executor: E) -> Self {
        Self { executor }
    }

    pub fn match_session_tab(&self, project_name: &str) -> Result<Option<KittyTab>> {
        debug!("Matching session tab for project: {}", project_name);

        let command = KittenLsCommand::new().match_env("KITTY_SESSION_PROJECT", project_name);
        let output = self.executor.execute_ls_command(command)?;

        if !output.status.success() {
            debug!("No matching session found for project: {}", project_name);
            return Ok(None);
        }

        let windows: Vec<KittyWindow> = serde_json::from_slice(&output.stdout).map_err(|e| {
            error!("Failed to parse kitten ls output: {}", e);
            e
        })?;

        for window in windows {
            if let Some(tab) = window.tabs.into_iter().next() {
                info!(
                    "Found existing session tab for project '{}' with id: {}",
                    project_name, tab.id
                );
                return Ok(Some(tab));
            }
        }

        debug!(
            "No tabs found in matching windows for project: {}",
            project_name
        );
        Ok(None)
    }

    pub fn focus_tab(&self, tab_id: u32) -> Result<()> {
        use anyhow::anyhow;

        info!("Focusing tab with id: {}", tab_id);

        let command = KittenFocusTabCommand::new(tab_id);
        let status = self.executor.execute_focus_tab_command(command)?;

        if !status.success() {
            error!("Failed to focus tab {}", tab_id);
            return Err(anyhow!("Failed to focus tab {}", tab_id));
        }

        info!("Successfully focused tab: {}", tab_id);
        Ok(())
    }

    pub fn create_session_tab_by_path(&self, project_path: &str, project_name: &str) -> Result<()> {
        use anyhow::anyhow;

        info!(
            "Creating new session tab for project '{}' at path: {}",
            project_name, project_path
        );

        let session_name = format!("📁 {}", project_name);

        let command = KittenLaunchCommand::new()
            .launch_type("tab")
            .cwd(project_path)
            .env("KITTY_SESSION_PROJECT", project_name)
            .tab_title(&session_name);
        let status = self.executor.execute_launch_command(command)?;

        if !status.success() {
            error!(
                "Failed to create session tab for project '{}'",
                project_name
            );
            return Err(anyhow!("Failed to create session tab"));
        }

        info!(
            "Successfully created session tab for project: {}",
            project_name
        );
        Ok(())
    }
}
