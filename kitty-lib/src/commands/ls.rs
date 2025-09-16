#[derive(Debug, Clone)]
pub struct KittenLsCommand {
    pub match_arg: Option<String>,
    pub use_tab_match: bool,
}

impl Default for KittenLsCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl KittenLsCommand {
    pub fn new() -> Self {
        Self {
            match_arg: None,
            use_tab_match: false,
        }
    }

    pub fn match_env(mut self, env_var: &str, value: &str) -> Self {
        self.match_arg = Some(format!("env:{}={}", env_var, value));
        self
    }

    pub fn match_tab_env(mut self, env_var: &str, value: &str) -> Self {
        self.match_arg = Some(format!("env:{}={}", env_var, value));
        self.use_tab_match = true;
        self
    }

    pub fn match_tab_title(mut self, title_pattern: &str) -> Self {
        self.match_arg = Some(format!("title:{}", title_pattern));
        self.use_tab_match = true;
        self
    }
}
