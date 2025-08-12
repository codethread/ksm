#[derive(Debug, Clone)]
pub struct KittenLsCommand {
    pub match_arg: Option<String>,
}

impl Default for KittenLsCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl KittenLsCommand {
    pub fn new() -> Self {
        Self { match_arg: None }
    }

    pub fn match_env(mut self, env_var: &str, value: &str) -> Self {
        self.match_arg = Some(format!("env:{}={}", env_var, value));
        self
    }
}
