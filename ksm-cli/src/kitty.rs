use anyhow::Result;
use kitty_lib::{
    CommandExecutor, KittenFocusTabCommand, KittenLaunchCommand, KittenLsCommand, KittyExecutor,
    KittyTab,
};
use log::{debug, error, info};

pub struct Kitty<E: CommandExecutor> {
    kitty: E,
}

impl Default for Kitty<KittyExecutor> {
    fn default() -> Self {
        Self::new()
    }
}

impl Kitty<KittyExecutor> {
    pub fn new() -> Self {
        Self {
            kitty: KittyExecutor::new(),
        }
    }
}

impl<E: CommandExecutor> Kitty<E> {
    pub fn with_executor(executor: E) -> Self {
        Self { kitty: executor }
    }

    pub fn match_session_tab(&self, project_name: &str) -> Result<Option<KittyTab>> {
        debug!("Matching session tab for project: {}", project_name);

        let ls_command = KittenLsCommand::new().match_env("KITTY_SESSION_PROJECT", project_name);
        let os_windows = self.kitty.ls(ls_command)?;

        if os_windows.is_empty() {
            debug!("No matching session found for project: {}", project_name);
            return Ok(None);
        }

        for os_window in os_windows {
            for tab in os_window.tabs {
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
        info!("Focusing tab with id: {}", tab_id);

        let focus_command = KittenFocusTabCommand::new(tab_id);
        let result = self.kitty.focus_tab(focus_command)?;

        if !result.is_success() {
            let error_msg = result
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            error!("Failed to focus tab {}: {}", tab_id, error_msg);
            return Err(anyhow::anyhow!(
                "Failed to focus tab {}: {}",
                tab_id,
                error_msg
            ));
        }

        info!("Successfully focused tab: {}", tab_id);
        Ok(())
    }

    pub fn create_session_tab_by_path(&self, project_path: &str, project_name: &str) -> Result<()> {
        info!(
            "Creating new session tab for project '{}' at path: {}",
            project_name, project_path
        );

        let session_name = format!("📁 {}", project_name);

        let launch_command = KittenLaunchCommand::new()
            .launch_type("tab")
            .cwd(project_path)
            .env("KITTY_SESSION_PROJECT", project_name)
            .tab_title(&session_name);
        let result = self.kitty.launch(launch_command)?;

        if !result.is_success() {
            let error_msg = result
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            error!(
                "Failed to create session tab for project '{}': {}",
                project_name, error_msg
            );
            return Err(anyhow::anyhow!(
                "Failed to create session tab: {}",
                error_msg
            ));
        }

        info!(
            "Successfully created session tab for project: {}",
            project_name
        );
        Ok(())
    }
}
