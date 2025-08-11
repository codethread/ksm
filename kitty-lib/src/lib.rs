use anyhow::Result;
use log::debug;
use std::process::{Command, Output};

pub struct KittenLsCommand {
    socket: String,
    match_arg: Option<String>,
}

impl KittenLsCommand {
    pub fn new(socket: String) -> Self {
        Self {
            socket,
            match_arg: None,
        }
    }

    pub fn match_env(mut self, env_var: &str, value: &str) -> Self {
        self.match_arg = Some(format!("env:{}={}", env_var, value));
        self
    }

    pub fn execute(self) -> Result<Output> {
        let socket_arg = format!("--to={}", self.socket);
        let mut args = vec!["@", &socket_arg, "ls"];
        
        let match_formatted;
        if let Some(match_arg) = &self.match_arg {
            match_formatted = format!("--match={}", match_arg);
            args.push(&match_formatted);
            debug!("Running kitten @ --to={} ls --match={}", self.socket, match_arg);
        } else {
            debug!("Running kitten @ --to={} ls", self.socket);
        }

        Ok(Command::new("kitten").args(&args).output()?)
    }
}

pub struct KittenFocusTabCommand {
    socket: String,
    tab_id: u32,
}

impl KittenFocusTabCommand {
    pub fn new(socket: String, tab_id: u32) -> Self {
        Self { socket, tab_id }
    }

    pub fn execute(self) -> Result<std::process::ExitStatus> {
        let socket_arg = format!("--to={}", self.socket);
        let match_arg = format!("--match=id:{}", self.tab_id);
        let args = [
            "@",
            &socket_arg,
            "focus-tab",
            &match_arg,
        ];

        debug!(
            "Running kitten @ --to={} focus-tab --match=id:{}",
            self.socket, self.tab_id
        );

        Ok(Command::new("kitten").args(&args).status()?)
    }
}

pub struct KittenLaunchCommand {
    socket: String,
    launch_type: String,
    cwd: Option<String>,
    env: Option<String>,
    tab_title: Option<String>,
}

impl KittenLaunchCommand {
    pub fn new(socket: String) -> Self {
        Self {
            socket,
            launch_type: "tab".to_string(),
            cwd: None,
            env: None,
            tab_title: None,
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

    pub fn execute(self) -> Result<std::process::ExitStatus> {
        let socket_arg = format!("--to={}", self.socket);
        let type_arg = format!("--type={}", self.launch_type);
        let mut args = vec![
            "@",
            &socket_arg,
            "launch",
            &type_arg,
        ];

        let cwd_formatted;
        if let Some(cwd) = &self.cwd {
            cwd_formatted = format!("--cwd={}", cwd);
            args.push(&cwd_formatted);
        }

        let env_formatted;
        if let Some(env) = &self.env {
            env_formatted = format!("--env={}", env);
            args.push(&env_formatted);
        }

        if let Some(tab_title) = &self.tab_title {
            args.push("--tab-title");
            args.push(tab_title);
        }

        debug!(
            "Running kitten @ --to={} launch --type={} {}{}{}",
            self.socket,
            self.launch_type,
            self.cwd.as_ref().map(|c| format!("--cwd={} ", c)).unwrap_or_default(),
            self.env.as_ref().map(|e| format!("--env={} ", e)).unwrap_or_default(),
            self.tab_title.as_ref().map(|t| format!("--tab-title={}", t)).unwrap_or_default()
        );

        Ok(Command::new("kitten").args(&args).status()?)
    }
}