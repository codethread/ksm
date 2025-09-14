#[derive(Debug, Clone)]
pub struct KittenLaunchCommand {
    pub launch_type: String,
    pub cwd: Option<String>,
    pub env: Option<String>,
    pub tab_title: Option<String>,
    pub inherit_session: bool,
}

impl Default for KittenLaunchCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl KittenLaunchCommand {
    pub fn new() -> Self {
        Self {
            launch_type: "tab".to_string(),
            cwd: None,
            env: None,
            tab_title: None,
            inherit_session: false,
        }
    }

    pub fn launch_type(mut self, launch_type: &str) -> Self {
        self.launch_type = launch_type.to_string();
        self
    }

    pub fn cwd(mut self, cwd: &str) -> Self {
        self.cwd = Some(cwd.to_string());
        self
    }

    pub fn env(mut self, env_var: &str, value: &str) -> Self {
        self.env = Some(format!("{}={}", env_var, value));
        self
    }

    pub fn tab_title(mut self, title: &str) -> Self {
        self.tab_title = Some(title.to_string());
        self
    }

    /// Enable automatic session inheritance from the current environment
    pub fn inherit_current_session(mut self) -> Self {
        self.inherit_session = true;
        self
    }
}
